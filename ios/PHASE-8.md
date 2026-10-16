# Phase 8 — DirectX DDS surface sampling on native iPhone Metal

**Goal:** The user-verified 142-triangle New Vegas bottle NIF gains a companion locally obtained DDS diffuse texture, mapped by the NIF's original UV coordinates. This is a **one-material-per-preview diagnostic**, not a complete Fallout New Vegas material system or playable game.

## What this change does

- ByroRedux's imported `mesh.uvs` flows from native Rust into a **48-byte vertex ABI** (3 × 16-byte float4: position/color/uv-light). Both Metal and Swift use this exact layout; the previous 32-byte mesh ABI was explicitly versioned to ABI 3.
- A bounded Rust DDS decoder accepts classic uncompressed 32-bit RGB and BC1/DXT1, BC2/DXT3, BC3/DXT5 diffuse textures and emits top-mip RGBA8. It rejects unsupported DX10/BC7, huge images, truncated blocks, cube maps, invalid headers. The CPU expands DXT compression because Metal iOS support for BC block compression must not be assumed.
- Swift's Files picker imports one local DDS, transfers a copy out of Rust before releasing it, and creates a native Metal RGBA8 sRGB texture. The shader samples UVs, applies already-computed diagnostic face illumination, and discards alpha below 0.5. It retains untextured vertex colors for any vertices without UVs and for the procedural cube.
- The locally built Windows extractor can also export a **diffuse DDS from the owner's legally installed Textures BSA**; no game data is committed or included in CI artifacts.

## Windows — extract a local matching texture

Download the successful `FalloutNV-local-NIF-extractor-Windows` artifact from the [Windows workflow](https://github.com/ForrestOgden/ByroRedux_Fallout3/actions/workflows/fnv-windows-extractor.yml). This newer executable still supports the Phase 6 NIF export.

In PowerShell, opened in the directory containing `falloutnv-ios-nif-extractor.exe`:

```powershell
.\falloutnv-ios-nif-extractor.exe --list-textures "C:\Program Files (x86)\Steam\steamapps\common\Fallout New Vegas\Data\Fallout - Textures.bsa" bottle
.\falloutnv-ios-nif-extractor.exe --texture "C:\Program Files (x86)\Steam\steamapps\common\Fallout New Vegas\Data\Fallout - Textures.bsa" "$env:USERPROFILE\Desktop\fnv-bottle.dds"
```

- `--list-textures` displays up to 40 archive paths matching the word `bottle` (no extraction).
- `--texture` chooses a diffuse DDS, preferring `beerbottle01`, `beerbottle`, then `bottle`. It excludes common normal/specular suffixes and refuses to overwrite an existing output. An optional 4th argument can filter the texture filename if the default does not match the mesh.
- `"$env:USERPROFILE\Desktop\fnv-bottle.dds"` is the local output path. Upload **only to your iPhone's Files app**, never to the public GitHub repository.
- If there is no matching diffuse texture, **stop and report the extractor output**; don't randomly choose an unrelated asset. The correct DDS need not have the same name as the NIF and could be stored in another archive.

After importing the NIF in the Phase 8 app, tap the new **Import matching Fallout New Vegas .dds from Files** button and choose the DDS. You should then see the original color/pattern on the model, albeit with simplified ambient/directional shading.

## Known limitations

- A single diffuse texture is applied to *all* submeshes. Source material assignment, UV tiling modes, transparency blending, normal/specular maps, palette textures, per-material effect flags, textured world scenes, and dynamically changing lighting are not implemented.
- Original NIF UV origin and DDS orientation may require adaptation on specific assets; device testing is required.
- This decoder intentionally supports a *subset* of vanilla DDS variants. A clear "unsupported" error is preferable to silently rendering incorrect data.
- The new 48-byte layout must be verified by native CI and physical iPhone GPU. A successful compiler run alone does not prove texture accuracy.

## Exit criteria

1. Pinned native Rust tests (including synthetic DDS fixtures) and iOS ARM64 staticlib build pass.
2. Xcode compiles the Swift and Metal shader and packages an unsigned installable-after-signing IPA.
3. Windows extractor Rust tests/build pass and artifact uploaded.
4. Owner extracts an appropriate DDS from their installed New Vegas, transfers it locally to Files.
5. Owner confirms textured bottle on real A19 Pro with video/screenshot, without source leaks or GPU geometry corruption.
