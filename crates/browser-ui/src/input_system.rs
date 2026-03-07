//! Production-grade Input Handling System for Rusty Browser
//!
//! This module provides comprehensive input event handling with:
//! - Event collection from winit 0.30+ and Iced 0.14
//! - Input event batching and coalescing (mouse move, scroll)
//! - Real-time input state tracking (pressed keys, mouse position)
//! - Gesture recognition (click, double-click, long-press, pinch-zoom, swipe)
//! - Focus management with tab order navigation
//! - Platform-specific key code mapping
//! - Thread-safe event processing
//! - Accessibility support
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
//! │   winit/Iced    │────▶│  InputManager   │────▶│  InputBatcher   │
//! │   Events        │     │                 │     │                 │
//! └─────────────────┘     └─────────────────┘     └─────────────────┘
//!                                │                         │
//!                                ▼                         ▼
//!                       ┌─────────────────┐     ┌─────────────────┐
//!                       │  GestureRecognizer│    │  Batched Events │
//!                       │                 │     │  (per frame)    │
//!                       └─────────────────┘     └─────────────────┘
//!                                │
//!                                ▼
//!                       ┌─────────────────┐
//!                       │  FocusManager   │
//!                       │  (tab order)    │
//!                       └─────────────────┘
//! ```

use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, trace, warn};

// Re-export types from gpu_renderer for compatibility
pub use crate::gpu_renderer::{InputBatcher, InputEvent as GpuInputEvent};

/// Maximum number of events to process per frame
const MAX_EVENTS_PER_FRAME: usize = 256;

/// Maximum number of touch points for multi-touch
const MAX_TOUCH_POINTS: usize = 10;

/// Double-click time threshold in milliseconds
const DOUBLE_CLICK_THRESHOLD_MS: u128 = 500;

/// Long-press time threshold in milliseconds
const LONG_PRESS_THRESHOLD_MS: u128 = 800;

/// Click movement threshold (pixels) - max movement to still count as a click
const CLICK_MOVEMENT_THRESHOLD: f64 = 5.0;

/// Swipe velocity threshold for gesture recognition
const SWIPE_VELOCITY_THRESHOLD: f64 = 100.0;

/// Pinch zoom threshold
const PINCH_ZOOM_THRESHOLD: f64 = 10.0;

/// ====================================================================================
/// INPUT EVENT TYPES
/// ====================================================================================

/// Mouse button types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

impl MouseButton {
    /// Convert from winit mouse button
    #[cfg(feature = "winit")]
    pub fn from_winit(button: winit::event::MouseButton) -> Self {
        match button {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Back => MouseButton::Back,
            winit::event::MouseButton::Forward => MouseButton::Forward,
            winit::event::MouseButton::Other(n) => MouseButton::Other(n),
        }
    }

    /// Convert from Iced mouse button
    pub fn from_iced(button: iced::mouse::Button) -> Self {
        match button {
            iced::mouse::Button::Left => MouseButton::Left,
            iced::mouse::Button::Right => MouseButton::Right,
            iced::mouse::Button::Middle => MouseButton::Middle,
            iced::mouse::Button::Back => MouseButton::Back,
            iced::mouse::Button::Forward => MouseButton::Forward,
            iced::mouse::Button::Other(n) => MouseButton::Other(n),
        }
    }

    /// Convert to browser-core MouseButton
    pub fn to_core(&self) -> browser_core::webview::MouseButton {
        match self {
            MouseButton::Left => browser_core::webview::MouseButton::Left,
            MouseButton::Right => browser_core::webview::MouseButton::Right,
            MouseButton::Middle => browser_core::webview::MouseButton::Middle,
            _ => browser_core::webview::MouseButton::Left,
        }
    }
}

/// Scroll delta types
#[derive(Debug, Clone, Copy)]
pub enum ScrollDelta {
    /// Scroll by pixels
    Pixels { x: f64, y: f64 },
    /// Scroll by lines
    Lines { x: f64, y: f64 },
}

impl ScrollDelta {
    /// Convert from winit scroll delta
    #[cfg(feature = "winit")]
    pub fn from_winit(delta: winit::event::MouseScrollDelta) -> Self {
        match delta {
            winit::event::MouseScrollDelta::LineDelta(x, y) => {
                ScrollDelta::Lines { x: x as f64, y: y as f64 }
            }
            winit::event::MouseScrollDelta::PixelDelta(pos) => {
                ScrollDelta::Pixels { x: pos.x, y: pos.y }
            }
        }
    }

    /// Convert to browser-core WheelDelta
    pub fn to_core(&self) -> browser_core::webview::WheelDelta {
        match self {
            ScrollDelta::Pixels { x, y } => browser_core::webview::WheelDelta {
                x: *x as f32,
                y: *y as f32,
            },
            ScrollDelta::Lines { x, y } => browser_core::webview::WheelDelta {
                x: *x as f32 * 20.0, // Approximate line height
                y: *y as f32 * 20.0,
            },
        }
    }
}

/// Key representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    /// Named key (Enter, Escape, etc.)
    Named(NamedKey),
    /// Character key
    Character(String),
    /// Unidentified key
    Unidentified,
}

/// Named keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamedKey {
    Alt,
    AltGraph,
    CapsLock,
    Control,
    Fn,
    FnLock,
    Meta,
    NumLock,
    ScrollLock,
    Shift,
    Symbol,
    SymbolLock,
    Hyper,
    Super,
    Enter,
    Tab,
    Space,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    End,
    Home,
    PageDown,
    PageUp,
    Backspace,
    Clear,
    Copy,
    CrSel,
    Cut,
    Delete,
    EraseEof,
    ExSel,
    Insert,
    Paste,
    Redo,
    Undo,
    Accept,
    Again,
    Attn,
    Cancel,
    ContextMenu,
    Escape,
    Execute,
    Find,
    Help,
    Pause,
    Play,
    Props,
    Select,
    ZoomIn,
    ZoomOut,
    BrightnessDown,
    BrightnessUp,
    Eject,
    LogOff,
    Power,
    PowerOff,
    PrintScreen,
    Hibernate,
    Standby,
    WakeUp,
    AllCandidates,
    Alphanumeric,
    CodeInput,
    Compose,
    Convert,
    Dead,
    FinalMode,
    GroupFirst,
    GroupLast,
    GroupNext,
    GroupPrevious,
    ModeChange,
    NextCandidate,
    NonConvert,
    PreviousCandidate,
    Process,
    SingleCandidate,
    HangulMode,
    HanjaMode,
    JunjaMode,
    Eisu,
    Hankaku,
    Hiragana,
    HiraganaKatakana,
    KanaMode,
    KanjiMode,
    Katakana,
    Romaji,
    Zenkaku,
    ZenkakuHankaku,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F25,
    F26,
    F27,
    F28,
    F29,
    F30,
    F31,
    F32,
    F33,
    F34,
    F35,
}

impl Key {
    /// Convert from winit key
    #[cfg(feature = "winit")]
    pub fn from_winit(key: &winit::keyboard::Key) -> Self {
        match key {
            winit::keyboard::Key::Named(named) => Key::Named(NamedKey::from_winit(*named)),
            winit::keyboard::Key::Character(c) => Key::Character(c.to_string()),
            winit::keyboard::Key::Unidentified => Key::Unidentified,
            _ => Key::Unidentified,
        }
    }

    /// Convert from Iced key
    pub fn from_iced(key: &iced::keyboard::Key) -> Self {
        match key {
            iced::keyboard::Key::Named(named) => Key::Named(NamedKey::from_iced(*named)),
            iced::keyboard::Key::Character(c) => Key::Character(c.to_string()),
            iced::keyboard::Key::Unidentified => Key::Unidentified,
        }
    }

    /// Get the string representation for browser-core
    pub fn to_core_string(&self) -> String {
        match self {
            Key::Named(named) => format!("{:?}", named),
            Key::Character(c) => c.clone(),
            Key::Unidentified => "Unidentified".to_string(),
        }
    }
}

