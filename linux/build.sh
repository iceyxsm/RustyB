#!/bin/bash
# ============================================
# Rusty Browser - Linux Build Script
# Auto-detects and installs missing dependencies
# ============================================

set -e  # Exit on error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

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

# Package definitions
UBUNTU_PACKAGES=(
    "pkg-config:pkg-config"
    "libwebkit2gtk-4.1-dev:webkit2gtk-4.1"
    "libgtk-3-dev:gtk+-3.0"
    "libsoup-3.0-dev:libsoup-3.0"
    "libglib2.0-dev:glib-2.0"
    "libcairo2-dev:cairo"
    "libpango1.0-dev:pango"
    "libgdk-pixbuf2.0-dev:gdk-pixbuf-2.0"
    "libssl-dev:openssl"
    "build-essential:"
)

FEDORA_PACKAGES=(
    "pkgconf:pkg-config"
    "webkit2gtk4.1-devel:webkit2gtk-4.1"
    "gtk3-devel:gtk+-3.0"
    "libsoup3-devel:libsoup-3.0"
    "glib2-devel:glib-2.0"
    "cairo-devel:cairo"
    "pango-devel:pango"
    "gdk-pixbuf2-devel:gdk-pixbuf-2.0"
    "openssl-devel:openssl"
    "gcc:"
)

ARCH_PACKAGES=(
    "pkgconf:pkg-config"
    "webkit2gtk-4.1:webkit2gtk-4.1"
    "gtk3:gtk+-3.0"
    "libsoup3:libsoup-3.0"
    "glib2:glib-2.0"
    "cairo:cairo"
    "pango:pango"
    "gdk-pixbuf2:gdk-pixbuf-2.0"
    "openssl:openssl"
    "base-devel:"
)

# Function to check if package is installed (by pkg-config name or command)
check_package() {
    local pkg_config_name="$1"
    local cmd_name="$2"
    
    # If pkg-config name provided, check with pkg-config
    if [ -n "$pkg_config_name" ]; then
        if pkg-config --exists "$pkg_config_name" 2>/dev/null; then
            return 0  # Found
        fi
    fi
    
    # If command name provided, check command exists
    if [ -n "$cmd_name" ]; then
        if command -v "$cmd_name" &> /dev/null; then
            return 0  # Found
        fi
    fi
    
    return 1  # Not found
}

# Function to install packages based on distro
install_packages() {
    local packages=("$@")
    local to_install=()
    
    echo "Checking required packages..."
    echo ""
    
    for pkg_def in "${packages[@]}"; do
        IFS=':' read -r pkg_name pkg_check <<< "$pkg_def"
        
        if check_package "$pkg_check" "$pkg_check"; then
            echo "  ✓ $pkg_name (found)"
        else
            echo "  ✗ $pkg_name (missing)"
            to_install+=("$pkg_name")
        fi
    done
    
    if [ ${#to_install[@]} -eq 0 ]; then
        echo ""
        echo "All dependencies satisfied!"
        return 0
    fi
    
    echo ""
    echo "Installing missing packages: ${to_install[*]}"
    echo ""
    
    case $DISTRO in
        ubuntu|debian)
            sudo apt-get update
            sudo apt-get install -y "${to_install[@]}"
            ;;
        fedora|rhel|centos)
            sudo dnf install -y "${to_install[@]}"
            ;;
        arch|manjaro)
            sudo pacman -Sy --noconfirm "${to_install[@]}"
            ;;
        *)
            echo "Error: Unknown distro '$DISTRO'"
            echo "Please install manually: ${to_install[*]}"
            return 1
            ;;
    esac
    
    echo ""
    echo "Packages installed successfully!"
}

# Install dependencies
echo "Checking and installing dependencies..."
echo ""

case $DISTRO in
    ubuntu|debian)
        install_packages "${UBUNTU_PACKAGES[@]}"
        ;;
    fedora|rhel|centos)
        install_packages "${FEDORA_PACKAGES[@]}"
        ;;
    arch|manjaro)
        install_packages "${ARCH_PACKAGES[@]}"
        ;;
    *)
        echo "Warning: Unknown distro '$DISTRO'"
        echo "Assuming packages are already installed..."
        ;;
esac

echo ""

# Check Rust
echo "Checking Rust toolchain..."
if ! command -v rustc &> /dev/null; then
    echo "Rust not found. Installing..."
    curl --proto '=https' --tls=v1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo "Rust installed!"
else
    echo "  ✓ Rust ($(rustc --version))"
fi

# Check nightly toolchain
if ! rustup toolchain list 2>/dev/null | grep -q nightly; then
    echo "  Installing nightly toolchain..."
    rustup toolchain install nightly
else
    echo "  ✓ Nightly toolchain"
fi

# Check for Cranelift (optional)
if ! rustup component list --toolchain nightly 2>/dev/null | grep -q "rustc-codegen-cranelift.*installed"; then
    echo "  Installing Cranelift..."
    rustup component add rustc-codegen-cranelift-preview --toolchain nightly 2>/dev/null || true
else
    echo "  ✓ Cranelift backend"
fi

echo ""
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
