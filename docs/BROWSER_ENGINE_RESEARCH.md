# Building a Chrome-Level Browser in Rust: Research & Feasibility Study

**Objective:** Build a powerful browser like Chrome using Rust, focused on low RAM usage and high performance.

**Date:** March 2025

---

## Executive Summary

**Can you build a Chrome-level browser in Rust with low RAM?**

**Short Answer:** Not yet with Chrome-level compatibility AND low RAM. You must choose:
- **Chrome-level compatibility** → Use CEF/Chromium (300-800MB RAM)
- **Low RAM** → Use lightweight engine (50-200MB RAM) but accept broken sites
- **Hybrid approach** → Use Servo (Rust-based) - best compromise but not production-ready

---

## 1. Browser Engine Options Comparison

### 1.1 Servo (Mozilla's Rust Engine) ⭐ RECOMMENDED FOR RUST

| Metric | Value |
|--------|-------|
| **Language** | Rust |
| **RAM Usage** | 100-300MB estimated |
| **WPT Pass Rate** | 79% (1.5M+ subtests) |
| **Chrome-Level?** | ❌ Not yet - many sites break |
| **Status** | Active development (Igalia, 2024-2025) |
| **License** | MPL-2.0 |

**Pros:**
- ✅ Written in Rust (memory safety, no vulnerabilities)
- ✅ Parallel rendering (multi-core usage)
- ✅ Modular architecture
- ✅ Embeddable WebView API
- ✅ Cross-platform (Windows, macOS, Linux, Android, OpenHarmony)
- ✅ Growing community (129 contributors in 2024)

