//! Comprehensive theme system for the Rusty Browser UI
//!
//! This module provides a production-grade dark/light theme system with:
//! - Custom `BrowserTheme` struct implementing Iced 0.14 theme traits
//! - Support for Dark, Light, and Auto (system preference) modes
//! - Browser-specific color palette (toolbar, address bar, tabs, etc.)
//! - Runtime theme switching with persistence
//! - System theme detection and change listening
//! - High contrast accessibility support
//!
//! # Example
//!
//! ```rust
//! use browser_ui::theme::{BrowserTheme, ThemeMode};
//!
//! // Create a new theme with auto mode (follows system preference)
//! let theme = BrowserTheme::new(ThemeMode::Auto);
//!
//! // Switch to dark mode
//! let dark_theme = theme.with_mode(ThemeMode::Dark);
//!
//! // Get effective theme (resolves Auto to actual Dark/Light)
//! let effective = dark_theme.effective_theme();
//! ```

use iced::{
    color, widget::button, Border, Color, Theme,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Theme mode selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ThemeMode {
    /// Always use dark theme
    Dark,
    /// Always use light theme
    Light,
    /// Follow system preference (default)
    Auto,
    /// High contrast mode for accessibility
    HighContrast,
}

impl Default for ThemeMode {
    fn default() -> Self {
        ThemeMode::Auto
    }
}

impl ThemeMode {
    /// Get a human-readable name for the theme mode
    pub fn display_name(&self) -> &'static str {
        match self {
            ThemeMode::Dark => "Dark",
            ThemeMode::Light => "Light",
            ThemeMode::Auto => "Auto",
            ThemeMode::HighContrast => "High Contrast",
        }
    }

    /// Get all available theme modes
    pub fn all_modes() -> &'static [ThemeMode] {
        &[ThemeMode::Dark, ThemeMode::Light, ThemeMode::Auto, ThemeMode::HighContrast]
    }

    /// Detect system theme preference
    #[cfg(target_os = "windows")]
    pub fn detect_system() -> Self {
        // On Windows, check registry for system theme
        // For now, default to dark as most modern systems prefer it
        ThemeMode::Dark
    }

    #[cfg(target_os = "macos")]
    pub fn detect_system() -> Self {
        // On macOS, we would use NSApp.appearance or similar
        // For now, default to auto which will be handled at runtime
        ThemeMode::Auto
    }

    #[cfg(target_os = "linux")]
    pub fn detect_system() -> Self {
        // On Linux, check GTK theme or XDG settings
        // For now, default to auto
        ThemeMode::Auto
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn detect_system() -> Self {
        ThemeMode::Auto
    }
}

/// Browser-specific color palette
///
/// This struct defines all colors used in the browser UI, organized by component.
/// Each color is defined as an `iced::Color` for maximum flexibility.
#[derive(Debug, Clone, PartialEq)]
pub struct BrowserColors {
    // === Toolbar Colors ===
    /// Toolbar background color
    pub toolbar_background: Color,
    /// Toolbar border color
    pub toolbar_border: Color,
    /// Toolbar button background (normal state)
    pub toolbar_button_background: Color,
    /// Toolbar button background (hover state)
    pub toolbar_button_hover: Color,
    /// Toolbar button background (active/pressed state)
    pub toolbar_button_active: Color,
    /// Toolbar button icon color
    pub toolbar_button_icon: Color,
    /// Toolbar button icon color when disabled
    pub toolbar_button_icon_disabled: Color,

    // === Address Bar Colors ===
    /// Address bar background color
    pub address_bar_background: Color,
    /// Address bar border color (normal state)
    pub address_bar_border: Color,
    /// Address bar border color (focused state)
    pub address_bar_border_focused: Color,
    /// Address bar text color
    pub address_bar_text: Color,
    /// Address bar placeholder text color
    pub address_bar_placeholder: Color,
    /// Address bar selection highlight color
    pub address_bar_selection: Color,
    /// Secure connection indicator color (HTTPS)
    pub address_bar_secure: Color,
    /// Insecure connection indicator color (HTTP)
    pub address_bar_insecure: Color,

    // === Tab Colors ===
    /// Tab bar background color
    pub tab_bar_background: Color,
    /// Active tab background color
    pub tab_active_background: Color,
    /// Active tab text color
    pub tab_active_text: Color,
    /// Inactive tab background color
    pub tab_inactive_background: Color,
    /// Inactive tab text color
    pub tab_inactive_text: Color,
    /// Tab hover background color
    pub tab_hover_background: Color,
    /// Tab close button color
    pub tab_close_button: Color,
    /// Tab close button hover color
    pub tab_close_button_hover: Color,
    /// New tab button background
    pub tab_new_button_background: Color,
    /// New tab button hover background
    pub tab_new_button_hover: Color,

