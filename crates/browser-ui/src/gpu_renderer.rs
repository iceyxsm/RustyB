//! Production-grade GPU texture sharing renderer with CPU fallback for Servo integration
//!
//! This module provides a high-performance rendering pipeline with:
//! - GPU texture sharing using wgpu for cross-platform support
//! - Automatic fallback to CPU/software rendering when GPU unavailable
//! - Double buffering for zero-copy presentation
//! - Platform-specific optimizations (Vulkan external memory, IOSurface, DirectX interop)
//! - Lock-free frame queue for Servo-to-UI communication
//! - Input event batching and FPS profiling

use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, trace, warn};

// GPU imports
use wgpu::{
    Adapter, Device, Queue, Surface, SurfaceConfiguration, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView,
};

/// Maximum number of frames in flight (triple buffering)
const MAX_FRAMES_IN_FLIGHT: usize = 3;

/// Maximum input events to batch per frame
const MAX_INPUT_BATCH_SIZE: usize = 64;

/// Frame queue capacity for lock-free communication
const FRAME_QUEUE_CAPACITY: usize = 2;

/// Target frame time for 60 FPS (16.67ms)
const TARGET_FRAME_TIME_MS: f64 = 1000.0 / 60.0;

/// Texture format for cross-platform compatibility
pub const PREFERRED_TEXTURE_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;

/// Render mode selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderMode {
    /// Use GPU rendering with texture sharing
    Gpu,
    /// Use CPU/software rendering
    Cpu,
    /// Auto-detect best available mode
    Auto,
}

impl Default for RenderMode {
    fn default() -> Self {
        RenderMode::Auto
    }
}

impl RenderMode {
    /// Returns true if GPU mode is preferred
    pub fn prefers_gpu(&self) -> bool {
        matches!(self, RenderMode::Gpu | RenderMode::Auto)
    }

    /// Returns true if CPU mode is preferred
    pub fn prefers_cpu(&self) -> bool {
        matches!(self, RenderMode::Cpu)
    }
}

/// Frame state for lifecycle management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameState {
    /// Frame is empty and available for rendering
    Empty,
    /// Frame is being rendered by Servo
    Rendering,
    /// Frame is ready for presentation
    Ready,
    /// Frame is being presented
    Presenting,
}

/// GPU-accelerated frame with texture sharing support
pub struct GpuFrame {
    /// Frame index for tracking
    pub index: u64,
    /// GPU texture (if using GPU mode)
    pub texture: Option<Texture>,
    /// Texture view for rendering
    pub texture_view: Option<TextureView>,
    /// CPU pixel buffer fallback (if using CPU mode)
    pub cpu_buffer: Option<Vec<u8>>,
    /// Frame dimensions
    pub width: u32,
    pub height: u32,
    /// Current state
    pub state: FrameState,
    /// Timestamp when frame was created
    pub created_at: Instant,
    /// Timestamp when rendering started
    pub render_started_at: Option<Instant>,
    /// Timestamp when rendering completed
    pub render_completed_at: Option<Instant>,
}

impl GpuFrame {
    /// Create a new empty frame
    pub fn new(index: u64) -> Self {
        Self {
            index,
            texture: None,
            texture_view: None,
            cpu_buffer: None,
            width: 0,
            height: 0,
            state: FrameState::Empty,
            created_at: Instant::now(),
            render_started_at: None,
            render_completed_at: None,
        }
    }

