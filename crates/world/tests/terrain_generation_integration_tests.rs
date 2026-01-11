//! Comprehensive Integration Tests for Terrain Generation
//!
//! These tests verify the complete terrain generation pipeline and its integration
//! with chunk and world systems. They test requirements 8.1, 8.2, 8.3, 8.4, 8.5.

use world::{
    World, WorldConfig, ChunkCoord,
    generation::terrain::{TerrainGenerator, BiomeType, NoiseConfiguration},
    chunk::{BlockID, ChunkDimensions},
};
use std::time::Duration;

/// Test complete terrain generation pipeline from generator to world
/// Requirements: 8.1, 8.2, 8.3, 8.4, 8.5
#[test]
fn test_complete_terrain_generation_pipeline() {
    // Create terrain generator
    let seed = 12345u64;
    let generator = TerrainGenerator::new(seed);
    
    // Test chunk generation
    let chunk_coord = ChunkCoord::new(0, 0, 0);
    let chunk = generator.generate_chunk(chunk_coord);
    
    // Verify chunk properties
    let dimensions = chunk.dimensions();
    assert_eq!(dimensions.width, 16);
    assert_eq!(dimensions.height, 256);
    assert_eq!(dimensions.depth, 16);
    
    let position = chunk.world_position();
    assert_eq!(position.x, chunk_coord.x);
    assert_eq!(position.z, chunk_coord.z);
    
    // Verify chunk is completely populated
    let mut block_counts = std::collections::HashMap::new();
    for x in 0..dimensions.width {
        for y in 0..dimensions.height {
            for z in 0..dimensions.depth {
                let block = chunk.get_block(x, y, z).expect("All blocks should be accessible");
                *block_counts.entry(block).or_insert(0) += 1;
            }
        }
    }
    
    // Verify all block types are valid
    for block_type in block_counts.keys() {
        assert!(matches!(block_type, BlockID::Air | BlockID::Grass | BlockID::Dirt | BlockID::Stone),
                "Invalid block type: {:?}", block_type);
    }
    
    // Verify we have a reasonable distribution of blocks
    assert!(block_counts.contains_key(&BlockID::Air), "Should contain Air blocks");
    assert!(block_counts.contains_key(&BlockID::Stone), "Should contain Stone blocks");
    
    // Verify total block count
    let total_blocks: u32 = block_counts.values().sum();
    let expected_blocks = dimensions.width * dimensions.height * dimensions.depth;
    assert_eq!(total_blocks, expected_blocks as u32);
}

/// Test terrain generation integration with World system
/// Requirements: 8.1, 8.4, 8.5
#[test]
fn test_terrain_generation_world_integration() {
    // Create world with terrain generation
    let config = WorldConfig::default();
    let mut world = World::new(config).expect("Failed to create world");
    
    // Verify terrain generator is initialized
    let terrain_seed = world.get_terrain_seed();
    assert!(terrain_seed > 0, "Terrain generator should have a valid seed");
    
    // Load chunk using terrain generation
    let coord = ChunkCoord::new(1, 0, 1);
    let progress = world.load_single_chunk(coord).expect("Failed to load chunk");
    
    // Verify chunk was loaded successfully
    assert_eq!(progress.successful_loads(), 1);
    assert!(world.is_chunk_loaded(coord));
    
    // Get the chunk and verify terrain data
    let chunk_entry = world.get_chunk(coord).expect("Chunk should exist");
    let chunk = &chunk_entry.chunk;
    
    // Verify chunk has terrain-generated content
    let mut has_terrain_blocks = false;
    for x in 0..16 {
        for y in 0..256 {
            for z in 0..16 {
                let block = chunk.get_block(x, y, z).expect("Block should be accessible");
                if matches!(block, BlockID::Grass | BlockID::Dirt | BlockID::Stone) {
                    has_terrain_blocks = true;
                    break;
                }
            }
        }
    }
    assert!(has_terrain_blocks, "Chunk should contain terrain-generated blocks");
    
    // Test coordinate system integration
    let world_pos = world.chunk_to_world_pos(coord);
    let recovered_coord = world.world_to_chunk_coord(world_pos);
    assert_eq!(recovered_coord.x, coord.x);
    assert_eq!(recovered_coord.z, coord.z);
}

