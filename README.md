# Rusty Browser

A **hybrid browser** built in Rust combining a native Iced UI with system WebView rendering via IPC. Features network interception, web-to-API conversion, AI integration, and remote control capabilities.

> **Architecture Note:** This is a **hybrid browser** - not Electron! We use Iced (Rust) for the UI and native OS WebView (Edge WebView2 on Windows, WebKit on macOS) for rendering via process-separated IPC.

## Architecture Overview

```
┌────────────────────────────────────────────────────────────────┐
│                     Rusty Browser                              │
├────────────────────────────────────────────────────────────────┤
│  Main Process (Iced UI)                                        │
│  ┌────────────────┐  ┌────────────────┐  ┌─────────────────┐   │
│  │   Toolbar      │  │  Address Bar   │  │   Status Bar    │   │
│  │   (Rust)       │  │   (Rust)       │  │   (Rust)        │   │
│  └────────────────┘  └────────────────┘  └─────────────────┘   │
│                                                                │
│  IPC Controller (JSON-RPC over stdin/stdout)                   │
│  ├─ Commands: Navigate, Reload, Back/Forward, Execute JS       │
│  └─ Events: Load, Title, URL, Navigation, Errors               │
└────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│              rusty-browser-webview.exe                          │
│              (Separate Process - Tao + Wry)                     │
├─────────────────────────────────────────────────────────────────┤
│  Native Window + System WebView                                 │
│  ├─ Windows: Edge WebView2 (system)                             │
│  ├─ macOS:   WebKit WKWebView (system)                          │
│  └─ Linux:   WebKitGTK (system)                                 │
│                                                                 │
│  Event Handlers → JSON stdout                                   │
│  stdin ← Command Parser → WebView API                           │
└─────────────────────────────────────────────────────────────────┘
```

### Why This Architecture?

| Approach | Binary Size | Memory | Pros | Cons |
|----------|-------------|--------|------|------|
| **Rusty (Hybrid)** | ~93MB | ~200MB | Native UI, system WebView, fast | Two windows |
| Electron | 200MB+ | 500MB+ | Single window | Bundles Chromium, bloat |
| Pure Servo | N/A | N/A | 100% Rust | Incomplete, crashes |
| CEF | 300MB+ | 400MB+ | Full control | Massive size, slow build |

## Features

###  Implemented

#### Core Browser
- **Hybrid UI/Rendering** - Iced toolbar + WebView content window
- **Multi-process Architecture** - UI and WebView run separately for stability
- **Navigation** - Address bar, back/forward (via IPC), reload
- **Tab State** - URL tracking, loading indicators, title updates

#### IPC System
- **JSON-RPC Protocol** - Type-safe communication over pipes
- **Bidirectional** - Commands to WebView, events from WebView
- **Graceful Degradation** - UI works even without WebView subprocess
- **Process Isolation** - WebView crashes don't kill the browser

#### Network Layer
- **TLS Certificate Authority** - Custom CA for traffic inspection
- **MITM Proxy** - HTTPS interception with rustls
- **DNS Resolution** - Hickory resolver with caching
- **Request/Response Logging** - Full traffic inspection

#### Web-to-API
- **Schema-based Extraction** - Define extraction schemas
- **REST API Generation** - Convert websites to APIs
- **HTML Parsing** - scraper-based content extraction

#### AI Integration
- **Local LLM Support** - Candle-based inference
- **Page Summarization** - AI-powered content analysis
- **Text Generation** - Local model execution

#### Remote Control
- **WebSocket API** - Real-time remote control
- **REST API** - HTTP-based commands
- **Command Protocol** - Structured automation

###  In Progress / Planned

- [ ] Embedded WebView (single window via win32 child windows)
- [ ] Extension system
- [ ] DevTools integration
- [ ] Mobile support

## Technology Stack

### UI (Main Process)
| Component | Technology | Version |
|-----------|------------|---------|
| Framework | Iced | 0.14 |
| Rendering | wgpu | 23 |
| Windows | Tao | 0.32 |
| Icons | iced_aw | 0.13 |