**Cons:**
- ❌ Only 79% web compatibility (vs Chrome's 95%+)
- ❌ No WebGL/WebGPU fully working
- ❌ Many modern sites break
- ❌ No extensions support
- ❌ DevTools incomplete

**2025 Roadmap:**
- CSS Grid support (via taffy)
- Accessibility support
- Incremental layout improvements
- Better embedding API

**Verdict:** Best option for Rust-native, but NOT production-ready for daily browsing.

---

### 1.2 CEF (Chromium Embedded Framework)

| Metric | Value |
|--------|-------|
| **Language** | C++ (bindings available for Rust) |
| **RAM Usage** | 300-800MB+ per tab |
| **WPT Pass Rate** | 95%+ |
| **Chrome-Level?** | ✅ YES - it's real Chrome |
| **Binary Size** | +300MB |
| **First Build Time** | 10-20 minutes |

**Pros:**
- ✅ Real Chromium engine (V8, Blink)
- ✅ Full Chrome DevTools
- ✅ 100% web compatibility
- ✅ GPU acceleration
- ✅ Extensions support

**Cons:**
- ❌ High RAM usage (same as Chrome)
- ❌ Large binary size
- ❌ Long build times
- ❌ Written in C++ (not Rust)

**Verdict:** Use if you want Chrome-level features, but RAM usage will be HIGH regardless of Rust UI.

---

### 1.3 Ultralight (Lightweight WebKit)

| Metric | Value |
|--------|-------|
| **Language** | C++ (WebKit fork) |
| **RAM Usage** | 20-50MB |
| **Binary Size** | ~10-20MB |
| **Chrome-Level?** | ❌ No - limited web features |
| **License** | Commercial/Proprietary |

**Pros:**
- ✅ Extremely lightweight
- ✅ GPU-accelerated rendering
- ✅ Good for game UI
- ✅ Cross-platform (Windows, macOS, Linux, Xbox, PS4/5)

**Cons:**
- ❌ NO WebGL, WebRTC, HTML5 Video/Audio
- ❌ Many modern sites broken
- ❌ Commercial license (expensive for commercial use)
- ❌ Not a full browser engine

**Verdict:** Good for embedded UI, NOT for full web browsing.

---

### 1.4 WebView2 (Edge) / wry (Rust wrapper)

| Metric | Value |
|--------|-------|
| **Language** | Rust + System WebView |
| **RAM Usage** | 200-500MB (Edge WebView2) |
| **WPT Pass Rate** | 90%+ |
| **Chrome-Level?** | ⚠️ Mostly - Edge is Chromium-based |
| **Binary Size** | ~5MB |

**Pros:**
- ✅ Lightweight app size
- ✅ Uses OS-native browser
- ✅ Good web compatibility
- ✅ Memory-efficient than full Chrome
- ✅ Easy Rust integration via `wry`

**Cons:**
- ❌ Platform-dependent behavior
- ❌ Limited customization
- ❌ Still uses significant RAM (Edge engine)

**Verdict:** Best practical option for hybrid Rust browser TODAY.

---

### 1.5 Ladybird Browser Engine

| Metric | Value |
|--------|-------|
| **Language** | C++ (being rewritten) |
| **RAM Usage** | Unknown |
| **Status** | Very early development |
| **License** | BSD-2-Clause |

**Pros:**
- ✅ From SerenityOS team
- ✅ Clean modern codebase

**Cons:**
- ❌ Extremely early stage
- ❌ Not written in Rust

**Verdict:** Watch this space, but not usable yet.

---

## 2. RAM Usage Reality Check

### Browser RAM Comparison (10 tabs open)

| Browser | RAM Usage | Engine | Notes |
|---------|-----------|--------|-------|
| **Microsoft Edge** | 790 MB | Blink | Most efficient major browser |
| **Opera** | 899 MB | Blink | Good for low-end PCs |
| **Brave** | 920 MB | Blink | Privacy-focused |
| **Chromium** | 930 MB | Blink | Base Chrome engine |
| **Firefox** | 960 MB | Gecko | Quantum engine |
| **Google Chrome** | 1000 MB | Blink | Feature-rich but heavy |
| **Safari** | 1200 MB | WebKit | macOS only |

### Key Insight

**The web rendering engine consumes 90%+ of RAM.** Your Rust UI will use only ~10-50MB.

```
Total Browser RAM = WebView Engine (90%) + Rust UI (10%)

Chrome:     1000 MB = 950 MB (Blink) + 50 MB (UI)
Edge:        790 MB = 740 MB (Blink) + 50 MB (UI)
Servo:       200 MB = 150 MB (Servo) + 50 MB (UI) - BUT sites break
Ultralight:   50 MB =  30 MB (WebKit) + 20 MB (UI) - BUT limited features
```

**Rust CANNOT reduce WebView RAM usage - it only affects the UI layer.**

---

## 3. Architecture Options

### Option A: Full Rust Stack (Servo)

```
┌─────────────────────────────────────┐
│  Rust UI (Iced)                     │  ← 10MB RAM
│  - Address bar                      │
│  - Tabs                             │
│  - Control panel                    │
├─────────────────────────────────────┤
│  Servo Web Engine (Rust)            │  ← 150MB RAM
│  - Layout                           │
│  - Rendering                        │
│  - JavaScript (SpiderMonkey)        │
└─────────────────────────────────────┘

Total: ~160MB RAM
Status: Experimental - sites will break
```

**Pros:** Pure Rust, memory-safe, low RAM
**Cons:** 79% web compatibility, not production-ready

---

### Option B: Hybrid - Rust UI + System WebView

```
┌─────────────────────────────────────┐
│  Rust UI (Iced)                     │  ← 10MB RAM
│  - Address bar                      │
│  - Tabs                             │
│  - Control panel                    │
├─────────────────────────────────────┤
│  wry -> WebView2/Edge               │  ← 700MB RAM
│  (Windows: Edge WebView2)           │
│  (macOS: WebKit)                    │
│  (Linux: WebKitGTK)                 │
└─────────────────────────────────────┘

Total: ~710MB RAM
Status: Production-ready TODAY
```

**Pros:** Works now, good compatibility, lightweight UI
**Cons:** Still uses 700MB+ RAM, platform-dependent

---

### Option C: Multi-Engine Strategy (RECOMMENDED)

```
┌──────────────────────────────────────────────┐
│  Rust UI (Iced) - 10MB                       │
│  - Smart engine selector                     │
│  - User can switch engines per-tab           │
├──────────────────────────────────────────────┤
│  Engine Pool:                                │
│  ┌─────────────┐  ┌─────────────┐           │
│  │ Lite Mode   │  │ Full Mode   │           │
│  │ WebKitGTK   │  │ CEF/Chromium│           │
│  │ ~150MB      │  │ ~800MB      │           │
│  │ Basic sites │  │ Heavy sites │           │
│  └─────────────┘  └─────────────┘           │
└──────────────────────────────────────────────┘

Simple site (news):  160MB (Lite)
YouTube/Google Docs: 810MB (Full)
```

**Pros:** Best of both worlds, user control
**Cons:** Complex implementation, maintenance

---

## 4. Performance Optimization Strategies

### 4.1 Tab Hibernation (Like Edge Sleeping Tabs)

```rust
// Auto-suspend inactive tabs after 5 minutes
if tab.inactive_duration > Duration::minutes(5) {
    tab.hibernate(); // Free RAM, keep state
}
```

**Potential RAM savings:** 40-60%

### 4.2 Process Isolation per Tab

Chrome uses separate processes per tab for stability.
- **Pros:** Crash isolation, security
- **Cons:** Higher RAM usage

**Alternative:** Group tabs by domain, single process per domain
- **Pros:** Lower RAM (shared resources)
- **Cons:** Less isolation

### 4.3 GPU-Accelerated Rendering

Use GPU for:
- CSS animations
- Canvas/WebGL
- Scrolling

**RAM savings:** Offloads from main memory to GPU VRAM

### 4.4 Aggressive Caching with Compression

```rust
// Compress cached resources
let compressed = zstd::encode(&resource, 3)?;
// Decompress on demand
```

**Trade-off:** CPU usage vs RAM usage

---

## 5. Recommendation for Your Project

### Phase 1: MVP (Now)

Use **wry (WebView2)** with Iced:
- ✅ Works immediately
- ✅ Good compatibility
- ✅ Rust-native UI
- ⚠️ 700MB+ RAM usage

### Phase 2: Optimization (3-6 months)

Implement:
1. Tab hibernation
2. Ad/tracker blocking (reduces RAM by blocking heavy scripts)
3. Process pooling (limit max processes)
4. Memory pressure handling

**Target:** 500-600MB for 10 tabs (vs Edge's 790MB)

### Phase 3: Experimental (6-12 months)

Add **Servo** as "Lite Mode":
- For simple sites (news, blogs)
- Fallback to WebView2 for complex sites
- Users can toggle per-tab

**Target:** 200MB for lite tabs, 800MB for heavy tabs

### Phase 4: Future (12+ months)

If Servo matures:
- Make Servo primary engine
- Full Rust stack
- True low-RAM Chrome alternative

---

## 6. Current Servo Status (2025)

### What's Working:
- ✅ Basic HTML/CSS rendering
- ✅ JavaScript (SpiderMonkey)
- ✅ Flexbox layout
- ✅ Floats, tables
- ✅ SVG (via resvg)
- ✅ Fonts
- ✅ WebDriver automation

### What's Missing:
- ❌ CSS Grid (in progress via taffy)
- ❌ WebGL/WebGPU (in progress)
- ❌ Extensions
- ❌ Accessibility
- ❌ Form interactivity
- ❌ Video/Audio

### Community Growth:
- 1,771 PRs merged in 2024 (+163% from 2023)
- 129 unique contributors (+143%)
- 25K+ GitHub stars

---

## 7. Conclusion

### The Hard Truth

**You CANNOT build a Chrome-level browser with low RAM today.**

The rendering engine (Blink/WebKit) is what consumes RAM. Rust makes your UI fast and safe, but doesn't reduce the WebView's memory footprint.

### The Practical Path

1. **Build with wry/WebView2 now** - works, good compatibility
2. **Aggressive optimizations** - hibernation, blocking, pooling
3. **Monitor Servo** - switch when it reaches 90%+ compatibility
4. **Hybrid approach** - let users choose engine per-tab

### Final Verdict

| Goal | Solution | Timeline |
|------|----------|----------|
| Working browser NOW | wry + WebView2 | Immediate |
| Lower RAM | Optimizations + hibernation | 3-6 months |
| Chrome-level + Low RAM | Servo (if it matures) | 2-3 years |
| True innovation | Contribute to Servo | Ongoing |

---

## References

1. [Servo Project 2025 Update](https://blogs.igalia.com/mrego/servo-a-new-web-engine-written-in-rust/)
2. [Servo Official Website](https://servo.org/)
3. [Browser RAM Usage Comparison 2025](https://monovm.com/blog/which-browser-uses-the-least-ram/)
4. [Ultralight Documentation](https://ultralig.ht/)
5. [Blitz Layout Engine](https://blitz.is/about)
6. [Web Platform Tests](https://servo.org/wpt)

---

*Document created for Rusty Browser project - March 2025*
