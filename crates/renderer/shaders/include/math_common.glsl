// Directional-ambient cube sample, octahedral normal codec, noise / disk / hemisphere sampling, ortho basis
//
// NON-STANDALONE shader fragment. Included by triangle.frag in dependency
// order via GL_GOOGLE_include_directive; it references symbols (structs,
// SSBO/UBO bindings, helper functions, constants) defined in shader_constants.glsl
// and in earlier includes. Do not compile on its own.


// Sample the 6-axis directional ambient cube along surface normal `N`.
// Weights are the squared components of N, with the sign selecting the
// positive or negative face on each axis. For a unit normal they sum to
// exactly 1, so diagonal and normal-mapped surfaces interpolate the cube
// without gaining up to sqrt(3) extra irradiance. This is the standard
// ambient-cube evaluation used by Valve's Source shading model. Replaces
// the hand-tuned `AMBIENT_AO_FLOOR = 0.3` constant with a directionally-
// correct sample of the WTHR/XCLL-authored hemisphere. See #993.
vec3 sampleDalcCube(vec3 N) {
    vec3 n2 = N * N;
    vec3 pw = mix(vec3(0.0), n2, greaterThanEqual(N, vec3(0.0)));
    vec3 nw = mix(vec3(0.0), n2, lessThan(N, vec3(0.0)));
    return dalcPosX.xyz * pw.x + dalcNegX.xyz * nw.x
         + dalcPosY.xyz * pw.y + dalcNegY.xyz * nw.y
         + dalcPosZ.xyz * pw.z + dalcNegZ.xyz * nw.z;
}

// CLUSTER_TILES_X/Y, CLUSTER_SLICES_Z, CLUSTER_NEAR/FAR_FLOOR/FAR_FALLBACK
// from shader_constants.glsl. #628 — cluster-grid far plane sourced from
// CLMT fog_far (`screen.w`) at runtime; clamped to CLUSTER_FAR_FLOOR/FALLBACK.

const float PI = 3.14159265359;

// ── Octahedral normal codec (Cigolle et al. 2014) ───────────────────
// `octEncode` / `octDecode` moved to their own STANDALONE header under
// #4008 so the four compute consumers can share this exact pair instead
// of carrying copies. This header is not standalone (see the file
// docstring), so it could never be the shared definition itself.
#include "oct_codec.glsl"

// ── Noise for stochastic shadow rays ────────────────────────────────

// Interleaved gradient noise (Jimenez 2014) — excellent spatial distribution,
// cheap to compute, and when seeded with frame counter gives temporally
// varying patterns that average to smooth penumbra over a few frames.
//
// #4951 — the frame term is wrapped to Jimenez's 64-frame period before the
// multiply. Unwrapped, `frameCount * 5.588238` grows to ~9e7 over a session
// (`frameCount` runs to 2^24), and the f32 sum with `fragCoord` drops the
// pixel coordinate's low bits: ~1 M frames left 32 distinct outputs and 81 %
// of vertically adjacent pixels equal. `mod` by a power of two is exact in
// f32, so the wrapped term stays below 64 * 5.588238 for the whole session.
float interleavedGradientNoise(vec2 fragCoord, float frameCount) {
    vec3 magic = vec3(0.06711056, 0.00583715, 52.9829189);
    float frame = mod(frameCount, 64.0);
    float shifted = fract(magic.z * fract(dot(fragCoord + frame * vec2(5.588238, 5.588238), magic.xy)));
    return shifted;
}

// PCG 2D hash (Jarzynski & Olano 2020, "Hash Functions for GPU Rendering").
// Maps a 2D integer key to two well-distributed 32-bit values. Used to build
// DECORRELATED per-(pixel, frame) 2D samples for 1-spp Monte-Carlo passes.
uvec2 pcg2d(uvec2 v) {
    v = v * 1664525u + 1013904223u;
    v.x += v.y * 1664525u;
    v.y += v.x * 1664525u;
    v.x ^= v.x >> 16u;
    v.y ^= v.y >> 16u;
    v.x += v.y * 1664525u;
    v.y += v.x * 1664525u;
    v.x ^= v.x >> 16u;
    v.y ^= v.y >> 16u;
    return v;
}

// Two uniforms in [0,1) decorrelated per pixel AND per frame. Unlike
// `interleavedGradientNoise` (a SCALAR low-discrepancy dither — correlated
// when used for both axes of a 2D sample), this yields white-noise 2D
// samples: the 1-spp error is spatially AND temporally UNCORRELATED, so the
// à-trous spatial filter and the SVGF temporal accumulation can both resolve
// it. IGN-derived 2D samples instead leave a coherent grid moiré that no
// denoiser can remove (neighbours share the bias). Reference: Jimenez 2014
// (IGN, scalar use) vs. Jarzynski/Olano 2020 (hash, 2D use).
vec2 hash2_pixel_frame(uvec2 pixel, uint frame) {
    uvec2 key = pixel ^ uvec2(frame * 0x9E3779B9u, frame * 0x85EBCA6Bu);
    uvec2 h = pcg2d(key);
    return vec2(h) * (1.0 / 4294967296.0);
}

