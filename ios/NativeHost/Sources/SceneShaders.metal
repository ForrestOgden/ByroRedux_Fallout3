#include <metal_stdlib>
using namespace metal;

// Shared 48-byte FFI layout with Rust FnvVertex and Swift FnvVertex.
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
    float opacity;
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
    o.opacity = v[id].color.w;
    // Rust encodes the diffuse slot in the integer part of this lane and
    // alpha/additive state in the fractional part; 0.xx is untextured.
    o.hasUV = floor(v[id].uvLight.w + 0.0001);
    return o;
}
fragment float4 fnv_scene_fragment(
    SceneOutput v [[stage_in]],
    texture2d<float, access::sample> diffuse [[texture(0)]],
    constant uint &textureEnabled [[buffer(0)]],
    constant uint &blendEnabled [[buffer(1)]]) {
    constexpr sampler textureSampler(coord::normalized, address::repeat,
                                     filter::linear, mip_filter::none);
    if (textureEnabled == 0 || v.hasUV < 0.5) {
        // Untextured debug geometry is intentionally solid; do not display
        // missing translucent DDSes as opaque full-screen light planes.
        return float4(v.color, 1.0);
    }
    float4 sample = diffuse.sample(textureSampler, v.uv);
    if (blendEnabled != 0) {
        // NiAlphaProperty expects continuous opacity, not the binary
        // alpha-cutout the old renderer applied to volumetric light cards.
        if (sample.a * v.opacity < 0.002) { discard_fragment(); }
        return float4(sample.rgb * v.light, sample.a * v.opacity);
    }
    // Preserve the original alpha-test path for cutouts/opaque scenery.
    if (sample.a < 0.5) { discard_fragment(); }
    return float4(sample.rgb * v.light, 1.0);
}
