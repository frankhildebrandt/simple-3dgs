#!/usr/bin/env bash
# Build or copy macOS Apple Silicon sidecars into src-tauri/binaries/.
# Build-machine tools (Homebrew, Rust) are not required at runtime.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BINARIES="$ROOT/src-tauri/binaries"
VENDOR="$ROOT/vendor"
TRIPLE="${TRIPLE:-aarch64-apple-darwin}"
BRUSH_REF="${BRUSH_REF:-main}"
FFMPEG_VERSION="${FFMPEG_VERSION:-7.1.1}"
COLMAP_METAL_SHA="${COLMAP_METAL_SHA:-bf01a458b958fbe31fcb67643c44e873e6ec2dd0}"

STUB=0
BUNDLE=0
LGPL_FFMPEG=0
BUILD_BRUSH=0
BUILD_COLMAP_METAL=0

usage() {
  cat <<'EOF'
Usage: scripts/fetch-sidecars.sh [options]

  --stub           Write PATH-forwarding wrappers (dev default)
  --bundle         Copy Homebrew ffmpeg/colmap and make them relocatable
  --lgpl-ffmpeg    Compile an LGPL FFmpeg with VideoToolbox (no --enable-gpl)
  --brush          Clone and cargo-build Brush CLI at BRUSH_REF
  --colmap-metal   Build COLMAP with Metal SIFT (replaces the Homebrew bottle)
  --all            --lgpl-ffmpeg --brush --colmap-metal

Environment: TRIPLE, BRUSH_REF, FFMPEG_VERSION, COLMAP_METAL_SHA
EOF
}

if [[ $# -eq 0 ]]; then
  STUB=1
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --stub) STUB=1 ;;
    --bundle) BUNDLE=1 ;;
    --lgpl-ffmpeg) LGPL_FFMPEG=1 ;;
    --brush) BUILD_BRUSH=1 ;;
    --colmap-metal) BUILD_COLMAP_METAL=1 ;;
    --all) LGPL_FFMPEG=1; BUILD_BRUSH=1; BUILD_COLMAP_METAL=1 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage; exit 1 ;;
  esac
  shift
done

mkdir -p "$BINARIES" "$VENDOR"

install_stub() {
  local name="$1"
  local dest="$BINARIES/${name}-${TRIPLE}"
  cat > "$dest" <<EOF
#!/usr/bin/env bash
set -euo pipefail
cmd="$name"
if [[ "\$cmd" == "brush" ]]; then
  if command -v brush-cli >/dev/null 2>&1; then
    exec brush-cli "\$@"
  fi
fi
if command -v "\$cmd" >/dev/null 2>&1; then
  exec "\$cmd" "\$@"
fi
echo "simple-3dgs: \$cmd is not installed. Run scripts/fetch-sidecars.sh --all" >&2
exit 127
EOF
  chmod +x "$dest"
  echo "stub $dest"
}

relocate() {
  local binary="$1"
  local libdir
  libdir="$(dirname "$binary")/$(basename "$binary")-libs"
  mkdir -p "$libdir"
  if command -v dylibbundler >/dev/null 2>&1; then
    dylibbundler -od -b -x "$binary" -d "$libdir" -p "@executable_path/$(basename "$libdir")/" || true
  else
    echo "warning: dylibbundler not found; $binary may not be relocatable" >&2
  fi
}

copy_from_path() {
  local name="$1"
  local src
  src="$(command -v "$name" || true)"
  if [[ -z "$src" ]]; then
    return 1
  fi
  local dest="$BINARIES/${name}-${TRIPLE}"
  cp "$src" "$dest"
  chmod +x "$dest"
  if [[ "$name" == "colmap" ]]; then
    relocate_colmap "$dest"
  else
    relocate "$dest"
  fi
  echo "copied $src -> $dest"
}

