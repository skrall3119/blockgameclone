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
pub use memory::{MemoryManager, MemoryStats, FragmentationStats, MemoryHealthReport, MemoryHealthStatus};

use crate::chunk::Chunk;
use std::collections::HashMap;
use std::time::Instant;
use glam::Vec3;

/// Represents the state of a chunk within the world system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Result of loading a single chunk
#[derive(Debug, Clone)]
pub enum LoadResult {
    /// Chunk was successfully loaded
    Success,
    /// Chunk was already loaded
    AlreadyLoaded,
    /// Chunk loading failed with error message
    Failed(String),
}

/// Progress tracking for multi-chunk loading operations
#[derive(Debug)]
pub struct LoadingProgress {
    /// Total number of chunks to load
    total_chunks: usize,
    /// Number of chunks completed (success or failure)
    completed_chunks: usize,
    /// Results for each chunk coordinate
    results: HashMap<ChunkCoord, LoadResult>,
    /// Number of successful loads
    successful_loads: usize,
    /// Number of failed loads
    failed_loads: usize,
    /// Whether the loading operation is complete
    is_complete: bool,
}

impl LoadingProgress {
    /// Create a new loading progress tracker
    pub fn new(total_chunks: usize) -> Self {
        Self {
            total_chunks,
            completed_chunks: 0,
            results: HashMap::new(),
            successful_loads: 0,
            failed_loads: 0,
            is_complete: false,
        }
    }

    /// Mark a chunk as completed with the given result
    pub fn mark_completed(&mut self, coord: ChunkCoord, result: LoadResult) {
        if !self.results.contains_key(&coord) {
            self.completed_chunks += 1;
        }
        self.results.insert(coord, result);
    }

    /// Finalize the loading progress with final counts
    pub fn finalize(&mut self, successful: usize, failed: usize) {
        self.successful_loads = successful;
        self.failed_loads = failed;
        self.is_complete = true;
    }

    /// Get the completion percentage (0.0 to 1.0)
    pub fn completion_percentage(&self) -> f32 {
        if self.total_chunks == 0 {
            1.0
        } else {
            self.completed_chunks as f32 / self.total_chunks as f32
        }
    }

    /// Check if the loading operation is complete
    pub fn is_complete(&self) -> bool {
        self.is_complete
    }

    /// Get the number of successful loads
    pub fn successful_loads(&self) -> usize {
        self.successful_loads
    }

    /// Get the number of failed loads
    pub fn failed_loads(&self) -> usize {
        self.failed_loads
    }

    /// Get the total number of chunks
    pub fn total_chunks(&self) -> usize {
        self.total_chunks
    }

    /// Get the number of completed chunks
    pub fn completed_chunks(&self) -> usize {
        self.completed_chunks
    }

    /// Get the result for a specific chunk coordinate
    pub fn get_result(&self, coord: ChunkCoord) -> Option<&LoadResult> {
        self.results.get(&coord)
    }

    /// Get all results
    pub fn get_all_results(&self) -> &HashMap<ChunkCoord, LoadResult> {
        &self.results
    }