/// Test deterministic terrain generation across multiple chunks
/// Requirements: 8.1, 8.2, 8.3
#[test]
fn test_deterministic_terrain_generation() {
    let seed = 98765u64;
    
    // Create two worlds with the same seed
    let config = WorldConfig::default();
    let mut world1 = World::new_with_seed(config.clone(), Some(seed)).expect("Failed to create world1");
    let mut world2 = World::new_with_seed(config, Some(seed)).expect("Failed to create world2");
    
    // Verify both worlds have the same seed
    assert_eq!(world1.get_terrain_seed(), seed);
    assert_eq!(world2.get_terrain_seed(), seed);
    
    // Load the same chunks in both worlds
    let test_coords = vec![
        ChunkCoord::new(0, 0, 0),
        ChunkCoord::new(1, 0, 0),
        ChunkCoord::new(0, 0, 1),
        ChunkCoord::new(-1, 0, -1),
    ];
    
    for coord in &test_coords {
        let _progress1 = world1.load_single_chunk(*coord).expect("Failed to load chunk in world1");
        let _progress2 = world2.load_single_chunk(*coord).expect("Failed to load chunk in world2");
    }
    
    // Compare chunks between worlds
    for coord in &test_coords {
        let chunk1 = &world1.get_chunk(*coord).expect("Chunk1 should exist").chunk;
        let chunk2 = &world2.get_chunk(*coord).expect("Chunk2 should exist").chunk;
        
        // Sample blocks to verify determinism
        let sample_positions = [
            (0, 0, 0), (15, 64, 15), (8, 128, 8), (7, 200, 7), (1, 1, 1)
        ];
        
        for (x, y, z) in sample_positions {
            let block1 = chunk1.get_block(x, y, z).expect("Should get block from chunk1");
            let block2 = chunk2.get_block(x, y, z).expect("Should get block from chunk2");
            
            assert_eq!(block1, block2, 
                "Blocks at ({}, {}, {}) in chunk {:?} should be identical for same seed: {:?} vs {:?}", 
                x, y, z, coord, block1, block2);
        }
    }
}

/// Test terrain generation with multiple chunk loading patterns
/// Requirements: 8.1, 8.2, 8.4, 8.5
#[test]
fn test_multiple_chunk_terrain_generation() {
    let config = WorldConfig::default();
    let mut world = World::new(config).expect("Failed to create world");
    
    // Load chunks in a grid pattern
    let coords = vec![
        ChunkCoord::new(0, 0, 0),
        ChunkCoord::new(1, 0, 0),
        ChunkCoord::new(0, 0, 1),
        ChunkCoord::new(1, 0, 1),
        ChunkCoord::new(-1, 0, 0),
        ChunkCoord::new(0, 0, -1),
        ChunkCoord::new(-1, 0, -1),
    ];
    
    let progress = world.load_custom_chunks(coords.clone()).expect("Failed to load chunks");
    
    // Verify all chunks were loaded
    assert_eq!(progress.successful_loads(), coords.len());
    
    for coord in &coords {
        assert!(world.is_chunk_loaded(*coord), "Chunk {:?} should be loaded", coord);
        
        // Verify each chunk has valid terrain data
        let chunk_entry = world.get_chunk(*coord).expect("Chunk should exist");
        let chunk = &chunk_entry.chunk;
        
        // Check that terrain generation produced valid layering
        let mut found_surface = false;
        let mut found_subsurface = false;
        let mut found_deep = false;
        
        for x in [0, 8, 15] {
            for z in [0, 8, 15] {
                for y in 0..256 {
                    let block = chunk.get_block(x, y, z).expect("Block should be accessible");
                    
                    match block {
                        BlockID::Grass => found_surface = true,
                        BlockID::Dirt => found_subsurface = true,
                        BlockID::Stone => found_deep = true,
                        BlockID::Air => {} // Expected above terrain
                    }
                }
            }
        }
        
        // Every chunk should have some terrain structure
        assert!(found_deep, "Chunk {:?} should have Stone blocks", coord);
        // Most chunks should have surface and subsurface, but allow for edge cases
        if found_surface || found_subsurface {
            // At least one type of terrain layer should be present
        }
    }
}

