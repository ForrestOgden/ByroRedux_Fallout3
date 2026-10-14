#!/usr/bin/env python3
"""Inventory owned Fallout 3 PC game archives without extracting or uploading assets.

Uses Python standard library only. Never writes into game files.
This checks file *layout and headers*, not whether the game will run.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

SUFFIXES = {".esm", ".esp", ".esl", ".bsa", ".ba2"}
REQUIRED = ("Fallout3.esm", "Fallout - Meshes.bsa", "Fallout - Textures.bsa")
DLC = ("Anchorage", "ThePitt", "BrokenSteel", "PointLookout", "Zeta")
HEADERS = {".esm": b"TES4", ".esp": b"TES4", ".esl": b"TES4",
           ".bsa": b"BSA\x00", ".ba2": b"BTDX"}


def find_data_dir(candidate: Path) -> Path:
    candidate = candidate.expanduser().resolve(strict=True)
    if not candidate.is_dir():
        raise ValueError("Expected a Fallout 3 installation or Data directory.")
    if candidate.name.casefold() == "data" or any(
        p.is_file() and p.name.casefold() == "fallout3.esm"
        for p in candidate.iterdir()
    ):
        return candidate
    for p in candidate.iterdir():
        if p.is_dir() and p.name.casefold() == "data":
            return p.resolve(strict=True)
    raise ValueError("No Data directory found. Provide the game's Data folder or installation root.")


def inspect(data_dir: Path) -> dict:
    """Return stable, content-free metadata. Hashes can verify later copies."""
    data = find_data_dir(data_dir)
    files = sorted((p for p in data.iterdir()
                    if p.suffix.casefold() in SUFFIXES and (p.is_file() or p.is_symlink())),
                   key=lambda p: (p.name.casefold(), p.name))
    errors: list[str] = []
    warnings: list[str] = []
    inventory: list[dict] = []
    names: set[str] = set()

    for path in files:
        name = path.name.casefold()
        if name in names:
            errors.append(f"Duplicate case-insensitive filename: {path.name}")
            continue
        names.add(name)
        if path.is_symlink():
            errors.append(f"Symlink is not accepted as game input: {path.name}")
            continue
        try:
            with path.open("rb") as stream:
                header = stream.read(4)
                hasher = hashlib.sha256()
                hasher.update(header)
                for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                    hasher.update(chunk)
            size = path.stat().st_size
        except OSError as exc:
            errors.append(f"Could not read {path.name}: {exc.strerror or type(exc).__name__}")
            continue
        expected = HEADERS[path.suffix.casefold()]
        if header != expected:
            errors.append(f"Unexpected {path.suffix.upper()} header: {path.name}")
        inventory.append({"name": path.name, "size_bytes": size, "sha256": hasher.hexdigest()})

    missing = [name for name in REQUIRED if name.casefold() not in names]
    errors.extend(f"Missing required base-game file: {name}" for name in missing)
    dlc = [name for name in DLC if (name + ".esm").casefold() in names]
    for name in dlc:
        if (name + " - Main.bsa").casefold() not in names:
            warnings.append(f"DLC {name} has an ESM but no matching Main BSA")
    plugins = [entry["name"] for entry in inventory
               if Path(entry["name"]).suffix.casefold() in {".esp", ".esl"}]
    if plugins:
        warnings.append("Additional ESP/ESL plugins present; use an unmodded game for baseline tests")

    return {
        "schema_version": 1,
        "product": "Fallout 3 PC",
        "scope": "top-level Data archive/plugin files only; no data bytes included",
        "files": inventory,
        "required_files": list(REQUIRED),
        "dlc_detected": dlc,
        "additional_plugins": plugins,
        "errors": errors,
        "warnings": warnings,
        "ready_for_parser_tests": not errors,
    }


def compare(original: dict, current: dict) -> dict:
    """Compare two metadata manifests after a local-only file transfer."""
    if original.get("schema_version") != 1:
        raise ValueError("Reference manifest schema_version must be 1")
    def indexed(report: dict) -> dict:
        return {x["name"].casefold(): x for x in report["files"]}
    before, after = indexed(original), indexed(current)
    missing = sorted(before.keys() - after.keys())
    added = sorted(after.keys() - before.keys())
    changed = sorted(
        name for name in before.keys() & after.keys()
        if before[name]["sha256"] != after[name]["sha256"]
        or before[name]["size_bytes"] != after[name]["size_bytes"]
    )
    return {"matches": not (missing or added or changed),
            "missing": missing, "added": added, "changed": changed}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("game_path", type=Path, help="Fallout 3 installation or Data directory")
    parser.add_argument("--output", type=Path, help="Write JSON metadata manifest (not game data)")
    parser.add_argument("--verify", type=Path, help="Compare to an earlier local metadata manifest")
    args = parser.parse_args(argv)
    try:
        result = inspect(args.game_path)
        if args.verify:
            baseline = json.loads(args.verify.read_text(encoding="utf-8"))
            result["verification"] = compare(baseline, result)
        encoded = json.dumps(result, sort_keys=True, indent=2) + "\n"
        if args.output:
            if args.output.suffix.casefold() != ".json":
                raise ValueError("Output must be a .json file, never a game archive.")
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(encoded, encoding="utf-8")
            print(f"Metadata manifest written: {args.output}")
        else:
            print(encoded, end="")
        return 0 if result["ready_for_parser_tests"] and (
            "verification" not in result or result["verification"]["matches"]
        ) else 1
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"Data audit error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