impl NamedKey {
    /// Convert from winit named key
    #[cfg(feature = "winit")]
    pub fn from_winit(key: winit::keyboard::NamedKey) -> Self {
        use winit::keyboard::NamedKey as WinitKey;
        match key {
            WinitKey::Alt => NamedKey::Alt,
            WinitKey::AltGraph => NamedKey::AltGraph,
            WinitKey::CapsLock => NamedKey::CapsLock,
            WinitKey::Control => NamedKey::Control,
            WinitKey::Fn => NamedKey::Fn,
            WinitKey::FnLock => NamedKey::FnLock,
            WinitKey::Meta => NamedKey::Meta,
            WinitKey::NumLock => NamedKey::NumLock,
            WinitKey::ScrollLock => NamedKey::ScrollLock,
            WinitKey::Shift => NamedKey::Shift,
            WinitKey::Symbol => NamedKey::Symbol,
            WinitKey::SymbolLock => NamedKey::SymbolLock,
            WinitKey::Hyper => NamedKey::Hyper,
            WinitKey::Super => NamedKey::Super,
            WinitKey::Enter => NamedKey::Enter,
            WinitKey::Tab => NamedKey::Tab,
            WinitKey::Space => NamedKey::Space,
            WinitKey::ArrowDown => NamedKey::ArrowDown,
            WinitKey::ArrowLeft => NamedKey::ArrowLeft,
            WinitKey::ArrowRight => NamedKey::ArrowRight,
            WinitKey::ArrowUp => NamedKey::ArrowUp,
            WinitKey::End => NamedKey::End,
            WinitKey::Home => NamedKey::Home,
            WinitKey::PageDown => NamedKey::PageDown,
            WinitKey::PageUp => NamedKey::PageUp,
            WinitKey::Backspace => NamedKey::Backspace,
            WinitKey::Clear => NamedKey::Clear,
            WinitKey::Copy => NamedKey::Copy,
            WinitKey::CrSel => NamedKey::CrSel,
            WinitKey::Cut => NamedKey::Cut,
            WinitKey::Delete => NamedKey::Delete,
            WinitKey::EraseEof => NamedKey::EraseEof,
            WinitKey::ExSel => NamedKey::ExSel,
            WinitKey::Insert => NamedKey::Insert,
            WinitKey::Paste => NamedKey::Paste,
            WinitKey::Redo => NamedKey::Redo,
            WinitKey::Undo => NamedKey::Undo,
            WinitKey::Accept => NamedKey::Accept,
            WinitKey::Again => NamedKey::Again,
            WinitKey::Attn => NamedKey::Attn,
            WinitKey::Cancel => NamedKey::Cancel,
            WinitKey::ContextMenu => NamedKey::ContextMenu,
            WinitKey::Escape => NamedKey::Escape,
            WinitKey::Execute => NamedKey::Execute,
            WinitKey::Find => NamedKey::Find,
            WinitKey::Help => NamedKey::Help,
            WinitKey::Pause => NamedKey::Pause,
            WinitKey::Play => NamedKey::Play,
            WinitKey::Props => NamedKey::Props,
            WinitKey::Select => NamedKey::Select,
            WinitKey::ZoomIn => NamedKey::ZoomIn,
            WinitKey::ZoomOut => NamedKey::ZoomOut,
            WinitKey::BrightnessDown => NamedKey::BrightnessDown,
            WinitKey::BrightnessUp => NamedKey::BrightnessUp,
            WinitKey::Eject => NamedKey::Eject,
            WinitKey::LogOff => NamedKey::LogOff,
            WinitKey::Power => NamedKey::Power,
            WinitKey::PowerOff => NamedKey::PowerOff,
            WinitKey::PrintScreen => NamedKey::PrintScreen,
            WinitKey::Hibernate => NamedKey::Hibernate,
            WinitKey::Standby => NamedKey::Standby,
            WinitKey::WakeUp => NamedKey::WakeUp,
            WinitKey::AllCandidates => NamedKey::AllCandidates,
            WinitKey::Alphanumeric => NamedKey::Alphanumeric,
            WinitKey::CodeInput => NamedKey::CodeInput,
            WinitKey::Compose => NamedKey::Compose,
            WinitKey::Convert => NamedKey::Convert,
            WinitKey::Dead => NamedKey::Dead,
            WinitKey::FinalMode => NamedKey::FinalMode,
            WinitKey::GroupFirst => NamedKey::GroupFirst,
            WinitKey::GroupLast => NamedKey::GroupLast,
            WinitKey::GroupNext => NamedKey::GroupNext,
            WinitKey::GroupPrevious => NamedKey::GroupPrevious,
            WinitKey::ModeChange => NamedKey::ModeChange,
            WinitKey::NextCandidate => NamedKey::NextCandidate,
            WinitKey::NonConvert => NamedKey::NonConvert,
            WinitKey::PreviousCandidate => NamedKey::PreviousCandidate,
            WinitKey::Process => NamedKey::Process,
            WinitKey::SingleCandidate => NamedKey::SingleCandidate,
            WinitKey::HangulMode => NamedKey::HangulMode,
            WinitKey::HanjaMode => NamedKey::HanjaMode,
            WinitKey::JunjaMode => NamedKey::JunjaMode,
            WinitKey::Eisu => NamedKey::Eisu,
            WinitKey::Hankaku => NamedKey::Hankaku,
            WinitKey::Hiragana => NamedKey::Hiragana,
            WinitKey::HiraganaKatakana => NamedKey::HiraganaKatakana,
            WinitKey::KanaMode => NamedKey::KanaMode,
            WinitKey::KanjiMode => NamedKey::KanjiMode,
            WinitKey::Katakana => NamedKey::Katakana,
            WinitKey::Romaji => NamedKey::Romaji,
            WinitKey::Zenkaku => NamedKey::Zenkaku,
            WinitKey::ZenkakuHankaku => NamedKey::ZenkakuHankaku,
            WinitKey::F1 => NamedKey::F1,
            WinitKey::F2 => NamedKey::F2,
            WinitKey::F3 => NamedKey::F3,
            WinitKey::F4 => NamedKey::F4,
            WinitKey::F5 => NamedKey::F5,
            WinitKey::F6 => NamedKey::F6,
            WinitKey::F7 => NamedKey::F7,
            WinitKey::F8 => NamedKey::F8,
            WinitKey::F9 => NamedKey::F9,
            WinitKey::F10 => NamedKey::F10,
            WinitKey::F11 => NamedKey::F11,
            WinitKey::F12 => NamedKey::F12,
            _ => NamedKey::Unidentified,
        }
    }

    /// Convert from Iced named key
    pub fn from_iced(key: iced::keyboard::key::Named) -> Self {
        use iced::keyboard::key::Named as IcedKey;
        match key {
            IcedKey::Alt => NamedKey::Alt,
            IcedKey::CapsLock => NamedKey::CapsLock,
            IcedKey::Control => NamedKey::Control,
            IcedKey::Fn => NamedKey::Fn,
            IcedKey::FnLock => NamedKey::FnLock,
            IcedKey::Meta => NamedKey::Meta,
            IcedKey::NumLock => NamedKey::NumLock,
            IcedKey::ScrollLock => NamedKey::ScrollLock,
            IcedKey::Shift => NamedKey::Shift,
            IcedKey::Enter => NamedKey::Enter,
            IcedKey::Tab => NamedKey::Tab,
            IcedKey::Space => NamedKey::Space,
            IcedKey::ArrowDown => NamedKey::ArrowDown,
            IcedKey::ArrowLeft => NamedKey::ArrowLeft,
            IcedKey::ArrowRight => NamedKey::ArrowRight,
            IcedKey::ArrowUp => NamedKey::ArrowUp,
            IcedKey::End => NamedKey::End,
            IcedKey::Home => NamedKey::Home,
            IcedKey::PageDown => NamedKey::PageDown,
            IcedKey::PageUp => NamedKey::PageUp,
            IcedKey::Backspace => NamedKey::Backspace,
            IcedKey::Delete => NamedKey::Delete,
            IcedKey::Escape => NamedKey::Escape,
            IcedKey::Insert => NamedKey::Insert,
            IcedKey::F1 => NamedKey::F1,
            IcedKey::F2 => NamedKey::F2,
            IcedKey::F3 => NamedKey::F3,
            IcedKey::F4 => NamedKey::F4,
            IcedKey::F5 => NamedKey::F5,
            IcedKey::F6 => NamedKey::F6,
            IcedKey::F7 => NamedKey::F7,
            IcedKey::F8 => NamedKey::F8,
            IcedKey::F9 => NamedKey::F9,
            IcedKey::F10 => NamedKey::F10,
            IcedKey::F11 => NamedKey::F11,
            IcedKey::F12 => NamedKey::F12,
            IcedKey::F13 => NamedKey::F13,
            IcedKey::F14 => NamedKey::F14,
            IcedKey::F15 => NamedKey::F15,
            IcedKey::F16 => NamedKey::F16,
            IcedKey::F17 => NamedKey::F17,
            IcedKey::F18 => NamedKey::F18,
            IcedKey::F19 => NamedKey::F19,
            IcedKey::F20 => NamedKey::F20,
            IcedKey::F21 => NamedKey::F21,
            IcedKey::F22 => NamedKey::F22,
            IcedKey::F23 => NamedKey::F23,
            IcedKey::F24 => NamedKey::F24,
            IcedKey::F25 => NamedKey::F25,
            IcedKey::F26 => NamedKey::F26,
            IcedKey::F27 => NamedKey::F27,
            IcedKey::F28 => NamedKey::F28,
            IcedKey::F29 => NamedKey::F29,
            IcedKey::F30 => NamedKey::F30,
            IcedKey::F31 => NamedKey::F31,
            IcedKey::F32 => NamedKey::F32,
            IcedKey::F33 => NamedKey::F33,
            IcedKey::F34 => NamedKey::F34,
            IcedKey::F35 => NamedKey::F35,
            _ => NamedKey::Unidentified,
        }
    }
}

/// Modifier keys state
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Modifiers {
    /// Create new modifiers
    pub fn new(shift: bool, ctrl: bool, alt: bool, meta: bool) -> Self {
        Self {
            shift,
            ctrl,
            alt,
            meta,
        }
    }

    /// Convert from winit modifiers
    #[cfg(feature = "winit")]
    pub fn from_winit(modifiers: winit::keyboard::ModifiersState) -> Self {
        Self {
            shift: modifiers.shift_key(),
            ctrl: modifiers.control_key(),
            alt: modifiers.alt_key(),
            meta: modifiers.super_key(),
        }
    }

    /// Convert from Iced modifiers
    pub fn from_iced(modifiers: iced::keyboard::Modifiers) -> Self {
        Self {
            shift: modifiers.shift(),
            ctrl: modifiers.control(),
            alt: modifiers.alt(),
            meta: modifiers.logo(),
        }
    }

    /// Convert to browser-core Modifiers
    pub fn to_core(&self) -> browser_core::webview::Modifiers {
        browser_core::webview::Modifiers {
            shift: self.shift,
            ctrl: self.ctrl,
            alt: self.alt,
            meta: self.meta,
        }
    }

    /// Check if any modifier is pressed
    pub fn any(&self) -> bool {
        self.shift || self.ctrl || self.alt || self.meta
    }

    /// Check if no modifiers are pressed
    pub fn is_empty(&self) -> bool {
        !self.any()
    }
}