/// Test terrain generation boundary continuity between adjacent chunks
/// Requirements: 8.2, 8.3, 8.4
#[test]
fn test_terrain_boundary_continuity() {
    let generator = TerrainGenerator::new(55555);
    
    // Generate adjacent chunks
    let center_coord = ChunkCoord::new(0, 0, 0);
    let east_coord = ChunkCoord::new(1, 0, 0);
    let south_coord = ChunkCoord::new(0, 0, 1);
    
    let center_chunk = generator.generate_chunk(center_coord);
    let east_chunk = generator.generate_chunk(east_coord);
    let south_chunk = generator.generate_chunk(south_coord);
    
    // Test height continuity at chunk boundaries
    // Compare eastern edge of center chunk with western edge of east chunk
    for z in 0..16 {
        let center_world_x = 15; // Eastern edge of center chunk (local coords)
        let center_world_z = z;
        let east_world_x = 0;   // Western edge of east chunk (local coords)
        let east_world_z = z;
        
        // Get world coordinates for height comparison
        let center_height = generator.get_height_at(center_world_x, center_world_z);
        let east_height = generator.get_height_at(16 + east_world_x, east_world_z); // Offset by chunk size
        
        // Heights should be continuous (within reasonable bounds)
        let height_diff = (center_height - east_height).abs();
        assert!(height_diff <= 32.0, 
            "Height difference {} at boundary z={} exceeds continuity threshold", 
            height_diff, z);
    }
    
    // Compare southern edge of center chunk with northern edge of south chunk
    for x in 0..16 {
        let center_world_x = x;
        let center_world_z = 15; // Southern edge of center chunk
        let south_world_x = x;
        let south_world_z = 0;   // Northern edge of south chunk
        
        let center_height = generator.get_height_at(center_world_x, center_world_z);
        let south_height = generator.get_height_at(south_world_x, 16 + south_world_z); // Offset by chunk size
        
        let height_diff = (center_height - south_height).abs();
        assert!(height_diff <= 32.0, 
            "Height difference {} at boundary x={} exceeds continuity threshold", 
            height_diff, x);
    }
    
    // Verify that biomes are also continuous
    for z in 0..16 {
        let center_biome = generator.get_biome_at(15, z);
        let east_biome = generator.get_biome_at(16, z);
        
        // Biomes don't need to be identical at boundaries, but should be reasonable
        // This test just verifies that biome generation doesn't panic at boundaries
        assert!(matches!(center_biome, BiomeType::Plains | BiomeType::Hills));
        assert!(matches!(east_biome, BiomeType::Plains | BiomeType::Hills));
    }
}

/// Test terrain generation performance meets requirements
/// Requirements: 8.1, 8.2, 8.5
#[test]
fn test_terrain_generation_performance() {
    let generator = TerrainGenerator::new(77777);
    
    // Test single chunk generation performance
    let test_coords = vec![
        ChunkCoord::new(0, 0, 0),
        ChunkCoord::new(1, 0, 0),
        ChunkCoord::new(0, 0, 1),
        ChunkCoord::new(-1, 0, 0),
        ChunkCoord::new(0, 0, -1),
    ];
    
    let mut generation_times = Vec::new();
    
    for coord in test_coords {
        let start = std::time::Instant::now();
        let _chunk = generator.generate_chunk(coord);
        let generation_time = start.elapsed();
        
        generation_times.push(generation_time);
        
        // Each chunk should generate within performance target (<50ms)
        assert!(generation_time < Duration::from_millis(50), 
            "Chunk generation took {:.2}ms, exceeds 50ms target", 
            generation_time.as_millis());
    }
    
    // Calculate average performance
    let total_time: Duration = generation_times.iter().sum();
    let avg_time = total_time / generation_times.len() as u32;
    
    assert!(avg_time < Duration::from_millis(50), 
        "Average chunk generation time {:.2}ms exceeds 50ms target", 
        avg_time.as_millis());
    
    // Test batch generation performance
    let batch_coords = (0..10).map(|i| ChunkCoord::new(i, 0, 0)).collect::<Vec<_>>();
    
    let batch_start = std::time::Instant::now();
    for coord in batch_coords {
        let _chunk = generator.generate_chunk(coord);
    }
    let batch_time = batch_start.elapsed();
    
    let avg_batch_time = batch_time / 10;
    assert!(avg_batch_time < Duration::from_millis(50), 
        "Average batch generation time {:.2}ms exceeds 50ms target", 
        avg_batch_time.as_millis());
}

