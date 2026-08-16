Sidecar binaries for macOS Apple Silicon live here, named with the
Tauri target triple:

- `ffmpeg-aarch64-apple-darwin`
- `colmap-aarch64-apple-darwin`
- `brush-aarch64-apple-darwin`

`colmap-libs/` holds relocated dylibs. A Metal COLMAP build also drops
`sift.metallib` there so feature extraction can find the shaders next to
the sidecar (`tauri dev`) or under `Resources/colmap-libs` in the `.app`.

They are gitignored. Produce them with:

```sh
./scripts/fetch-sidecars.sh --all
```

`--bundle` copies Homebrew COLMAP (CPU SIFT). `--colmap-metal` builds the
Metal fork and needs full Xcode plus `xcodebuild -downloadComponent MetalToolchain`.
Named presets stay on CPU; Expert/Custom can switch SIFT to Metal.
Matching stays CPU Eigen brute-force unless SIFT is Metal, which uses a
GPU matcher in `sift.metallib`. Rebuild with `--colmap-metal` so Homebrew
FAISS 1.15 headers do not shadow the bundled FAISS include path.
