import Metal
import MetalKit
import SwiftUI
import simd
import QuartzCore
import GameController

/// The first native Metal walk camera for a real (not gallery-arranged)
/// FalloutNV.esm CELL. Draws source NIF objects at original REFR transforms.
struct GoodspringsWalkView: UIViewRepresentable {
    let world: GoodspringsWorld
    let move: SIMD2<Float>
    let jumpSerial: Int

    func makeCoordinator() -> GoodspringsWorldRenderer { GoodspringsWorldRenderer() }
    func makeUIView(context: Context) -> MTKView {
        let view = MTKView(frame: .zero, device: MTLCreateSystemDefaultDevice())
        view.colorPixelFormat = .bgra8Unorm_srgb
        view.depthStencilPixelFormat = .depth32Float
        view.clearColor = MTLClearColor(red: 0.08, green: 0.10, blue: 0.12, alpha: 1)
        view.preferredFramesPerSecond = 60
        view.isPaused = false
        view.enableSetNeedsDisplay = false
        context.coordinator.configure(view, world: world)
        let look = UIPanGestureRecognizer(target: context.coordinator,
                                          action: #selector(GoodspringsWorldRenderer.look(_:)))
        look.maximumNumberOfTouches = 1
        view.addGestureRecognizer(look)
        view.delegate = context.coordinator
        return view
    }
    func updateUIView(_ view: MTKView, context: Context) {
        context.coordinator.move = move
        context.coordinator.enqueueJump(jumpSerial)
    }
}

private struct WorldUniforms {
    var mvp: simd_float4x4
}

final class GoodspringsWorldRenderer: NSObject, MTKViewDelegate {
    private struct GPUBatch {
        let start: Int
        let count: Int
        let texture: MTLTexture?
    }
    private struct GPUModel {
        let vertexBuffer: MTLBuffer
        let vertexCount: Int
        let batches: [GPUBatch]
    }
    private struct GPUInstance {
        let modelIndex: Int
        let transform: simd_float4x4
        let authoredPosition: SIMD3<Float>
    }
    private var models: [GPUModel?] = []
    private var instances: [GPUInstance] = []
    private var pipeline: MTLRenderPipelineState?
    private var depthState: MTLDepthStencilState?
    private var queue: MTLCommandQueue?
    private var whiteTexture: MTLTexture?
    private var lastTick = CACurrentMediaTime()

    // World coordinates are meters relative to the authentic ESM COC marker.
    // Camera eye starts roughly 1.65 meters over the authored floor marker.
    private var eye = SIMD3<Float>(0, 1.65, 0)
    private var yaw: Float = 0
    private var pitch: Float = 0
    var move = SIMD2<Float>(0,0)
    private var motor: WalkMotor?
    private var lastJumpSerial = 0
    private var jumpQueued = false

    func enqueueJump(_ serial: Int) {
        if serial != lastJumpSerial {
            lastJumpSerial = serial
            jumpQueued = true
        }
    }

    @objc func look(_ gesture: UIPanGestureRecognizer) {
        guard let view = gesture.view else { return }
        let delta = gesture.translation(in: view)
        yaw += Float(delta.x) * 0.006
        pitch = max(-1.36, min(1.36, pitch - Float(delta.y) * 0.006))
        gesture.setTranslation(.zero, in: view)
    }

    private func translate(_ p: SIMD3<Float>) -> simd_float4x4 {
        var result = matrix_identity_float4x4
        result.columns.3 = SIMD4<Float>(p.x,p.y,p.z,1)
        return result
    }
    private func scaleMatrix(_ factor: Float) -> simd_float4x4 {
        var result = matrix_identity_float4x4
        result.columns.0.x = factor
        result.columns.1.y = factor
        result.columns.2.z = factor
        return result
    }
    private func projection(_ aspect: Float) -> simd_float4x4 {
        let near:Float=0.06, far:Float=350
        let fov:Float = .pi / 2.9
        let y:Float = 1 / tan(fov * 0.5)
        let x = y / max(aspect, 0.05)
        return simd_float4x4(columns: (
            SIMD4<Float>(x,0,0,0),
            SIMD4<Float>(0,y,0,0),
            SIMD4<Float>(0,0,far/(near-far),-1),
            SIMD4<Float>(0,0,(far*near)/(near-far),0)
        ))
    }

