# TD6-2026-10-11-04: The Starfield WTHS value-reader "tracked follow-up (see the issue)" points at the closed decode issue; `exal.md` still says WTHS "stays undecoded"

URL: https://github.com/matiaszanolli/ByroRedux/issues/5611
Labels: low, esm-plugin, terrain-exterior, game:starfield, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 6
- **Location**: `crates/plugin/src/esm/records/weather_settings.rs:50-58,138-143`; `docs/engine/exal.md:153`; consumer-less map `EsmIndex.weather_settings` (`index.rs:225`, dispatch `dispatch_misc_gameplay_a.rs:61`)
- **Status**: NEW
- **Age**: `a5484faf6` (2026-10-09), `f0f8dceef` (2026-10-11)
- **Effort**: trivial (open one tracker + 1-line doc edit)
- **Description**: Both doc comments defer the OBJT/LIST/USER instance-value decode to "the tracked follow-up (see the issue)", but the only referenced issue, #5364, is CLOSED ("Decode Starfield WTHS ... the real SF weather schema") and `gh issue list --search WTHS` returns no open issue. The schema half landed; the value half (fog distances, colours, wind) is the part a renderer would need, and `WeatherSettingsRecord` has zero readers (`grep -rn "weather_settings\|WeatherSettingsRecord"` outside the record module = index/dispatch only). `exal.md:153` ("while Starfield's WTHS stays undecoded") is now half-true and will mislead the next EXAL audit.
- **Evidence**: commands above; `weather_root_class` (new pub fn in the same file) also has no prod caller.
- **Impact**: An untracked placeholder in a 2-day-old record decoder; Starfield exteriors keep using the `DefaultWeather` WTHR stand-in (#5363/#5424) with nothing recording what removes it.
- **Related**: #5364, #5514, #5363, #5424
- **Suggested Fix**: File "Starfield WTHS instance-value reader (OBJT/LIST/USER) -> weather params", cite its number in both doc comments, and reword `exal.md:153` to "schema decoded (#5364), values not".
