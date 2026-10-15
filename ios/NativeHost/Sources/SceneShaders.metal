#include <metal_stdlib>
using namespace metal;
struct SceneVertex { packed_float3 position; packed_float3 color; };
struct SceneUniforms { float4x4 mvp; };
struct SceneOutput { float4 position [[position]]; float3 color; };
vertex SceneOutput fnv_scene_vertex(const device SceneVertex *v [[buffer(0)]],
                                   constant SceneUniforms &u [[buffer(1)]],
                                   uint id [[vertex_id]]) {
    SceneOutput o;
    o.position = u.mvp * float4(float3(v[id].position), 1.0);
    o.color = float3(v[id].color);
    return o;
}
fragment float4 fnv_scene_fragment(SceneOutput v [[stage_in]]) {
    return float4(v.color, 1.0);
}
