//! Integration test for terrain generation with World system

use crate::{World, WorldConfig, ChunkCoord};

#[test]
fn test_terrain_generation_integration() {
    // Create a world with terrain generator
    let config = WorldConfig::default();
    let mut world = World::new(config).expect("Failed to create world");
    
    // Verify terrain generator is initialized
    assert!(world.get_terrain_seed() > 0, "Terrain generator should have a valid seed");
    
    // Load a chunk using terrain generation
    let coord = ChunkCoord::new(0, 0, 0);
    let progress = world.load_single_chunk(coord).expect("Failed to load chunk");
    
    // Verify chunk was loaded successfully
    assert_eq!(progress.successful_loads(), 1, "Should have loaded 1 chunk");
    assert!(world.is_chunk_loaded(coord), "Chunk should be loaded");
    
    // Get the chunk and verify it has terrain data
    let chunk_entry = world.get_chunk(coord).expect("Chunk should exist");
    let chunk = &chunk_entry.chunk;
    
    // Verify chunk dimensions
    let dimensions = chunk.dimensions();
    assert_eq!(dimensions.width, 16, "Chunk width should be 16");
    assert_eq!(dimensions.height, 256, "Chunk height should be 256");
    assert_eq!(dimensions.depth, 16, "Chunk depth should be 16");
    
    // Sample blocks to verify terrain generation worked
    let surface_block = chunk.get_block(8, 64, 8).expect("Should be able to get surface block");
    let air_block = chunk.get_block(8, 200, 8).expect("Should be able to get air block");
    
    // Verify we have valid block types (terrain generation should produce valid blocks)
    use crate::chunk::BlockID;
    assert!(matches!(surface_block, BlockID::Air | BlockID::Grass | BlockID::Dirt | BlockID::Stone),
        "Surface block should be a valid terrain block type: {:?}", surface_block);
    assert!(matches!(air_block, BlockID::Air | BlockID::Grass | BlockID::Dirt | BlockID::Stone),
        "Air block should be a valid terrain block type: {:?}", air_block);
    
    println!("✓ Terrain generation integration test passed!");
    println!("  - World seed: {}", world.get_terrain_seed());
    println!("  - Surface block at (8,64,8): {:?}", surface_block);
    println!("  - Air block at (8,200,8): {:?}", air_block);
}

#[test]
fn test_terrain_generator_determinism() {
    // Create two worlds with the same seed
    let seed = 12345u64;
    let config = WorldConfig::default();
    
    let mut world1 = World::new_with_seed(config.clone(), Some(seed)).expect("Failed to create world1");
    let mut world2 = World::new_with_seed(config, Some(seed)).expect("Failed to create world2");
    
    // Verify both worlds have the same seed
    assert_eq!(world1.get_terrain_seed(), seed);
    assert_eq!(world2.get_terrain_seed(), seed);
    
    // Load the same chunk in both worlds
    let coord = ChunkCoord::new(1, 0, 1);
    
    let _progress1 = world1.load_single_chunk(coord).expect("Failed to load chunk in world1");
    let _progress2 = world2.load_single_chunk(coord).expect("Failed to load chunk in world2");
    
    // Get chunks from both worlds
    let chunk1 = &world1.get_chunk(coord).expect("Chunk1 should exist").chunk;
    let chunk2 = &world2.get_chunk(coord).expect("Chunk2 should exist").chunk;
    
    // Sample several blocks to verify determinism
    let sample_positions = [(0, 0, 0), (15, 64, 15), (8, 128, 8), (7, 200, 7)];
    
    for (x, y, z) in sample_positions {
        let block1 = chunk1.get_block(x, y, z).expect("Should get block from chunk1");
        let block2 = chunk2.get_block(x, y, z).expect("Should get block from chunk2");
        
        assert_eq!(block1, block2, 
            "Blocks at ({}, {}, {}) should be identical for same seed: {:?} vs {:?}", 
            x, y, z, block1, block2);
    }
    
    println!("✓ Terrain generator determinism test passed!");
}

#[test]
fn test_multiple_chunk_generation() {
    // Create a world
    let config = WorldConfig::default();
    let mut world = World::new(config).expect("Failed to create world");
    
    // Load multiple chunks
    let coords = vec![
        ChunkCoord::new(0, 0, 0),
        ChunkCoord::new(1, 0, 0),
        ChunkCoord::new(0, 0, 1),
        ChunkCoord::new(-1, 0, 0),
        ChunkCoord::new(0, 0, -1),
    ];
    
    let progress = world.load_custom_chunks(coords.clone()).expect("Failed to load chunks");
    
    // Verify all chunks were loaded
    assert_eq!(progress.successful_loads(), coords.len(), "All chunks should be loaded successfully");
    
    for coord in coords {
        assert!(world.is_chunk_loaded(coord), "Chunk {:?} should be loaded", coord);
        
        // Verify each chunk has valid terrain data
        let chunk_entry = world.get_chunk(coord).expect("Chunk should exist");
        let chunk = &chunk_entry.chunk;
        
        // Sample a block from each chunk to verify terrain generation
        let sample_block = chunk.get_block(8, 64, 8).expect("Should get sample block");
        
        use crate::chunk::BlockID;
        assert!(matches!(sample_block, BlockID::Air | BlockID::Grass | BlockID::Dirt | BlockID::Stone),
            "Sample block should be valid: {:?}", sample_block);
    }
    
    println!("✓ Multiple chunk generation test passed!");
}

