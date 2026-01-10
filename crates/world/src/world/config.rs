//! World configuration types and validation

use super::{ChunkCoord, WorldError, WorldResult};
use std::time::Duration;

/// Configuration for world behavior and parameters
#[derive(Debug, Clone)]
pub struct WorldConfig {
    /// Maximum distance from player where chunks are loaded and rendered
    pub render_distance: u32,
    /// Pattern for initial chunk loading
    pub initial_load_pattern: LoadPattern,
    /// Maximum number of chunks to keep loaded (None = unlimited)
    pub max_chunks_loaded: Option<usize>,
    /// Delay before unloading unused chunks
    pub chunk_unload_delay: Duration,
    /// Whether to enable performance monitoring
    pub performance_monitoring: bool,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            render_distance: super::DEFAULT_RENDER_DISTANCE,
            initial_load_pattern: LoadPattern::Single(ChunkCoord::new(0, 0, 0)),
            max_chunks_loaded: Some(1000),
            chunk_unload_delay: Duration::from_secs(super::CHUNK_UNLOAD_DELAY_SECONDS),
            performance_monitoring: true,
        }
    }
}

impl WorldConfig {
    /// Create a new world configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the render distance
    pub fn with_render_distance(mut self, distance: u32) -> Self {
        self.render_distance = distance;
        self
    }

    /// Set the initial loading pattern
    pub fn with_initial_load_pattern(mut self, pattern: LoadPattern) -> Self {
        self.initial_load_pattern = pattern;
        self
    }

    /// Set the maximum number of loaded chunks
    pub fn with_max_chunks(mut self, max_chunks: Option<usize>) -> Self {
        self.max_chunks_loaded = max_chunks;
        self
    }

    /// Set the chunk unload delay
    pub fn with_unload_delay(mut self, delay: Duration) -> Self {
        self.chunk_unload_delay = delay;
        self
    }

    /// Enable or disable performance monitoring
    pub fn with_performance_monitoring(mut self, enabled: bool) -> Self {
        self.performance_monitoring = enabled;
        self
    }

    /// Validate the configuration parameters
    pub fn validate(&self) -> WorldResult<()> {
        if self.render_distance == 0 {
            return Err(WorldError::InvalidConfiguration {
                parameter: "render_distance".to_string(),
                value: self.render_distance.to_string(),
                reason: "Render distance must be greater than 0".to_string(),
            });
        }

        if self.render_distance > 64 {
            return Err(WorldError::InvalidConfiguration {
                parameter: "render_distance".to_string(),
                value: self.render_distance.to_string(),
                reason: "Render distance should not exceed 64 for performance reasons".to_string(),
            });
        }

        if let Some(max_chunks) = self.max_chunks_loaded {
            if max_chunks == 0 {
                return Err(WorldError::InvalidConfiguration {
                    parameter: "max_chunks_loaded".to_string(),
                    value: max_chunks.to_string(),
                    reason: "Maximum chunks must be greater than 0 if specified".to_string(),
                });
            }
        }

        if self.chunk_unload_delay.as_secs() > 3600 {
            return Err(WorldError::InvalidConfiguration {
                parameter: "chunk_unload_delay".to_string(),
                value: format!("{:?}", self.chunk_unload_delay),
                reason: "Chunk unload delay should not exceed 1 hour".to_string(),
            });
        }

        Ok(())
    }
}

/// Patterns for loading chunks in the world
#[derive(Debug, Clone, PartialEq)]
pub enum LoadPattern {
    /// Load a single chunk at the specified coordinates
    Single(ChunkCoord),
    /// Load chunks in a grid pattern around a center point
    Grid {
        /// Center of the grid
        center: ChunkCoord,
        /// Radius of the grid (in chunks)
        radius: u32,
    },
    /// Load chunks at custom specified coordinates
    Custom(Vec<ChunkCoord>),
}

impl LoadPattern {
    /// Get all chunk coordinates that should be loaded for this pattern
    pub fn get_coordinates(&self) -> Vec<ChunkCoord> {
        match self {
            LoadPattern::Single(coord) => vec![*coord],
            LoadPattern::Grid { center, radius } => {
                let mut coords = Vec::new();
                let r = *radius as i32;
                
                for x in -r..=r {
                    for y in -r..=r {
                        for z in -r..=r {
                            coords.push(ChunkCoord::new(
                                center.x + x,
                                center.y + y,
                                center.z + z,
                            ));
                        }
                    }
                }
                
                coords
            }
            LoadPattern::Custom(coords) => coords.clone(),
        }
    }

    /// Get the estimated number of chunks this pattern will load
    pub fn estimated_chunk_count(&self) -> usize {
        match self {
            LoadPattern::Single(_) => 1,
            LoadPattern::Grid { radius, .. } => {
                let diameter = (radius * 2 + 1) as usize;
                diameter * diameter * diameter
            }
            LoadPattern::Custom(coords) => coords.len(),
        }
    }
}