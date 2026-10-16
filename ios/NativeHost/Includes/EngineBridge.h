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
/* 32-byte Metal vertex: float4 position + float4 color. */
typedef struct { float position[4]; float color[4]; } FnvVertex;
typedef struct { FnvVertex *vertices; size_t count; } FnvMesh;
/* 0 on success; 1 unreadable, 2 unsupported geometry, negatives invalid input. */
int32_t fnv_ios_load_nif_mesh(const char *path, FnvMesh *out);
/* Release successful mesh pointer using the exact count, once. */
void fnv_ios_release_mesh(FnvVertex *vertices, size_t count);

#ifdef __cplusplus
}
#endif
#endif
