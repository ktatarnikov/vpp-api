#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 4 ]; then
  echo "Usage: $0 <vpp_version> <dest_folder> <distro> <distro_version>"
  echo "Example: $0 25.10-release /opt/vpp-sdk debian bookworm"
  exit 1
fi

VPP_VERSION="$1"
DEST="$(realpath "$2")"
DISTRO="$3"
DISTRO_VERSION="$4"

TMP_ROOT="$(mktemp -d)"

API_DEST="$DEST/api/core"
API_PLUGINS_DEST="$DEST/api/plugins"
mkdir -p "$API_DEST" "$API_PLUGINS_DEST"

# arch mapping
declare -A MAP_ARCH
MAP_ARCH["amd64"]="x86_64"
MAP_ARCH["arm64"]="aarch64"

ARCHES=("amd64" "arm64")

download_and_extract() {
  local ARCH="$1"
  local MAPPED_ARCH="${MAP_ARCH[$ARCH]}"

  local TMP_DIR="$TMP_ROOT/$ARCH"
  local EXTRACT_DIR="$TMP_DIR/extract"
  mkdir -p "$EXTRACT_DIR"

  local LIB_DEST="$DEST/lib/$MAPPED_ARCH"
  mkdir -p "$LIB_DEST"

  # ------------------------------------------------------------
  # base VPP package
  # ------------------------------------------------------------
  local DEB_NAME="vpp_${VPP_VERSION}_${ARCH}.deb"
  local URL="https://packagecloud.io/fdio/release/packages/${DISTRO}/${DISTRO_VERSION}/${DEB_NAME}/download.deb"

  echo ""
  echo "=== Processing arch: $ARCH  -> $MAPPED_ARCH ==="
  echo "Downloading VPP package:"
  echo "  $URL"

  wget -q -O "$TMP_DIR/$DEB_NAME" "$URL"
  echo "Extracting vpp-core package into $EXTRACT_DIR ..."
  dpkg-deb -x "$TMP_DIR/$DEB_NAME" "$EXTRACT_DIR"

  # ------------------------------------------------------------
  # copy core + base API JSONs
  # ------------------------------------------------------------
  local API_SRC="$EXTRACT_DIR/usr/share/vpp/api"

  if [ -d "$API_SRC" ]; then
    echo "Copying core + base API JSONs -> $API_DEST"
    find "$API_SRC" -maxdepth 2 -name '*.json' -exec cp -f {} "$API_DEST/" \;
  else
    echo "WARNING: API directory not found in $DEB_NAME"
  fi

  # ------------------------------------------------------------
  # copy vppapiclient.so
  # ------------------------------------------------------------
  echo "Searching for vppapiclient.so..."
  local SO_PATH
  SO_PATH=$(find "$EXTRACT_DIR/usr/lib" -type f \
    \( -name "*vppapiclient*.so*" -o -name "libvppapiclient.so*" \) \
    | head -1 || true)

  if [ -z "$SO_PATH" ]; then
    echo "ERROR: vppapiclient.so not found for $ARCH"
    exit 2
  fi

  echo "Copying client lib -> $LIB_DEST/"
  cp -f "$SO_PATH" "$LIB_DEST/"

  # ------------------------------------------------------------
  # download + extract plugin package (dpdk)
  # ------------------------------------------------------------
  local PLUGIN_DEB="vpp-plugin-core_${VPP_VERSION}_${ARCH}.deb"
  local PLUGIN_URL="https://packagecloud.io/fdio/release/packages/${DISTRO}/${DISTRO_VERSION}/${PLUGIN_DEB}/download.deb"

  echo "Downloading plugin package:"
  echo "  $PLUGIN_URL"

  wget -q -O "$TMP_DIR/$PLUGIN_DEB" "$PLUGIN_URL"

  local PLUGIN_EXTRACT="$TMP_DIR/plugin_extract"
  mkdir -p "$PLUGIN_EXTRACT"
  echo "Extracting vpp-plugin package into $PLUGIN_EXTRACT ..." 
  dpkg-deb -x "$TMP_DIR/$PLUGIN_DEB" "$PLUGIN_EXTRACT"

  local PLUGIN_API_SRC="$PLUGIN_EXTRACT/usr/share/vpp/api"

  if [ -d "$PLUGIN_API_SRC" ]; then
    echo "Copying plugin API JSONs -> $API_PLUGINS_DEST"
    find "$PLUGIN_API_SRC" -maxdepth 2 -name '*.json' -exec cp -f {} "$API_PLUGINS_DEST/" \;
  else
    echo "WARNING: Plugin API directory not found in $PLUGIN_DEB"
  fi
}

for ARCH in "${ARCHES[@]}"; do
  download_and_extract "$ARCH"
done

echo ""
echo "=== Completed ==="
echo "Core API        -> $API_DEST"
echo "Plugin API      -> $API_PLUGINS_DEST"
echo "Libraries under -> $DEST/lib/{x86_64,aarch64}"