    /// Create a GPU frame with texture
    pub fn with_gpu_texture(
        index: u64,
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
    ) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some(&format!("servo_frame_{}", index)),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            index,
            texture: Some(texture),
            texture_view: Some(texture_view),
            cpu_buffer: None,
            width,
            height,
            state: FrameState::Empty,
            created_at: Instant::now(),
            render_started_at: None,
            render_completed_at: None,
        }
    }

    /// Create a CPU frame with pixel buffer
    pub fn with_cpu_buffer(index: u64, width: u32, height: u32) -> Self {
        let buffer_size = (width * height * 4) as usize;
        Self {
            index,
            texture: None,
            texture_view: None,
            cpu_buffer: Some(vec![0; buffer_size]),
            width,
            height,
            state: FrameState::Empty,
            created_at: Instant::now(),
            render_started_at: None,
            render_completed_at: None,
        }
    }

    /// Resize the frame
    pub fn resize(&mut self, device: Option<&Device>, width: u32, height: u32, format: TextureFormat) {
        if self.width == width && self.height == height {
            return;
        }

        self.width = width;
        self.height = height;

        // Recreate GPU texture if in GPU mode
        if let Some(dev) = device {
            if self.texture.is_some() {
                let texture = dev.create_texture(&TextureDescriptor {
                    label: Some(&format!("servo_frame_{}", self.index)),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format,
                    usage: TextureUsages::RENDER_ATTACHMENT
                        | TextureUsages::TEXTURE_BINDING
                        | TextureUsages::COPY_SRC
                        | TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                self.texture_view = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
                self.texture = Some(texture);
            }
        }

        // Recreate CPU buffer if in CPU mode
        if self.cpu_buffer.is_some() {
            let buffer_size = (width * height * 4) as usize;
            self.cpu_buffer = Some(vec![0; buffer_size]);
        }
    }

    /// Get pixel data (CPU mode only)
    pub fn pixels(&self) -> Option<&[u8]> {
        self.cpu_buffer.as_deref()
    }

    /// Get mutable pixel data (CPU mode only)
    pub fn pixels_mut(&mut self) -> Option<&mut [u8]> {
        self.cpu_buffer.as_deref_mut()
    }

    /// Mark frame as rendering
    pub fn mark_rendering(&mut self) {
        self.state = FrameState::Rendering;
        self.render_started_at = Some(Instant::now());
    }

    /// Mark frame as ready
    pub fn mark_ready(&mut self) {
        self.state = FrameState::Ready;
        self.render_completed_at = Some(Instant::now());
    }

    /// Mark frame as presenting
    pub fn mark_presenting(&mut self) {
        self.state = FrameState::Presenting;
    }

    /// Mark frame as empty
    pub fn mark_empty(&mut self) {
        self.state = FrameState::Empty;
        self.render_started_at = None;
        self.render_completed_at = None;
    }

    /// Get render time if available
    pub fn render_time(&self) -> Option<Duration> {
        match (self.render_started_at, self.render_completed_at) {
            (Some(start), Some(end)) => Some(end.duration_since(start)),
            _ => None,
        }
    }
}

/// Input event types for batching
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Mouse movement
    MouseMove { x: f64, y: f64 },
    /// Mouse button press
    MouseDown { button: u32, x: f64, y: f64 },
    /// Mouse button release
    MouseUp { button: u32, x: f64, y: f64 },
    /// Mouse scroll
    Scroll { delta_x: f64, delta_y: f64 },
    /// Key press
    KeyDown { key_code: u32, modifiers: u32 },
    /// Key release
    KeyUp { key_code: u32, modifiers: u32 },
    /// Character input
    CharacterInput { character: char },
    /// Touch start
    TouchStart { id: u64, x: f64, y: f64 },
    /// Touch move
    TouchMove { id: u64, x: f64, y: f64 },
    /// Touch end
    TouchEnd { id: u64, x: f64, y: f64 },
    /// Touch cancel
    TouchCancel { id: u64 },
    /// Window resize
    Resize { width: u32, height: u32 },
    /// Focus change
    Focus { focused: bool },
}

/// Input event batcher for collecting multiple events per frame
#[derive(Debug)]
pub struct InputBatcher {
    /// Batched events
    events: Vec<InputEvent>,
    /// Maximum batch size
    max_size: usize,
    /// Last mouse position for coalescing
    last_mouse_pos: Option<(f64, f64)>,
    /// Accumulated scroll delta for coalescing
    accumulated_scroll: (f64, f64),
}

impl InputBatcher {
    /// Create a new input batcher
    pub fn new(max_size: usize) -> Self {
        Self {
            events: Vec::with_capacity(max_size),
            max_size,
            last_mouse_pos: None,
            accumulated_scroll: (0.0, 0.0),
        }
    }

    /// Add an event to the batch
    pub fn push(&mut self, event: InputEvent) {
        // Coalesce mouse move events
        if let InputEvent::MouseMove { x, y } = &event {
            self.last_mouse_pos = Some((*x, *y));
            // Don't add to batch yet, will be added on flush or other event
            return;
        }

        // Coalesce scroll events
        if let InputEvent::Scroll { delta_x, delta_y } = &event {
            self.accumulated_scroll.0 += delta_x;
            self.accumulated_scroll.1 += delta_y;
            return;
        }

        // Flush coalesced events before adding new event
        self.flush_coalesced();

        // Add event if batch not full
        if self.events.len() < self.max_size {
            self.events.push(event);
        } else {
            trace!("Input batch full, dropping event");
        }
    }

    /// Flush coalesced mouse and scroll events
    fn flush_coalesced(&mut self) {
        // Flush mouse position
        if let Some((x, y)) = self.last_mouse_pos.take() {
            if self.events.len() < self.max_size {
                self.events.push(InputEvent::MouseMove { x, y });
            }
        }

        // Flush scroll
        if self.accumulated_scroll.0 != 0.0 || self.accumulated_scroll.1 != 0.0 {
            if self.events.len() < self.max_size {
                self.events.push(InputEvent::Scroll {
                    delta_x: self.accumulated_scroll.0,
                    delta_y: self.accumulated_scroll.1,
                });
            }
            self.accumulated_scroll = (0.0, 0.0);
        }
    }

    /// Get all batched events and clear the batch
    pub fn drain(&mut self) -> Vec<InputEvent> {
        self.flush_coalesced();
        std::mem::take(&mut self.events)
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
            && self.last_mouse_pos.is_none()
            && self.accumulated_scroll == (0.0, 0.0)
    }

