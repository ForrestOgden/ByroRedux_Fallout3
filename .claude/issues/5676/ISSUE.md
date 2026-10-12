# AUD-2026-10-12-D5-02: `--sounds-bsa` is documented as "first hit wins, list overrides first" in three places; the provider has been last-listed-wins since #3917

Labels: low,audio,documentation,doc-rot
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5676

Filed from `docs/audits/AUDIT_AUDIO_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Engine Consumers (archive precedence)
- **Location**:
  - `docs/engine/game-loop.md:56` (the user-facing CLI table)
  - `byroredux/src/asset_provider/audio.rs:83`
  - `byroredux/src/asset_provider/texture.rs:297` (and `try_load_default_water_splash`'s "first archive/candidate hit wins")
- **Status**: NEW
- **Description**: The #3776 wording ("override/mod archives listed before the vanilla one, first hit wins") predates #3917, which flipped `SoundArchiveProvider::extract` to `extract_first(self.archives.iter().rev(), …)`. The same file's `extract` doc, `assets/debug_profiles.toml` (which lists `Update.bsa` *after* `Fallout - Sound.bsa` for exactly this reason) and the CLI-profile comments all say last-wins. The three drifted sites were not updated.
- **Evidence**:
  ```text
  game-loop.md:56   Repeatable — override/mod archives listed before the vanilla one, first hit wins (#3776)
  audio.rs:141      extract_first(self.archives.iter().rev(), archive_path)
  ```
- **Impact**: A user following the CLI documentation lists their override first; the vanilla archive then shadows it, and the footstep, splash, REGN and voice lookups serve unmodded audio with no log line. Workaround: list overrides last.
- **Related**: #3776 (closed), #3917 (closed). `script.rs:935` documents `ScriptProvider`'s deliberate first-wins correctly.
- **Suggested Fix**: Change all three wordings to "later `--sounds-bsa` archives override earlier ones (last-listed wins; `ScriptProvider` is the first-wins exception)".

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites

