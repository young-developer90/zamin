#!/usr/bin/env bash
set -euo pipefail

VERSION="1.7.0"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

usage() {
    echo "Usage: $0 [--luna]"
    echo "  --luna   Bundle GTK4 libraries for portable GUI apps"
    exit 1
}

BUNDLE_GTK=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --luna) BUNDLE_GTK=1 ;;
        *) usage ;;
    esac
    shift
done

if [ "$BUNDLE_GTK" -eq 1 ]; then
    NAME="zamin-${VERSION}-luna"
    FEATURES="--features luna"
    echo "==> Building with luna (GTK4) support..."
else
    NAME="zamin-${VERSION}"
    FEATURES=""
    echo "==> Building without luna (no GUI)..."
fi

OUTDIR="${ROOT}/dist/${NAME}"

cargo build --release ${FEATURES} --bin zamin --bin zamin-rs 2>&1

echo "==> Building C extension modules..."
make -C "${ROOT}/modules" clean all 2>&1

echo "==> Creating package: ${OUTDIR}"
rm -rf "${OUTDIR}"
mkdir -p "${OUTDIR}/bin"
mkdir -p "${OUTDIR}/modules"
mkdir -p "${OUTDIR}/examples"

cp "${ROOT}/target/release/zamin" "${OUTDIR}/bin/"
cp "${ROOT}/target/release/zamin-rs" "${OUTDIR}/bin/"
cp "${ROOT}/modules/"*.so "${OUTDIR}/modules/" 2>/dev/null || true
cp "${ROOT}/examples/"*.zamin "${OUTDIR}/examples/" 2>/dev/null || true

# Bundle shared libraries for portability
echo "==> Bundling shared libraries..."
mkdir -p "${OUTDIR}/lib"
for lib in $(ldd "${OUTDIR}/bin/zamin" 2>/dev/null | grep "=> /" | awk '{print $3}'); do
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
cat > "${OUTDIR}/zamin" << 'LAUNCHER'
#!/usr/bin/env bash
DIR="$(cd "$(dirname "$0")" && pwd)"
export LD_LIBRARY_PATH="${DIR}/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export LUNA_LIB_DIR="${DIR}/lib"
exec "${DIR}/bin/zamin" "$@"
LAUNCHER
chmod +x "${OUTDIR}/zamin"

cat > "${OUTDIR}/zamin-rs" << 'LAUNCHER_RS'
#!/usr/bin/env bash
DIR="$(cd "$(dirname "$0")" && pwd)"
export LD_LIBRARY_PATH="${DIR}/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export LUNA_LIB_DIR="${DIR}/lib"
exec "${DIR}/bin/zamin-rs" "$@"
LAUNCHER_RS
chmod +x "${OUTDIR}/zamin-rs"

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
echo "      ./zamin run examples/hello.zamin"
echo "      ./zamin-rs examples/hello.zamin       # quick runner"
if [ "$BUNDLE_GTK" -eq 1 ]; then
    echo ""
    echo "    Note: GTK4 runtime libraries are bundled, but system deps"
    echo "    like X11/Wayland, fontconfig, and mesa are still required."
fi