    // === Content Area Colors ===
    /// Main content area background
    pub content_background: Color,
    /// Content area border color
    pub content_border: Color,

    // === Loading & Progress Colors ===
    /// Loading indicator color
    pub loading_indicator: Color,
    /// Progress bar background
    pub progress_bar_background: Color,
    /// Progress bar fill color
    pub progress_bar_fill: Color,

    // === Error State Colors ===
    /// Error background color
    pub error_background: Color,
    /// Error text color
    pub error_text: Color,
    /// Error border color
    pub error_border: Color,
    /// Warning background color
    pub warning_background: Color,
    /// Warning text color
    pub warning_text: Color,

    // === Status Bar Colors ===
    /// Status bar background
    pub status_bar_background: Color,
    /// Status bar text color
    pub status_bar_text: Color,

    // === Menu & Dropdown Colors ===
    /// Menu background color
    pub menu_background: Color,
    /// Menu border color
    pub menu_border: Color,
    /// Menu item hover background
    pub menu_item_hover: Color,
    /// Menu item text color
    pub menu_item_text: Color,
    /// Menu separator color
    pub menu_separator: Color,

    // === Scrollbar Colors ===
    /// Scrollbar background
    pub scrollbar_background: Color,
    /// Scrollbar thumb color
    pub scrollbar_thumb: Color,
    /// Scrollbar thumb hover color
    pub scrollbar_thumb_hover: Color,

    // === General UI Colors ===
    /// Primary accent color (buttons, links, etc.)
    pub accent: Color,
    /// Primary accent color (hover state)
    pub accent_hover: Color,
    /// Primary text color
    pub text_primary: Color,
    /// Secondary text color
    pub text_secondary: Color,
    /// Disabled text color
    pub text_disabled: Color,
    /// Divider/separator color
    pub divider: Color,
    /// Tooltip background
    pub tooltip_background: Color,
    /// Tooltip text color
    pub tooltip_text: Color,
}

impl BrowserColors {
    /// Create a dark theme color palette
    pub fn dark() -> Self {
        Self {
            // Toolbar - Dark gray with subtle border
            toolbar_background: color!(0x2d, 0x2d, 0x2d),
            toolbar_border: color!(0x3d, 0x3d, 0x3d),
            toolbar_button_background: Color::TRANSPARENT,
            toolbar_button_hover: color!(0x3d, 0x3d, 0x3d),
            toolbar_button_active: color!(0x4d, 0x4d, 0x4d),
            toolbar_button_icon: color!(0xcc, 0xcc, 0xcc),
            toolbar_button_icon_disabled: color!(0x66, 0x66, 0x66),

            // Address Bar - Slightly lighter than toolbar
            address_bar_background: color!(0x1e, 0x1e, 0x1e),
            address_bar_border: color!(0x4d, 0x4d, 0x4d),
            address_bar_border_focused: color!(0x4d, 0x90, 0xfe),
            address_bar_text: color!(0xe0, 0xe0, 0xe0),
            address_bar_placeholder: color!(0x80, 0x80, 0x80),
            address_bar_selection: color!(0x26, 0x4f, 0x78),
            address_bar_secure: color!(0x4c, 0xaf, 0x50),
            address_bar_insecure: color!(0xff, 0x98, 0x00),

            // Tabs - Distinct from toolbar
            tab_bar_background: color!(0x25, 0x25, 0x25),
            tab_active_background: color!(0x2d, 0x2d, 0x2d),
            tab_active_text: color!(0xe0, 0xe0, 0xe0),
            tab_inactive_background: color!(0x1a, 0x1a, 0x1a),
            tab_inactive_text: color!(0x99, 0x99, 0x99),
            tab_hover_background: color!(0x30, 0x30, 0x30),
            tab_close_button: color!(0x99, 0x99, 0x99),
            tab_close_button_hover: color!(0xff, 0x44, 0x44),
            tab_new_button_background: Color::TRANSPARENT,
            tab_new_button_hover: color!(0x3d, 0x3d, 0x3d),

            // Content Area
            content_background: color!(0x1e, 0x1e, 0x1e),
            content_border: color!(0x3d, 0x3d, 0x3d),

            // Loading & Progress
            loading_indicator: color!(0x4d, 0x90, 0xfe),
            progress_bar_background: color!(0x3d, 0x3d, 0x3d),
            progress_bar_fill: color!(0x4d, 0x90, 0xfe),

            // Error States
            error_background: color!(0x3d, 0x1a, 0x1a),
            error_text: color!(0xff, 0x6b, 0x6b),
            error_border: color!(0xff, 0x44, 0x44),
            warning_background: color!(0x3d, 0x2d, 0x1a),
            warning_text: color!(0xff, 0xcc, 0x00),

            // Status Bar
            status_bar_background: color!(0x2d, 0x2d, 0x2d),
            status_bar_text: color!(0x99, 0x99, 0x99),

            // Menu & Dropdown
            menu_background: color!(0x2d, 0x2d, 0x2d),
            menu_border: color!(0x4d, 0x4d, 0x4d),
            menu_item_hover: color!(0x3d, 0x3d, 0x3d),
            menu_item_text: color!(0xe0, 0xe0, 0xe0),
            menu_separator: color!(0x4d, 0x4d, 0x4d),

            // Scrollbar
            scrollbar_background: Color::TRANSPARENT,
            scrollbar_thumb: color!(0x66, 0x66, 0x66),
            scrollbar_thumb_hover: color!(0x80, 0x80, 0x80),

            // General UI
            accent: color!(0x4d, 0x90, 0xfe),
            accent_hover: color!(0x6a, 0xa8, 0xff),
            text_primary: color!(0xe0, 0xe0, 0xe0),
            text_secondary: color!(0x99, 0x99, 0x99),
            text_disabled: color!(0x66, 0x66, 0x66),
            divider: color!(0x3d, 0x3d, 0x3d),
            tooltip_background: color!(0x33, 0x33, 0x33),
            tooltip_text: color!(0xe0, 0xe0, 0xe0),
        }
    }

