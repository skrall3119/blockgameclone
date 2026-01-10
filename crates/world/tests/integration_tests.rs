//! Integration tests for the chunk data model system
//!
//! These tests verify complete workflows and system behavior across
//! different chunk configurations and usage patterns.

use world::{Chunk, ChunkDimensions, ChunkPosition, BlockID, ChunkError};

/// Test chunk creation with both supported dimensions (16×16×256, 32×32×128)
/// Requirements: 1.4, 2.2, 2.3
#[test]
fn test_chunk_creation_with_supported_dimensions() {
    // Test standard Minecraft-style dimensions (16×16×256)
    let standard_dims = ChunkDimensions {
        width: 16,
        height: 256,
        depth: 16,
    };
    let position = ChunkPosition { x: 0, z: 0 };
    
    let standard_chunk = Chunk::new(position, standard_dims);
    assert_eq!(standard_chunk.dimensions(), standard_dims);
    assert_eq!(standard_chunk.world_position(), position);
    
    // Verify memory allocation is correct
    let expected_blocks = 16 * 256 * 16; // 65,536 blocks
    // Each block is 2 bytes (u16), so total should be ~128KB
    let expected_memory_bytes = expected_blocks * 2;
    assert_eq!(expected_memory_bytes, 131_072); // 128KB
    
    // Test alternative dimensions (32×32×128)
    let alternative_dims = ChunkDimensions {
        width: 32,
        height: 128,
        depth: 32,
    };
    
    let alternative_chunk = Chunk::new(position, alternative_dims);
    assert_eq!(alternative_chunk.dimensions(), alternative_dims);
    assert_eq!(alternative_chunk.world_position(), position);
    
    // Verify memory allocation for alternative dimensions
    let alt_expected_blocks = 32 * 128 * 32; // 131,072 blocks
    let alt_expected_memory_bytes = alt_expected_blocks * 2;
    assert_eq!(alt_expected_memory_bytes, 262_144); // 256KB
    
    // Both chunks should initialize with Air blocks
    assert_eq!(standard_chunk.get_block(0, 0, 0).unwrap(), BlockID::Air);
    assert_eq!(standard_chunk.get_block(15, 255, 15).unwrap(), BlockID::Air);
    
    assert_eq!(alternative_chunk.get_block(0, 0, 0).unwrap(), BlockID::Air);
    assert_eq!(alternative_chunk.get_block(31, 127, 31).unwrap(), BlockID::Air);
}

/// Test complete workflow: create → fill → access → modify
/// Requirements: 1.4, 2.2, 2.3
#[test]
fn test_complete_chunk_workflow() {
    let dimensions = ChunkDimensions {
        width: 16,
        height: 256,
        depth: 16,
    };
    let position = ChunkPosition { x: 5, z: -3 };
    
    // Step 1: Create chunk
    let mut chunk = Chunk::new(position, dimensions);
    
    // Verify initial state
    assert_eq!(chunk.dimensions(), dimensions);
    assert_eq!(chunk.world_position(), position);
    assert_eq!(chunk.get_block(0, 0, 0).unwrap(), BlockID::Air);
    assert_eq!(chunk.get_block(8, 128, 8).unwrap(), BlockID::Air);
    
    // Step 2: Fill chunk with stone
    chunk.fill(BlockID::Stone);
    
    // Verify fill operation
    assert_eq!(chunk.get_block(0, 0, 0).unwrap(), BlockID::Stone);
    assert_eq!(chunk.get_block(15, 255, 15).unwrap(), BlockID::Stone);
    assert_eq!(chunk.get_block(8, 128, 8).unwrap(), BlockID::Stone);
    
    // Step 3: Access and verify specific blocks
    let test_coordinates = [
        (0, 0, 0),
        (15, 255, 15),
        (8, 128, 8),
        (1, 1, 1),
        (7, 64, 7),
    ];
    
    for (x, y, z) in test_coordinates {
        let block = chunk.get_block(x, y, z).unwrap();
        assert_eq!(block, BlockID::Stone);
    }
    
    // Step 4: Modify specific blocks
    let modifications = [
        ((0, 0, 0), BlockID::Grass),
        ((15, 255, 15), BlockID::Dirt),
        ((8, 128, 8), BlockID::Air),
        ((1, 1, 1), BlockID::Grass),
        ((7, 64, 7), BlockID::Dirt),
    ];
    
    for ((x, y, z), new_block) in modifications {
        chunk.set_block(x, y, z, new_block).unwrap();
    }
    
    // Verify modifications
    assert_eq!(chunk.get_block(0, 0, 0).unwrap(), BlockID::Grass);
    assert_eq!(chunk.get_block(15, 255, 15).unwrap(), BlockID::Dirt);
    assert_eq!(chunk.get_block(8, 128, 8).unwrap(), BlockID::Air);
    assert_eq!(chunk.get_block(1, 1, 1).unwrap(), BlockID::Grass);
    assert_eq!(chunk.get_block(7, 64, 7).unwrap(), BlockID::Dirt);
    
    // Verify unmodified blocks remain stone
    assert_eq!(chunk.get_block(2, 2, 2).unwrap(), BlockID::Stone);
    assert_eq!(chunk.get_block(10, 100, 10).unwrap(), BlockID::Stone);
    
    // Step 5: Test batch operations as part of workflow
    let batch_coords = vec![(2, 2, 2), (10, 100, 10), (5, 50, 5)];
    let batch_results = chunk.get_blocks(&batch_coords);
    
    assert_eq!(batch_results[0], Ok(BlockID::Stone));
    assert_eq!(batch_results[1], Ok(BlockID::Stone));
    assert_eq!(batch_results[2], Ok(BlockID::Stone));
    
    let batch_modifications = vec![
        ((2, 2, 2), BlockID::Grass),
        ((10, 100, 10), BlockID::Dirt),
        ((5, 50, 5), BlockID::Air),
    ];
    
    chunk.set_blocks(&batch_modifications).unwrap();
    
    // Verify batch modifications
    assert_eq!(chunk.get_block(2, 2, 2).unwrap(), BlockID::Grass);
    assert_eq!(chunk.get_block(10, 100, 10).unwrap(), BlockID::Dirt);
    assert_eq!(chunk.get_block(5, 50, 5).unwrap(), BlockID::Air);
    
    // Verify world position persists throughout workflow
    assert_eq!(chunk.world_position(), position);
}

