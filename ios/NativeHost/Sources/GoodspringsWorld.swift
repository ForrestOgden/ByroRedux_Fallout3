import Foundation
import simd

/// Local-only world assembled from authentic FalloutNV.esm references and
/// installed NIF/DDS archives by the Windows exporter. Source formats are
/// re-parsed by the same Rust ARM64 bridge used for the verified model viewer.
struct GoodspringsWorld {
    struct MaterialTexture {
        let pixels: Data
        let width: Int
        let height: Int
    }
    struct DrawBatch {
        let start: Int
        let count: Int
        let materialSlot: Int // -1 means untextured, else 0-based diffuse slot
        let blendMode: Int // 0=opaque/cutout, 1=source-alpha, 2=additive
    }
    struct Model {
        let sourcePath: String
        let vertexData: Data
        let triangleCount: Int
        let textures: [MaterialTexture?]
        let batches: [DrawBatch]
    }
    struct Instance {
        let modelIndex: Int
        let position: SIMD3<Float> // authored REFR relative to spawn, meters
        let rotation: simd_quatf  // Bethesda Z-up converted by ByroRedux
        let scale: Float          // includes 1 / 70 world-unit conversion
        let formID: UInt32
    }
    let loadID: UUID // forces a new Metal view for every successfully selected world
    let cellName: String
    let formatVersion: UInt32 // visible proof that the material-complete v3 file is loaded
    let initialYaw: Float // authored COCMarkerHeading, or fallback facing
    let models: [Model?] // preserves index if a single NIF fails to decode
    let instances: [Instance]
    let usableModelCount: Int

    enum WorldError: LocalizedError {
        case invalid(String)
        var errorDescription: String? {
            if case let .invalid(reason) = self { return reason }
            return "Invalid New Vegas world scene"
        }
    }

    static func load(from url: URL) throws -> GoodspringsWorld {
        guard url.pathExtension.lowercased() == "fnvworld" else {
            throw WorldError.invalid("Choose the .fnvworld file from your own FalloutNV.esm export.")
        }
        let permission = url.startAccessingSecurityScopedResource()
        defer { if permission { url.stopAccessingSecurityScopedResource() } }
        let payload = try Data(contentsOf: url, options: .mappedIfSafe)
        guard payload.count >= 22 && payload.count <= 320 * 1024 * 1024 else {
            throw WorldError.invalid("World file empty or exceeds the 320 MiB mobile import cap.")
        }
        var cursor = 0
        func take(_ length: Int) throws -> Data {
            guard length >= 0, length <= payload.count - cursor else {
                throw WorldError.invalid("Truncated New Vegas world file.")
            }
            defer { cursor += length }
            return payload.subdata(in: cursor..<(cursor + length))
        }
        func u16() throws -> Int {
            let b = Array(try take(2))
            return Int(UInt16(b[0]) | UInt16(b[1]) << 8)
        }
        func u32() throws -> UInt32 {
            let b = Array(try take(4))
            return UInt32(b[0]) | UInt32(b[1]) << 8 |
                UInt32(b[2]) << 16 | UInt32(b[3]) << 24
        }
        func float32() throws -> Float {
            Float(bitPattern: try u32())
        }
        func label(_ cap: Int) throws -> String {
            let n = try u16()
            guard n > 0 && n < cap,
                  let name = String(data: try take(n), encoding: .utf8) else {
                throw WorldError.invalid("Invalid world source identifier.")
            }
            return name
        }

        guard String(data: try take(8), encoding: .ascii) == "FNVWLD01" else {
            throw WorldError.invalid("Invalid FNVWLD01 world-package signature.")
        }
        let version = try u32()
        guard version == 2 || version == 3 else {
            throw WorldError.invalid("Unsupported FNVWLD01 world-package version (header v\(version)).")
        }
        // Renaming the old file to '-v3' does NOT upgrade its binary format.
        // Fail before decoding models so users cannot mistake a recycled v2
        // package for an export with complete authored material slots.
        if url.lastPathComponent.lowercased().contains("-v3") && version != 3 {
            throw WorldError.invalid("\(url.lastPathComponent) contains a legacy v\(version) header. The file was made by an old Windows exporter (or merely renamed). Rebuild using the new downloaded Windows exporter.")
        }
        let modelCount = Int(try u32())
        let instanceCount = Int(try u32())
        guard modelCount >= 5 && modelCount <= 160,
              instanceCount >= 10 && instanceCount <= 480 else {
            throw WorldError.invalid("World contains unsupported model/instance counts.")
        }
        let cellName = try label(512)
        let initialYaw = try float32()
        guard initialYaw.isFinite, abs(initialYaw) <= 4 * .pi else {
            throw WorldError.invalid("Invalid authored Goodsprings spawn orientation.")
        }
        let fm = FileManager.default
        let scratch = fm.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try fm.createDirectory(at: scratch, withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: scratch) }
        let meshURL = scratch.appendingPathComponent("mesh.nif")
        let textureURL = scratch.appendingPathComponent("diffuse.dds")

