import Metal
import SwiftUI

/// Minimal native iOS host for the real Rust ByroRedux archive-reader bridge.
/// This is a GPU capabilities diagnostic, NOT a playable Fallout 3 game.
@main
struct Fallout3NativeApp: App {
    private let metalDevice = MTLCreateSystemDefaultDevice()

    var body: some Scene {
        WindowGroup {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Text("Fallout 3 — Native iOS")
                        .font(.title2.bold())
                    Text("Rust archive bridge ABI: \(fo3_ios_abi_version())")
                        .font(.system(.body, design: .monospaced))
                    if let device = metalDevice {
                        Text("Metal GPU: \(device.name)")
                        Text("Hardware ray tracing: \(device.supportsRaytracing ? "supported" : "unavailable")")
                        Text("Apple GPU family 8: \(device.supportsFamily(.apple8) ? "supported" : "unavailable")")
                        Text("Max compute threads: \(device.maxThreadsPerThreadgroup.width) × \(device.maxThreadsPerThreadgroup.height) × \(device.maxThreadsPerThreadgroup.depth)")
                    } else {
                        Text("Metal GPU: unavailable")
                    }
                    Text("Diagnostic only. No Fallout 3 renderer or gameplay yet.")
                        .foregroundStyle(.secondary)
                }
                .padding()
                .textSelection(.enabled)
            }
        }
    }
}