/// Main input event enum - unified for all input types
#[derive(Debug, Clone)]
pub enum InputEvent {
    // Mouse events
    MouseMove {
        x: f64,
        y: f64,
        delta_x: f64,
        delta_y: f64,
    },
    MouseDown {
        button: MouseButton,
        x: f64,
        y: f64,
    },
    MouseUp {
        button: MouseButton,
        x: f64,
        y: f64,
    },
    MouseScroll {
        delta: ScrollDelta,
        x: f64,
        y: f64,
    },

    // Keyboard events
    KeyDown {
        key: Key,
        modifiers: Modifiers,
        repeat: bool,
    },
    KeyUp {
        key: Key,
        modifiers: Modifiers,
    },
    CharacterInput {
        character: char,
    },

    // Touch events (multi-touch)
    TouchStart {
        id: u64,
        x: f64,
        y: f64,
        pressure: f64,
    },
    TouchMove {
        id: u64,
        x: f64,
        y: f64,
        pressure: f64,
    },
    TouchEnd {
        id: u64,
        x: f64,
        y: f64,
    },
    TouchCancel {
        id: u64,
    },

    // Window events
    FocusGained,
    FocusLost,
    Resize {
        width: u32,
        height: u32,
    },
    ScaleFactorChanged {
        scale: f64,
    },
}

impl InputEvent {
    /// Get the position associated with this event (if any)
    pub fn position(&self) -> Option<(f64, f64)> {
        match self {
            InputEvent::MouseMove { x, y, .. } => Some((*x, *y)),
            InputEvent::MouseDown { x, y, .. } => Some((*x, *y)),
            InputEvent::MouseUp { x, y, .. } => Some((*x, *y)),
            InputEvent::MouseScroll { x, y, .. } => Some((*x, *y)),
            InputEvent::TouchStart { x, y, .. } => Some((*x, *y)),
            InputEvent::TouchMove { x, y, .. } => Some((*x, *y)),
            InputEvent::TouchEnd { x, y, .. } => Some((*x, *y)),
            _ => None,
        }
    }

    /// Check if this is a keyboard event
    pub fn is_keyboard(&self) -> bool {
        matches!(self, InputEvent::KeyDown { .. } | InputEvent::KeyUp { .. } | InputEvent::CharacterInput { .. })
    }

    /// Check if this is a mouse event
    pub fn is_mouse(&self) -> bool {
        matches!(self, InputEvent::MouseMove { .. } | InputEvent::MouseDown { .. } | InputEvent::MouseUp { .. } | InputEvent::MouseScroll { .. })
    }

    /// Check if this is a touch event
    pub fn is_touch(&self) -> bool {
        matches!(self, InputEvent::TouchStart { .. } | InputEvent::TouchMove { .. } | InputEvent::TouchEnd { .. } | InputEvent::TouchCancel { .. })
    }

    /// Convert to browser-core InputEvent
    pub fn to_core(&self) -> Option<browser_core::webview::InputEvent> {
        use browser_core::webview::*;

        match self {
            InputEvent::MouseMove { x, y, .. } => {
                Some(InputEvent::MouseMove(MouseMoveEvent {
                    point: Point2D::new(*x as f32, *y as f32),
                }))
            }
            InputEvent::MouseDown { button, .. } => {
                Some(InputEvent::MouseButton(MouseButtonEvent {
                    button: button.to_core(),
                    action: MouseButtonAction::Down,
                }))
            }
            InputEvent::MouseUp { button, .. } => {
                Some(InputEvent::MouseButton(MouseButtonEvent {
                    button: button.to_core(),
                    action: MouseButtonAction::Up,
                }))
            }
            InputEvent::MouseScroll { delta, .. } => {
                Some(InputEvent::Wheel(WheelEvent {
                    delta: delta.to_core(),
                    mode: WheelMode::DeltaPixel,
                }))
            }
            InputEvent::KeyDown { key, modifiers, .. } => {
                Some(InputEvent::Keyboard(KeyboardEvent {
                    key: key.to_core_string(),
                    code: key.to_core_string(),
                    modifiers: modifiers.to_core(),
                    state: KeyState::Down,
                }))
            }
            InputEvent::KeyUp { key, modifiers, .. } => {
                Some(InputEvent::Keyboard(KeyboardEvent {
                    key: key.to_core_string(),
                    code: key.to_core_string(),
                    modifiers: modifiers.to_core(),
                    state: KeyState::Up,
                }))
            }
            InputEvent::CharacterInput { character } => {
                Some(InputEvent::Keyboard(KeyboardEvent {
                    key: character.to_string(),
                    code: character.to_string(),
                    modifiers: Modifiers::default(),
                    state: KeyState::Down,
                }))
            }
            InputEvent::TouchStart { id, x, y, .. } => {
                Some(InputEvent::Touch(TouchEvent {
                    id: TouchId(*id as i32),
                    point: Point2D::new(*x as f32, *y as f32),
                    event_type: TouchEventType::Down,
                }))
            }
            InputEvent::TouchMove { id, x, y, .. } => {
                Some(InputEvent::Touch(TouchEvent {
                    id: TouchId(*id as i32),
                    point: Point2D::new(*x as f32, *y as f32),
                    event_type: TouchEventType::Move,
                }))
            }
            InputEvent::TouchEnd { id, x, y, .. } => {
                Some(InputEvent::Touch(TouchEvent {
                    id: TouchId(*id as i32),
                    point: Point2D::new(*x as f32, *y as f32),
                    event_type: TouchEventType::Up,
                }))
            }
            InputEvent::TouchCancel { id } => {
                Some(InputEvent::Touch(TouchEvent {
                    id: TouchId(*id as i32),
                    point: Point2D::zero(),
                    event_type: TouchEventType::Cancel,
                }))
            }
            _ => None,
        }
    }
}

/// ====================================================================================
/// INPUT STATE
/// ====================================================================================

/// Tracks the current state of all input devices
#[derive(Debug, Clone)]
pub struct InputState {
    /// Current mouse position
    mouse_position: (f64, f64),
    /// Previous mouse position (for delta calculation)
    last_mouse_position: (f64, f64),
    /// Currently pressed mouse buttons
    pressed_mouse_buttons: HashSet<MouseButton>,
    /// Currently pressed keys
    pressed_keys: HashSet<Key>,
    /// Current modifier state
    modifiers: Modifiers,
    /// Active touch points
    touch_points: HashMap<u64, TouchPoint>,
    /// Window has focus
    has_focus: bool,
    /// Current window size
    window_size: (u32, u32),
    /// Current scale factor
    scale_factor: f64,
}

/// Touch point state
#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    pub id: u64,
    pub x: f64,
    pub y: f64,
    pub pressure: f64,
    pub start_time: Instant,
    pub start_x: f64,
    pub start_y: f64,
}

impl InputState {
    /// Create a new input state
    pub fn new() -> Self {
        Self {
            mouse_position: (0.0, 0.0),
            last_mouse_position: (0.0, 0.0),
            pressed_mouse_buttons: HashSet::new(),
            pressed_keys: HashSet::new(),
            modifiers: Modifiers::default(),
            touch_points: HashMap::with_capacity(MAX_TOUCH_POINTS),
            has_focus: true,
            window_size: (800, 600),
            scale_factor: 1.0,
        }
    }

    /// Update state from an input event
    pub fn update(&mut self, event: &InputEvent) {
        match event {
            InputEvent::MouseMove { x, y, .. } => {
                self.last_mouse_position = self.mouse_position;
                self.mouse_position = (*x, *y);
            }
            InputEvent::MouseDown { button, .. } => {
                self.pressed_mouse_buttons.insert(*button);
            }
            InputEvent::MouseUp { button, .. } => {
                self.pressed_mouse_buttons.remove(button);
            }
            InputEvent::KeyDown { key, modifiers, .. } => {
                self.modifiers = *modifiers;
                self.pressed_keys.insert(key.clone());
            }
            InputEvent::KeyUp { key, modifiers, .. } => {
                self.modifiers = *modifiers;
                self.pressed_keys.remove(key);
            }
            InputEvent::TouchStart { id, x, y, pressure } => {
                if self.touch_points.len() < MAX_TOUCH_POINTS {
                    self.touch_points.insert(*id, TouchPoint {
                        id: *id,
                        x: *x,
                        y: *y,
                        pressure: *pressure,
                        start_time: Instant::now(),
                        start_x: *x,
                        start_y: *y,
                    });
                }
            }
            InputEvent::TouchMove { id, x, y, pressure } => {
                if let Some(point) = self.touch_points.get_mut(id) {
                    point.x = *x;
                    point.y = *y;
                    point.pressure = *pressure;
                }
            }
            InputEvent::TouchEnd { id, .. } | InputEvent::TouchCancel { id } => {
                self.touch_points.remove(id);
            }
            InputEvent::FocusGained => {
                self.has_focus = true;
            }
            InputEvent::FocusLost => {
                self.has_focus = false;
                // Clear pressed keys/buttons when focus is lost
                self.pressed_keys.clear();
                self.pressed_mouse_buttons.clear();
                self.touch_points.clear();
            }
            InputEvent::Resize { width, height } => {
                self.window_size = (*width, *height);
            }
            InputEvent::ScaleFactorChanged { scale } => {
                self.scale_factor = *scale;
            }
            _ => {}
        }
    }