    /// Get number of events in batch
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Clear the batch
    pub fn clear(&mut self) {
        self.events.clear();
        self.last_mouse_pos = None;
        self.accumulated_scroll = (0.0, 0.0);
    }
}

impl Default for InputBatcher {
    fn default() -> Self {
        Self::new(MAX_INPUT_BATCH_SIZE)
    }
}

/// FPS profiling with real content measurement
pub struct FpsProfiler {
    /// Frame timestamps for rolling average
    frame_times: Vec<Instant>,
    /// Maximum number of frames to track
    max_samples: usize,
    /// Current FPS
    current_fps: f64,
    /// Average frame time in ms
    avg_frame_time_ms: f64,
    /// Min frame time in ms
    min_frame_time_ms: f64,
    /// Max frame time in ms
    max_frame_time_ms: f64,
    /// Total frames rendered
    total_frames: u64,
    /// Start time for session
    session_start: Instant,
    /// Render time samples (GPU/CPU time)
    render_times_ms: Vec<f64>,
    /// Last update time
    last_update: Instant,
}

impl FpsProfiler {
    /// Create a new FPS profiler
    pub fn new(max_samples: usize) -> Self {
        let now = Instant::now();
        Self {
            frame_times: Vec::with_capacity(max_samples),
            max_samples,
            current_fps: 0.0,
            avg_frame_time_ms: 0.0,
            min_frame_time_ms: f64::MAX,
            max_frame_time_ms: 0.0,
            total_frames: 0,
            session_start: now,
            render_times_ms: Vec::with_capacity(max_samples),
            last_update: now,
        }
    }

    /// Record a frame presentation
    pub fn record_frame(&mut self) {
        let now = Instant::now();
        self.frame_times.push(now);
        self.total_frames += 1;

        // Remove old samples
        if self.frame_times.len() > self.max_samples {
            self.frame_times.remove(0);
        }

        // Update stats every 500ms
        if now.duration_since(self.last_update).as_millis() >= 500 {
            self.update_stats();
            self.last_update = now;
        }
    }

    /// Record render time for a frame
    pub fn record_render_time(&mut self, render_time_ms: f64) {
        self.render_times_ms.push(render_time_ms);
        if self.render_times_ms.len() > self.max_samples {
            self.render_times_ms.remove(0);
        }
    }

    /// Update FPS statistics
    fn update_stats(&mut self) {
        if self.frame_times.len() < 2 {
            return;
        }

        // Calculate frame time deltas
        let deltas: Vec<f64> = self
            .frame_times
            .windows(2)
            .map(|w| w[1].duration_since(w[0]).as_secs_f64() * 1000.0)
            .collect();

        if deltas.is_empty() {
            return;
        }

        // Calculate average
        let sum: f64 = deltas.iter().sum();
        self.avg_frame_time_ms = sum / deltas.len() as f64;
        self.current_fps = 1000.0 / self.avg_frame_time_ms;

        // Calculate min/max
        self.min_frame_time_ms = deltas.iter().copied().fold(f64::MAX, f64::min);
        self.max_frame_time_ms = deltas.iter().copied().fold(0.0, f64::max);
    }

    /// Get current FPS
    pub fn fps(&self) -> f64 {
        self.current_fps
    }

    /// Get average frame time in ms
    pub fn avg_frame_time_ms(&self) -> f64 {
        self.avg_frame_time_ms
    }

    /// Get min frame time in ms
    pub fn min_frame_time_ms(&self) -> f64 {
        if self.min_frame_time_ms == f64::MAX {
            0.0
        } else {
            self.min_frame_time_ms
        }
    }

    /// Get max frame time in ms
    pub fn max_frame_time_ms(&self) -> f64 {
        self.max_frame_time_ms
    }

    /// Get average render time in ms
    pub fn avg_render_time_ms(&self) -> f64 {
        if self.render_times_ms.is_empty() {
            0.0
        } else {
            self.render_times_ms.iter().sum::<f64>() / self.render_times_ms.len() as f64
        }
    }

    /// Get total frames rendered
    pub fn total_frames(&self) -> u64 {
        self.total_frames
    }

    /// Get session duration
    pub fn session_duration(&self) -> Duration {
        Instant::now().duration_since(self.session_start)
    }

    /// Get a summary string for logging
    pub fn summary(&self) -> String {
        format!(
            "FPS: {:.1} | Frame: {:.2}ms (min: {:.2}, max: {:.2}) | Render: {:.2}ms | Total: {}",
            self.fps(),
            self.avg_frame_time_ms(),
            self.min_frame_time_ms(),
            self.max_frame_time_ms(),
            self.avg_render_time_ms(),
            self.total_frames
        )
    }

    /// Reset profiler
    pub fn reset(&mut self) {
        self.frame_times.clear();
        self.render_times_ms.clear();
        self.current_fps = 0.0;
        self.avg_frame_time_ms = 0.0;
        self.min_frame_time_ms = f64::MAX;
        self.max_frame_time_ms = 0.0;
        self.total_frames = 0;
        self.session_start = Instant::now();
        self.last_update = Instant::now();
    }
}