    /// Right-handed Y-up view matrix, camera looking along local -Z.
    /// Rotation is applied to the CAMERA, so the model geometry stays at
    /// its original game-world transforms.
    private func cameraMatrix() -> simd_float4x4 {
        let forward=SIMD3<Float>(sin(yaw)*cos(pitch),sin(pitch),-cos(yaw)*cos(pitch))
        let z=simd_normalize(-forward)
        let x=simd_normalize(simd_cross(SIMD3<Float>(0,1,0),z))
        let y=simd_cross(z,x)
        return simd_float4x4(columns:(
            SIMD4<Float>(x.x,y.x,z.x,0),
            SIMD4<Float>(x.y,y.y,z.y,0),
            SIMD4<Float>(x.z,y.z,z.z,0),
            SIMD4<Float>(-simd_dot(x,eye),-simd_dot(y,eye),-simd_dot(z,eye),1)
        ))
    }

    func configure(_ view: MTKView, world: GoodspringsWorld) {
        yaw = world.initialYaw
        motor = WalkMotor(collision:world.collision)
        if let motor { eye = motor.feet + SIMD3<Float>(0,1.65,0) }
        NSLog("Goodsprings movement: %d source triangles indexed for floor/walls",
              world.collision.triangleCount)
        guard let device=view.device,let library=device.makeDefaultLibrary(),
              let vertex=library.makeFunction(name:"fnv_scene_vertex"),
              let fragment=library.makeFunction(name:"fnv_scene_fragment") else {
            NSLog("Goodsprings: native Metal device/shaders unavailable")
            return
        }
        let desc=MTLRenderPipelineDescriptor()
        desc.vertexFunction=vertex
        desc.fragmentFunction=fragment
        desc.colorAttachments[0].pixelFormat=view.colorPixelFormat
        desc.depthAttachmentPixelFormat=view.depthStencilPixelFormat
        do { pipeline=try device.makeRenderPipelineState(descriptor:desc) }
        catch {NSLog("Goodsprings: Metal pipeline error %@",String(describing:error));return}
        let depth=MTLDepthStencilDescriptor()
        depth.depthCompareFunction = .less
        depth.isDepthWriteEnabled = true
        depthState=device.makeDepthStencilState(descriptor:depth)
        queue=device.makeCommandQueue()
        let fallback=MTLTextureDescriptor.texture2DDescriptor(
            pixelFormat:.rgba8Unorm_srgb,width:1,height:1,mipmapped:false)
        fallback.usage=[.shaderRead]
        whiteTexture=device.makeTexture(descriptor:fallback)
        let white:[UInt8]=[255,255,255,255]
        white.withUnsafeBytes { raw in
            if let ptr=raw.baseAddress {
                whiteTexture?.replace(region:MTLRegionMake2D(0,0,1,1),mipmapLevel:0,
                                      withBytes:ptr,bytesPerRow:4)
            }
        }
        models.reserveCapacity(world.models.count)
        for (idx,source) in world.models.enumerated() {
            guard let source,
                  source.vertexData.count>=48,
                  source.vertexData.count%48==0 else {
                models.append(nil);continue
            }
            let vertexBuffer=source.vertexData.withUnsafeBytes { raw -> MTLBuffer? in
                guard let base=raw.baseAddress else {return nil}
                return device.makeBuffer(bytes:base,length:raw.count,options:.storageModeShared)
            }
            guard let vertexBuffer else {models.append(nil);continue}
            var textures: [MTLTexture?] = []
            textures.reserveCapacity(source.materials.count)
            for material in source.materials {
                var texture: MTLTexture? = nil
                if let material,
                   material.width > 0, material.height > 0,
                   material.pixels.count == material.width * material.height * 4 {
                    let descriptor=MTLTextureDescriptor.texture2DDescriptor(
                        pixelFormat:.rgba8Unorm_srgb,width:material.width,
                        height:material.height,mipmapped:false)
                    descriptor.usage=[.shaderRead]
                    texture=device.makeTexture(descriptor:descriptor)
                    material.pixels.withUnsafeBytes { raw in
                        if let ptr=raw.baseAddress {
                            texture?.replace(region:MTLRegionMake2D(
                                0,0,material.width,material.height),
                                mipmapLevel:0,withBytes:ptr,
                                bytesPerRow:material.width*4)
                        }
                    }
                }
                textures.append(texture)
            }
            let batches: [GPUBatch] = source.batches.map { sourceBatch in
                GPUBatch(start: sourceBatch.start, count: sourceBatch.count,
                         texture: textures[sourceBatch.materialIndex])
            }
            models.append(GPUModel(vertexBuffer:vertexBuffer,
                                   vertexCount:source.vertexData.count/48,
                                   batches:batches))
            if idx%40 == 0 {NSLog("Goodsprings: uploaded model %d/%d",idx,world.models.count)}
        }
        for instance in world.instances {
            guard models.indices.contains(instance.modelIndex),
                  models[instance.modelIndex] != nil else {continue}
            let transform=translate(instance.position)
                * simd_float4x4(instance.rotation)
                * scaleMatrix(instance.scale)
            instances.append(GPUInstance(modelIndex:instance.modelIndex,
                                         transform:transform,authoredPosition:instance.position))
        }
        NSLog("Goodsprings: native renderer loaded %d models, %d authored REFR placements",
              models.compactMap{$0}.count,instances.count)
    }

    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {}