# COLMAP from Homebrew is heavily dylib-linked. Rewrite install names to
# @rpath so the same binary works in `tauri dev` (@executable_path/colmap-libs)
# and in the .app (@executable_path/../Resources/colmap-libs).
relocate_colmap() {
  local binary="$1"
  local libdir="$BINARIES/colmap-libs"
  rm -rf "$libdir"
  mkdir -p "$libdir"
  if ! command -v dylibbundler >/dev/null 2>&1; then
    echo "warning: dylibbundler not found; COLMAP may not be relocatable" >&2
    return 0
  fi
  local search=()
  local dir
  for dir in \
    "$(brew --prefix colmap)/lib" \
    "$(brew --prefix)/lib" \
    "$(brew --prefix qtbase)/lib" \
    "$(brew --prefix qtsvg)/lib"
  do
    if [[ -d "$dir" ]]; then
      search+=(-s "$dir")
    fi
  done
  while IFS= read -r dir; do
    search+=(-s "$dir")
  done < <(find "$(brew --prefix)/opt" -type d \( -name lib -o -name Frameworks \) 2>/dev/null | head -400)

  # stdin is closed so a missing dylib cannot hang on an interactive prompt.
  if ! dylibbundler -od -b -of -x "$binary" -d "$libdir" -p "@rpath/" "${search[@]}" </dev/null; then
    echo "warning: dylibbundler reported errors; COLMAP sidecar may need Homebrew libs" >&2
  fi
  # Rust driver around otool / install_name_tool / codesign (no Python).
  cargo run --manifest-path "$ROOT/tools/colmap-bundle/Cargo.toml" --release --quiet -- "$binary" "$libdir"
  echo "relocated COLMAP dylibs -> $libdir ($(ls "$libdir" | wc -l | tr -d ' ') files)"
}

need_brew() {
  if ! command -v brew >/dev/null 2>&1; then
    echo "Homebrew is required for --bundle / --colmap-metal. Install from https://brew.sh" >&2
    exit 1
  fi
}

# `xcrun metal` needs full Xcode plus the Metal Toolchain component (Xcode 26+).
need_metal_compiler() {
  local dev
  dev="$(xcode-select -p 2>/dev/null || true)"
  if [[ "$dev" != *"/Xcode.app/"* ]]; then
    echo "COLMAP Metal needs full Xcode (not Command Line Tools). $dev" >&2
    echo "Install Xcode from the App Store, then:" >&2
    echo "  sudo xcode-select -s /Applications/Xcode.app/Contents/Developer" >&2
    echo "  sudo xcodebuild -license accept" >&2
    echo "  xcodebuild -runFirstLaunch" >&2
    echo "  xcodebuild -downloadComponent MetalToolchain" >&2
    exit 1
  fi
  local metal_err
  metal_err="$(xcrun -sdk macosx metal 2>&1 || true)"
  if echo "$metal_err" | grep -qi "missing Metal Toolchain"; then
    echo "Xcode is selected, but the Metal shader compiler is a separate component." >&2
    echo "Run:" >&2
    echo "  xcodebuild -runFirstLaunch" >&2
    echo "  xcodebuild -downloadComponent MetalToolchain" >&2
    echo "Or Xcode → Settings → Components → Metal Toolchain." >&2
    echo "Until then, keep SIFT on CPU or use ./scripts/fetch-sidecars.sh --bundle." >&2
    exit 1
  fi
  if ! xcrun -sdk macosx -find metal >/dev/null 2>&1; then
    echo "xcrun cannot find the metal compiler under $dev" >&2
    exit 1
  fi
}