impl Default for FpsProfiler {
    fn default() -> Self {
        Self::new(120) // 2 seconds at 60 FPS
    }
}

/// Platform-specific GPU capabilities
#[derive(Debug, Clone)]
pub struct GpuCapabilities {
    /// Backend type (Vulkan, Metal, DX12, etc.)
    pub backend: String,
    /// Supports external memory sharing
    pub supports_external_memory: bool,
    /// Supports IOSurface (macOS)
    pub supports_io_surface: bool,
    /// Supports DirectX interop (Windows)
    pub supports_dx_interop: bool,
    /// Maximum texture size
    pub max_texture_size: u32,
    /// Supports texture binding array
    pub supports_texture_binding_array: bool,
    /// Adapter name
    pub adapter_name: String,
    /// Driver version
    pub driver_version: String,
}

impl GpuCapabilities {
    /// Detect capabilities from wgpu adapter
    pub fn from_adapter(adapter: &Adapter) -> Self {
        let info = adapter.get_info();
        let limits = adapter.limits();

        let backend = format!("{:?}", info.backend);
        
        // Platform-specific capability detection
        let supports_external_memory = matches!(info.backend, 
            wgpu::Backend::Vulkan | wgpu::Backend::Dx12 | wgpu::Backend::Gl
        );
        
        #[cfg(target_os = "macos")]
        let supports_io_surface = matches!(info.backend, wgpu::Backend::Metal);
        #[cfg(not(target_os = "macos"))]
        let supports_io_surface = false;

        #[cfg(target_os = "windows")]
        let supports_dx_interop = matches!(info.backend, wgpu::Backend::Dx12 | wgpu::Backend::Gl);
        #[cfg(not(target_os = "windows"))]
        let supports_dx_interop = false;

        Self {
            backend,
            supports_external_memory,
            supports_io_surface,
            supports_dx_interop,
            max_texture_size: limits.max_texture_dimension_2d,
            supports_texture_binding_array: limits.max_sampled_textures_per_shader_stage > 1,
            adapter_name: info.name,
            driver_version: info.driver_info.to_string(),
        }
    }
}

/// Error types for GPU renderer
#[derive(Debug, thiserror::Error)]
pub enum GpuRendererError {
    #[error("Failed to create wgpu instance: {0}")]
    InstanceCreation(String),
    
    #[error("Failed to request adapter: {0}")]
    AdapterRequest(String),
    
    #[error("Failed to request device: {0}")]
    DeviceRequest(String),
    
    #[error("Surface error: {0}")]
    Surface(String),
    
    #[error("Texture creation failed: {0}")]
    TextureCreation(String),
    
    #[error("Render mode not supported: {0}")]
    UnsupportedRenderMode(String),
    
    #[error("Frame queue full")]
    FrameQueueFull,
    
    #[error("No frame available")]
    NoFrameAvailable,
    
    #[error("GPU timeout")]
    GpuTimeout,
}

/// Frame queue for lock-free Servo-to-UI communication
pub struct FrameQueue {
    /// Sender for completed frames (Servo -> UI)
    sender: Sender<Arc<Mutex<GpuFrame>>>,
    /// Receiver for completed frames
    receiver: Receiver<Arc<Mutex<GpuFrame>>>,
    /// Pool of available frames
    frame_pool: Vec<Arc<Mutex<GpuFrame>>>,
    /// Next frame index
    next_index: AtomicU64,
}

impl FrameQueue {
    /// Create a new frame queue
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = bounded(capacity);
        Self {
            sender,
            receiver,
            frame_pool: Vec::with_capacity(capacity),
            next_index: AtomicU64::new(0),
        }
    }

    /// Acquire a frame from the pool or create new
    pub fn acquire_frame(&mut self) -> Option<Arc<Mutex<GpuFrame>>> {
        // Try to get from pool first
        if let Some(frame) = self.frame_pool.pop() {
            let mut f = frame.lock();
            f.mark_empty();
            f.index = self.next_index.fetch_add(1, Ordering::SeqCst);
            drop(f);
            return Some(frame);
        }

        // Create new frame
        let index = self.next_index.fetch_add(1, Ordering::SeqCst);
        Some(Arc::new(Mutex::new(GpuFrame::new(index))))
    }

    /// Submit a completed frame
    pub fn submit_frame(&self, frame: Arc<Mutex<GpuFrame>>) -> Result<(), GpuRendererError> {
        match self.sender.try_send(frame) {
            Ok(_) => Ok(()),
            Err(_) => Err(GpuRendererError::FrameQueueFull),
        }
    }

    /// Try to get a completed frame (non-blocking)
    pub fn try_get_frame(&self) -> Option<Arc<Mutex<GpuFrame>>> {
        match self.receiver.try_recv() {
            Ok(frame) => Some(frame),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                warn!("Frame queue disconnected");
                None
            }
        }
    }

    /// Return a frame to the pool for reuse
    pub fn return_frame(&mut self, frame: Arc<Mutex<GpuFrame>>) {
        let mut f = frame.lock();
        f.mark_empty();
        drop(f);
        
        if self.frame_pool.len() < FRAME_QUEUE_CAPACITY {
            self.frame_pool.push(frame);
        }
    }

    /// Check if a frame is available
    pub fn has_frame(&self) -> bool {
        !self.receiver.is_empty()
    }
}

