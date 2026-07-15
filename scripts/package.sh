#!/usr/bin/env bash
set -euo pipefail

VERSION="1.6.2"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

usage() {
    echo "Usage: $0 [--panther]"
    echo "  --panther   Bundle GTK4 libraries for portable GUI apps"
    exit 1
}

BUNDLE_GTK=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --panther) BUNDLE_GTK=1 ;;
        *) usage ;;
    esac
    shift
done

if [ "$BUNDLE_GTK" -eq 1 ]; then
    NAME="lion-${VERSION}-panther"
    FEATURES="--features panther"
    echo "==> Building with panther (GTK4) support..."
else
    NAME="lion-${VERSION}"
    FEATURES=""
    echo "==> Building without panther (no GUI)..."
fi

OUTDIR="${ROOT}/dist/${NAME}"

cargo build --release ${FEATURES} --bin lion 2>&1

echo "==> Building C extension modules..."
make -C "${ROOT}/modules" clean all 2>&1

echo "==> Creating package: ${OUTDIR}"
rm -rf "${OUTDIR}"
mkdir -p "${OUTDIR}/bin"
mkdir -p "${OUTDIR}/modules"
mkdir -p "${OUTDIR}/examples"

cp "${ROOT}/target/release/lion" "${OUTDIR}/bin/"
cp "${ROOT}/modules/"*.so "${OUTDIR}/modules/" 2>/dev/null || true
cp "${ROOT}/examples/"*.lion "${OUTDIR}/examples/" 2>/dev/null || true

# Bundle shared libraries for portability
echo "==> Bundling shared libraries..."
mkdir -p "${OUTDIR}/lib"
for lib in $(ldd "${OUTDIR}/bin/lion" 2>/dev/null | grep "=> /" | awk '{print $3}'); do
    base="$(basename "$lib")"
    case "$base" in
        linux-*.so*|ld-*.so*|libc.so*|libm.so*|libdl.so*|libpthread.so*|librt.so*|libutil.so*|libanl.so*|libBrokenLocale.so*)
            continue ;;
    esac
    if [ ! -f "${OUTDIR}/lib/$base" ]; then
        cp -L "$lib" "${OUTDIR}/lib/$base" 2>/dev/null || true
    fi
done

# Bundle GTK icon/font config if available
if command -v gtk4-update-icon-cache &>/dev/null; then
    GTK_DATA="$(pkg-config --variable=prefix gtk4 2>/dev/null)/share"
    [ -d "$GTK_DATA" ] && cp -rL "$GTK_DATA" "${OUTDIR}/share/" 2>/dev/null || true
fi

echo "==> Creating launcher..."
cat > "${OUTDIR}/lion" << 'LAUNCHER'
#!/usr/bin/env bash
DIR="$(cd "$(dirname "$0")" && pwd)"
export LD_LIBRARY_PATH="${DIR}/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export PANTHER_LIB_DIR="${DIR}/lib"
exec "${DIR}/bin/lion" "$@"
LAUNCHER
chmod +x "${OUTDIR}/lion"

echo "==> Creating tarball..."
cd "${ROOT}/dist"
tar czf "${NAME}.tar.gz" "${NAME}/"

echo "==> Done!"
echo "    Package: dist/${NAME}.tar.gz"
echo "    Size:    $(du -sh ${NAME}.tar.gz | cut -f1)"
echo ""
echo "    On target machine:"
echo "      tar xzf dist/${NAME}.tar.gz"
echo "      cd ${NAME}"
echo "      ./lion run examples/hello.lion"
if [ "$BUNDLE_GTK" -eq 1 ]; then
    echo ""
    echo "    Note: GTK4 runtime libraries are bundled, but system deps"
    echo "    like X11/Wayland, fontconfig, and mesa are still required."
fi