# Build the Metal-SIFT COLMAP fork, relocate dylibs, and drop sift.metallib in colmap-libs.
build_colmap_metal() {
  need_brew
  need_metal_compiler
  brew list cmake >/dev/null 2>&1 || brew install cmake
  brew list ninja >/dev/null 2>&1 || brew install ninja
  brew list colmap >/dev/null 2>&1 || brew install colmap
  brew list dylibbundler >/dev/null 2>&1 || brew install dylibbundler

  local src="$VENDOR/colmap-metal"
  local patch="$ROOT/scripts/patches/sift-metal-metallib-path.patch"
  if [[ ! -d "$src/.git" ]]; then
    git clone https://github.com/byplay-io/colmap-metal.git "$src"
  fi
  git -C "$src" fetch origin "$COLMAP_METAL_SHA" --depth 1 || git -C "$src" fetch origin
  git -C "$src" checkout --force "$COLMAP_METAL_SHA"
  git -C "$src" reset --hard "$COLMAP_METAL_SHA"
  git -C "$src" apply "$patch"
  git -C "$src" apply "$ROOT/scripts/patches/faiss-cpu-matcher-index-flat.patch"
  git -C "$src" apply "$ROOT/scripts/patches/sift-metal-matcher.patch"

  local build="$src/build-simple-3dgs"
  local prefix="$VENDOR/colmap-metal-prefix"
  rm -rf "$build"
  mkdir -p "$build" "$prefix"
  local generator=()
  if command -v ninja >/dev/null 2>&1; then
    generator=(-G Ninja)
  fi
  local openmp=()
  if [[ -d "$(brew --prefix libomp 2>/dev/null || true)" ]]; then
    openmp=(-DOpenMP_ROOT="$(brew --prefix libomp)")
  fi
  # Homebrew FAISS 1.15 headers shadow FetchContent FAISS (`-isystem /opt/homebrew/include`
  # before `_deps/faiss-src`). The Index vtable then disagrees and matcher `add` SIGSEGVs.
  cmake -S "$src" -B "$build" \
    "${generator[@]}" \
    "${openmp[@]}" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$prefix" \
    -DCMAKE_PREFIX_PATH="$(brew --prefix)" \
    -DCMAKE_CXX_FLAGS="-I${build}/_deps/faiss-src" \
    -DBUILD_SHARED_LIBS=OFF \
    -DMETAL_ENABLED=ON \
    -DCUDA_ENABLED=OFF \
    -DGUI_ENABLED=OFF \
    -DOPENGL_ENABLED=OFF \
    -DONNX_ENABLED=OFF \
    -DCGAL_ENABLED=OFF \
    -DMVS_ENABLED=OFF \
    -DLSD_ENABLED=OFF \
    -DDOWNLOAD_ENABLED=OFF \
    -DTESTS_ENABLED=OFF
  cmake --build "$build" --parallel "$(sysctl -n hw.ncpu)"
  cmake --install "$build"

  local dest="$BINARIES/colmap-${TRIPLE}"
  if [[ ! -x "$prefix/bin/colmap" ]]; then
    echo "COLMAP Metal binary not found at $prefix/bin/colmap" >&2
    exit 1
  fi
  cp "$prefix/bin/colmap" "$dest"
  chmod +x "$dest"
  relocate_colmap "$dest"

  local metalib=""
  for candidate in \
    "$prefix/lib/sift.metallib" \
    "$build/src/thirdparty/SiftMetal/sift.metallib"
  do
    if [[ -f "$candidate" ]]; then
      metalib="$candidate"
      break
    fi
  done
  if [[ -z "$metalib" ]]; then
    echo "sift.metallib not found after COLMAP Metal build" >&2
    exit 1
  fi
  mkdir -p "$BINARIES/colmap-libs"
  cp "$metalib" "$BINARIES/colmap-libs/sift.metallib"
  echo "colmap metal -> $dest (sift.metallib in colmap-libs)"
}

if [[ "$STUB" -eq 1 && "$BUNDLE" -eq 0 && "$LGPL_FFMPEG" -eq 0 && "$BUILD_BRUSH" -eq 0 && "$BUILD_COLMAP_METAL" -eq 0 ]]; then
  install_stub ffmpeg
  install_stub colmap
  install_stub brush
  echo "Wrote PATH stubs. Install ffmpeg/colmap/brush on PATH, or re-run with --all."
  exit 0
fi