    /// Create a light theme color palette
    pub fn light() -> Self {
        Self {
            // Toolbar - Light gray
            toolbar_background: color!(0xf5, 0xf5, 0xf5),
            toolbar_border: color!(0xd0, 0xd0, 0xd0),
            toolbar_button_background: Color::TRANSPARENT,
            toolbar_button_hover: color!(0xe0, 0xe0, 0xe0),
            toolbar_button_active: color!(0xd0, 0xd0, 0xd0),
            toolbar_button_icon: color!(0x33, 0x33, 0x33),
            toolbar_button_icon_disabled: color!(0x99, 0x99, 0x99),

            // Address Bar - White with subtle border
            address_bar_background: color!(0xff, 0xff, 0xff),
            address_bar_border: color!(0xc0, 0xc0, 0xc0),
            address_bar_border_focused: color!(0x1a, 0x73, 0xe8),
            address_bar_text: color!(0x20, 0x20, 0x20),
            address_bar_placeholder: color!(0x80, 0x80, 0x80),
            address_bar_selection: color!(0xb8, 0xd4, 0xff),
            address_bar_secure: color!(0x34, 0xa8, 0x53),
            address_bar_insecure: color!(0xf2, 0x99, 0x00),

            // Tabs
            tab_bar_background: color!(0xe8, 0xe8, 0xe8),
            tab_active_background: color!(0xf5, 0xf5, 0xf5),
            tab_active_text: color!(0x20, 0x20, 0x20),
            tab_inactive_background: color!(0xd8, 0xd8, 0xd8),
            tab_inactive_text: color!(0x66, 0x66, 0x66),
            tab_hover_background: color!(0xe0, 0xe0, 0xe0),
            tab_close_button: color!(0x66, 0x66, 0x66),
            tab_close_button_hover: color!(0xe8, 0x1c, 0x1c),
            tab_new_button_background: Color::TRANSPARENT,
            tab_new_button_hover: color!(0xe0, 0xe0, 0xe0),

            // Content Area
            content_background: color!(0xff, 0xff, 0xff),
            content_border: color!(0xd0, 0xd0, 0xd0),

            // Loading & Progress
            loading_indicator: color!(0x1a, 0x73, 0xe8),
            progress_bar_background: color!(0xe0, 0xe0, 0xe0),
            progress_bar_fill: color!(0x1a, 0x73, 0xe8),

            // Error States
            error_background: color!(0xff, 0xeb, 0xee),
            error_text: color!(0xd3, 0x2f, 0x2f),
            error_border: color!(0xe8, 0x1c, 0x1c),
            warning_background: color!(0xff, 0xf3, 0xe0),
            warning_text: color!(0xf5, 0x7c, 0x00),

            // Status Bar
            status_bar_background: color!(0xf5, 0xf5, 0xf5),
            status_bar_text: color!(0x66, 0x66, 0x66),

            // Menu & Dropdown
            menu_background: color!(0xff, 0xff, 0xff),
            menu_border: color!(0xd0, 0xd0, 0xd0),
            menu_item_hover: color!(0xe8, 0xe8, 0xe8),
            menu_item_text: color!(0x20, 0x20, 0x20),
            menu_separator: color!(0xe0, 0xe0, 0xe0),

            // Scrollbar
            scrollbar_background: Color::TRANSPARENT,
            scrollbar_thumb: color!(0xc0, 0xc0, 0xc0),
            scrollbar_thumb_hover: color!(0xa0, 0xa0, 0xa0),

            // General UI
            accent: color!(0x1a, 0x73, 0xe8),
            accent_hover: color!(0x15, 0x5b, 0xb5),
            text_primary: color!(0x20, 0x20, 0x20),
            text_secondary: color!(0x66, 0x66, 0x66),
            text_disabled: color!(0x99, 0x99, 0x99),
            divider: color!(0xe0, 0xe0, 0xe0),
            tooltip_background: color!(0x33, 0x33, 0x33),
            tooltip_text: color!(0xff, 0xff, 0xff),
        }
    }

