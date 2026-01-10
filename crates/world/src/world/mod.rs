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
use glam::Vec3;

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
        (1u32..=32u32, 1usize..=100usize).prop_map(|(render_distance, max_chunks)| {
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
}