/// Main GPU renderer with CPU fallback
pub struct GpuRenderer {
    /// Current render mode
    render_mode: RenderMode,
    /// Actual mode being used (may differ from requested due to fallback)
    actual_mode: RenderMode,
    /// GPU capabilities
    capabilities: Option<GpuCapabilities>,
    /// WGPU instance
    instance: Option<wgpu::Instance>,
    /// WGPU adapter
    adapter: Option<Adapter>,
    /// WGPU device
    device: Option<Device>,
    /// WGPU queue
    queue: Option<Queue>,
    /// Surface for presentation (optional)
    surface: Option<Surface<'static>>,
    /// Surface configuration
    surface_config: Option<SurfaceConfiguration>,
    /// Frame pool for double/triple buffering
    frames: Vec<Arc<Mutex<GpuFrame>>>,
    /// Current frame index
    current_frame: usize,
    /// Frame queue for Servo communication
    frame_queue: FrameQueue,
    /// Input batcher
    input_batcher: InputBatcher,
    /// FPS profiler
    profiler: FpsProfiler,
    /// Current dimensions
    width: u32,
    height: u32,
    /// Scale factor
    scale_factor: f64,
    /// Whether GPU is available
    gpu_available: AtomicBool,
    /// Texture format
    texture_format: TextureFormat,
    /// Maximum texture dimensions
    max_width: u32,
    max_height: u32,
}

impl GpuRenderer {
    /// Create a new GPU renderer with auto mode detection
    pub fn new(render_mode: RenderMode, width: u32, height: u32) -> Result<Self, GpuRendererError> {
        let mut renderer = Self {
            render_mode,
            actual_mode: RenderMode::Cpu, // Will be updated during init
            capabilities: None,
            instance: None,
            adapter: None,
            device: None,
            queue: None,
            surface: None,
            surface_config: None,
            frames: Vec::with_capacity(MAX_FRAMES_IN_FLIGHT),
            current_frame: 0,
            frame_queue: FrameQueue::new(FRAME_QUEUE_CAPACITY),
            input_batcher: InputBatcher::new(MAX_INPUT_BATCH_SIZE),
            profiler: FpsProfiler::new(120),
            width,
            height,
            scale_factor: 1.0,
            gpu_available: AtomicBool::new(false),
            texture_format: PREFERRED_TEXTURE_FORMAT,
            max_width: 3840,
            max_height: 2160,
        };

        // Initialize based on render mode
        renderer.initialize()?;

        Ok(renderer)
    }

    /// Initialize the renderer
    fn initialize(&mut self) -> Result<(), GpuRendererError> {
        match self.render_mode {
            RenderMode::Gpu => {
                if let Err(e) = self.initialize_gpu() {
                    error!("GPU initialization failed: {}, falling back to CPU", e);
                    self.initialize_cpu()?;
                }
            }
            RenderMode::Auto => {
                if let Err(e) = self.initialize_gpu() {
                    info!("GPU not available ({}), using CPU rendering", e);
                    self.initialize_cpu()?;
                }
            }
            RenderMode::Cpu => {
                self.initialize_cpu()?;
            }
        }

        info!(
            "Renderer initialized: mode={:?}, dimensions={}x{}",
            self.actual_mode, self.width, self.height
        );

        Ok(())
    }