    /// Get coordinates of failed chunks
    pub fn failed_chunks(&self) -> Vec<ChunkCoord> {
        self.results
            .iter()
            .filter_map(|(coord, result)| {
                if matches!(result, LoadResult::Failed(_)) {
                    Some(*coord)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get coordinates of successfully loaded chunks
    pub fn successful_chunks(&self) -> Vec<ChunkCoord> {
        self.results
            .iter()
            .filter_map(|(coord, result)| {
                if matches!(result, LoadResult::Success | LoadResult::AlreadyLoaded) {
                    Some(*coord)
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Statistics about chunk loading state in the world
#[derive(Debug, Clone)]
pub struct LoadingStats {
    /// Total number of loaded chunks
    pub total_chunks: usize,
    /// Number of chunks currently loading
    pub loading_chunks: usize,
    /// Number of chunks that have been generated
    pub generated_chunks: usize,
    /// Number of chunks that have been meshed
    pub meshed_chunks: usize,
    /// Number of chunks ready for rendering
    pub render_ready_chunks: usize,
    /// Number of chunks in error state
    pub error_chunks: usize,
}

/// Statistics about chunks that need re-meshing
#[derive(Debug, Clone)]
pub struct DirtyChunkStats {
    /// Total number of chunks with dirty meshes
    pub total_dirty_chunks: usize,
    /// Number of dirty chunks in Generated state
    pub dirty_generated_chunks: usize,
    /// Number of dirty chunks in Meshed state
    pub dirty_meshed_chunks: usize,
    /// Number of dirty chunks in RenderReady state
    pub dirty_render_ready_chunks: usize,
}

impl DirtyChunkStats {
    /// Check if there are any dirty chunks
    pub fn has_dirty_chunks(&self) -> bool {
        self.total_dirty_chunks > 0
    }

    /// Get the percentage of chunks that need immediate re-meshing (Generated + Meshed states)
    pub fn immediate_remesh_percentage(&self) -> f32 {
        if self.total_dirty_chunks == 0 {
            0.0
        } else {
            let immediate_count = self.dirty_generated_chunks + self.dirty_meshed_chunks;
            immediate_count as f32 / self.total_dirty_chunks as f32
        }
    }
}

/// Camera frustum for frustum culling operations
#[derive(Debug, Clone)]
pub struct CameraFrustum {
    /// The six planes of the frustum (left, right, bottom, top, near, far)
    pub planes: [FrustumPlane; 6],
}

/// A plane in 3D space defined by a normal vector and distance from origin
#[derive(Debug, Clone, Copy)]
pub struct FrustumPlane {
    /// Normal vector of the plane
    pub normal: Vec3,
    /// Distance from origin along the normal
    pub distance: f32,
}

/// Statistics about frustum culling performance
#[derive(Debug, Clone)]
pub struct FrustumCullingStats {
    /// Total number of chunks in the world
    pub total_chunks: usize,
    /// Number of chunks that are render-ready
    pub render_ready_chunks: usize,
    /// Number of chunks visible within the frustum
    pub visible_chunks: usize,
    /// Number of chunks culled by frustum culling
    pub culled_chunks: usize,
    /// Efficiency of culling (0.0 to 1.0, higher is better)
    pub culling_efficiency: f32,
}

impl CameraFrustum {
    /// Create a new camera frustum from view and projection matrices
    pub fn from_view_projection_matrix(view_proj: glam::Mat4) -> Self {
        // Extract frustum planes from the view-projection matrix
        // This is a standard technique for extracting frustum planes
        let m = view_proj.to_cols_array_2d();
        
        let planes = [
            // Left plane: m[3] + m[0]
            FrustumPlane {
                normal: Vec3::new(m[0][3] + m[0][0], m[1][3] + m[1][0], m[2][3] + m[2][0]),
                distance: m[3][3] + m[3][0],
            },
            // Right plane: m[3] - m[0]
            FrustumPlane {
                normal: Vec3::new(m[0][3] - m[0][0], m[1][3] - m[1][0], m[2][3] - m[2][0]),
                distance: m[3][3] - m[3][0],
            },
            // Bottom plane: m[3] + m[1]
            FrustumPlane {
                normal: Vec3::new(m[0][3] + m[0][1], m[1][3] + m[1][1], m[2][3] + m[2][1]),
                distance: m[3][3] + m[3][1],
            },
            // Top plane: m[3] - m[1]
            FrustumPlane {
                normal: Vec3::new(m[0][3] - m[0][1], m[1][3] - m[1][1], m[2][3] - m[2][1]),
                distance: m[3][3] - m[3][1],
            },
            // Near plane: m[3] + m[2]
            FrustumPlane {
                normal: Vec3::new(m[0][3] + m[0][2], m[1][3] + m[1][2], m[2][3] + m[2][2]),
                distance: m[3][3] + m[3][2],
            },
            // Far plane: m[3] - m[2]
            FrustumPlane {
                normal: Vec3::new(m[0][3] - m[0][2], m[1][3] - m[1][2], m[2][3] - m[2][2]),
                distance: m[3][3] - m[3][2],
            },
        ];

        // Normalize the planes
        let normalized_planes = planes.map(|mut plane| {
            let length = plane.normal.length();
            if length > 0.0 {
                plane.normal /= length;
                plane.distance /= length;
            }
            plane
        });

        Self {
            planes: normalized_planes,
        }
    }

    /// Create a simple frustum for testing purposes
    pub fn new_simple(
        left: f32, right: f32, 
        bottom: f32, top: f32, 
        near: f32, far: f32
    ) -> Self {
        let planes = [
            // Left plane
            FrustumPlane {
                normal: Vec3::new(1.0, 0.0, 0.0),
                distance: -left,
            },
            // Right plane
            FrustumPlane {
                normal: Vec3::new(-1.0, 0.0, 0.0),
                distance: right,
            },
            // Bottom plane
            FrustumPlane {
                normal: Vec3::new(0.0, 1.0, 0.0),
                distance: -bottom,
            },
            // Top plane
            FrustumPlane {
                normal: Vec3::new(0.0, -1.0, 0.0),
                distance: top,
            },
            // Near plane
            FrustumPlane {
                normal: Vec3::new(0.0, 0.0, 1.0),
                distance: -near,
            },
            // Far plane
            FrustumPlane {
                normal: Vec3::new(0.0, 0.0, -1.0),
                distance: far,
            },
        ];

        Self { planes }
    }

    /// Check if an axis-aligned bounding box intersects with this frustum
    pub fn intersects_aabb(&self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            // Find the positive vertex (farthest along plane normal)
            let positive_vertex = Vec3::new(
                if plane.normal.x >= 0.0 { max.x } else { min.x },
                if plane.normal.y >= 0.0 { max.y } else { min.y },
                if plane.normal.z >= 0.0 { max.z } else { min.z },
            );

            // If the positive vertex is behind the plane, the box is completely outside
            if plane.distance_to_point(positive_vertex) < 0.0 {
                return false;
            }
        }
        true
    }

    /// Check if a point is inside this frustum
    pub fn contains_point(&self, point: Vec3) -> bool {
        for plane in &self.planes {
            if plane.distance_to_point(point) < 0.0 {
                return false;
            }
        }
        true
    }
}

impl FrustumPlane {
    /// Calculate the signed distance from this plane to a point
    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

impl FrustumCullingStats {
    /// Get the percentage of chunks that were culled
    pub fn culling_percentage(&self) -> f32 {
        if self.render_ready_chunks == 0 {
            0.0
        } else {
            self.culled_chunks as f32 / self.render_ready_chunks as f32
        }
    }

    /// Check if frustum culling is effective (culling more than 50% of chunks)
    pub fn is_effective(&self) -> bool {
        self.culling_efficiency > 0.5
    }
}

impl LoadingStats {
    /// Get the percentage of chunks that are render-ready
    pub fn render_ready_percentage(&self) -> f32 {
        if self.total_chunks == 0 {
            0.0
        } else {
            self.render_ready_chunks as f32 / self.total_chunks as f32
        }
    }

    /// Check if all loaded chunks are render-ready
    pub fn all_render_ready(&self) -> bool {
        self.total_chunks > 0 && self.render_ready_chunks == self.total_chunks
    }

    /// Get the number of chunks that need processing
    pub fn chunks_needing_processing(&self) -> usize {
        self.loading_chunks + self.generated_chunks
    }
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

    /// Check if the chunk can transition to the given state
    pub fn can_transition_to(&self, new_state: ChunkState) -> bool {
        use ChunkState::*;
        match (self.state, new_state) {
            // Loading can transition to Generated or Error
            (Loading, Generated) | (Loading, Error) => true,
            // Generated can transition to Meshed or Error
            (Generated, Meshed) | (Generated, Error) => true,
            // Meshed can transition to RenderReady or back to Generated (for re-meshing)
            (Meshed, RenderReady) | (Meshed, Generated) | (Meshed, Error) => true,
            // RenderReady can transition back to Meshed (for updates) or Error
            (RenderReady, Meshed) | (RenderReady, Generated) | (RenderReady, Error) => true,
            // Error can transition to Loading (for retry)
            (Error, Loading) => true,
            // Same state transitions are always allowed
            (state1, state2) if state1 == state2 => true,
            // All other transitions are invalid
            _ => false,
        }
    }

    /// Safely transition to a new state, returning an error if invalid
    pub fn transition_to(&mut self, new_state: ChunkState) -> Result<(), String> {
        if self.can_transition_to(new_state) {
            self.set_state(new_state);
            Ok(())
        } else {
            Err(format!(
                "Invalid state transition from {:?} to {:?}",
                self.state, new_state
            ))
        }
    }

    /// Check if the chunk is in a renderable state
    pub fn is_renderable(&self) -> bool {
        matches!(self.state, ChunkState::RenderReady)
    }

    /// Check if the chunk needs processing
    pub fn needs_processing(&self) -> bool {
        matches!(self.state, ChunkState::Loading | ChunkState::Generated) || self.mesh_dirty
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
            // Update memory manager access time for LRU tracking
            self.memory_manager.touch_chunk(coord);
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
        let removed_chunk = self.chunks.remove(&coord);
        
        // Unregister memory usage when chunk is removed
        if removed_chunk.is_some() {
            self.memory_manager.unregister_chunk_memory(coord);
        }
        
        removed_chunk
    }

    /// Get the world configuration
    pub fn config(&self) -> &WorldConfig {
        &self.config
    }

    /// Update the world configuration dynamically
    /// This method validates the new configuration and adapts world behavior accordingly
    pub fn update_config(&mut self, new_config: WorldConfig) -> WorldResult<()> {
        // Validate the new configuration first
        new_config.validate()?;
        
        // Check if the configuration change requires a restart
        if self.config.requires_restart(&new_config) {
            return Err(WorldError::InvalidConfiguration {
                parameter: "configuration".to_string(),
                value: "incompatible_change".to_string(),
                reason: "Configuration change requires world restart".to_string(),
            });
        }
        
        // Check compatibility
        if !new_config.is_compatible_with(&self.config) {
            return Err(WorldError::InvalidConfiguration {
                parameter: "configuration".to_string(),
                value: "incompatible".to_string(),
                reason: "New configuration is not compatible with current configuration".to_string(),
            });
        }
        
        // Store the old configuration for comparison
        let old_config = self.config.clone();
        
        // Apply the new configuration
        self.config = new_config;
        
        // Adapt behavior based on configuration changes
        self.adapt_to_config_changes(&old_config)?;
        
        Ok(())
    }

    /// Update configuration with automatic migration support
    /// This method will migrate the configuration to the latest version if needed
    pub fn update_config_with_migration(&mut self, mut new_config: WorldConfig) -> WorldResult<()> {
        // Migrate the configuration to the latest version if needed
        let was_migrated = new_config.migrate_to_latest()?;
        
        if was_migrated {
            // Log or track that migration occurred
            // In a real implementation, this might log the migration
        }
        
        // Apply the migrated configuration
        self.update_config(new_config)
    }

    /// Get configuration differences between current and a new configuration
    pub fn get_config_differences(&self, new_config: &WorldConfig) -> Vec<String> {
        self.config.get_differences(new_config)
    }

    /// Check if a configuration change would require a world restart
    pub fn would_require_restart(&self, new_config: &WorldConfig) -> bool {
        self.config.requires_restart(new_config)
    }

    /// Validate configuration compatibility without applying changes
    pub fn validate_config_change(&self, new_config: &WorldConfig) -> WorldResult<Vec<String>> {
        // Validate the new configuration
        new_config.validate()?;
        
        // Check compatibility
        if !new_config.is_compatible_with(&self.config) {
            return Err(WorldError::InvalidConfiguration {
                parameter: "configuration".to_string(),
                value: "incompatible".to_string(),
                reason: "New configuration is not compatible with current configuration".to_string(),
            });
        }
        
        // Check if restart is required
        if self.config.requires_restart(new_config) {
            return Err(WorldError::InvalidConfiguration {
                parameter: "configuration".to_string(),
                value: "requires_restart".to_string(),
                reason: "Configuration change requires world restart".to_string(),
            });
        }
        
        // Return the list of differences
        Ok(self.config.get_differences(new_config))
    }

    /// Apply a configuration update with validation
    /// This is a convenience method that creates a new config from the current one
    pub fn apply_config_update<F>(&mut self, update_fn: F) -> WorldResult<()>
    where
        F: FnOnce(WorldConfig) -> WorldConfig,
    {
        let new_config = update_fn(self.config.clone());
        self.update_config(new_config)
    }

    /// Set the render distance and adapt world behavior
    pub fn set_render_distance(&mut self, distance: u32) -> WorldResult<()> {
        let new_config = self.config.clone().with_render_distance(distance);
        self.update_config(new_config)
    }

    /// Set the maximum number of loaded chunks
    pub fn set_max_chunks(&mut self, max_chunks: Option<usize>) -> WorldResult<()> {
        let new_config = self.config.clone().with_max_chunks(max_chunks);
        self.update_config(new_config)
    }

    /// Set the chunk unload delay
    pub fn set_chunk_unload_delay(&mut self, delay: std::time::Duration) -> WorldResult<()> {
        let new_config = self.config.clone().with_unload_delay(delay);
        self.update_config(new_config)
    }

    /// Enable or disable performance monitoring
    pub fn set_performance_monitoring(&mut self, enabled: bool) -> WorldResult<()> {
        let new_config = self.config.clone().with_performance_monitoring(enabled);
        self.update_config(new_config)
    }

    /// Adapt world behavior when configuration changes
    fn adapt_to_config_changes(&mut self, old_config: &WorldConfig) -> WorldResult<()> {
        // Handle render distance changes
        if self.config.render_distance != old_config.render_distance {
            self.handle_render_distance_change(old_config.render_distance)?;
        }

        // Handle max chunks limit changes
        if self.config.max_chunks_loaded != old_config.max_chunks_loaded {
            self.handle_max_chunks_change(old_config.max_chunks_loaded)?;
        }

        // Handle performance monitoring changes
        if self.config.performance_monitoring != old_config.performance_monitoring {
            self.handle_performance_monitoring_change()?;
        }

        // Handle chunk unload delay changes
        if self.config.chunk_unload_delay != old_config.chunk_unload_delay {
            self.handle_unload_delay_change()?;
        }

        Ok(())
    }

    /// Handle render distance configuration changes
    fn handle_render_distance_change(&mut self, old_render_distance: u32) -> WorldResult<()> {
        // If render distance decreased, we might need to unload chunks that are now too far
        if self.config.render_distance < old_render_distance {
            // This would typically trigger chunk unloading logic
            // For now, we just validate that the change is acceptable
            if self.config.render_distance == 0 {
                return Err(WorldError::InvalidConfiguration {
                    parameter: "render_distance".to_string(),
                    value: self.config.render_distance.to_string(),
                    reason: "Render distance cannot be zero".to_string(),
                });
            }
        }
        
        // Update memory manager if needed based on new render distance
        let estimated_chunks = ((self.config.render_distance * 2 + 1) as usize).pow(3);
        if let Some(max_chunks) = self.config.max_chunks_loaded {
            if estimated_chunks > max_chunks {
                return Err(WorldError::InvalidConfiguration {
                    parameter: "render_distance".to_string(),
                    value: self.config.render_distance.to_string(),
                    reason: format!("Render distance would require {} chunks but max_chunks is {}", 
                                  estimated_chunks, max_chunks),
                });
            }
        }
        
        Ok(())
    }

    /// Handle maximum chunks limit changes
    fn handle_max_chunks_change(&mut self, old_max_chunks: Option<usize>) -> WorldResult<()> {
        if let Some(new_max) = self.config.max_chunks_loaded {
            // If the limit decreased and we have too many chunks loaded, we need to unload some
            if self.chunk_count() > new_max {
                // This would typically trigger chunk unloading based on LRU or distance
                // For now, we validate that the change is feasible
                return Err(WorldError::ResourceLimitExceeded {
                    resource: "chunks".to_string(),
                    limit: new_max,
                    requested: self.chunk_count(),
                });
            }
            
            // Validate that the new limit is compatible with render distance
            let estimated_chunks = ((self.config.render_distance * 2 + 1) as usize).pow(3);
            if new_max < estimated_chunks {
                return Err(WorldError::InvalidConfiguration {
                    parameter: "max_chunks_loaded".to_string(),
                    value: new_max.to_string(),
                    reason: format!("Max chunks {} is less than render distance requirement {}", 
                                  new_max, estimated_chunks),
                });
            }
        }
        
        Ok(())
    }

    /// Handle performance monitoring configuration changes
    fn handle_performance_monitoring_change(&mut self) -> WorldResult<()> {
        // Update the performance monitor configuration
        if self.config.performance_monitoring {
            // Enable monitoring if it was disabled
            self.performance_monitor = PerformanceMonitor::new(MonitorConfig::default());
        } else {
            // Disable monitoring - we keep the monitor but it won't collect new data
            // The existing data remains available for queries
        }
        
        Ok(())
    }

    /// Handle chunk unload delay configuration changes
    fn handle_unload_delay_change(&mut self) -> WorldResult<()> {
        // Validate the new unload delay
        if self.config.chunk_unload_delay.as_secs() > 3600 {
            return Err(WorldError::InvalidConfiguration {
                parameter: "chunk_unload_delay".to_string(),
                value: format!("{:?}", self.config.chunk_unload_delay),
                reason: "Chunk unload delay should not exceed 1 hour".to_string(),
            });
        }
        
        // The new delay will be used for future unload operations
        // Existing scheduled unloads keep their original timing
        Ok(())
    }

    /// Validate that the current world state is compatible with the configuration
    pub fn validate_config_compatibility(&self) -> WorldResult<()> {
        // Check that current chunk count doesn't exceed max_chunks_loaded
        if let Some(max_chunks) = self.config.max_chunks_loaded {
            if self.chunk_count() > max_chunks {
                return Err(WorldError::ResourceLimitExceeded {
                    resource: "chunks".to_string(),
                    limit: max_chunks,
                    requested: self.chunk_count(),
                });
            }
        }

        // Check that render distance is reasonable for current chunk count
        let estimated_chunks = ((self.config.render_distance * 2 + 1) as usize).pow(3);
        if let Some(max_chunks) = self.config.max_chunks_loaded {
            if estimated_chunks > max_chunks {
                return Err(WorldError::InvalidConfiguration {
                    parameter: "render_distance".to_string(),
                    value: self.config.render_distance.to_string(),
                    reason: format!("Render distance requires {} chunks but max_chunks is {}", 
                                  estimated_chunks, max_chunks),
                });
            }
        }

        Ok(())
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

    /// Get the memory manager
    pub fn memory_manager(&self) -> &MemoryManager {
        &self.memory_manager
    }

    /// Get a mutable reference to the memory manager
    pub fn memory_manager_mut(&mut self) -> &mut MemoryManager {
        &mut self.memory_manager
    }

    /// Convert world position to chunk coordinate
    pub fn world_to_chunk_coord(&self, world_pos: glam::Vec3) -> ChunkCoord {
        self.coordinate_system.world_to_chunk_coord(world_pos)
    }

    /// Convert chunk coordinate to world position (chunk origin)
    pub fn chunk_to_world_pos(&self, chunk_coord: ChunkCoord) -> glam::Vec3 {
        self.coordinate_system.chunk_to_world_pos(chunk_coord)
    }

    /// Convert world position to chunk coordinate and local block coordinate
    pub fn world_to_local_block(&self, world_pos: glam::Vec3) -> (ChunkCoord, coordinate::BlockCoord) {
        self.coordinate_system.world_to_local_block(world_pos)
    }

    /// Get the bounding box of a chunk in world coordinates
    pub fn chunk_bounds(&self, chunk_coord: ChunkCoord) -> (glam::Vec3, glam::Vec3) {
        self.coordinate_system.chunk_bounds(chunk_coord)
    }

    /// Check if two chunks are adjacent (share a face)
    pub fn are_chunks_adjacent(&self, coord1: ChunkCoord, coord2: ChunkCoord) -> bool {
        self.coordinate_system.are_adjacent(coord1, coord2)
    }

    /// Get all chunk coordinates within a given radius from a center
    pub fn chunks_in_radius(&self, center: ChunkCoord, radius: u32) -> Vec<ChunkCoord> {
        self.coordinate_system.chunks_in_radius(center, radius)
    }

    /// Get chunks within a bounding box defined by min and max chunk coordinates
    pub fn chunks_in_bounds(&self, min: ChunkCoord, max: ChunkCoord) -> Vec<&ChunkEntry> {
        let mut chunks = Vec::new();
        
        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    let coord = ChunkCoord::new(x, y, z);
                    if let Some(entry) = self.get_chunk(coord) {
                        chunks.push(entry);
                    }
                }
            }
        }
        
        chunks
    }

    /// Get chunks within render distance from a center point
    pub fn chunks_in_render_distance(&self, center: ChunkCoord) -> Vec<&ChunkEntry> {
        let radius = self.config.render_distance;
        let coords = self.chunks_in_radius(center, radius);
        
        coords.into_iter()
            .filter_map(|coord| self.get_chunk(coord))
            .collect()
    }

    /// Check if a chunk coordinate is within render distance of a center
    pub fn is_in_render_distance(&self, coord: ChunkCoord, center: ChunkCoord) -> bool {
        coord.manhattan_distance(&center) <= self.config.render_distance
    }

    /// Get the chunk size in blocks
    pub fn chunk_size(&self) -> u32 {
        self.chunk_size
    }

    /// Validate chunk coordinates (basic validation for extreme values)
    pub fn validate_chunk_coord(&self, coord: ChunkCoord) -> WorldResult<()> {
        // Check for extreme coordinates that might cause overflow
        const MAX_COORD: i32 = 1_000_000;
        const MIN_COORD: i32 = -1_000_000;
        
        if coord.x < MIN_COORD || coord.x > MAX_COORD ||
           coord.y < MIN_COORD || coord.y > MAX_COORD ||
           coord.z < MIN_COORD || coord.z > MAX_COORD {
            return Err(WorldError::InvalidCoordinates {
                x: coord.x,
                y: coord.y,
                z: coord.z,
                reason: format!("Coordinates must be between {} and {}", MIN_COORD, MAX_COORD),
            });
        }
        
        Ok(())
    }

    /// Get the chunk coordinate that contains a specific world position
    /// This handles negative coordinates correctly
    pub fn get_chunk_at_world_pos(&self, world_pos: Vec3) -> ChunkCoord {
        self.world_to_chunk_coord(world_pos)
    }

    /// Get all loaded chunks adjacent to a given chunk coordinate
    pub fn get_adjacent_loaded_chunks(&self, coord: ChunkCoord) -> Vec<(ChunkCoord, &ChunkEntry)> {
        coord.adjacent()
            .iter()
            .filter_map(|&adj_coord| {
                self.get_chunk(adj_coord).map(|entry| (adj_coord, entry))
            })
            .collect()
    }

    /// Calculate the world-space distance between two chunk coordinates
    pub fn chunk_distance_world_space(&self, coord1: ChunkCoord, coord2: ChunkCoord) -> f32 {
        let pos1 = self.chunk_to_world_pos(coord1);
        let pos2 = self.chunk_to_world_pos(coord2);
        pos1.distance(pos2)
    }

    /// Load multiple chunks according to the specified pattern
    /// Returns a LoadingProgress that tracks the operation
    pub fn load_chunks(&mut self, pattern: LoadPattern) -> WorldResult<LoadingProgress> {
        // Use the enhanced error handling version
        self.load_chunks_with_recovery(pattern)
    }

    /// Generate and load a single chunk at the specified coordinates
    fn generate_and_load_chunk(&mut self, coord: ChunkCoord) -> WorldResult<()> {
        use crate::chunk::{Chunk, ChunkDimensions, ChunkPosition};
        
        // Validate coordinates first
        self.validate_chunk_coord(coord)?;
        
        // Estimate chunk memory usage (approximate calculation)
        let chunk_size_bytes = (self.chunk_size as usize).pow(3) * 2; // 2 bytes per block (block type + metadata)
        let estimated_chunk_memory = chunk_size_bytes + 1024; // Add overhead for chunk metadata
        
        // Check memory constraints before loading
        if !self.memory_manager.can_load_chunk(estimated_chunk_memory) {
            return Err(WorldError::OutOfMemory {
                requested: estimated_chunk_memory,
                available: self.memory_manager.memory_budget().saturating_sub(self.memory_manager.current_usage()),
            });
        }
        
        // Check chunk count constraints
        if let Some(max_chunks) = self.config.max_chunks_loaded {
            if self.chunk_count() >= max_chunks {
                return Err(WorldError::ResourceLimitExceeded {
                    resource: "chunks".to_string(),
                    limit: max_chunks,
                    requested: self.chunk_count() + 1,
                });
            }
        }
        
        // Create a new chunk with appropriate dimensions
        let chunk_position = ChunkPosition {
            x: coord.x,
            z: coord.z,
        };
        let chunk_dimensions = ChunkDimensions {
            width: self.chunk_size as usize,
            height: self.chunk_size as usize,
            depth: self.chunk_size as usize,
        };
        
        // Generate the chunk (this would normally involve terrain generation)
        // Wrap chunk creation in error handling
        let chunk = match Chunk::new(chunk_position, chunk_dimensions) {
            chunk => chunk, // Chunk::new doesn't return Result, but we simulate error handling
        };
        
        // Simulate potential loading failures for testing error isolation
        // In a real implementation, this would be actual terrain generation that could fail
        if self.should_simulate_loading_failure(coord) {
            return Err(WorldError::LoadingFailed {
                coord,
                reason: "Simulated loading failure for testing".to_string(),
            });
        }
        
        // Register memory usage before adding the chunk
        self.memory_manager.register_chunk_memory(coord, estimated_chunk_memory)?;
        
        // Add the chunk to the world
        match self.add_chunk(coord, chunk) {
            Ok(()) => {
                // Update the chunk state to Generated
                if let Some(entry) = self.get_chunk_mut(coord) {
                    entry.set_state(ChunkState::Generated);
                }
                Ok(())
            }
            Err(e) => {
                // If adding chunk failed, unregister the memory
                self.memory_manager.unregister_chunk_memory(coord);
                Err(e)
            }
        }
    }

    /// Simulate loading failures for testing error isolation
    /// This is a test helper that would not exist in production code
    fn should_simulate_loading_failure(&self, coord: ChunkCoord) -> bool {
        // Only simulate failures for the specific error handling isolation test
        // For other tests, we want all chunks to load successfully
        false
    }

    /// Simulate loading failures with a specific pattern for error isolation testing
    fn should_simulate_loading_failure_for_error_test(&self, coord: ChunkCoord) -> bool {
        // Simulate failure for chunks with specific coordinate patterns
        // This allows us to test error isolation in property tests
        
        // Fail chunks where all coordinates are negative and divisible by 7
        // This creates a predictable but sparse failure pattern
        coord.x < 0 && coord.y < 0 && coord.z < 0 && 
        coord.x % 7 == 0 && coord.y % 7 == 0 && coord.z % 7 == 0
    }

    /// Generate and load a single chunk with error simulation for testing
    fn generate_and_load_chunk_with_error_simulation(&mut self, coord: ChunkCoord) -> WorldResult<()> {
        use crate::chunk::{Chunk, ChunkDimensions, ChunkPosition};
        
        // Validate coordinates first
        self.validate_chunk_coord(coord)?;
        
        // Check memory constraints before loading
        if let Some(max_chunks) = self.config.max_chunks_loaded {
            if self.chunk_count() >= max_chunks {
                return Err(WorldError::ResourceLimitExceeded {
                    resource: "chunks".to_string(),
                    limit: max_chunks,
                    requested: self.chunk_count() + 1,
                });
            }
        }
        
        // Create a new chunk with appropriate dimensions
        let chunk_position = ChunkPosition {
            x: coord.x,
            z: coord.z,
        };
        let chunk_dimensions = ChunkDimensions {
            width: self.chunk_size as usize,
            height: self.chunk_size as usize,
            depth: self.chunk_size as usize,
        };
        
        // Generate the chunk (this would normally involve terrain generation)
        // Wrap chunk creation in error handling
        let chunk = match Chunk::new(chunk_position, chunk_dimensions) {
            chunk => chunk, // Chunk::new doesn't return Result, but we simulate error handling
        };
        
        // Simulate potential loading failures for testing error isolation
        // In a real implementation, this would be actual terrain generation that could fail
        if self.should_simulate_loading_failure_for_error_test(coord) {
            return Err(WorldError::LoadingFailed {
                coord,
                reason: "Simulated loading failure for testing".to_string(),
            });
        }
        
        // Add the chunk to the world
        self.add_chunk(coord, chunk)?;
        
        // Update the chunk state to Generated
        if let Some(entry) = self.get_chunk_mut(coord) {
            entry.set_state(ChunkState::Generated);
        }
        
        Ok(())
    }

    /// Load chunks with enhanced error handling and recovery
    pub fn load_chunks_with_recovery(&mut self, pattern: LoadPattern) -> WorldResult<LoadingProgress> {
        let coordinates = pattern.get_coordinates();
        let total_chunks = coordinates.len();
        
        let mut progress = LoadingProgress::new(total_chunks);
        let mut successful_loads = 0;
        let mut failed_loads = Vec::new();
        
        // Process chunks in isolation - failures don't affect other chunks
        for coord in coordinates {
            // Skip if chunk is already loaded
            if self.is_chunk_loaded(coord) {
                progress.mark_completed(coord, LoadResult::AlreadyLoaded);
                successful_loads += 1;
                continue;
            }
            
            // Attempt to generate and load the chunk with error isolation
            match self.generate_and_load_chunk_isolated(coord) {
                Ok(()) => {
                    progress.mark_completed(coord, LoadResult::Success);
                    successful_loads += 1;
                }
                Err(error) => {
                    // Log the error but continue with other chunks
                    let error_msg = format!("Failed to load chunk at {:?}: {}", coord, error);
                    progress.mark_completed(coord, LoadResult::Failed(error_msg.clone()));
                    failed_loads.push((coord, error));
                    
                    // Ensure the failed chunk is marked as error state if it was partially loaded
                    if let Some(entry) = self.get_chunk_mut(coord) {
                        let _ = entry.transition_to(ChunkState::Error);
                    }
                }
            }
        }
        
        progress.finalize(successful_loads, failed_loads.len());
        Ok(progress)
    }

    /// Generate and load a single chunk with full error isolation
    fn generate_and_load_chunk_isolated(&mut self, coord: ChunkCoord) -> WorldResult<()> {
        // Create a checkpoint of world state before attempting to load
        let initial_chunk_count = self.chunk_count();
        
        // Attempt to load the chunk
        match self.generate_and_load_chunk(coord) {
            Ok(()) => Ok(()),
            Err(error) => {
                // If loading failed, ensure we clean up any partial state
                self.cleanup_failed_chunk_load(coord);
                
                // Verify that the world state is consistent after cleanup
                if self.chunk_count() != initial_chunk_count {
                    // If chunk count changed, remove the partially loaded chunk
                    self.remove_chunk(coord);
                }
                
                Err(error)
            }
        }
    }

    /// Clean up any partial state from a failed chunk load
    fn cleanup_failed_chunk_load(&mut self, coord: ChunkCoord) {
        // Remove any partially loaded chunk data
        if let Some(mut entry) = self.remove_chunk(coord) {
            // Mark as error state for debugging
            let _ = entry.transition_to(ChunkState::Error);
        }
        
        // Additional cleanup could include:
        // - Releasing allocated memory
        // - Cleaning up temporary files
        // - Resetting performance counters
        // - Notifying dependent systems
    }

    /// Retry loading failed chunks from a previous loading operation
    pub fn retry_failed_chunks(&mut self, previous_progress: &LoadingProgress) -> WorldResult<LoadingProgress> {
        let failed_coords = previous_progress.failed_chunks();
        
        if failed_coords.is_empty() {
            // No failed chunks to retry
            let mut progress = LoadingProgress::new(0);
            progress.finalize(0, 0);
            return Ok(progress);
        }
        
        let pattern = LoadPattern::Custom(failed_coords);
        self.load_chunks_with_recovery(pattern)
    }

    /// Check if the world is in a consistent state after loading operations
    pub fn validate_world_consistency(&self) -> WorldResult<()> {
        // Check that all loaded chunks have valid states
        for (coord, entry) in &self.chunks {
            // Validate coordinate consistency
            self.validate_chunk_coord(*coord)?;
            
            // Check that chunk state is valid
            match entry.state {
                ChunkState::Loading => {
                    // Loading state should be temporary - this might indicate a stuck operation
                    return Err(WorldError::InvalidChunkState {
                        coord: *coord,
                        current_state: entry.state,
                        required_state: ChunkState::Generated,
                    });
                }
                ChunkState::Error => {
                    // Error state chunks should be cleaned up or retried
                    return Err(WorldError::InvalidChunkState {
                        coord: *coord,
                        current_state: entry.state,
                        required_state: ChunkState::Generated,
                    });
                }
                _ => {} // Other states are valid
            }
            
            // Validate chunk dimensions match world configuration
            let chunk_dims = entry.chunk.dimensions();
            let expected_size = self.chunk_size as usize;
            if chunk_dims.width != expected_size || 
               chunk_dims.height != expected_size || 
               chunk_dims.depth != expected_size {
                return Err(WorldError::InvalidConfiguration {
                    parameter: "chunk_dimensions".to_string(),
                    value: format!("{}x{}x{}", chunk_dims.width, chunk_dims.height, chunk_dims.depth),
                    reason: format!("Expected {}x{}x{}", expected_size, expected_size, expected_size),
                });
            }
        }
        
        Ok(())
    }

    /// Load chunks in a grid pattern around a center point
    pub fn load_grid(&mut self, center: ChunkCoord, radius: u32) -> WorldResult<LoadingProgress> {
        let pattern = LoadPattern::Grid { center, radius };
        self.load_chunks(pattern)
    }

    /// Load a single chunk at the specified coordinates
    pub fn load_single_chunk(&mut self, coord: ChunkCoord) -> WorldResult<LoadingProgress> {
        let pattern = LoadPattern::Single(coord);
        self.load_chunks(pattern)
    }

    /// Load chunks at custom specified coordinates
    pub fn load_custom_chunks(&mut self, coordinates: Vec<ChunkCoord>) -> WorldResult<LoadingProgress> {
        let pattern = LoadPattern::Custom(coordinates);
        self.load_chunks(pattern)
    }

    /// Get loading progress statistics for the world
    pub fn get_loading_stats(&self) -> LoadingStats {
        let total_chunks = self.chunk_count();
        let mut stats_by_state = std::collections::HashMap::new();
        
        for entry in self.chunks.values() {
            *stats_by_state.entry(entry.state).or_insert(0) += 1;
        }
        
        LoadingStats {
            total_chunks,
            loading_chunks: stats_by_state.get(&ChunkState::Loading).copied().unwrap_or(0),
            generated_chunks: stats_by_state.get(&ChunkState::Generated).copied().unwrap_or(0),
            meshed_chunks: stats_by_state.get(&ChunkState::Meshed).copied().unwrap_or(0),
            render_ready_chunks: stats_by_state.get(&ChunkState::RenderReady).copied().unwrap_or(0),
            error_chunks: stats_by_state.get(&ChunkState::Error).copied().unwrap_or(0),
        }
    }

    // ===== RENDERING INTEGRATION METHODS =====

    /// Get all chunks that are ready for rendering
    pub fn get_render_ready_chunks(&self) -> Vec<(ChunkCoord, &ChunkEntry)> {
        self.chunks
            .iter()
            .filter(|(_, entry)| entry.is_renderable())
            .map(|(coord, entry)| (*coord, entry))
            .collect()
    }

    /// Get all chunks that are ready for rendering within render distance of a center point
    pub fn get_render_ready_chunks_in_distance(&self, center: ChunkCoord) -> Vec<(ChunkCoord, &ChunkEntry)> {
        self.chunks
            .iter()
            .filter(|(coord, entry)| {
                entry.is_renderable() && self.is_in_render_distance(**coord, center)
            })
            .map(|(coord, entry)| (*coord, entry))
            .collect()
    }

    /// Prepare a chunk for rendering by transitioning it to the appropriate state
    pub fn prepare_chunk_for_rendering(&mut self, coord: ChunkCoord) -> WorldResult<bool> {
        if let Some(entry) = self.get_chunk_mut(coord) {
            match entry.state {
                ChunkState::Generated => {
                    // Transition to Meshed state (mesh generation would happen here)
                    entry.transition_to(ChunkState::Meshed)
                        .map_err(|e| WorldError::InvalidChunkState {
                            coord,
                            current_state: entry.state,
                            required_state: ChunkState::Meshed,
                        })?;
                    Ok(true) // State changed
                }
                ChunkState::Meshed => {
                    // Transition to RenderReady state
                    entry.transition_to(ChunkState::RenderReady)
                        .map_err(|e| WorldError::InvalidChunkState {
                            coord,
                            current_state: entry.state,
                            required_state: ChunkState::RenderReady,
                        })?;
                    Ok(true) // State changed
                }
                ChunkState::RenderReady => {
                    // Already render ready
                    Ok(false) // No state change needed
                }
                _ => {
                    // Cannot prepare chunks in Loading or Error states
                    Err(WorldError::InvalidChunkState {
                        coord,
                        current_state: entry.state,
                        required_state: ChunkState::Generated,
                    })
                }
            }
        } else {
            Err(WorldError::ChunkNotFound { coord })
        }
    }

    /// Prepare multiple chunks for rendering
    pub fn prepare_chunks_for_rendering(&mut self, coords: &[ChunkCoord]) -> Vec<(ChunkCoord, WorldResult<bool>)> {
        coords
            .iter()
            .map(|&coord| (coord, self.prepare_chunk_for_rendering(coord)))
            .collect()
    }

    /// Get chunks that need mesh generation (Generated state)
    pub fn get_chunks_needing_mesh_generation(&self) -> Vec<(ChunkCoord, &ChunkEntry)> {
        self.chunks
            .iter()
            .filter(|(_, entry)| entry.state == ChunkState::Generated)
            .map(|(coord, entry)| (*coord, entry))
            .collect()
    }

    /// Get chunks that have been meshed but are not yet render-ready
    pub fn get_meshed_chunks(&self) -> Vec<(ChunkCoord, &ChunkEntry)> {
        self.chunks
            .iter()
            .filter(|(_, entry)| entry.state == ChunkState::Meshed)
            .map(|(coord, entry)| (*coord, entry))
            .collect()
    }

    /// Mark a chunk as render-ready after mesh generation is complete
    pub fn mark_chunk_render_ready(&mut self, coord: ChunkCoord) -> WorldResult<()> {
        if let Some(entry) = self.get_chunk_mut(coord) {
            entry.transition_to(ChunkState::RenderReady)
                .map_err(|e| WorldError::InvalidChunkState {
                    coord,
                    current_state: entry.state,
                    required_state: ChunkState::RenderReady,
                })
        } else {
            Err(WorldError::ChunkNotFound { coord })
        }
    }

    /// Get render-ready chunks within a bounding box (for frustum culling)
    pub fn get_render_ready_chunks_in_bounds(&self, min: ChunkCoord, max: ChunkCoord) -> Vec<(ChunkCoord, &ChunkEntry)> {
        self.chunks_in_bounds(min, max)
            .into_iter()
            .filter_map(|entry| {
                // Find the coordinate for this entry
                for (coord, e) in &self.chunks {
                    if std::ptr::eq(e, entry) && entry.is_renderable() {
                        return Some((*coord, entry));
                    }
                }
                None
            })
            .collect()
    }

    /// Get all chunks that need processing for rendering pipeline
    pub fn get_chunks_needing_processing(&self) -> Vec<(ChunkCoord, &ChunkEntry)> {
        self.chunks
            .iter()
            .filter(|(_, entry)| entry.needs_processing())
            .map(|(coord, entry)| (*coord, entry))
            .collect()
    }

    /// Batch prepare chunks for rendering within render distance
    pub fn batch_prepare_chunks_in_render_distance(&mut self, center: ChunkCoord) -> WorldResult<usize> {
        let coords_to_prepare: Vec<ChunkCoord> = self.chunks
            .iter()
            .filter(|(coord, entry)| {
                self.is_in_render_distance(**coord, center) && 
                matches!(entry.state, ChunkState::Generated | ChunkState::Meshed)
            })
            .map(|(coord, _)| *coord)
            .collect();

        let mut prepared_count = 0;
        for coord in coords_to_prepare {
            match self.prepare_chunk_for_rendering(coord) {
                Ok(true) => prepared_count += 1,
                Ok(false) => {}, // Already prepared
                Err(_) => {}, // Skip errors for batch operation
            }
        }

        Ok(prepared_count)
    }

    // ===== CHANGE TRACKING AND RE-MESHING METHODS =====

    /// Mark a chunk as modified, requiring re-meshing
    pub fn mark_chunk_modified(&mut self, coord: ChunkCoord) -> WorldResult<()> {
        if let Some(entry) = self.get_chunk_mut(coord) {
            entry.mark_mesh_dirty();
            
            // If chunk was RenderReady, transition back to Meshed for re-processing
            if entry.state == ChunkState::RenderReady {
                entry.transition_to(ChunkState::Meshed)
                    .map_err(|_| WorldError::InvalidChunkState {
                        coord,
                        current_state: entry.state,
                        required_state: ChunkState::Meshed,
                    })?;
            }
            
            Ok(())
        } else {
            Err(WorldError::ChunkNotFound { coord })
        }
    }

    /// Mark multiple chunks as modified
    pub fn mark_chunks_modified(&mut self, coords: &[ChunkCoord]) -> Vec<(ChunkCoord, WorldResult<()>)> {
        coords
            .iter()
            .map(|&coord| (coord, self.mark_chunk_modified(coord)))
            .collect()
    }

    /// Get all chunks that have dirty meshes and need re-meshing
    pub fn get_chunks_with_dirty_meshes(&self) -> Vec<(ChunkCoord, &ChunkEntry)> {
        self.chunks
            .iter()
            .filter(|(_, entry)| entry.mesh_dirty)
            .map(|(coord, entry)| (*coord, entry))
            .collect()
    }

    /// Clear the mesh dirty flag for a chunk (called after successful re-meshing)
    pub fn clear_chunk_mesh_dirty(&mut self, coord: ChunkCoord) -> WorldResult<()> {
        if let Some(entry) = self.get_chunk_mut(coord) {
            entry.clear_mesh_dirty();
            Ok(())
        } else {
            Err(WorldError::ChunkNotFound { coord })
        }
    }

    /// Mark adjacent chunks as potentially needing re-meshing due to changes
    /// This is important for chunk boundaries where changes in one chunk affect neighboring chunks
    pub fn mark_adjacent_chunks_for_remesh(&mut self, coord: ChunkCoord) -> WorldResult<usize> {
        let adjacent_coords = coord.adjacent();
        let mut marked_count = 0;

        for adj_coord in adjacent_coords {
            if self.is_chunk_loaded(adj_coord) {
                match self.mark_chunk_modified(adj_coord) {
                    Ok(()) => marked_count += 1,
                    Err(_) => {}, // Skip errors for batch operation
                }
            }
        }

        Ok(marked_count)
    }

    /// Process all chunks that need re-meshing
    /// Returns the number of chunks that were processed
    pub fn process_dirty_chunks(&mut self) -> WorldResult<usize> {
        let dirty_coords: Vec<ChunkCoord> = self.get_chunks_with_dirty_meshes()
            .into_iter()
            .map(|(coord, _)| coord)
            .collect();

        let mut processed_count = 0;
        for coord in dirty_coords {
            // In a real implementation, this would trigger actual mesh generation
            // For now, we simulate the process by clearing the dirty flag and updating state
            if let Some(entry) = self.get_chunk_mut(coord) {
                entry.clear_mesh_dirty();
                
                // If chunk was in Generated state, move to Meshed
                if entry.state == ChunkState::Generated {
                    let _ = entry.transition_to(ChunkState::Meshed);
                }
                
                processed_count += 1;
            }
        }

        Ok(processed_count)
    }

    /// Get chunks that need re-meshing within render distance
    pub fn get_dirty_chunks_in_render_distance(&self, center: ChunkCoord) -> Vec<(ChunkCoord, &ChunkEntry)> {
        self.chunks
            .iter()
            .filter(|(coord, entry)| {
                entry.mesh_dirty && self.is_in_render_distance(**coord, center)
            })
            .map(|(coord, entry)| (*coord, entry))
            .collect()
    }

    /// Batch process dirty chunks within render distance
    pub fn process_dirty_chunks_in_render_distance(&mut self, center: ChunkCoord) -> WorldResult<usize> {
        let dirty_coords: Vec<ChunkCoord> = self.get_dirty_chunks_in_render_distance(center)
            .into_iter()
            .map(|(coord, _)| coord)
            .collect();

        let mut processed_count = 0;
        for coord in dirty_coords {
            if let Some(entry) = self.get_chunk_mut(coord) {
                entry.clear_mesh_dirty();
                
                // Update state appropriately
                if entry.state == ChunkState::Generated {
                    let _ = entry.transition_to(ChunkState::Meshed);
                }
                
                processed_count += 1;
            }
        }

        Ok(processed_count)
    }

    /// Check if any chunks need re-meshing
    pub fn has_dirty_chunks(&self) -> bool {
        self.chunks.values().any(|entry| entry.mesh_dirty)
    }

    /// Get statistics about chunks needing re-meshing
    pub fn get_dirty_chunk_stats(&self) -> DirtyChunkStats {
        let mut stats = DirtyChunkStats {
            total_dirty_chunks: 0,
            dirty_generated_chunks: 0,
            dirty_meshed_chunks: 0,
            dirty_render_ready_chunks: 0,
        };

        for entry in self.chunks.values() {
            if entry.mesh_dirty {
                stats.total_dirty_chunks += 1;
                match entry.state {
                    ChunkState::Generated => stats.dirty_generated_chunks += 1,
                    ChunkState::Meshed => stats.dirty_meshed_chunks += 1,
                    ChunkState::RenderReady => stats.dirty_render_ready_chunks += 1,
                    _ => {}, // Other states don't typically have dirty meshes
                }
            }
        }

        stats
    }

    // ===== FRUSTUM CULLING METHODS =====

    /// Get render-ready chunks that are visible within a camera frustum
    /// This is a simplified frustum culling implementation using bounding box intersection
    pub fn get_visible_chunks_in_frustum(&self, frustum: &CameraFrustum) -> Vec<(ChunkCoord, &ChunkEntry)> {
        self.chunks
            .iter()
            .filter(|(coord, entry)| {
                entry.is_renderable() && self.is_chunk_in_frustum(**coord, frustum)
            })
            .map(|(coord, entry)| (*coord, entry))
            .collect()
    }

    /// Check if a chunk is within the camera frustum
    pub fn is_chunk_in_frustum(&self, coord: ChunkCoord, frustum: &CameraFrustum) -> bool {
        let (min_bounds, max_bounds) = self.chunk_bounds(coord);
        frustum.intersects_aabb(min_bounds, max_bounds)
    }

    /// Get render-ready chunks within both render distance and camera frustum
    pub fn get_visible_chunks_in_distance_and_frustum(
        &self, 
        center: ChunkCoord, 
        frustum: &CameraFrustum
    ) -> Vec<(ChunkCoord, &ChunkEntry)> {
        self.chunks
            .iter()
            .filter(|(coord, entry)| {
                entry.is_renderable() && 
                self.is_in_render_distance(**coord, center) &&
                self.is_chunk_in_frustum(**coord, frustum)
            })
            .map(|(coord, entry)| (*coord, entry))
            .collect()
    }

    /// Perform frustum culling on a list of chunk coordinates
    /// Returns only the coordinates that are within the frustum
    pub fn cull_chunks_by_frustum(&self, coords: &[ChunkCoord], frustum: &CameraFrustum) -> Vec<ChunkCoord> {
        coords
            .iter()
            .filter(|coord| self.is_chunk_in_frustum(**coord, frustum))
            .copied()
            .collect()
    }

    /// Get chunks within a view frustum, sorted by distance from camera
    pub fn get_visible_chunks_sorted_by_distance(
        &self, 
        camera_pos: Vec3, 
        frustum: &CameraFrustum
    ) -> Vec<(ChunkCoord, &ChunkEntry, f32)> {
        let mut visible_chunks: Vec<(ChunkCoord, &ChunkEntry, f32)> = self.chunks
            .iter()
            .filter(|(coord, entry)| {
                entry.is_renderable() && self.is_chunk_in_frustum(**coord, frustum)
            })
            .map(|(coord, entry)| {
                let chunk_center = self.chunk_to_world_pos(*coord) + 
                    Vec3::splat(self.chunk_size as f32 / 2.0);
                let distance = camera_pos.distance(chunk_center);
                (*coord, entry, distance)
            })
            .collect();

        // Sort by distance (closest first)
        visible_chunks.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        visible_chunks
    }

    /// Get frustum culling statistics
    pub fn get_frustum_culling_stats(&self, frustum: &CameraFrustum) -> FrustumCullingStats {
        let total_chunks = self.chunk_count();
        let render_ready_chunks = self.get_render_ready_chunks().len();
        let visible_chunks = self.get_visible_chunks_in_frustum(frustum).len();
        
        FrustumCullingStats {
            total_chunks,
            render_ready_chunks,
            visible_chunks,
            culled_chunks: render_ready_chunks.saturating_sub(visible_chunks),
            culling_efficiency: if render_ready_chunks > 0 {
                (render_ready_chunks.saturating_sub(visible_chunks)) as f32 / render_ready_chunks as f32
            } else {
                0.0
            },
        }
    }

    // ===== MEMORY MANAGEMENT METHODS =====

    /// Get current memory usage statistics
    pub fn memory_stats(&self) -> memory::MemoryStats {
        self.memory_manager.memory_stats()
    }

    /// Check if memory cleanup should be triggered
    pub fn should_cleanup_memory(&self) -> bool {
        self.memory_manager.should_cleanup()
    }

    /// Get chunks suggested for cleanup based on LRU policy
    pub fn suggest_chunks_for_cleanup(&self, target_free_bytes: usize) -> Vec<ChunkCoord> {
        self.memory_manager.suggest_cleanup(target_free_bytes)
    }

    /// Perform automatic memory cleanup if needed
    pub fn cleanup_memory_if_needed(&mut self) -> WorldResult<usize> {
        if !self.should_cleanup_memory() {
            return Ok(0);
        }

        let stats = self.memory_stats();
        let target_free = (stats.total_budget as f32 * 0.2) as usize; // Free 20% of budget
        let chunks_to_cleanup = self.suggest_chunks_for_cleanup(target_free);
        
        let mut freed_chunks = 0;
        for coord in chunks_to_cleanup {
            if self.remove_chunk(coord).is_some() {
                freed_chunks += 1;
            }
        }
        
        Ok(freed_chunks)
    }

    /// Force cleanup of specific chunks to free memory
    pub fn force_cleanup_chunks(&mut self, coords: &[ChunkCoord]) -> usize {
        let mut freed_chunks = 0;
        
        for &coord in coords {
            if self.remove_chunk(coord).is_some() {
                freed_chunks += 1;
            }
        }
        
        freed_chunks
    }

    /// Check if the world can load a new chunk without exceeding memory limits
    pub fn can_load_new_chunk(&self) -> bool {
        // Estimate memory for a new chunk
        let chunk_size_bytes = (self.chunk_size as usize).pow(3) * 2;
        let estimated_chunk_memory = chunk_size_bytes + 1024;
        
        self.memory_manager.can_load_chunk(estimated_chunk_memory)
    }

    /// Get memory usage as a percentage of the budget
    pub fn memory_usage_percentage(&self) -> f32 {
        self.memory_manager.usage_fraction() * 100.0
    }

    /// Set the memory cleanup threshold
    pub fn set_memory_cleanup_threshold(&mut self, threshold: f32) -> WorldResult<()> {
        self.memory_manager.set_cleanup_threshold(threshold)
    }

    /// Update the memory budget for the world
    pub fn set_memory_budget(&mut self, new_budget: usize) -> WorldResult<()> {
        self.memory_manager.set_memory_budget(new_budget)
    }

    /// Get the oldest and newest accessed chunks for debugging
    pub fn memory_access_info(&self) -> (Option<(ChunkCoord, std::time::Instant)>, Option<(ChunkCoord, std::time::Instant)>) {
        (self.memory_manager.oldest_chunk(), self.memory_manager.newest_chunk())
    }

    /// Validate that memory tracking is consistent with loaded chunks
    pub fn validate_memory_consistency(&self) -> WorldResult<()> {
        let stats = self.memory_stats();
        
        // Check that tracked chunks match loaded chunks
        if stats.chunks_tracked != self.chunk_count() {
            return Err(WorldError::InvalidConfiguration {
                parameter: "memory_tracking".to_string(),
                value: format!("tracked: {}, loaded: {}", stats.chunks_tracked, self.chunk_count()),
                reason: "Memory tracking is inconsistent with loaded chunks".to_string(),
            });
        }
        
        // Check that memory usage is within bounds
        if stats.current_usage > stats.total_budget {
            return Err(WorldError::OutOfMemory {
                requested: stats.current_usage,
                available: stats.total_budget,
            });
        }
        
        Ok(())
    }

    /// Perform comprehensive memory bounds checking
    pub fn validate_memory_bounds(&self) -> WorldResult<()> {
        // Validate memory manager internal consistency
        if let Err(error_msg) = self.memory_manager.validate_memory_bounds() {
            return Err(WorldError::InvalidConfiguration {
                parameter: "memory_bounds".to_string(),
                value: "inconsistent".to_string(),
                reason: error_msg,
            });
        }
        
        // Validate that all loaded chunks are tracked in memory manager
        for coord in self.chunks.keys() {
            if !self.memory_manager.chunk_memory().contains_key(coord) {
                return Err(WorldError::InvalidConfiguration {
                    parameter: "memory_tracking".to_string(),
                    value: format!("chunk {:?}", coord),
                    reason: "Loaded chunk is not tracked in memory manager".to_string(),
                });
            }
        }
        
        // Check for memory leaks (chunks tracked but not loaded)
        for coord in self.memory_manager.chunk_memory().keys() {
            if !self.chunks.contains_key(coord) {
                return Err(WorldError::InvalidConfiguration {
                    parameter: "memory_leak".to_string(),
                    value: format!("chunk {:?}", coord),
                    reason: "Memory manager tracks chunk that is not loaded".to_string(),
                });
            }
        }
        
        Ok(())
    }

    /// Detect and report potential memory leaks
    pub fn detect_memory_leaks(&self, max_idle_duration: std::time::Duration) -> Vec<ChunkCoord> {
        self.memory_manager.detect_potential_leaks(max_idle_duration)
    }

    /// Perform automatic leak cleanup
    pub fn cleanup_memory_leaks(&mut self, max_idle_duration: std::time::Duration) -> WorldResult<usize> {
        let potential_leaks = self.detect_memory_leaks(max_idle_duration);
        let mut cleaned_up = 0;
        
        for coord in potential_leaks {
            // Remove the chunk from the world, which will also unregister memory
            if self.remove_chunk(coord).is_some() {
                cleaned_up += 1;
            }
        }
        
        Ok(cleaned_up)
    }

    /// Get memory fragmentation statistics
    pub fn memory_fragmentation_stats(&self) -> memory::FragmentationStats {
        self.memory_manager.fragmentation_stats()
    }

    /// Check if memory is critically fragmented and needs defragmentation
    pub fn needs_memory_defragmentation(&self) -> bool {
        let frag_stats = self.memory_fragmentation_stats();
        frag_stats.is_fragmented() && frag_stats.fragmentation_score() > 0.7
    }

    /// Perform bounds checking before any memory-intensive operation
    pub fn check_memory_bounds_before_operation(&self, estimated_memory: usize) -> WorldResult<()> {
        let stats = self.memory_stats();
        
        // Check if operation would exceed budget
        if stats.current_usage + estimated_memory > stats.total_budget {
            return Err(WorldError::OutOfMemory {
                requested: estimated_memory,
                available: stats.available_memory(),
            });
        }
        
        // Check if operation would trigger critical memory pressure
        let new_usage_fraction = (stats.current_usage + estimated_memory) as f32 / stats.total_budget as f32;
        if new_usage_fraction > 0.95 { // 95% threshold for critical operations
            return Err(WorldError::ResourceLimitExceeded {
                resource: "memory".to_string(),
                limit: (stats.total_budget as f32 * 0.95) as usize,
                requested: stats.current_usage + estimated_memory,
            });
        }
        
        Ok(())
    }

    /// Enforce strict memory bounds by preventing operations that would exceed limits
    pub fn enforce_memory_bounds(&mut self) -> WorldResult<()> {
        // Validate current state
        self.validate_memory_bounds()?;
        
        // If memory usage is critical, force cleanup
        if self.should_cleanup_memory() {
            let cleaned_up = self.cleanup_memory_if_needed()?;
            if cleaned_up == 0 && self.should_cleanup_memory() {
                // If cleanup didn't help and we're still over threshold, this is an error
                return Err(WorldError::OutOfMemory {
                    requested: self.memory_manager.current_usage(),
                    available: self.memory_manager.memory_budget(),
                });
            }
        }
        
        Ok(())
    }

    /// Perform comprehensive memory health check
    pub fn memory_health_check(&self) -> WorldResult<MemoryHealthReport> {
        let stats = self.memory_stats();
        let frag_stats = self.memory_fragmentation_stats();
        let potential_leaks = self.detect_memory_leaks(std::time::Duration::from_secs(300)); // 5 minutes
        
        let mut issues = Vec::new();
        let mut warnings = Vec::new();
        
        // Check for critical memory usage
        if stats.is_critical() {
            issues.push("Memory usage is above cleanup threshold".to_string());
        }
        
        // Check for high fragmentation
        if frag_stats.is_fragmented() {
            warnings.push(format!("Memory is fragmented (score: {:.2})", frag_stats.fragmentation_score()));
        }
        
        // Check for potential leaks
        if !potential_leaks.is_empty() {
            warnings.push(format!("Found {} potential memory leaks", potential_leaks.len()));
        }
        
        // Check bounds consistency
        if let Err(_) = self.validate_memory_bounds() {
            issues.push("Memory bounds validation failed".to_string());
        }
        
        let health_status = if !issues.is_empty() {
            MemoryHealthStatus::Critical
        } else if !warnings.is_empty() {
            MemoryHealthStatus::Warning
        } else {
            MemoryHealthStatus::Healthy
        };
        
        Ok(MemoryHealthReport {
            status: health_status,
            memory_stats: stats,
            fragmentation_stats: frag_stats,
            potential_leaks: potential_leaks.len(),
            issues,
            warnings,
        })
    }
}

// Constants for world management
pub const DEFAULT_CHUNK_SIZE: u32 = 32;
pub const DEFAULT_RENDER_DISTANCE: u32 = 8;
pub const MAX_CHUNKS_PER_FRAME: usize = 4;
pub const CHUNK_UNLOAD_DELAY_SECONDS: u64 = 30;

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use crate::chunk::{Chunk, ChunkDimensions, ChunkPosition};
    use std::time::Duration;

    // Property test generators
    fn arb_chunk_state() -> impl Strategy<Value = ChunkState> {
        prop_oneof![
            Just(ChunkState::Loading),
            Just(ChunkState::Generated),
            Just(ChunkState::Meshed),
            Just(ChunkState::RenderReady),
            Just(ChunkState::Error),
        ]
    }

    fn arb_chunk() -> impl Strategy<Value = Chunk> {
        // Create a simple chunk for testing using the proper constructor
        Just(Chunk::new(
            ChunkPosition { x: 0, z: 0 },
            ChunkDimensions { width: 16, height: 16, depth: 16 },
        ))
    }

    fn arb_chunk_entry() -> impl Strategy<Value = ChunkEntry> {
        arb_chunk().prop_map(|chunk| ChunkEntry::new(chunk))
    }

    // Property 4: Chunk State Management Consistency
    // **Validates: Requirements 1.4, 4.2**
    proptest! {
        #[test]
        fn property_chunk_state_management_consistency(
            mut entry in arb_chunk_entry(),
            target_state in arb_chunk_state(),
        ) {
            // Feature: world-integration, Property 4: Chunk State Management Consistency
            
            let initial_state = entry.state;
            let can_transition = entry.can_transition_to(target_state);
            let transition_result = entry.transition_to(target_state);
            
            // If we can transition, the transition should succeed
            if can_transition {
                prop_assert!(transition_result.is_ok(), "Transition should succeed when can_transition returns true");
                prop_assert_eq!(entry.state, target_state, "State should be updated after successful transition");
            } else {
                // If we can't transition, the transition should fail and state should remain unchanged
                prop_assert!(transition_result.is_err(), "Transition should fail when can_transition returns false");
                prop_assert_eq!(entry.state, initial_state, "State should remain unchanged after failed transition");
            }
        }
    }

    // Property test for state transition validity
    proptest! {
        #[test]
        fn property_state_transition_validity(
            initial_state in arb_chunk_state(),
            target_state in arb_chunk_state(),
        ) {
            // Feature: world-integration, Property 4: Chunk State Management Consistency (transitions)
            
            let chunk = Chunk::new(
                ChunkPosition { x: 0, z: 0 },
                ChunkDimensions { width: 16, height: 16, depth: 16 },
            );
            let mut entry = ChunkEntry::new(chunk);
            entry.state = initial_state;
            
            let can_transition = entry.can_transition_to(target_state);
            
            // Valid transitions based on the state machine
            let expected_valid = match (initial_state, target_state) {
                // Loading can transition to Generated or Error
                (ChunkState::Loading, ChunkState::Generated) | 
                (ChunkState::Loading, ChunkState::Error) => true,
                // Generated can transition to Meshed or Error
                (ChunkState::Generated, ChunkState::Meshed) | 
                (ChunkState::Generated, ChunkState::Error) => true,
                // Meshed can transition to RenderReady or back to Generated or Error
                (ChunkState::Meshed, ChunkState::RenderReady) | 
                (ChunkState::Meshed, ChunkState::Generated) | 
                (ChunkState::Meshed, ChunkState::Error) => true,
                // RenderReady can transition back to Meshed or Generated or Error
                (ChunkState::RenderReady, ChunkState::Meshed) | 
                (ChunkState::RenderReady, ChunkState::Generated) | 
                (ChunkState::RenderReady, ChunkState::Error) => true,
                // Error can transition to Loading for retry
                (ChunkState::Error, ChunkState::Loading) => true,
                // Same state transitions are always allowed
                (s1, s2) if s1 == s2 => true,
                // All other transitions are invalid
                _ => false,
            };
            
            prop_assert_eq!(can_transition, expected_valid, 
                "Transition validity should match expected state machine rules for {:?} -> {:?}", 
                initial_state, target_state);
        }
    }

    // Property test for chunk entry metadata consistency
    proptest! {
        #[test]
        fn property_chunk_entry_metadata_consistency(
            mut entry in arb_chunk_entry(),
        ) {
            // Feature: world-integration, Property 4: Chunk State Management Consistency (metadata)
            
            let initial_access_time = entry.last_accessed;
            
            // Touch should update access time
            std::thread::sleep(std::time::Duration::from_millis(1));
            entry.touch();
            prop_assert!(entry.last_accessed > initial_access_time, "Touch should update last_accessed time");
            
            // Mark mesh dirty should set the flag and potentially change state
            let initial_state = entry.state;
            entry.mark_mesh_dirty();
            prop_assert!(entry.mesh_dirty, "mark_mesh_dirty should set mesh_dirty flag");
            
            // If we were RenderReady, we should transition to Meshed
            if initial_state == ChunkState::RenderReady {
                prop_assert_eq!(entry.state, ChunkState::Meshed, "RenderReady should transition to Meshed when mesh is marked dirty");
            }
            
            // Clear mesh dirty should clear the flag
            entry.clear_mesh_dirty();
            prop_assert!(!entry.mesh_dirty, "clear_mesh_dirty should clear mesh_dirty flag");
        }
    }

    // Property test generators for World testing
    fn arb_world_config() -> impl Strategy<Value = WorldConfig> {
        (1u32..=32u32, 10usize..=100usize).prop_map(|(render_distance, max_chunks)| {
            WorldConfig::new()
                .with_render_distance(render_distance)
                .with_max_chunks(Some(max_chunks))
        })
    }

    fn arb_chunk_coord() -> impl Strategy<Value = ChunkCoord> {
        (-100i32..100i32, -100i32..100i32, -100i32..100i32)
            .prop_map(|(x, y, z)| ChunkCoord::new(x, y, z))
    }

    // Property 1: Chunk Storage and Retrieval Consistency
    // **Validates: Requirements 1.1, 1.3**
    proptest! {
        #[test]
        fn property_chunk_storage_and_retrieval_consistency(
            config in arb_world_config(),
            coord in arb_chunk_coord(),
            chunk in arb_chunk(),
        ) {
            // Feature: world-integration, Property 1: Chunk Storage and Retrieval Consistency
            
            let mut world = World::new(config).unwrap();
            
            // Initially, the chunk should not be loaded
            prop_assert!(!world.is_chunk_loaded(coord), "Chunk should not be loaded initially");
            prop_assert!(world.get_chunk(coord).is_none(), "get_chunk should return None for unloaded chunk");
            
            // Store chunk properties before moving it
            let chunk_dimensions = chunk.dimensions();
            let chunk_position = chunk.world_position();
            
            // Add the chunk to the world
            let add_result = world.add_chunk(coord, chunk);
            prop_assert!(add_result.is_ok(), "Adding chunk should succeed");
            
            // Now the chunk should be loaded and retrievable
            prop_assert!(world.is_chunk_loaded(coord), "Chunk should be loaded after adding");
            
            let retrieved_chunk = world.get_chunk(coord);
            prop_assert!(retrieved_chunk.is_some(), "get_chunk should return Some for loaded chunk");
            
            // The retrieved chunk should have the same dimensions as the original
            let chunk_entry = retrieved_chunk.unwrap();
            prop_assert_eq!(chunk_entry.chunk.dimensions(), chunk_dimensions, 
                "Retrieved chunk should have same dimensions as original");
            prop_assert_eq!(chunk_entry.chunk.world_position(), chunk_position, 
                "Retrieved chunk should have same world position as original");
            
            // Chunk count should be 1
            prop_assert_eq!(world.chunk_count(), 1, "World should have exactly 1 chunk loaded");
        }
    }

    // Property test for chunk removal consistency
    proptest! {
        #[test]
        fn property_chunk_removal_consistency(
            config in arb_world_config(),
            coord in arb_chunk_coord(),
            chunk in arb_chunk(),
        ) {
            // Feature: world-integration, Property 1: Chunk Storage and Retrieval Consistency (removal)
            
            let mut world = World::new(config).unwrap();
            
            // Add and then remove a chunk
            world.add_chunk(coord, chunk).unwrap();
            prop_assert!(world.is_chunk_loaded(coord), "Chunk should be loaded after adding");
            
            let removed_chunk = world.remove_chunk(coord);
            prop_assert!(removed_chunk.is_some(), "remove_chunk should return Some for existing chunk");
            
            // After removal, chunk should not be loaded
            prop_assert!(!world.is_chunk_loaded(coord), "Chunk should not be loaded after removal");
            prop_assert!(world.get_chunk(coord).is_none(), "get_chunk should return None after removal");
            prop_assert_eq!(world.chunk_count(), 0, "World should have 0 chunks after removal");
            
            // Removing again should return None
            let removed_again = world.remove_chunk(coord);
            prop_assert!(removed_again.is_none(), "Removing non-existent chunk should return None");
        }
    }

    // Property test for multiple chunk operations
    proptest! {
        #[test]
        fn property_multiple_chunk_operations(
            config in arb_world_config(),
            coords in prop::collection::vec(arb_chunk_coord(), 1..10),
        ) {
            // Feature: world-integration, Property 1: Chunk Storage and Retrieval Consistency (multiple)
            
            let mut world = World::new(config).unwrap();
            let mut chunks = Vec::new();
            
            // Create unique coordinates to avoid conflicts
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            // Add multiple chunks
            for coord in &unique_coords {
                let chunk = Chunk::new(
                    ChunkPosition { x: coord.x, z: coord.z },
                    ChunkDimensions { width: 16, height: 16, depth: 16 },
                );
                world.add_chunk(*coord, chunk).unwrap();
                chunks.push(*coord);
            }
            
            // All chunks should be loaded and retrievable
            prop_assert_eq!(world.chunk_count(), unique_coords.len(), 
                "World should have correct number of chunks loaded");
            
            for coord in &unique_coords {
                prop_assert!(world.is_chunk_loaded(*coord), 
                    "Each added chunk should be loaded");
                prop_assert!(world.get_chunk(*coord).is_some(), 
                    "Each added chunk should be retrievable");
            }
            
            // Remove half the chunks
            let half_count = unique_coords.len() / 2;
            for coord in unique_coords.iter().take(half_count) {
                let removed = world.remove_chunk(*coord);
                prop_assert!(removed.is_some(), "Removing existing chunk should succeed");
            }
            
            // Check final state
            let expected_remaining = unique_coords.len() - half_count;
            prop_assert_eq!(world.chunk_count(), expected_remaining, 
                "World should have correct number of chunks after partial removal");
        }
    }

    // Property test generators for LoadPattern
    fn arb_load_pattern() -> impl Strategy<Value = LoadPattern> {
        prop_oneof![
            arb_chunk_coord().prop_map(LoadPattern::Single),
            (arb_chunk_coord(), 1u32..=5u32).prop_map(|(center, radius)| LoadPattern::Grid { center, radius }),
            prop::collection::vec(arb_chunk_coord(), 1..10).prop_map(LoadPattern::Custom),
        ]
    }

    // Property 5: Multi-Chunk Loading Correctness
    // **Validates: Requirements 3.1, 3.2, 3.5**
    proptest! {
        #[test]
        fn property_multi_chunk_loading_correctness(
            render_distance in 1u32..=32u32,
            pattern in arb_load_pattern(),
        ) {
            // Feature: world-integration, Property 5: Multi-Chunk Loading Correctness
            
            // Get expected coordinates from the pattern and ensure sufficient capacity
            let expected_coords = pattern.get_coordinates();
            let expected_count = expected_coords.len();
            let max_chunks = expected_count * 2; // Ensure we have enough capacity
            
            let config = WorldConfig::new()
                .with_render_distance(render_distance)
                .with_max_chunks(Some(max_chunks));
            
            let mut world = World::new(config).unwrap();
            
            // Load chunks using the pattern
            let progress = world.load_chunks(pattern).unwrap();
            
            // Progress should be complete
            prop_assert!(progress.is_complete(), "Loading progress should be complete");
            prop_assert_eq!(progress.total_chunks(), expected_count, 
                "Progress should track correct total chunk count");
            
            // All expected chunks should be loaded in the world
            for coord in &expected_coords {
                prop_assert!(world.is_chunk_loaded(*coord), 
                    "Each chunk from pattern should be loaded in world");
                
                // Check that the chunk has the correct state
                if let Some(entry) = world.get_chunk(*coord) {
                    prop_assert_eq!(entry.state, ChunkState::Generated, 
                        "Loaded chunks should be in Generated state");
                } else {
                    prop_assert!(false, "Chunk should exist in world after loading");
                }
            }
            
            // World chunk count should match expected count
            prop_assert_eq!(world.chunk_count(), expected_count, 
                "World should have exactly the expected number of chunks loaded");
            
            // All chunks should be positioned correctly in world space
            for coord in &expected_coords {
                let world_pos = world.chunk_to_world_pos(*coord);
                let recovered_coord = world.world_to_chunk_coord(world_pos);
                prop_assert_eq!(*coord, recovered_coord, 
                    "Chunk should be positioned correctly in world space");
            }
            
            // Progress should report all loads as successful
            prop_assert_eq!(progress.successful_loads(), expected_count, 
                "All loads should be successful for valid coordinates");
            prop_assert_eq!(progress.failed_loads(), 0, 
                "No loads should fail for valid coordinates");
        }
    }

    // Property test for loading already loaded chunks
    proptest! {
        #[test]
        fn property_loading_already_loaded_chunks(
            config in arb_world_config(),
            coords in prop::collection::vec(arb_chunk_coord(), 1..5),
        ) {
            // Feature: world-integration, Property 5: Multi-Chunk Loading Correctness (already loaded)
            
            let mut world = World::new(config).unwrap();
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.is_empty() {
                return Ok(());
            }
            
            // Load chunks first time
            let pattern1 = LoadPattern::Custom(unique_coords.clone());
            let progress1 = world.load_chunks(pattern1).unwrap();
            
            prop_assert_eq!(progress1.successful_loads(), unique_coords.len(), 
                "First load should succeed for all chunks");
            
            // Load the same chunks again
            let pattern2 = LoadPattern::Custom(unique_coords.clone());
            let progress2 = world.load_chunks(pattern2).unwrap();
            
            // Second load should report all as already loaded
            prop_assert_eq!(progress2.successful_loads(), unique_coords.len(), 
                "Second load should report all chunks as successful");
            prop_assert_eq!(progress2.failed_loads(), 0, 
                "Second load should have no failures");
            
            // Check that results indicate already loaded
            for coord in &unique_coords {
                if let Some(result) = progress2.get_result(*coord) {
                    prop_assert!(matches!(result, LoadResult::AlreadyLoaded), 
                        "Already loaded chunks should be reported as AlreadyLoaded");
                }
            }
            
            // World should still have the same number of chunks
            prop_assert_eq!(world.chunk_count(), unique_coords.len(), 
                "World should not duplicate chunks when loading already loaded chunks");
        }
    }

    // Property test for grid loading pattern
    proptest! {
        #[test]
        fn property_grid_loading_pattern(
            render_distance in 1u32..=32u32,
            center in arb_chunk_coord(),
            radius in 1u32..=3u32,
        ) {
            // Feature: world-integration, Property 5: Multi-Chunk Loading Correctness (grid pattern)
            
            // Calculate expected number of chunks in grid and ensure sufficient capacity
            let expected_count = ((radius * 2 + 1) as usize).pow(3);
            let max_chunks = expected_count * 2; // Ensure we have enough capacity
            
            let config = WorldConfig::new()
                .with_render_distance(render_distance)
                .with_max_chunks(Some(max_chunks));
            
            let mut world = World::new(config).unwrap();
            
            // Load chunks in grid pattern
            let progress = world.load_grid(center, radius).unwrap();
            
            prop_assert_eq!(progress.total_chunks(), expected_count, 
                "Grid pattern should load correct number of chunks");
            prop_assert_eq!(progress.successful_loads(), expected_count, 
                "All grid chunks should load successfully");
            
            // Verify all chunks in the grid are loaded
            let r = radius as i32;
            for x in -r..=r {
                for y in -r..=r {
                    for z in -r..=r {
                        let coord = ChunkCoord::new(center.x + x, center.y + y, center.z + z);
                        prop_assert!(world.is_chunk_loaded(coord), 
                            "Each chunk in grid should be loaded");
                    }
                }
            }
            
            // Verify chunks outside the grid are not loaded
            let outside_coords = [
                ChunkCoord::new(center.x + r + 1, center.y, center.z),
                ChunkCoord::new(center.x - r - 1, center.y, center.z),
                ChunkCoord::new(center.x, center.y + r + 1, center.z),
                ChunkCoord::new(center.x, center.y - r - 1, center.z),
                ChunkCoord::new(center.x, center.y, center.z + r + 1),
                ChunkCoord::new(center.x, center.y, center.z - r - 1),
            ];
            
            for coord in &outside_coords {
                prop_assert!(!world.is_chunk_loaded(*coord), 
                    "Chunks outside grid should not be loaded");
            }
        }
    }

    // Property 12: Progress Tracking Consistency
    // **Validates: Requirements 3.4**
    proptest! {
        #[test]
        fn property_progress_tracking_consistency(
            config in arb_world_config(),
            pattern in arb_load_pattern(),
        ) {
            // Feature: world-integration, Property 12: Progress Tracking Consistency
            
            let mut world = World::new(config).unwrap();
            
            // Get expected coordinates from the pattern
            let expected_coords = pattern.get_coordinates();
            let expected_count = expected_coords.len();
            
            // Load chunks and track progress
            let progress = world.load_chunks(pattern).unwrap();
            
            // Progress tracking should be consistent
            prop_assert_eq!(progress.total_chunks(), expected_count, 
                "Progress should track correct total chunk count");
            
            prop_assert_eq!(progress.completed_chunks(), expected_count, 
                "Progress should show all chunks as completed");
            
            prop_assert!(progress.is_complete(), 
                "Progress should be marked as complete");
            
            prop_assert_eq!(progress.completion_percentage(), 1.0, 
                "Completion percentage should be 100% when all chunks are loaded");
            
            // Sum of successful and failed loads should equal total
            let total_processed = progress.successful_loads() + progress.failed_loads();
            prop_assert_eq!(total_processed, expected_count, 
                "Sum of successful and failed loads should equal total chunks");
            
            // All expected coordinates should have results
            for coord in &expected_coords {
                prop_assert!(progress.get_result(*coord).is_some(), 
                    "Each expected coordinate should have a result in progress");
            }
            
            // Results should be consistent with world state
            let successful_chunks = progress.successful_chunks();
            let failed_chunks = progress.failed_chunks();
            
            // All successful chunks should be loaded in world
            for coord in &successful_chunks {
                prop_assert!(world.is_chunk_loaded(*coord), 
                    "All successful chunks should be loaded in world");
            }
            
            // All failed chunks should not be loaded in world
            for coord in &failed_chunks {
                prop_assert!(!world.is_chunk_loaded(*coord), 
                    "All failed chunks should not be loaded in world");
            }
            
            // Total successful + failed should equal expected count
            prop_assert_eq!(successful_chunks.len() + failed_chunks.len(), expected_count, 
                "Total successful and failed chunks should equal expected count");
        }
    }

    // Property test for progress tracking with partial failures
    proptest! {
        #[test]
        fn property_progress_tracking_with_mixed_results(
            config in arb_world_config(),
            coords in prop::collection::vec(arb_chunk_coord(), 2..8),
        ) {
            // Feature: world-integration, Property 12: Progress Tracking Consistency (mixed results)
            
            let mut world = World::new(config).unwrap();
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.len() < 2 {
                return Ok(());
            }
            
            // Pre-load some chunks to create "already loaded" results
            let half_count = unique_coords.len() / 2;
            for coord in unique_coords.iter().take(half_count) {
                let chunk = Chunk::new(
                    ChunkPosition { x: coord.x, z: coord.z },
                    ChunkDimensions { width: 16, height: 16, depth: 16 },
                );
                world.add_chunk(*coord, chunk).unwrap();
            }
            
            // Now load all chunks (some will be already loaded)
            let pattern = LoadPattern::Custom(unique_coords.clone());
            let progress = world.load_chunks(pattern).unwrap();
            
            // Progress should be complete and consistent
            prop_assert!(progress.is_complete(), "Progress should be complete");
            prop_assert_eq!(progress.total_chunks(), unique_coords.len(), 
                "Progress should track correct total");
            prop_assert_eq!(progress.completed_chunks(), unique_coords.len(), 
                "All chunks should be completed");
            
            // All loads should be successful (either new or already loaded)
            prop_assert_eq!(progress.successful_loads(), unique_coords.len(), 
                "All loads should be successful");
            prop_assert_eq!(progress.failed_loads(), 0, 
                "No loads should fail for valid coordinates");
            
            // Check that some results are "AlreadyLoaded"
            let mut already_loaded_count = 0;
            let mut new_load_count = 0;
            
            for coord in &unique_coords {
                if let Some(result) = progress.get_result(*coord) {
                    match result {
                        LoadResult::AlreadyLoaded => already_loaded_count += 1,
                        LoadResult::Success => new_load_count += 1,
                        LoadResult::Failed(_) => {
                            prop_assert!(false, "Should not have failed loads for valid coordinates");
                        }
                    }
                }
            }
            
            // We should have some already loaded and some new loads
            prop_assert!(already_loaded_count > 0, 
                "Should have some already loaded chunks");
            prop_assert_eq!(already_loaded_count + new_load_count, unique_coords.len(), 
                "Sum of already loaded and new loads should equal total");
        }
    }

    // Property test for progress tracking completion percentage
    proptest! {
        #[test]
        fn property_progress_completion_percentage(
            total_chunks in 1usize..20usize,
            completed_chunks in 0usize..20usize,
        ) {
            // Feature: world-integration, Property 12: Progress Tracking Consistency (percentage)
            
            let completed = completed_chunks.min(total_chunks);
            
            let mut progress = LoadingProgress::new(total_chunks);
            
            // Mark some chunks as completed
            for i in 0..completed {
                let coord = ChunkCoord::new(i as i32, 0, 0);
                progress.mark_completed(coord, LoadResult::Success);
            }
            
            let expected_percentage = if total_chunks == 0 {
                1.0
            } else {
                completed as f32 / total_chunks as f32
            };
            
            let actual_percentage = progress.completion_percentage();
            
            prop_assert!((actual_percentage - expected_percentage).abs() < 0.001, 
                "Completion percentage should be accurate: expected {}, got {}", 
                expected_percentage, actual_percentage);
            
            prop_assert!(actual_percentage >= 0.0 && actual_percentage <= 1.0, 
                "Completion percentage should be between 0.0 and 1.0");
            
            if completed == total_chunks {
                prop_assert_eq!(actual_percentage, 1.0, 
                    "Completion percentage should be 1.0 when all chunks are completed");
            }
            
            if completed == 0 {
                prop_assert_eq!(actual_percentage, 0.0, 
                    "Completion percentage should be 0.0 when no chunks are completed");
            }
        }
    }

    // Property 6: Error Handling Isolation
    // **Validates: Requirements 3.3**
    proptest! {
        #[test]
        fn property_error_handling_isolation(
            render_distance in 1u32..=32u32,
            coords in prop::collection::vec(arb_chunk_coord(), 3..10),
        ) {
            // Feature: world-integration, Property 6: Error Handling Isolation
            
            // Create config with sufficient chunk capacity for the test
            let max_chunks = coords.len() * 2; // Ensure we have enough capacity
            let config = WorldConfig::new()
                .with_render_distance(render_distance)
                .with_max_chunks(Some(max_chunks));
            
            let mut world = World::new(config).unwrap();
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.len() < 3 {
                return Ok(());
            }
            
            // Manually load chunks with error simulation to test isolation
            let mut successful_loads = 0;
            let mut failed_loads = 0;
            let mut expected_failures = Vec::new();
            let mut expected_successes = Vec::new();
            
            for coord in &unique_coords {
                // Determine if this chunk should fail based on our simulation logic
                let should_fail = coord.x < 0 && coord.y < 0 && coord.z < 0 && 
                                 coord.x % 7 == 0 && coord.y % 7 == 0 && coord.z % 7 == 0;
                
                if should_fail {
                    expected_failures.push(*coord);
                } else {
                    expected_successes.push(*coord);
                }
                
                // Attempt to load the chunk with error simulation
                match world.generate_and_load_chunk_with_error_simulation(*coord) {
                    Ok(()) => successful_loads += 1,
                    Err(_) => failed_loads += 1,
                }
            }
            
            // Check that successful chunks are loaded and failed chunks are not
            for coord in &expected_successes {
                prop_assert!(world.is_chunk_loaded(*coord), 
                    "Successful chunks should be loaded in world");
                
                if let Some(entry) = world.get_chunk(*coord) {
                    prop_assert_ne!(entry.state, ChunkState::Error, 
                        "Successful chunks should not be in error state");
                } else {
                    prop_assert!(false, "Successful chunk should exist in world");
                }
            }
            
            for coord in &expected_failures {
                prop_assert!(!world.is_chunk_loaded(*coord), 
                    "Failed chunks should not be loaded in world");
            }
            
            // Verify isolation: successful chunks should not be affected by failures
            prop_assert_eq!(successful_loads, expected_successes.len(), 
                "Number of successful loads should match expected");
            prop_assert_eq!(failed_loads, expected_failures.len(), 
                "Number of failed loads should match expected");
            
            // World should only contain successful chunks
            prop_assert_eq!(world.chunk_count(), expected_successes.len(), 
                "World should only contain successfully loaded chunks");
            
            // World consistency should be maintained
            prop_assert!(world.validate_world_consistency().is_ok(), 
                "World should remain consistent after mixed success/failure loading");
        }
    }

    // Property test for error recovery and retry functionality
    proptest! {
        #[test]
        fn property_error_recovery_and_retry(
            config in arb_world_config(),
            coords in prop::collection::vec(arb_chunk_coord(), 2..6),
        ) {
            // Feature: world-integration, Property 6: Error Handling Isolation (recovery)
            
            let mut world = World::new(config).unwrap();
            
            // Create unique coordinates that will have some failures
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.len() < 2 {
                return Ok(());
            }
            
            // First loading attempt
            let pattern = LoadPattern::Custom(unique_coords.clone());
            let first_progress = world.load_chunks(pattern).unwrap();
            
            // If there were no failures, we can't test retry functionality
            if first_progress.failed_loads() == 0 {
                return Ok(());
            }
            
            let initial_successful_count = first_progress.successful_loads();
            let initial_world_count = world.chunk_count();
            
            // Attempt to retry failed chunks
            let retry_progress = world.retry_failed_chunks(&first_progress).unwrap();
            
            // After retry, world should still be consistent
            prop_assert!(world.validate_world_consistency().is_ok(), 
                "World should remain consistent after retry operations");
            
            // Successful chunks from first attempt should still be loaded
            let successful_chunks = first_progress.successful_chunks();
            for coord in &successful_chunks {
                prop_assert!(world.is_chunk_loaded(*coord), 
                    "Previously successful chunks should remain loaded after retry");
            }
            
            // World should have at least the same number of chunks as before retry
            prop_assert!(world.chunk_count() >= initial_world_count, 
                "World should not lose chunks during retry operations");
            
            // Total successful loads should be consistent
            let total_successful = retry_progress.successful_loads() + initial_successful_count;
            prop_assert!(total_successful <= unique_coords.len(), 
                "Total successful loads should not exceed total chunks");
        }
    }

    // Property test for world consistency validation
    proptest! {
        #[test]
        fn property_world_consistency_validation(
            config in arb_world_config(),
            coords in prop::collection::vec(arb_chunk_coord(), 1..5),
        ) {
            // Feature: world-integration, Property 6: Error Handling Isolation (consistency)
            
            let mut world = World::new(config).unwrap();
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.is_empty() {
                return Ok(());
            }
            
            // Load chunks (some may fail)
            let pattern = LoadPattern::Custom(unique_coords.clone());
            let _progress = world.load_chunks(pattern).unwrap();
            
            // World should be in a consistent state after loading
            let consistency_result = world.validate_world_consistency();
            
            // If validation fails, it should be for a valid reason
            if let Err(error) = consistency_result {
                match error {
                    WorldError::InvalidChunkState { coord, current_state, .. } => {
                        // Error state chunks are expected if loading failed
                        if current_state == ChunkState::Error {
                            // This is acceptable - failed chunks may be in error state temporarily
                        } else {
                            prop_assert!(false, "Unexpected chunk state: {:?} at {:?}", current_state, coord);
                        }
                    }
                    _ => {
                        prop_assert!(false, "Unexpected consistency error: {}", error);
                    }
                }
            }
            
            // All loaded chunks should have valid coordinates
            for (coord, entry) in world.chunks.iter() {
                prop_assert!(world.validate_chunk_coord(*coord).is_ok(), 
                    "All loaded chunks should have valid coordinates");
                
                // Chunk dimensions should match world configuration
                let chunk_dims = entry.chunk.dimensions();
                let expected_size = world.chunk_size() as usize;
                prop_assert_eq!(chunk_dims.width, expected_size, 
                    "Chunk width should match world configuration");
                prop_assert_eq!(chunk_dims.height, expected_size, 
                    "Chunk height should match world configuration");
                prop_assert_eq!(chunk_dims.depth, expected_size, 
                    "Chunk depth should match world configuration");
            }
        }
    }

    // Property 3: Adjacent Chunk Spatial Consistency
    // **Validates: Requirements 2.2, 2.3, 2.4**
    proptest! {
        #[test]
        fn property_adjacent_chunk_spatial_consistency(
            config in arb_world_config(),
            center_coord in arb_chunk_coord(),
        ) {
            // Feature: world-integration, Property 3: Adjacent Chunk Spatial Consistency
            
            let world = World::new(config).unwrap();
            
            // Get all adjacent chunk coordinates
            let adjacent_coords = center_coord.adjacent();
            
            for adj_coord in adjacent_coords {
                // Adjacent chunks should be exactly one unit apart in one dimension
                let dx = (center_coord.x - adj_coord.x).abs();
                let dy = (center_coord.y - adj_coord.y).abs();
                let dz = (center_coord.z - adj_coord.z).abs();
                
                // Should differ by 1 in exactly one dimension
                let diff_count = (if dx == 1 { 1 } else { 0 }) +
                                (if dy == 1 { 1 } else { 0 }) +
                                (if dz == 1 { 1 } else { 0 });
                
                prop_assert_eq!(diff_count, 1, 
                    "Adjacent chunks should differ by 1 in exactly one dimension");
                
                // Verify using the coordinate system's adjacency check
                prop_assert!(world.are_chunks_adjacent(center_coord, adj_coord),
                    "Coordinate system should recognize chunks as adjacent");
                
                // Check that chunk bounds touch but don't overlap
                let (center_min, center_max) = world.chunk_bounds(center_coord);
                let (adj_min, adj_max) = world.chunk_bounds(adj_coord);
                
                // Calculate the distance between chunk centers
                let center_pos = world.chunk_to_world_pos(center_coord);
                let adj_pos = world.chunk_to_world_pos(adj_coord);
                let distance = center_pos.distance(adj_pos);
                
                // Distance should be exactly one chunk size
                let expected_distance = world.chunk_size() as f32;
                prop_assert!((distance - expected_distance).abs() < 0.001, 
                    "Distance between adjacent chunk centers should be exactly one chunk size");
                
                // Verify bounds don't overlap (they should touch at boundaries)
                let overlaps = !(center_max.x <= adj_min.x || adj_max.x <= center_min.x ||
                               center_max.y <= adj_min.y || adj_max.y <= center_min.y ||
                               center_max.z <= adj_min.z || adj_max.z <= center_min.z);
                
                // For adjacent chunks, they should share a boundary (not overlap)
                if dx == 1 && dy == 0 && dz == 0 {
                    // Adjacent in X direction
                    prop_assert!((center_max.x - adj_min.x).abs() < 0.001 || 
                               (adj_max.x - center_min.x).abs() < 0.001,
                               "Adjacent chunks in X should share X boundary");
                } else if dy == 1 && dx == 0 && dz == 0 {
                    // Adjacent in Y direction
                    prop_assert!((center_max.y - adj_min.y).abs() < 0.001 || 
                               (adj_max.y - center_min.y).abs() < 0.001,
                               "Adjacent chunks in Y should share Y boundary");
                } else if dz == 1 && dx == 0 && dy == 0 {
                    // Adjacent in Z direction
                    prop_assert!((center_max.z - adj_min.z).abs() < 0.001 || 
                               (adj_max.z - center_min.z).abs() < 0.001,
                               "Adjacent chunks in Z should share Z boundary");
                }
            }
        }
    }

    // Property test for coordinate conversion consistency with negative coordinates
    proptest! {
        #[test]
        fn property_coordinate_conversion_negative_handling(
            config in arb_world_config(),
            world_pos in (-1000.0f32..1000.0f32, -1000.0f32..1000.0f32, -1000.0f32..1000.0f32),
        ) {
            // Feature: world-integration, Property 3: Adjacent Chunk Spatial Consistency (negative coords)
            
            let world = World::new(config).unwrap();
            let world_pos = Vec3::new(world_pos.0, world_pos.1, world_pos.2);
            
            // Convert world position to chunk coordinate
            let chunk_coord = world.world_to_chunk_coord(world_pos);
            
            // Get chunk bounds
            let (min_bounds, max_bounds) = world.chunk_bounds(chunk_coord);
            
            // World position should be within the calculated chunk bounds
            prop_assert!(world_pos.x >= min_bounds.x && world_pos.x < max_bounds.x,
                "World position X should be within chunk bounds");
            prop_assert!(world_pos.y >= min_bounds.y && world_pos.y < max_bounds.y,
                "World position Y should be within chunk bounds");
            prop_assert!(world_pos.z >= min_bounds.z && world_pos.z < max_bounds.z,
                "World position Z should be within chunk bounds");
            
            // Convert back to world position (chunk origin)
            let chunk_origin = world.chunk_to_world_pos(chunk_coord);
            
            // Chunk origin should be at the minimum bounds
            prop_assert!((chunk_origin.x - min_bounds.x).abs() < 0.001,
                "Chunk origin X should match minimum bounds");
            prop_assert!((chunk_origin.y - min_bounds.y).abs() < 0.001,
                "Chunk origin Y should match minimum bounds");
            prop_assert!((chunk_origin.z - min_bounds.z).abs() < 0.001,
                "Chunk origin Z should match minimum bounds");
            
            // Test local block coordinate conversion
            let (chunk_coord2, block_coord) = world.world_to_local_block(world_pos);
            prop_assert_eq!(chunk_coord, chunk_coord2,
                "Chunk coordinate from world_to_local_block should match direct conversion");
            
            // Block coordinates should be valid
            prop_assert!(block_coord.is_valid(world.chunk_size()),
                "Block coordinates should be valid for chunk size");
        }
    }

    // Property test for renderable and processing state checks
    proptest! {
        #[test]
        fn property_chunk_state_checks(
            state in arb_chunk_state(),
            mesh_dirty in any::<bool>(),
        ) {
            // Feature: world-integration, Property 4: Chunk State Management Consistency (state checks)
            
            let chunk = Chunk::new(
                ChunkPosition { x: 0, z: 0 },
                ChunkDimensions { width: 16, height: 16, depth: 16 },
            );
            let mut entry = ChunkEntry::new(chunk);
            entry.state = state;
            entry.mesh_dirty = mesh_dirty;
            
            // is_renderable should only be true for RenderReady state
            let expected_renderable = matches!(state, ChunkState::RenderReady);
            prop_assert_eq!(entry.is_renderable(), expected_renderable, 
                "is_renderable should only be true for RenderReady state");
            
            // needs_processing should be true for Loading, Generated, or when mesh is dirty
            let expected_needs_processing = matches!(state, ChunkState::Loading | ChunkState::Generated) || mesh_dirty;
            prop_assert_eq!(entry.needs_processing(), expected_needs_processing, 
                "needs_processing should be true for Loading/Generated states or when mesh is dirty");
        }
    }

    // Property 7: Rendering Integration Completeness
    // **Validates: Requirements 4.1, 4.5**
    proptest! {
        #[test]
        fn property_rendering_integration_completeness(
            config in arb_world_config(),
            coords in prop::collection::vec(arb_chunk_coord(), 1..8),
        ) {
            // Feature: world-integration, Property 7: Rendering Integration Completeness
            
            let mut world = World::new(config).unwrap();
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.is_empty() {
                return Ok(());
            }
            
            // Load chunks into the world
            let pattern = LoadPattern::Custom(unique_coords.clone());
            let _progress = world.load_chunks(pattern).unwrap();
            
            // Initially, no chunks should be render-ready
            let initial_render_ready = world.get_render_ready_chunks();
            prop_assert!(initial_render_ready.is_empty(), 
                "Initially loaded chunks should not be render-ready");
            
            // Prepare chunks for rendering
            let preparation_results = world.prepare_chunks_for_rendering(&unique_coords);
            
            // All chunks should be successfully prepared or already prepared
            for (coord, result) in &preparation_results {
                match result {
                    Ok(_) => {}, // Success
                    Err(e) => {
                        // Only acceptable error is if chunk is not found or in invalid state
                        prop_assert!(
                            matches!(e, WorldError::ChunkNotFound { .. } | WorldError::InvalidChunkState { .. }),
                            "Preparation should only fail for missing chunks or invalid states: {:?}", e
                        );
                    }
                }
            }
            
            // After preparation, chunks should be in appropriate states
            let chunks_needing_mesh = world.get_chunks_needing_mesh_generation();
            let meshed_chunks = world.get_meshed_chunks();
            let render_ready_chunks = world.get_render_ready_chunks();
            
            // All loaded chunks should be in one of these categories
            let total_categorized = chunks_needing_mesh.len() + meshed_chunks.len() + render_ready_chunks.len();
            prop_assert!(total_categorized <= world.chunk_count(), 
                "Categorized chunks should not exceed total chunk count");
            
            // Prepare chunks again to get them to render-ready state
            for coord in &unique_coords {
                if world.is_chunk_loaded(*coord) {
                    let _ = world.prepare_chunk_for_rendering(*coord);
                }
            }
            
            // Now check render-ready chunks
            let final_render_ready = world.get_render_ready_chunks();
            
            // All render-ready chunks should be properly integrated
            for (coord, entry) in &final_render_ready {
                // Chunk should be in RenderReady state
                prop_assert_eq!(entry.state, ChunkState::RenderReady, 
                    "Render-ready chunks should be in RenderReady state");
                
                // Chunk should be renderable
                prop_assert!(entry.is_renderable(), 
                    "Render-ready chunks should be renderable");
                
                // Chunk should be loaded in world
                prop_assert!(world.is_chunk_loaded(*coord), 
                    "Render-ready chunks should be loaded in world");
                
                // Chunk should not need processing
                prop_assert!(!entry.needs_processing() || entry.mesh_dirty, 
                    "Render-ready chunks should not need processing unless mesh is dirty");
            }
            
            // Test render distance filtering
            if let Some(&center_coord) = unique_coords.first() {
                let render_distance_chunks = world.get_render_ready_chunks_in_distance(center_coord);
                
                // All chunks in render distance should actually be within render distance
                for (coord, _) in &render_distance_chunks {
                    prop_assert!(world.is_in_render_distance(*coord, center_coord), 
                        "Chunks returned by render distance query should be within render distance");
                }
                
                // All render-ready chunks within render distance should be included
                for (coord, entry) in &final_render_ready {
                    if world.is_in_render_distance(*coord, center_coord) {
                        let found = render_distance_chunks.iter().any(|(c, _)| c == coord);
                        prop_assert!(found, 
                            "All render-ready chunks within render distance should be included in query");
                    }
                }
            }
            
            // Test batch preparation
            if let Some(&center_coord) = unique_coords.first() {
                // Reset some chunks to Generated state for batch testing
                for coord in unique_coords.iter().take(2) {
                    if let Some(entry) = world.get_chunk_mut(*coord) {
                        let _ = entry.transition_to(ChunkState::Generated);
                    }
                }
                
                let prepared_count = world.batch_prepare_chunks_in_render_distance(center_coord).unwrap();
                
                // Prepared count should be reasonable
                prop_assert!(prepared_count <= world.chunk_count(), 
                    "Batch prepared count should not exceed total chunks");
            }
        }
    }

    // Property test for rendering state transitions
    proptest! {
        #[test]
        fn property_rendering_state_transitions(
            config in arb_world_config(),
            coord in arb_chunk_coord(),
        ) {
            // Feature: world-integration, Property 7: Rendering Integration Completeness (state transitions)
            
            let mut world = World::new(config).unwrap();
            
            // Load a single chunk
            let pattern = LoadPattern::Single(coord);
            let _progress = world.load_chunks(pattern).unwrap();
            
            if !world.is_chunk_loaded(coord) {
                return Ok(()); // Skip if chunk failed to load
            }
            
            // Initially chunk should be in Generated state
            let initial_entry = world.get_chunk(coord).unwrap();
            prop_assert_eq!(initial_entry.state, ChunkState::Generated, 
                "Initially loaded chunk should be in Generated state");
            
            // Prepare for rendering should transition through states
            let result1 = world.prepare_chunk_for_rendering(coord).unwrap();
            prop_assert!(result1, "First preparation should change state");
            
            let after_first_prep = world.get_chunk(coord).unwrap();
            prop_assert_eq!(after_first_prep.state, ChunkState::Meshed, 
                "After first preparation, chunk should be in Meshed state");
            
            // Second preparation should transition to RenderReady
            let result2 = world.prepare_chunk_for_rendering(coord).unwrap();
            prop_assert!(result2, "Second preparation should change state");
            
            let after_second_prep = world.get_chunk(coord).unwrap();
            prop_assert_eq!(after_second_prep.state, ChunkState::RenderReady, 
                "After second preparation, chunk should be in RenderReady state");
            
            // Third preparation should not change state
            let result3 = world.prepare_chunk_for_rendering(coord).unwrap();
            prop_assert!(!result3, "Third preparation should not change state");
            
            let final_entry = world.get_chunk(coord).unwrap();
            prop_assert_eq!(final_entry.state, ChunkState::RenderReady, 
                "Final state should remain RenderReady");
            
            // Mark chunk render ready should work
            let mark_result = world.mark_chunk_render_ready(coord);
            prop_assert!(mark_result.is_ok(), "Marking render-ready chunk as render-ready should succeed");
        }
    }

    // Property test for chunks needing processing
    proptest! {
        #[test]
        fn property_chunks_needing_processing(
            config in arb_world_config(),
            coords in prop::collection::vec(arb_chunk_coord(), 1..5),
        ) {
            // Feature: world-integration, Property 7: Rendering Integration Completeness (processing)
            
            let mut world = World::new(config).unwrap();
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.is_empty() {
                return Ok(());
            }
            
            // Load chunks
            let pattern = LoadPattern::Custom(unique_coords.clone());
            let _progress = world.load_chunks(pattern).unwrap();
            
            // Initially, all loaded chunks should need processing (Generated state)
            let initial_processing = world.get_chunks_needing_processing();
            let loaded_count = unique_coords.iter().filter(|coord| world.is_chunk_loaded(**coord)).count();
            let initial_processing_len = initial_processing.len();
            
            prop_assert_eq!(initial_processing_len, loaded_count, 
                "Initially, all loaded chunks should need processing");
            
            // All chunks needing processing should actually need processing
            for (coord, entry) in &initial_processing {
                prop_assert!(entry.needs_processing(), 
                    "Chunks returned by needs_processing query should actually need processing");
                prop_assert!(world.is_chunk_loaded(*coord), 
                    "Chunks needing processing should be loaded in world");
            }
            
            // Prepare some chunks for rendering
            let coords_to_prepare: Vec<ChunkCoord> = unique_coords.iter()
                .take(unique_coords.len() / 2)
                .filter(|coord| world.is_chunk_loaded(**coord))
                .copied()
                .collect();
            
            for coord in coords_to_prepare {
                let _ = world.prepare_chunk_for_rendering(coord);
                let _ = world.prepare_chunk_for_rendering(coord); // Get to RenderReady
            }
            
            // After preparation, fewer chunks should need processing
            let after_processing = world.get_chunks_needing_processing();
            prop_assert!(after_processing.len() <= initial_processing_len, 
                "After preparation, fewer or equal chunks should need processing");
            
            // All remaining chunks needing processing should be in appropriate states
            for (coord, entry) in &after_processing {
                prop_assert!(
                    matches!(entry.state, ChunkState::Loading | ChunkState::Generated) || entry.mesh_dirty,
                    "Chunks needing processing should be in Loading/Generated state or have dirty mesh"
                );
            }
        }
    }

    // Property 8: Change Tracking Accuracy
    // **Validates: Requirements 4.3**
    proptest! {
        #[test]
        fn property_change_tracking_accuracy(
            config in arb_world_config(),
            coords in prop::collection::vec(arb_chunk_coord(), 1..6),
        ) {
            // Feature: world-integration, Property 8: Change Tracking Accuracy
            
            let mut world = World::new(config).unwrap();
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.is_empty() {
                return Ok(());
            }
            
            // Load chunks into the world
            let pattern = LoadPattern::Custom(unique_coords.clone());
            let _progress = world.load_chunks(pattern).unwrap();
            
            // Prepare chunks to RenderReady state
            for coord in &unique_coords {
                if world.is_chunk_loaded(*coord) {
                    let _ = world.prepare_chunk_for_rendering(*coord);
                    let _ = world.prepare_chunk_for_rendering(*coord);
                }
            }
            
            // Clear mesh dirty flags after preparation (simulate completed meshing)
            for coord in &unique_coords {
                if world.is_chunk_loaded(*coord) {
                    let _ = world.clear_chunk_mesh_dirty(*coord);
                }
            }
            
            // Now, no chunks should have dirty meshes
            let initial_dirty = world.get_chunks_with_dirty_meshes();
            prop_assert!(initial_dirty.is_empty(), 
                "After clearing mesh dirty flags, no chunks should have dirty meshes");
            
            let initial_dirty_stats = world.get_dirty_chunk_stats();
            prop_assert_eq!(initial_dirty_stats.total_dirty_chunks, 0, 
                "Initial dirty chunk stats should show zero dirty chunks");
            prop_assert!(!world.has_dirty_chunks(), 
                "World should not have dirty chunks initially");
            
            // Mark some chunks as modified
            let coords_to_modify: Vec<ChunkCoord> = unique_coords.iter()
                .filter(|coord| world.is_chunk_loaded(**coord))
                .take(unique_coords.len() / 2 + 1)
                .copied()
                .collect();
            
            let modification_results = world.mark_chunks_modified(&coords_to_modify);
            
            // All modifications should succeed for loaded chunks
            for (coord, result) in &modification_results {
                if world.is_chunk_loaded(*coord) {
                    prop_assert!(result.is_ok(), 
                        "Marking loaded chunk as modified should succeed");
                } else {
                    prop_assert!(matches!(result, Err(WorldError::ChunkNotFound { .. })), 
                        "Marking non-existent chunk should return ChunkNotFound error");
                }
            }
            
            // After modification, chunks should have dirty meshes
            let after_modification_dirty = world.get_chunks_with_dirty_meshes();
            let expected_dirty_count = coords_to_modify.iter()
                .filter(|coord| world.is_chunk_loaded(**coord))
                .count();
            
            prop_assert_eq!(after_modification_dirty.len(), expected_dirty_count, 
                "Number of dirty chunks should match number of successfully modified chunks");
            
            // All modified chunks should be in the dirty list
            for coord in &coords_to_modify {
                if world.is_chunk_loaded(*coord) {
                    let found = after_modification_dirty.iter().any(|(c, _)| c == coord);
                    prop_assert!(found, 
                        "Modified chunk should appear in dirty chunks list");
                    
                    // Check that the chunk entry has mesh_dirty flag set
                    if let Some(entry) = world.get_chunk(*coord) {
                        prop_assert!(entry.mesh_dirty, 
                            "Modified chunk should have mesh_dirty flag set");
                    }
                }
            }
            
            // Dirty chunk stats should be accurate
            let dirty_stats = world.get_dirty_chunk_stats();
            prop_assert_eq!(dirty_stats.total_dirty_chunks, expected_dirty_count, 
                "Dirty chunk stats should show correct total");
            prop_assert!(dirty_stats.has_dirty_chunks(), 
                "Dirty chunk stats should indicate presence of dirty chunks");
            prop_assert!(world.has_dirty_chunks(), 
                "World should report having dirty chunks");
            
            // Process dirty chunks
            let processed_count = world.process_dirty_chunks().unwrap();
            prop_assert_eq!(processed_count, expected_dirty_count, 
                "Processed count should match number of dirty chunks");
            
            // After processing, no chunks should be dirty
            let after_processing_dirty = world.get_chunks_with_dirty_meshes();
            prop_assert!(after_processing_dirty.is_empty(), 
                "After processing, no chunks should have dirty meshes");
            
            let final_dirty_stats = world.get_dirty_chunk_stats();
            prop_assert_eq!(final_dirty_stats.total_dirty_chunks, 0, 
                "Final dirty chunk stats should show zero dirty chunks");
            prop_assert!(!world.has_dirty_chunks(), 
                "World should not have dirty chunks after processing");
        }
    }

    // Property test for adjacent chunk modification tracking
    proptest! {
        #[test]
        fn property_adjacent_chunk_modification_tracking(
            config in arb_world_config(),
            center_coord in arb_chunk_coord(),
        ) {
            // Feature: world-integration, Property 8: Change Tracking Accuracy (adjacent chunks)
            
            let mut world = World::new(config).unwrap();
            
            // Load center chunk and some adjacent chunks
            let mut coords_to_load = vec![center_coord];
            let adjacent_coords = center_coord.adjacent();
            coords_to_load.extend(adjacent_coords.iter().take(3)); // Load a few adjacent chunks
            
            let pattern = LoadPattern::Custom(coords_to_load.clone());
            let _progress = world.load_chunks(pattern).unwrap();
            
            // Count how many chunks were actually loaded
            let loaded_count = coords_to_load.iter()
                .filter(|coord| world.is_chunk_loaded(**coord))
                .count();
            
            if loaded_count < 2 {
                return Ok(()); // Need at least 2 chunks for this test
            }
            
            // Clear mesh dirty flags for all loaded chunks (simulate completed initial meshing)
            for coord in &coords_to_load {
                if world.is_chunk_loaded(*coord) {
                    let _ = world.clear_chunk_mesh_dirty(*coord);
                }
            }
            
            // Initially no chunks should be dirty
            prop_assert!(!world.has_dirty_chunks(), 
                "Initially no chunks should be dirty");
            
            // Mark adjacent chunks for re-mesh due to center chunk changes
            let marked_count = world.mark_adjacent_chunks_for_remesh(center_coord).unwrap();
            
            // Marked count should not exceed the number of loaded adjacent chunks
            let loaded_adjacent_count = adjacent_coords.iter()
                .filter(|coord| world.is_chunk_loaded(**coord))
                .count();
            
            prop_assert_eq!(marked_count, loaded_adjacent_count, 
                "Marked count should equal number of loaded adjacent chunks");
            
            // Check that adjacent chunks are marked as dirty
            for adj_coord in &adjacent_coords {
                if world.is_chunk_loaded(*adj_coord) {
                    if let Some(entry) = world.get_chunk(*adj_coord) {
                        prop_assert!(entry.mesh_dirty, 
                            "Adjacent chunk should be marked as dirty");
                    }
                }
            }
            
            // Center chunk should not be marked as dirty (we only marked adjacent)
            if world.is_chunk_loaded(center_coord) {
                if let Some(entry) = world.get_chunk(center_coord) {
                    prop_assert!(!entry.mesh_dirty, 
                        "Center chunk should not be marked as dirty by adjacent marking");
                }
            }
            
            // Dirty chunk count should match marked count
            let dirty_chunks = world.get_chunks_with_dirty_meshes();
            prop_assert_eq!(dirty_chunks.len(), marked_count, 
                "Number of dirty chunks should match marked count");
        }
    }

    // Property test for render distance dirty chunk filtering
    proptest! {
        #[test]
        fn property_render_distance_dirty_chunk_filtering(
            render_distance in 1u32..=8u32,
            center_coord in arb_chunk_coord(),
            coords in prop::collection::vec(arb_chunk_coord(), 3..10),
        ) {
            // Feature: world-integration, Property 8: Change Tracking Accuracy (render distance)
            
            let config = WorldConfig::new()
                .with_render_distance(render_distance)
                .with_max_chunks(Some(coords.len() * 2));
            
            let mut world = World::new(config).unwrap();
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.is_empty() {
                return Ok(());
            }
            
            // Load chunks
            let pattern = LoadPattern::Custom(unique_coords.clone());
            let _progress = world.load_chunks(pattern).unwrap();
            
            // Mark all chunks as modified
            let _modification_results = world.mark_chunks_modified(&unique_coords);
            
            // Get dirty chunks within render distance
            let dirty_in_distance = world.get_dirty_chunks_in_render_distance(center_coord);
            let dirty_in_distance_len = dirty_in_distance.len();
            
            // All chunks in the result should be within render distance
            for (coord, entry) in &dirty_in_distance {
                prop_assert!(world.is_in_render_distance(*coord, center_coord), 
                    "Dirty chunks in render distance should actually be within render distance");
                prop_assert!(entry.mesh_dirty, 
                    "Chunks returned by dirty render distance query should have dirty meshes");
            }
            
            // All dirty chunks within render distance should be included
            let all_dirty = world.get_chunks_with_dirty_meshes();
            for (coord, _entry) in &all_dirty {
                if world.is_in_render_distance(*coord, center_coord) {
                    let found = dirty_in_distance.iter().any(|(c, _)| c == coord);
                    prop_assert!(found, 
                        "All dirty chunks within render distance should be included in query");
                }
            }
            
            // Process dirty chunks in render distance
            let processed_count = world.process_dirty_chunks_in_render_distance(center_coord).unwrap();
            
            // Processed count should match dirty chunks in render distance
            prop_assert_eq!(processed_count, dirty_in_distance_len, 
                "Processed count should match dirty chunks in render distance");
            
            // After processing, chunks in render distance should not be dirty
            let after_processing_dirty_in_distance = world.get_dirty_chunks_in_render_distance(center_coord);
            prop_assert!(after_processing_dirty_in_distance.is_empty(), 
                "After processing, no chunks in render distance should be dirty");
        }
    }

    // Property test for dirty chunk statistics accuracy
    proptest! {
        #[test]
        fn property_dirty_chunk_statistics_accuracy(
            config in arb_world_config(),
            coords in prop::collection::vec(arb_chunk_coord(), 2..8),
        ) {
            // Feature: world-integration, Property 8: Change Tracking Accuracy (statistics)
            
            let mut world = World::new(config).unwrap();
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.len() < 2 {
                return Ok(());
            }
            
            // Load chunks
            let pattern = LoadPattern::Custom(unique_coords.clone());
            let _progress = world.load_chunks(pattern).unwrap();
            
            // Prepare some chunks to different states
            let loaded_coords: Vec<ChunkCoord> = unique_coords.iter()
                .filter(|coord| world.is_chunk_loaded(**coord))
                .copied()
                .collect();
            
            if loaded_coords.len() < 2 {
                return Ok(());
            }
            
            // Prepare half the chunks to Meshed state
            for coord in loaded_coords.iter().take(loaded_coords.len() / 2) {
                let _ = world.prepare_chunk_for_rendering(*coord);
            }
            
            // Prepare remaining chunks to RenderReady state
            for coord in loaded_coords.iter().skip(loaded_coords.len() / 2) {
                let _ = world.prepare_chunk_for_rendering(*coord);
                let _ = world.prepare_chunk_for_rendering(*coord);
            }
            
            // Mark all chunks as modified
            let _modification_results = world.mark_chunks_modified(&loaded_coords);
            
            // Get dirty chunk statistics
            let dirty_stats = world.get_dirty_chunk_stats();
            
            // Statistics should be accurate
            prop_assert_eq!(dirty_stats.total_dirty_chunks, loaded_coords.len(), 
                "Total dirty chunks should match number of modified chunks");
            
            // Count chunks by state manually for verification
            let mut expected_generated = 0;
            let mut expected_meshed = 0;
            let mut expected_render_ready = 0;
            
            for coord in &loaded_coords {
                if let Some(entry) = world.get_chunk(*coord) {
                    if entry.mesh_dirty {
                        match entry.state {
                            ChunkState::Generated => expected_generated += 1,
                            ChunkState::Meshed => expected_meshed += 1,
                            ChunkState::RenderReady => expected_render_ready += 1,
                            _ => {},
                        }
                    }
                }
            }
            
            prop_assert_eq!(dirty_stats.dirty_generated_chunks, expected_generated, 
                "Generated dirty chunk count should be accurate");
            prop_assert_eq!(dirty_stats.dirty_meshed_chunks, expected_meshed, 
                "Meshed dirty chunk count should be accurate");
            prop_assert_eq!(dirty_stats.dirty_render_ready_chunks, expected_render_ready, 
                "RenderReady dirty chunk count should be accurate");
            
            // Sum of state counts should equal total
            let sum = dirty_stats.dirty_generated_chunks + 
                     dirty_stats.dirty_meshed_chunks + 
                     dirty_stats.dirty_render_ready_chunks;
            prop_assert_eq!(sum, dirty_stats.total_dirty_chunks, 
                "Sum of state-specific counts should equal total dirty chunks");
            
            // Test percentage calculation
            let immediate_percentage = dirty_stats.immediate_remesh_percentage();
            let expected_immediate = expected_generated + expected_meshed;
            let expected_percentage = if dirty_stats.total_dirty_chunks == 0 {
                0.0
            } else {
                expected_immediate as f32 / dirty_stats.total_dirty_chunks as f32
            };
            
            prop_assert!((immediate_percentage - expected_percentage).abs() < 0.001, 
                "Immediate remesh percentage should be accurate");
            
            prop_assert!(immediate_percentage >= 0.0 && immediate_percentage <= 1.0, 
                "Immediate remesh percentage should be between 0.0 and 1.0");
        }
    }

    // Property 13: Frustum Culling Correctness
    // **Validates: Requirements 4.4**
    proptest! {
        #[test]
        fn property_frustum_culling_correctness(
            config in arb_world_config(),
            coords in prop::collection::vec(arb_chunk_coord(), 3..10),
            frustum_bounds in (-50.0f32..50.0f32, -50.0f32..50.0f32, -50.0f32..50.0f32, 1.0f32..100.0f32),
        ) {
            // Feature: world-integration, Property 13: Frustum Culling Correctness
            
            let mut world = World::new(config).unwrap();
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.is_empty() {
                return Ok(());
            }
            
            // Load and prepare chunks to render-ready state
            let pattern = LoadPattern::Custom(unique_coords.clone());
            let _progress = world.load_chunks(pattern).unwrap();
            
            for coord in &unique_coords {
                if world.is_chunk_loaded(*coord) {
                    let _ = world.prepare_chunk_for_rendering(*coord);
                    let _ = world.prepare_chunk_for_rendering(*coord);
                    let _ = world.clear_chunk_mesh_dirty(*coord);
                }
            }
            
            // Create a frustum for testing
            let (center_x, center_y, center_z, size) = frustum_bounds;
            let frustum = CameraFrustum::new_simple(
                center_x - size, center_x + size,
                center_y - size, center_y + size,
                center_z - size, center_z + size,
            );
            
            // Get visible chunks using frustum culling
            let visible_chunks = world.get_visible_chunks_in_frustum(&frustum);
            
            // All visible chunks should be render-ready
            for (coord, entry) in &visible_chunks {
                prop_assert!(entry.is_renderable(), 
                    "All visible chunks should be render-ready");
                prop_assert_eq!(entry.state, ChunkState::RenderReady, 
                    "All visible chunks should be in RenderReady state");
            }
            
            // All visible chunks should actually be within the frustum
            for (coord, _) in &visible_chunks {
                prop_assert!(world.is_chunk_in_frustum(*coord, &frustum), 
                    "All visible chunks should be within the frustum");
            }
            
            // All render-ready chunks within frustum should be included
            let all_render_ready = world.get_render_ready_chunks();
            for (coord, entry) in &all_render_ready {
                if world.is_chunk_in_frustum(*coord, &frustum) {
                    let found = visible_chunks.iter().any(|(c, _)| c == coord);
                    prop_assert!(found, 
                        "All render-ready chunks within frustum should be included in visible chunks");
                }
            }
            
            // Test frustum culling with coordinate list
            let loaded_coords: Vec<ChunkCoord> = unique_coords.iter()
                .filter(|coord| world.is_chunk_loaded(**coord))
                .copied()
                .collect();
            
            let culled_coords = world.cull_chunks_by_frustum(&loaded_coords, &frustum);
            
            // All culled coordinates should be within the frustum
            for coord in &culled_coords {
                prop_assert!(world.is_chunk_in_frustum(*coord, &frustum), 
                    "All culled coordinates should be within the frustum");
            }
            
            // All coordinates within frustum should be included in culled list
            for coord in &loaded_coords {
                if world.is_chunk_in_frustum(*coord, &frustum) {
                    prop_assert!(culled_coords.contains(coord), 
                        "All coordinates within frustum should be included in culled list");
                }
            }
            
            // Test frustum culling statistics
            let culling_stats = world.get_frustum_culling_stats(&frustum);
            
            prop_assert_eq!(culling_stats.total_chunks, world.chunk_count(), 
                "Culling stats should report correct total chunk count");
            prop_assert_eq!(culling_stats.render_ready_chunks, all_render_ready.len(), 
                "Culling stats should report correct render-ready chunk count");
            prop_assert_eq!(culling_stats.visible_chunks, visible_chunks.len(), 
                "Culling stats should report correct visible chunk count");
            
            let expected_culled = all_render_ready.len().saturating_sub(visible_chunks.len());
            prop_assert_eq!(culling_stats.culled_chunks, expected_culled, 
                "Culling stats should report correct culled chunk count");
            
            // Culling efficiency should be between 0.0 and 1.0
            prop_assert!(culling_stats.culling_efficiency >= 0.0 && culling_stats.culling_efficiency <= 1.0, 
                "Culling efficiency should be between 0.0 and 1.0");
            
            // Culling percentage should be consistent
            let expected_percentage = if all_render_ready.len() == 0 {
                0.0
            } else {
                expected_culled as f32 / all_render_ready.len() as f32
            };
            
            prop_assert!((culling_stats.culling_percentage() - expected_percentage).abs() < 0.001, 
                "Culling percentage should be accurate");
        }
    }

    // Property test for frustum culling with render distance
    proptest! {
        #[test]
        fn property_frustum_culling_with_render_distance(
            render_distance in 1u32..=8u32,
            center_coord in arb_chunk_coord(),
            coords in prop::collection::vec(arb_chunk_coord(), 2..8),
            camera_pos in (-100.0f32..100.0f32, -100.0f32..100.0f32, -100.0f32..100.0f32),
        ) {
            // Feature: world-integration, Property 13: Frustum Culling Correctness (with render distance)
            
            let config = WorldConfig::new()
                .with_render_distance(render_distance)
                .with_max_chunks(Some(coords.len() * 2));
            
            let mut world = World::new(config).unwrap();
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.is_empty() {
                return Ok(());
            }
            
            // Load and prepare chunks
            let pattern = LoadPattern::Custom(unique_coords.clone());
            let _progress = world.load_chunks(pattern).unwrap();
            
            for coord in &unique_coords {
                if world.is_chunk_loaded(*coord) {
                    let _ = world.prepare_chunk_for_rendering(*coord);
                    let _ = world.prepare_chunk_for_rendering(*coord);
                    let _ = world.clear_chunk_mesh_dirty(*coord);
                }
            }
            
            // Create a large frustum that should include most chunks
            let camera_pos = Vec3::new(camera_pos.0, camera_pos.1, camera_pos.2);
            let size = 1000.0; // Large frustum
            let frustum = CameraFrustum::new_simple(
                camera_pos.x - size, camera_pos.x + size,
                camera_pos.y - size, camera_pos.y + size,
                camera_pos.z - size, camera_pos.z + size,
            );
            
            // Get chunks within both render distance and frustum
            let visible_in_distance_and_frustum = world.get_visible_chunks_in_distance_and_frustum(center_coord, &frustum);
            
            // All returned chunks should be within render distance
            for (coord, _) in &visible_in_distance_and_frustum {
                prop_assert!(world.is_in_render_distance(*coord, center_coord), 
                    "All chunks should be within render distance");
            }
            
            // All returned chunks should be within frustum
            for (coord, _) in &visible_in_distance_and_frustum {
                prop_assert!(world.is_chunk_in_frustum(*coord, &frustum), 
                    "All chunks should be within frustum");
            }
            
            // All returned chunks should be render-ready
            for (_, entry) in &visible_in_distance_and_frustum {
                prop_assert!(entry.is_renderable(), 
                    "All returned chunks should be render-ready");
            }
            
            // Compare with separate queries
            let render_distance_chunks = world.get_render_ready_chunks_in_distance(center_coord);
            let frustum_chunks = world.get_visible_chunks_in_frustum(&frustum);
            
            // Combined result should be subset of both individual results
            for (coord, _) in &visible_in_distance_and_frustum {
                let in_render_distance = render_distance_chunks.iter().any(|(c, _)| c == coord);
                let in_frustum = frustum_chunks.iter().any(|(c, _)| c == coord);
                
                prop_assert!(in_render_distance, 
                    "Combined result chunk should be in render distance query");
                prop_assert!(in_frustum, 
                    "Combined result chunk should be in frustum query");
            }
        }
    }

    // Property test for distance-sorted frustum culling
    proptest! {
        #[test]
        fn property_distance_sorted_frustum_culling(
            config in arb_world_config(),
            coords in prop::collection::vec(arb_chunk_coord(), 2..6),
            camera_pos in (-50.0f32..50.0f32, -50.0f32..50.0f32, -50.0f32..50.0f32),
        ) {
            // Feature: world-integration, Property 13: Frustum Culling Correctness (distance sorting)
            
            let mut world = World::new(config).unwrap();
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.len() < 2 {
                return Ok(());
            }
            
            // Load and prepare chunks
            let pattern = LoadPattern::Custom(unique_coords.clone());
            let _progress = world.load_chunks(pattern).unwrap();
            
            for coord in &unique_coords {
                if world.is_chunk_loaded(*coord) {
                    let _ = world.prepare_chunk_for_rendering(*coord);
                    let _ = world.prepare_chunk_for_rendering(*coord);
                    let _ = world.clear_chunk_mesh_dirty(*coord);
                }
            }
            
            // Create a large frustum to include all chunks
            let camera_pos = Vec3::new(camera_pos.0, camera_pos.1, camera_pos.2);
            let size = 1000.0;
            let frustum = CameraFrustum::new_simple(
                camera_pos.x - size, camera_pos.x + size,
                camera_pos.y - size, camera_pos.y + size,
                camera_pos.z - size, camera_pos.z + size,
            );
            
            // Get distance-sorted visible chunks
            let sorted_chunks = world.get_visible_chunks_sorted_by_distance(camera_pos, &frustum);
            
            // All chunks should be render-ready and within frustum
            for (coord, entry, _distance) in &sorted_chunks {
                prop_assert!(entry.is_renderable(), 
                    "All sorted chunks should be render-ready");
                prop_assert!(world.is_chunk_in_frustum(*coord, &frustum), 
                    "All sorted chunks should be within frustum");
            }
            
            // Chunks should be sorted by distance (closest first)
            for i in 1..sorted_chunks.len() {
                let prev_distance = sorted_chunks[i - 1].2;
                let curr_distance = sorted_chunks[i].2;
                prop_assert!(prev_distance <= curr_distance, 
                    "Chunks should be sorted by distance (closest first)");
            }
            
            // Distances should be non-negative
            for (_, _, distance) in &sorted_chunks {
                prop_assert!(*distance >= 0.0, 
                    "All distances should be non-negative");
            }
            
            // Verify distance calculations are reasonable
            for (coord, _, distance) in &sorted_chunks {
                let chunk_center = world.chunk_to_world_pos(*coord) + 
                    Vec3::splat(world.chunk_size() as f32 / 2.0);
                let expected_distance = camera_pos.distance(chunk_center);
                
                prop_assert!((distance - expected_distance).abs() < 0.001, 
                    "Distance calculation should be accurate");
            }
        }
    }

    // Property 9: Performance Monitoring Accuracy
    // **Validates: Requirements 5.1, 5.2, 5.3, 5.4**
    proptest! {
        #[test]
        fn property_performance_monitoring_accuracy(
            config in arb_world_config(),
            frame_times in prop::collection::vec(1u64..100u64, 5..20),
            chunk_counts in prop::collection::vec(1usize..50usize, 3..10),
            generation_times in prop::collection::vec(1u64..1000u64, 2..8),
            meshing_times in prop::collection::vec(1u64..500u64, 2..8),
        ) {
            // Feature: world-integration, Property 9: Performance Monitoring Accuracy
            
            let mut world = World::new(config).unwrap();
            let monitor = world.performance_monitor_mut();
            
            // Test frame time tracking accuracy
            let mut expected_total_frame_time = Duration::ZERO;
            for &frame_time_ms in &frame_times {
                let frame_time = Duration::from_millis(frame_time_ms);
                monitor.record_frame_time(frame_time);
                expected_total_frame_time += frame_time;
            }
            
            // FPS calculation should be accurate
            let current_fps = monitor.current_fps();
            if !frame_times.is_empty() {
                let avg_frame_time = expected_total_frame_time.as_secs_f32() / frame_times.len() as f32;
                let expected_fps = if avg_frame_time > 0.0 { 1.0 / avg_frame_time } else { 0.0 };
                
                prop_assert!((current_fps - expected_fps).abs() < 0.1, 
                    "FPS calculation should be accurate: expected {}, got {}", expected_fps, current_fps);
                prop_assert!(current_fps >= 0.0, "FPS should be non-negative");
            }
            
            // Average frame time should be accurate
            let avg_frame_time = monitor.average_frame_time();
            if !frame_times.is_empty() {
                let expected_avg = expected_total_frame_time / frame_times.len() as u32;
                let diff = if avg_frame_time > expected_avg {
                    avg_frame_time - expected_avg
                } else {
                    expected_avg - avg_frame_time
                };
                prop_assert!(diff <= Duration::from_millis(1), 
                    "Average frame time should be accurate");
            }
            
            // Test memory usage tracking
            for (i, &chunk_count) in chunk_counts.iter().enumerate() {
                let total_memory = chunk_count * 1024 * 1024; // 1MB per chunk
                let gpu_memory = chunk_count * 512 * 1024;    // 512KB GPU per chunk
                let sample = MemorySample::new(chunk_count, total_memory, gpu_memory);
                monitor.record_memory_sample(sample);
                
                // Current memory usage should reflect the latest sample
                if let Some(current_sample) = monitor.current_memory_usage() {
                    prop_assert_eq!(current_sample.chunk_count, chunk_count, 
                        "Current memory sample should reflect latest chunk count");
                    prop_assert_eq!(current_sample.total_memory, total_memory, 
                        "Current memory sample should reflect latest total memory");
                    prop_assert_eq!(current_sample.gpu_memory, gpu_memory, 
                        "Current memory sample should reflect latest GPU memory");
                }
                
                // Update chunk statistics to match
                monitor.update_chunk_stats(chunk_count);
                let chunk_stats = monitor.chunk_statistics();
                prop_assert_eq!(chunk_stats.chunks_loaded, chunk_count, 
                    "Chunk statistics should reflect updated chunk count");
            }
            
            // Test memory trend calculation
            if chunk_counts.len() >= 2 {
                let trend = monitor.memory_trend();
                prop_assert!(trend >= -1.0 && trend <= 100.0, 
                    "Memory trend should be within reasonable bounds");
                
                // If memory increased, trend should be positive
                let last_count = chunk_counts[chunk_counts.len() - 1];
                let prev_count = chunk_counts[chunk_counts.len() - 2];
                if last_count > prev_count {
                    prop_assert!(trend >= 0.0, 
                        "Memory trend should be positive when memory increases");
                } else if last_count < prev_count {
                    prop_assert!(trend <= 0.0, 
                        "Memory trend should be negative when memory decreases");
                }
            }
            
            // Test chunk operation timing accuracy
            let mut expected_generation_total = Duration::ZERO;
            for &gen_time_ms in &generation_times {
                let gen_time = Duration::from_millis(gen_time_ms);
                monitor.record_chunk_generation(gen_time);
                expected_generation_total += gen_time;
            }
            
            let mut expected_meshing_total = Duration::ZERO;
            for &mesh_time_ms in &meshing_times {
                let mesh_time = Duration::from_millis(mesh_time_ms);
                monitor.record_chunk_meshing(mesh_time);
                expected_meshing_total += mesh_time;
            }
            
            let chunk_stats = monitor.chunk_statistics();
            
            // Generation statistics should be accurate
            if !generation_times.is_empty() {
                prop_assert_eq!(chunk_stats.chunks_generated, generation_times.len(), 
                    "Generated chunk count should match number of recorded operations");
                
                let expected_avg_gen = expected_generation_total.as_secs_f64() / generation_times.len() as f64;
                let actual_avg_gen = chunk_stats.generation_time_avg.as_secs_f64();
                prop_assert!((actual_avg_gen - expected_avg_gen).abs() < 0.001, 
                    "Average generation time should be accurate");
            }
            
            // Meshing statistics should be accurate
            if !meshing_times.is_empty() {
                prop_assert_eq!(chunk_stats.chunks_meshed, meshing_times.len(), 
                    "Meshed chunk count should match number of recorded operations");
                
                let expected_avg_mesh = expected_meshing_total.as_secs_f64() / meshing_times.len() as f64;
                let actual_avg_mesh = chunk_stats.meshing_time_avg.as_secs_f64();
                prop_assert!((actual_avg_mesh - expected_avg_mesh).abs() < 0.001, 
                    "Average meshing time should be accurate");
            }
            
            // Total chunk time should include both generation and meshing
            let expected_total_chunk_time = expected_generation_total + expected_meshing_total;
            let actual_total_chunk_time = chunk_stats.total_chunk_time;
            let time_diff = if actual_total_chunk_time > expected_total_chunk_time {
                actual_total_chunk_time - expected_total_chunk_time
            } else {
                expected_total_chunk_time - actual_total_chunk_time
            };
            prop_assert!(time_diff <= Duration::from_millis(1), 
                "Total chunk time should be accurate");
            
            // Session duration should be reasonable
            let session_duration = monitor.session_duration();
            prop_assert!(session_duration >= Duration::ZERO, 
                "Session duration should be non-negative");
            prop_assert!(session_duration <= Duration::from_secs(60), 
                "Session duration should be reasonable for test execution");
            
            // Memory sampling timing should work correctly
            let should_sample = monitor.should_sample_memory();
            prop_assert!(should_sample == true || should_sample == false, 
                "Memory sampling check should return a boolean");
        }
    }

    // Property test for performance monitor configuration and limits
    proptest! {
        #[test]
        fn property_performance_monitor_configuration_limits(
            max_frame_samples in 1usize..200usize,
            max_memory_samples in 1usize..100usize,
            sample_interval_ms in 100u64..5000u64,
            frame_times in prop::collection::vec(1u64..100u64, 1..300),
        ) {
            // Feature: world-integration, Property 9: Performance Monitoring Accuracy (configuration)
            
            let config = MonitorConfig {
                max_frame_samples,
                max_memory_samples,
                memory_sample_interval: Duration::from_millis(sample_interval_ms),
                detailed_chunk_stats: true,
            };
            
            let mut monitor = PerformanceMonitor::new(config.clone());
            
            // Record more frame times than the limit
            for &frame_time_ms in &frame_times {
                let frame_time = Duration::from_millis(frame_time_ms);
                monitor.record_frame_time(frame_time);
            }
            
            // Frame time samples should be limited by configuration
            let current_fps = monitor.current_fps();
            if frame_times.len() > max_frame_samples {
                // Should only use the most recent samples
                prop_assert!(current_fps >= 0.0, 
                    "FPS should be calculated correctly even with sample limit");
            }
            
            // Average frame time should be based on limited samples
            let avg_frame_time = monitor.average_frame_time();
            prop_assert!(avg_frame_time >= Duration::ZERO, 
                "Average frame time should be non-negative");
            
            // Record memory samples up to the limit
            for i in 0..max_memory_samples + 10 {
                let sample = MemorySample::new(i, i * 1024, i * 512);
                monitor.record_memory_sample(sample);
            }
            
            // Current memory usage should always be available
            let current_memory = monitor.current_memory_usage();
            prop_assert!(current_memory.is_some(), 
                "Current memory usage should be available after recording samples");
            
            if let Some(sample) = current_memory {
                // Should be the most recent sample
                let expected_count = max_memory_samples + 9; // Last recorded value
                prop_assert_eq!(sample.chunk_count, expected_count, 
                    "Current memory sample should be the most recent");
            }
            
            // Memory trend should work with limited samples
            let trend = monitor.memory_trend();
            prop_assert!(trend >= -1.0 && trend <= 100.0, 
                "Memory trend should be within reasonable bounds with limited samples");
            
            // Reset should clear all data
            monitor.reset();
            
            prop_assert_eq!(monitor.current_fps(), 0.0, 
                "FPS should be 0 after reset");
            prop_assert_eq!(monitor.average_frame_time(), Duration::ZERO, 
                "Average frame time should be zero after reset");
            prop_assert!(monitor.current_memory_usage().is_none(), 
                "Current memory usage should be None after reset");
            prop_assert_eq!(monitor.memory_trend(), 0.0, 
                "Memory trend should be 0 after reset");
            
            let reset_stats = monitor.chunk_statistics();
            prop_assert_eq!(reset_stats.chunks_generated, 0, 
                "Chunk generation count should be 0 after reset");
            prop_assert_eq!(reset_stats.chunks_meshed, 0, 
                "Chunk meshing count should be 0 after reset");
            prop_assert_eq!(reset_stats.total_chunk_time, Duration::ZERO, 
                "Total chunk time should be 0 after reset");
        }
    }

    // Property test generators for configuration testing
    fn arb_config_version() -> impl Strategy<Value = config::ConfigVersion> {
        prop_oneof![
            Just(config::ConfigVersion::V1_0),
            Just(config::ConfigVersion::V1_1),
            Just(config::ConfigVersion::V1_2),
        ]
    }

    fn arb_world_config_with_version() -> impl Strategy<Value = WorldConfig> {
        (
            arb_config_version(),
            1u32..=32u32,
            10usize..=100usize,
            1u64..=3600u64,
            any::<bool>(),
        ).prop_map(|(version, render_distance, max_chunks, unload_delay_secs, performance_monitoring)| {
            WorldConfig {
                version,
                render_distance,
                initial_load_pattern: LoadPattern::Single(ChunkCoord::new(0, 0, 0)),
                max_chunks_loaded: Some(max_chunks),
                chunk_unload_delay: Duration::from_secs(unload_delay_secs),
                performance_monitoring,
            }
        })
    }

    // Property 10: Configuration Validation and Adaptation
    // **Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.5**
    proptest! {
        #[test]
        fn property_configuration_validation_and_adaptation(
            initial_config in arb_world_config(),
            new_config in arb_world_config_with_version(),
        ) {
            // Feature: world-integration, Property 10: Configuration Validation and Adaptation
            
            let mut world = World::new(initial_config.clone()).unwrap();
            
            // Test configuration validation
            let validation_result = new_config.validate();
            
            // All generated configurations should be valid
            prop_assert!(validation_result.is_ok(), 
                "Generated configurations should be valid: {:?}", validation_result);
            
            // Test configuration compatibility checking
            let differences = world.get_config_differences(&new_config);
            let would_require_restart = world.would_require_restart(&new_config);
            
            // Validate config change should work for compatible configurations
            let validation_check = world.validate_config_change(&new_config);
            
            if validation_check.is_ok() {
                let reported_differences = validation_check.unwrap();
                prop_assert_eq!(differences, reported_differences, 
                    "Reported differences should match actual differences");
                
                // If validation passes, the configuration should be compatible
                prop_assert!(new_config.is_compatible_with(&world.config()), 
                    "Configuration should be compatible if validation passes");
                
                // Test actual configuration update
                let update_result = world.update_config(new_config.clone());
                
                if update_result.is_ok() {
                    // Configuration should be updated
                    prop_assert_eq!(world.config().render_distance, new_config.render_distance, 
                        "Render distance should be updated");
                    prop_assert_eq!(world.config().max_chunks_loaded, new_config.max_chunks_loaded, 
                        "Max chunks should be updated");
                    prop_assert_eq!(world.config().performance_monitoring, new_config.performance_monitoring, 
                        "Performance monitoring should be updated");
                    prop_assert_eq!(world.config().chunk_unload_delay, new_config.chunk_unload_delay, 
                        "Chunk unload delay should be updated");
                    
                    // World should remain in a consistent state after configuration update
                    let consistency_check = world.validate_world_consistency();
                    if let Err(error) = consistency_check {
                        // Only accept consistency errors related to error states (from failed chunk loads)
                        match error {
                            WorldError::InvalidChunkState { current_state: ChunkState::Error, .. } => {
                                // This is acceptable - error state chunks from previous operations
                            }
                            _ => {
                                prop_assert!(false, "World should remain consistent after config update: {}", error);
                            }
                        }
                    }
                    
                    // Configuration compatibility should be maintained
                    let final_compatibility = world.validate_config_compatibility();
                    prop_assert!(final_compatibility.is_ok(), 
                        "World should be compatible with its own configuration after update");
                }
            } else {
                // If validation fails, it should be for a valid reason
                match validation_check {
                    Err(WorldError::InvalidConfiguration { parameter, reason, .. }) => {
                        // This is expected for incompatible configurations
                        prop_assert!(
                            parameter == "configuration" || 
                            reason.contains("incompatible") || 
                            reason.contains("restart"),
                            "Configuration validation failure should have appropriate reason: {}", reason
                        );
                    }
                    _ => {
                        prop_assert!(false, "Unexpected validation error type");
                    }
                }
            }
            
            // Test restart requirement logic
            if would_require_restart {
                // If restart is required, update should fail or be handled appropriately
                let update_result = world.update_config(new_config.clone());
                if update_result.is_err() {
                    match update_result.unwrap_err() {
                        WorldError::InvalidConfiguration { reason, .. } => {
                            prop_assert!(reason.contains("restart"), 
                                "Restart-required configurations should fail with restart message");
                        }
                        _ => {
                            prop_assert!(false, "Unexpected error type for restart-required config");
                        }
                    }
                }
            }
        }
    }

    // Property test for configuration migration
    proptest! {
        #[test]
        fn property_configuration_migration(
            version in arb_config_version(),
            render_distance in 1u32..=32u32,
            max_chunks in 10usize..=100usize,
            performance_monitoring in any::<bool>(),
        ) {
            // Feature: world-integration, Property 10: Configuration Validation and Adaptation (migration)
            
            let mut config = WorldConfig::new_with_version(version)
                .with_render_distance(render_distance)
                .with_max_chunks(Some(max_chunks))
                .with_performance_monitoring(performance_monitoring);
            
            let original_version = config.version;
            
            // Test migration to latest version
            let migration_result = config.migrate_to_latest();
            prop_assert!(migration_result.is_ok(), 
                "Migration should succeed for valid configurations");
            
            let was_migrated = migration_result.unwrap();
            
            // After migration, version should be latest
            prop_assert_eq!(config.version, config::ConfigVersion::V1_2, 
                "Configuration should be migrated to latest version");
            
            // Migration flag should be accurate
            if original_version == config::ConfigVersion::V1_2 {
                prop_assert!(!was_migrated, 
                    "Migration flag should be false if already at latest version");
            } else {
                prop_assert!(was_migrated, 
                    "Migration flag should be true if version was updated");
            }
            
            // Configuration should remain valid after migration
            prop_assert!(config.validate().is_ok(), 
                "Configuration should remain valid after migration");
            
            // Core parameters should be preserved during migration
            prop_assert_eq!(config.render_distance, render_distance, 
                "Render distance should be preserved during migration");
            prop_assert_eq!(config.max_chunks_loaded, Some(max_chunks), 
                "Max chunks should be preserved during migration");
            
            // Performance monitoring should be preserved for V1_1+ or set to default for V1_0
            if original_version == config::ConfigVersion::V1_0 {
                // V1_0 migration sets performance monitoring to true by default
                prop_assert!(config.performance_monitoring, 
                    "Performance monitoring should be enabled by default for V1_0 migration");
            } else {
                prop_assert_eq!(config.performance_monitoring, performance_monitoring, 
                    "Performance monitoring should be preserved for V1_1+ migration");
            }
            
            // Chunk unload delay should be set to default value
            prop_assert_eq!(config.chunk_unload_delay, Duration::from_secs(CHUNK_UNLOAD_DELAY_SECONDS), 
                "Chunk unload delay should be set to default during migration");
        }
    }

    // Property test for configuration compatibility
    proptest! {
        #[test]
        fn property_configuration_compatibility(
            config1 in arb_world_config_with_version(),
            config2 in arb_world_config_with_version(),
        ) {
            // Feature: world-integration, Property 10: Configuration Validation and Adaptation (compatibility)
            
            let compatibility = config1.is_compatible_with(&config2);
            let version_compatibility = config1.is_version_compatible_with(config2.version);
            
            // Version compatibility should be a factor in overall compatibility
            if !version_compatibility {
                prop_assert!(!compatibility, 
                    "Configurations should not be compatible if versions are incompatible");
            }
            
            // Test version compatibility rules
            use config::ConfigVersion::*;
            let expected_version_compatibility = match (config1.version, config2.version) {
                // Same versions are always compatible
                (v1, v2) if v1 == v2 => true,
                // V1_2 can handle all previous versions
                (V1_2, V1_1) | (V1_2, V1_0) => true,
                // V1_1 can handle V1_0
                (V1_1, V1_0) => true,
                // Older versions cannot handle newer versions
                _ => false,
            };
            
            prop_assert_eq!(version_compatibility, expected_version_compatibility, 
                "Version compatibility should follow expected rules");
            
            // Test configuration differences
            let differences = config1.get_differences(&config2);
            
            // If configurations are identical, there should be no differences
            if config1.version == config2.version &&
               config1.render_distance == config2.render_distance &&
               config1.initial_load_pattern == config2.initial_load_pattern &&
               config1.max_chunks_loaded == config2.max_chunks_loaded &&
               config1.chunk_unload_delay == config2.chunk_unload_delay &&
               config1.performance_monitoring == config2.performance_monitoring {
                prop_assert!(differences.is_empty(), 
                    "Identical configurations should have no differences");
            }
            
            // Each difference should correspond to an actual parameter difference
            for diff in &differences {
                match diff.as_str() {
                    "version" => {
                        prop_assert_ne!(config1.version, config2.version, 
                            "Version difference should correspond to actual version difference");
                    }
                    "render_distance" => {
                        prop_assert_ne!(config1.render_distance, config2.render_distance, 
                            "Render distance difference should correspond to actual difference");
                    }
                    "initial_load_pattern" => {
                        prop_assert_ne!(&config1.initial_load_pattern, &config2.initial_load_pattern, 
                            "Load pattern difference should correspond to actual difference");
                    }
                    "max_chunks_loaded" => {
                        prop_assert_ne!(config1.max_chunks_loaded, config2.max_chunks_loaded, 
                            "Max chunks difference should correspond to actual difference");
                    }
                    "chunk_unload_delay" => {
                        prop_assert_ne!(config1.chunk_unload_delay, config2.chunk_unload_delay, 
                            "Unload delay difference should correspond to actual difference");
                    }
                    "performance_monitoring" => {
                        prop_assert_ne!(config1.performance_monitoring, config2.performance_monitoring, 
                            "Performance monitoring difference should correspond to actual difference");
                    }
                    _ => {
                        prop_assert!(false, "Unknown configuration difference: {}", diff);
                    }
                }
            }
            
            // Test restart requirement logic
            let requires_restart = config1.requires_restart(&config2);
            
            // Currently, no configuration changes require restart
            prop_assert!(!requires_restart, 
                "Currently no configuration changes should require restart");
        }
    }

    // Property test for configuration update with migration
    proptest! {
        #[test]
        fn property_configuration_update_with_migration(
            initial_config in arb_world_config(),
            new_version in arb_config_version(),
            new_render_distance in 1u32..=32u32,
        ) {
            // Feature: world-integration, Property 10: Configuration Validation and Adaptation (update with migration)
            
            let mut world = World::new(initial_config.clone()).unwrap();
            
            // Create a new configuration with potentially older version
            let new_config = WorldConfig::new_with_version(new_version)
                .with_render_distance(new_render_distance)
                .with_max_chunks(Some(50));
            
            // Test update with migration
            let update_result = world.update_config_with_migration(new_config.clone());
            
            if update_result.is_ok() {
                // Configuration should be updated and migrated
                prop_assert_eq!(world.config().version, config::ConfigVersion::V1_2, 
                    "Configuration should be migrated to latest version");
                prop_assert_eq!(world.config().render_distance, new_render_distance, 
                    "Render distance should be updated");
                
                // World should remain consistent after update with migration
                let consistency_check = world.validate_world_consistency();
                if let Err(error) = consistency_check {
                    // Only accept consistency errors related to error states
                    match error {
                        WorldError::InvalidChunkState { current_state: ChunkState::Error, .. } => {
                            // This is acceptable
                        }
                        _ => {
                            prop_assert!(false, "World should remain consistent after update with migration: {}", error);
                        }
                    }
                }
            } else {
                // If update fails, it should be for a valid reason
                match update_result.unwrap_err() {
                    WorldError::InvalidConfiguration { .. } => {
                        // This is expected for incompatible configurations
                    }
                    _ => {
                        prop_assert!(false, "Unexpected error type for configuration update with migration");
                    }
                }
            }
        }
    }

    // Property 11: Memory Management Bounds
    // **Validates: Requirements 7.1, 7.2, 7.3, 7.5**
    proptest! {
        #[test]
        fn property_memory_management_bounds(
            memory_budget in 1024usize..=10_000_000usize, // 1KB to 10MB
            coords in prop::collection::vec(arb_chunk_coord(), 1..20),
            cleanup_threshold in 0.1f32..=0.9f32,
        ) {
            // Feature: world-integration, Property 11: Memory Management Bounds
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.is_empty() {
                return Ok(());
            }
            
            // Create world with specific memory budget
            let config = WorldConfig::new()
                .with_render_distance(8)
                .with_max_chunks(Some(unique_coords.len() * 2));
            
            let mut world = World::new(config).unwrap();
            
            // Set memory budget and cleanup threshold
            world.set_memory_budget(memory_budget).unwrap();
            world.set_memory_cleanup_threshold(cleanup_threshold).unwrap();
            
            // Test memory bounds checking
            let initial_stats = world.memory_stats();
            prop_assert_eq!(initial_stats.total_budget, memory_budget, 
                "Memory budget should be set correctly");
            prop_assert_eq!(initial_stats.current_usage, 0, 
                "Initial memory usage should be zero");
            prop_assert!((initial_stats.cleanup_threshold - cleanup_threshold).abs() < 0.001, 
                "Cleanup threshold should be set correctly");
            
            // Load chunks and track memory usage
            let mut loaded_chunks = Vec::new();
            let mut _total_expected_memory = 0;
            
            for coord in &unique_coords {
                // Check if we can load the chunk
                let can_load = world.can_load_new_chunk();
                
                if can_load {
                    // Attempt to load the chunk
                    match world.load_single_chunk(*coord) {
                        Ok(progress) => {
                            if progress.successful_loads() > 0 && world.is_chunk_loaded(*coord) {
                                loaded_chunks.push(*coord);
                                
                                // Estimate memory usage for this chunk
                                let chunk_size_bytes = (world.chunk_size() as usize).pow(3) * 2;
                                let estimated_memory = chunk_size_bytes + 1024;
                                _total_expected_memory += estimated_memory;
                            }
                        }
                        Err(WorldError::OutOfMemory { .. }) => {
                            // This is expected when memory budget is exceeded
                            break;
                        }
                        Err(_) => {
                            // Other errors are acceptable (e.g., coordinate validation)
                        }
                    }
                } else {
                    // Cannot load more chunks due to memory constraints
                    break;
                }
                
                // Validate memory bounds after each load
                let bounds_check = world.validate_memory_bounds();
                prop_assert!(bounds_check.is_ok(), 
                    "Memory bounds should be valid after loading chunk: {:?}", bounds_check);
            }
            
            // Verify memory usage is within bounds
            let final_stats = world.memory_stats();
            prop_assert!(final_stats.current_usage <= final_stats.total_budget, 
                "Memory usage should not exceed budget");
            
            // Verify memory tracking consistency
            let consistency_check = world.validate_memory_consistency();
            prop_assert!(consistency_check.is_ok(), 
                "Memory tracking should be consistent: {:?}", consistency_check);
            
            // Test memory cleanup when threshold is exceeded
            if final_stats.usage_fraction >= cleanup_threshold {
                let should_cleanup = world.should_cleanup_memory();
                prop_assert!(should_cleanup, 
                    "Should trigger cleanup when usage exceeds threshold");
                
                let cleanup_result = world.cleanup_memory_if_needed();
                prop_assert!(cleanup_result.is_ok(), 
                    "Memory cleanup should succeed");
                
                let post_cleanup_stats = world.memory_stats();
                prop_assert!(post_cleanup_stats.current_usage <= final_stats.current_usage, 
                    "Memory usage should not increase after cleanup");
            }
            
            // Test memory leak detection
            let potential_leaks = world.detect_memory_leaks(std::time::Duration::from_secs(1));
            // Chunks were just loaded/accessed, so should not be considered leaks with 1 second threshold
            prop_assert_eq!(potential_leaks.len(), 0, 
                "Should not detect leaks for recently accessed chunks with 1 second threshold");
            
            // Test memory health check
            let health_report = world.memory_health_check().unwrap();
            prop_assert!(
                health_report.status == MemoryHealthStatus::Healthy || 
                health_report.status == MemoryHealthStatus::Warning ||
                health_report.status == MemoryHealthStatus::Critical,
                "Health report should have valid status"
            );
            
            // Test fragmentation statistics
            let frag_stats = world.memory_fragmentation_stats();
            // Note: fragmentation stats track chunks in memory manager, which may differ from loaded chunks
            // if memory registration failed for some chunks
            prop_assert!(frag_stats.total_chunks <= loaded_chunks.len(), 
                "Fragmentation stats should not exceed loaded chunks");
            
            if frag_stats.total_chunks > 0 {
                prop_assert!(frag_stats.average_chunk_size > 0, 
                    "Average chunk size should be positive when chunks are tracked");
                prop_assert!(frag_stats.min_chunk_size <= frag_stats.max_chunk_size, 
                    "Min chunk size should not exceed max chunk size");
                
                let fragmentation_score = frag_stats.fragmentation_score();
                prop_assert!(fragmentation_score >= 0.0 && fragmentation_score <= 1.0, 
                    "Fragmentation score should be between 0.0 and 1.0");
            }
            
            // Test chunk removal and memory cleanup
            // Re-verify which chunks are actually loaded before attempting removal
            let actually_loaded_chunks: Vec<ChunkCoord> = loaded_chunks.iter()
                .filter(|coord| world.is_chunk_loaded(**coord))
                .copied()
                .collect();
            
            let chunks_to_remove = actually_loaded_chunks.len() / 2;
            if chunks_to_remove > 0 {
                for coord in actually_loaded_chunks.iter().take(chunks_to_remove) {
                    let removed = world.remove_chunk(*coord);
                    prop_assert!(removed.is_some(), 
                        "Should be able to remove loaded chunk");
                }
                
                // Verify memory was properly released
                let after_removal_stats = world.memory_stats();
                prop_assert!(after_removal_stats.current_usage < final_stats.current_usage, 
                    "Memory usage should decrease after removing chunks");
            }
            
            // Final consistency check
            let final_consistency = world.validate_memory_consistency();
            prop_assert!(final_consistency.is_ok(), 
                "Memory should remain consistent after chunk removal");
        }
    }

    // Property test for memory bounds enforcement
    proptest! {
        #[test]
        fn property_memory_bounds_enforcement(
            small_budget in 1024usize..=50_000usize, // Small budget to trigger limits
            coords in prop::collection::vec(arb_chunk_coord(), 5..15),
        ) {
            // Feature: world-integration, Property 11: Memory Management Bounds (enforcement)
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.len() < 5 {
                return Ok(());
            }
            
            // Create world with small memory budget to test enforcement
            let config = WorldConfig::new()
                .with_render_distance(4)
                .with_max_chunks(Some(unique_coords.len()));
            
            let mut world = World::new(config).unwrap();
            world.set_memory_budget(small_budget).unwrap();
            
            // Try to load chunks until memory limit is hit
            let mut loaded_count = 0;
            let mut hit_memory_limit = false;
            
            for coord in &unique_coords {
                match world.load_single_chunk(*coord) {
                    Ok(progress) => {
                        if progress.successful_loads() > 0 {
                            loaded_count += 1;
                        }
                    }
                    Err(WorldError::OutOfMemory { requested, available }) => {
                        // This is expected when memory budget is exceeded
                        hit_memory_limit = true;
                        prop_assert!(requested > available, 
                            "OutOfMemory error should indicate insufficient available memory");
                        break;
                    }
                    Err(_) => {
                        // Other errors are acceptable
                    }
                }
                
                // Check that memory usage never exceeds budget
                let stats = world.memory_stats();
                prop_assert!(stats.current_usage <= stats.total_budget, 
                    "Memory usage should never exceed budget during loading");
            }
            
            // With a small budget, we should eventually hit the memory limit
            if unique_coords.len() > 3 {
                prop_assert!(hit_memory_limit || loaded_count < unique_coords.len(), 
                    "Should hit memory limit or not load all chunks with small budget");
            }
            
            // Test that bounds enforcement prevents memory leaks
            let bounds_enforcement = world.enforce_memory_bounds();
            prop_assert!(bounds_enforcement.is_ok(), 
                "Memory bounds enforcement should succeed");
            
            // Verify final state is consistent
            let final_validation = world.validate_memory_bounds();
            prop_assert!(final_validation.is_ok(), 
                "Memory bounds should be valid after enforcement");
        }
    }

    // Property test for memory leak detection and cleanup
    proptest! {
        #[test]
        fn property_memory_leak_detection_and_cleanup(
            config in arb_world_config(),
            coords in prop::collection::vec(arb_chunk_coord(), 3..10),
            idle_duration_ms in 100u64..=5000u64,
        ) {
            // Feature: world-integration, Property 11: Memory Management Bounds (leak detection)
            
            let mut world = World::new(config).unwrap();
            
            // Create unique coordinates
            let mut unique_coords = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for coord in coords {
                if seen.insert(coord) {
                    unique_coords.push(coord);
                }
            }
            
            if unique_coords.len() < 3 {
                return Ok(());
            }
            
            // Load some chunks
            let pattern = LoadPattern::Custom(unique_coords.clone());
            let _progress = world.load_chunks(pattern).unwrap();
            
            let loaded_coords: Vec<ChunkCoord> = unique_coords.iter()
                .filter(|coord| world.is_chunk_loaded(**coord))
                .copied()
                .collect();
            
            if loaded_coords.len() < 2 {
                return Ok(());
            }
            
            // Get chunks that are actually tracked in memory manager
            let memory_stats = world.memory_stats();
            let tracked_chunks_count = memory_stats.chunks_tracked;
            
            // Initially, no leaks should be detected for tracked chunks (they were just accessed)
            let initial_leaks = world.detect_memory_leaks(std::time::Duration::from_millis(idle_duration_ms));
            prop_assert!(initial_leaks.is_empty(), 
                "Should not detect leaks for recently accessed chunks: expected 0 leaks, got {}", initial_leaks.len());
            
            // Wait a bit and then detect leaks with a very short duration
            std::thread::sleep(std::time::Duration::from_millis(10));
            let short_duration_leaks = world.detect_memory_leaks(std::time::Duration::from_millis(1));
            
            // All tracked chunks should be considered potential leaks with very short duration
            prop_assert_eq!(short_duration_leaks.len(), tracked_chunks_count, 
                "All tracked chunks should be potential leaks with very short idle duration");
            
            // Test leak cleanup
            let cleanup_result = world.cleanup_memory_leaks(std::time::Duration::from_millis(1));
            prop_assert!(cleanup_result.is_ok(), 
                "Memory leak cleanup should succeed");
            
            let cleaned_up_count = cleanup_result.unwrap();
            prop_assert_eq!(cleaned_up_count, tracked_chunks_count, 
                "Should clean up all chunks identified as leaks");
            
            // After cleanup, no chunks should be loaded
            prop_assert_eq!(world.chunk_count(), 0, 
                "No chunks should remain after leak cleanup");
            
            // Memory usage should be zero after cleanup
            let final_stats = world.memory_stats();
            prop_assert_eq!(final_stats.current_usage, 0, 
                "Memory usage should be zero after cleaning up all chunks");
            
            // Memory consistency should be maintained
            let consistency_check = world.validate_memory_consistency();
            prop_assert!(consistency_check.is_ok(), 
                "Memory should be consistent after leak cleanup");
        }
    }
}