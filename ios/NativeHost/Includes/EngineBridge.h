#ifndef FNV_IOS_ENGINE_BRIDGE_H
#define FNV_IOS_ENGINE_BRIDGE_H
#include <stdint.h>
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif

uint32_t fnv_ios_abi_version(void);
/* Returns 0 for valid BSA, 1 for invalid/unreadable, -1 null, -2 non-UTF8. */
int32_t fnv_ios_probe_bsa(const char *path);
/* 48-byte Metal vertex: float4 position, float4 color, float4 uv_light.
 * uv_light.xy = original UV, .z = geometric brightness, .w = hasUV. */
typedef struct { float position[4]; float color[4]; float uv_light[4]; } FnvVertex;
typedef struct { FnvVertex *vertices; size_t count; } FnvMesh;
/* 0 on success; 1 unreadable, 2 unsupported geometry, negatives invalid input. */
int32_t fnv_ios_load_nif_mesh(const char *path, FnvMesh *out);
/* Release successful mesh pointer using the exact count, once. */
void fnv_ios_release_mesh(FnvVertex *vertices, size_t count);
/* World scene import: preserve original authored Y-up NIF coordinates
 * instead of normalizing a mesh to the single-object preview scale. */
int32_t fnv_ios_load_nif_mesh_raw(const char *path, FnvMesh *out);
// World NIF import preserves authored per-submesh material draw ranges.
// material_index is the original ByroRedux ImportedMesh ordinal.
typedef struct {
    size_t start;
    size_t count;
    uint32_t material_index;
    uint32_t reserved;
} FnvWorldBatch;
typedef struct {
    FnvVertex *vertices;
    size_t count;
    FnvWorldBatch *batches;
    size_t batch_count;
    size_t mesh_count;
} FnvWorldMesh;
/* Call release once with BOTH returned pointer/count pairs. */
int32_t fnv_ios_load_nif_mesh_world(const char *path, FnvWorldMesh *out);
void fnv_ios_release_world_mesh(FnvVertex *vertices, size_t count,
                                FnvWorldBatch *batches, size_t batch_count);
typedef struct {
    uint8_t *pixels;
    size_t len;
    uint32_t width;
    uint32_t height;
} FnvTexture;
/* Decode local BC1/2/3 DDS into RGBA8; 0 success, 1 unreadable, 2 unsupported. */
int32_t fnv_ios_load_dds_rgba(const char *path, FnvTexture *out);
void fnv_ios_release_texture(uint8_t *pixels, size_t len);

#ifdef __cplusplus
}
#endif
#endif