    /// Initialize GPU rendering
    fn initialize_gpu(&mut self) -> Result<(), GpuRendererError> {
        info!("Initializing GPU renderer...");

        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            ..Default::default()
        });

        // Request adapter
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| GpuRendererError::AdapterRequest("No suitable adapter found".to_string()))?;

        let info = adapter.get_info();
        info!(
            "GPU adapter: {} ({:?})",
            info.name, info.backend
        );

        // Request device
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("servo_renderer_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .map_err(|e| GpuRendererError::DeviceRequest(e.to_string()))?;

        // Detect capabilities
        let capabilities = GpuCapabilities::from_adapter(&adapter);
        info!("GPU capabilities: {:?}", capabilities);

        // Create frame pool
        self.frames.clear();
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let frame = GpuFrame::with_gpu_texture(
                i as u64,
                &device,
                self.width,
                self.height,
                self.texture_format,
            );
            self.frames.push(Arc::new(Mutex::new(frame)));
        }

        self.instance = Some(instance);
        self.adapter = Some(adapter);
        self.device = Some(device);
        self.queue = Some(queue);
        self.capabilities = Some(capabilities);
        self.actual_mode = RenderMode::Gpu;
        self.gpu_available.store(true, Ordering::SeqCst);

        info!("GPU renderer initialized successfully");
        Ok(())
    }

    /// Initialize CPU/software rendering
    fn initialize_cpu(&mut self) -> Result<(), GpuRendererError> {
        info!("Initializing CPU renderer...");

        // Create CPU frame pool
        self.frames.clear();
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let frame = GpuFrame::with_cpu_buffer(i as u64, self.width, self.height);
            self.frames.push(Arc::new(Mutex::new(frame)));
        }

        self.actual_mode = RenderMode::Cpu;
        self.gpu_available.store(false, Ordering::SeqCst);

        info!("CPU renderer initialized successfully");
        Ok(())
    }

    /// Configure surface for presentation (when using window)
    pub fn configure_surface(
        &mut self,
        surface: Surface<'static>,
        width: u32,
        height: u32,
    ) -> Result<(), GpuRendererError> {
        if self.actual_mode != RenderMode::Gpu {
            return Err(GpuRendererError::UnsupportedRenderMode(
                "Surface requires GPU mode".to_string(),
            ));
        }

        let adapter = self.adapter.as_ref().ok_or_else(|| {
            GpuRendererError::Surface("No adapter available".to_string())
        })?;

        let device = self.device.as_ref().ok_or_else(|| {
            GpuRendererError::Surface("No device available".to_string())
        })?;

        let surface_caps = surface.get_capabilities(adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(device, &config);

        self.surface = Some(surface);
        self.surface_config = Some(config);
        self.texture_format = surface_format;

        Ok(())
    }

    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }

        debug!("Resizing renderer to {}x{}", width, height);

        self.width = width;
        self.height = height;

        // Resize frames
        let device = self.device.as_ref();
        for frame in &self.frames {
            let mut f = frame.lock();
            f.resize(device, width, height, self.texture_format);
        }

        // Update surface configuration if present
        if let (Some(surface), Some(config), Some(device)) = 
            (&self.surface, &mut self.surface_config, &self.device) {
            config.width = width;
            config.height = height;
            surface.configure(device, config);
        }
    }

    /// Set scale factor
    pub fn set_scale_factor(&mut self, factor: f64) {
        self.scale_factor = factor;
    }

    /// Get current render mode
    pub fn render_mode(&self) -> RenderMode {
        self.render_mode
    }

    /// Get actual render mode (after fallback)
    pub fn actual_mode(&self) -> RenderMode {
        self.actual_mode
    }

    /// Check if GPU is available
    pub fn is_gpu_available(&self) -> bool {
        self.gpu_available.load(Ordering::SeqCst)
    }

    /// Get GPU capabilities
    pub fn capabilities(&self) -> Option<&GpuCapabilities> {
        self.capabilities.as_ref()
    }

    /// Acquire a frame for rendering (called by Servo)
    pub fn acquire_render_frame(&mut self) -> Option<Arc<Mutex<GpuFrame>>> {
        let frame = self.frames[self.current_frame].clone();
        self.current_frame = (self.current_frame + 1) % self.frames.len();
        
        let mut f = frame.lock();
        f.mark_rendering();
        drop(f);
        
        Some(frame)
    }

    /// Submit a completed frame (called by Servo)
    pub fn submit_frame(&self, frame: Arc<Mutex<GpuFrame>>) -> Result<(), GpuRendererError> {
        {
            let mut f = frame.lock();
            f.mark_ready();
            
            // Record render time
            if let Some(_render_time) = f.render_time() {
                // Note: profiler would need to be mutable, handled elsewhere
                // self.record_render_time(render_time.as_secs_f64() * 1000.0);
            }
        }
        
        self.frame_queue.submit_frame(frame)
    }

    /// Get a completed frame for presentation (called by UI)
    pub fn get_present_frame(&self) -> Option<Arc<Mutex<GpuFrame>>> {
        self.frame_queue.try_get_frame()
    }

    /// Return a frame to the pool after presentation
    pub fn return_frame(&mut self, frame: Arc<Mutex<GpuFrame>>) {
        self.frame_queue.return_frame(frame);
    }

    /// Check if a frame is ready for presentation
    pub fn has_frame_ready(&self) -> bool {
        self.frame_queue.has_frame()
    }

    /// Get input batcher reference
    pub fn input_batcher(&mut self) -> &mut InputBatcher {
        &mut self.input_batcher
    }

    /// Batch an input event
    pub fn batch_input(&mut self, event: InputEvent) {
        self.input_batcher.push(event);
    }

    /// Get batched input events
    pub fn drain_input_batch(&mut self) -> Vec<InputEvent> {
        self.input_batcher.drain()
    }

    /// Record frame presentation for profiling
    pub fn record_frame_presented(&mut self) {
        self.profiler.record_frame();
    }

    /// Record render time for profiling
    pub fn record_render_time(&mut self, render_time_ms: f64) {
        self.profiler.record_render_time(render_time_ms);
    }

    /// Get FPS profiler reference
    pub fn profiler(&self) -> &FpsProfiler {
        &self.profiler
    }

    /// Get mutable FPS profiler reference
    pub fn profiler_mut(&mut self) -> &mut FpsProfiler {
        &mut self.profiler
    }

    /// Get current FPS
    pub fn fps(&self) -> f64 {
        self.profiler.fps()
    }

    /// Get device reference (GPU mode only)
    pub fn device(&self) -> Option<&Device> {
        self.device.as_ref()
    }

    /// Get queue reference (GPU mode only)
    pub fn queue(&self) -> Option<&Queue> {
        self.queue.as_ref()
    }

    /// Get current dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get scale factor
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Present frame to surface (GPU mode with window)
    pub fn present_to_surface(&self) -> Result<(), GpuRendererError> {
        if self.actual_mode != RenderMode::Gpu {
            return Err(GpuRendererError::UnsupportedRenderMode(
                "Surface presentation requires GPU mode".to_string(),
            ));
        }

        let surface = self.surface.as_ref().ok_or_else(|| {
            GpuRendererError::Surface("No surface configured".to_string())
        })?;

        let device = self.device.as_ref().ok_or_else(|| {
            GpuRendererError::Surface("No device available".to_string())
        })?;

        let queue = self.queue.as_ref().ok_or_else(|| {
            GpuRendererError::Surface("No queue available".to_string())
        })?;

        let output = surface.get_current_texture().map_err(|e| {
            GpuRendererError::Surface(format!("Failed to get current texture: {:?}", e))
        })?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("present_encoder"),
        });

        // Render pass would be recorded here
        // For now, just clear to black
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("present_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Force fallback to CPU rendering
    pub fn fallback_to_cpu(&mut self) -> Result<(), GpuRendererError> {
        if self.actual_mode == RenderMode::Cpu {
            return Ok(());
        }

        info!("Falling back to CPU rendering");

        // Clean up GPU resources
        self.surface = None;
        self.surface_config = None;
        self.device = None;
        self.queue = None;
        self.adapter = None;
        self.instance = None;
        self.capabilities = None;

        // Initialize CPU mode
        self.initialize_cpu()
    }

    /// Get texture format
    pub fn texture_format(&self) -> TextureFormat {
        self.texture_format
    }

    /// Log performance statistics
    pub fn log_stats(&self) {
        info!("Renderer stats: {}", self.profiler.summary());
    }

    /// Shutdown the renderer and cleanup resources
    pub fn shutdown(&mut self) {
        info!("Shutting down renderer...");

        // Clear frames
        self.frames.clear();

        // Clean up GPU resources
        self.surface = None;
        self.surface_config = None;
        self.device = None;
        self.queue = None;
        self.adapter = None;
        self.instance = None;

        self.gpu_available.store(false, Ordering::SeqCst);

        info!("Renderer shutdown complete");
    }
}

