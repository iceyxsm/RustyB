# Linux Build for Rusty Browser

This directory contains Linux-specific build configuration and scripts for Rusty Browser.

## Quick Start

```bash
# From project root, run:
bash linux/build.sh
```

The script will:
1. Check which dependencies are missing
2. Install only the missing ones
3. Build the project

## Prerequisites

The script auto-detects your distro and installs missing packages:

| Package | Purpose |
|---------|---------|
| webkit2gtk-4.1-dev | WebView backend (WebKitGTK) |
| libgtk-3-dev | GTK3 UI framework |
| libsoup-3.0-dev | HTTP client/server |
| libssl-dev | TLS/HTTPS support |
| libcairo2-dev | 2D graphics |
| libpango1.0-dev | Text rendering |
| libglib2.0-dev | Core utilities |
| libgdk-pixbuf2.0-dev | Image loading |
| pkg-config | Package detection |
| build-essential | GCC, make, etc. |

### Supported Distros

- **Ubuntu/Debian** - uses `apt-get`
- **Fedora/RHEL/CentOS** - uses `dnf`
- **Arch/Manjaro** - uses `pacman`

### Manual Install (if needed)

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

**Arch:**
```bash
sudo pacman -S webkit2gtk-4.1 gtk3 libsoup3 glib2 cairo pango \
    gdk-pixbuf2 openssl pkgconf base-devel
```

## Build Script Options

```bash
# Standard build (checks deps, installs missing, builds)
bash linux/build.sh

# Clean build (removes previous artifacts)
bash linux/build.sh --clean
```

## What the Script Does

1. **Detects distro** - Identifies your Linux distribution
2. **Checks each package** - Uses `pkg-config` to see what's installed
3. **Shows status** - ✓ for found, ✗ for missing
4. **Installs missing** - Only installs packages that are actually needed
5. **Checks Rust** - Installs Rust if not present
6. **Builds** - Compiles both binaries

Example output:
```
Checking required packages...
  ✓ pkg-config (found)
  ✓ libwebkit2gtk-4.1-dev (found)
  ✓ libgtk-3-dev (found)
  ✗ libsoup-3.0-dev (missing)
  ✓ libssl-dev (found)
  ...

Installing missing packages: libsoup-3.0-dev
...
Packages installed successfully!
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
- **Linker**: Standard system linker
- **Codegen**: Standard LLVM (stable)

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
