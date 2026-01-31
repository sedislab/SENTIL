set -euo pipefail

VERSION="${SENTIL_VERSION:-0.3.0}"
REPO="sedislab/SENTIL"
DEST="${1:-prebuilt-core}"

os="$(uname -s)"
arch="$(uname -m)"
case "$os-$arch" in
  Linux-x86_64)  target="linux-x86_64";  ext="so" ;;
  Darwin-x86_64) target="macos-x86_64";  ext="dylib" ;;
  Darwin-arm64)  target="macos-arm64";   ext="dylib" ;;
  Linux-aarch64|Linux-arm64)
    echo "For aarch64 Linux, including the Raspberry Pi, see packaging/pi/INSTALL.md." >&2
    exit 1 ;;
  *)
    echo "No prebuilt bundle for $os-$arch. Build the core from source with:" >&2
    echo "  cargo build --release -p sentil-ffi" >&2
    exit 1 ;;
esac

bundle="sentil-${VERSION}-${target}"
url="https://github.com/${REPO}/releases/download/v${VERSION}/${bundle}.tar.gz"

mkdir -p "$DEST"
echo "Fetching $url"
if ! curl -fsSL "$url" -o "$DEST/${bundle}.tar.gz"; then
  echo "Could not download $url." >&2
  echo "A release for v${VERSION} may not be published yet. Build from source with:" >&2
  echo "  cargo build --release -p sentil-ffi" >&2
  exit 1
fi

sums_url="https://github.com/${REPO}/releases/download/v${VERSION}/SHA256SUMS"
if ! curl -fsSL "$sums_url" -o "$DEST/SHA256SUMS"; then
  echo "Could not download $sums_url, so ${bundle}.tar.gz cannot be verified." >&2
  echo "Build from source instead:  cargo build --release -p sentil-ffi" >&2
  rm -f "$DEST/${bundle}.tar.gz"
  exit 1
fi
if ! (cd "$DEST" && grep " ${bundle}.tar.gz\$" SHA256SUMS | sha256sum -c -) >/dev/null 2>&1; then
  echo "Checksum mismatch for ${bundle}.tar.gz. Not unpacking it." >&2
  rm -f "$DEST/${bundle}.tar.gz"
  exit 1
fi
echo "Checksum verified."

tar -C "$DEST" -xzf "$DEST/${bundle}.tar.gz"
libdir="$(cd "$DEST/${bundle}/lib" && pwd)"
echo "Installed $libdir/libsentil.${ext}"
echo
echo "Point a binding at it:"
echo "  export SENTIL_LIB=$libdir/libsentil.${ext}"
echo "  export SENTIL_LIB_DIR=$libdir"