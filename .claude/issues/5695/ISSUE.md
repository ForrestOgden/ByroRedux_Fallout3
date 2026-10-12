# TOOL-D6-2026-10-12-01: The new read-only output/scratch refusal in texture-upscale has no test

Labels: low,tech-debt,test-gap,bug
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5695

Filed from `docs/audits/AUDIT_TOOLING_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Tool CLIs (test gap)
- **Exposure**: developers only
- **Location**: `tools/texture-upscale/src/space.rs:36-60` (`ensure_output_and_scratch_space`), added by `1ca54a3ac`
- **Status**: NEW
- **Description**: The commit makes the pre-flight bail with "`output disk <mount> is read-only`" before an hours-long upscale pass. The decision is taken inside `ensure_output_and_scratch_space`, which reads real disks through `volume_for`, so the existing tests (`insufficient_space_is_rejected_before_writes` exercises only `ensure_available`) never reach it. Nothing pins that the check covers both roles, that the input volume is deliberately exempt, or that the message names the role.
- **Evidence**: `grep -rn 'read_only\|read-only' tools/texture-upscale/src` hits only the implementation (`space.rs:11,44-50,116`).
- **Impact**: a refactor of `VolumeSpace` or of the loop would silently drop the refusal; the user then loses the upscale pass at the first write. The behaviour is correct today (sysinfo populates `is_read_only` on the `with_storage()` refresh the code uses).
- **Related**: #4761, #4762
- **Suggested Fix**: Split the loop into a pure `fn refuse_read_only(output: &VolumeSpace, scratch: &VolumeSpace) -> Result<()>` and unit-test it with hand-built `VolumeSpace` values for output, scratch and neither.

## Completeness Checks
- [ ] **TESTS**: A regression test pins this specific fix

