//! Error types for chunk rendering operations

use std::fmt;

/// Result type for rendering operations
pub type RenderResult<T> = Result<T, RenderError>;

/// Errors that can occur during chunk rendering operations
#[derive(Debug, Clone, PartialEq)]
pub enum RenderError {
    /// Failed to create GPU buffer
    BufferCreationFailed,
    
    /// Mesh data is invalid or corrupted
    InvalidMeshData,
    
    /// Shader compilation failed
    ShaderCompilationFailed,
    
    /// Render pipeline creation failed
    PipelineCreationFailed,
    
    /// Failed to upload data to GPU buffer
    BufferUploadFailed,
    
    /// Chunk data is invalid for mesh generation
    InvalidChunkData,
    
    /// GPU device is not available
    DeviceNotAvailable,
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::BufferCreationFailed => write!(f, "Failed to create GPU buffer"),
            RenderError::InvalidMeshData => write!(f, "Mesh data is invalid or corrupted"),
            RenderError::ShaderCompilationFailed => write!(f, "Shader compilation failed"),
            RenderError::PipelineCreationFailed => write!(f, "Render pipeline creation failed"),
            RenderError::BufferUploadFailed => write!(f, "Failed to upload data to GPU buffer"),
            RenderError::InvalidChunkData => write!(f, "Chunk data is invalid for mesh generation"),
            RenderError::DeviceNotAvailable => write!(f, "GPU device is not available"),
        }
    }
}

impl std::error::Error for RenderError {}