    /// Get current mouse position
    pub fn mouse_position(&self) -> (f64, f64) {
        self.mouse_position
    }

    /// Get mouse delta from last position
    pub fn mouse_delta(&self) -> (f64, f64) {
        (
            self.mouse_position.0 - self.last_mouse_position.0,
            self.mouse_position.1 - self.last_mouse_position.1,
        )
    }

    /// Check if a mouse button is pressed
    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.pressed_mouse_buttons.contains(&button)
    }

    /// Check if any mouse button is pressed
    pub fn is_any_mouse_button_pressed(&self) -> bool {
        !self.pressed_mouse_buttons.is_empty()
    }

    /// Get all pressed mouse buttons
    pub fn pressed_mouse_buttons(&self) -> &HashSet<MouseButton> {
        &self.pressed_mouse_buttons
    }

    /// Check if a key is pressed
    pub fn is_key_pressed(&self, key: &Key) -> bool {
        self.pressed_keys.contains(key)
    }

    /// Check if any key is pressed
    pub fn is_any_key_pressed(&self) -> bool {
        !self.pressed_keys.is_empty()
    }

    /// Get all pressed keys
    pub fn pressed_keys(&self) -> &HashSet<Key> {
        &self.pressed_keys
    }

    /// Get current modifiers
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Get active touch points
    pub fn touch_points(&self) -> &HashMap<u64, TouchPoint> {
        &self.touch_points
    }

    /// Get number of active touch points
    pub fn touch_count(&self) -> usize {
        self.touch_points.len()
    }

    /// Check if window has focus
    pub fn has_focus(&self) -> bool {
        self.has_focus
    }

    /// Get window size
    pub fn window_size(&self) -> (u32, u32) {
        self.window_size
    }

    /// Get scale factor
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Clear all pressed keys and buttons
    pub fn clear(&mut self) {
        self.pressed_keys.clear();
        self.pressed_mouse_buttons.clear();
        self.touch_points.clear();
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

/// ====================================================================================
/// GESTURE RECOGNITION
/// ====================================================================================

/// Recognized gesture types
#[derive(Debug, Clone, PartialEq)]
pub enum Gesture {
    /// Single click
    Click {
        button: MouseButton,
        x: f64,
        y: f64,
    },
    /// Double click
    DoubleClick {
        button: MouseButton,
        x: f64,
        y: f64,
    },
    /// Long press (for context menus)
    LongPress {
        x: f64,
        y: f64,
    },
    /// Pinch zoom gesture
    PinchZoom {
        scale: f64,
        center_x: f64,
        center_y: f64,
    },
    /// Swipe gesture
    Swipe {
        direction: SwipeDirection,
        velocity: f64,
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
    },
    /// Pan gesture (drag)
    Pan {
        delta_x: f64,
        delta_y: f64,
    },
}

/// Swipe directions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeDirection {
    Left,
    Right,
    Up,
    Down,
}

/// Gesture recognizer state
#[derive(Debug)]
pub struct GestureRecognizer {
    /// Click tracking for double-click detection
    last_click: Option<(MouseButton, Instant, f64, f64)>,
    /// Long press tracking
    long_press_timer: Option<Instant>,
    /// Long press position
    long_press_pos: Option<(f64, f64)>,
    /// Touch start positions for pinch detection
    pinch_start_distance: Option<f64>,
    /// Swipe tracking
    swipe_start_pos: Option<(f64, f64, Instant)>,
    /// Detected gestures ready to be consumed
    pending_gestures: VecDeque<Gesture>,
}

impl GestureRecognizer {
    /// Create a new gesture recognizer
    pub fn new() -> Self {
        Self {
            last_click: None,
            long_press_timer: None,
            long_press_pos: None,
            pinch_start_distance: None,
            swipe_start_pos: None,
            pending_gestures: VecDeque::new(),
        }
    }

    /// Process an input event for gesture recognition
    pub fn process_event(&mut self, event: &InputEvent, state: &InputState) {
        match event {
            InputEvent::MouseDown { button, x, y } => {
                self.handle_mouse_down(*button, *x, *y);
            }
            InputEvent::MouseUp { button, x, y } => {
                self.handle_mouse_up(*button, *x, *y, state);
            }
            InputEvent::MouseMove { x, y, .. } => {
                self.handle_mouse_move(*x, *y, state);
            }
            InputEvent::TouchStart { id, x, y, .. } => {
                self.handle_touch_start(*id, *x, *y, state);
            }
            InputEvent::TouchMove { .. } => {
                self.handle_touch_move(state);
            }
            InputEvent::TouchEnd { id, x, y } => {
                self.handle_touch_end(*id, *x, *y, state);
            }
            InputEvent::TouchCancel { .. } => {
                self.reset();
            }
            _ => {}
        }
    }

    fn handle_mouse_down(&mut self, button: MouseButton, x: f64, y: f64) {
        // Start long press timer
        self.long_press_timer = Some(Instant::now());
        self.long_press_pos = Some((x, y));

        // Start swipe tracking
        self.swipe_start_pos = Some((x, y, Instant::now()));
    }

    fn handle_mouse_up(&mut self, button: MouseButton, x: f64, y: f64, state: &InputState) {
        // Check for long press
        if let Some(timer) = self.long_press_timer {
            if timer.elapsed().as_millis() >= LONG_PRESS_THRESHOLD_MS {
                // This was a long press, don't generate click
                self.reset();
                return;
            }
        }

        // Check for click movement
        if let Some((start_x, start_y)) = self.long_press_pos {
            let dx = x - start_x;
            let dy = y - start_y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance > CLICK_MOVEMENT_THRESHOLD {
                // Movement too large, check for swipe
                self.check_for_swipe(start_x, start_y, x, y);
                self.reset();
                return;
            }
        }

        // Check for double click
        if let Some((last_button, last_time, last_x, last_y)) = self.last_click {
            let time_diff = last_time.elapsed().as_millis();
            let dx = x - last_x;
            let dy = y - last_y;
            let distance = (dx * dx + dy * dy).sqrt();

            if button == last_button
                && time_diff < DOUBLE_CLICK_THRESHOLD_MS
                && distance < CLICK_MOVEMENT_THRESHOLD
            {
                // Double click detected
                self.pending_gestures.push_back(Gesture::DoubleClick {
                    button,
                    x,
                    y,
                });
                self.last_click = None;
                self.reset();
                return;
            }
        }

        // Single click
        self.pending_gestures.push_back(Gesture::Click {
            button,
            x,
            y,
        });

        // Record for double-click detection
        self.last_click = Some((button, Instant::now(), x, y));
        self.reset();
    }

    fn handle_mouse_move(&mut self, x: f64, y: f64, state: &InputState) {
        // Check for pan gesture when mouse is down
        if state.is_any_mouse_button_pressed() {
            if let Some((start_x, start_y, _)) = self.swipe_start_pos {
                let delta_x = x - start_x;
                let delta_y = y - start_y;

                // Only generate pan if moved enough
                if delta_x.abs() > 5.0 || delta_y.abs() > 5.0 {
                    self.pending_gestures.push_back(Gesture::Pan {
                        delta_x,
                        delta_y,
                    });
                    self.swipe_start_pos = Some((x, y, Instant::now()));
                }
            }
        }

        // Cancel long press if moved too much
        if let Some((start_x, start_y)) = self.long_press_pos {
            let dx = x - start_x;
            let dy = y - start_y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance > CLICK_MOVEMENT_THRESHOLD {
                self.long_press_timer = None;
                self.long_press_pos = None;
            }
        }
    }

    fn handle_touch_start(&mut self, id: u64, x: f64, y: f64, state: &InputState) {
        let touch_count = state.touch_count();

        if touch_count == 1 {
            // Single touch - start long press and swipe tracking
            self.long_press_timer = Some(Instant::now());
            self.long_press_pos = Some((x, y));
            self.swipe_start_pos = Some((x, y, Instant::now()));
        } else if touch_count == 2 {
            // Two touches - start pinch tracking
            let points: Vec<_> = state.touch_points().values().collect();
            if points.len() >= 2 {
                let dx = points[0].x - points[1].x;
                let dy = points[0].y - points[1].y;
                let distance = (dx * dx + dy * dy).sqrt();
                self.pinch_start_distance = Some(distance);
            }
            // Cancel long press
            self.long_press_timer = None;
        }
    }

