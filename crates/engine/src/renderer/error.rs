//! Rendering system error types

use thiserror::Error;

/// Errors that can occur during window management operations
#[derive(Error, Debug)]
pub enum WindowError {
    #[error("Failed to create event loop: {0}")]
    EventLoopCreation(String),
    
    #[error("Failed to create window: {0}")]
    WindowCreation(#[from] winit::error::OsError),
    
    #[error("Window handle is invalid")]
    InvalidHandle,
}

/// Errors that can occur during graphics context initialization
#[derive(Error, Debug)]
pub enum GraphicsError {
    #[error("No suitable graphics adapter found. Ensure your system has a compatible GPU with Vulkan, DirectX 12, or Metal support")]
    NoAdapter,
    
    #[error("Failed to request graphics adapter: {reason}. This may indicate driver issues or incompatible hardware")]
    AdapterRequest { reason: String },
    
    #[error("Failed to create graphics device: {0}. Check that your GPU drivers are up to date")]
    DeviceCreation(#[from] wgpu::RequestDeviceError),
    
    #[error("Failed to create surface: {0}. This may indicate window system compatibility issues")]
    SurfaceCreation(#[from] wgpu::CreateSurfaceError),
    
    #[error("Surface configuration failed: {reason}. The window surface may be incompatible with the selected adapter")]
    SurfaceConfiguration { reason: String },
    
    #[error("Required graphics features not supported by adapter: {missing_features:?}. Consider updating GPU drivers or using different hardware")]
    MissingFeatures { missing_features: Vec<String> },
    
    #[error("Graphics adapter does not support the required limits. Required: {required}, Available: {available}")]
    InsufficientLimits { required: String, available: String },
    
    #[error("Surface lost during operation. This typically occurs during window minimize/restore")]
    SurfaceLost,
    
    #[error("Invalid surface dimensions: {width}x{height}. Dimensions must be greater than 0")]
    InvalidDimensions { width: u32, height: u32 },
}

/// Errors that can occur during shader operations
#[derive(Error, Debug)]
pub enum ShaderError {
    #[error("Shader compilation failed: {0}")]
    CompilationFailed(String),
    
    #[error("Failed to create shader module: {0}")]
    ModuleCreation(String),
    
    #[error("Failed to create render pipeline: {0}")]
    PipelineCreation(String),
    
    #[error("Invalid shader source: {0}")]
    InvalidSource(String),
    
    #[error("Shader validation failed: {0}")]
    ValidationFailed(String),
}

/// Errors that can occur during buffer operations
#[derive(Error, Debug)]
pub enum BufferError {
    #[error("Failed to create buffer: {0}")]
    CreationFailed(String),
    
    #[error("Buffer mapping failed: {0}")]
    MappingFailed(#[from] wgpu::BufferAsyncError),
    
    #[error("Invalid buffer size: expected {expected}, got {actual}")]
    InvalidSize { expected: u64, actual: u64 },
    
    #[error("Buffer usage incompatible with operation")]
    IncompatibleUsage,
}

/// Errors that can occur during texture operations
#[derive(Error, Debug)]
pub enum TextureError {
    #[error("Failed to create texture: {0}")]
    CreationFailed(String),
    
    #[error("Invalid texture format: {0}")]
    InvalidFormat(String),
    
    #[error("Texture dimensions invalid: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },
    
    #[error("Failed to load texture data: {0}")]
    LoadFailed(String),
}

/// General rendering errors that encompass all rendering operations
#[derive(Error, Debug)]
pub enum RenderError {
    #[error("Window error: {0}")]
    Window(#[from] WindowError),
    
    #[error("Graphics context error: {0}")]
    Graphics(#[from] GraphicsError),
    
    #[error("Shader error: {0}")]
    Shader(#[from] ShaderError),
    
    #[error("Buffer error: {0}")]
    Buffer(#[from] BufferError),
    
    #[error("Texture error: {0}")]
    Texture(#[from] TextureError),
    
    #[error("Surface lost, recreation required")]
    SurfaceLost,
    
    #[error("GPU timeout occurred during operation: {operation}")]
    Timeout { operation: String },
    
    #[error("Out of GPU memory")]
    OutOfMemory,
    
    #[error("Rendering operation failed: {0}")]
    OperationFailed(String),
}

/// Result type for rendering operations
pub type RenderResult<T> = Result<T, RenderError>;

/// Result type for window operations
pub type WindowResult<T> = Result<T, WindowError>;

/// Result type for graphics operations
pub type GraphicsResult<T> = Result<T, GraphicsError>;

/// Result type for shader operations
pub type ShaderResult<T> = Result<T, ShaderError>;

/// Result type for buffer operations
pub type BufferResult<T> = Result<T, BufferError>;

/// Result type for texture operations
pub type TextureResult<T> = Result<T, TextureError>;