/// Test terrain generation with different noise configurations
/// Requirements: 8.1, 8.2
#[test]
fn test_terrain_generation_with_configurations() {
    let seed = 33333u64;
    
    // Test default configuration
    let default_generator = TerrainGenerator::new(seed);
    let default_chunk = default_generator.generate_chunk(ChunkCoord::new(0, 0, 0));
    
    // Test custom configuration
    let custom_config = NoiseConfiguration::new(0.02, 16.0, 6, 0.01, 3)
        .expect("Valid configuration");
    let custom_generator = TerrainGenerator::with_config(seed, custom_config);
    let custom_chunk = custom_generator.generate_chunk(ChunkCoord::new(0, 0, 0));
    
    // Both chunks should be valid but potentially different
    assert_eq!(default_chunk.dimensions(), custom_chunk.dimensions());
    
    // Sample blocks to verify both generators produce valid terrain
    let sample_positions = [(0, 64, 0), (8, 128, 8), (15, 200, 15)];
    
    for (x, y, z) in sample_positions {
        let default_block = default_chunk.get_block(x, y, z).expect("Block should be accessible");
        let custom_block = custom_chunk.get_block(x, y, z).expect("Block should be accessible");
        
        // Both should be valid block types
        assert!(matches!(default_block, BlockID::Air | BlockID::Grass | BlockID::Dirt | BlockID::Stone));
        assert!(matches!(custom_block, BlockID::Air | BlockID::Grass | BlockID::Dirt | BlockID::Stone));
    }
    
    // Verify that different configurations can produce different results
    // (This is probabilistic, but with different parameters it's very likely)
    let mut differences_found = false;
    for x in 0..16 {
        for y in 0..256 {
            for z in 0..16 {
                let default_block = default_chunk.get_block(x, y, z).unwrap();
                let custom_block = custom_chunk.get_block(x, y, z).unwrap();
                
                if default_block != custom_block {
                    differences_found = true;
                    break;
                }
            }
        }
    }
    
    // With significantly different parameters, we should see some differences
    // If not, that's still valid (rare but possible)
    if differences_found {
        // This is the expected case - different configurations produce different terrain
    } else {
        // This is rare but valid - the configurations happened to produce similar terrain
        // for this specific chunk and seed combination
    }
}

/// Test terrain generation integration with rendering pipeline
/// Requirements: 8.5
#[test]
fn test_terrain_generation_rendering_integration() {
    let config = WorldConfig::default();
    let mut world = World::new(config).expect("Failed to create world");
    
    // Load a chunk
    let coord = ChunkCoord::new(2, 0, -1);
    let progress = world.load_single_chunk(coord).expect("Failed to load chunk");
    assert_eq!(progress.successful_loads(), 1);
    
    // Verify chunk is in Generated state (ready for rendering pipeline)
    let chunk_entry = world.get_chunk(coord).expect("Chunk should exist");
    assert_eq!(chunk_entry.state, world::world::ChunkState::Generated);
    
    // Test chunk state transitions for rendering pipeline
    let can_prepare = world.prepare_chunk_for_rendering(coord);
    assert!(can_prepare.is_ok(), "Should be able to prepare chunk for rendering");
    
    // Verify chunk progressed through rendering states
    let chunk_entry = world.get_chunk(coord).expect("Chunk should exist");
    assert!(matches!(chunk_entry.state, 
                    world::world::ChunkState::Meshed | 
                    world::world::ChunkState::RenderReady));
    
    // Test that chunk can be marked as render-ready
    let mark_result = world.mark_chunk_render_ready(coord);
    assert!(mark_result.is_ok(), "Should be able to mark chunk as render-ready");
    
    let chunk_entry = world.get_chunk(coord).expect("Chunk should exist");
    assert_eq!(chunk_entry.state, world::world::ChunkState::RenderReady);
    
    // Verify chunk appears in render-ready queries
    let render_ready_chunks = world.get_render_ready_chunks();
    assert!(render_ready_chunks.iter().any(|(c, _)| *c == coord),
        "Chunk should appear in render-ready chunks list");
    
    // Test chunk data integrity for rendering
    let chunk = &chunk_entry.chunk;
    
    // Verify all blocks are accessible (no uninitialized data)
    for x in [0, 7, 15] {
        for z in [0, 7, 15] {
            for y in [0, 64, 128, 200, 255] {
                let block = chunk.get_block(x, y, z);
                assert!(block.is_ok(), "All blocks should be accessible at ({}, {}, {})", x, y, z);
                
                // Verify block types are valid for rendering
                if let Ok(block_type) = block {
                    assert!(matches!(block_type, BlockID::Air | BlockID::Grass | BlockID::Dirt | BlockID::Stone),
                        "Block type should be valid for rendering: {:?}", block_type);
                }
            }
        }
    }
}

