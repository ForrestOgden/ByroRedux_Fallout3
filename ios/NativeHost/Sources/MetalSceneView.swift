import Metal
import MetalKit
import SwiftUI
import simd

struct MetalSceneView: UIViewRepresentable {
    let nifData: Data?
    let revision: Int
    func makeCoordinator() -> SceneCoordinator { SceneCoordinator() }
    func makeUIView(context: Context) -> MTKView {
        let view = MTKView(frame: .zero, device: MTLCreateSystemDefaultDevice())
        view.colorPixelFormat = .bgra8Unorm
        view.depthStencilPixelFormat = .depth32Float
        view.clearColor = MTLClearColor(red: 0.025, green: 0.035, blue: 0.05, alpha: 1)
        view.preferredFramesPerSecond = 60
        context.coordinator.configure(view)
        context.coordinator.setPreview(nifData, revision: revision, device: view.device)
        view.delegate = context.coordinator
        return view
    }
    func updateUIView(_ view: MTKView, context: Context) {
        context.coordinator.setPreview(nifData, revision: revision, device: view.device)
    }
}

private struct SceneVertex {
    // Explicit 16-byte lanes: 32 bytes per vertex, identical to Metal float4/float4.
    var position: SIMD4<Float>
    var color: SIMD4<Float>
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
    private var activeRevision = -1
    private var importedVertices: MTLBuffer?
    private var importedVertexCount = 0

    func setPreview(_ bytes: Data?, revision: Int, device: MTLDevice?) {
        guard revision != activeRevision else { return }
        activeRevision = revision
        importedVertices = nil
        importedVertexCount = 0
        guard let bytes, !bytes.isEmpty, bytes.count % 32 == 0,
              bytes.count <= 180_000 * 32, let device else { return }
        let buffer = bytes.withUnsafeBytes { raw -> MTLBuffer? in
            guard let base = raw.baseAddress else { return nil }
            return device.makeBuffer(bytes: base, length: raw.count)
        }
        importedVertices = buffer
        importedVertexCount = buffer == nil ? 0 : bytes.count / 32
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
                    color: SIMD4<Float>(color.x, color.y, color.z, 1)
                )
            }
        }
        // Catch an ABI regression on-device instead of silently drawing corrupt triangles.
        guard MemoryLayout<SceneVertex>.stride == 32,
              MemoryLayout<SceneVertex>.offset(of: \.color) == 16 else {
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
        let c = cos(t * 0.55), s = sin(t * 0.55)
        let rotation = simd_float4x4(columns: (
            SIMD4<Float>(c,0,-s,0), SIMD4<Float>(0,1,0,0),
            SIMD4<Float>(s,0,c,0), SIMD4<Float>(0,0,0,1)
        ))
        var translation = matrix_identity_float4x4
        translation.columns.3.z = -5
        var uniforms = SceneUniforms(mvp: projection * translation * rotation)
        encoder.setRenderPipelineState(pipeline)
        encoder.setDepthStencilState(depthState)
        encoder.setVertexBuffer(importedVertices ?? vertices, offset: 0, index: 0)
        encoder.setVertexBytes(&uniforms, length: MemoryLayout<SceneUniforms>.stride, index: 1)
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: importedVertices == nil ? vertexCount : importedVertexCount)
        encoder.endEncoding()
        cmd.present(drawable)
        cmd.commit()
    }
}
