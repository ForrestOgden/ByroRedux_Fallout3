import Foundation
import simd

/// A bounded physics fallback built ONLY from actual ESM-placed NIF triangles.
/// This is not Havok's separate collision representation. It prevents basic
/// fall-through and wall clipping where visible geometry is available.
struct WalkTriangle {
    let a: SIMD3<Float>
    let b: SIMD3<Float>
    let c: SIMD3<Float>
    let normal: SIMD3<Float>

    init?(_ a: SIMD3<Float>, _ b: SIMD3<Float>, _ c: SIMD3<Float>) {
        let n = simd_cross(b - a, c - a)
        let length = simd_length(n)
        guard a.x.isFinite, a.y.isFinite, a.z.isFinite,
              b.x.isFinite, b.y.isFinite, b.z.isFinite,
              c.x.isFinite, c.y.isFinite, c.z.isFinite,
              length.isFinite, length > 0.000001,
              simd_length(a - b) < 50,
              simd_length(a - c) < 50,
              simd_length(b - c) < 50 else { return nil }
        self.a = a; self.b = b; self.c = c
        self.normal = n / length
    }

    var minimum: SIMD3<Float> { simd_min(a, simd_min(b, c)) }
    var maximum: SIMD3<Float> { simd_max(a, simd_max(b, c)) }

    /// Triangle containment in horizontal projection. This is also used to
    /// compute floor height by barycentric interpolation across the surface.
    func floorHeight(x: Float, z: Float) -> Float? {
        let v0 = SIMD2<Float>(b.x - a.x, b.z - a.z)
        let v1 = SIMD2<Float>(c.x - a.x, c.z - a.z)
        let v2 = SIMD2<Float>(x - a.x, z - a.z)
        let det = v0.x * v1.y - v0.y * v1.x
        guard abs(det) > 0.0000001 else { return nil }
        let w1 = (v2.x * v1.y - v2.y * v1.x) / det
        let w2 = (v0.x * v2.y - v0.y * v2.x) / det
        let w0 = 1 - w1 - w2
        guard w0 >= -0.001 && w1 >= -0.001 && w2 >= -0.001 else { return nil }
        return a.y * w0 + b.y * w1 + c.y * w2
    }

    /// 2D minimum distance from a point to any projected wall edge.
    /// Vertical mesh triangles project to a line, so the longest edge is
    /// sufficient for those surfaces without a full 3D capsule solver.
    func distanceToWall(x: Float, z: Float) -> Float {
        let p = SIMD2<Float>(x, z)
        let pairs = [(a,b),(b,c),(c,a)]
        var best = Float.infinity
        for (v0,v1) in pairs {
            let a2 = SIMD2<Float>(v0.x,v0.z)
            let b2 = SIMD2<Float>(v1.x,v1.z)
            let dir = b2-a2
            let lengthSquared = simd_length_squared(dir)
            if lengthSquared <= 0.0000001 { continue }
            let fraction = max(0, min(1, simd_dot(p-a2,dir)/lengthSquared))
            best = min(best, simd_length(p - (a2 + dir*fraction)))
        }
        return best
    }
}

/// Fast local X/Z lookup. Each bucket holds original authored triangles.
/// Cells have a hard per-triangle insertion bound so malformed geometry
/// cannot expand across millions of buckets.
struct WalkCollisionMap {
    struct Key: Hashable {
        let x: Int
        let z: Int
    }
    private let triangles: [WalkTriangle]
    private let tiles: [Key: [Int]]
    let triangleCount: Int
    let cellSize: Float

    init(triangles: [WalkTriangle], cellSize: Float = 2) {
        self.triangles = triangles
        self.triangleCount = triangles.count
        self.cellSize = cellSize
        var cells: [Key: [Int]] = [:]
        for (index, t) in triangles.enumerated() {
            let lo = t.minimum, hi = t.maximum
            let minX = Int(floor(lo.x/cellSize)), maxX = Int(floor(hi.x/cellSize))
            let minZ = Int(floor(lo.z/cellSize)), maxZ = Int(floor(hi.z/cellSize))
            guard maxX >= minX, maxZ >= minZ,
                  maxX-minX <= 24, maxZ-minZ <= 24 else { continue }
            for x in minX...maxX {
                for z in minZ...maxZ {
                    cells[Key(x:x,z:z),default:[]].append(index)
                }
            }
        }
        self.tiles = cells
    }