    func draw(in view: MTKView) {
        guard let pipeline,let depthState,let queue,
              let renderPass=view.currentRenderPassDescriptor,
              let drawable=view.currentDrawable,
              let command=queue.makeCommandBuffer(),
              let encoder=command.makeRenderCommandEncoder(descriptor:renderPass) else {return}
        let now=CACurrentMediaTime()
        let delta=Float(max(0,min(now-lastTick,0.05)))
        lastTick=now

        // Walk with grounded foot position, 0.28m wall radius, gravity and
        // jump, using authored NIF collision triangles as a conservative
        // fallback. Original Havok collision meshes remain future work.
        // Optional game controller works entirely offline.
        var walking = move
        var speed:Float = 2.5
        if let gamepad=GCController.controllers().first(where:{$0.extendedGamepad != nil})?.extendedGamepad {
            let left=SIMD2<Float>(gamepad.leftThumbstick.xAxis.value,
                                  gamepad.leftThumbstick.yAxis.value)
            if simd_length(left)>0.14 { walking=left }
            let rightStick=SIMD2<Float>(gamepad.rightThumbstick.xAxis.value,
                                        gamepad.rightThumbstick.yAxis.value)
            if simd_length(rightStick)>0.12 {
                yaw+=rightStick.x * delta * 2.5
                pitch=max(-1.36,min(1.36,pitch+rightStick.y * delta * 1.8))
            }
            if gamepad.leftTrigger.value>0.5 {speed=4.4}
        }
        let forward=SIMD3<Float>(sin(yaw),0,-cos(yaw))
        let right=SIMD3<Float>(cos(yaw),0,sin(yaw))
        let magnitude=simd_length(walking)
        let direction = magnitude>0.02
            ? (forward*walking.y+right*walking.x) / max(1,magnitude)
            : SIMD3<Float>(repeating:0)
        let controllerJump=GCController.controllers().first(where:{$0.extendedGamepad != nil})?
            .extendedGamepad?.buttonA.isPressed ?? false
        let jumpNow=jumpQueued || controllerJump
        jumpQueued=false
        motor?.tick(move:direction*speed,dt:delta,jump:jumpNow)
        if let motor { eye = motor.feet + SIMD3<Float>(0,1.65,0) }
        let aspect=Float(view.drawableSize.width / max(view.drawableSize.height,1))
        let viewProjection=projection(aspect) * cameraMatrix()
        encoder.setRenderPipelineState(pipeline)
        encoder.setDepthStencilState(depthState)
        // Every placement retains the CELL's original transform; the same
        // mesh can occur several times without duplicating its GPU buffer.
        for instance in instances {
            guard let model=models[instance.modelIndex] else {continue}
            let distance=simd_length(instance.authoredPosition-eye)
            if distance>180 {continue} // bounded distance budget for mobile
            var uniforms=WorldUniforms(mvp:viewProjection*instance.transform)
            encoder.setVertexBuffer(model.vertexBuffer,offset:0,index:0)
            encoder.setVertexBytes(&uniforms,length:MemoryLayout<WorldUniforms>.stride,index:1)
            // Each source NIF shape draws with its OWN authored diffuse.
            // Missing DDS falls back only for that shape, never across meshes.
            for batch in model.batches {
                var textured:UInt32=batch.texture==nil ? 0 : 1
                encoder.setFragmentBytes(&textured,length:MemoryLayout<UInt32>.stride,index:0)
                encoder.setFragmentTexture(batch.texture ?? whiteTexture,index:0)
                encoder.drawPrimitives(type:.triangle,vertexStart:batch.start,vertexCount:batch.count)
            }
        }
        encoder.endEncoding()
        command.present(drawable)
        command.commit()
    }
}
