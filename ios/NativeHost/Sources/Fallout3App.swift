import SwiftUI

/// Minimal native iOS host for the real Rust ByroRedux archive-reader bridge.
/// No Fallout 3 world, renderer, audio, or gameplay is wired up yet.
@main
struct Fallout3NativeApp: App {
    var body: some Scene {
        WindowGroup {
            VStack(spacing: 20) {
                Text("Fallout 3 — Native iOS")
                    .font(.title2.bold())
                Text("Rust archive bridge ABI: \(fo3_ios_abi_version())")
                    .font(.system(.body, design: .monospaced))
                Text("Native ARM64 host proof-of-concept. Not a playable game.")
                    .multilineTextAlignment(.center)
                    .foregroundStyle(.secondary)
            }
            .padding()
        }
    }
}
