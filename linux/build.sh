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

# Check Cranelift component
if ! rustup component list --toolchain nightly | grep -q "rustc-codegen-cranelift-preview.*installed"; then
    echo "Installing Cranelift codegen backend..."
    rustup component add rustc-codegen-cranelift-preview --toolchain nightly
fi

# Check system dependencies
echo "Checking system dependencies..."

MISSING_DEPS=""

# Check for webkit2gtk-4.1
if ! pkg-config --exists webkit2gtk-4.1; then
    MISSING_DEPS="$MISSING_DEPS\n  - webkit2gtk-4.1-dev (libwebkit2gtk-4.1-dev)"
fi

# Check for gtk3
if ! pkg-config --exists gtk+-3.0; then
    MISSING_DEPS="$MISSING_DEPS\n  - gtk3-dev (libgtk-3-dev)"
fi

# Check for libsoup
if ! pkg-config --exists libsoup-3.0; then
    MISSING_DEPS="$MISSING_DEPS\n  - libsoup-3.0-dev"
fi

if [ -n "$MISSING_DEPS" ]; then
    echo "Error: Missing system dependencies:"
    echo -e "$MISSING_DEPS"
    echo ""
    echo "Install on Ubuntu/Debian:"
    echo "  sudo apt-get update"
    echo "  sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev"
    echo ""
    echo "Install on Fedora/RHEL:"
    echo "  sudo dnf install webkit2gtk4.1-devel gtk3-devel libsoup3-devel"
    echo ""
    echo "Install on Arch:"
    echo "  sudo pacman -S webkit2gtk-4.1 gtk3 libsoup3"
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
echo "This may take 10-20 minutes on first build (downloads CEF/WebKit)"
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

# Restore Windows config (optional - comment out if you want to keep Linux config)
# Uncomment the next line if you want to auto-restore Windows config after build
# git checkout .cargo/config.toml 2>/dev/null || true