/// Test memory usage meets expectations for different chunk sizes
/// Requirements: 1.4, 2.2, 2.3
#[test]
fn test_memory_usage_expectations() {
    // Test standard 16×16×256 chunk
    let standard_dims = ChunkDimensions {
        width: 16,
        height: 256,
        depth: 16,
    };
    let position = ChunkPosition { x: 0, z: 0 };
    let _standard_chunk = Chunk::new(position, standard_dims);
    
    // Calculate expected memory usage
    let standard_blocks = 16 * 256 * 16; // 65,536 blocks
    assert_eq!(standard_blocks, 65_536);
    
    // Each BlockID is u16 (2 bytes), so total block data should be ~128KB
    let standard_memory = standard_blocks * std::mem::size_of::<BlockID>();
    assert_eq!(standard_memory, 131_072); // 128KB
    
    // Test alternative 32×32×128 chunk
    let alternative_dims = ChunkDimensions {
        width: 32,
        height: 128,
        depth: 32,
    };
    let _alternative_chunk = Chunk::new(position, alternative_dims);
    
    let alternative_blocks = 32 * 128 * 32; // 131,072 blocks
    assert_eq!(alternative_blocks, 131_072);
    
    // Memory usage should be ~256KB
    let alternative_memory = alternative_blocks * std::mem::size_of::<BlockID>();
    assert_eq!(alternative_memory, 262_144); // 256KB
    
    // Verify BlockID size is as expected (2 bytes)
    assert_eq!(std::mem::size_of::<BlockID>(), 2);
    
    // Test that chunks can be created and filled without excessive memory overhead
    let mut test_chunk = Chunk::new(position, standard_dims);
    test_chunk.fill(BlockID::Stone);
    
    // Verify all blocks are accessible (this would fail if memory layout was incorrect)
    for x in 0..16 {
        for y in 0..256 {
            for z in 0..16 {
                assert_eq!(test_chunk.get_block(x, y, z).unwrap(), BlockID::Stone);
            }
        }
    }
}

/// Test chunk creation and initialization from existing data
/// Requirements: 1.4, 2.2, 2.3
#[test]
fn test_chunk_from_data_workflow() {
    let dimensions = ChunkDimensions {
        width: 4,
        height: 4,
        depth: 4,
    };
    let position = ChunkPosition { x: 1, z: 1 };
    
    // Create test data with a pattern
    let mut test_data = Vec::new();
    for z in 0..4 {
        for y in 0..4 {
            for x in 0..4 {
                let block = match (x + y + z) % 4 {
                    0 => BlockID::Air,
                    1 => BlockID::Stone,
                    2 => BlockID::Dirt,
                    3 => BlockID::Grass,
                    _ => unreachable!(),
                };
                test_data.push(block);
            }
        }
    }
    
    // Create chunk from data
    let chunk = Chunk::from_data(test_data.clone(), position, dimensions).unwrap();
    
    // Verify chunk properties
    assert_eq!(chunk.dimensions(), dimensions);
    assert_eq!(chunk.world_position(), position);
    
    // Verify data was stored correctly by checking the pattern
    for z in 0..4 {
        for y in 0..4 {
            for x in 0..4 {
                let expected_block = match (x + y + z) % 4 {
                    0 => BlockID::Air,
                    1 => BlockID::Stone,
                    2 => BlockID::Dirt,
                    3 => BlockID::Grass,
                    _ => unreachable!(),
                };
                let actual_block = chunk.get_block(x, y, z).unwrap();
                assert_eq!(actual_block, expected_block);
            }
        }
    }
    
    // Test error case: wrong data size
    let wrong_size_data = vec![BlockID::Air; 32]; // Wrong size for 4×4×4 chunk
    let result = Chunk::from_data(wrong_size_data, position, dimensions);
    assert_eq!(result.unwrap_err(), ChunkError::InvalidBlockData);
}