### WebView (Subprocess)
| Component | Technology | Version |
|-----------|------------|---------|
| Window | Tao | 0.32 |
| WebView | WRY | 0.50 |
| Backend | Edge WebView2 | System |

### Network & AI
| Component | Technology | Version |
|-----------|------------|---------|
| HTTP | reqwest/hyper | 0.12/1.5 |
| TLS | rustls | 0.23 |
| DNS | hickory-resolver | 0.25 |
| AI | Candle | 0.9 |
| Async | Tokio | 1.48 |

### Storage
| Component | Technology |
|-----------|------------|
| KV Store | sled / redb |
| Cache | In-memory + disk |

## Project Structure

```
rusty-browser/
├── crates/
│   ├── browser-core/        # Core browser logic, tabs, navigation
│   ├── browser-ui/          # Iced UI + IPC controller
│   │   ├── src/
│   │   │   ├── main.rs              # Entry point
│   │   │   ├── integrated_app.rs    # Main Iced app
│   │   │   ├── webview_ipc.rs       # IPC implementation
│   │   │   ├── webview_widget.rs    # WebView widget
│   │   │   └── ...
│   │   └── build.rs         # Build script (finds subprocess)
│   ├── webview-subprocess/  # WebView binary (Tao + WRY)
│   │   └── src/
│   │       └── main.rs      # Subprocess entry
│   ├── network-layer/       # Proxy, TLS, DNS
│   ├── web-to-api/          # Scraping, API generation
│   ├── ai-engine/           # Local LLM inference
│   ├── remote-api/          # WebSocket/REST APIs
│   └── shared/              # Common types, errors
├── Cargo.toml               # Workspace definition
└── README.md                # This file
```

## Getting Started

### Prerequisites

- **Rust** 1.85+ (2024 edition)
- **Windows 10/11**, macOS, or Linux
- **Edge WebView2 Runtime** (Windows - usually pre-installed)

### Building

```bash
# Clone repository
git clone https://github.com/yourusername/rusty-browser.git
cd rusty-browser

# Build both binaries (main + subprocess)
cargo +nightly build --release -p browser-ui -p rusty-browser-webview

# Or build everything
cargo +nightly build --release
```

### Running

```bash
# Both binaries must be in same directory
./target/release/rusty-browser.exe
```

This starts:
1. **Main Window** (Iced) - Toolbar, address bar, controls
2. **WebView Window** - Content rendering via Edge WebView2

### Development

```bash
# Run with debug logging
RUST_LOG=debug cargo +nightly run --release -p browser-ui

# Run tests
cargo test --workspace

# Format code
cargo fmt

# Check lints
cargo clippy --workspace
```

## IPC Protocol

### Commands (UI → WebView)

```json
{"method":"Navigate","params":{"url":"https://example.com"}}
{"method":"Reload"}
{"method":"GoBack"}
{"method":"GoForward"}
{"method":"ExecuteScript","params":{"script":"alert('hi')"}}
{"method":"SetBounds","params":{"x":0,"y":80,"width":1024,"height":688}}
{"method":"Show"}
{"method":"Hide"}
{"method":"Close"}
```

### Events (WebView → UI)

```json
{"event":"LoadStarted","url":"https://example.com"}
{"event":"LoadFinished","url":"https://example.com","success":true}
{"event":"UrlChanged","url":"https://example.com/page2"}
{"event":"TitleChanged","title":"Example Domain"}
{"event":"NavigationRequested","url":"https://example.com/link"}
{"event":"PageError","error":"Failed to load"}
{"event":"WindowClosed"}
```

## Configuration

Configuration directory:
- **Windows:** `%APPDATA%/rusty-browser/`
- **macOS:** `~/Library/Application Support/rusty-browser/`
- **Linux:** `~/.config/rusty-browser/`

### Example `config.toml`

```toml
homepage = "https://start.duckduckgo.com"
search_engine = "https://duckduckgo.com/?q={}"
download_path = "~/Downloads"

[proxy]
enabled = true
host = "127.0.0.1"
port = 8080
ca_cert_path = "~/.rusty-browser/certs/ca.pem"

[ai]
enabled = true
model_path = "~/.rusty-browser/models/llama-7b.gguf"
context_length = 4096

[webview]
initial_width = 1024
initial_height = 768
devtools = false
```