    private func nearby(_ x:Float, _ z:Float, radius:Float) -> Set<Int> {
        let minX=Int(floor((x-radius)/cellSize))
        let maxX=Int(floor((x+radius)/cellSize))
        let minZ=Int(floor((z-radius)/cellSize))
        let maxZ=Int(floor((z+radius)/cellSize))
        var result=Set<Int>()
        for tx in minX...maxX {
            for tz in minZ...maxZ {
                for idx in tiles[Key(x:tx,z:tz)] ?? [] { result.insert(idx) }
            }
        }
        return result
    }

    /// Return the highest nearby walkable polygon within a step/drop window.
    /// We do not mistake a ceiling above the player's head for a floor.
    func ground(x:Float, z:Float, nearY:Float,
                climb:Float=0.34, drop:Float=1.5) -> Float? {
        var height: Float?
        for i in nearby(x,z,radius:0.001) {
            let t=triangles[i]
            guard abs(t.normal.y) >= 0.65,
                  let level=t.floorHeight(x:x,z:z),
                  level <= nearY+climb,
                  level >= nearY-drop else { continue }
            height = max(height ?? -Float.infinity, level)
        }
        return height
    }

    /// Conservative vertical-wall check for a human-sized player cylinder.
    /// This is NOT complete Bethesda/Havok physics or proper stair stepping.
    func wallBlocks(x:Float,z:Float,feetY:Float,radius:Float=0.28) -> Bool {
        for i in nearby(x,z,radius:radius) {
            let t=triangles[i]
            guard abs(t.normal.y) < 0.48 else { continue }
            let lo=t.minimum.y,hi=t.maximum.y
            guard hi > feetY+0.20 && lo < feetY+1.72 else { continue }
            if t.distanceToWall(x:x,z:z) <= radius { return true }
        }
        return false
    }
}

/// Deterministic, fixed timestep capped walking with axis-sliding and
/// vertical gravity. Kept separate from Metal so movements can be tested.
final class WalkMotor {
    let collision: WalkCollisionMap
    private(set) var feet: SIMD3<Float>
    private(set) var verticalSpeed: Float = 0
    private(set) var grounded: Bool = false
    private var lastJumpRequested = false
    private let origin: SIMD3<Float>

    init(collision: WalkCollisionMap, origin: SIMD3<Float> = .zero) {
        self.collision = collision
        self.origin = origin
        self.feet = origin
        if let floor = collision.ground(x: origin.x,z: origin.z,
                                         nearY: origin.y, climb:2.5, drop:3.0) {
            feet.y = floor
            grounded = true
        }
    }

    func tick(move: SIMD3<Float>, dt:Float, jump:Bool=false) {
        guard dt.isFinite && dt > 0 else { return }
        let delta=max(0.001,min(0.05,dt))
        let horizontal=SIMD3<Float>(move.x,0,move.z)
        let len=simd_length(horizontal)
        // Caller supplies metres/second (e.g. 2.5 for walking); retain
        // actual speed rather than incorrectly normalizing it to 1 m/s.
        let capped = len > 6 ? horizontal * (6 / len) : horizontal
        let displacement = len > 0.001 ? capped * delta : SIMD3<Float>.zero

        // Compare both axes independently to permit sliding along a wall.
        let targetX=feet.x+displacement.x
        if !collision.wallBlocks(x:targetX,z:feet.z,feetY:feet.y) {
            feet.x=targetX
        }
        let targetZ=feet.z+displacement.z
        if !collision.wallBlocks(x:feet.x,z:targetZ,feetY:feet.y) {
            feet.z=targetZ
        }
        if jump && !lastJumpRequested && grounded {
            grounded=false
            verticalSpeed=4.4
        }
        lastJumpRequested=jump

        if grounded {
            if let floor=collision.ground(x:feet.x,z:feet.z,
                                          nearY:feet.y,climb:0.34,drop:0.50) {
                feet.y=floor
                verticalSpeed=0
            } else {
                grounded=false
            }
        }
        if !grounded {
            let previous=feet.y
            verticalSpeed=max(-22,verticalSpeed-9.81*delta)
            feet.y+=verticalSpeed*delta
            if verticalSpeed<=0,
               let floor=collision.ground(x:feet.x,z:feet.z,
                                          nearY:previous,climb:0.2,
                                          drop:max(0.5,previous-feet.y+0.25)),
               floor >= feet.y-0.02 {
                feet.y=floor
                verticalSpeed=0
                grounded=true
            }
        }
        // A broken spawn/missing floor must not strand the player below the
        // map forever. Return to actual authored origin, not a made-up CELL.
        if feet.y < origin.y-25 {
            feet=origin
            verticalSpeed=0
            grounded=false
            if let floor=collision.ground(x:feet.x,z:feet.z,
                                          nearY:feet.y,climb:2.5,drop:3) {
                feet.y=floor
                grounded=true
            }
        }
    }
}