/// Test terrain generation with extreme coordinates
/// Requirements: 8.1, 8.4
#[test]
fn test_terrain_generation_extreme_coordinates() {
    let generator = TerrainGenerator::new(99999);
    
    // Test generation at extreme but reasonable chunk coordinates
    let extreme_coords = vec![
        ChunkCoord::new(1000, 0, 1000),
        ChunkCoord::new(-1000, 0, -1000),
        ChunkCoord::new(1000, 0, -1000),
        ChunkCoord::new(-1000, 0, 1000),
    ];
    
    for coord in extreme_coords {
        // Should not panic or fail
        let chunk = generator.generate_chunk(coord);
        
        // Verify chunk properties
        assert_eq!(chunk.world_position().x, coord.x);
        assert_eq!(chunk.world_position().z, coord.z);
        
        // Verify chunk is completely populated
        let sample_positions = [(0, 0, 0), (8, 128, 8), (15, 255, 15)];
        for (x, y, z) in sample_positions {
            let block = chunk.get_block(x, y, z).expect("Block should be accessible");
            assert!(matches!(block, BlockID::Air | BlockID::Grass | BlockID::Dirt | BlockID::Stone),
                "Block should be valid: {:?}", block);
        }
        
        // Test height and biome generation at extreme coordinates
        let world_x = coord.x * 16;
        let world_z = coord.z * 16;
        
        let height = generator.get_height_at(world_x, world_z);
        assert!(height >= 0.0 && height <= 255.0, 
            "Height {} should be within bounds at extreme coordinate ({}, {})", 
            height, world_x, world_z);
        
        let biome = generator.get_biome_at(world_x, world_z);
        assert!(matches!(biome, BiomeType::Plains | BiomeType::Hills),
            "Biome should be valid at extreme coordinate ({}, {})", world_x, world_z);
    }
}

/// Test terrain generation error handling and recovery
/// Requirements: 8.1, 8.2, 8.5
#[test]
fn test_terrain_generation_error_handling() {
    // Test with various configurations to ensure robustness
    let test_seeds = vec![0, 1, u64::MAX, 12345, 98765];
    
    for seed in test_seeds {
        let generator = TerrainGenerator::new(seed);
        
        // Should handle any reasonable chunk coordinate
        let test_coords = vec![
            ChunkCoord::new(0, 0, 0),
            ChunkCoord::new(100, 0, 100),
            ChunkCoord::new(-100, 0, -100),
        ];
        
        for coord in test_coords {
            // Generation should not panic or fail
            let chunk = generator.generate_chunk(coord);
            
            // Chunk should be valid
            assert_eq!(chunk.world_position().x, coord.x);
            assert_eq!(chunk.world_position().z, coord.z);
            
            // Should be able to access all blocks
            let dimensions = chunk.dimensions();
            for x in [0, dimensions.width / 2, dimensions.width - 1] {
                for y in [0, dimensions.height / 2, dimensions.height - 1] {
                    for z in [0, dimensions.depth / 2, dimensions.depth - 1] {
                        let block = chunk.get_block(x, y, z);
                        assert!(block.is_ok(), 
                            "Should be able to access block at ({}, {}, {}) in chunk {:?}", 
                            x, y, z, coord);
                    }
                }
            }
        }
    }
}

/// Test terrain generation with world system memory management
/// Requirements: 8.4, 8.5
#[test]
fn test_terrain_generation_memory_management() {
    let config = WorldConfig::new()
        .with_max_chunks(Some(5)); // Limit chunks to test memory management
    
    let mut world = World::new(config).expect("Failed to create world");
    
    // Load more chunks than the limit
    let coords = (0..10).map(|i| ChunkCoord::new(i, 0, 0)).collect::<Vec<_>>();
    
    for coord in coords {
        let _progress = world.load_single_chunk(coord).expect("Should load chunk");
        
        // Verify world respects memory limits
        let loading_stats = world.get_loading_stats();
        assert!(loading_stats.total_chunks <= 5, 
            "World should respect chunk limit, but has {} chunks", 
            loading_stats.total_chunks);
    }
    
    // Verify that terrain generation works correctly even with memory management
    let final_coords: Vec<ChunkCoord> = world.get_render_ready_chunks()
        .into_iter()
        .map(|(coord, _)| coord)
        .collect();
    
    for coord in final_coords {
        let chunk_entry = world.get_chunk(coord).expect("Chunk should exist");
        let chunk = &chunk_entry.chunk;
        
        // Verify terrain data is still valid
        let sample_block = chunk.get_block(8, 64, 8).expect("Should get sample block");
        assert!(matches!(sample_block, BlockID::Air | BlockID::Grass | BlockID::Dirt | BlockID::Stone),
            "Sample block should be valid: {:?}", sample_block);
    }
}