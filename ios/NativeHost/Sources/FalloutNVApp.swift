import Metal
import SwiftUI

/// Minimal native iOS host for the real Rust ByroRedux archive-reader bridge.
/// This is a GPU capabilities diagnostic, NOT a playable Fallout: New Vegas game.
@main
struct FalloutNVNativeApp: App {
    private let metalDevice = MTLCreateSystemDefaultDevice()

    // Force-link and exercise the real BSA reader without shipping game files.
    private var bsaProbeResult: Int32 {
        "/__falloutnv_ios_uninstalled__/absent.bsa".withCString { fnv_ios_probe_bsa($0) }
    }

    var body: some Scene {
        WindowGroup {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Text("Fallout: New Vegas — Native iOS")
                        .font(.title2.bold())
                    Text("Rust archive bridge ABI: \(fnv_ios_abi_version())")
                        .font(.system(.body, design: .monospaced))
                    Text("ByroRedux BSA probe: \(bsaProbeResult) (1 = missing, expected)")
                        .font(.system(.footnote, design: .monospaced))
                    if let device = metalDevice {
                        Text("Metal GPU: \(device.name)")
                        Text("Hardware ray tracing: \(device.supportsRaytracing ? "supported" : "unavailable")")
                        Text("Apple GPU family 8: \(device.supportsFamily(.apple8) ? "supported" : "unavailable")")
                        Text("Max compute threads: \(device.maxThreadsPerThreadgroup.width) × \(device.maxThreadsPerThreadgroup.height) × \(device.maxThreadsPerThreadgroup.depth)")
                    } else {
                        Text("Metal GPU: unavailable")
                    }
                    Text("Diagnostic only. No Fallout: New Vegas renderer or gameplay yet.")
                        .foregroundStyle(.secondary)
                }
                .padding()
                .textSelection(.enabled)
            }
        }
    }
}
