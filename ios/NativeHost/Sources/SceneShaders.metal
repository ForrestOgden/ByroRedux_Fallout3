#include <metal_stdlib>
using namespace metal;

// Rust FnvVertex and Swift SceneVertex use three aligned float4 fields
// (48-byte stride): preserving exact cross-language ABI is mandatory.
struct SceneVertex {
    float4 position;
    float4 color;
    float4 uvLight;
};
struct SceneUniforms { float4x4 mvp; };
struct SceneOutput {
    float4 position [[position]];
    float3 color;
    float2 uv;
    float light;
    float hasUV;
};
vertex SceneOutput fnv_scene_vertex(const device SceneVertex *v [[buffer(0)]],
                                   constant SceneUniforms &u [[buffer(1)]],
                                   uint id [[vertex_id]]) {
    SceneOutput o;
    o.position = u.mvp * v[id].position;
    o.color = v[id].color.xyz;
    o.uv = v[id].uvLight.xy;
    o.light = v[id].uvLight.z;
    o.hasUV = v[id].uvLight.w;
    return o;
}
fragment float4 fnv_scene_fragment(
    SceneOutput v [[stage_in]],
    texture2d<float, access::sample> diffuse [[texture(0)]],
    constant uint &textureEnabled [[buffer(0)]]) {
    constexpr sampler textureSampler(coord::normalized, address::repeat,
                                     filter::linear, mip_filter::none);
    if (textureEnabled == 0 || v.hasUV < 0.5) {
        return float4(v.color, 1.0);
    }
    float4 sample = diffuse.sample(textureSampler, v.uv);
    // The texture is real; lighting still uses per-face diagnostic brightness.
    if (sample.a < 0.5) { discard_fragment(); }
    return float4(sample.rgb * v.light, 1.0);
}