## Usage Examples

### Network Interception

```rust
use network_layer::proxy::{ProxyServer, ProxyConfig};

let config = ProxyConfig::default()
    .with_port(8080)
    .with_tls_interception(true);

let proxy = ProxyServer::new(config).await?;
proxy.run().await?;
```

### Web-to-API

```rust
use web_to_api::{ExtractionSchema, FieldSelector};

let schema = ExtractionSchema::new("Products", "https://example.com/products")
    .with_selector(FieldSelector::new("name", ".product-name").required())
    .with_selector(FieldSelector::new("price", ".product-price"));
```

### Remote Control (WebSocket)

```javascript
const ws = new WebSocket('ws://localhost:9001');

ws.send(JSON.stringify({
    type: 'navigate',
    url: 'https://example.com'
}));
```

### AI Summarization

```rust
use ai_engine::CandleEngine;

let ai = CandleEngine::new("model.gguf")?;
let summary = ai.summarize(page_content).await?;
```

## Performance

### Binary Sizes
| Component | Debug | Release |
|-----------|-------|---------|
| rusty-browser.exe | ~400MB | **84MB** |
| rusty-browser-webview.exe | ~50MB | **9MB** |
| **Total** | ~450MB | **93MB** |

### Memory Usage
| Component | Typical |
|-----------|---------|
| Main Process (Iced) | ~170MB |
| WebView Process | ~25MB |
| **Total** | **~195MB** |

### Build Times
- Fresh build: ~10-15 minutes
- Incremental: ~10-30 seconds

## Troubleshooting

### "WebView subprocess not found"
Ensure both binaries are in the same directory:
```bash
cargo build --release -p browser-ui -p rusty-browser-webview
```

### "Failed to create WebView"
Install Edge WebView2 Runtime:
- Windows 11: Pre-installed
- Windows 10: Download from Microsoft

### High memory usage
- Use release profile: `--release`
- Enable Cranelift: `codegen-backend = "cranelift"`
- Reduce codegen units in `.cargo/config.toml`

## Roadmap

### Phase 1: Foundation 
- [x] Hybrid Iced + WebView architecture
- [x] IPC system (JSON-RPC)
- [x] Multi-process separation
- [x] Network layer (proxy, TLS, DNS)

### Phase 2: Core Features 
- [x] Navigation (address bar, back/forward)
- [x] Tab management
- [x] URL/title synchronization
- [x] Error handling

### Phase 3: Advanced Network
- [x] TLS interception with custom CA
- [x] Request/response logging
- [ ] Filter rule engine UI
- [ ] Traffic analyzer

### Phase 4: AI Integration 🚧
- [x] Candle integration
- [x] Local LLM loading
- [ ] Page summarization UI
- [ ] Chat interface

### Phase 5: Web-to-API
- [x] Extraction schemas
- [x] HTML parsing
- [ ] REST API server
- [ ] Auto-generated documentation

### Phase 6: Polish
- [ ] Single-window mode (embedded WebView)
- [ ] Extension system
- [ ] DevTools integration
- [ ] Settings UI
- [ ] Themes

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md).

### Development Setup

```bash
# Install nightly Rust
rustup default nightly

# Clone and build
git clone https://github.com/yourusername/rusty-browser.git
cd rusty-browser
cargo build --release

# Run tests
cargo test --workspace
```

## License

MIT OR Apache-2.0

## Acknowledgments

- [Iced](https://iced.rs/) - Native Rust GUI framework
- [WRY](https://github.com/tauri-apps/wry) - WebView library for Rust
- [Tao](https://github.com/tauri-apps/tao) - Cross-platform windowing
- [Candle](https://github.com/huggingface/candle) - Rust ML framework
- [rustls](https://github.com/rustls/rustls) - Modern TLS library
- [hickory-dns](https://github.com/hickory-dns/hickory-dns) - Rust DNS resolver

## Disclaimer

This is a research/educational project. Not intended for production use as a daily driver. Use at your own risk.

---
