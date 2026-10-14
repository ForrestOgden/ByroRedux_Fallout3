#!/usr/bin/env bash
set -euo pipefail

# CI policy: only source code/test fixtures may enter this public fork.
# Rejected extensions are not needed for Rust-only Phase 1 verification.
blocked=0
while IFS= read -r -d '' path; do
  lower=${path,,}
  case "$lower" in
    *.esm|*.esp|*.esl|*.bsa|*.ba2|*.ipa|*.mobileprovision|*.p12|*.p8)
      printf 'BLOCKED tracked game data or signing artifact: %s\n' "$path" >&2
      blocked=1 ;;
  esac
  case "$lower" in
    fallout3-data/*|game-data/*|local-game-data/*|ios/imported-game-data/*|ios/game-data/*)
      printf 'BLOCKED tracked private data directory: %s\n' "$path" >&2
      blocked=1 ;;
  esac
done < <(git ls-files -z)
if ((blocked)); then
  echo 'Source-only policy FAILED.' >&2
  exit 1
fi
echo 'Source-only policy PASSED.'