    /// Create a high contrast theme for accessibility
    pub fn high_contrast() -> Self {
        let mut colors = Self::dark();
        
        // Override with high contrast values
        colors.toolbar_background = Color::BLACK;
        colors.toolbar_border = Color::WHITE;
        colors.toolbar_button_icon = Color::WHITE;
        colors.toolbar_button_icon_disabled = color!(0x80, 0x80, 0x80);
        
        colors.address_bar_background = Color::BLACK;
        colors.address_bar_border = Color::WHITE;
        colors.address_bar_border_focused = color!(0xff, 0xff, 0x00); // Yellow focus
        colors.address_bar_text = Color::WHITE;
        colors.address_bar_placeholder = color!(0xc0, 0xc0, 0xc0);
        
        colors.tab_active_background = Color::BLACK;
        colors.tab_active_text = Color::WHITE;
        colors.tab_inactive_background = color!(0x40, 0x40, 0x40);
        colors.tab_inactive_text = color!(0xc0, 0xc0, 0xc0);
        
        colors.content_background = Color::BLACK;
        colors.content_border = Color::WHITE;
        
        colors.accent = color!(0xff, 0xff, 0x00); // Yellow accent
        colors.accent_hover = color!(0xff, 0xff, 0x66);
        colors.text_primary = Color::WHITE;
        colors.text_secondary = color!(0xc0, 0xc0, 0xc0);
        colors.text_disabled = color!(0x80, 0x80, 0x80);
        
        colors
    }
}

/// Custom browser theme implementing Iced's theme traits
///
/// This struct wraps a `ThemeMode` and `BrowserColors` to provide
/// a complete theme system for the browser UI.
#[derive(Debug, Clone)]
pub struct BrowserTheme {
    /// The current theme mode
    mode: ThemeMode,
    /// The resolved effective theme (Dark or Light)
    effective_theme: Theme,
    /// Browser-specific colors
    colors: BrowserColors,
    /// Whether to use high contrast mode
    high_contrast: bool,
}

impl Default for BrowserTheme {
    fn default() -> Self {
        Self::new(ThemeMode::Auto)
    }
}

impl BrowserTheme {
    /// Create a new browser theme with the specified mode
    ///
    /// # Arguments
    ///
    /// * `mode` - The theme mode (Dark, Light, Auto, or HighContrast)
    ///
    /// # Example
    ///
    /// ```rust
    /// use browser_ui::theme::{BrowserTheme, ThemeMode};
    ///
    /// let theme = BrowserTheme::new(ThemeMode::Dark);
    /// ```
    pub fn new(mode: ThemeMode) -> Self {
        let effective_theme = Self::resolve_effective_theme(mode);
        let colors = Self::colors_for_mode(mode);
        let high_contrast = mode == ThemeMode::HighContrast;

        info!("Created browser theme with mode: {:?}", mode);

        Self {
            mode,
            effective_theme,
            colors,
            high_contrast,
        }
    }

    /// Create a new theme with auto-detection of system preference
    pub fn auto() -> Self {
        Self::new(ThemeMode::detect_system())
    }

    /// Get the current theme mode
    pub fn mode(&self) -> ThemeMode {
        self.mode
    }

    /// Get the effective Iced theme (Dark or Light)
    pub fn effective_theme(&self) -> &Theme {
        &self.effective_theme
    }

    /// Get the browser-specific colors
    pub fn colors(&self) -> &BrowserColors {
        &self.colors
    }

    /// Check if high contrast mode is enabled
    pub fn is_high_contrast(&self) -> bool {
        self.high_contrast
    }

    /// Create a new theme with a different mode
    ///
    /// This is useful for switching themes at runtime.
    ///
    /// # Example
    ///
    /// ```rust
    /// use browser_ui::theme::{BrowserTheme, ThemeMode};
    ///
    /// let dark_theme = BrowserTheme::new(ThemeMode::Dark);
    /// let light_theme = dark_theme.with_mode(ThemeMode::Light);
    /// ```
    pub fn with_mode(&self, mode: ThemeMode) -> Self {
        Self::new(mode)
    }

