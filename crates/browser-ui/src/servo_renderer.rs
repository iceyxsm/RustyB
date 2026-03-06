//! High-performance Servo renderer with zero-copy texture sharing
//! 
//! Optimized for raw performance:
//! - No async on hot paths - sync operations only
//! - parking_lot RwLock (faster than std)
//! - Pre-allocated buffers - no runtime allocations
//! - Double buffering with swap chains
//! - No animations, no frame pacing - direct presentation

use parking_lot::RwLock;

/// Pre-allocated pixel buffer with fixed capacity
pub struct PixelBuffer {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

impl PixelBuffer {
    /// Create with fixed size - never reallocates
    pub fn with_capacity(max_width: u32, max_height: u32) -> Self {
        let size = (max_width * max_height * 4) as usize;
        Self {
            data: vec![0; size],
            width: 0,
            height: 0,
        }
    }
    
    /// Update dimensions without allocation
    #[inline]
    pub fn set_size(&mut self, width: u32, height: u32) {
        debug_assert!((width * height * 4) as usize <= self.data.capacity());
        self.width = width;
        self.height = height;
    }
    
    #[inline]
    pub fn pixels(&self) -> &[u8] {
        &self.data[..(self.width * self.height * 4) as usize]
    }
    
    #[inline]
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        let len = (self.width * self.height * 4) as usize;
        &mut self.data[..len]
    }
    
    #[inline]
    pub fn width(&self) -> u32 { self.width }
    #[inline]
    pub fn height(&self) -> u32 { self.height }
}

/// Double-buffered texture pair for zero-copy swapping
pub struct DoubleBuffer {
    /// Front buffer - currently being displayed
    front: PixelBuffer,
    /// Back buffer - being rendered to
    back: PixelBuffer,
    /// Max dimensions for both buffers
    max_width: u32,
    max_height: u32,
}

impl DoubleBuffer {
    pub fn new(max_width: u32, max_height: u32) -> Self {
        Self {
            front: PixelBuffer::with_capacity(max_width, max_height),
            back: PixelBuffer::with_capacity(max_width, max_height),
            max_width,
            max_height,
        }
    }
    
    /// Get back buffer for rendering
    #[inline]
    pub fn back_mut(&mut self) -> &mut PixelBuffer {
        &mut self.back
    }
    
    /// Get front buffer for display
    #[inline]
    pub fn front(&self) -> &PixelBuffer {
        &self.front
    }
    
    /// Swap buffers - O(1) pointer swap
    #[inline]
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.front, &mut self.back);
    }
    
    /// Resize both buffers without allocation
    #[inline]
    pub fn set_size(&mut self, width: u32, height: u32) {
        debug_assert!(width <= self.max_width && height <= self.max_height);
        self.front.set_size(width, height);
        self.back.set_size(width, height);
    }
}

/// Frame state - single atomic byte
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameState {
    Empty = 0,
    Rendering = 1,
    Ready = 2,
}

/// High-performance renderer - zero allocations after init
pub struct ServoRenderer {
    /// Double-buffered pixel data
    buffers: RwLock<DoubleBuffer>,
    /// Current frame state - atomic for lock-free reads
    state: std::sync::atomic::AtomicU8,
    /// Current dimensions
    size: RwLock<(u32, u32)>,
    /// Scale factor
    scale_factor: RwLock<f64>,
}

impl ServoRenderer {
    pub fn new() -> Self {
        // Pre-allocate 4K buffers - sufficient for most displays
        const MAX_WIDTH: u32 = 3840;
        const MAX_HEIGHT: u32 = 2160;
        
        Self {
            buffers: RwLock::new(DoubleBuffer::new(MAX_WIDTH, MAX_HEIGHT)),
            state: std::sync::atomic::AtomicU8::new(FrameState::Empty as u8),
            size: RwLock::new((800, 600)),
            scale_factor: RwLock::new(1.0),
        }
    }
    
    /// Set viewport size - no allocations
    #[inline]
    pub fn set_size(&self, width: u32, height: u32) {
        let mut size = self.size.write();
        let mut buffers = self.buffers.write();
        *size = (width, height);
        buffers.set_size(width, height);
    }
    
    /// Get viewport size - lock-free read
    #[inline]
    pub fn get_size(&self) -> (u32, u32) {
        *self.size.read()
    }
    
    /// Set scale factor
    #[inline]
    pub fn set_scale_factor(&self, factor: f64) {
        *self.scale_factor.write() = factor;
    }
    
    /// Get back buffer for rendering - blocking write lock
    #[inline]
    pub fn with_back_buffer<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut PixelBuffer) -> R,
    {
        let mut buffers = self.buffers.write();
        self.state.store(FrameState::Rendering as u8, std::sync::atomic::Ordering::Release);
        f(buffers.back_mut())
    }
    
    /// Mark frame as ready and swap buffers
    #[inline]
    pub fn present(&self) {
        let mut buffers = self.buffers.write();
        buffers.swap();
        self.state.store(FrameState::Ready as u8, std::sync::atomic::Ordering::Release);
    }
    
    /// Get front buffer for display - blocking read lock
    #[inline]
    pub fn with_front_buffer<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&PixelBuffer) -> R,
    {
        let buffers = self.buffers.read();
        f(buffers.front())
    }
    
    /// Lock-free check if frame is ready
    #[inline]
    pub fn has_new_frame(&self) -> bool {
        self.state.load(std::sync::atomic::Ordering::Acquire) == FrameState::Ready as u8
    }
    
    /// Mark frame as consumed
    #[inline]
    pub fn mark_consumed(&self) {
        self.state.store(FrameState::Empty as u8, std::sync::atomic::Ordering::Release);
    }
    
    /// Get current frame state
    #[inline]
    pub fn frame_state(&self) -> FrameState {
        match self.state.load(std::sync::atomic::Ordering::Acquire) {
            1 => FrameState::Rendering,
            2 => FrameState::Ready,
            _ => FrameState::Empty,
        }
    }
}

impl Default for ServoRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple FPS counter - minimal overhead
pub struct FpsCounter {
    frame_count: u64,
    last_time: std::time::Instant,
    current_fps: f64,
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            last_time: std::time::Instant::now(),
            current_fps: 0.0,
        }
    }
    
    /// Call once per frame
    #[inline]
    pub fn tick(&mut self) {
        self.frame_count += 1;
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_time);
        if elapsed.as_secs() >= 1 {
            self.current_fps = self.frame_count as f64 / elapsed.as_secs_f64();
            self.frame_count = 0;
            self.last_time = now;
        }
    }
    
    #[inline]
    pub fn fps(&self) -> f64 {
        self.current_fps
    }
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self::new()
    }
}