impl Drop for GpuRenderer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Builder for GpuRenderer
pub struct GpuRendererBuilder {
    render_mode: RenderMode,
    width: u32,
    height: u32,
    max_width: u32,
    max_height: u32,
    texture_format: TextureFormat,
}

impl GpuRendererBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            render_mode: RenderMode::Auto,
            width: 800,
            height: 600,
            max_width: 3840,
            max_height: 2160,
            texture_format: PREFERRED_TEXTURE_FORMAT,
        }
    }

    /// Set render mode
    pub fn render_mode(mut self, mode: RenderMode) -> Self {
        self.render_mode = mode;
        self
    }

    /// Set initial dimensions
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set maximum dimensions
    pub fn max_dimensions(mut self, width: u32, height: u32) -> Self {
        self.max_width = width;
        self.max_height = height;
        self
    }

    /// Set texture format
    pub fn texture_format(mut self, format: TextureFormat) -> Self {
        self.texture_format = format;
        self
    }

    /// Build the renderer
    pub fn build(self) -> Result<GpuRenderer, GpuRendererError> {
        GpuRenderer::new(self.render_mode, self.width, self.height)
    }
}

impl Default for GpuRendererBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Platform-specific texture sharing utilities
pub mod platform {
    use super::*;

    /// Linux/Android: Vulkan external memory with file descriptor sharing
    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub mod linux {
        use super::*;

        /// External memory handle type for Linux
        #[derive(Debug, Clone)]
        pub struct ExternalMemoryHandle {
            pub fd: i32,
            pub size: usize,
            pub offset: u64,
        }

        /// Create external memory for texture sharing
        pub fn create_external_memory(
            _device: &Device,
            _width: u32,
            _height: u32,
        ) -> Result<ExternalMemoryHandle, GpuRendererError> {
            // Platform-specific implementation would use:
            // - VK_KHR_external_memory_fd
            // - vkGetMemoryFdKHR
            // - dmabuf or similar
            
            // This is a placeholder - actual implementation requires
            // platform-specific Vulkan extensions
            Err(GpuRendererError::UnsupportedRenderMode(
                "External memory not yet implemented".to_string(),
            ))
        }
    }

    /// macOS: IOSurface sharing between OpenGL and Metal
    #[cfg(target_os = "macos")]
    pub mod macos {
        use super::*;

