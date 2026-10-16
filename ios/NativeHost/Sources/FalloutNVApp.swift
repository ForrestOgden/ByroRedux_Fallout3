import Metal
import SwiftUI
import UniformTypeIdentifiers

@main
struct FalloutNVNativeApp: App {
    var body: some Scene {
        WindowGroup { FNVPreviewScreen() }
    }
}

/// User imports a small authentic .nif from Files. Assets stay on the iPhone.
private struct FNVPreviewScreen: View {
    @State private var filePickerVisible = false
    @State private var previewBytes: Data?
    @State private var previewRevision = 0
    @State private var message = "Showing procedural cube. Import a New Vegas .nif to test genuine geometry."
    @State private var assetName = "Procedural cube"
    private let gpu = MTLCreateSystemDefaultDevice()

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Text("Fallout: New Vegas — Native iOS")
                    .font(.title2.bold())
                Text("Rust NIF engine bridge ABI: \(fnv_ios_abi_version())")
                    .font(.system(.footnote, design: .monospaced))
                Text("ByroRedux BSA probe: \(bsaProbeResult) (1 = missing, expected)")
                    .font(.system(.footnote, design: .monospaced))
                Text("Native NIF mesh preview — lit orbit v4")
                    .font(.headline)
                MetalSceneView(nifData: previewBytes, revision: previewRevision)
                    .frame(height: 340)
                    .accessibilityLabel("3D native Metal preview of imported New Vegas NIF")
                Text("Drag to orbit · Pinch to zoom · Double-tap to reset")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Text("Preview source: \(assetName)")
                    .font(.footnote)
                Button("Import Fallout New Vegas .nif from Files") {
                    filePickerVisible = true
                }
                .buttonStyle(.borderedProminent)
                Text(message)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
                if let gpu {
                    Text("Metal GPU: \(gpu.name)")
                    Text("Hardware ray tracing: \(gpu.supportsRaytracing ? "supported" : "unavailable")")
                    Text("GPU family 8: \(gpu.supportsFamily(.apple8) ? "supported" : "unavailable")")
                }
                Text("Geometry diagnostic only; no textures, NPCs, quests or gameplay.")
                    .foregroundStyle(.secondary)
            }
            .padding()
        }
        .fileImporter(isPresented: $filePickerVisible, allowedContentTypes: [.data]) { result in
            switch result {
            case .failure(let error):
                message = "File picker failed: \(error.localizedDescription)"
            case .success(let url):
                importNif(url)
            }
        }
    }

    private var bsaProbeResult: Int32 {
        "/__falloutnv_ios_uninstalled__/absent.bsa".withCString { fnv_ios_probe_bsa($0) }
    }

    private func importNif(_ url: URL) {
        guard url.pathExtension.lowercased() == "nif" else {
            message = "Select a .nif mesh, not a BSA/ESM or ZIP."
            return
        }
        let granted = url.startAccessingSecurityScopedResource()
        defer { if granted { url.stopAccessingSecurityScopedResource() } }
        var mesh = FnvMesh(vertices: nil, count: 0)
        let code = url.path.withCString { fnv_ios_load_nif_mesh($0, &mesh) }
        guard code == 0, let vertices = mesh.vertices else {
            message = "ByroRedux NIF parse failed (error \(code)). Try another static mesh."
            return
        }
        defer { fnv_ios_release_mesh(vertices, mesh.count) }
        let stride = MemoryLayout<FnvVertex>.stride
        guard stride == 32, mesh.count > 0, mesh.count <= 180_000 else {
            message = "Unexpected NIF vertex layout or quantity."
            return
        }
        // Copy into Swift-owned Data BEFORE releasing the Rust-owned memory.
        previewBytes = Data(bytes: vertices, count: Int(mesh.count) * stride)
        previewRevision += 1
        assetName = url.lastPathComponent
        message = "Loaded \(mesh.count / 3) actual NIF triangles using native ByroRedux. Faceted preview shading from geometry; original textures not yet imported."
    }
}