    /// Toggle between dark and light modes
    pub fn toggle(&self) -> Self {
        let new_mode = match self.mode {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Auto => {
                // Toggle based on current effective theme
                match self.effective_theme {
                    Theme::Dark => ThemeMode::Light,
                    _ => ThemeMode::Dark,
                }
            }
            ThemeMode::HighContrast => ThemeMode::Auto,
        };
        self.with_mode(new_mode)
    }

    /// Resolve a theme mode to an effective Iced theme
    fn resolve_effective_theme(mode: ThemeMode) -> Theme {
        match mode {
            ThemeMode::Dark => Theme::Dark,
            ThemeMode::Light => Theme::Light,
            ThemeMode::HighContrast => Theme::Dark, // High contrast uses dark base
            ThemeMode::Auto => {
                // In a real implementation, this would detect system preference
                // For now, default to dark as it's more common for browsers
                Theme::Dark
            }
        }
    }

    /// Get colors for a specific theme mode
    fn colors_for_mode(mode: ThemeMode) -> BrowserColors {
        match mode {
            ThemeMode::Dark => BrowserColors::dark(),
            ThemeMode::Light => BrowserColors::light(),
            ThemeMode::HighContrast => BrowserColors::high_contrast(),
            ThemeMode::Auto => {
                // Default to dark for auto until system detection is implemented
                BrowserColors::dark()
            }
        }
    }

    /// Update the theme based on system preference changes
    ///
    /// This should be called when the system theme changes.
    /// Returns true if the theme was updated.
    pub fn update_from_system(&mut self) -> bool {
        if self.mode != ThemeMode::Auto {
            return false;
        }

        let detected = ThemeMode::detect_system();
        let new_theme = Self::resolve_effective_theme(detected);
        let new_colors = Self::colors_for_mode(detected);

        if self.effective_theme != new_theme {
            debug!("System theme changed, updating effective theme");
            self.effective_theme = new_theme;
            self.colors = new_colors;
            true
        } else {
            false
        }
    }

    // === Style Helper Methods ===

    /// Get the style for a toolbar button
    pub fn toolbar_button_style(&self) -> button::Style {
        button::Style {
            background: Some(self.colors.toolbar_button_background.into()),
            text_color: self.colors.toolbar_button_icon,
            border: Border::default(),
            shadow: Default::default(),
            snap: true,
        }
    }

    /// Get the style for a toolbar button (hover state)
    pub fn toolbar_button_style_hover(&self) -> button::Style {
        button::Style {
            background: Some(self.colors.toolbar_button_hover.into()),
            text_color: self.colors.toolbar_button_icon,
            border: Border::default(),
            shadow: Default::default(),
            snap: true,
        }
    }

    /// Get the style for an active tab
    pub fn tab_active_style(&self) -> button::Style {
        button::Style {
            background: Some(self.colors.tab_active_background.into()),
            text_color: self.colors.tab_active_text,
            border: Border::default(),
            shadow: Default::default(),
            snap: true,
        }
    }

    /// Get the style for an inactive tab
    pub fn tab_inactive_style(&self) -> button::Style {
        button::Style {
            background: Some(self.colors.tab_inactive_background.into()),
            text_color: self.colors.tab_inactive_text,
            border: Border::default(),
            shadow: Default::default(),
            snap: true,
        }
    }

    /// Get the background color for the toolbar
    pub fn toolbar_background(&self) -> Color {
        self.colors.toolbar_background
    }

    /// Get the background color for the address bar
    pub fn address_bar_background(&self) -> Color {
        self.colors.address_bar_background
    }

    /// Get the background color for the tab bar
    pub fn tab_bar_background(&self) -> Color {
        self.colors.tab_bar_background
    }

    /// Get the background color for the content area
    pub fn content_background(&self) -> Color {
        self.colors.content_background
    }

    /// Get the accent color
    pub fn accent_color(&self) -> Color {
        self.colors.accent
    }

    /// Get the loading indicator color
    pub fn loading_color(&self) -> Color {
        self.colors.loading_indicator
    }

    /// Get the error color
    pub fn error_color(&self) -> Color {
        self.colors.error_text
    }

    /// Get the warning color
    pub fn warning_color(&self) -> Color {
        self.colors.warning_text
    }

    /// Get the surface color (for cards, panels)
    pub fn surface_color(&self) -> Color {
        self.colors.toolbar_background
    }

    /// Get the border color
    pub fn border_color(&self) -> Color {
        self.colors.divider
    }

    /// Get the success color
    pub fn success_color(&self) -> Color {
        self.colors.address_bar_secure
    }

    /// Get the info color
    pub fn info_color(&self) -> Color {
        self.colors.accent
    }

    /// Get the background color
    pub fn background_color(&self) -> Color {
        self.colors.content_background
    }
}

/// Theme persistence for saving/loading user preferences
pub mod persistence {
    use super::{BrowserTheme, ThemeMode};
    use std::path::PathBuf;
    use tracing::{debug, error, info, warn};

