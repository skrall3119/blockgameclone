//! Chunk data structures and management
//!
//! This module provides the core data structures for representing voxel world data
//! in fixed-size 3D regions. The design prioritizes cache-friendly flat array storage,
//! type-safe block identification, and efficient coordinate conversion utilities.

/// Chunk dimension constants - compile-time configurable
pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_HEIGHT: usize = 256;
pub const CHUNK_DEPTH: usize = 16;

/// Default chunk dimensions
pub const DEFAULT_DIMENSIONS: ChunkDimensions = ChunkDimensions {
    width: CHUNK_WIDTH,
    height: CHUNK_HEIGHT,
    depth: CHUNK_DEPTH,
};

/// Chunk dimensions structure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkDimensions {
    pub width: usize,   // x-axis
    pub height: usize,  // y-axis
    pub depth: usize,   // z-axis
}

/// Chunk position in world coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPosition {
    pub x: i32,
    pub z: i32,
}

/// Type-safe block identifier enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u16)]
pub enum BlockID {
    Air = 0,
    Stone = 1,
    Dirt = 2,
    Grass = 3,
}

impl BlockID {
    /// Convert from u16 representation (safe conversion)
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0 => Some(BlockID::Air),
            1 => Some(BlockID::Stone),
            2 => Some(BlockID::Dirt),
            3 => Some(BlockID::Grass),
            _ => None,
        }
    }

    /// Convert to u16 representation for serialization
    pub fn to_u16(self) -> u16 {
        self as u16
    }
}

/// Chunk error types
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkError {
    OutOfBounds,
    InvalidDimensions,
    InvalidBlockData,
}

impl std::fmt::Display for ChunkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkError::OutOfBounds => write!(f, "Coordinates are out of chunk bounds"),
            ChunkError::InvalidDimensions => write!(f, "Invalid chunk dimensions provided"),
            ChunkError::InvalidBlockData => write!(f, "Invalid block data for chunk initialization"),
        }
    }
}

impl std::error::Error for ChunkError {}

/// A chunk of blocks stored as a flat array for cache efficiency
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Block data stored as flat array using row-major ordering
    blocks: Vec<BlockID>,
    /// World position coordinates
    world_position: ChunkPosition,
    /// Chunk dimensions
    dimensions: ChunkDimensions,
}

impl Chunk {
    /// Create a new chunk with default Air initialization
    pub fn new(world_position: ChunkPosition, dimensions: ChunkDimensions) -> Self {
        let total_blocks = dimensions.width * dimensions.height * dimensions.depth;
        Self {
            blocks: vec![BlockID::Air; total_blocks],
            world_position,
            dimensions,
        }
    }

    /// Create chunk from existing block data
    pub fn from_data(
        data: Vec<BlockID>,
        world_position: ChunkPosition,
        dimensions: ChunkDimensions,
    ) -> Result<Self, ChunkError> {
        let expected_size = dimensions.width * dimensions.height * dimensions.depth;
        if data.len() != expected_size {
            return Err(ChunkError::InvalidBlockData);
        }

        Ok(Self {
            blocks: data,
            world_position,
            dimensions,
        })
    }

    /// Fill entire chunk with specified block type
    pub fn fill(&mut self, block: BlockID) {
        self.blocks.fill(block);
    }

    /// Get block at 3D coordinates
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> Result<BlockID, ChunkError> {
        let index = self.coords_to_index(x, y, z)?;
        Ok(self.blocks[index])
    }