    fn handle_touch_move(&mut self, state: &InputState) {
        let touch_count = state.touch_count();

        if touch_count == 2 {
            // Check for pinch zoom
            if let Some(start_distance) = self.pinch_start_distance {
                let points: Vec<_> = state.touch_points().values().collect();
                if points.len() >= 2 {
                    let dx = points[0].x - points[1].x;
                    let dy = points[0].y - points[1].y;
                    let current_distance = (dx * dx + dy * dy).sqrt();

                    if (current_distance - start_distance).abs() > PINCH_ZOOM_THRESHOLD {
                        let scale = current_distance / start_distance;
                        let center_x = (points[0].x + points[1].x) / 2.0;
                        let center_y = (points[0].y + points[1].y) / 2.0;

                        self.pending_gestures.push_back(Gesture::PinchZoom {
                            scale,
                            center_x,
                            center_y,
                        });

                        // Reset for continuous pinch
                        self.pinch_start_distance = Some(current_distance);
                    }
                }
            }
        }

        // Cancel long press on significant movement
        if let Some((start_x, start_y)) = self.long_press_pos {
            if let Some(point) = state.touch_points().values().next() {
                let dx = point.x - start_x;
                let dy = point.y - start_y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance > CLICK_MOVEMENT_THRESHOLD {
                    self.long_press_timer = None;
                }
            }
        }
    }

    fn handle_touch_end(&mut self, id: u64, x: f64, y: f64, state: &InputState) {
        let touch_count = state.touch_count();

        if touch_count == 0 {
            // All touches ended - check for tap or swipe
            if let Some((start_x, start_y)) = self.long_press_pos {
                let dx = x - start_x;
                let dy = y - start_y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance > CLICK_MOVEMENT_THRESHOLD {
                    self.check_for_swipe(start_x, start_y, x, y);
                } else if let Some(timer) = self.long_press_timer {
                    if timer.elapsed().as_millis() < LONG_PRESS_THRESHOLD_MS {
                        // Tap detected
                        self.pending_gestures.push_back(Gesture::Click {
                            button: MouseButton::Left,
                            x,
                            y,
                        });
                    } else {
                        // Long press
                        self.pending_gestures.push_back(Gesture::LongPress { x, y });
                    }
                }
            }

            self.reset();
        } else if touch_count == 1 {
            // One touch remaining - end pinch
            self.pinch_start_distance = None;
        }
    }

    fn check_for_swipe(&mut self, start_x: f64, start_y: f64, end_x: f64, end_y: f64) {
        if let Some((_, _, start_time)) = self.swipe_start_pos {
            let dx = end_x - start_x;
            let dy = end_y - start_y;
            let dt = start_time.elapsed().as_secs_f64();

            if dt > 0.0 {
                let vx = dx / dt;
                let vy = dy / dt;
                let velocity = (vx * vx + vy * vy).sqrt();

                if velocity > SWIPE_VELOCITY_THRESHOLD {
                    let direction = if dx.abs() > dy.abs() {
                        if dx > 0.0 {
                            SwipeDirection::Right
                        } else {
                            SwipeDirection::Left
                        }
                    } else {
                        if dy > 0.0 {
                            SwipeDirection::Down
                        } else {
                            SwipeDirection::Up
                        }
                    };

                    self.pending_gestures.push_back(Gesture::Swipe {
                        direction,
                        velocity,
                        start_x,
                        start_y,
                        end_x,
                        end_y,
                    });
                }
            }
        }
    }

    fn reset(&mut self) {
        self.long_press_timer = None;
        self.long_press_pos = None;
        self.pinch_start_distance = None;
        self.swipe_start_pos = None;
    }

    /// Poll for the next detected gesture
    pub fn next_gesture(&mut self) -> Option<Gesture> {
        self.pending_gestures.pop_front()
    }

    /// Check if there are pending gestures
    pub fn has_gestures(&self) -> bool {
        !self.pending_gestures.is_empty()
    }

    /// Get all pending gestures
    pub fn drain_gestures(&mut self) -> Vec<Gesture> {
        self.pending_gestures.drain(..).collect()
    }

    /// Update for long press detection (call regularly)
    pub fn update(&mut self) {
        if let Some(timer) = self.long_press_timer {
            if timer.elapsed().as_millis() >= LONG_PRESS_THRESHOLD_MS {
                if let Some((x, y)) = self.long_press_pos {
                    self.pending_gestures.push_back(Gesture::LongPress { x, y });
                    self.long_press_timer = None;
                }
            }
        }
    }
}

impl Default for GestureRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

/// ====================================================================================
/// FOCUS MANAGEMENT
/// ====================================================================================

/// Focusable element identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusId(pub u64);

impl FocusId {
    /// Generate a new unique focus ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        FocusId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for FocusId {
    fn default() -> Self {
        Self::new()
    }
}

/// Focusable element information
#[derive(Debug, Clone)]
pub struct FocusableElement {
    pub id: FocusId,
    pub tab_index: i32,
    pub enabled: bool,
    pub visible: bool,
    pub bounds: (f64, f64, f64, f64), // x, y, width, height
}

/// Focus manager for handling keyboard navigation
#[derive(Debug)]
pub struct FocusManager {
    /// Currently focused element
    focused_id: Option<FocusId>,
    /// All registered focusable elements
    elements: HashMap<FocusId, FocusableElement>,
    /// Tab order (sorted by tab_index)
    tab_order: Vec<FocusId>,
    /// Focus history for shift+tab
    focus_history: VecDeque<FocusId>,
    /// Maximum history size
    max_history: usize,
}

impl FocusManager {
    /// Create a new focus manager
    pub fn new() -> Self {
        Self {
            focused_id: None,
            elements: HashMap::new(),
            tab_order: Vec::new(),
            focus_history: VecDeque::with_capacity(16),
            max_history: 16,
        }
    }

    /// Register a focusable element
    pub fn register_element(&mut self, element: FocusableElement) {
        let id = element.id;
        self.elements.insert(id, element);
        self.rebuild_tab_order();
    }

    /// Unregister a focusable element
    pub fn unregister_element(&mut self, id: FocusId) {
        self.elements.remove(&id);
        if self.focused_id == Some(id) {
            self.focused_id = None;
        }
        self.rebuild_tab_order();
    }

    /// Update an element's properties
    pub fn update_element(&mut self, id: FocusId, f: impl FnOnce(&mut FocusableElement)) {
        if let Some(element) = self.elements.get_mut(&id) {
            f(element);
            self.rebuild_tab_order();
        }
    }

    /// Set element bounds
    pub fn set_bounds(&mut self, id: FocusId, bounds: (f64, f64, f64, f64)) {
        if let Some(element) = self.elements.get_mut(&id) {
            element.bounds = bounds;
        }
    }

    /// Set element enabled state
    pub fn set_enabled(&mut self, id: FocusId, enabled: bool) {
        if let Some(element) = self.elements.get_mut(&id) {
            element.enabled = enabled;
            if !enabled && self.focused_id == Some(id) {
                self.focus_next();
            }
        }
    }

    /// Set element visibility
    pub fn set_visible(&mut self, id: FocusId, visible: bool) {
        if let Some(element) = self.elements.get_mut(&id) {
            element.visible = visible;
            if !visible && self.focused_id == Some(id) {
                self.focus_next();
            }
        }
    }

    /// Focus a specific element
    pub fn focus(&mut self, id: FocusId) -> bool {
        if let Some(element) = self.elements.get(&id) {
            if element.enabled && element.visible {
                let old_focus = self.focused_id;
                self.focused_id = Some(id);

                // Add to history
                if let Some(old) = old_focus {
                    if old != id {
                        self.focus_history.push_back(old);
                        if self.focus_history.len() > self.max_history {
                            self.focus_history.pop_front();
                        }
                    }
                }

                return true;
            }
        }
        false
    }

    /// Focus the next element in tab order
    pub fn focus_next(&mut self) -> Option<FocusId> {
        if self.tab_order.is_empty() {
            return None;
        }

        let start_idx = self.focused_id
            .and_then(|id| self.tab_order.iter().position(|&x| x == id))
            .map(|i| (i + 1) % self.tab_order.len())
            .unwrap_or(0);

        for i in 0..self.tab_order.len() {
            let idx = (start_idx + i) % self.tab_order.len();
            let id = self.tab_order[idx];

            if let Some(element) = self.elements.get(&id) {
                if element.enabled && element.visible {
                    self.focus(id);
                    return Some(id);
                }
            }
        }

        None
    }

    /// Focus the previous element in tab order
    pub fn focus_previous(&mut self) -> Option<FocusId> {
        if self.tab_order.is_empty() {
            return None;
        }

        // Try history first
        while let Some(id) = self.focus_history.pop_back() {
            if let Some(element) = self.elements.get(&id) {
                if element.enabled && element.visible && Some(id) != self.focused_id {
                    self.focused_id = Some(id);
                    return Some(id);
                }
            }
        }

        // Fall back to tab order
        let start_idx = self.focused_id
            .and_then(|id| self.tab_order.iter().position(|&x| x == id))
            .map(|i| if i == 0 { self.tab_order.len() - 1 } else { i - 1 })
            .unwrap_or(self.tab_order.len().saturating_sub(1));

        for i in 0..self.tab_order.len() {
            let idx = (start_idx + self.tab_order.len() - i) % self.tab_order.len();
            let id = self.tab_order[idx];

            if let Some(element) = self.elements.get(&id) {
                if element.enabled && element.visible {
                    self.focus(id);
                    return Some(id);
                }
            }
        }

        None
    }

    /// Focus element at a specific point (for mouse/touch)
    pub fn focus_at_point(&mut self, x: f64, y: f64) -> Option<FocusId> {
        // Find the topmost element at this point
        // (iterate in reverse tab order to get the most recently added/focused)
        for id in self.tab_order.iter().rev() {
            if let Some(element) = self.elements.get(id) {
                if element.enabled && element.visible {
                    let (ex, ey, ew, eh) = element.bounds;
                    if x >= ex && x < ex + ew && y >= ey && y < ey + eh {
                        self.focus(*id);
                        return Some(*id);
                    }
                }
            }
        }
        None
    }

