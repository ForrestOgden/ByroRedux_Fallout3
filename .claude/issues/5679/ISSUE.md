# AUD-2026-10-12-D3-01: Audio documentation drifted behind the kira 0.12 / voice / Wave-4 work

Labels: low,audio,documentation,doc-rot
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5679

Filed from `docs/audits/AUDIT_AUDIO_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Music & SoundCache / docs
- **Location**:
  - `crates/audio/src/lib.rs:42` ("WAV + OGG covered by kira's default features") and `:1640-1642` ("kira pulls in symphonia with the `wav`, `ogg`, `mp3`, and `flac` features by default")
  - `docs/COMPATIBILITY.md:317` ("kira 0.10")
  - `docs/feature-matrix.md` Audio table (143-155): no row for the shipped Phase-V dialogue voice or the Wave-4 impact mechanism
  - `byroredux/src/asset_provider/texture.rs` (`DefaultSoundCandidate.games` doc says tags are `debug_profiles.toml` profile names; the profile is `skyrim_se`, the tag `skyrimse`)
- **Status**: NEW for the lib.rs, COMPATIBILITY.md and tag items. The feature-matrix gap is **Existing: #4747** (status docs; the REGN row is now qualified, the voice and impact rows are still absent). `docs/feature-matrix.md:147` "kira 0.10" was fixed after TD3-2026-10-11-04.
- **Description**: Since `804c715` the workspace sets `default-features = false` and lists the codecs explicitly (0.12 split container from codec: `wav` needs `pcm`, `ogg` needs `vorbis`). The crate docs still describe the features as kira's defaults, which is the framing that hides the "drop a codec and decode silently stops" trap the workspace `Cargo.toml` comment warns about.
- **Evidence**: `Cargo.toml:209-218` vs `lib.rs:42`, `:1641`.
- **Impact**: Documentation only.
- **Related**: #4747, #5600 (tech-debt).
- **Suggested Fix**: Point the two lib.rs sites at `Cargo.toml`'s explicit feature list and the `pcm`/`vorbis` requirement; update COMPATIBILITY.md; add voice and impact rows to the feature matrix.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites

