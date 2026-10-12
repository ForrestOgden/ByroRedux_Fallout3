# AUD-2026-10-12-D2-01: The "send level can't retro-apply" limitation is documented as a kira limit; kira 0.12.5 has `SpatialTrackHandle::set_send`, and the limit is our own `with_send` gate

Labels: low,audio,documentation,doc-rot
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5677

Filed from `docs/audits/AUDIT_AUDIO_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW (doc claim; no behavioural effect today)
- **Dimension**: Send Graph
- **Location**: `crates/audio/src/lib.rs:869-882` (the `set_reverb_send_db` doc), `crates/audio/src/tests.rs:501`, `apply_reverb_send` at `lib.rs:295`
- **Status**: NEW (the claim is inherited from the closed #847 / 2026-06-23 audit and was never re-checked, including by the kira 0.12 bump)
- **Description**: The docs say "kira 0.12's `with_send` is build-time only … there is no `SpatialTrackHandle::set_send_volume`, so a level change cannot retro-apply to already-playing tracks". kira-0.12.5 `src/track/sub/spatial_handle.rs:126` has `pub fn set_send(&mut self, to, volume, tween) -> Result<(), NonexistentRoute>`; it changes the volume of an existing route, and 0.10.8 has the same method. The real constraint is that a route has to exist, and `apply_reverb_send` only calls `with_send` when the level is audible, so a track built dry has no route.
- **Evidence**: `kira-0.12.5/src/track/sub/spatial_handle.rs:121-135`: "This can only be used to change the volume of existing routes, not to add new routes."
- **Impact**: None today. Long-running looping emitters (REGN ambient loops, #3301) are not shipped. It matters at design time: a live fader is possible if every sub-track is built with a route at `SILENCE_DB` and `set_reverb_send_db` retargets live tracks.
- **Related**: #847 (closed).
- **Suggested Fix**: Reword the doc to "routes can be retargeted but not added; tracks built while the send was silent have no route", and note the always-build-the-route option for the future per-cell reverb work.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites

