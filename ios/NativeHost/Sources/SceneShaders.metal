#include <metal_stdlib>
using namespace metal;
// The Swift buffer contains two SIMD4<Float> lanes (32-byte stride).
// Never use packed_float3 here: that is 24 bytes per vertex, not 32.
struct SceneVertex { float4 position; float4 color; };
struct SceneUniforms { float4x4 mvp; };
struct SceneOutput { float4 position [[position]]; float3 color; };
vertex SceneOutput fnv_scene_vertex(const device SceneVertex *v [[buffer(0)]],
                                   constant SceneUniforms &u [[buffer(1)]],
                                   uint id [[vertex_id]]) {
    SceneOutput o;
    o.position = u.mvp * v[id].position;
    o.color = v[id].color.xyz;
    return o;
}
fragment float4 fnv_scene_fragment(SceneOutput v [[stage_in]]) {
    return float4(v.color, 1.0);
}
