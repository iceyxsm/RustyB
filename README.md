# Rusty Browser

A custom, feature-rich browser built entirely in Rust with advanced capabilities including network interception, web-to-API conversion, AI integration, and remote command execution.

## Features

### Core Browser Features
- **Pure Rust Implementation** - Built from scratch using Rust's memory safety and performance
- **Multi-tab/Window Support** - Full tab management with session persistence
- **Navigation** - Back/forward history, reload, stop loading
- **Private Browsing** - Incognito mode support

### Advanced Network Capabilities
- **Request/Response Interception** - Inspect and modify all HTTP/HTTPS traffic
- **Filter Rules** - Block, redirect, or modify requests based on custom rules
- **Traffic Logging** - Log all network activity for analysis
- **TLS Inspection** - MITM proxy for HTTPS traffic analysis (with certificate generation)

### Web-to-API Conversion
- **Schema-based Extraction** - Define extraction schemas for any website
- **Automatic API Generation** - Convert websites to REST APIs
- **Pagination Support** - Handle paginated content
- **Data Transformation** - Apply transformations to extracted data
- **Caching** - Cache extracted data with configurable refresh intervals

### AI Integration
- **Local LLM Support** - Run models locally using Candle
- **Page Summarization** - AI-powered content summarization
- **Smart Extraction** - Natural language data extraction
- **Chat Interface** - Chat with AI about page content
- **RAG Support** - Retrieval-Augmented Generation for browser history

### Remote Control & Automation
- **WebSocket API** - Real-time remote control
- **REST API** - HTTP-based control interface
- **Automation Scripts** - Create and run automation workflows
- **Command Batching** - Execute multiple commands in sequence
- **Screenshot Capture** - Remote screenshot capabilities

## Architecture

```
rusty-browser/
├── crates/
│   ├── browser-core/     # Core browser logic (tabs, windows, navigation)
│   ├── browser-ui/       # Iced-based user interface
│   ├── network-layer/    # Network interception and filtering
│   ├── web-to-api/       # Web scraping and API conversion
│   ├── ai-engine/        # AI/ML integration
│   ├── remote-api/       # Remote command and automation
│   └── shared/           # Shared types and utilities
├── assets/               # Icons, themes, certificates
└── config/               # Configuration files
```

## Technology Stack (2026)

### Rendering Engine
- **Servo** - Mozilla's Rust browser engine (recommended)
- Alternative: **Blitz** - Modular Rust web engine

### UI Framework
- **Iced** - Elm-inspired Rust GUI framework
- GPU-accelerated with wgpu
- Cross-platform (Windows, macOS, Linux)

### Network Stack
- **reqwest** - HTTP client
- **hyper** - HTTP server for proxy
- **rustls** - TLS implementation
- **tokio** - Async runtime

### AI/ML
- **Candle** - Hugging Face's Rust ML framework
- **tokenizers** - Fast tokenization
- **mistral.rs** - Alternative inference engine

### Storage
- **sled** / **redb** - Embedded databases
- **SQLite** - Structured data storage

## Getting Started

### Prerequisites
- Rust 1.85+ (2024 edition)
- Windows 10/11, macOS, or Linux
- GPU with Vulkan/Metal/DirectX support (optional, for AI acceleration)

### Building

```bash
# Clone the repository
git clone https://github.com/yourusername/rusty-browser.git
cd rusty-browser

# Build the project
cargo build --release

# Run the browser
cargo run --release
```

### Development

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test --workspace

# Format code
cargo fmt

# Run clippy
cargo clippy --workspace
```

## Configuration

Configuration files are stored in:
- Windows: `%APPDATA%/rusty-browser/`
- macOS: `~/Library/Application Support/rusty-browser/`
- Linux: `~/.config/rusty-browser/`

### Example Config (`config.toml`)

```toml
homepage = "https://start.duckduckgo.com"
search_engine = "https://duckduckgo.com/?q={}"
download_path = "~/Downloads"
user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"

[proxy]
enabled = true
host = "127.0.0.1"
port = 8080

[filter_rules]
block_ads = true
block_trackers = true

[ai]
model_path = "~/.rusty-browser/models/llama-7b.gguf"
context_length = 4096
```

## Usage

### Basic Browsing

Navigate to URLs using the address bar. The browser supports:
- Standard HTTP/HTTPS
- Custom protocols
- Search from address bar

### Network Interception

Filter rules can be configured via the UI or API:

```json
{
  "name": "Block Analytics",
  "condition": {
    "type": "domain_ends_with",
    "suffix": "google-analytics.com"
  },
  "action": {
    "type": "block",
    "reason": "Tracking blocked"
  }
}
```

### Web-to-API

Create extraction schemas to convert websites to APIs:

```rust
let schema = ExtractionSchema::new("Product List", "https://example.com/products")
    .with_selector(
        FieldSelector::new("name", ".product-name")
            .required()
    )
    .with_selector(
        FieldSelector::new("price", ".product-price")
            .with_transform(TransformRule::Trim)
    );
```

Access via REST API:
```bash
curl http://localhost:3000/api/extract/{schema_id}
```

### AI Features

```rust
// Summarize page
let summary = ai.summarize_page(page_content).await?;

// Ask about page
let answer = ai.ask_about_page(page_content, "What is the main topic?").await?;

// Smart extraction
let data = ai.smart_extract(html, "Extract all product names and prices").await?;
```

### Remote Control

Connect via WebSocket:
```javascript
const ws = new WebSocket('ws://localhost:9001');

ws.send(JSON.stringify({
  type: 'navigate',
  url: 'https://example.com'
}));
```

Or use REST API:
```bash
curl -X POST http://localhost:9000/api/commands \
  -H "Content-Type: application/json" \
  -d '{"type": "screenshot", "full_page": true}'
```

## Roadmap

### Phase 1: Foundation ✅
- [x] Project structure
- [x] Core browser types
- [x] Basic UI layout
- [ ] Servo integration

### Phase 2: Network Layer
- [ ] HTTP proxy implementation
- [ ] TLS interception
- [ ] Filter rule engine
- [ ] Traffic logging

### Phase 3: Web-to-API
- [ ] Schema definition UI
- [ ] Extraction engine
- [ ] REST API server
- [ ] Caching system

### Phase 4: AI Integration
- [ ] Local LLM loading
- [ ] Text generation
- [ ] Page summarization
- [ ] RAG implementation

### Phase 5: Remote Control
- [ ] WebSocket server
- [ ] Command protocol
- [ ] Automation engine
- [ ] REST API

### Phase 6: Polish
- [ ] DevTools
- [ ] Extension system
- [ ] Settings UI
- [ ] Documentation

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the MIT OR Apache-2.0 license.

## Acknowledgments

- [Servo](https://servo.org/) - The Rust browser engine
- [Iced](https://iced.rs/) - The Rust GUI framework
- [Candle](https://github.com/huggingface/candle) - Hugging Face's Rust ML framework
- The Rust community for the amazing ecosystem

## Disclaimer

This browser is a research/educational project. It is not intended for production use as a daily driver browser. Use at your own risk.

---

Built with ❤️ in Rust, 2026