        var models: [Model?] = []
        models.reserveCapacity(modelCount)
        var validModels = 0
        var decodedTextureBytes = 0
        let decodedTextureBudget = 384 * 1024 * 1024
        for i in 0..<modelCount {
            let name = try label(1024)
            let nifLength = Int(try u32())
            let oldDDSLength: Int
            let textureCount: Int
            if version == 2 {
                oldDDSLength = Int(try u32())
                textureCount = 1
            } else {
                oldDDSLength = 0
                textureCount = try u16()
            }
            guard nifLength > 0 && nifLength <= 24 * 1024 * 1024,
                  textureCount >= 0 && textureCount <= 64,
                  oldDDSLength >= 0 && oldDDSLength <= 48 * 1024 * 1024 else {
                throw WorldError.invalid("Invalid NIF/material lengths in model \(i).")
            }
            let nif = try take(nifLength)
            var ddsPayloads: [Data] = []
            ddsPayloads.reserveCapacity(textureCount)
            for _ in 0..<textureCount {
                let length = version == 2 ? oldDDSLength : Int(try u32())
                guard length >= 0 && length <= 48 * 1024 * 1024 else {
                    throw WorldError.invalid("Invalid diffuse texture length for model \(i).")
                }
                ddsPayloads.append(try take(length))
            }
            try nif.write(to: meshURL, options: .atomic)
            var mesh = FnvMesh(vertices: nil, count: 0)
            let status = meshURL.path.withCString {
                fnv_ios_load_nif_mesh_raw($0, &mesh)
            }
            guard status == 0, let raw = mesh.vertices,
                  mesh.count >= 3, mesh.count <= 180_000,
                  mesh.count % 3 == 0,
                  MemoryLayout<FnvVertex>.stride == 48 else {
                if let raw = mesh.vertices { fnv_ios_release_mesh(raw, mesh.count) }
                models.append(nil)
                NSLog("World model %d failed native NIF import: %@", i, name)
                continue
            }
            let vertexData = Data(bytes: raw, count: Int(mesh.count) * 48)
            let triangleCount = Int(mesh.count) / 3
            fnv_ios_release_mesh(raw, mesh.count)

            var textures: [MaterialTexture?] = []
            textures.reserveCapacity(ddsPayloads.count)
            for dds in ddsPayloads {
                guard !dds.isEmpty else { textures.append(nil); continue }
                try dds.write(to: textureURL, options: .atomic)
                var image = FnvTexture(pixels: nil, len: 0, width: 0, height: 0)
                let result = textureURL.path.withCString {
                    fnv_ios_load_dds_rgba($0, &image)
                }
                var decoded: MaterialTexture? = nil
                if result == 0, let ptr = image.pixels {
                    let w = Int(image.width), h = Int(image.height)
                    if w > 0 && h > 0 && w <= 2048 && h <= 2048,
                       image.len == w * h * 4,
                       decodedTextureBytes + image.len <= decodedTextureBudget {
                        decoded = MaterialTexture(pixels: Data(bytes: ptr, count: image.len),
                                                  width: w, height: h)
                        decodedTextureBytes += image.len
                    }
                    fnv_ios_release_texture(ptr, image.len)
                }
                textures.append(decoded)
            }
            // The Rust importer orders triangles by their authored diffuse
            // slot. Read the final float of each 48-byte vertex, one triangle
            // at a time, to identify precise contiguous draw ranges.
            var batches: [DrawBatch] = []
            vertexData.withUnsafeBytes { raw in
                var batchStart = 0
                var currentSlot = -2
                var currentBlend = -1
                for triangleStart in stride(from: 0, to: Int(mesh.count), by: 3) {
                    let tag = raw.loadUnaligned(fromByteOffset: triangleStart * 48 + 44,
                                                as: Float.self)
                    let base = tag.isFinite && tag >= 0 ? Int(tag) : 0
                    let slot = base >= 1 && base <= 64 ? base - 1 : -1
                    let fraction = tag.isFinite ? tag - Float(base) : 0
                    let blend = fraction > 0.375 ? 2 : (fraction > 0.125 ? 1 : 0)
                    if slot != currentSlot || blend != currentBlend {
                        if triangleStart > batchStart {
                            batches.append(DrawBatch(start: batchStart,
                                                     count: triangleStart - batchStart,
                                                     materialSlot: currentSlot,
                                                     blendMode: currentBlend))
                        }
                        batchStart = triangleStart
                        currentSlot = slot
                        currentBlend = blend
                    }
                }
                if batchStart < Int(mesh.count) {
                    batches.append(DrawBatch(start: batchStart,
                                             count: Int(mesh.count) - batchStart,
                                             materialSlot: currentSlot,
                                             blendMode: currentBlend))
                }
            }
            validModels += 1
            models.append(Model(sourcePath: name, vertexData: vertexData,
                                triangleCount: triangleCount, textures: textures,
                                batches: batches))
        }
        var instances: [Instance] = []
        instances.reserveCapacity(instanceCount)
        for _ in 0..<instanceCount {
            let index = Int(try u32())
            let p = SIMD3<Float>(try float32(), try float32(), try float32())
            let q = SIMD4<Float>(try float32(), try float32(), try float32(), try float32())
            let scale = try float32()
            let fid = try u32()
            guard index >= 0 && index < modelCount, p.x.isFinite, p.y.isFinite, p.z.isFinite,
                  q.x.isFinite, q.y.isFinite, q.z.isFinite, q.w.isFinite,
                  scale.isFinite, scale > 0 && scale < 100 else {
                throw WorldError.invalid("Invalid REFR placement in world scene.")
            }
            // A single unsupported NIF is omitted, not turned into fake world geometry.
            guard models[index] != nil else { continue }
            let quaternion = simd_quatf(ix: q.x, iy: q.y, iz: q.z, r: q.w)
            let len = simd_length(quaternion.vector)
            guard len > 0.01 && len.isFinite else { continue }
            instances.append(Instance(modelIndex: index, position: p,
                                      rotation: simd_normalize(quaternion), scale: scale, formID: fid))
        }
        guard cursor == payload.count, validModels >= 5, instances.count >= 10 else {
            throw WorldError.invalid("World import incomplete; too few valid placed New Vegas meshes.")
        }
        return GoodspringsWorld(loadID: UUID(), cellName: cellName, formatVersion: version, initialYaw: initialYaw,
                               models: models, instances: instances,
                               usableModelCount: validModels)
    }
}
