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

STUB=0
BUNDLE=0
LGPL_FFMPEG=0
BUILD_BRUSH=0

usage() {
  cat <<'EOF'
Usage: scripts/fetch-sidecars.sh [options]

  --stub           Write PATH-forwarding wrappers (dev default)
  --bundle         Copy Homebrew ffmpeg/colmap and make them relocatable
  --lgpl-ffmpeg    Compile an LGPL FFmpeg with VideoToolbox (no --enable-gpl)
  --brush          Clone and cargo-build Brush CLI at BRUSH_REF
  --all            --bundle --lgpl-ffmpeg --brush

Environment: TRIPLE, BRUSH_REF, FFMPEG_VERSION
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
    --all) BUNDLE=1; LGPL_FFMPEG=1; BUILD_BRUSH=1 ;;
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
    echo "Homebrew is required for --bundle. Install from https://brew.sh" >&2
    exit 1
  fi
}

if [[ "$STUB" -eq 1 && "$BUNDLE" -eq 0 && "$LGPL_FFMPEG" -eq 0 && "$BUILD_BRUSH" -eq 0 ]]; then
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

echo "Sidecars in $BINARIES:"
ls -lh "$BINARIES"/*-"$TRIPLE" 2>/dev/null || true
