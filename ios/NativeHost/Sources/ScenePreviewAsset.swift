import Foundation

/// One independently decoded New Vegas NIF/DDS pair in the on-device scene.
/// Files from the owner's installed game remain on device and never enter GitHub.
struct ScenePreviewAsset: Identifiable {
    let id = UUID()
    let name: String
    let vertices: Data
    let texture: Data
    let width: Int
    let height: Int
    let triangleCount: Int

    enum ImportError: LocalizedError {
        case invalidPackage(String)
        case nifError(Int32)
        case textureError(Int32)
        var errorDescription: String? {
            switch self {
            case .invalidPackage(let reason): return reason
            case .nifError(let code): return "Native NIF parse returned \(code)"
            case .textureError(let code): return "Native DDS decode returned \(code)"
            }
        }
    }

    static func load(from url: URL) throws -> ScenePreviewAsset {
        guard url.pathExtension.lowercased() == "fnvpack" else {
            throw ImportError.invalidPackage("Choose a .fnvpack made by the local Windows extractor.")
        }
        let access = url.startAccessingSecurityScopedResource()
        defer { if access { url.stopAccessingSecurityScopedResource() } }
        let data = try Data(contentsOf: url, options: .mappedIfSafe)
        let maxNif = 24 * 1024 * 1024
        let maxDDS = 48 * 1024 * 1024
        guard data.count >= 16, data.count <= 16 + maxNif + maxDDS,
              Array(data.prefix(8)) == Array("FNVPK001".utf8) else {
            throw ImportError.invalidPackage("Invalid or oversized FNVPK001 package.")
        }
        let header = Array(data.prefix(16))
        func length(at offset: Int) -> Int {
            Int(UInt32(header[offset]) |
                UInt32(header[offset+1]) << 8 |
                UInt32(header[offset+2]) << 16 |
                UInt32(header[offset+3]) << 24)
        }
        let nifCount = length(at: 8)
        let ddsCount = length(at: 12)
        guard nifCount > 0, nifCount <= maxNif,
              ddsCount > 0, ddsCount <= maxDDS,
              16 + nifCount + ddsCount == data.count else {
            throw ImportError.invalidPackage("Package sizes or boundaries are invalid.")
        }
        let scratch = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: scratch, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: scratch) }
        let nif = scratch.appendingPathComponent("model.nif")
        let dds = scratch.appendingPathComponent("diffuse.dds")
        try data.subdata(in: 16..<(16+nifCount)).write(to: nif, options: .atomic)
        try data.subdata(in: (16+nifCount)..<data.count).write(to: dds, options: .atomic)

        var mesh = FnvMesh(vertices: nil, count: 0)
        let meshCode = nif.path.withCString { fnv_ios_load_nif_mesh($0, &mesh) }
        guard meshCode == 0, let vertexPtr = mesh.vertices else {
            throw ImportError.nifError(meshCode)
        }
        defer { fnv_ios_release_mesh(vertexPtr, mesh.count) }
        guard MemoryLayout<FnvVertex>.stride == 48,
              mesh.count >= 3, mesh.count <= 180_000,
              mesh.count % 3 == 0 else {
            throw ImportError.invalidPackage("NIF vertex format or count is invalid.")
        }
        let vertexData = Data(bytes: vertexPtr, count: mesh.count * 48)

        var decoded = FnvTexture(pixels: nil, len: 0, width: 0, height: 0)
        let textureCode = dds.path.withCString { fnv_ios_load_dds_rgba($0, &decoded) }
        guard textureCode == 0, let pixels = decoded.pixels else {
            throw ImportError.textureError(textureCode)
        }
        defer { fnv_ios_release_texture(pixels, decoded.len) }
        let w = Int(decoded.width)
        let h = Int(decoded.height)
        guard w > 0, h > 0, w <= 2048, h <= 2048, decoded.len == w * h * 4 else {
            throw ImportError.invalidPackage("Decoded DDS size invalid.")
        }
        return ScenePreviewAsset(name: url.lastPathComponent, vertices: vertexData,
                                 texture: Data(bytes: pixels, count: decoded.len),
                                 width: w, height: h, triangleCount: mesh.count / 3)
    }
}