    /// Get the currently focused element ID
    pub fn focused_id(&self) -> Option<FocusId> {
        self.focused_id
    }

    /// Get the currently focused element
    pub fn focused_element(&self) -> Option<&FocusableElement> {
        self.focused_id.and_then(|id| self.elements.get(&id))
    }

    /// Check if an element is focused
    pub fn is_focused(&self, id: FocusId) -> bool {
        self.focused_id == Some(id)
    }

    /// Clear focus
    pub fn clear_focus(&mut self) {
        self.focused_id = None;
    }

    /// Handle tab navigation
    pub fn handle_tab(&mut self, shift: bool) -> Option<FocusId> {
        if shift {
            self.focus_previous()
        } else {
            self.focus_next()
        }
    }

    /// Rebuild the tab order
    fn rebuild_tab_order(&mut self) {
        let mut elements: Vec<_> = self.elements
            .values()
            .filter(|e| e.visible)
            .collect();

        // Sort by tab_index
        elements.sort_by_key(|e| e.tab_index);

        self.tab_order = elements.into_iter().map(|e| e.id).collect();
    }

    /// Get all registered elements
    pub fn elements(&self) -> &HashMap<FocusId, FocusableElement> {
        &self.elements
    }

    /// Clear all elements
    pub fn clear(&mut self) {
        self.elements.clear();
        self.tab_order.clear();
        self.focus_history.clear();
        self.focused_id = None;
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

/// ====================================================================================
/// INPUT MANAGER
/// ====================================================================================

/// Main input manager that coordinates all input handling
#[derive(Debug)]
pub struct InputManager {
    /// Event batcher for frame-based processing
    batcher: InputBatcher,
    /// Current input state
    state: Arc<RwLock<InputState>>,
    /// Gesture recognizer
    gesture_recognizer: Arc<Mutex<GestureRecognizer>>,
    /// Focus manager
    focus_manager: Arc<RwLock<FocusManager>>,
    /// Event queue for cross-thread communication
    event_queue: Arc<Mutex<VecDeque<InputEvent>>>,
    /// Whether the manager is active
    active: AtomicBool,
    /// Last update time
    last_update: Instant,
}

impl InputManager {
    /// Create a new input manager
    pub fn new() -> Self {
        Self {
            batcher: InputBatcher::new(MAX_EVENTS_PER_FRAME),
            state: Arc::new(RwLock::new(InputState::new())),
            gesture_recognizer: Arc::new(Mutex::new(GestureRecognizer::new())),
            focus_manager: Arc::new(RwLock::new(FocusManager::new())),
            event_queue: Arc::new(Mutex::new(VecDeque::new())),
            active: AtomicBool::new(true),
            last_update: Instant::now(),
        }
    }

    /// Process a single input event
    pub fn process_event(&self, event: InputEvent) {
        if !self.active.load(Ordering::SeqCst) {
            return;
        }

        // Update state
        {
            let mut state = self.state.write();
            state.update(&event);
        }

        // Process for gestures
        {
            let state = self.state.read();
            let mut recognizer = self.gesture_recognizer.lock();
            recognizer.process_event(&event, &state);
        }

        // Handle focus for mouse/touch events
        match &event {
            InputEvent::MouseDown { x, y, .. } => {
                let mut focus = self.focus_manager.write();
                focus.focus_at_point(*x, *y);
            }
            InputEvent::TouchStart { x, y, .. } => {
                let mut focus = self.focus_manager.write();
                focus.focus_at_point(*x, *y);
            }
            InputEvent::KeyDown { key, modifiers, .. } => {
                if let Key::Named(NamedKey::Tab) = key {
                    let mut focus = self.focus_manager.write();
                    focus.handle_tab(modifiers.shift);
                }
            }
            _ => {}
        }

        // Add to batcher
        self.batcher.push(event.to_gpu_event());
    }

    /// Queue an event for later processing (thread-safe)
    pub fn queue_event(&self, event: InputEvent) {
        let mut queue = self.event_queue.lock();
        if queue.len() < MAX_EVENTS_PER_FRAME {
            queue.push_back(event);
        } else {
            warn!("Input event queue full, dropping event");
        }
    }

    /// Process all queued events
    pub fn process_queued_events(&self) {
        let mut queue = self.event_queue.lock();
        while let Some(event) = queue.pop_front() {
            drop(queue);
            self.process_event(event);
            queue = self.event_queue.lock();
        }
    }

    /// Get batched events for the current frame
    pub fn drain_batched_events(&mut self) -> Vec<GpuInputEvent> {
        self.batcher.drain()
    }

    /// Get the current input state
    pub fn state(&self) -> Arc<RwLock<InputState>> {
        self.state.clone()
    }

    /// Get the gesture recognizer
    pub fn gesture_recognizer(&self) -> Arc<Mutex<GestureRecognizer>> {
        self.gesture_recognizer.clone()
    }

    /// Get the focus manager
    pub fn focus_manager(&self) -> Arc<RwLock<FocusManager>> {
        self.focus_manager.clone()
    }

    /// Poll for the next gesture
    pub fn next_gesture(&self) -> Option<Gesture> {
        let mut recognizer = self.gesture_recognizer.lock();
        recognizer.next_gesture()
    }

    /// Get all pending gestures
    pub fn drain_gestures(&self) -> Vec<Gesture> {
        let mut recognizer = self.gesture_recognizer.lock();
        recognizer.drain_gestures()
    }

    /// Check if there are pending gestures
    pub fn has_gestures(&self) -> bool {
        let recognizer = self.gesture_recognizer.lock();
        recognizer.has_gestures()
    }

    /// Update the input manager (call once per frame)
    pub fn update(&mut self) {
        self.process_queued_events();

        // Update gesture recognizer for long press detection
        {
            let mut recognizer = self.gesture_recognizer.lock();
            recognizer.update();
        }

        self.last_update = Instant::now();
    }

    /// Check if the manager is active
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    /// Set the active state
    pub fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::SeqCst);
        if !active {
            // Clear state when deactivated
            let mut state = self.state.write();
            state.clear();
        }
    }

    /// Clear all input state
    pub fn clear(&mut self) {
        self.batcher.clear();
        {
            let mut state = self.state.write();
            state.clear();
        }
        {
            let mut queue = self.event_queue.lock();
            queue.clear();
        }
    }

    /// Get time since last update
    pub fn time_since_update(&self) -> Duration {
        self.last_update.elapsed()
    }
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}

/// ====================================================================================
/// EVENT CONVERSION TRAITS
/// ====================================================================================

/// Trait for converting to GPU input events
trait ToGpuEvent {
    fn to_gpu_event(&self) -> GpuInputEvent;
}

impl ToGpuEvent for InputEvent {
    fn to_gpu_event(&self) -> GpuInputEvent {
        match self {
            InputEvent::MouseMove { x, y, .. } => GpuInputEvent::MouseMove { x: *x, y: *y },
            InputEvent::MouseDown { button, x, y } => {
                let button_num = match button {
                    MouseButton::Left => 0,
                    MouseButton::Right => 1,
                    MouseButton::Middle => 2,
                    MouseButton::Back => 3,
                    MouseButton::Forward => 4,
                    MouseButton::Other(n) => *n as u32,
                };
                GpuInputEvent::MouseDown { button: button_num, x: *x, y: *y }
            }
            InputEvent::MouseUp { button, x, y } => {
                let button_num = match button {
                    MouseButton::Left => 0,
                    MouseButton::Right => 1,
                    MouseButton::Middle => 2,
                    MouseButton::Back => 3,
                    MouseButton::Forward => 4,
                    MouseButton::Other(n) => *n as u32,
                };
                GpuInputEvent::MouseUp { button: button_num, x: *x, y: *y }
            }
            InputEvent::MouseScroll { delta, .. } => {
                let (dx, dy) = match delta {
                    ScrollDelta::Pixels { x, y } => (*x, *y),
                    ScrollDelta::Lines { x, y } => (*x * 20.0, *y * 20.0),
                };
                GpuInputEvent::Scroll { delta_x: dx, delta_y: dy }
            }
            InputEvent::KeyDown { key, modifiers, .. } => {
                let key_code = key_to_code(key);
                let mods = modifiers_to_bits(*modifiers);
                GpuInputEvent::KeyDown { key_code, modifiers: mods }
            }
            InputEvent::KeyUp { key, modifiers, .. } => {
                let key_code = key_to_code(key);
                let mods = modifiers_to_bits(*modifiers);
                GpuInputEvent::KeyUp { key_code, modifiers: mods }
            }
            InputEvent::CharacterInput { character } => {
                GpuInputEvent::CharacterInput { character: *character }
            }
            InputEvent::TouchStart { id, x, y, .. } => {
                GpuInputEvent::TouchStart { id: *id, x: *x, y: *y }
            }
            InputEvent::TouchMove { id, x, y, .. } => {
                GpuInputEvent::TouchMove { id: *id, x: *x, y: *y }
            }
            InputEvent::TouchEnd { id, x, y } => {
                GpuInputEvent::TouchEnd { id: *id, x: *x, y: *y }
            }
            InputEvent::Resize { width, height } => {
                GpuInputEvent::Resize { width: *width, height: *height }
            }
            InputEvent::FocusGained => GpuInputEvent::Focus { focused: true },
            InputEvent::FocusLost => GpuInputEvent::Focus { focused: false },
            _ => GpuInputEvent::Focus { focused: true }, // Default fallback
        }
    }
}

