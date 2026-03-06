//! WebView widget for Iced that displays Servo-rendered content
//!
//! This widget integrates Servo's rendering output into Iced's UI system.
//! It handles:
//! - Displaying GPU textures from Servo
//! - Input event forwarding (mouse, keyboard, touch)
//! - Resize handling
//! - Frame synchronization

use crate::servo_renderer::ServoRenderer;
use iced::{
    widget::image,
    Element, Length,
};
use std::sync::Arc;

/// Message type for WebView widget events
#[derive(Debug, Clone)]
pub enum WebViewMessage {
    /// Mouse moved
    MouseMoved(f32, f32),
    /// Mouse button pressed
    MousePressed(iced::mouse::Button),
    /// Mouse button released
    MouseReleased(iced::mouse::Button),
    /// Mouse wheel scrolled
    MouseScrolled(f32, f32),
    /// Keyboard key pressed
    KeyPressed(iced::keyboard::Key, iced::keyboard::Modifiers),
    /// Keyboard key released
    KeyReleased(iced::keyboard::Key),
    /// Text input
    TextInput(String),
    /// Frame ready from Servo
    FrameReady,
    /// Resize requested
    Resize(f32, f32),
}

/// WebView widget that displays Servo content
pub struct WebViewWidget<'a> {
    /// The Servo renderer
    renderer: Arc<ServoRenderer>,
    /// Current image handle for software rendering fallback
    image_handle: Option<image::Handle>,
    /// Size of the widget
    size: Length,
    /// Callback for input events
    on_input: Option<Box<dyn Fn(WebViewMessage) -> Message + 'a>>,
}

/// Internal message type for the widget
pub enum Message {
    WebViewMessage(WebViewMessage),
    NoOp,
}

impl<'a> WebViewWidget<'a> {
    /// Create a new WebView widget
    pub fn new(servo_renderer: Arc<ServoRenderer>) -> Self {
        Self {
            renderer: servo_renderer,
            image_handle: None,
            size: Length::Fill,
            on_input: None,
        }
    }
    
    /// Set the size of the widget
    pub fn size(mut self, size: Length) -> Self {
        self.size = size;
        self
    }
    
    /// Set the input event callback
    pub fn on_input<F>(mut self, callback: F) -> Self
    where
        F: Fn(WebViewMessage) -> Message + 'a,
    {
        self.on_input = Some(Box::new(callback));
        self
    }
}

/// Convert WebViewMessage to Servo InputEvent
pub fn to_servo_input_event(message: &WebViewMessage) -> Option<browser_core::webview::InputEvent> {
    use browser_core::webview::*;
    
    match message {
        WebViewMessage::MouseMoved(x, y) => {
            Some(InputEvent::MouseMove(MouseMoveEvent {
                point: Point2D::new(*x, *y),
            }))
        }
        WebViewMessage::MousePressed(button) => {
            let servo_button = match button {
                iced::mouse::Button::Left => MouseButton::Left,
                iced::mouse::Button::Right => MouseButton::Right,
                iced::mouse::Button::Middle => MouseButton::Middle,
                _ => MouseButton::Left,
            };
            Some(InputEvent::MouseButton(MouseButtonEvent {
                button: servo_button,
                action: MouseButtonAction::Down,
            }))
        }
        WebViewMessage::MouseReleased(button) => {
            let servo_button = match button {
                iced::mouse::Button::Left => MouseButton::Left,
                iced::mouse::Button::Right => MouseButton::Right,
                iced::mouse::Button::Middle => MouseButton::Middle,
                _ => MouseButton::Left,
            };
            Some(InputEvent::MouseButton(MouseButtonEvent {
                button: servo_button,
                action: MouseButtonAction::Up,
            }))
        }
        WebViewMessage::MouseScrolled(x, y) => {
            Some(InputEvent::Wheel(WheelEvent {
                delta: WheelDelta {
                    x: *x,
                    y: *y,
                },
                mode: WheelMode::DeltaPixel,
            }))
        }
        _ => None,
    }
}

/// Helper function to create the WebView widget as an Element
pub fn webview<'a, Message>(
    renderer: Arc<ServoRenderer>,
    on_input: impl Fn(WebViewMessage) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: 'a,
{
    // For now, return a placeholder element
    // In a full implementation, this would create a custom widget
    iced::widget::container(
        iced::widget::text("WebView - Servo integration in progress")
            .size(16)
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