    /// Configuration file name
    const CONFIG_FILE: &str = "theme_config.json";

    /// Get the config directory path
    fn config_dir() -> Option<PathBuf> {
        // Use dirs crate to get config directory
        dirs::config_dir().map(|dir| dir.join("rusty-browser"))
    }

    /// Get the full config file path
    fn config_file_path() -> Option<PathBuf> {
        config_dir().map(|dir| dir.join(CONFIG_FILE))
    }

    /// Ensure the config directory exists
    fn ensure_config_dir() -> anyhow::Result<PathBuf> {
        let dir = config_dir().ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Theme configuration for persistence
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct ThemeConfig {
        mode: ThemeMode,
    }

    /// Save the current theme mode to disk
    ///
    /// # Errors
    ///
    /// Returns an error if the config directory cannot be created or the file cannot be written.
    pub fn save_theme(theme: &BrowserTheme) -> anyhow::Result<()> {
        let config = ThemeConfig { mode: theme.mode() };
        let config_dir = ensure_config_dir()?;
        let config_path = config_dir.join(CONFIG_FILE);

        let json = serde_json::to_string_pretty(&config)?;
        std::fs::write(&config_path, json)?;

        info!("Theme saved to: {:?}", config_path);
        Ok(())
    }

    /// Load the theme from disk
    ///
    /// Returns `None` if no saved theme exists or if loading fails.
    pub fn load_theme() -> Option<BrowserTheme> {
        let config_path = config_file_path()?;

        if !config_path.exists() {
            debug!("No saved theme found at: {:?}", config_path);
            return None;
        }

        match std::fs::read_to_string(&config_path) {
            Ok(json) => match serde_json::from_str::<ThemeConfig>(&json) {
                Ok(config) => {
                    info!("Theme loaded from: {:?}", config_path);
                    Some(BrowserTheme::new(config.mode))
                }
                Err(e) => {
                    error!("Failed to parse theme config: {}", e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to read theme config: {}", e);
                None
            }
        }
    }

    /// Load theme or create default
    pub fn load_or_default() -> BrowserTheme {
        load_theme().unwrap_or_default()
    }

    /// Clear saved theme preference
    pub fn clear_saved_theme() -> anyhow::Result<()> {
        if let Some(config_path) = config_file_path() {
            if config_path.exists() {
                std::fs::remove_file(&config_path)?;
                info!("Theme preference cleared");
            }
        }
        Ok(())
    }
}

/// System theme detection and monitoring
/// 
/// Note: Enable the `system-theme-detection` feature to use this module.
/// Currently requires platform-specific implementations.
#[cfg(feature = "system-theme-detection")]
pub mod system_detection {
    use super::ThemeMode;
    use std::sync::Arc;
    use tokio::sync::watch;
    use tracing::{debug, error};

    /// System theme watcher that monitors OS theme changes
    pub struct SystemThemeWatcher {
        #[allow(dead_code)]
        rx: watch::Receiver<ThemeMode>,
        _handle: Arc<tokio::task::JoinHandle<()>>,
    }

    impl SystemThemeWatcher {
        /// Create a new system theme watcher
        ///
        /// This spawns a background task that monitors the system theme
        /// and sends updates through the returned receiver.
        pub fn new() -> (Self, watch::Receiver<ThemeMode>) {
            let (tx, rx) = watch::channel(ThemeMode::detect_system());
            let rx_monitor = rx.clone();
            let rx_return = rx.clone();

            let handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
                
                loop {
                    interval.tick().await;
                    
                    let current = ThemeMode::detect_system();
                    if current != *rx_monitor.borrow() {
                        debug!("System theme changed to: {:?}", current);
                        if let Err(e) = tx.send(current) {
                            error!("Failed to send theme update: {}", e);
                            break;
                        }
                    }
                }
            });

            (
                Self {
                    rx,
                    _handle: Arc::new(handle),
                },
                rx_return,
            )
        }
    }
}

// === Iced Trait Implementations ===

// Note: In Iced 0.14, the application styling is handled through the Theme type.
// The BrowserTheme integrates with Iced's Theme system through the Into/From traits.

/// Get the application background color for the current theme
pub fn application_background_color(theme: &BrowserTheme) -> Color {
    theme.colors.content_background
}

/// Get the application text color for the current theme
pub fn application_text_color(theme: &BrowserTheme) -> Color {
    theme.colors.text_primary
}

impl From<BrowserTheme> for Theme {
    fn from(theme: BrowserTheme) -> Self {
        theme.effective_theme
    }
}

impl From<&BrowserTheme> for Theme {
    fn from(theme: &BrowserTheme) -> Self {
        theme.effective_theme.clone()
    }
}

// === Widget Style Implementations ===

/// Button style variants for the browser theme
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    /// Standard toolbar button
    Toolbar,
    /// Navigation button (back/forward)
    Navigation,
    /// Tab button (active)
    TabActive,
    /// Tab button (inactive)
    TabInactive,
    /// Action button (primary)
    Primary,
    /// Danger button (close, delete)
    Danger,
    /// Secondary button
    Secondary,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        ButtonStyle::Toolbar
    }
}

