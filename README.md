# Simple 3DGS

Desktop app that turns a video (or a folder of stills) into a 3D Gaussian
splat. macOS Apple Silicon only.

```
Video → FFmpeg frames → COLMAP poses → Brush training → Spark viewer
```

No Homebrew on the end-user machine. Sidecars are copied into the `.app`.

## Requirements

- macOS 14+ on Apple Silicon
- 16 GB unified memory minimum (24 GB+ is more comfortable)
- For **building**: Node 22+, Rust 1.88+, Xcode CLT
- For **bundling sidecars**: Homebrew (COLMAP), Rust (Brush + `tools/colmap-bundle`), network (FFmpeg source)

## Develop

```sh
./scripts/fetch-sidecars.sh          # PATH stubs so Tauri can start
npm install
npm run test:rust
npm run tauri dev
```

Stubs forward to `ffmpeg`, `colmap`, and `brush`/`brush-cli` on your PATH.
To ship a self-contained app:

```sh
./scripts/fetch-sidecars.sh --all
npm run tauri build
```

`--all` compiles an **LGPL** FFmpeg with VideoToolbox (no `--enable-gpl`),
copies a relocatable COLMAP 4.1 from Homebrew (dylibs + Qt frameworks),
and builds Brush `brush-cli` from `main`. The resulting sidecars stay
gitignored under `src-tauri/binaries/`.

## Use

1. Choose a capture type: Object (orbit), Room, or Outdoor.
2. Drop a video (slow motion, overlap, no zoom) or pick an image folder.
3. Choose a project folder and a preset: Fast / Balanced / Quality.
4. Click **Reconstruct**. Quality can take hours.
5. Fly the splat in the viewer. Output is `project/output/scene.ply`.

If COLMAP cannot recover cameras, reshoot: slower motion, more overlap,
less blur. Rooms need textured walls; outdoors, tilt down off empty sky.

## License

Apache-2.0 for the app. Bundled tools are separate executables; see
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and [NOTICE](NOTICE).

GPL-3.0 and AGPL-3.0 sidecars are allowed when they form a compatible
set with the rest of the bundle (GPL-3 / AGPL-3 / LGPL / permissive).
GPLv2 is not: it conflicts with Apache-2.0.

The FFmpeg sidecar is LGPL and replaceable: swap `ffmpeg` next to the
app binary with another LGPL build.

Not included: Inria Gaussian Splatting (research license, not OSI),
Nerfstudio / gsplat (CUDA / Python, not a Mac sidecar).