// Interleaved gradient noise WITHOUT the frame term: a fixed per-pixel
// scalar whose spatial spectrum is blue (Jimenez 2014) — neighbouring
// pixels are anti-correlated, so an error field keyed on it is dither, not
// grain. The frame-dependent `interleavedGradientNoise` above crawls; this
// one is a static mask.
float ignMask(vec2 pixel) {
    return fract(52.9829189 * fract(dot(pixel, vec2(0.06711056, 0.00583715))));
}

// #5369 — n-th 2D sample of a per-pixel DITHERED low-discrepancy sequence.
//
// The shadow-ray disk samples used one white-noise hash per (pixel, frame):
// per-pixel temporal error ~1/sqrt(N) AND spatially white, so the standing
// EMA grain read as salt-and-pepper. This is the standard
// "blue-noise-dithered QMC" construction (Roberts 2018 R2 sequence in time,
// Cranley-Patterson rotated per pixel by a spatially blue mask):
//   * TEMPORAL — the R2 additive recurrence (alpha = 1/phi2, 1/phi2^2) is a
//     2D low-discrepancy sequence, so a pixel's running average converges
//     faster than random sampling (~N^-0.75 vs N^-0.5 on a penumbra step);
//   * SPATIAL — the per-pixel rotation offset comes from IGN (blue), so what
//     error remains is anti-correlated between neighbours and averages out
//     under TAA/upscale/the eye instead of reading as grain.
// The recurrence is evaluated in 32-bit integers: n * round(alpha * 2^32)
// wraps modulo 2^32, i.e. an EXACT fract(n * alpha) for any frame count (a
// float multiply loses the fraction past ~1e5 frames). `salt` decorrelates
// independent consumers (lights, rays) so they do not share a rotation.
vec2 ditheredSequence2(uvec2 pixel, uint n, uint salt) {
    vec2 seq = vec2(uvec2(n * 3242174889u, n * 2447445414u)) * (1.0 / 4294967296.0);
    vec2 p = vec2(pixel) + float(salt) * vec2(37.0, 17.0);
    vec2 rotation = vec2(ignMask(p), ignMask(p.yx + vec2(113.5, 247.3)));
    return fract(seq + rotation);
}

// Generate a 2D sample on a unit disk using concentric mapping.
// t1, t2 in [0,1] → (x,y) uniformly distributed on unit disk.
vec2 concentricDiskSample(float t1, float t2) {
    float r = sqrt(t1);
    float theta = 2.0 * PI * t2;
    return vec2(r * cos(theta), r * sin(theta));
}

// Build an orthonormal basis from a unit direction vector (for jittering
// the RT ray). Frisvad (2012), "Building an Orthonormal Basis from a 3D
// Unit Vector Without Normalization" — singularity-free everywhere except
// `dir.z = -1` exactly (which is not a plausible terrain normal in our
// Y-up Z-up→Y-up converted scene; the only place that would surface is a
// downward-facing reflection from a perfectly horizontal mirror, where
// the analytic flip-axis branch handles it).
//
// Pre-#574 the implementation was a `cross(up, dir)` with `up` toggling
// to `vec3(1,0,0)` when `abs(dir.y) >= 0.999`. The 0.999 threshold left
// a NaN window: a fragment whose normal is *exactly* `(0,1,0)` (every
// vertex on a flat terrain LAND quad and on horizontal platform meshes)
// fell on the `<` side, so `up = (0,1,0)` was crossed with itself,
// producing `(0,0,0)` and a NaN after `normalize`. The NaN tangent
// propagated into `cosineWeightedHemisphere`'s direction, fed
// `rayQueryInitializeEXT` with NaN, and the entire frame's GI on flat
// exterior cells (Tamriel, Wasteland, etc.) was undefined per the
// Vulkan RT spec.
//
// Frisvad's method has no degenerate near-pole and is branchless except
// for the sign-axis pick. See #574 (RT-2).
void buildOrthoBasis(vec3 dir, out vec3 tangent, out vec3 bitangent) {
    float sign_z = dir.z >= 0.0 ? 1.0 : -1.0;
    float a = -1.0 / (sign_z + dir.z);
    float b = dir.x * dir.y * a;
    tangent = vec3(1.0 + sign_z * dir.x * dir.x * a, sign_z * b, -sign_z * dir.x);
    bitangent = vec3(b, sign_z + dir.y * dir.y * a, -dir.y);
}

// ── Cosine-weighted hemisphere sampling for GI ─────────────────────

// Generate a cosine-weighted random direction in the hemisphere above N.
// u1, u2 in [0,1] — use noise functions seeded by fragCoord + frameCount.
vec3 cosineWeightedHemisphere(vec3 N, float u1, float u2) {
    float r = sqrt(u1);
    float theta = 2.0 * PI * u2;
    vec3 T, B;
    buildOrthoBasis(N, T, B);
    return normalize(T * (r * cos(theta)) + B * (r * sin(theta)) + N * sqrt(max(1.0 - u1, 0.0)));
}