/// Implement button styling for the browser theme
pub fn button_style(theme: &BrowserTheme, style: ButtonStyle) -> button::Style {
    match style {
        ButtonStyle::Toolbar => button::Style {
            background: Some(theme.colors.toolbar_button_background.into()),
            text_color: theme.colors.toolbar_button_icon,
            border: Border::default(),
            shadow: Default::default(),
            snap: true,
        },
        ButtonStyle::Navigation => button::Style {
            background: Some(theme.colors.toolbar_button_background.into()),
            text_color: theme.colors.toolbar_button_icon,
            border: Border::default(),
            shadow: Default::default(),
            snap: true,
        },
        ButtonStyle::TabActive => button::Style {
            background: Some(theme.colors.tab_active_background.into()),
            text_color: theme.colors.tab_active_text,
            border: Border {
                color: theme.colors.toolbar_border,
                width: 1.0,
                radius: 4.0.into(),
            },
            shadow: Default::default(),
            snap: true,
        },
        ButtonStyle::TabInactive => button::Style {
            background: Some(theme.colors.tab_inactive_background.into()),
            text_color: theme.colors.tab_inactive_text,
            border: Border {
                color: theme.colors.toolbar_border,
                width: 1.0,
                radius: 4.0.into(),
            },
            shadow: Default::default(),
            snap: true,
        },
        ButtonStyle::Primary => button::Style {
            background: Some(theme.colors.accent.into()),
            text_color: Color::WHITE,
            border: Border::default(),
            shadow: Default::default(),
            snap: true,
        },
        ButtonStyle::Danger => button::Style {
            background: Some(theme.colors.error_background.into()),
            text_color: theme.colors.error_text,
            border: Border {
                color: theme.colors.error_border,
                width: 1.0,
                radius: 4.0.into(),
            },
            shadow: Default::default(),
            snap: true,
        },
        ButtonStyle::Secondary => button::Style {
            background: Some(theme.colors.toolbar_button_hover.into()),
            text_color: theme.colors.text_primary,
            border: Border {
                color: theme.colors.divider,
                width: 1.0,
                radius: 4.0.into(),
            },
            shadow: Default::default(),
            snap: true,
        },
    }
}

/// Container style variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerStyle {
    /// Toolbar container
    Toolbar,
    /// Address bar container
    AddressBar,
    /// Tab bar container
    TabBar,
    /// Content area container
    Content,
    /// Menu/dropdown container
    Menu,
    /// Tooltip container
    Tooltip,
    /// Error state container
    Error,
    /// Warning state container
    Warning,
    /// Card/container for tools
    Card,
}

impl Default for ContainerStyle {
    fn default() -> Self {
        ContainerStyle::Content
    }
}

/// Get container background color
pub fn container_background(theme: &BrowserTheme, style: ContainerStyle) -> Color {
    match style {
        ContainerStyle::Toolbar => theme.colors.toolbar_background,
        ContainerStyle::AddressBar => theme.colors.address_bar_background,
        ContainerStyle::TabBar => theme.colors.tab_bar_background,
        ContainerStyle::Content => theme.colors.content_background,
        ContainerStyle::Menu => theme.colors.menu_background,
        ContainerStyle::Tooltip => theme.colors.tooltip_background,
        ContainerStyle::Error => theme.colors.error_background,
        ContainerStyle::Warning => theme.colors.warning_background,
        ContainerStyle::Card => theme.colors.toolbar_button_hover,
    }
}

/// Text style variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextStyle {
    /// Primary text
    Primary,
    /// Secondary/muted text
    Secondary,
    /// Disabled text
    Disabled,
    /// Accent text (links, etc.)
    Accent,
    /// Error text
    Error,
    /// Warning text
    Warning,
    /// Toolbar text
    Toolbar,
}

impl Default for TextStyle {
    fn default() -> Self {
        TextStyle::Primary
    }
}

