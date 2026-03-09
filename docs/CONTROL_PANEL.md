# Rusty Control Panel - Hybrid Mode

## Overview

The **Rusty Control Panel** is a unified interface that combines the browser's powerful tools with the web browsing experience. It provides a split-view layout where:

- **Left Side**: Control Panel with all browser tools
- **Right Side**: WebView for actual browsing

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│  Navigation Bar (URL, Back/Forward, Refresh)                   │
├──────────────────┬─────────────────────────────────────────────┤
│                  │                                             │
│  Sidebar         │   Tool Panel        │    WebView            │
│  ─────────────   │   ────────────     │    ─────────────       │
│  🌐 Network      │   [Tool Content]   │                       │
│  🤖 AI Engine    │                    │    [Web Page]         │
│  ⚡ Automation   │   - Settings       │                       │
│  📊 Extraction   │   - Controls       │                       │
│  🔧 DevTools     │   - Monitors       │                       │
│  ⚙️ Settings     │                    │                       │
│                   │                    │                       │
│  [◀ Toggle]      │                    │                       │
└──────────────────┴────────────────────┴───────────────────────┘
```

## Components

### 1. Control Panel (`crates/browser-ui/src/control_panel/`)

#### Core Module (`mod.rs`)
- `ToolCategory` enum: Defines all tool categories
- `ControlPanelState`: Manages panel expansion, selected category, width

#### Sidebar (`sidebar.rs`)
- Collapsible sidebar with category icons
- Shows tool names when expanded, icons only when collapsed
- Toggle button to collapse/expand
- Auto-expands when selecting a category

#### Tool Panels (`panels/`)

| Panel | File | Features |
|-------|------|----------|
| **Network** | `network_panel.rs` | MITM Proxy toggle, Ad Blocker, Privacy Mode, DNS settings |
| **AI Engine** | `ai_panel.rs` | LLM chat, RAG toggle, Model selection, Page summarization |
| **Automation** | `automation_panel.rs` | Macro recording, Command console, Saved macros, Quick actions |
| **Extraction** | `extraction_panel.rs` | Schema builder, Field mapping, Live preview, Export |
| **DevTools** | `devtools_panel.rs` | Console, Network monitor, Elements, Storage |
| **Settings** | `settings_panel.rs` | Theme, Performance, Privacy, Data management |

#### Tool Implementations (`tools/`)
- `network.rs`: Network proxy control, ad blocker management
- `ai.rs`: LLM integration, RAG queries, page analysis
- `automation.rs`: Macro recording/playback, command execution
- `extraction.rs`: Schema management, data extraction

### 2. Hybrid Application (`hybrid_app.rs`)

Main application combining the control panel with the webview:

```rust
pub struct HybridBrowserApp {
    // Control panel components
    sidebar: Sidebar,
    network_panel: NetworkPanel,
    ai_panel: AiPanel,
    // ... other panels
    
    // Browser state
    current_url: String,
    is_loading: bool,
    
    // WebView
    webview: WebViewWidget,
}
```

### 3. Updated Theme System

Added new theme helpers for the control panel:
- `surface_color()`: Background for cards/panels
- `border_color()`: Border/divider colors
- `success_color()`: Success states
- `info_color()`: Info states
- `ContainerStyle::Card`: New container style for tool cards

## Features

### Tool Categories

1. **Network** 🌐
   - MITM Proxy with HTTPS interception
   - Ad Blocker with EasyList support
   - Privacy Mode with fingerprint randomization
   - DNS-over-HTTPS/TLS configuration
   - Network logging

2. **AI Engine** 🤖
   - LLM chat interface
   - Retrieval Augmented Generation (RAG)
   - Page content summarization
   - Data extraction via natural language

3. **Automation** ⚡
   - Record and replay macros
   - Command console for scripting
   - Integration with remote API
   - Quick action buttons (Wait, Click, Type, Scroll)

4. **Extraction** 📊
   - Visual schema builder
   - CSS selector testing
   - Live preview of extracted data
   - Export to JSON/CSV/XML

5. **DevTools** 🔧
   - JavaScript console
   - Network request monitoring
   - DOM inspector
   - Storage (Cookies, LocalStorage, SessionStorage)

6. **Settings** ⚙️
   - Theme switching (Dark/Light/High Contrast)
   - Zoom controls
   - Hardware acceleration toggle
   - Data clearing and import/export

### Keyboard Shortcuts (Planned)

| Shortcut | Action |
|----------|--------|
| `Ctrl/Cmd + B` | Toggle control panel |
| `Ctrl/Cmd + 1-6` | Switch to tool category |
| `Ctrl/Cmd + Shift + R` | Start recording macro |
| `F12` | Toggle DevTools |

## Usage

### Running in Hybrid Mode

The application now defaults to hybrid mode. Simply run:

```bash
cargo run -p browser-ui
```

### Collapsing the Control Panel

- Click the `◀` button at the top of the sidebar to collapse
- Click the `▶` button to expand
- Selecting a tool category auto-expands the panel

### Using Tools

1. **Network Tools**: Enable proxy/adblocker with toggles, configure DNS settings
2. **AI Chat**: Type prompts in the input box, use quick actions for page analysis
3. **Automation**: Click "Start Recording" to capture actions, save as macros
4. **Extraction**: Build schemas with CSS selectors, preview results live
5. **DevTools**: Switch tabs to view console, network logs, or DOM
6. **Settings**: Change themes, adjust performance settings

## Integration with Existing Crates

```
browser-ui (Control Panel + Hybrid App)
    ├── network-layer (Proxy, AdBlock, DNS)
    ├── ai-engine (LLM, RAG, Embeddings)
    ├── remote-api (Commands, Automation)
    ├── web-to-api (Extraction, Schema)
    └── browser-core (Tabs, Navigation, WebView)
```

## Future Enhancements

1. **Floating/Dockable Panels**: Allow tools to be popped out into separate windows
2. **Keyboard Navigation**: Full keyboard control of the control panel
3. **Tool Workspaces**: Save and restore tool panel layouts
4. **Extensions**: Plugin system for third-party tools
5. **Command Palette**: Quick access to all tools via search

## File Structure

```
crates/browser-ui/src/
├── control_panel/
│   ├── mod.rs           # Core control panel types
│   ├── sidebar.rs       # Navigation sidebar
│   ├── panels/
│   │   ├── mod.rs
│   │   ├── network_panel.rs
│   │   ├── ai_panel.rs
│   │   ├── automation_panel.rs
│   │   ├── extraction_panel.rs
│   │   ├── devtools_panel.rs
│   │   └── settings_panel.rs
│   └── tools/
│       ├── mod.rs
│       ├── network.rs
│       ├── ai.rs
│       ├── automation.rs
│       └── extraction.rs
├── hybrid_app.rs        # Main hybrid application
└── main.rs              # Entry point (now uses HybridBrowserApp)
```

## Migration from Old UI

The old `IntegratedBrowserApp` is still available in `integrated_app.rs` for backward compatibility. To use it, change `main.rs` to:

```rust
use browser_ui::integrated_app::{IntegratedBrowserApp, Message};
```

The new default is `HybridBrowserApp` which provides the control panel experience.
