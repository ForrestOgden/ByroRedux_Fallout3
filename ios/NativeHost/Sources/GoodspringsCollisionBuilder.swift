import Foundation
import simd

enum GoodspringsCollisionBuilder {
    /// Extract source triangle soup from each original game NIF once and
    /// apply true ESM REFR placements. The collision grid is local-only and
    /// in meters centered at the authored Goodsprings spawn marker.
    /// Prevent gigantic/unbounded build work on large corrupted meshes.
    static func build(models: [GoodspringsWorld.Model?],
                      instances: [GoodspringsWorld.Instance]) -> WalkCollisionMap {
        let maxTriangles = 120_000
        let spawnRadius: Float = 75
        var worldTriangles: [WalkTriangle] = []
        worldTriangles.reserveCapacity(min(maxTriangles, 35_000))

        // Close instances first, to protect the walkable start area if the
        // scene contains more geometry than the mobile collision budget.
        let placements=instances.sorted {
            simd_length_squared($0.position) < simd_length_squared($1.position)
        }
        for placed in placements {
            if worldTriangles.count >= maxTriangles { break }
            if simd_length(placed.position) > spawnRadius { continue }
            guard models.indices.contains(placed.modelIndex),
                  let model=models[placed.modelIndex] else { continue }
            let buffer=model.vertexData
            if buffer.isEmpty || buffer.count%144 != 0 { continue }
            let scale=placed.scale
            let origin=placed.position
            let rotation=placed.rotation
            // Preserve original triangle grouping and positions. Never
            // normalize the mesh as the previous stand-alone viewer did.
            buffer.withUnsafeBytes { raw in
                func vertex(_ index:Int) -> SIMD3<Float> {
                    let start=index*48
                    let x=raw.loadUnaligned(fromByteOffset:start,as:Float.self)
                    let y=raw.loadUnaligned(fromByteOffset:start+4,as:Float.self)
                    let z=raw.loadUnaligned(fromByteOffset:start+8,as:Float.self)
                    return origin + rotation.act(SIMD3<Float>(x,y,z)*scale)
                }
                let triCount=buffer.count/144
                for i in 0..<triCount {
                    if worldTriangles.count >= maxTriangles { break }
                    let a=vertex(i*3)
                    let b=vertex(i*3+1)
                    let c=vertex(i*3+2)
                    // Culling is a conservative collision-only distance
                    // gate. Does not alter the renderable authored geometry.
                    if a.x < -spawnRadius-5 && b.x < -spawnRadius-5 && c.x < -spawnRadius-5 { continue }
                    if a.x > spawnRadius+5 && b.x > spawnRadius+5 && c.x > spawnRadius+5 { continue }
                    if a.z < -spawnRadius-5 && b.z < -spawnRadius-5 && c.z < -spawnRadius-5 { continue }
                    if a.z > spawnRadius+5 && b.z > spawnRadius+5 && c.z > spawnRadius+5 { continue }
                    if let triangle=WalkTriangle(a,b,c) {
                        // Ignore tiny clutter slivers below 2 cm² area.
                        let area=simd_length(simd_cross(b-a,c-a))*0.5
                        if area>=0.0002 {worldTriangles.append(triangle)}
                    }
                }
            }
        }
        NSLog("Goodsprings collision: %d authentic ESM-placed triangles indexed",worldTriangles.count)
        return WalkCollisionMap(triangles:worldTriangles)
    }
}
