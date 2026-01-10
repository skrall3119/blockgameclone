//! Error types for world management operations

use crate::chunk::ChunkError;
use std::fmt;

/// Result type for world operations
pub type WorldResult<T> = Result<T, WorldError>;

/// Errors that can occur during world management operations
#[derive(Debug, Clone, PartialEq)]
pub enum WorldError {
    /// Error occurred during chunk operations
    ChunkError(ChunkError),
    /// Invalid chunk coordinates provided
    InvalidCoordinates {
        x: i32,
        y: i32,
        z: i32,
        reason: String,
    },
    /// Memory allocation failed
    OutOfMemory {
        requested: usize,
        available: usize,
    },
    /// Configuration parameter is invalid
    InvalidConfiguration {
        parameter: String,
        value: String,
        reason: String,
    },
    /// Chunk loading operation failed
    LoadingFailed {
        coord: super::ChunkCoord,
        reason: String,
    },
    /// Chunk is in an invalid state for the requested operation
    InvalidChunkState {
        coord: super::ChunkCoord,
        current_state: super::ChunkState,
        required_state: super::ChunkState,
    },
    /// Performance monitoring operation failed
    MonitoringError {
        operation: String,
        reason: String,
    },
    /// Resource limit exceeded
    ResourceLimitExceeded {
        resource: String,
        limit: usize,
        requested: usize,
    },
    /// Chunk not found at the specified coordinates
    ChunkNotFound {
        coord: super::ChunkCoord,
    },
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldError::ChunkError(err) => write!(f, "Chunk error: {}", err),
            WorldError::InvalidCoordinates { x, y, z, reason } => {
                write!(f, "Invalid coordinates ({}, {}, {}): {}", x, y, z, reason)
            }
            WorldError::OutOfMemory { requested, available } => {
                write!(f, "Out of memory: requested {} bytes, {} available", requested, available)
            }
            WorldError::InvalidConfiguration { parameter, value, reason } => {
                write!(f, "Invalid configuration parameter '{}' = '{}': {}", parameter, value, reason)
            }
            WorldError::LoadingFailed { coord, reason } => {
                write!(f, "Failed to load chunk at {:?}: {}", coord, reason)
            }
            WorldError::InvalidChunkState { coord, current_state, required_state } => {
                write!(f, "Chunk at {:?} is in state {:?}, but {:?} is required", coord, current_state, required_state)
            }
            WorldError::MonitoringError { operation, reason } => {
                write!(f, "Performance monitoring error during '{}': {}", operation, reason)
            }
            WorldError::ResourceLimitExceeded { resource, limit, requested } => {
                write!(f, "Resource limit exceeded for '{}': limit {} < requested {}", resource, limit, requested)
            }
            WorldError::ChunkNotFound { coord } => {
                write!(f, "Chunk not found at coordinates {:?}", coord)
            }
        }
    }
}

impl std::error::Error for WorldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WorldError::ChunkError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ChunkError> for WorldError {
    fn from(err: ChunkError) -> Self {
        WorldError::ChunkError(err)
    }
}