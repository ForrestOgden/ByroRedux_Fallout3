# TD2-2026-10-11-09: `extensions/dispatch.rs` repeats one delivery loop nine times (eligibility, guest entry, log drain, commit)

URL: https://github.com/matiaszanolli/ByroRedux/issues/5592
Labels: low, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/extensions/dispatch.rs:251-290, 323-358, 392-430, 453-486, 505-541, 553-585, 663-710, 726-760` (eight `stats.deliveries += 1` sites plus the update loop); helper home `byroredux/src/extensions/commands.rs:138` (`enter_guest`), `mod.rs:142` (`delivery_commit_context!`)
- **Status**: NEW (the earlier #3863 consolidated the snapshot prologue; this is the delivery body)
- **Age**: split out in `55cbc0d6c` 2026-09-11; touched 4 times since 10-08.
- **Effort**: medium
- **Description**: each `dispatch_*` builds `for hosted in &mut self.components`, tests `hosted.receives_X && grants().contains(EVENTS_SUBSCRIBE_CAPABILITY) && status() == Active` (8 copies of the three-line predicate), `stats.deliveries += 1`, `enter_guest`, `set_entity_projections(entity_projections.clone())`, calls one `on_*`, drains `take_logs()` into `ExtensionDiagnostic::Log` (9 identical 7-line blocks), then `apply_delivery_result(hosted, result, LifecyclePhase::X, &principal, delivery_commit_context!(self, stats))`. Only the flag, event constructor and phase differ.
- **Evidence**: `grep -c EVENTS_SUBSCRIBE_CAPABILITY dispatch.rs` = 8; `grep -c 'take_logs().into_iter().map' dispatch.rs` = 9; `grep -c 'apply_delivery_result(' dispatch.rs` = 9.
- **Impact**: a lifecycle-wide rule (a new capability gate, a fault policy) is a 9-site edit; the predicate drifting in one loop would silently deliver to a revoked component.
- **Related**: #3863, #3843.
- **Suggested Fix**: add `fn deliver(&mut self, phase: LifecyclePhase, stats: &mut ExtensionDispatchStats, projections: &[EntityProjection], eligible: impl Fn(&HostedComponent) -> bool, call: impl FnMut(&mut HostedComponent) -> DeliveryResult)` to `dispatch.rs` (split-borrow `self.components` vs `self.diagnostics`), and a `fn drain_logs(&mut self.diagnostics, hosted)` for the log block. Each `dispatch_*` becomes ~10 lines.