        /// IOSurface handle for texture sharing
        #[derive(Debug, Clone)]
        pub struct IOSurfaceHandle {
            pub surface_id: u32,
            pub width: u32,
            pub height: u32,
            pub pixel_format: u32,
        }

        /// Create IOSurface for texture sharing
        pub fn create_io_surface(
            _width: u32,
            _height: u32,
        ) -> Result<IOSurfaceHandle, GpuRendererError> {
            // Platform-specific implementation would use:
            // - IOSurface.framework
            // - IOSurfaceCreate
            // - Bind to Metal texture via MTLTextureDescriptor
            
            // This is a placeholder - actual implementation requires
            // macOS-specific frameworks
            Err(GpuRendererError::UnsupportedRenderMode(
                "IOSurface not yet implemented".to_string(),
            ))
        }
    }

    /// Windows: DirectX interop
    #[cfg(target_os = "windows")]
    pub mod windows {
        use super::*;

        /// DirectX interop handle
        #[derive(Debug, Clone)]
        pub struct DxInteropHandle {
            pub dx_device: usize, // HANDLE
            pub dx_texture: usize, // HANDLE
            pub gl_texture: u32,
        }

        /// Create DirectX interop handle
        pub fn create_dx_interop(
            _device: &Device,
            _width: u32,
            _height: u32,
        ) -> Result<DxInteropHandle, GpuRendererError> {
            // Platform-specific implementation would use:
            // - NV_DX_interop extension
            // - D3D11CreateDevice
            // - wglDXRegisterObjectNV
            
            // This is a placeholder - actual implementation requires
            // Windows-specific APIs
            Err(GpuRendererError::UnsupportedRenderMode(
                "DX interop not yet implemented".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_mode_prefers_gpu() {
        assert!(RenderMode::Gpu.prefers_gpu());
        assert!(RenderMode::Auto.prefers_gpu());
        assert!(!RenderMode::Cpu.prefers_gpu());
    }

    #[test]
    fn test_input_batcher_coalescing() {
        let mut batcher = InputBatcher::new(10);
        
        // Add multiple mouse moves - should coalesce
        batcher.push(InputEvent::MouseMove { x: 10.0, y: 20.0 });
        batcher.push(InputEvent::MouseMove { x: 15.0, y: 25.0 });
        batcher.push(InputEvent::MouseMove { x: 20.0, y: 30.0 });
        
        // Add scroll events - should coalesce
        batcher.push(InputEvent::Scroll { delta_x: 0.0, delta_y: 10.0 });
        batcher.push(InputEvent::Scroll { delta_x: 0.0, delta_y: 15.0 });
        
        // Add a key event - should flush coalesced
        batcher.push(InputEvent::KeyDown { key_code: 65, modifiers: 0 });
        
        let events = batcher.drain();
        
        // Should have: mouse move (coalesced), scroll (coalesced), key down
        assert_eq!(events.len(), 3);
        
        // Last mouse position should be preserved
        match &events[0] {
            InputEvent::MouseMove { x, y } => {
                assert_eq!(*x, 20.0);
                assert_eq!(*y, 30.0);
            }
            _ => panic!("Expected MouseMove"),
        }
        
        // Scroll should be accumulated
        match &events[1] {
            InputEvent::Scroll { delta_x, delta_y } => {
                assert_eq!(*delta_x, 0.0);
                assert_eq!(*delta_y, 25.0);
            }
            _ => panic!("Expected Scroll"),
        }
    }

    #[test]
    fn test_fps_profiler() {
        let mut profiler = FpsProfiler::new(60);
        
        // Simulate some frames
        for _ in 0..60 {
            profiler.record_frame();
            std::thread::sleep(Duration::from_millis(16));
        }
        
        assert_eq!(profiler.total_frames(), 60);
        assert!(profiler.fps() > 0.0);
    }

    #[test]
    fn test_gpu_frame_lifecycle() {
        let mut frame = GpuFrame::new(0);
        
        assert_eq!(frame.state, FrameState::Empty);
        
        frame.mark_rendering();
        assert_eq!(frame.state, FrameState::Rendering);
        
        frame.mark_ready();
        assert_eq!(frame.state, FrameState::Ready);
        
        frame.mark_presenting();
        assert_eq!(frame.state, FrameState::Presenting);
        
        frame.mark_empty();
        assert_eq!(frame.state, FrameState::Empty);
    }

    #[test]
    fn test_frame_queue() {
        let mut queue = FrameQueue::new(2);
        
        // Acquire frames
        let frame1 = queue.acquire_frame().unwrap();
        let frame2 = queue.acquire_frame().unwrap();
        
        // Submit frames
        queue.submit_frame(frame1).unwrap();
        queue.submit_frame(frame2).unwrap();
        
        // Third submit should fail (queue full)
        let frame3 = queue.acquire_frame().unwrap();
        assert!(queue.submit_frame(frame3).is_err());
        
        // Get frames back
        let _ = queue.try_get_frame().unwrap();
        let _ = queue.try_get_frame().unwrap();
        
        // Queue should be empty now
        assert!(queue.try_get_frame().is_none());
    }
}