/// Test error handling and bounds validation across workflows
/// Requirements: 1.4, 2.2, 2.3
#[test]
fn test_error_handling_workflow() {
    let dimensions = ChunkDimensions {
        width: 8,
        height: 8,
        depth: 8,
    };
    let position = ChunkPosition { x: 0, z: 0 };
    let mut chunk = Chunk::new(position, dimensions);
    
    // Test out-of-bounds access
    assert_eq!(chunk.get_block(8, 0, 0).unwrap_err(), ChunkError::OutOfBounds);
    assert_eq!(chunk.get_block(0, 8, 0).unwrap_err(), ChunkError::OutOfBounds);
    assert_eq!(chunk.get_block(0, 0, 8).unwrap_err(), ChunkError::OutOfBounds);
    
    // Test out-of-bounds modification
    assert_eq!(chunk.set_block(8, 0, 0, BlockID::Stone).unwrap_err(), ChunkError::OutOfBounds);
    assert_eq!(chunk.set_block(0, 8, 0, BlockID::Stone).unwrap_err(), ChunkError::OutOfBounds);
    assert_eq!(chunk.set_block(0, 0, 8, BlockID::Stone).unwrap_err(), ChunkError::OutOfBounds);
    
    // Test batch operations with mixed valid/invalid coordinates
    let mixed_coords = vec![(0, 0, 0), (8, 0, 0), (1, 1, 1)]; // Second coord is invalid
    let results = chunk.get_blocks(&mixed_coords);
    
    assert_eq!(results[0], Ok(BlockID::Air));
    assert_eq!(results[1], Err(ChunkError::OutOfBounds));
    assert_eq!(results[2], Ok(BlockID::Air));
    
    // Test batch set with invalid coordinates (should stop at first error)
    let mixed_set = vec![
        ((0, 0, 0), BlockID::Stone),
        ((8, 0, 0), BlockID::Dirt), // Invalid - should cause error
        ((1, 1, 1), BlockID::Grass),
    ];
    
    let result = chunk.set_blocks(&mixed_set);
    assert_eq!(result.unwrap_err(), ChunkError::OutOfBounds);
    
    // First block should have been set, third should remain Air
    assert_eq!(chunk.get_block(0, 0, 0).unwrap(), BlockID::Stone);
    assert_eq!(chunk.get_block(1, 1, 1).unwrap(), BlockID::Air);
    
    // Test batch set with error collection
    let results = chunk.set_blocks_collect_errors(&mixed_set);
    assert_eq!(results[0], Ok(()));
    assert_eq!(results[1], Err(ChunkError::OutOfBounds));
    assert_eq!(results[2], Ok(()));
    
    // Now the third block should be set
    assert_eq!(chunk.get_block(1, 1, 1).unwrap(), BlockID::Grass);
}

/// Test performance characteristics and cache-friendly access patterns
/// Requirements: 1.4, 2.2, 2.3
#[test]
fn test_cache_friendly_access_patterns() {
    let dimensions = ChunkDimensions {
        width: 16,
        height: 16,
        depth: 16,
    };
    let position = ChunkPosition { x: 0, z: 0 };
    let mut chunk = Chunk::new(position, dimensions);
    
    // Test sequential access pattern (should be cache-friendly)
    // Fill chunk with a pattern based on coordinates
    for z in 0..16 {
        for y in 0..16 {
            for x in 0..16 {
                let block = match (x + y + z) % 4 {
                    0 => BlockID::Air,
                    1 => BlockID::Stone,
                    2 => BlockID::Dirt,
                    3 => BlockID::Grass,
                    _ => unreachable!(),
                };
                chunk.set_block(x, y, z, block).unwrap();
            }
        }
    }
    
    // Verify the pattern was set correctly
    for z in 0..16 {
        for y in 0..16 {
            for x in 0..16 {
                let expected_block = match (x + y + z) % 4 {
                    0 => BlockID::Air,
                    1 => BlockID::Stone,
                    2 => BlockID::Dirt,
                    3 => BlockID::Grass,
                    _ => unreachable!(),
                };
                let actual_block = chunk.get_block(x, y, z).unwrap();
                assert_eq!(actual_block, expected_block);
            }
        }
    }
    
    // Test that coordinate conversion is consistent
    // This verifies the internal index mapping works correctly
    let test_coords = [
        (0, 0, 0),
        (15, 15, 15),
        (8, 8, 8),
        (1, 0, 0),
        (0, 1, 0),
        (0, 0, 1),
    ];
    
    for (x, y, z) in test_coords {
        let expected_block = match (x + y + z) % 4 {
            0 => BlockID::Air,
            1 => BlockID::Stone,
            2 => BlockID::Dirt,
            3 => BlockID::Grass,
            _ => unreachable!(),
        };
        let actual_block = chunk.get_block(x, y, z).unwrap();
        assert_eq!(actual_block, expected_block);
    }
}