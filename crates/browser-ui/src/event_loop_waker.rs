//! Event loop waker for Servo integration with Iced
//!
//! Servo runs on its own threads but needs to communicate with the main event loop
//! when it has work to do (new frames, input events, etc.). This module provides
//! the bridge between Servo's async events and Iced's event loop.

use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::trace;

/// Message types for the event loop waker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakerMessage {
    /// Servo has a new frame ready
    NewFrameReady,
    /// Servo needs to process events
    ProcessEvents,
    /// Servo needs to be shut down
    Shutdown,
}

/// Custom waker that implements Servo's EventLoopWaker trait
/// and bridges to Iced's event loop
pub struct IcedEventLoopWaker {
    sender: mpsc::UnboundedSender<WakerMessage>,
}

impl IcedEventLoopWaker {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<WakerMessage>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self { sender }, receiver)
    }
    
    /// Wake the event loop with a specific message
    pub fn wake_with(&self, message: WakerMessage) {
        trace!("Waking event loop: {:?}", message);
        let _ = self.sender.send(message);
    }
}

/// Handle for receiving waker messages in the event loop
pub struct WakerHandle {
    receiver: mpsc::UnboundedReceiver<WakerMessage>,
}

impl WakerHandle {
    pub fn new(receiver: mpsc::UnboundedReceiver<WakerMessage>) -> Self {
        Self { receiver }
    }
    
    /// Try to receive a message without blocking
    pub fn try_recv(&mut self) -> Option<WakerMessage> {
        self.receiver.try_recv().ok()
    }
    
    /// Receive a message asynchronously
    pub async fn recv(&mut self) -> Option<WakerMessage> {
        self.receiver.recv().await
    }
    
    /// Check if there are pending messages
    pub fn has_messages(&self) -> bool {
        !self.receiver.is_empty()
    }
}

/// Thread-safe waker that can be shared with Servo
#[derive(Clone)]
pub struct SharedWaker {
    inner: Arc<IcedEventLoopWaker>,
}

impl SharedWaker {
    pub fn new(waker: IcedEventLoopWaker) -> Self {
        Self {
            inner: Arc::new(waker),
        }
    }
    
    pub fn wake(&self) {
        self.inner.wake_with(WakerMessage::ProcessEvents);
    }
    
    pub fn wake_for_frame(&self) {
        self.inner.wake_with(WakerMessage::NewFrameReady);
    }
}

/// Frame timing controller for smooth rendering
pub struct FrameController {
    waker: SharedWaker,
    target_fps: u32,
    vsync_enabled: bool,
}

impl FrameController {
    pub fn new(waker: SharedWaker) -> Self {
        Self {
            waker,
            target_fps: 60,
            vsync_enabled: true,
        }
    }
    
    /// Request a new frame from Servo
    pub fn request_frame(&self) {
        self.waker.wake_for_frame();
    }
    
    /// Set target FPS
    pub fn set_target_fps(&mut self, fps: u32) {
        self.target_fps = fps;
    }
    
    /// Enable/disable VSync
    pub fn set_vsync(&mut self, enabled: bool) {
        self.vsync_enabled = enabled;
    }
}
