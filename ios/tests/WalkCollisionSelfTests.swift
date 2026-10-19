import Foundation
import simd

@main
struct WalkCollisionSelfTests {
    static func main() {
        let floorGeometry: [WalkTriangle] = [
            WalkTriangle(SIMD3<Float>(-5,0,-5), SIMD3<Float>(5,0,-5),
                         SIMD3<Float>(-5,0,5))!,
            WalkTriangle(SIMD3<Float>(5,0,-5), SIMD3<Float>(5,0,5),
                         SIMD3<Float>(-5,0,5))!
        ]
        let wallGeometry: [WalkTriangle] = [
            WalkTriangle(SIMD3<Float>(1,0,-2),SIMD3<Float>(1,2,-2),
                         SIMD3<Float>(1,0,2))!,
            WalkTriangle(SIMD3<Float>(1,2,-2),SIMD3<Float>(1,2,2),
                         SIMD3<Float>(1,0,2))!
        ]
        let map=WalkCollisionMap(triangles:floorGeometry+wallGeometry)
        precondition(map.triangleCount==4)
        precondition(abs((map.ground(x:0,z:0,nearY:0) ?? -99) - 0) < 0.001,
                     "Floor ray should hit source triangle")
        precondition(map.ground(x:10,z:0,nearY:0)==nil,
                     "No invented floor outside authored room")
        precondition(!map.wallBlocks(x:0,z:0,feetY:0),
                     "Walking space should remain clear")
        precondition(map.wallBlocks(x:0.9,z:0,feetY:0),
                     "Vertical triangle blocks capsule near wall")
        let player=WalkMotor(collision:map)
        precondition(player.grounded,"Player begins on authored floor")
        for _ in 0..<25 {
            player.tick(move:SIMD3<Float>(2,0,0),dt:0.05)
        }
        precondition(player.feet.x < 0.83,
                     "Wall must stop movement instead of clipping through")
        precondition(abs(player.feet.y) < 0.01,
                     "Walking on a floor must remain grounded")
        let y0=player.feet.y
        player.tick(move:.zero,dt:0.016,jump:true)
        precondition(player.feet.y > y0 && !player.grounded,
                     "Jump should lift feet above actual floor")
        for _ in 0..<220 {
            player.tick(move:.zero,dt:0.016,jump:false)
        }
        precondition(player.grounded && abs(player.feet.y-y0)<0.001,
                     "Gravity must return player to authored floor")
        let absent=WalkCollisionMap(triangles:[])
        precondition(absent.ground(x:0,z:0,nearY:0)==nil,
                     "Blank scene never manufactures ground")
        print("Goodsprings walking collision: floor, wall, jump, landing and absent-floor checks PASS")
    }
}
