# Phase 2 — original Fallout: New Vegas installation preflight

Status: **source-only auditor prepared; real game data not supplied/tested**.

On Steam: **Library → Fallout: New Vegas → gear icon → Manage → Browse local files**, then open `Data`. No ISO or product key is required for the already-installed game.

The Python standard-library script `ios/tools/audit_game_data.py` reads only the top-level PC `Data` ESM/ESP/ESL/BSA/BA2 files. It validates the three New Vegas base-game names and the TES4/BSA header magic, produces file sizes and SHA-256 hashes, and warns about optional DLC and user modifications.

Official New Vegas DLC and Courier's Stash bonus masters are recognized separately; only story DLC needs a matching `- Main.bsa` warning. A manifest can contain file paths/names/hashes, but never raw Bethesda asset bytes.

It **does not** extract, modify, upload, decrypt, or redistribute Bethesda game assets. File headers and hash matching do not prove all internal records are valid or that the game runs.

## Running on Windows (Python 3 required)

From a local clone of the repository:

```powershell
py -3 ios/tools/audit_game_data.py "C:\\Games\\Fallout New Vegas\\Data" --output "$env:USERPROFILE\\Desktop\\falloutnv_manifest.json"
```

Replace the example Data path with your actual Fallout: New Vegas installation's Data folder. **Do not upload game archives to GitHub.**

- `py`: Windows Python launcher; `-3`: select a Python 3 interpreter.
- `--output`: location of the JSON *metadata-only* manifest. Omit this option to print JSON to the terminal.
- The script returns exit code 0 when essential names/headers pass, 1 for a layout/header failure, and 2 for invalid input or IO errors.

To verify a later copied Data folder has identical archives:

```powershell
py -3 ios/tools/audit_game_data.py "D:\\CopiedFalloutNV\\Data" --verify "$env:USERPROFILE\\Desktop\\falloutnv_manifest.json"
```

- `--verify`: compare sizes and SHA-256 hashes against the previously generated metadata manifest.

## Advancement gate

1. Unit tests pass on GitHub Actions using synthetic files.
2. User runs the audit against their own real game and checks that `ready_for_parser_tests` is true.
3. Record only necessary metadata results; never accept game bytes in GitHub.
4. On supported Linux/GPU hardware, run upstream Fallout: New Vegas parser and smoke-test scripts with the original data and record explicit pass/fail/skip results.
5. No iOS gameplay claims until rendering and runtime device tests exist.
