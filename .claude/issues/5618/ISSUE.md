# TD7-2026-10-11-07: `heave_hold_damping` — the stabilising coefficient is a measured constant for one density ratio and gravity, and its `1e-3` floor does not prevent the instability it floors against

URL: https://github.com/matiaszanolli/ByroRedux/issues/5618
Labels: low, physics, water, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md


- **Severity**: LOW (analytic, not reproduced; reachability of the failing range unproven)
- **Dimension**: 7
- **Location**: `crates/physics/src/water.rs:196-202,1123`; `crates/physics/src/convert.rs:28-30` (`clamp_shape_extent`)
- **Status**: NEW
- **Age**: c5bf46e59, 2026-10-11 (#5547)
- **Effort**: small
- **Description**:
  - `HEAVE_HOLD_COEFFICIENT = 11.0` is labelled "measured, not derived" and applied as `(11.0 / height.max(1.0e-3) - base)`.
  - The doc derives the needed damping as proportional to `ω² = ρ·|g|/h`, "here ≈ 1146/h". 1146 is `1.67 × 686.7`, i.e. the default `buoyancy_density_ratio` times gravity in BU/s².
  - `density_ratio` is a runtime resource field (`PhysicsWaterConstants::buoyancy_density_ratio`, with no documented range) and `gravity_y` is world state. Both are in scope at the call site (`water.rs:1123-1126`) and carried on `WaterLift` (`:181-182`), but `heave_hold_damping(height, base)` takes neither.
  - `thin_floats_settle_and_sleep_at_any_frame_rate` only runs `PhysicsWaterConstants::default()`.
- **Evidence**: Heave damping is applied as an explicit force, `now.y -= heave_damping * mass * vy`, refreshed every `PHYSICS_DT = 1/60`. That is stable only while `c·dt < 2`. With `c = 11/h − 1.5` this needs `h > 11 / (120 + 1.5) ≈ 0.0905` BU. The `.max(1.0e-3)` floor lets `c` reach `11000`, so `|1 − c·dt| ≈ 182` per tick. A cuboid at `clamp_shape_extent`'s floor (half-extent `1e-3`, so `h = 0.002`) gives `c·dt ≈ 91.6` and a multiplier of about 90 per tick. The lift spring itself (`ω² = 1146/h`, so `ω·dt > 2` below `h ≈ 0.08`) crosses the same stability edge. I did not run this; trimesh and convex-hull shapes are not extent-clamped either.
- **Impact**: Two parts.
  - A retuned `buoyancy_density_ratio`, for example the 2.0 used by a physics fixture, leaves the coefficient mis-sized, and the #5547 limit cycle on thin floats can come back without any test noticing.
  - Bodies thinner than about 0.09 BU (1.3 mm) would diverge in water instead of being damped. Whether such colliders exist in shipped content is unproven.
- **Related**: #5547, #5561.
- **Suggested Fix**:
  - Make the coefficient a function: `heave_hold_damping(height, base, density_ratio, gravity_y.abs())`, with `K = 11.0 / (1.67 * 686.7)` as the measured constant and `K * density_ratio * |g| / height`.
  - Clamp the result to `c <= 1.0 / PHYSICS_DT` (for example `min(c, 0.9 / dt)`) instead of `height.max(1e-3)`, so it cannot overshoot.
  - Add one `thin_float_settle` run at `buoyancy_density_ratio = 2.5`.