/// Convert key to virtual key code
fn key_to_code(key: &Key) -> u32 {
    match key {
        Key::Named(named) => named_key_to_code(*named),
        Key::Character(c) => {
            // Return ASCII code for single characters
            c.chars().next().map(|ch| ch as u32).unwrap_or(0)
        }
        Key::Unidentified => 0,
    }
}

/// Convert named key to virtual key code
fn named_key_to_code(key: NamedKey) -> u32 {
    match key {
        NamedKey::Enter => 13,
        NamedKey::Tab => 9,
        NamedKey::Space => 32,
        NamedKey::ArrowUp => 38,
        NamedKey::ArrowDown => 40,
        NamedKey::ArrowLeft => 37,
        NamedKey::ArrowRight => 39,
        NamedKey::Home => 36,
        NamedKey::End => 35,
        NamedKey::PageUp => 33,
        NamedKey::PageDown => 34,
        NamedKey::Backspace => 8,
        NamedKey::Delete => 46,
        NamedKey::Escape => 27,
        NamedKey::Insert => 45,
        NamedKey::F1 => 112,
        NamedKey::F2 => 113,
        NamedKey::F3 => 114,
        NamedKey::F4 => 115,
        NamedKey::F5 => 116,
        NamedKey::F6 => 117,
        NamedKey::F7 => 118,
        NamedKey::F8 => 119,
        NamedKey::F9 => 120,
        NamedKey::F10 => 121,
        NamedKey::F11 => 122,
        NamedKey::F12 => 123,
        NamedKey::Shift => 16,
        NamedKey::Control => 17,
        NamedKey::Alt => 18,
        NamedKey::Meta => 91,
        NamedKey::CapsLock => 20,
        _ => 0,
    }
}

/// Convert modifiers to bit flags
fn modifiers_to_bits(modifiers: Modifiers) -> u32 {
    let mut bits = 0u32;
    if modifiers.shift {
        bits |= 1;
    }
    if modifiers.ctrl {
        bits |= 2;
    }
    if modifiers.alt {
        bits |= 4;
    }
    if modifiers.meta {
        bits |= 8;
    }
    bits
}

/// ====================================================================================
/// ICED INTEGRATION
/// ====================================================================================

/// Convert Iced mouse event to InputEvent
pub fn from_iced_mouse_event(
    event: iced::mouse::Event,
    position: iced::Point,
) -> Option<InputEvent> {
    match event {
        iced::mouse::Event::CursorMoved { .. } => Some(InputEvent::MouseMove {
            x: position.x as f64,
            y: position.y as f64,
            delta_x: 0.0,
            delta_y: 0.0,
        }),
        iced::mouse::Event::ButtonPressed(button) => Some(InputEvent::MouseDown {
            button: MouseButton::from_iced(button),
            x: position.x as f64,
            y: position.y as f64,
        }),
        iced::mouse::Event::ButtonReleased(button) => Some(InputEvent::MouseUp {
            button: MouseButton::from_iced(button),
            x: position.x as f64,
            y: position.y as f64,
        }),
        iced::mouse::Event::WheelScrolled { delta } => {
            let (dx, dy) = match delta {
                iced::mouse::ScrollDelta::Lines { x, y } => (x as f64 * 20.0, y as f64 * 20.0),
                iced::mouse::ScrollDelta::Pixels { x, y } => (x as f64, y as f64),
            };
            Some(InputEvent::MouseScroll {
                delta: ScrollDelta::Pixels { x: dx, y: dy },
                x: position.x as f64,
                y: position.y as f64,
            })
        }
        _ => None,
    }
}

/// Convert Iced keyboard event to InputEvent
pub fn from_iced_keyboard_event(event: iced::keyboard::Event) -> Option<InputEvent> {
    match event {
        iced::keyboard::Event::KeyPressed { key, modifiers, .. } => Some(InputEvent::KeyDown {
            key: Key::from_iced(&key),
            modifiers: Modifiers::from_iced(modifiers),
            repeat: false,
        }),
        iced::keyboard::Event::KeyReleased { key, modifiers, .. } => Some(InputEvent::KeyUp {
            key: Key::from_iced(&key),
            modifiers: Modifiers::from_iced(modifiers),
        }),
        iced::keyboard::Event::CharacterReceived(c) => Some(InputEvent::CharacterInput { character: c }),
        iced::keyboard::Event::ModifiersChanged(modifiers) => {
            // This event only changes modifiers, we don't create a separate event
            None
        }
    }
}

/// ====================================================================================
/// WINIT INTEGRATION
/// ====================================================================================

#[cfg(feature = "winit")]
/// Convert winit window event to InputEvent
pub fn from_winit_window_event(event: &winit::event::WindowEvent) -> Option<InputEvent> {
    use winit::event::WindowEvent;

    match event {
        WindowEvent::CursorMoved { position, .. } => Some(InputEvent::MouseMove {
            x: position.x,
            y: position.y,
            delta_x: 0.0,
            delta_y: 0.0,
        }),
        WindowEvent::MouseInput { state, button, .. } => {
            // Note: position would need to be tracked separately
            let (x, y) = (0.0, 0.0); // Should be retrieved from cursor position tracking
            match state {
                winit::event::ElementState::Pressed => Some(InputEvent::MouseDown {
                    button: MouseButton::from_winit(*button),
                    x,
                    y,
                }),
                winit::event::ElementState::Released => Some(InputEvent::MouseUp {
                    button: MouseButton::from_winit(*button),
                    x,
                    y,
                }),
            }
        }
        WindowEvent::MouseWheel { delta, .. } => {
            let (x, y) = (0.0, 0.0); // Should be retrieved from cursor position tracking
            Some(InputEvent::MouseScroll {
                delta: ScrollDelta::from_winit(*delta),
                x,
                y,
            })
        }
        WindowEvent::KeyboardInput { event, .. } => {
            let key = Key::from_winit(&event.logical_key);
            let modifiers = Modifiers::from_winit(event.state.into());
            match event.state {
                winit::event::ElementState::Pressed => Some(InputEvent::KeyDown {
                    key,
                    modifiers,
                    repeat: event.repeat,
                }),
                winit::event::ElementState::Released => Some(InputEvent::KeyUp {
                    key,
                    modifiers,
                }),
            }
        }
        WindowEvent::Ime(ime) => {
            match ime {
                winit::event::Ime::Commit(text) => {
                    text.chars().next().map(|c| InputEvent::CharacterInput { character: c })
                }
                _ => None,
            }
        }
        WindowEvent::Touch(touch) => {
            match touch.phase {
                winit::event::TouchPhase::Started => Some(InputEvent::TouchStart {
                    id: touch.id,
                    x: touch.location.x,
                    y: touch.location.y,
                    pressure: touch.force.map(|f| f.normalized() as f64).unwrap_or(1.0),
                }),
                winit::event::TouchPhase::Moved => Some(InputEvent::TouchMove {
                    id: touch.id,
                    x: touch.location.x,
                    y: touch.location.y,
                    pressure: touch.force.map(|f| f.normalized() as f64).unwrap_or(1.0),
                }),
                winit::event::TouchPhase::Ended => Some(InputEvent::TouchEnd {
                    id: touch.id,
                    x: touch.location.x,
                    y: touch.location.y,
                }),
                winit::event::TouchPhase::Cancelled => Some(InputEvent::TouchCancel {
                    id: touch.id,
                }),
            }
        }
        WindowEvent::Focused(focused) => {
            if *focused {
                Some(InputEvent::FocusGained)
            } else {
                Some(InputEvent::FocusLost)
            }
        }
        WindowEvent::Resized(size) => Some(InputEvent::Resize {
            width: size.width,
            height: size.height,
        }),
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => Some(InputEvent::ScaleFactorChanged {
            scale: *scale_factor,
        }),
        _ => None,
    }
}

/// ====================================================================================
/// ACCESSIBILITY SUPPORT
/// ====================================================================================

/// Accessibility announcement types
#[derive(Debug, Clone)]
pub enum AccessibilityAnnouncement {
    /// Announce text to screen reader
    Text(String),
    /// Announce focus change
    FocusChanged { element_id: FocusId, label: String },
    /// Announce value change
    ValueChanged { element_id: FocusId, value: String },
    /// Announce state change
    StateChanged { element_id: FocusId, state: String },
}

/// Accessibility support for input handling
#[derive(Debug)]
pub struct AccessibilitySupport {
    /// Screen reader enabled
    screen_reader_enabled: bool,
    /// High contrast mode
    high_contrast: bool,
    /// Reduced motion preference
    reduced_motion: bool,
    /// Announcement queue
    announcements: VecDeque<AccessibilityAnnouncement>,
}

impl AccessibilitySupport {
    /// Create new accessibility support
    pub fn new() -> Self {
        Self {
            screen_reader_enabled: false,
            high_contrast: false,
            reduced_motion: false,
            announcements: VecDeque::new(),
        }
    }

    /// Announce to screen reader
    pub fn announce(&mut self, announcement: AccessibilityAnnouncement) {
        if self.screen_reader_enabled {
            self.announcements.push_back(announcement);
        }
    }

