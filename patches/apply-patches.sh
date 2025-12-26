#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

WGPU_TAG="${WGPU_TAG:-wgpu-v27.0.1}"
GLOW_TAG="${GLOW_TAG:-0.16.0}"

WGPU_DIR="$PROJECT_ROOT/crates/wgpu-patched"
GLOW_DIR="$WGPU_DIR/glow"

echo "=== Applying patches ==="
echo "WGPU tag: $WGPU_TAG"
echo "GLOW tag: $GLOW_TAG"
echo ""

# Clone or reset wgpu
if [ -d "$WGPU_DIR/.git" ]; then
    echo "Resetting existing wgpu..."
    cd "$WGPU_DIR"
    git fetch origin
    git checkout "$WGPU_TAG"
    git reset --hard "$WGPU_TAG"
else
    echo "Cloning wgpu..."
    rm -rf "$WGPU_DIR"
    git clone --depth 1 --branch "$WGPU_TAG" https://github.com/gfx-rs/wgpu.git "$WGPU_DIR"
fi

# Clone or reset glow
if [ -d "$GLOW_DIR/.git" ]; then
    echo "Resetting existing glow..."
    cd "$GLOW_DIR"
    git fetch origin
    git checkout "$GLOW_TAG"
    git reset --hard "$GLOW_TAG"
else
    echo "Cloning glow..."
    rm -rf "$GLOW_DIR"
    git clone --depth 1 --branch "$GLOW_TAG" https://github.com/grovesNL/glow.git "$GLOW_DIR"
fi

# Apply patches
echo ""
echo "Applying wgpu patch..."
cd "$WGPU_DIR"
git apply "$SCRIPT_DIR/wgpu.patch" || {
    echo "ERROR: Failed to apply wgpu.patch"
    echo "Manual conflict resolution may be needed for newer wgpu versions"
    exit 1
}

echo "Applying glow patch..."
cd "$GLOW_DIR"
git apply "$SCRIPT_DIR/glow.patch" || {
    echo "ERROR: Failed to apply glow.patch"
    echo "Manual conflict resolution may be needed for newer glow versions"
    exit 1
}

echo ""
echo "=== Done! ==="
echo "wgpu: $WGPU_DIR"
echo "glow: $GLOW_DIR"
