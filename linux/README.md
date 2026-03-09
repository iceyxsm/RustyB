# Linux Build for Rusty Browser

This directory contains Linux-specific build configuration and scripts for Rusty Browser.

## Quick Start

```bash
# From project root, run:
bash linux/build.sh --install
```

The `--install` flag automatically installs all required system dependencies.

### Manual Dependency Install

If you prefer to install dependencies manually:

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
    libglib2.0-dev libcairo2-dev libpango1.0-dev libgdk-pixbuf2.0-dev \
    libssl-dev pkg-config build-essential

# Fedora/RHEL
sudo dnf install -y webkit2gtk4.1-devel gtk3-devel libsoup3-devel \
    glib2-devel cairo-devel pango-devel gdk-pixbuf2-devel \
    openssl-devel pkgconf

# Arch
sudo pacman -S webkit2gtk-4.1 gtk3 libsoup3 glib2 cairo pango \
    gdk-pixbuf2 openssl pkgconf base-devel
```

## Prerequisites

### Required Packages

| Package | Purpose |
|---------|---------|
| webkit2gtk-4.1-dev | WebView backend (WebKitGTK) |
| libgtk-3-dev | GTK3 UI framework |
| libsoup-3.0-dev | HTTP client/server |
| libssl-dev | TLS/HTTPS support (OpenSSL) |
| libcairo2-dev | 2D graphics library |
| libpango1.0-dev | Text rendering |
| libglib2.0-dev | Core utilities |
| libgdk-pixbuf2.0-dev | Image loading |
| pkg-config | Package detection |
| build-essential | GCC, make, etc. |

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
    libglib2.0-dev libcairo2-dev libpango1.0-dev libgdk-pixbuf2.0-dev \
    libssl-dev pkg-config build-essential
```

**Fedora/RHEL:**
```bash
sudo dnf install -y webkit2gtk4.1-devel gtk3-devel libsoup3-devel \
    glib2-devel cairo-devel pango-devel gdk-pixbuf2-devel \
    openssl-devel pkgconf gcc
```

**Arch Linux:**
```bash
sudo pacman -S webkit2gtk-4.1 gtk3 libsoup3 glib2 cairo pango \
    gdk-pixbuf2 openssl pkgconf base-devel
```

> **Note:** If webkit2gtk-4.1 is not available on your distribution, the build script will fallback to webkit2gtk-4.0.

### Rust Toolchain

The build script will automatically install:
- Nightly Rust toolchain
- Cranelift codegen backend (for faster builds with less memory)

## Build Script Options

```bash
# Auto-install dependencies and build
bash linux/build.sh --install

# Install dependencies and clean build
bash linux/build.sh --install --clean

# Just build (assumes deps already installed)
bash linux/build.sh

# Clean build only (removes previous artifacts)
bash linux/build.sh --clean
```

## Manual Build

If you prefer to build manually:

```bash
# Apply Linux config
cp linux/config.toml .cargo/config.toml

# Build both binaries
cargo +nightly build --release -p browser-ui -p rusty-browser-webview
```

## Running

```bash
# Both binaries must be in the same directory
cd target/release
./rusty-browser
```

## Configuration

The Linux build uses:
- **WebView Backend**: WebKitGTK 4.1 (via WRY)
- **Linker**: rust-lld (LLVM linker)
- **Codegen**: Cranelift (faster compilation, lower memory usage)

## Troubleshooting

### "WebView subprocess not found"
Ensure both binaries are in the same directory and have execute permissions:
```bash
chmod +x target/release/rusty-browser target/release/rusty-browser-webview
```

### Missing WebKitGTK
If you get errors about missing `webkit2gtk`, check which version is available:
```bash
pkg-config --modversion webkit2gtk-4.1 || pkg-config --modversion webkit2gtk-4.0
```

The build script will automatically use 4.1 if available, or fall back to 4.0.

### Build Memory Issues
If the build runs out of memory, edit `linux/config.toml` and reduce `jobs`:
```toml
[build]
jobs = 1  # Use single job
```