#[test]
fn test_chunk_positioning_integration() {
    // Test requirement 8.4: Terrain_Generator SHALL coordinate with World system for chunk positioning
    let config = WorldConfig::default();
    let mut world = World::new(config).expect("Failed to create world");
    
    // Test various chunk coordinates including negative ones
    let test_coords = vec![
        ChunkCoord::new(0, 0, 0),
        ChunkCoord::new(5, 0, 3),
        ChunkCoord::new(-2, 0, 4),
        ChunkCoord::new(-1, 0, -1),
        ChunkCoord::new(10, 0, -5),
    ];
    
    for coord in test_coords {
        // Load the chunk
        let progress = world.load_single_chunk(coord).expect("Failed to load chunk");
        assert_eq!(progress.successful_loads(), 1, "Should load chunk successfully");
        
        // Verify chunk positioning
        let chunk_entry = world.get_chunk(coord).expect("Chunk should exist");
        let chunk = &chunk_entry.chunk;
        
        // Check that chunk position matches requested coordinates
        let chunk_position = chunk.world_position();
        assert_eq!(chunk_position.x, coord.x, "Chunk X position should match coordinate");
        assert_eq!(chunk_position.z, coord.z, "Chunk Z position should match coordinate");
        
        // Verify coordinate system integration
        let world_pos = world.chunk_to_world_pos(coord);
        let recovered_coord = world.world_to_chunk_coord(world_pos);
        assert_eq!(recovered_coord.x, coord.x, "Coordinate conversion should be consistent for X");
        assert_eq!(recovered_coord.z, coord.z, "Coordinate conversion should be consistent for Z");
        
        // Test chunk bounds calculation
        let (min_bounds, max_bounds) = world.chunk_bounds(coord);
        let expected_min_x = coord.x as f32 * 32.0; // Assuming 32x32 chunks
        let expected_min_z = coord.z as f32 * 32.0;
        assert!((min_bounds.x - expected_min_x).abs() < 0.001, 
            "Chunk bounds should be correctly calculated");
        assert!((min_bounds.z - expected_min_z).abs() < 0.001, 
            "Chunk bounds should be correctly calculated");
        
        // Verify chunk dimensions are correct
        let dimensions = chunk.dimensions();
        assert_eq!(dimensions.width, 16, "Chunk width should be 16");
        assert_eq!(dimensions.height, 256, "Chunk height should be 256");
        assert_eq!(dimensions.depth, 16, "Chunk depth should be 16");
    }
    
    println!("✓ Chunk positioning integration test passed!");
}

#[test]
fn test_rendering_pipeline_integration() {
    // Test requirement 8.5: Terrain_Generator SHALL ensure generated chunks are ready for rendering pipeline
    let config = WorldConfig::default();
    let mut world = World::new(config).expect("Failed to create world");
    
    // Load a chunk
    let coord = ChunkCoord::new(2, 0, -1);
    let progress = world.load_single_chunk(coord).expect("Failed to load chunk");
    assert_eq!(progress.successful_loads(), 1, "Should load chunk successfully");
    
    // Verify chunk is in Generated state (ready for rendering pipeline)
    let chunk_entry = world.get_chunk(coord).expect("Chunk should exist");
    assert_eq!(chunk_entry.state, crate::world::ChunkState::Generated, 
        "Loaded chunk should be in Generated state");
    
    // Test chunk state transitions for rendering pipeline
    let can_prepare = world.prepare_chunk_for_rendering(coord);
    assert!(can_prepare.is_ok(), "Should be able to prepare chunk for rendering");
    
    // Verify chunk progressed through rendering states
    let chunk_entry = world.get_chunk(coord).expect("Chunk should exist");
    assert!(matches!(chunk_entry.state, crate::world::ChunkState::Meshed | crate::world::ChunkState::RenderReady),
        "Chunk should be in Meshed or RenderReady state after preparation");
    
    // Test that chunk can be marked as render-ready
    let mark_result = world.mark_chunk_render_ready(coord);
    assert!(mark_result.is_ok(), "Should be able to mark chunk as render-ready");
    
    let chunk_entry = world.get_chunk(coord).expect("Chunk should exist");
    assert_eq!(chunk_entry.state, crate::world::ChunkState::RenderReady,
        "Chunk should be in RenderReady state");
    
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
                    use crate::chunk::BlockID;
                    assert!(matches!(block_type, BlockID::Air | BlockID::Grass | BlockID::Dirt | BlockID::Stone),
                        "Block type should be valid for rendering: {:?}", block_type);
                }
            }
        }
    }
    
    println!("✓ Rendering pipeline integration test passed!");
}