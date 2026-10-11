# TD6-2026-10-11-02: The Redux-native plugin pipeline (`DataStore` + `DependencyResolver` + `PluginManifest`) has no production consumer, while four docs and a save-registry row present it as the live load path

URL: https://github.com/matiaszanolli/ByroRedux/issues/5609
Labels: low, esm-plugin, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 6
- **Location**: `crates/plugin/src/datastore.rs` (155 prod LOC), `resolver.rs` (238), `manifest.rs` (74), `record.rs::{Record::add_component, ErasedComponentData}`; claims in `crates/plugin/src/lib.rs:1-10`, `docs/engine/plugin-loading.md:10-30,131-160`, `docs/engine/architecture.md:283`, `CLAUDE.md` workspace tree, `byroredux/src/save_io/registry_completeness_tests.rs:520`, `crates/core/src/ecs/components/inventory.rs:4`
- **Status**: NEW (adjacent: #4384, which deleted only the `legacy/` half and left this half; AUDIT_TECH_DEBT_2026-09-14 TD8-005 noted "neither does the `DataStore::add_plugin` layer they would feed")
- **Age**: `e472e321e` (2026-03-28), untouched in intent for 6.5 months
- **Effort**: medium (delete + doc rewrite) or large (wire an esm -> `Record` bridge)
- **Description**: Every use of `DataStore`, `DependencyResolver`, `ConflictResolution`, `ResolvedRecord`, `PluginManifest::from_toml`, `ErasedComponentData` is inside `crates/plugin/src/{datastore,resolver,manifest,record}.rs` (own tests included) or a doc comment. `grep -rnw "DataStore\|DependencyResolver\|PluginManifest" crates byroredux tools --include='*.rs'` outside those files: `lib.rs` re-export, `extension.rs` (a doc sentence saying it is *separate* from `PluginManifest`), and `registry_completeness_tests.rs:520`.
- **Evidence**:
  - `plugin-loading.md` diagram: `plugin.toml / FalloutNV.esm -> PluginManifest / ESM parser -> DataStore.add_plugin() -> DependencyResolver -> DataStore.resolve() -> EsmCellIndex / FormIdPool`. The real path is `parse_esm_with_load_order` + `FormIdRemap`/`GlobalSlot` (`cell_loader/load_order.rs`); `DataStore` never reaches `EsmCellIndex`. (The doc even has a "Legacy Bridge (removed)" section explaining the *other* dead half.)
  - `registry_completeness_tests.rs:520` justifies a save exemption for `DataStore` as an "immutable resolved plugin-record database rebuilt from manifests/plugins at boot". Nothing builds one at boot; `grep "insert_resource(DataStore"` = 0.
  - `inventory.rs:4` cites `DataStore` as the base-item source of truth; the real source is `EsmIndex`.
  - `RecordType` (same crate) *is* used by the binary (`StaticObject.record_type`); that is why `record.rs` is not wholly dead.
- **Impact**: Misleading architecture picture for contributors and for the save audit; ~470 prod LOC + ~360 test LOC maintained for nothing; the "UUID-keyed, no load-order slots" selling point of the Redux plugin format has no code path, yet ROADMAP does not list it as owed.
- **Related**: #4384, TD8-005 (2026-09-14), `docs/engine/modern_plugin_system` memory note (design intent: Redux-native format is a stated goal)
- **Suggested Fix**: Decide one way. Keep: add a ROADMAP "Owed" row ("Redux-native manifest/DataStore loader has no caller") and put a status banner at the top of `plugin-loading.md` and `lib.rs`; fix the save-registry row's reason string to "defined, never inserted". Drop: delete `datastore.rs`/`resolver.rs`/`manifest.rs` + `Record::add_component`/`ErasedComponentData` and rewrite the diagram.
