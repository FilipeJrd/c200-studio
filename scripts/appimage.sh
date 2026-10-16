#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"
if [[ "$(uname -m)" != x86_64 ]]; then
  echo 'This release script currently supports x86_64 only.' >&2
  exit 1
fi

# Build on Ubuntu 22.04 for the supported glibc baseline (see CI).
if [[ "${C200_SKIP_BUILD:-0}" != 1 ]]; then cargo build --release --locked -p c200-app; fi
mkdir -p dist .tools
if [[ -x "$PROJECT_ROOT/.tools/bin/cargo-about" ]]; then export PATH="$PROJECT_ROOT/.tools/bin:$PATH"; fi
APPDIR="$PROJECT_ROOT/dist/C200-Studio.AppDir"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/256x256/apps" "$APPDIR/usr/share/doc/c200-studio"
install -m755 target/release/c200-studio "$APPDIR/usr/bin/c200-studio"
install -m644 assets/io.github.c200studio.C200Studio.desktop "$APPDIR/usr/share/applications/"
install -m644 assets/c200-studio.png "$APPDIR/usr/share/icons/hicolor/256x256/apps/"
install -m644 LICENSE README.md THIRD_PARTY_NOTICES.md "$APPDIR/usr/share/doc/c200-studio/"
mkdir -p "$APPDIR/usr/share/X11"
cp -a /usr/share/X11/locale "$APPDIR/usr/share/X11/"
if [[ -f /usr/share/doc/libx11-data/copyright ]]; then
  install -Dm644 /usr/share/doc/libx11-data/copyright "$APPDIR/usr/share/doc/c200-studio/system-licenses/libx11-data.txt"
fi
if ! command -v cargo-about >/dev/null; then
  echo 'Install the notice generator first: cargo install cargo-about --locked --features cli' >&2
  exit 1
fi
cargo about generate --config about.toml assets/licenses.hbs > "$APPDIR/usr/share/doc/c200-studio/dependency-licenses.html"

LINUXDEPLOY="${C200_LINUXDEPLOY:-$PROJECT_ROOT/.tools/linuxdeploy-x86_64.AppImage}"
if [[ ! -f "$LINUXDEPLOY" ]]; then
  curl --fail --location --retry 3 https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage -o "$LINUXDEPLOY"
fi
chmod +x "$LINUXDEPLOY"
# winit loads these using dlopen, so an ldd-only scan cannot discover them.
GRAPHICS_LIBS=()
for LIBRARY in libxkbcommon.so.0 libxkbcommon-x11.so.0 libwayland-client.so.0 libwayland-cursor.so.0 libwayland-egl.so.1 libX11.so.6 libXcursor.so.1 libXi.so.6 libXrandr.so.2; do
  LIBRARY_PATH="$(ldconfig -p | awk -v name="$LIBRARY" '$1 == name && !printed {print $NF; printed=1}')"
  if [[ -z "$LIBRARY_PATH" ]]; then echo "Missing build dependency: $LIBRARY" >&2; exit 1; fi
  GRAPHICS_LIBS+=(--library "$LIBRARY_PATH")
done
# Extract-and-run also works in containers without FUSE. Host graphics drivers
# and glibc remain system-provided; linuxdeploy applies its standard exclusions.
export APPIMAGE_EXTRACT_AND_RUN=1
export OUTPUT="$PROJECT_ROOT/dist/C200-Studio-x86_64.AppImage"
"$LINUXDEPLOY" --appdir "$APPDIR" --executable "$APPDIR/usr/bin/c200-studio" --desktop-file assets/io.github.c200studio.C200Studio.desktop --icon-file assets/c200-studio.png --custom-apprun assets/AppRun "${GRAPHICS_LIBS[@]}"
# Preserve distribution notices for bundled libraries and their dependencies.
if command -v dpkg-query >/dev/null; then
  for BUNDLED in "$APPDIR"/usr/lib/*.so*; do
    LIBRARY="$(basename "$BUNDLED")"
    SYSTEM_PATH="$(ldconfig -p | awk -v name="$LIBRARY" '$1 == name && !printed {print $NF; printed=1}')"
    [[ -n "$SYSTEM_PATH" ]] || continue
    PACKAGE="$(dpkg-query -S "$SYSTEM_PATH" "$(readlink -f "$SYSTEM_PATH")" 2>/dev/null | head -n1 | cut -d: -f1 || true)"
    if [[ -f "/usr/share/doc/$PACKAGE/copyright" ]]; then
      install -Dm644 "/usr/share/doc/$PACKAGE/copyright" "$APPDIR/usr/share/doc/c200-studio/system-licenses/$PACKAGE.txt"
    fi
  done
fi
"$LINUXDEPLOY" --appdir "$APPDIR" --output appimage
(cd "$PROJECT_ROOT/dist" && sha256sum "$(basename "$OUTPUT")" > "$(basename "$OUTPUT").sha256")
echo "Created $OUTPUT"
