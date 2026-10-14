#ifndef FALLOUT3_IOS_ENGINE_BRIDGE_H
#define FALLOUT3_IOS_ENGINE_BRIDGE_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

uint32_t fo3_ios_abi_version(void);
/* Returns 0 for valid BSA, 1 for invalid/unreadable, -1 null, -2 non-UTF8. */
int32_t fo3_ios_probe_bsa(const char *path);

#ifdef __cplusplus
}
#endif
#endif
