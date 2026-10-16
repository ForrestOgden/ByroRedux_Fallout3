import Metal
import MetalKit
import SwiftUI
import simd

struct MetalSceneView: UIViewRepresentable {
    let nifData: Data?
    let revision: Int
    let textureData: Data?
    let textureWidth: Int
    let textureHeight: Int
    let textureRevision: Int
    func makeCoordinator() -> SceneCoordinator { SceneCoordinator() }
    func makeUIView(context: Context) -> MTKView {
        let view = MTKView(frame: .zero, device: MTLCreateSystemDefaultDevice())
        // DDS texture sampling decodes sRGB into linear RGB.
        // The display target must encode linear fragments back to sRGB.
        // The previous unorm drawable caused unexpectedly dark textures.
        view.colorPixelFormat = .bgra8Unorm_srgb
        view.depthStencilPixelFormat = .depth32Float
        view.clearColor = MTLClearColor(red: 0.025, green: 0.035, blue: 0.05, alpha: 1)
        view.preferredFramesPerSecond = 60
        context.coordinator.configure(view)
        context.coordinator.setPreview(nifData, revision: revision, device: view.device)
        context.coordinator.setTexture(textureData, width: textureWidth, height: textureHeight,
                                       revision: textureRevision, device: view.device)
        let pan = UIPanGestureRecognizer(target: context.coordinator, action: #selector(SceneCoordinator.handlePan(_:)))
        pan.maximumNumberOfTouches = 1
        view.addGestureRecognizer(pan)
        view.addGestureRecognizer(UIPinchGestureRecognizer(target: context.coordinator, action: #selector(SceneCoordinator.handlePinch(_:))))
        let reset = UITapGestureRecognizer(target: context.coordinator, action: #selector(SceneCoordinator.resetCamera(_:)))
        reset.numberOfTapsRequired = 2
        view.addGestureRecognizer(reset)
        view.delegate = context.coordinator
        return view
    }
    func updateUIView(_ view: MTKView, context: Context) {
        context.coordinator.setPreview(nifData, revision: revision, device: view.device)
        context.coordinator.setTexture(textureData, width: textureWidth, height: textureHeight,
                                       revision: textureRevision, device: view.device)
    }
}

private struct SceneVertex {
    // Three 16-byte float4 lanes = 48-byte vertex ABI, matching Rust and Metal.
    var position: SIMD4<Float>
    var color: SIMD4<Float>
    var uvLight: SIMD4<Float>
}

private struct SceneUniforms {
    var mvp: simd_float4x4
}

final class SceneCoordinator: NSObject, MTKViewDelegate {
    private var pipeline: MTLRenderPipelineState?
    private var depthState: MTLDepthStencilState?
    private var queue: MTLCommandQueue?
    private var vertices: MTLBuffer?
    private var vertexCount = 0
    private var start = CACurrentMediaTime()
    private var orbitYaw: Float = 0
    private var orbitPitch: Float = 0.12
    private var cameraDistance: Float = 5

    @objc func handlePan(_ gesture: UIPanGestureRecognizer) {
        guard let view = gesture.view else { return }
        let p = gesture.translation(in: view)
        orbitYaw += Float(p.x) * 0.012
        orbitPitch = max(-1.35, min(1.35, orbitPitch + Float(p.y) * 0.012))
        gesture.setTranslation(.zero, in: view)
    }

    @objc func handlePinch(_ gesture: UIPinchGestureRecognizer) {
        let scale = Float(gesture.scale)
        guard scale.isFinite && scale > 0 else { return }
        cameraDistance = max(2.8, min(12, cameraDistance / scale))
        gesture.scale = 1
    }

    @objc func resetCamera(_ gesture: UITapGestureRecognizer) {
        orbitYaw = 0
        orbitPitch = 0.12
        cameraDistance = 5
        start = CACurrentMediaTime()
    }
    private var activeRevision = -1
    private var importedVertices: MTLBuffer?
    private var importedVertexCount = 0
    private var activeTextureRevision = -1
    private var importedTexture: MTLTexture?
    private var fallbackTexture: MTLTexture?

    func setTexture(_ data: Data?, width: Int, height: Int,
                    revision: Int, device: MTLDevice?) {
        guard activeTextureRevision != revision else { return }
        activeTextureRevision = revision
        importedTexture = nil
        guard let data, let device, width > 0, height > 0,
              width <= 2048, height <= 2048,
              data.count == width * height * 4 else { return }

        let descriptor = MTLTextureDescriptor.texture2DDescriptor(
            pixelFormat: .rgba8Unorm_srgb, width: width, height: height, mipmapped: false)
        descriptor.usage = [.shaderRead]
        guard let texture = device.makeTexture(descriptor: descriptor) else { return }
        data.withUnsafeBytes { source in
            guard let base = source.baseAddress else { return }
            texture.replace(region: MTLRegionMake2D(0, 0, width, height),
                            mipmapLevel: 0, withBytes: base, bytesPerRow: width * 4)
        }
        importedTexture = texture
        NSLog("FNV DDS preview: uploaded %d x %d RGBA pixels", width, height)
    }

    func setPreview(_ bytes: Data?, revision: Int, device: MTLDevice?) {
        guard revision != activeRevision else { return }
        activeRevision = revision
        importedVertices = nil
        importedVertexCount = 0
        guard let bytes, !bytes.isEmpty, bytes.count % 48 == 0,
              bytes.count <= 180_000 * 48, let device else { return }
        let buffer = bytes.withUnsafeBytes { raw -> MTLBuffer? in
            guard let base = raw.baseAddress else { return nil }
            return device.makeBuffer(bytes: base, length: raw.count)
        }
        importedVertices = buffer
        importedVertexCount = buffer == nil ? 0 : bytes.count / 48
        NSLog("FNV NIF preview: %d triangle vertices", importedVertexCount)
    }

    func configure(_ view: MTKView) {
        guard let device = view.device,
              let lib = device.makeDefaultLibrary(),
              let vs = lib.makeFunction(name: "fnv_scene_vertex"),
              let fs = lib.makeFunction(name: "fnv_scene_fragment") else { return }
        let desc = MTLRenderPipelineDescriptor()
        desc.vertexFunction = vs
        desc.fragmentFunction = fs
        desc.colorAttachments[0].pixelFormat = view.colorPixelFormat
        desc.depthAttachmentPixelFormat = view.depthStencilPixelFormat
        do { pipeline = try device.makeRenderPipelineState(descriptor: desc) }
        catch { NSLog("FNV 3D pipeline error: %@", String(describing: error)); return }
        let depth = MTLDepthStencilDescriptor()
        depth.depthCompareFunction = .less
        depth.isDepthWriteEnabled = true
        depthState = device.makeDepthStencilState(descriptor: depth)
        queue = device.makeCommandQueue()
        let fallback = MTLTextureDescriptor.texture2DDescriptor(
            pixelFormat: .rgba8Unorm_srgb, width: 1, height: 1, mipmapped: false)
        fallback.usage = [.shaderRead]
        fallbackTexture = device.makeTexture(descriptor: fallback)
        var white: [UInt8] = [255, 255, 255, 255]
        white.withUnsafeBytes { pixels in
            if let base = pixels.baseAddress {
                fallbackTexture?.replace(region: MTLRegionMake2D(0, 0, 1, 1),
                                         mipmapLevel: 0, withBytes: base, bytesPerRow: 4)
            }
        }

        let corners: [SIMD3<Float>] = [
            [-1,-1,-1], [1,-1,-1], [1,1,-1], [-1,1,-1],
            [-1,-1,1], [1,-1,1], [1,1,1], [-1,1,1]
        ]
        let faces: [([Int], SIMD3<Float>)] = [
            ([0,1,2, 0,2,3], [0.95,0.65,0.18]),
            ([5,4,7, 5,7,6], [0.15,0.70,0.80]),
            ([4,0,3, 4,3,7], [0.75,0.35,0.85]),
            ([1,5,6, 1,6,2], [0.35,0.85,0.40]),
            ([3,2,6, 3,6,7], [0.85,0.35,0.35]),
            ([4,5,1, 4,1,0], [0.35,0.45,0.95])
        ]
        let mesh = faces.flatMap { indices, color in
            indices.map { index in
                let p = corners[index]
                return SceneVertex(
                    position: SIMD4<Float>(p.x, p.y, p.z, 1),
                    color: SIMD4<Float>(color.x, color.y, color.z, 1),
                    uvLight: SIMD4<Float>(0, 0, 1, 0)
                )
            }
        }
        // Catch an ABI regression on-device instead of silently drawing corrupt triangles.
        guard MemoryLayout<SceneVertex>.stride == 48,
              MemoryLayout<SceneVertex>.offset(of: \.color) == 16,
              MemoryLayout<SceneVertex>.offset(of: \.uvLight) == 32 else {
            NSLog("FNV 3D probe: invalid SceneVertex memory layout")
            return
        }
        vertexCount = mesh.count
        vertices = mesh.withUnsafeBytes { bytes in
            guard let base = bytes.baseAddress else { return nil }
            return device.makeBuffer(bytes: base, length: bytes.count)
        }
    }

    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {}

    func draw(in view: MTKView) {
        guard let pipeline, let depthState, let queue, let vertices,
              let pass = view.currentRenderPassDescriptor,
              let drawable = view.currentDrawable,
              let cmd = queue.makeCommandBuffer(),
              let encoder = cmd.makeRenderCommandEncoder(descriptor: pass) else { return }
        let t = Float(CACurrentMediaTime() - start)
        let aspect = Float(view.drawableSize.width / max(view.drawableSize.height, 1))
        let fov: Float = .pi / 3
        let y = 1 / tan(fov / 2)
        let x = y / max(aspect, 0.01)
        let near: Float = 0.1
        let far: Float = 100
        // Metal clip space z is [0,1]; camera looks toward -Z.
        let projection = simd_float4x4(columns: (
            SIMD4<Float>(x,0,0,0),
            SIMD4<Float>(0,y,0,0),
            SIMD4<Float>(0,0,far/(near-far),-1),
            SIMD4<Float>(0,0,(far*near)/(near-far),0)
        ))
        let angle = t * 0.28 + orbitYaw
        let c = cos(angle), s = sin(angle)
        let yaw = simd_float4x4(columns: (
            SIMD4<Float>(c,0,-s,0), SIMD4<Float>(0,1,0,0),
            SIMD4<Float>(s,0,c,0), SIMD4<Float>(0,0,0,1)
        ))
        let cp = cos(orbitPitch), sp = sin(orbitPitch)
        let pitch = simd_float4x4(columns: (
            SIMD4<Float>(1,0,0,0), SIMD4<Float>(0,cp,sp,0),
            SIMD4<Float>(0,-sp,cp,0), SIMD4<Float>(0,0,0,1)
        ))
        var translation = matrix_identity_float4x4
        translation.columns.3.z = -cameraDistance
        var uniforms = SceneUniforms(mvp: projection * translation * yaw * pitch)
        encoder.setRenderPipelineState(pipeline)
        encoder.setDepthStencilState(depthState)
        encoder.setVertexBuffer(importedVertices ?? vertices, offset: 0, index: 0)
        encoder.setVertexBytes(&uniforms, length: MemoryLayout<SceneUniforms>.stride, index: 1)
        var textureMode: UInt32 = importedVertices != nil && importedTexture != nil ? 1 : 0
        encoder.setFragmentBytes(&textureMode, length: MemoryLayout<UInt32>.stride, index: 0)
        encoder.setFragmentTexture(importedTexture ?? fallbackTexture, index: 0)
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: importedVertices == nil ? vertexCount : importedVertexCount)
        encoder.endEncoding()
        cmd.present(drawable)
        cmd.commit()
    }
}
