import Metal
import MetalKit
import SwiftUI

/// Runs *one real native Metal render pipeline*, not the ByroRedux Vulkan renderer.
/// All shader vertices/colors are self-generated; no game assets are embedded.
struct MetalTriangleView: UIViewRepresentable {
    func makeCoordinator() -> MetalTriangleCoordinator {
        MetalTriangleCoordinator()
    }

    func makeUIView(context: Context) -> MTKView {
        let view = MTKView(frame: .zero, device: MTLCreateSystemDefaultDevice())
        view.colorPixelFormat = .bgra8Unorm
        view.clearColor = MTLClearColor(red: 0.06, green: 0.07, blue: 0.09, alpha: 1)
        view.preferredFramesPerSecond = 30
        view.isPaused = false
        view.enableSetNeedsDisplay = false
        context.coordinator.configure(view)
        view.delegate = context.coordinator
        return view
    }

    func updateUIView(_ uiView: MTKView, context: Context) {
        // Stateless diagnostic; no SwiftUI reconfiguration needed.
    }
}

final class MetalTriangleCoordinator: NSObject, MTKViewDelegate {
    private var commandQueue: MTLCommandQueue?
    private var pipeline: MTLRenderPipelineState?

    func configure(_ view: MTKView) {
        guard let device = view.device else {
            NSLog("FNV Metal probe: no Metal device")
            return
        }
        guard let shaderLibrary = device.makeDefaultLibrary(),
              let vertex = shaderLibrary.makeFunction(name: "fnv_triangle_vertex"),
              let fragment = shaderLibrary.makeFunction(name: "fnv_triangle_fragment") else {
            NSLog("FNV Metal probe: precompiled Metal shader functions unavailable")
            return
        }

        let descriptor = MTLRenderPipelineDescriptor()
        descriptor.vertexFunction = vertex
        descriptor.fragmentFunction = fragment
        descriptor.colorAttachments[0].pixelFormat = view.colorPixelFormat
        do {
            pipeline = try device.makeRenderPipelineState(descriptor: descriptor)
            commandQueue = device.makeCommandQueue()
            NSLog("FNV Metal probe: native pipeline successfully created")
        } catch {
            NSLog("FNV Metal probe: pipeline creation error: \\(error)")
        }
    }

    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {
        // The procedural vertex shader uses normalized clip-space coordinates.
    }

    func draw(in view: MTKView) {
        guard let commandQueue = commandQueue,
              let pipeline = pipeline,
              let pass = view.currentRenderPassDescriptor,
              let drawable = view.currentDrawable,
              let commandBuffer = commandQueue.makeCommandBuffer(),
              let encoder = commandBuffer.makeRenderCommandEncoder(descriptor: pass) else {
            return
        }
        encoder.setRenderPipelineState(pipeline)
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)
        encoder.endEncoding()
        commandBuffer.present(drawable)
        commandBuffer.commit()
    }
}
