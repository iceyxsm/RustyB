//! Window Manager - Positions WebView and Control Panel side-by-side
//!
//! Creates a split-view effect by managing two windows:
//! - Left: WebView window (actual browser content)
//! - Right: Iced control panel window

use std::ptr::addr_of;
use tracing::info;

/// Window layout configuration
pub struct WindowLayout {
    /// Screen position X
    pub x: i32,
    /// Screen position Y
    pub y: i32,
    /// Total width for both windows
    pub total_width: u32,
    /// Height
    pub height: u32,
    /// Width of webview portion (0-1 ratio)
    pub webview_ratio: f32,
}

impl Default for WindowLayout {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            total_width: 1600,
            height: 900,
            webview_ratio: 0.7, // 70% for webview, 30% for control panel
        }
    }
}

impl WindowLayout {
    pub fn webview_width(&self) -> u32 {
        (self.total_width as f32 * self.webview_ratio) as u32
    }
    
    pub fn panel_width(&self) -> u32 {
        self.total_width - self.webview_width()
    }
    
    pub fn webview_x(&self) -> i32 {
        self.x
    }
    
    pub fn panel_x(&self) -> i32 {
        self.x + self.webview_width() as i32
    }
}

/// Window position and size
#[derive(Debug, Clone, Copy)]
pub struct WindowPos {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Global window layout
static mut WINDOW_LAYOUT: Option<WindowLayout> = None;

/// Initialize the global window manager
pub fn init_window_manager() {
    unsafe {
        WINDOW_LAYOUT = Some(WindowLayout::default());
    }
    info!("Split window manager initialized");
}

/// Get the window layout
pub fn get_layout() -> Option<&'static WindowLayout> {
    unsafe { addr_of!(WINDOW_LAYOUT).as_ref().and_then(|r| r.as_ref()) }
}

/// Calculate positions for split view
pub fn calculate_positions() -> Option<(WindowPos, WindowPos)> {
    get_layout().map(|layout| {
        let webview_pos = WindowPos {
            x: layout.webview_x(),
            y: layout.y,
            width: layout.webview_width(),
            height: layout.height,
        };
        
        let panel_pos = WindowPos {
            x: layout.panel_x(),
            y: layout.y,
            width: layout.panel_width(),
            height: layout.height,
        };
        
        (webview_pos, panel_pos)
    })
}

#[cfg(target_os = "windows")]
/// Position a window using Windows API
pub fn position_window(hwnd: isize, pos: &WindowPos) {
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_FRAMECHANGED, HWND_TOP};
    use windows::Win32::Foundation::HWND;
    
    unsafe {
        let _ = SetWindowPos(
            HWND(hwnd as *mut std::ffi::c_void),
            HWND_TOP,
            pos.x,
            pos.y,
            pos.width as i32,
            pos.height as i32,
            SWP_FRAMECHANGED,
        );
    }
}

#[cfg(target_os = "windows")]
/// Find and position webview window by title
pub fn find_and_position_webview() {
    use windows::Win32::Foundation::{BOOL, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, IsWindowVisible};
    
    unsafe extern "system" fn enum_windows_callback(hwnd: windows::Win32::Foundation::HWND, _lparam: LPARAM) -> BOOL {
        if !IsWindowVisible(hwnd).as_bool() {
            return true.into();
        }
        
        let mut title = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut title);
        if len > 0 {
            let title = String::from_utf16_lossy(&title[..len as usize]);
            
            // Look for webview window
            if title.contains("WebView") && title.contains("Rusty") {
                info!("Found WebView window: {}", title);
                
                if let Some((webview_pos, _)) = calculate_positions() {
                    position_window(hwnd.0 as isize, &webview_pos);
                }
                return false.into(); // Stop enumeration
            }
        }
        
        true.into()
    }
    
    unsafe {
        let _ = EnumWindows(Some(enum_windows_callback), LPARAM(0));
    }
}

#[cfg(target_os = "windows")]
/// Position the Iced control panel window
pub fn position_control_panel() {
    use windows::Win32::Foundation::{BOOL, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, IsWindowVisible};
    
    unsafe extern "system" fn enum_windows_callback(hwnd: windows::Win32::Foundation::HWND, _lparam: LPARAM) -> BOOL {
        if !IsWindowVisible(hwnd).as_bool() {
            return true.into();
        }
        
        let mut title = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut title);
        if len > 0 {
            let title = String::from_utf16_lossy(&title[..len as usize]);
            
            // Look for control panel window (should contain "Rusty Browser" but not "WebView")
            if title.contains("Rusty Browser") && !title.contains("WebView") {
                info!("Found Control Panel window: {}", title);
                
                if let Some((_, panel_pos)) = calculate_positions() {
                    position_window(hwnd.0 as isize, &panel_pos);
                }
                return false.into();
            }
        }
        
        true.into()
    }
    
    unsafe {
        let _ = EnumWindows(Some(enum_windows_callback), LPARAM(0));
    }
}

#[cfg(not(target_os = "windows"))]
pub fn find_and_position_webview() {
    // Not implemented on non-Windows platforms
}

#[cfg(not(target_os = "windows"))]
pub fn position_control_panel() {
    // Not implemented on non-Windows platforms
}