    /// Get next announcement
    pub fn next_announcement(&mut self) -> Option<AccessibilityAnnouncement> {
        self.announcements.pop_front()
    }

    /// Set screen reader enabled
    pub fn set_screen_reader_enabled(&mut self, enabled: bool) {
        self.screen_reader_enabled = enabled;
    }

    /// Check if screen reader is enabled
    pub fn is_screen_reader_enabled(&self) -> bool {
        self.screen_reader_enabled
    }

    /// Set high contrast mode
    pub fn set_high_contrast(&mut self, enabled: bool) {
        self.high_contrast = enabled;
    }

    /// Check if high contrast is enabled
    pub fn is_high_contrast(&self) -> bool {
        self.high_contrast
    }

    /// Set reduced motion preference
    pub fn set_reduced_motion(&mut self, enabled: bool) {
        self.reduced_motion = enabled;
    }

    /// Check if reduced motion is preferred
    pub fn is_reduced_motion(&self) -> bool {
        self.reduced_motion
    }
}

impl Default for AccessibilitySupport {
    fn default() -> Self {
        Self::new()
    }
}

/// ====================================================================================
/// ERROR HANDLING
/// ====================================================================================

/// Input system errors
#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("Event queue overflow")]
    QueueOverflow,
    #[error("Invalid event: {0}")]
    InvalidEvent(String),
    #[error("Focus element not found: {0:?}")]
    ElementNotFound(FocusId),
    #[error("Gesture recognition error: {0}")]
    GestureError(String),
}

/// Result type for input operations
pub type InputResult<T> = Result<T, InputError>;

/// ====================================================================================
/// TESTS
/// ====================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_state_mouse() {
        let mut state = InputState::new();

        // Initial state
        assert_eq!(state.mouse_position(), (0.0, 0.0));
        assert!(!state.is_any_mouse_button_pressed());

        // Mouse move
        state.update(&InputEvent::MouseMove {
            x: 100.0,
            y: 200.0,
            delta_x: 100.0,
            delta_y: 200.0,
        });
        assert_eq!(state.mouse_position(), (100.0, 200.0));

        // Mouse down
        state.update(&InputEvent::MouseDown {
            button: MouseButton::Left,
            x: 100.0,
            y: 200.0,
        });
        assert!(state.is_mouse_button_pressed(MouseButton::Left));
        assert!(state.is_any_mouse_button_pressed());

        // Mouse up
        state.update(&InputEvent::MouseUp {
            button: MouseButton::Left,
            x: 100.0,
            y: 200.0,
        });
        assert!(!state.is_mouse_button_pressed(MouseButton::Left));
    }

    #[test]
    fn test_input_state_keyboard() {
        let mut state = InputState::new();

        // Key down
        let key = Key::Character("a".to_string());
        state.update(&InputEvent::KeyDown {
            key: key.clone(),
            modifiers: Modifiers::default(),
            repeat: false,
        });
        assert!(state.is_key_pressed(&key));
        assert!(state.is_any_key_pressed());

        // Key up
        state.update(&InputEvent::KeyUp {
            key: key.clone(),
            modifiers: Modifiers::default(),
        });
        assert!(!state.is_key_pressed(&key));
    }

    #[test]
    fn test_input_state_modifiers() {
        let mut state = InputState::new();

        state.update(&InputEvent::KeyDown {
            key: Key::Character("a".to_string()),
            modifiers: Modifiers::new(true, true, false, false),
            repeat: false,
        });

        assert!(state.modifiers().shift);
        assert!(state.modifiers().ctrl);
        assert!(!state.modifiers().alt);
    }

    #[test]
    fn test_input_state_touch() {
        let mut state = InputState::new();

        // Touch start
        state.update(&InputEvent::TouchStart {
            id: 1,
            x: 100.0,
            y: 200.0,
            pressure: 0.5,
        });
        assert_eq!(state.touch_count(), 1);

        // Second touch
        state.update(&InputEvent::TouchStart {
            id: 2,
            x: 150.0,
            y: 250.0,
            pressure: 0.7,
        });
        assert_eq!(state.touch_count(), 2);

        // Touch end
        state.update(&InputEvent::TouchEnd {
            id: 1,
            x: 100.0,
            y: 200.0,
        });
        assert_eq!(state.touch_count(), 1);
    }

    #[test]
    fn test_gesture_recognizer_click() {
        let mut recognizer = GestureRecognizer::new();
        let state = InputState::new();

        // Mouse down
        recognizer.process_event(&InputEvent::MouseDown {
            button: MouseButton::Left,
            x: 100.0,
            y: 200.0,
        }, &state);

        // Mouse up quickly (click)
        recognizer.process_event(&InputEvent::MouseUp {
            button: MouseButton::Left,
            x: 100.0,
            y: 200.0,
        }, &state);

        let gesture = recognizer.next_gesture();
        assert!(matches!(gesture, Some(Gesture::Click { .. })));
    }

    #[test]
    fn test_focus_manager() {
        let mut focus = FocusManager::new();

        // Register elements
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        let id3 = FocusId::new();

        focus.register_element(FocusableElement {
            id: id1,
            tab_index: 0,
            enabled: true,
            visible: true,
            bounds: (0.0, 0.0, 100.0, 100.0),
        });
        focus.register_element(FocusableElement {
            id: id2,
            tab_index: 1,
            enabled: true,
            visible: true,
            bounds: (100.0, 0.0, 100.0, 100.0),
        });
        focus.register_element(FocusableElement {
            id: id3,
            tab_index: 2,
            enabled: false, // Disabled
            visible: true,
            bounds: (200.0, 0.0, 100.0, 100.0),
        });

        // Focus next
        assert_eq!(focus.focus_next(), Some(id1));
        assert_eq!(focus.focus_next(), Some(id2));
        // Should skip id3 (disabled) and wrap to id1
        assert_eq!(focus.focus_next(), Some(id1));

        // Focus at point
        assert_eq!(focus.focus_at_point(150.0, 50.0), Some(id2));
        assert!(focus.is_focused(id2));
    }

    #[test]
    fn test_modifiers() {
        let mods = Modifiers::new(true, true, false, false);
        assert!(mods.shift);
        assert!(mods.ctrl);
        assert!(!mods.alt);
        assert!(!mods.meta);
        assert!(mods.any());
        assert!(!mods.is_empty());

        let empty = Modifiers::default();
        assert!(!empty.any());
        assert!(empty.is_empty());
    }

    #[test]
    fn test_mouse_button_conversion() {
        let left = MouseButton::Left;
        assert!(matches!(left.to_core(), browser_core::webview::MouseButton::Left));

        let right = MouseButton::Right;
        assert!(matches!(right.to_core(), browser_core::webview::MouseButton::Right));
    }

    #[test]
    fn test_key_conversion() {
        let key = Key::Character("a".to_string());
        assert_eq!(key.to_core_string(), "a");

        let named = Key::Named(NamedKey::Enter);
        assert_eq!(named.to_core_string(), "Enter");
    }

    #[test]
    fn test_input_event_to_core() {
        let mouse_move = InputEvent::MouseMove {
            x: 100.0,
            y: 200.0,
            delta_x: 10.0,
            delta_y: 20.0,
        };
        let core = mouse_move.to_core();
        assert!(core.is_some());

        let key_down = InputEvent::KeyDown {
            key: Key::Character("a".to_string()),
            modifiers: Modifiers::default(),
            repeat: false,
        };
        let core = key_down.to_core();
        assert!(core.is_some());
    }

    #[test]
    fn test_input_manager() {
        let mut manager = InputManager::new();

        // Process some events
        manager.process_event(InputEvent::MouseMove {
            x: 100.0,
            y: 200.0,
            delta_x: 10.0,
            delta_y: 20.0,
        });

        manager.process_event(InputEvent::MouseDown {
            button: MouseButton::Left,
            x: 100.0,
            y: 200.0,
        });

        manager.process_event(InputEvent::MouseUp {
            button: MouseButton::Left,
            x: 100.0,
            y: 200.0,
        });

        // Update to process gestures
        manager.update();

        // Check for click gesture
        assert!(manager.has_gestures());
        let gestures = manager.drain_gestures();
        assert!(!gestures.is_empty());
    }

    #[test]
    fn test_accessibility_support() {
        let mut acc = AccessibilitySupport::new();
        
        // Initially disabled
        assert!(!acc.is_screen_reader_enabled());
        
        // Enable and announce
        acc.set_screen_reader_enabled(true);
        acc.announce(AccessibilityAnnouncement::Text("Hello".to_string()));
        
        assert!(acc.next_announcement().is_some());
        assert!(acc.next_announcement().is_none());
    }

    #[test]
    fn test_focus_id_generation() {
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        
        assert_ne!(id1, id2);
        assert_ne!(id1.0, id2.0);
    }

    #[test]
    fn test_scroll_delta_conversion() {
        let pixels = ScrollDelta::Pixels { x: 100.0, y: 200.0 };
        let core = pixels.to_core();
        assert_eq!(core.x, 100.0);
        assert_eq!(core.y, 200.0);

        let lines = ScrollDelta::Lines { x: 1.0, y: 2.0 };
        let core = lines.to_core();
        assert_eq!(core.x, 20.0); // 1.0 * 20.0
        assert_eq!(core.y, 40.0); // 2.0 * 20.0
    }
}
