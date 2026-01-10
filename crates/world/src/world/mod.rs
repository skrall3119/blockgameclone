//! World integration module for managing multiple chunks in a unified world system
//! 
//! This module provides the core World struct and related types for coordinating
//! multiple chunks, handling spatial positioning, and managing world-scale operations.

pub mod error;
pub mod config;
pub mod coordinate;
pub mod performance;
pub mod memory;

// Re-export core types for convenience
pub use error::{WorldError, WorldResult};
pub use config::{WorldConfig, LoadPattern};
pub use coordinate::{ChunkCoord, CoordinateSystem};
pub use performance::{PerformanceMonitor, MemorySample, ChunkStats, MonitorConfig};
pub use memory::{MemoryManager};

use crate::chunk::Chunk;
use std::collections::HashMap;
use std::time::Instant;

/// Represents the state of a chunk within the world system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkState {
    /// Chunk is currently being loaded
    Loading,
    /// Chunk terrain has been generated
    Generated,
    /// Chunk has been meshed for rendering
    Meshed,
    /// Chunk is ready for rendering
    RenderReady,
    /// Chunk encountered an error during processing
    Error,
}

/// Container for a chunk with its associated metadata
#[derive(Debug)]
pub struct ChunkEntry {
    /// The actual chunk data
    pub chunk: Chunk,
    /// Current state of the chunk
    pub state: ChunkState,
    /// Last time this chunk was accessed
    pub last_accessed: Instant,
    /// Whether the chunk mesh needs to be regenerated
    pub mesh_dirty: bool,
}

impl ChunkEntry {
    /// Create a new chunk entry with the given chunk
    pub fn new(chunk: Chunk) -> Self {
        Self {
            chunk,
            state: ChunkState::Generated,
            last_accessed: Instant::now(),
            mesh_dirty: true,
        }
    }

    /// Update the chunk state
    pub fn set_state(&mut self, state: ChunkState) {
        self.state = state;
        self.last_accessed = Instant::now();
    }

    /// Mark the chunk as accessed
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    /// Mark the chunk mesh as dirty
    pub fn mark_mesh_dirty(&mut self) {
        self.mesh_dirty = true;
        if self.state == ChunkState::RenderReady {
            self.state = ChunkState::Meshed;
        }
    }

    /// Clear the mesh dirty flag
    pub fn clear_mesh_dirty(&mut self) {
        self.mesh_dirty = false;
    }
}

/// The main World struct that manages multiple chunks in a unified system
#[derive(Debug)]
pub struct World {
    /// Storage for all loaded chunks indexed by their coordinates
    chunks: HashMap<ChunkCoord, ChunkEntry>,
    /// World configuration parameters
    config: WorldConfig,
    /// Size of each chunk in blocks
    chunk_size: u32,
    /// Performance monitoring system
    performance_monitor: PerformanceMonitor,
    /// Memory management system
    memory_manager: MemoryManager,
    /// Coordinate system for transformations
    coordinate_system: CoordinateSystem,
}

impl World {
    /// Create a new world with the given configuration
    pub fn new(config: WorldConfig) -> WorldResult<Self> {
        let chunk_size = 32; // Default chunk size, could be configurable
        let coordinate_system = CoordinateSystem::new(chunk_size);
        let performance_monitor = PerformanceMonitor::new(MonitorConfig::default());
        let memory_manager = MemoryManager::new(config.max_chunks_loaded.unwrap_or(1000) * 1024 * 1024); // Default 1GB

        Ok(Self {
            chunks: HashMap::new(),
            config,
            chunk_size,
            performance_monitor,
            memory_manager,
            coordinate_system,
        })
    }

    /// Get the number of loaded chunks
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Check if a chunk is loaded at the given coordinates
    pub fn is_chunk_loaded(&self, coord: ChunkCoord) -> bool {
        self.chunks.contains_key(&coord)
    }

    /// Get a reference to a chunk at the given coordinates
    pub fn get_chunk(&self, coord: ChunkCoord) -> Option<&ChunkEntry> {
        self.chunks.get(&coord)
    }

    /// Get a mutable reference to a chunk at the given coordinates
    pub fn get_chunk_mut(&mut self, coord: ChunkCoord) -> Option<&mut ChunkEntry> {
        if let Some(entry) = self.chunks.get_mut(&coord) {
            entry.touch();
            Some(entry)
        } else {
            None
        }
    }

    /// Add a chunk to the world at the given coordinates
    pub fn add_chunk(&mut self, coord: ChunkCoord, chunk: Chunk) -> WorldResult<()> {
        let entry = ChunkEntry::new(chunk);
        self.chunks.insert(coord, entry);
        Ok(())
    }

    /// Remove a chunk from the world
    pub fn remove_chunk(&mut self, coord: ChunkCoord) -> Option<ChunkEntry> {
        self.chunks.remove(&coord)
    }

    /// Get the world configuration
    pub fn config(&self) -> &WorldConfig {
        &self.config
    }

    /// Get the coordinate system
    pub fn coordinate_system(&self) -> &CoordinateSystem {
        &self.coordinate_system
    }

    /// Get the performance monitor
    pub fn performance_monitor(&self) -> &PerformanceMonitor {
        &self.performance_monitor
    }

    /// Get a mutable reference to the performance monitor
    pub fn performance_monitor_mut(&mut self) -> &mut PerformanceMonitor {
        &mut self.performance_monitor
    }
}

// Constants for world management
pub const DEFAULT_CHUNK_SIZE: u32 = 32;
pub const DEFAULT_RENDER_DISTANCE: u32 = 8;
pub const MAX_CHUNKS_PER_FRAME: usize = 4;
pub const CHUNK_UNLOAD_DELAY_SECONDS: u64 = 30;