    /// Set block at 3D coordinates
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: BlockID) -> Result<(), ChunkError> {
        let index = self.coords_to_index(x, y, z)?;
        self.blocks[index] = block;
        Ok(())
    }

    /// Get multiple blocks at once using coordinate-block pairs
    /// Returns a vector of results in the same order as input coordinates
    pub fn get_blocks(&self, coordinates: &[(usize, usize, usize)]) -> Vec<Result<BlockID, ChunkError>> {
        coordinates
            .iter()
            .map(|&(x, y, z)| self.get_block(x, y, z))
            .collect()
    }

    /// Set multiple blocks at once using coordinate-block pairs
    /// Returns the first error encountered, or Ok(()) if all operations succeed
    /// If an error occurs, some blocks may have been set before the error
    pub fn set_blocks(&mut self, blocks: &[((usize, usize, usize), BlockID)]) -> Result<(), ChunkError> {
        for &((x, y, z), block) in blocks {
            self.set_block(x, y, z, block)?;
        }
        Ok(())
    }

    /// Set multiple blocks at once, collecting all errors instead of stopping at first error
    /// Returns a vector of results in the same order as input blocks
    pub fn set_blocks_collect_errors(&mut self, blocks: &[((usize, usize, usize), BlockID)]) -> Vec<Result<(), ChunkError>> {
        blocks
            .iter()
            .map(|&((x, y, z), block)| self.set_block(x, y, z, block))
            .collect()
    }

    /// Get world position
    pub fn world_position(&self) -> ChunkPosition {
        self.world_position
    }

    /// Get dimensions
    pub fn dimensions(&self) -> ChunkDimensions {
        self.dimensions
    }

    /// Convert 3D coordinates to flat array index
    fn coords_to_index(&self, x: usize, y: usize, z: usize) -> Result<usize, ChunkError> {
        if x >= self.dimensions.width || y >= self.dimensions.height || z >= self.dimensions.depth {
            return Err(ChunkError::OutOfBounds);
        }

        // Row-major order: x + y * width + z * width * height
        Ok(x + y * self.dimensions.width + z * self.dimensions.width * self.dimensions.height)
    }

    /// Convert flat array index to 3D coordinates
    fn index_to_coords(&self, index: usize) -> Result<(usize, usize, usize), ChunkError> {
        let total_size = self.dimensions.width * self.dimensions.height * self.dimensions.depth;
        if index >= total_size {
            return Err(ChunkError::OutOfBounds);
        }

        let z = index / (self.dimensions.width * self.dimensions.height);
        let remainder = index % (self.dimensions.width * self.dimensions.height);
        let y = remainder / self.dimensions.width;
        let x = remainder % self.dimensions.width;

        Ok((x, y, z))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Property test generators
    prop_compose! {
        fn arb_chunk_dimensions()(
            width in 1usize..=64,
            height in 1usize..=512,
            depth in 1usize..=64
        ) -> ChunkDimensions {
            ChunkDimensions { width, height, depth }
        }
    }

    prop_compose! {
        fn arb_chunk_position()(
            x in -1000i32..=1000,
            z in -1000i32..=1000
        ) -> ChunkPosition {
            ChunkPosition { x, z }
        }
    }

    prop_compose! {
        fn arb_block_id()(
            id in 0u16..=3
        ) -> BlockID {
            BlockID::from_u16(id).unwrap()
        }
    }

    // Property 1: Coordinate Round-Trip Consistency
    proptest! {
        #[test]
        fn property_coordinate_round_trip_consistency(
            dimensions in arb_chunk_dimensions(),
            position in arb_chunk_position()
        ) {
            // Feature: chunk-data-model, Property 1: Coordinate Round-Trip Consistency
            // Validates: Requirements 4.1, 4.2
            let chunk = Chunk::new(position, dimensions);
            
            // For any valid 3D coordinates within chunk bounds, converting to flat array 
            // index and back to coordinates should produce the original coordinates
            for x in 0..dimensions.width {
                for y in 0..dimensions.height {
                    for z in 0..dimensions.depth {
                        // Convert coordinates to index
                        let index = chunk.coords_to_index(x, y, z).unwrap();
                        
                        // Convert index back to coordinates
                        let (recovered_x, recovered_y, recovered_z) = chunk.index_to_coords(index).unwrap();
                        
                        // Should recover the original coordinates
                        assert_eq!((recovered_x, recovered_y, recovered_z), (x, y, z));
                    }
                }
            }
        }
    }

    // Property 2: Chunk Dimensions Consistency
    proptest! {
        #[test]
        fn property_chunk_dimensions_consistency(
            dimensions in arb_chunk_dimensions(),
            position in arb_chunk_position()
        ) {
            // Feature: chunk-data-model, Property 2: Chunk Dimensions Consistency
            // Validates: Requirements 1.1, 1.4
            let chunk = Chunk::new(position, dimensions);
            
            // Chunk should maintain the exact dimensions it was created with
            assert_eq!(chunk.dimensions(), dimensions);
            
            // Chunk should store exactly width × height × depth blocks
            let expected_size = dimensions.width * dimensions.height * dimensions.depth;
            assert_eq!(chunk.blocks.len(), expected_size);
        }
    }

    // Property 5: Default Initialization
    proptest! {
        #[test]
        fn property_default_initialization(
            dimensions in arb_chunk_dimensions(),
            position in arb_chunk_position()
        ) {
            // Feature: chunk-data-model, Property 5: Default Initialization
            // Validates: Requirements 5.1
            let chunk = Chunk::new(position, dimensions);
            
            // All blocks should be initialized to Air
            for x in 0..dimensions.width {
                for y in 0..dimensions.height {
                    for z in 0..dimensions.depth {
                        let block = chunk.get_block(x, y, z).unwrap();
                        assert_eq!(block, BlockID::Air);
                    }
                }
            }
        }
    }

    // Property 6: World Position Persistence
    proptest! {
        #[test]
        fn property_world_position_persistence(
            dimensions in arb_chunk_dimensions(),
            position in arb_chunk_position()
        ) {
            // Feature: chunk-data-model, Property 6: World Position Persistence
            // Validates: Requirements 1.5, 5.2
            let chunk = Chunk::new(position, dimensions);
            
            // Chunk should maintain the world position it was created with
            assert_eq!(chunk.world_position(), position);
            
            // Position should persist after operations
            let mut chunk_mut = chunk;
            chunk_mut.fill(BlockID::Stone);
            assert_eq!(chunk_mut.world_position(), position);
        }
    }

    // Property 7: Bulk Fill Operations
    proptest! {
        #[test]
        fn property_bulk_fill_operations(
            dimensions in arb_chunk_dimensions(),
            position in arb_chunk_position(),
            fill_block in arb_block_id()
        ) {
            // Feature: chunk-data-model, Property 7: Bulk Fill Operations
            // Validates: Requirements 5.3
            let mut chunk = Chunk::new(position, dimensions);
            
            // Fill entire chunk with specified block type
            chunk.fill(fill_block);
            
            // All blocks should be set to the fill block type
            for x in 0..dimensions.width {
                for y in 0..dimensions.height {
                    for z in 0..dimensions.depth {
                        let block = chunk.get_block(x, y, z).unwrap();
                        assert_eq!(block, fill_block);
                    }
                }
            }
        }
    }

    // Property 8: Data Initialization Validation
    proptest! {
        #[test]
        fn property_data_initialization_validation(
            dimensions in arb_chunk_dimensions(),
            position in arb_chunk_position(),
            block_data in prop::collection::vec(arb_block_id(), 0..1000)
        ) {
            // Feature: chunk-data-model, Property 8: Data Initialization Validation
            // Validates: Requirements 5.4, 5.5
            let expected_size = dimensions.width * dimensions.height * dimensions.depth;
            
            let result = Chunk::from_data(block_data.clone(), position, dimensions);
            
            if block_data.len() == expected_size {
                // Should succeed when data length matches expected dimensions
                let chunk = result.unwrap();
                assert_eq!(chunk.dimensions(), dimensions);
                assert_eq!(chunk.world_position(), position);
                
                // Verify that the data was stored correctly
                for (i, &expected_block) in block_data.iter().enumerate() {
                    let (x, y, z) = chunk.index_to_coords(i).unwrap();
                    let actual_block = chunk.get_block(x, y, z).unwrap();
                    assert_eq!(actual_block, expected_block);
                }
            } else {
                // Should fail when data length doesn't match expected dimensions
                assert_eq!(result.unwrap_err(), ChunkError::InvalidBlockData);
            }
        }
    }

    // Property 3: Block Storage and Retrieval
    proptest! {
        #[test]
        fn property_block_storage_and_retrieval(
            dimensions in arb_chunk_dimensions(),
            position in arb_chunk_position(),
            x in 0usize..64,
            y in 0usize..512,
            z in 0usize..64,
            block in arb_block_id()
        ) {
            // Feature: chunk-data-model, Property 3: Block Storage and Retrieval
            // Validates: Requirements 2.1, 6.1, 6.2
            let mut chunk = Chunk::new(position, dimensions);
            
            // Only test coordinates that are within the chunk bounds
            if x < dimensions.width && y < dimensions.height && z < dimensions.depth {
                // Set a block at the coordinates
                let set_result = chunk.set_block(x, y, z, block);
                assert!(set_result.is_ok());
                
                // Get the block back - should return the same block type
                let get_result = chunk.get_block(x, y, z);
                assert_eq!(get_result, Ok(block));
            }
        }
    }

    // Property 10: Batch Operations Consistency
    proptest! {
        #[test]
        fn property_batch_operations_consistency(
            dimensions in arb_chunk_dimensions(),
            position in arb_chunk_position(),
            blocks in prop::collection::vec(
                (0usize..64, 0usize..512, 0usize..64, arb_block_id()),
                0..20
            )
        ) {
            // Feature: chunk-data-model, Property 10: Batch Operations Consistency
            // Validates: Requirements 6.5
            let mut chunk1 = Chunk::new(position, dimensions);
            let mut chunk2 = Chunk::new(position, dimensions);
            
            // Filter blocks to only include those within chunk bounds
            let valid_blocks: Vec<_> = blocks
                .into_iter()
                .filter(|(x, y, z, _)| {
                    *x < dimensions.width && *y < dimensions.height && *z < dimensions.depth
                })
                .collect();
            
            if !valid_blocks.is_empty() {
                // Set blocks individually in chunk1
                for (x, y, z, block) in &valid_blocks {
                    chunk1.set_block(*x, *y, *z, *block).unwrap();
                }
                
                // Set blocks using batch operation in chunk2
                let batch_blocks: Vec<_> = valid_blocks
                    .iter()
                    .map(|(x, y, z, block)| ((*x, *y, *z), *block))
                    .collect();
                
                let batch_result = chunk2.set_blocks(&batch_blocks);
                assert!(batch_result.is_ok());
                
                // Both chunks should have identical block data
                for (x, y, z, expected_block) in &valid_blocks {
                    let block1 = chunk1.get_block(*x, *y, *z).unwrap();
                    let block2 = chunk2.get_block(*x, *y, *z).unwrap();
                    assert_eq!(block1, *expected_block);
                    assert_eq!(block2, *expected_block);
                    assert_eq!(block1, block2);
                }
            }
        }
    }

    // Unit tests for batch operations
    #[test]
    fn test_batch_get_blocks() {
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, DEFAULT_DIMENSIONS);
        
        // Set some test blocks
        chunk.set_block(0, 0, 0, BlockID::Stone).unwrap();
        chunk.set_block(1, 1, 1, BlockID::Dirt).unwrap();
        chunk.set_block(2, 2, 2, BlockID::Grass).unwrap();
        
        // Test batch get
        let coordinates = vec![(0, 0, 0), (1, 1, 1), (2, 2, 2), (3, 3, 3)];
        let results = chunk.get_blocks(&coordinates);
        
        assert_eq!(results[0], Ok(BlockID::Stone));
        assert_eq!(results[1], Ok(BlockID::Dirt));
        assert_eq!(results[2], Ok(BlockID::Grass));
        assert_eq!(results[3], Ok(BlockID::Air)); // Default initialization
    }

    #[test]
    fn test_batch_get_blocks_with_invalid_coordinates() {
        let chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, DEFAULT_DIMENSIONS);
        
        // Test with some valid and some invalid coordinates
        let coordinates = vec![
            (0, 0, 0),                    // Valid
            (CHUNK_WIDTH, 0, 0),          // Invalid - x out of bounds
            (1, 1, 1),                    // Valid
            (0, CHUNK_HEIGHT, 0),         // Invalid - y out of bounds
        ];
        let results = chunk.get_blocks(&coordinates);
        
        assert_eq!(results[0], Ok(BlockID::Air));
        assert_eq!(results[1], Err(ChunkError::OutOfBounds));
        assert_eq!(results[2], Ok(BlockID::Air));
        assert_eq!(results[3], Err(ChunkError::OutOfBounds));
    }

    #[test]
    fn test_batch_set_blocks() {
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, DEFAULT_DIMENSIONS);
        
        // Test batch set
        let blocks = vec![
            ((0, 0, 0), BlockID::Stone),
            ((1, 1, 1), BlockID::Dirt),
            ((2, 2, 2), BlockID::Grass),
        ];
        
        let result = chunk.set_blocks(&blocks);
        assert!(result.is_ok());
        
        // Verify blocks were set correctly
        assert_eq!(chunk.get_block(0, 0, 0).unwrap(), BlockID::Stone);
        assert_eq!(chunk.get_block(1, 1, 1).unwrap(), BlockID::Dirt);
        assert_eq!(chunk.get_block(2, 2, 2).unwrap(), BlockID::Grass);
    }

    #[test]
    fn test_batch_set_blocks_with_invalid_coordinates() {
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, DEFAULT_DIMENSIONS);
        
        // Test with some valid and some invalid coordinates
        let blocks = vec![
            ((0, 0, 0), BlockID::Stone),          // Valid
            ((CHUNK_WIDTH, 0, 0), BlockID::Dirt), // Invalid - should stop here
            ((1, 1, 1), BlockID::Grass),          // Valid but won't be reached
        ];
        
        let result = chunk.set_blocks(&blocks);
        assert_eq!(result, Err(ChunkError::OutOfBounds));
        
        // First block should have been set, others should not
        assert_eq!(chunk.get_block(0, 0, 0).unwrap(), BlockID::Stone);
        assert_eq!(chunk.get_block(1, 1, 1).unwrap(), BlockID::Air); // Should remain Air
    }

    #[test]
    fn test_batch_set_blocks_collect_errors() {
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, DEFAULT_DIMENSIONS);
        
        // Test with mixed valid and invalid coordinates
        let blocks = vec![
            ((0, 0, 0), BlockID::Stone),          // Valid
            ((CHUNK_WIDTH, 0, 0), BlockID::Dirt), // Invalid
            ((1, 1, 1), BlockID::Grass),          // Valid
            ((0, CHUNK_HEIGHT, 0), BlockID::Air), // Invalid
        ];
        
        let results = chunk.set_blocks_collect_errors(&blocks);
        
        assert_eq!(results[0], Ok(()));
        assert_eq!(results[1], Err(ChunkError::OutOfBounds));
        assert_eq!(results[2], Ok(()));
        assert_eq!(results[3], Err(ChunkError::OutOfBounds));
        
        // Valid blocks should have been set
        assert_eq!(chunk.get_block(0, 0, 0).unwrap(), BlockID::Stone);
        assert_eq!(chunk.get_block(1, 1, 1).unwrap(), BlockID::Grass);
    }
}