import Foundation
import simd

/// Local-only world assembled from authentic FalloutNV.esm references and
/// installed NIF/DDS archives by the Windows exporter. Source formats are
/// re-parsed by the same Rust ARM64 bridge used for the verified model viewer.
struct GoodspringsWorld {
    struct Texture {
        let pixels: Data
        let width: Int
        let height: Int
    }
    struct Batch {
        let start: Int
        let count: Int
        let materialIndex: Int
    }
    struct Model {
        let sourcePath: String
        let vertexData: Data
        let triangleCount: Int
        let batches: [Batch]
        /// Index is the original NIF ImportedMesh ordinal. Nil means no
        /// supported authored DDS; never borrow a neighboring material.
        let materials: [Texture?]
    }
    struct Instance {
        let modelIndex: Int
        let position: SIMD3<Float> // authored REFR relative to spawn, meters
        let rotation: simd_quatf  // Bethesda Z-up converted by ByroRedux
        let scale: Float          // includes 1 / 70 world-unit conversion
        let formID: UInt32
    }
    let cellName: String
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
        guard payload.count >= 22 && payload.count <= 256 * 1024 * 1024 else {
            throw WorldError.invalid("World file empty or exceeds the 256 MiB mobile import cap.")
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

        let magic = String(data: try take(8), encoding: .ascii)
        let version = try u32()
        guard magic == "FNVWLD01", version == 2 || version == 3 else {
            throw WorldError.invalid("Incompatible FNVWLD01 world-package format (expected v2 or v3).")
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
        for i in 0..<modelCount {
            let name = try label(1024)
            let nifLength = Int(try u32())
            let materialCount: Int
            let legacyDDSLength: Int
            if version == 3 {
                materialCount = try u16()
                legacyDDSLength = 0
            } else {
                legacyDDSLength = Int(try u32())
                materialCount = 1
            }
            guard nifLength > 0 && nifLength <= 24 * 1024 * 1024,
                  materialCount > 0 && materialCount <= 256,
                  legacyDDSLength <= 48 * 1024 * 1024 else {
                throw WorldError.invalid("Invalid NIF/material length for original mesh \\(i).")
            }
            let nif = try take(nifLength)
            var ddsPayloads: [Data] = []
            ddsPayloads.reserveCapacity(materialCount)
            for _ in 0..<materialCount {
                let length = version == 3 ? Int(try u32()) : legacyDDSLength
                guard length >= 0 && length <= 48 * 1024 * 1024 else {
                    throw WorldError.invalid("Oversized authored DDS in mesh \\(i).")
                }
                ddsPayloads.append(try take(length))
            }

            try nif.write(to: meshURL, options: .atomic)
            var mesh = FnvWorldMesh(vertices: nil, count: 0, batches: nil,
                                    batch_count: 0, mesh_count: 0)
            let status = meshURL.path.withCString {
                fnv_ios_load_nif_mesh_world($0, &mesh)
            }
            guard status == 0 else {
                models.append(nil)
                NSLog("World model %d failed NIF batch import: %@", i, name)
                continue
            }
            defer {
                fnv_ios_release_world_mesh(mesh.vertices, mesh.count,
                                           mesh.batches, mesh.batch_count)
            }
            guard let raw = mesh.vertices, let rawBatches = mesh.batches,
                  mesh.count >= 3, mesh.count <= 180_000,
                  mesh.count % 3 == 0,
                  mesh.batch_count > 0, mesh.batch_count <= 256,
                  MemoryLayout<FnvVertex>.stride == 48,
                  // Never apply a differently ordered material manifest.
                  version == 2 || mesh.mesh_count == materialCount else {
                models.append(nil)
                NSLog("World model %d rejected: corrupt vertex/batch or material mapping: %@", i, name)
                continue
            }

            var batches: [Batch] = []
            var validRanges = true
            var nextStart = 0
            for j in 0..<Int(mesh.batch_count) {
                let batch = rawBatches[j]
                let start = Int(batch.start)
                let count = Int(batch.count)
                let materialIndex = version == 2 ? 0 : Int(batch.material_index)
                if start != nextStart || count < 3 || count % 3 != 0 ||
                   start > Int(mesh.count) || count > Int(mesh.count) - start ||
                   materialIndex >= materialCount {
                    validRanges = false
                    break
                }
                batches.append(Batch(start: start, count: count, materialIndex: materialIndex))
                nextStart = start + count
            }
            guard validRanges && nextStart == Int(mesh.count) else {
                models.append(nil)
                NSLog("World model %d rejected: invalid material draw ranges: %@", i, name)
                continue
            }

            let vertexData = Data(bytes: raw, count: Int(mesh.count) * 48)
            let triangleCount = Int(mesh.count) / 3
            var materials: [Texture?] = []
            materials.reserveCapacity(materialCount)
            for dds in ddsPayloads {
                guard !dds.isEmpty else {
                    materials.append(nil)
                    continue
                }
                try dds.write(to: textureURL, options: .atomic)
                var image = FnvTexture(pixels: nil, len: 0, width: 0, height: 0)
                let result = textureURL.path.withCString {
                    fnv_ios_load_dds_rgba($0, &image)
                }
                var decoded: Texture? = nil
                if result == 0, let ptr = image.pixels {
                    let w = Int(image.width), h = Int(image.height)
                    if w > 0 && h > 0 && w <= 2048 && h <= 2048,
                       image.len == w * h * 4,
                       image.len <= 256 * 1024 * 1024 - decodedTextureBytes {
                        decoded = Texture(pixels: Data(bytes: ptr, count: image.len),
                                          width: w, height: h)
                        decodedTextureBytes += image.len
                    }
                    fnv_ios_release_texture(ptr, image.len)
                }
                materials.append(decoded)
            }
            validModels += 1
            models.append(Model(sourcePath: name, vertexData: vertexData,
                                triangleCount: triangleCount, batches: batches,
                                materials: materials))
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
        return GoodspringsWorld(cellName: cellName, initialYaw: initialYaw,
                               models: models, instances: instances,
                               usableModelCount: validModels)
    }
}
