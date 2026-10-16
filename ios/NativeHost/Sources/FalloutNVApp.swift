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
    // Keep a single SwiftUI file importer: two .fileImporter modifiers on the
    // same view conflicted on iOS and the NIF Files sheet never appeared.
    private enum ImportKind { case nif, dds }
    @State private var importKind: ImportKind = .nif
    @State private var filePickerVisible = false
    @State private var textureData: Data?
    @State private var textureWidth = 0
    @State private var textureHeight = 0
    @State private var textureRevision = 0
    @State private var textureName = "No DDS texture imported"
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
                Text("Native NIF mesh preview — DDS textures v5.1 (picker fix)")
                    .font(.headline)
                MetalSceneView(
                    nifData: previewBytes, revision: previewRevision,
                    textureData: textureData, textureWidth: textureWidth,
                    textureHeight: textureHeight, textureRevision: textureRevision)
                    .frame(height: 340)
                    .accessibilityLabel("3D native Metal preview of imported New Vegas NIF")
                Text("Drag to orbit · Pinch to zoom · Double-tap to reset")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Text("Preview source: \(assetName)")
                Text("Diffuse texture: \(textureName)")
                    .font(.footnote)
                Button("Import Fallout New Vegas .nif from Files") {
                    importKind = .nif
                    filePickerVisible = true
                }
                .buttonStyle(.borderedProminent)
                Button("Import matching Fallout New Vegas .dds from Files") {
                    importKind = .dds
                    filePickerVisible = true
                }
                .buttonStyle(.bordered)
                .disabled(previewBytes == nil)
                Text(message)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
                if let gpu {
                    Text("Metal GPU: \(gpu.name)")
                    Text("Hardware ray tracing: \(gpu.supportsRaytracing ? "supported" : "unavailable")")
                    Text("GPU family 8: \(gpu.supportsFamily(.apple8) ? "supported" : "unavailable")")
                }
                Text("NIF/DDS preview only; no full New Vegas materials, NPCs, quests or gameplay.")
                    .foregroundStyle(.secondary)
            }
            .padding()
        }
        // Both file types share the *same* presentation modifier.
        // The previously verified Phase 6/7 NIF picker also used one importer.
        .fileImporter(isPresented: $filePickerVisible, allowedContentTypes: [.data]) { result in
            switch result {
            case .failure(let error):
                message = "Files picker failed: \(error.localizedDescription)"
            case .success(let url):
                switch importKind {
                case .nif: importNif(url)
                case .dds: importDDS(url)
                }
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
        guard stride == 48, mesh.count > 0, mesh.count <= 180_000 else {
            message = "Unexpected NIF vertex layout or quantity."
            return
        }
        // Copy into Swift-owned Data BEFORE releasing the Rust-owned memory.
        previewBytes = Data(bytes: vertices, count: Int(mesh.count) * stride)
        // A different mesh must never inherit the previous mesh's texture.
        textureData = nil
        textureWidth = 0
        textureHeight = 0
        textureName = "No DDS texture imported"
        textureRevision += 1
        previewRevision += 1
        assetName = url.lastPathComponent
        message = "Loaded \(mesh.count / 3) actual NIF triangles using native ByroRedux. Faceted preview shading from geometry; original textures not yet imported."
    }

    private func importDDS(_ url: URL) {
        guard url.pathExtension.lowercased() == "dds" else {
            message = "Select a .dds file, not a NIF or BSA archive."
            return
        }
        let granted = url.startAccessingSecurityScopedResource()
        defer { if granted { url.stopAccessingSecurityScopedResource() } }
        var texture = FnvTexture(pixels: nil, len: 0, width: 0, height: 0)
        let code = url.path.withCString { fnv_ios_load_dds_rgba($0, &texture) }
        guard code == 0, let pixels = texture.pixels else {
            message = "DDS decode failed (error \(code)). Only BC1/DXT1, BC2/DXT3, BC3/DXT5 or 32-bit RGB DDS supported."
            return
        }
        defer { fnv_ios_release_texture(pixels, texture.len) }
        let w = Int(texture.width)
        let h = Int(texture.height)
        guard w > 0, h > 0, w <= 2048, h <= 2048,
              texture.len == w * h * 4 else {
            message = "Decoded DDS dimensions or RGBA size invalid."
            return
        }
        textureData = Data(bytes: pixels, count: texture.len)
        textureWidth = w
        textureHeight = h
        textureRevision += 1
        textureName = url.lastPathComponent
        message = "Imported \(w) x \(h) DDS texture for native Metal UV preview. Texture is supplied independently; ensure it belongs to this mesh."
    }
}
