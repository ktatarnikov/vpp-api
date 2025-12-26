#!/usr/bin/env bash
set -euo pipefail

# -------------------------------
# Versions to ignore
# Add folder names (not paths)
# -------------------------------
IGNORED_VERSIONS=(
    "bin"
    "src"
)

# Directory containing this script
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

# Resolve workspace root (via git if possible)
WORKSPACE_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$WORKSPACE_ROOT" ]]; then
    WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
fi

DEST="${SCRIPT_DIR}/../gen"
VPP_SYS_DIR="${WORKSPACE_ROOT}/vpp-native-client-lib-sys"

is_ignored() {
    local name="$1"
    for ignored in "${IGNORED_VERSIONS[@]}"; do
        if [[ "$ignored" == "$name" ]]; then
            return 0
        fi
    done
    return 1
}

generate_api() {
    local VERSION="$1"
    mkdir -p "$DEST/${VERSION}/src"
    cargo run --package vpp-api-gen --bin api-gen -- \
        --in-file "${WORKSPACE_ROOT}/vpp-native-client-lib-sys/${VERSION}/api" \
        --out-file "." \
        --parse-type "Tree" \
        --package-name "${VERSION}" \
        --package-path "${DEST}" \
        --print-message-names \
        --create-binding \
        --create-package \
        --generate-code \
        --verbose \
        --verbose
}

if [[ ! -d "$VPP_SYS_DIR" ]]; then
    echo "Directory not found: $VPP_SYS_DIR" >&2
    exit 1
fi

for dir in "$VPP_SYS_DIR"/*; do
    [[ -d "$dir" ]] || continue
    version="$(basename "$dir")"

    if is_ignored "$version"; then
        echo "Skipping ignored version: $version"
        continue
    fi

    echo "Generating API for version: $version"
    generate_api "$version"
done
