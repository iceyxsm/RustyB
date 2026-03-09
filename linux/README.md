# Linux Build for Rusty Browser

This directory contains Linux-specific build configuration and scripts for Rusty Browser.

## Quick Start

```bash
# From project root, run:
bash linux/build.sh
```

## Prerequisites

### Required Packages

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev
```

**Fedora/RHEL:**
```bash
sudo dnf install webkit2gtk4.1-devel gtk3-devel libsoup3-devel
```

**Arch Linux:**
```bash
sudo pacman -S webkit2gtk-4.1 gtk3 libsoup3
```

### Rust Toolchain

The build script will automatically install:
- Nightly Rust toolchain
- Cranelift codegen backend (for faster builds with less memory)

## Build Script Options

```bash
# Standard build
bash linux/build.sh

# Clean build (removes previous build artifacts)
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
If you get errors about missing `webkit2gtk`, make sure you have version **4.1** (not 4.0):
```bash
pkg-config --modversion webkit2gtk-4.1
```

### Build Memory Issues
If the build runs out of memory, edit `linux/config.toml` and reduce `jobs`:
```toml
[build]
jobs = 1  # Use single job
```
