#include <metal_stdlib>
using namespace metal;

// This source compiles into the signed app's default .metallib during Xcode build.
struct FNVTriangleOutput {
    float4 position [[position]];
    float3 color;
};

vertex FNVTriangleOutput fnv_triangle_vertex(uint index [[vertex_id]]) {
    const float2 positions[3] = {
        float2(0.0, 0.78),
        float2(-0.78, -0.72),
        float2(0.78, -0.72)
    };
    const float3 colors[3] = {
        float3(1.0, 0.25, 0.25),
        float3(0.30, 1.0, 0.35),
        float3(0.25, 0.40, 1.0)
    };
    FNVTriangleOutput output;
    output.position = float4(positions[index], 0.0, 1.0);
    output.color = colors[index];
    return output;
}

fragment float4 fnv_triangle_fragment(FNVTriangleOutput input [[stage_in]]) {
    return float4(input.color, 1.0);
}