/// Get text color
pub fn text_color(theme: &BrowserTheme, style: TextStyle) -> Color {
    match style {
        TextStyle::Primary => theme.colors.text_primary,
        TextStyle::Secondary => theme.colors.text_secondary,
        TextStyle::Disabled => theme.colors.text_disabled,
        TextStyle::Accent => theme.colors.accent,
        TextStyle::Error => theme.colors.error_text,
        TextStyle::Warning => theme.colors.warning_text,
        TextStyle::Toolbar => theme.colors.toolbar_button_icon,
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_mode_display_names() {
        assert_eq!(ThemeMode::Dark.display_name(), "Dark");
        assert_eq!(ThemeMode::Light.display_name(), "Light");
        assert_eq!(ThemeMode::Auto.display_name(), "Auto");
        assert_eq!(ThemeMode::HighContrast.display_name(), "High Contrast");
    }

    #[test]
    fn test_theme_creation() {
        let dark_theme = BrowserTheme::new(ThemeMode::Dark);
        assert_eq!(dark_theme.mode(), ThemeMode::Dark);
        assert!(matches!(dark_theme.effective_theme(), Theme::Dark));

        let light_theme = BrowserTheme::new(ThemeMode::Light);
        assert_eq!(light_theme.mode(), ThemeMode::Light);
        assert!(matches!(light_theme.effective_theme(), Theme::Light));
    }

    #[test]
    fn test_theme_toggle() {
        let dark_theme = BrowserTheme::new(ThemeMode::Dark);
        let light_theme = dark_theme.toggle();
        assert_eq!(light_theme.mode(), ThemeMode::Light);

        let toggled_back = light_theme.toggle();
        assert_eq!(toggled_back.mode(), ThemeMode::Dark);
    }

    #[test]
    fn test_with_mode() {
        let theme = BrowserTheme::new(ThemeMode::Dark);
        let light_theme = theme.with_mode(ThemeMode::Light);
        
        assert_eq!(theme.mode(), ThemeMode::Dark);
        assert_eq!(light_theme.mode(), ThemeMode::Light);
    }

    #[test]
    fn test_color_palette_dark() {
        let colors = BrowserColors::dark();
        
        // Verify dark theme has appropriate dark colors
        assert!(colors.toolbar_background.r < 0.5);
        assert!(colors.toolbar_background.g < 0.5);
        assert!(colors.toolbar_background.b < 0.5);
        
        // Text should be light
        assert!(colors.text_primary.r > 0.5);
        assert!(colors.text_primary.g > 0.5);
        assert!(colors.text_primary.b > 0.5);
    }

    #[test]
    fn test_color_palette_light() {
        let colors = BrowserColors::light();
        
        // Verify light theme has appropriate light colors
        assert!(colors.toolbar_background.r > 0.8);
        assert!(colors.toolbar_background.g > 0.8);
        assert!(colors.toolbar_background.b > 0.8);
        
        // Text should be dark
        assert!(colors.text_primary.r < 0.5);
        assert!(colors.text_primary.g < 0.5);
        assert!(colors.text_primary.b < 0.5);
    }

    #[test]
    fn test_high_contrast_theme() {
        let theme = BrowserTheme::new(ThemeMode::HighContrast);
        
        assert!(theme.is_high_contrast());
        assert_eq!(theme.mode(), ThemeMode::HighContrast);
    }

    #[test]
    fn test_button_styles() {
        let theme = BrowserTheme::new(ThemeMode::Dark);
        
        let toolbar_style = button_style(&theme, ButtonStyle::Toolbar);
        assert_eq!(toolbar_style.text_color, theme.colors.toolbar_button_icon);
        
        let primary_style = button_style(&theme, ButtonStyle::Primary);
        assert_eq!(primary_style.text_color, Color::WHITE);
    }

    #[test]
    fn test_text_styles() {
        let theme = BrowserTheme::new(ThemeMode::Dark);
        
        assert_eq!(text_color(&theme, TextStyle::Primary), theme.colors.text_primary);
        assert_eq!(text_color(&theme, TextStyle::Error), theme.colors.error_text);
        assert_eq!(text_color(&theme, TextStyle::Accent), theme.colors.accent);
    }

    #[test]
    fn test_container_backgrounds() {
        let theme = BrowserTheme::new(ThemeMode::Dark);
        
        assert_eq!(container_background(&theme, ContainerStyle::Toolbar), theme.colors.toolbar_background);
        assert_eq!(container_background(&theme, ContainerStyle::Content), theme.colors.content_background);
        assert_eq!(container_background(&theme, ContainerStyle::Error), theme.colors.error_background);
    }

    #[test]
    fn test_theme_into_iced_theme() {
        let dark_theme = BrowserTheme::new(ThemeMode::Dark);
        let iced_theme: Theme = dark_theme.into();
        assert!(matches!(iced_theme, Theme::Dark));

        let light_theme = BrowserTheme::new(ThemeMode::Light);
        let iced_theme: Theme = light_theme.into();
        assert!(matches!(iced_theme, Theme::Light));
    }

    #[test]
    fn test_default_theme() {
        let theme = BrowserTheme::default();
        assert_eq!(theme.mode(), ThemeMode::Auto);
    }
}
