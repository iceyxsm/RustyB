#!/bin/bash
# ============================================
# Rusty Browser - Linux Build Script
# ============================================

set -e  # Exit on error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=========================================="
echo "Rusty Browser - Linux Build"
echo "=========================================="
echo ""

# Check prerequisites
echo "Checking prerequisites..."

# Check Rust
if ! command -v rustc &> /dev/null; then
    echo "Error: Rust is not installed. Please install Rust first."
    echo "Visit: https://rustup.rs/"
    exit 1
fi

# Check nightly toolchain
if ! rustup toolchain list | grep -q nightly; then
    echo "Installing nightly toolchain..."
    rustup toolchain install nightly
fi

# Check system dependencies
echo "Checking system dependencies..."

MISSING_DEPS=""

# Check for webkit2gtk-4.1 (preferred) or 4.0 (fallback)
if pkg-config --exists webkit2gtk-4.1; then
    WEBKIT_VERSION="4.1"
elif pkg-config --exists webkit2gtk-4.0; then
    WEBKIT_VERSION="4.0"
    echo "Note: webkit2gtk-4.0 found (4.1 preferred)"
else
    MISSING_DEPS="$MISSING_DEPS\n  - webkit2gtk-4.1-dev (or webkit2gtk-4.0-dev)"
fi

# Check for gtk3
if ! pkg-config --exists gtk+-3.0; then
    MISSING_DEPS="$MISSING_DEPS\n  - gtk3-dev (libgtk-3-dev)"
fi

# Check for libsoup
if ! pkg-config --exists libsoup-3.0; then
    MISSING_DEPS="$MISSING_DEPS\n  - libsoup-3.0-dev"
fi

# Check for additional commonly required dependencies
for pkg in glib-2.0 cairo-1.0 pango-1.0 gdk-pixbuf-2.0; do
    if ! pkg-config --exists $pkg; then
        MISSING_DEPS="$MISSING_DEPS\n  - lib${pkg}-dev"
    fi
done

if [ -n "$MISSING_DEPS" ]; then
    echo "Error: Missing system dependencies:"
    echo -e "$MISSING_DEPS"
    echo ""
    echo "Install on Ubuntu/Debian:"
    echo "  sudo apt-get update"
    echo "  sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libglib2.0-dev libcairo2-dev libpango1.0-dev libgdk-pixbuf2.0-dev"
    echo ""
    echo "Install on Fedora/RHEL:"
    echo "  sudo dnf install webkit2gtk4.1-devel gtk3-devel libsoup3-devel glib2-devel cairo-devel pango-devel gdk-pixbuf2-devel"
    echo ""
    echo "Install on Arch:"
    echo "  sudo pacman -S webkit2gtk-4.1 gtk3 libsoup3 glib2 cairo pango gdk-pixbuf"
    exit 1
fi

echo "All prerequisites satisfied!"
echo ""

# Apply Linux config
echo "Applying Linux build configuration..."
cp "$SCRIPT_DIR/config.toml" "$PROJECT_ROOT/.cargo/config.toml"

# Navigate to project root
cd "$PROJECT_ROOT"

# Clean previous builds (optional)
if [ "$1" == "--clean" ]; then
    echo "Cleaning previous builds..."
    cargo clean
fi

# Build
echo ""
echo "Building Rusty Browser for Linux..."
echo "This may take 10-20 minutes on first build (downloads dependencies)"
echo ""

cargo +nightly build --release -p browser-ui -p rusty-browser-webview

# Check if build succeeded
if [ ! -f "$PROJECT_ROOT/target/release/rusty-browser" ]; then
    echo "Error: Build failed - main binary not found"
    exit 1
fi

if [ ! -f "$PROJECT_ROOT/target/release/rusty-browser-webview" ]; then
    echo "Error: Build failed - webview binary not found"
    exit 1
fi

echo ""
echo "=========================================="
echo "Build Successful!"
echo "=========================================="
echo ""
echo "Binaries location:"
echo "  - Main:    $PROJECT_ROOT/target/release/rusty-browser"
echo "  - WebView: $PROJECT_ROOT/target/release/rusty-browser-webview"
echo ""
echo "To run:"
echo "  cd $PROJECT_ROOT/target/release"
echo "  ./rusty-browser"
echo ""
echo "Or run from anywhere (both must be in same directory):"
echo "  ./target/release/rusty-browser"
echo ""
