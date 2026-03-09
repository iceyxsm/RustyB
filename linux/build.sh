#!/bin/bash
# ============================================
# Rusty Browser - Linux Build Script
# Auto-installs dependencies and builds
# ============================================

set -e  # Exit on error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

AUTO_INSTALL=false
if [ "$1" == "--install" ] || [ "$1" == "-i" ]; then
    AUTO_INSTALL=true
fi

echo "=========================================="
echo "Rusty Browser - Linux Build"
echo "=========================================="
echo ""

# Detect distro
if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO=$ID
else
    DISTRO="unknown"
fi

echo "Detected distro: $DISTRO"
echo ""

# Install function
install_packages() {
    case $DISTRO in
        ubuntu|debian)
            echo "Installing packages via apt..."
            sudo apt-get update
            sudo apt-get install -y \
                libwebkit2gtk-4.1-dev \
                libgtk-3-dev \
                libsoup-3.0-dev \
                libglib2.0-dev \
                libcairo2-dev \
                libpango1.0-dev \
                libgdk-pixbuf2.0-dev \
                libssl-dev \
                pkg-config \
                build-essential \
                curl
            ;;
        fedora|rhel|centos)
            echo "Installing packages via dnf..."
            sudo dnf install -y \
                webkit2gtk4.1-devel \
                gtk3-devel \
                libsoup3-devel \
                glib2-devel \
                cairo-devel \
                pango-devel \
                gdk-pixbuf2-devel \
                openssl-devel \
                pkgconf \
                gcc \
                curl
            ;;
        arch|manjaro)
            echo "Installing packages via pacman..."
            sudo pacman -Sy --noconfirm \
                webkit2gtk-4.1 \
                gtk3 \
                libsoup3 \
                glib2 \
                cairo \
                pango \
                gdk-pixbuf2 \
                openssl \
                pkgconf \
                base-devel \
                curl
            ;;
        *)
            echo "Unknown distro. Please install packages manually."
            return 1
            ;;
    esac
}

# Check prerequisites
echo "Checking prerequisites..."

# Check Rust
if ! command -v rustc &> /dev/null; then
    echo "Rust not found. Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Check nightly toolchain
if ! rustup toolchain list 2>/dev/null | grep -q nightly; then
    echo "Installing nightly toolchain..."
    rustup toolchain install nightly
fi

# Check for Cranelift (optional)
if ! rustup component list --toolchain nightly 2>/dev/null | grep -q "rustc-codegen-cranelift.*installed"; then
    echo "Installing Cranelift codegen backend..."
    rustup component add rustc-codegen-cranelift-preview --toolchain nightly 2>/dev/null || true
fi

# Check system dependencies
echo "Checking system dependencies..."

MISSING_DEPS=""

# Check for pkg-config
if ! command -v pkg-config &> /dev/null; then
    MISSING_DEPS="$MISSING_DEPS pkg-config"
fi

# Check for webkit2gtk
if pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
    WEBKIT_VERSION="4.1"
elif pkg-config --exists webkit2gtk-4.0 2>/dev/null; then
    WEBKIT_VERSION="4.0"
    echo "Note: webkit2gtk-4.0 found (4.1 preferred)"
else
    MISSING_DEPS="$MISSING_DEPS webkit2gtk"
fi

# Check for other required packages
for pkg in gtk+-3.0 libsoup-3.0 glib-2.0 cairo pango gdk-pixbuf-2.0; do
    if ! pkg-config --exists $pkg 2>/dev/null; then
        MISSING_DEPS="$MISSING_DEPS $pkg"
    fi
done

# Check for OpenSSL (needed for reqwest, hyper)
if ! pkg-config --exists openssl 2>/dev/null; then
    MISSING_DEPS="$MISSING_DEPS openssl"
fi

# Install missing packages if auto-install enabled
if [ -n "$MISSING_DEPS" ]; then
    echo "Missing dependencies:$MISSING_DEPS"
    
    if [ "$AUTO_INSTALL" = true ]; then
        echo ""
        echo "Auto-installing dependencies..."
        install_packages
    else
        echo ""
        echo "Install command:"
        case $DISTRO in
            ubuntu|debian)
                echo "  sudo apt-get update"
                echo "  sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libglib2.0-dev libcairo2-dev libpango1.0-dev libgdk-pixbuf2.0-dev libssl-dev pkg-config build-essential"
                ;;
            fedora|rhel|centos)
                echo "  sudo dnf install -y webkit2gtk4.1-devel gtk3-devel libsoup3-devel glib2-devel cairo-devel pango-devel gdk-pixbuf2-devel openssl-devel pkgconf"
                ;;
            arch|manjaro)
                echo "  sudo pacman -S webkit2gtk-4.1 gtk3 libsoup3 glib2 cairo pango gdk-pixbuf2 openssl pkgconf base-devel"
                ;;
        esac
        echo ""
        echo "Or run: bash linux/build.sh --install"
        exit 1
    fi
fi

echo "All prerequisites satisfied!"
echo ""

# Apply Linux config
echo "Applying Linux build configuration..."
cp "$SCRIPT_DIR/config.toml" "$PROJECT_ROOT/.cargo/config.toml"

# Navigate to project root
cd "$PROJECT_ROOT"

# Clean previous builds (optional)
if [ "$1" == "--clean" ] || [ "$2" == "--clean" ]; then
    echo "Cleaning previous builds..."
    cargo clean
fi

# Build
echo ""
echo "Building Rusty Browser for Linux..."
echo "This may take 10-30 minutes on first build"
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

# Copy to binaries folder
mkdir -p "$PROJECT_ROOT/binaries/linux"
cp "$PROJECT_ROOT/target/release/rusty-browser" "$PROJECT_ROOT/binaries/linux/rusty-browser-linux"
cp "$PROJECT_ROOT/target/release/rusty-browser-webview" "$PROJECT_ROOT/binaries/linux/rusty-browser-webview-linux"
echo "Copied to: $PROJECT_ROOT/binaries/linux/"
echo ""
