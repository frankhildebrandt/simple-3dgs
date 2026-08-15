# Third-party notices

simple-3dgs is licensed under Apache-2.0. Bundled tools are separate
executables (aggregates), not linked into the app. Their licenses still
apply to those binaries.

## FFmpeg

- Project: https://ffmpeg.org/
- License: LGPL-2.1-or-later (bundled build is configured **without** `--enable-gpl`)
- Role: extract still frames from video (VideoToolbox decode on macOS)

LGPL requires that the FFmpeg binary remain replaceable. On macOS the
sidecar lives next to the app executable as `ffmpeg`. You may swap it for
another LGPL-compatible FFmpeg build with the same filename.

Source for the bundled build is fetched by `scripts/fetch-sidecars.sh`.

## COLMAP

- Project: https://github.com/colmap/colmap
- License: BSD-3-Clause
- Role: structure-from-motion (camera poses and sparse point cloud)

COLMAP also redistributes third-party libraries (Ceres, Boost, Eigen,
SQLite, …) under their own permissive licenses. Those travel with the
relocatable sidecar produced by `scripts/fetch-sidecars.sh`.

## Brush

- Project: https://github.com/ArthurBrussee/brush
- License: Apache-2.0
- Role: 3D Gaussian Splatting training and PLY export (wgpu / Metal)

Pinned via `BRUSH_REF` in `scripts/fetch-sidecars.sh` (default: `main`, which ships the headless `brush-cli` binary).

## Spark

- Project: https://github.com/sparkjsdev/spark
- License: MIT
- Role: in-app Gaussian splat viewer (WebGL2 / Three.js)

## Three.js

- Project: https://github.com/mrdoob/three.js
- License: MIT
- Role: WebGL renderer used by Spark

## Intentionally not bundled

The original Inria/MPII Gaussian Splatting implementation is **not**
OSI-open (non-commercial research license). Nerfstudio / gsplat are
CUDA/Python stacks and are not a macOS sidecar.

OpenSplat (AGPLv3) and LichtFeld Studio (GPLv3) are license-compatible
as **separate executables**. They are not in this tree: LichtFeld needs
NVIDIA CUDA; OpenSplat is an optional Metal trainer, not the current
Brush path. Linking either into the Tauri binary would relicense the
combined work (AGPL-3 if OpenSplat is linked).