if [[ "$STUB" -eq 1 ]]; then
  install_stub ffmpeg
  install_stub colmap
  install_stub brush
fi

if [[ "$BUNDLE" -eq 1 ]]; then
  need_brew
  brew list ffmpeg >/dev/null 2>&1 || brew install ffmpeg
  brew list colmap >/dev/null 2>&1 || brew install colmap
  brew list dylibbundler >/dev/null 2>&1 || brew install dylibbundler
  copy_from_path colmap || true
  if [[ ! -x "$BINARIES/ffmpeg-${TRIPLE}" ]] || [[ "$(stat -f%z "$BINARIES/ffmpeg-${TRIPLE}" 2>/dev/null || echo 0)" -lt 1000000 ]]; then
    copy_from_path ffmpeg || true
    echo "note: Homebrew FFmpeg is often GPL. Use --lgpl-ffmpeg for an LGPL build."
  else
    echo "keeping existing FFmpeg sidecar (likely LGPL build)"
  fi
fi

if [[ "$LGPL_FFMPEG" -eq 1 ]]; then
  srcdir="$VENDOR/ffmpeg-${FFMPEG_VERSION}"
  tarball="$VENDOR/ffmpeg-${FFMPEG_VERSION}.tar.xz"
  if [[ ! -d "$srcdir" ]]; then
    curl -L "https://ffmpeg.org/releases/ffmpeg-${FFMPEG_VERSION}.tar.xz" -o "$tarball"
    tar -xJf "$tarball" -C "$VENDOR"
  fi
  prefix="$VENDOR/ffmpeg-prefix"
  mkdir -p "$prefix"
  pushd "$srcdir" >/dev/null
  ./configure \
    --prefix="$prefix" \
    --disable-gpl \
    --disable-nonfree \
    --disable-debug \
    --disable-doc \
    --disable-network \
    --enable-videotoolbox \
    --enable-audiotoolbox \
    --enable-static \
    --disable-shared \
    --pkg-config-flags=--static
  make -j"$(sysctl -n hw.ncpu)"
  make install
  popd >/dev/null
  cp "$prefix/bin/ffmpeg" "$BINARIES/ffmpeg-${TRIPLE}"
  chmod +x "$BINARIES/ffmpeg-${TRIPLE}"
  echo "LGPL ffmpeg -> $BINARIES/ffmpeg-${TRIPLE}"
fi

if [[ "$BUILD_BRUSH" -eq 1 ]]; then
  if [[ ! -d "$VENDOR/brush/.git" ]]; then
    git clone --depth 1 --branch "$BRUSH_REF" https://github.com/ArthurBrussee/brush.git "$VENDOR/brush" \
      || git clone --depth 1 https://github.com/ArthurBrussee/brush.git "$VENDOR/brush"
  fi
  pushd "$VENDOR/brush" >/dev/null
  git fetch --depth 1 origin "$BRUSH_REF" || true
  if ! git checkout "$BRUSH_REF" 2>/dev/null; then
    git checkout -B "$BRUSH_REF" FETCH_HEAD
  fi
  git clean -fdX
  cargo build --release -p brush-cli || cargo build --release -p brush-app
  bin=""
  for candidate in target/release/brush-cli target/release/brush target/release/brush_app; do
    if [[ -x "$candidate" && ! -d "$candidate" ]]; then
      bin="$candidate"
      break
    fi
  done
  if [[ -z "$bin" ]]; then
    echo "Brush CLI binary not found after build" >&2
    exit 1
  fi
  cp "$bin" "$BINARIES/brush-${TRIPLE}"
  chmod +x "$BINARIES/brush-${TRIPLE}"
  popd >/dev/null
  echo "brush -> $BINARIES/brush-${TRIPLE} (from $bin)"
fi

if [[ "$BUILD_COLMAP_METAL" -eq 1 ]]; then
  build_colmap_metal
fi

echo "Sidecars in $BINARIES:"
ls -lh "$BINARIES"/*-"$TRIPLE" 2>/dev/null || true
