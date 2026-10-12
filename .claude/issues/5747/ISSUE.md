# FNV-2026-10-12-D2-01: INFO `DATA` Flags 2 — Say Once a Day and Always Darken — is decoded into `InfoDataHeader::flags2` and never read, so 105 FalloutNV.esm and 89 DLC INFOs repeat on every conversation

- **GitHub**: #5747 (https://github.com/matiaszanolli/ByroRedux/issues/5747)
- **Labels**: low,game:fnv,game:fo3,legacy-compat,esm-plugin,dialogue,bug
- **Source**: docs/audits/AUDIT_FNV_2026-10-12.md (`FNV-2026-10-12-D2-01`)

**Audit finding** `FNV-2026-10-12-D2-01` — severity LOW — from `docs/audits/AUDIT_FNV_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `crates/plugin/src/esm/records/misc/dialogue.rs` (`flags2` field, decode of `DATA` byte 3); consumers of `InfoRecord::say_once()` / `goodbye()` / `random()` / `random_end()` in `crates/scripting/src/dialogue.rs` read `Flags 1` only

## Description
FO3/FNV INFO `DATA` byte 3 is `Flags 2`: bit 0 = Say Once a Day, bit 1 = Always Darken (xEdit `wbINFO`; the field's own doc names both). The GECK wiki (`Category/Dialogue`) defines Say Once a Day as "this info will only be played once per 'day' per actor", with the day length set by the `fSayOncePerDayInfoTimer` game setting (12 hours). The decoder stores the byte, but no reader consumes it (validated at publish time: `flags2` is only referenced inside `dialogue.rs` and its tests). `select_info` therefore treats such an INFO as repeatable on every conversation. Always Darken is a prompt-shading hint and has no selection effect.

## Evidence
Probe `sayday.py` over the shipped masters, counting INFOs whose `DATA[3] & 1` is set:

| Plugin | INFOs | Say Once a Day |
|---|---:|---:|
| FalloutNV.esm | 23,247 | 105 |
| DeadMoney.esm | 2,151 | 16 |
| HonestHearts.esm | 1,238 | 1 |
| OldWorldBlues.esm | 1,620 | 71 |
| LonesomeRoad.esm | 667 | 1 |

By topic: `IdleChatter` 62 lines, `GREETING` 8, plus DLC radio, sink and brain barks. `Always Darken` (bit 1) is set on 6 FalloutNV.esm INFOs. FO3 reach (from the FO3 audit): 1,108 FO3 INFOs carry bit 0 (1,033 `Fallout3.esm`, 11 Anchorage, 37 BrokenSteel, 7 PointLookout, 16 ThePitt, 4 Zeta) and 46 carry bit 1; e.g. Cafe Beau Monde's "Welcome to Cafe Beau Monde!" (`0x2213F`).

## Impact
A Say-Once-a-Day greeting or bark that should yield to the next qualifying INFO after its first play stays first in file order and is spoken on every conversation (the "welcome to my shop" repeat the flag exists to prevent). Small: ~0.45% of FNV INFOs, mostly idle chatter.

## Suggested Fix
Either extend the spoken ledger with a game-time stamp so a Say-Once-a-Day INFO is skipped for `fSayOncePerDayInfoTimer` hours per `(speaker, INFO)`, or document the flag as unmodelled in `dialogue-trees.md` next to the other unmodelled INFO flags.

## Related
#5300 (Say Once ledger), #5367 (dialogue layers), `docs/engine/dialogue-trees.md`.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **TESTS**: A regression test pins this specific fix
