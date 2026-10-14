# Fallout 3 assets and signing materials

This public repository contains **source code only**. Bethesda game data must originate from a game copy the end user is entitled to use. No game installers, archive contents, textures, voices, scripts, DLC, keys, or recovered proprietary assets may be committed or uploaded to GitHub Actions artifacts.

## Local asset import

- Keep user files outside the checkout, or in local directories excluded by `.gitignore`.
- Never push `Fallout3.esm`, `*.bsa`, `*.esp`, `*.esl`, `*.ba2`, game executables, or decrypted platform content.
- Do not require private game assets in ordinary CI; perform game-data smoke tests locally when the actual user files are present.
- Generate a SHA-256 manifest locally; exclude licensed bytes from reports.
- Keep Apple signing keys, provisioning profiles, and certificates out of source control and build logs.

The CI check `scripts/ios/check-no-game-assets.sh` fails if any tracked file has a known proprietary game-data or signing extension, even if an ignore rule was overridden.
