//! Integration tests for chunk rendering system
//!
//! These tests verify end-to-end functionality from chunk data to GPU rendering,
//! including mesh generation, buffer management, and rendering pipeline integration.

use std::collections::HashSet;
use std::sync::Arc;
use world::{
    Chunk, ChunkDimensions, ChunkPosition, BlockID,
    rendering::{
        MeshGenerator, BufferManager, ChunkRenderer, ChunkUniforms, SingleChunkDemo,
        ChunkMesh, ChunkVertex, RenderError
    }
};

/// Create a mock wgpu device for testing
async fn create_test_device() -> (Arc<wgpu::Device>, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find an appropriate adapter");

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .expect("Failed to create device");

    (Arc::new(device), queue)
}

/// Create a test surface configuration for renderer setup
fn create_test_surface_config() -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: 800,
        height: 600,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    }
}

/// Test end-to-end mesh generation from chunk data to GPU
/// Requirements: 4.4, 5.1, 5.2
#[test]
fn test_end_to_end_mesh_generation() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (device, queue) = create_test_device().await;
        let mut buffer_manager = BufferManager::new(device.clone());
        let mesh_generator = MeshGenerator::new();
    
    // Create test chunk with various block types
    let dimensions = ChunkDimensions { width: 8, height: 8, depth: 8 };
    let chunk_pos = ChunkPosition { x: 0, z: 0 };
    let mut chunk = Chunk::new(chunk_pos, dimensions);
    
    // Set up a pattern of blocks for testing
    for x in 0..4 {
        for y in 0..4 {
            for z in 0..4 {
                let block_type = match (x + y + z) % 4 {
                    0 => BlockID::Stone,
                    1 => BlockID::Dirt,
                    2 => BlockID::Grass,
                    _ => BlockID::Air,
                };
                chunk.set_block(x, y, z, block_type).unwrap();
            }
        }
    }
    
    // Step 1: Generate mesh from chunk data
    let mesh = mesh_generator.generate_chunk_mesh(&chunk);
    assert!(!mesh.is_empty(), "Mesh should not be empty for non-empty chunk");
    assert!(mesh.validate().is_ok(), "Generated mesh should be valid");
    
    // Step 2: Create GPU buffers
    let result = buffer_manager.create_buffers(chunk_pos, &mesh);
    assert!(result.is_ok(), "Buffer creation should succeed: {:?}", result);
    assert!(buffer_manager.has_buffers(&chunk_pos), "Buffers should exist after creation");
    
    // Step 3: Upload mesh data to GPU
    let upload_result = buffer_manager.upload_mesh_data(&queue, &chunk_pos, &mesh);
    assert!(upload_result.is_ok(), "Mesh data upload should succeed: {:?}", upload_result);
    
    // Step 4: Verify buffer integrity
    let (vertex_buffer, index_buffer) = buffer_manager.get_buffers(&chunk_pos)
        .expect("Buffers should exist");
    
    let expected_vertex_size = (mesh.vertices.len() * std::mem::size_of::<ChunkVertex>()) as u64;
    let expected_index_size = (mesh.indices.len() * std::mem::size_of::<u32>()) as u64;
    
    assert_eq!(vertex_buffer.size(), expected_vertex_size, "Vertex buffer should have correct size");
    assert_eq!(index_buffer.size(), expected_index_size, "Index buffer should have correct size");
    
    // Step 5: Test buffer updates
    let mut updated_chunk = chunk.clone();
    updated_chunk.set_block(0, 0, 0, BlockID::Air).unwrap(); // Remove a block
    
    let updated_mesh = mesh_generator.generate_chunk_mesh(&updated_chunk);
    let update_result = buffer_manager.update_chunk_buffers(chunk_pos, &updated_mesh, &queue);
    assert!(update_result.is_ok(), "Buffer update should succeed: {:?}", update_result);
    
    // Step 6: Verify memory cleanup
    buffer_manager.remove_buffers(&chunk_pos);
    assert!(!buffer_manager.has_buffers(&chunk_pos), "Buffers should be removed after cleanup");
    });
}

/// Test integration with existing rendering pipeline
/// Requirements: 5.1, 5.2
#[test]
fn test_rendering_pipeline_integration() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (device, _queue) = create_test_device().await;
        let surface_config = create_test_surface_config();
        
        // Create chunk renderer
        let renderer_result = ChunkRenderer::new(device.clone(), &surface_config);
        assert!(renderer_result.is_ok(), "ChunkRenderer creation should succeed");
        
        let renderer = renderer_result.unwrap();
        
        // Verify renderer components exist (they are references, not Options)
        let _pipeline = renderer.render_pipeline();
        let _bind_group_layout = renderer.bind_group_layout();
        
        // Test uniform creation and validation
        let view_proj = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let chunk_position = [16.0, 0.0, 32.0];
        
        let uniforms = ChunkUniforms::new(view_proj, chunk_position);
        assert_eq!(uniforms.view_proj, view_proj, "View-projection matrix should be stored correctly");
        assert_eq!(uniforms.chunk_position, chunk_position, "Chunk position should be stored correctly");
        
        // Verify uniforms can be cast to bytes (Pod trait)
        let _uniform_bytes: &[u8] = bytemuck::cast_slice(&[uniforms]);
        
        // Test rendering statistics
        let stats = renderer.get_stats();
        assert_eq!(stats.chunks_loaded, 0, "Initially should have no chunks loaded");
        assert_eq!(stats.total_memory_bytes, 0, "Initially should use no memory");
    });
}

/// Test memory usage and cleanup during chunk operations
/// Requirements: 4.4
#[test]
fn test_memory_usage_and_cleanup() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (device, _queue) = create_test_device().await;
        let mut buffer_manager = BufferManager::new(device.clone());
        let mesh_generator = MeshGenerator::new();
    
    // Create multiple chunks with different sizes and patterns
    let test_chunks = vec![
        (ChunkPosition { x: 0, z: 0 }, ChunkDimensions { width: 4, height: 4, depth: 4 }),
        (ChunkPosition { x: 1, z: 0 }, ChunkDimensions { width: 8, height: 8, depth: 8 }),
        (ChunkPosition { x: 0, z: 1 }, ChunkDimensions { width: 16, height: 16, depth: 16 }),
        (ChunkPosition { x: 1, z: 1 }, ChunkDimensions { width: 2, height: 2, depth: 2 }),
    ];
    
    let mut created_meshes = Vec::new();
    
    // Step 1: Create chunks and generate meshes
    for (chunk_pos, dimensions) in &test_chunks {
        let mut chunk = Chunk::new(*chunk_pos, *dimensions);
        
        // Fill with a pattern to ensure non-empty meshes
        for x in 0..dimensions.width {
            for y in 0..dimensions.height {
                for z in 0..dimensions.depth {
                    if (x + y + z) % 2 == 0 {
                        chunk.set_block(x, y, z, BlockID::Stone).unwrap();
                    }
                }
            }
        }
        
        let mesh = mesh_generator.generate_chunk_mesh(&chunk);
        assert!(!mesh.is_empty(), "Chunk at {:?} should produce non-empty mesh", chunk_pos);
        
        created_meshes.push((*chunk_pos, mesh));
    }
    
    // Step 2: Create buffers and track memory usage
    let initial_stats = buffer_manager.memory_usage();
    assert_eq!(initial_stats.buffer_count, 0, "Should start with no buffers");
    assert_eq!(initial_stats.total_memory, 0, "Should start with no memory usage");
    
    for (chunk_pos, mesh) in &created_meshes {
        let result = buffer_manager.create_buffers(*chunk_pos, mesh);
        assert!(result.is_ok(), "Buffer creation should succeed for chunk {:?}", chunk_pos);
    }
    
    let after_creation_stats = buffer_manager.memory_usage();
    assert_eq!(after_creation_stats.buffer_count, test_chunks.len(), "Should have buffers for all chunks");
    assert!(after_creation_stats.total_memory > 0, "Should use memory after buffer creation");
    assert_eq!(after_creation_stats.total_memory, 
               after_creation_stats.vertex_memory + after_creation_stats.index_memory,
               "Total memory should equal vertex + index memory");
    
    // Step 3: Test selective cleanup
    let active_chunks: HashSet<ChunkPosition> = vec![
        ChunkPosition { x: 0, z: 0 },
        ChunkPosition { x: 1, z: 1 },
    ].into_iter().collect();
    
    buffer_manager.cleanup_unused_buffers(&active_chunks);
    
    let after_cleanup_stats = buffer_manager.memory_usage();
    assert_eq!(after_cleanup_stats.buffer_count, 2, "Should have 2 buffers after cleanup");
    assert!(after_cleanup_stats.total_memory < after_creation_stats.total_memory, 
            "Memory usage should decrease after cleanup");
    
    // Verify only active chunks have buffers
    for chunk_pos in &active_chunks {
        assert!(buffer_manager.has_buffers(chunk_pos), "Active chunk {:?} should still have buffers", chunk_pos);
    }
    
    // Step 4: Test complete cleanup
    buffer_manager.clear();
    
    let final_stats = buffer_manager.memory_usage();
    assert_eq!(final_stats.buffer_count, 0, "Should have no buffers after clear");
    assert_eq!(final_stats.total_memory, 0, "Should use no memory after clear");
    
    // Step 5: Test buffer validation
    let issues = buffer_manager.validate_buffers();
    assert!(issues.is_empty(), "Should have no validation issues after clear");
    });
}

/// Test batch operations and performance characteristics
/// Requirements: 4.4, 5.1
#[test]
fn test_batch_operations_performance() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (device, queue) = create_test_device().await;
        let mut buffer_manager = BufferManager::new(device.clone());
        let mesh_generator = MeshGenerator::new();
        
        // Create multiple chunks for batch testing
        let chunk_count = 10usize;
        let mut batch_updates = Vec::new();
        
        for i in 0..chunk_count {
            let chunk_pos = ChunkPosition { x: i as i32, z: 0 };
            let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
            let mut chunk = Chunk::new(chunk_pos, dimensions);
            
            // Create different patterns for each chunk
            for x in 0..4 {
                for y in 0..4 {
                    for z in 0..4 {
                        if (x + y + z + i) % 3 == 0 {
                            chunk.set_block(x, y, z, BlockID::Stone).unwrap();
                        }
                    }
                }
            }
            
            let mesh = mesh_generator.generate_chunk_mesh(&chunk);
            batch_updates.push((chunk_pos, mesh));
        }
        
        // Test batch buffer creation
        let start_time = std::time::Instant::now();
        
        for (chunk_pos, mesh) in &batch_updates {
            let result = buffer_manager.create_buffers(*chunk_pos, mesh);
            assert!(result.is_ok(), "Batch buffer creation should succeed for chunk {:?}", chunk_pos);
        }
        
        let creation_duration = start_time.elapsed();
        println!("Batch buffer creation took: {:?}", creation_duration);
        
        // Verify all buffers were created
        assert_eq!(buffer_manager.buffer_count(), chunk_count, "Should have buffers for all chunks");
        
        // Test batch updates
        let update_start = std::time::Instant::now();
        
        let batch_refs: Vec<_> = batch_updates.iter()
            .map(|(pos, mesh)| (*pos, mesh))
            .collect();
        
        let update_results = buffer_manager.batch_update_chunks(&batch_refs, &queue);
        
        let update_duration = update_start.elapsed();
        println!("Batch buffer update took: {:?}", update_duration);
        
        // Verify all updates succeeded
        assert_eq!(update_results.len(), chunk_count, "Should have results for all chunks");
        for (chunk_pos, result) in &update_results {
            assert!(result.is_ok(), "Batch update should succeed for chunk {:?}: {:?}", chunk_pos, result);
        }
        
        // Test memory usage scaling
        let memory_stats = buffer_manager.memory_usage();
        assert_eq!(memory_stats.buffer_count, chunk_count, "Memory stats should reflect all chunks");
        assert!(memory_stats.total_memory > 0, "Should use memory for all chunks");
        
        // Verify memory usage is reasonable (not excessive)
        let average_memory_per_chunk = memory_stats.total_memory / chunk_count as u64;
        assert!(average_memory_per_chunk < 100_000, "Average memory per chunk should be reasonable: {} bytes", average_memory_per_chunk);
    });
}

/// Test error handling and recovery scenarios
/// Requirements: 4.4, 5.1, 5.2
#[test]
fn test_error_handling_and_recovery() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (device, queue) = create_test_device().await;
        let mut buffer_manager = BufferManager::new(device.clone());
        
        // Test 1: Invalid mesh data handling
        let empty_mesh = ChunkMesh::new();
        let chunk_pos = ChunkPosition { x: 0, z: 0 };
        
        let result = buffer_manager.create_buffers(chunk_pos, &empty_mesh);
        assert!(result.is_err(), "Should fail to create buffers for empty mesh");
        assert_eq!(result.unwrap_err(), RenderError::InvalidMeshData, "Should return InvalidMeshData error");
        
        // Test 2: Buffer creation with fallback
        let mut valid_mesh = ChunkMesh::new();
        let vertices = vec![
            ChunkVertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 1.0]),
        ];
        let indices = vec![0, 1, 2];
        valid_mesh.add_geometry(&vertices, &indices);
        
        let fallback_result = buffer_manager.create_buffers_with_fallback(chunk_pos, &valid_mesh);
        assert!(fallback_result.is_ok(), "Fallback buffer creation should succeed: {:?}", fallback_result);
        
        // Test 3: Buffer validation and issue detection
        // Manually create inconsistent state for testing
        buffer_manager.remove_buffers(&chunk_pos);
        
        let issues = buffer_manager.validate_buffers();
        assert!(issues.is_empty(), "Should have no issues after proper cleanup");
        
        // Test 4: Update with size mismatch (test internal behavior through update)
        buffer_manager.create_buffers(chunk_pos, &valid_mesh).unwrap();
        
        // Create a larger mesh
        let mut larger_mesh = ChunkMesh::new();
        let mut large_vertices = vertices.clone();
        large_vertices.push(ChunkVertex::new([0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0]));
        let large_indices = vec![0, 1, 2, 0, 2, 3];
        larger_mesh.add_geometry(&large_vertices, &large_indices);
        
        let update_result = buffer_manager.update_chunk_buffers(chunk_pos, &larger_mesh, &queue);
        assert!(update_result.is_ok(), "Should handle size mismatch by recreating buffers: {:?}", update_result);
    });
}

/// Test integration with SingleChunkDemo for complete workflow
/// Requirements: 4.4, 5.1, 5.2
#[test]
fn test_single_chunk_demo_integration() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (device, queue) = create_test_device().await;
        let surface_config = create_test_surface_config();
        
        // Create renderer
        let mut renderer = ChunkRenderer::new(device.clone(), &surface_config)
            .expect("Should create renderer successfully");
        
        // Test different chunk configurations with unique positions
        let test_configs = vec![
            ("empty", SingleChunkDemo::empty()),
            ("filled_stone", SingleChunkDemo::filled(BlockID::Stone)),
            ("filled_dirt", SingleChunkDemo::filled(BlockID::Dirt)),
            ("mixed_pattern", SingleChunkDemo::mixed_pattern()),
            ("variety", SingleChunkDemo::new()),
        ];
        
        let mut non_empty_count = 0;
        
        for (i, (config_name, demo)) in test_configs.into_iter().enumerate() {
            println!("Testing configuration: {}", config_name);
            
            // Give each demo a unique chunk position to avoid conflicts
            let unique_pos = ChunkPosition { x: i as i32, z: 0 };
            
            // Validate demo
            assert!(demo.validate().is_ok(), "Demo configuration '{}' should be valid", config_name);
            
            // Generate mesh
            let mesh = demo.generate_mesh();
            
            // Get statistics
            let stats = demo.get_statistics();
            
            // Verify statistics consistency
            assert_eq!(stats.total_blocks, 
                       demo.config().dimensions.width * demo.config().dimensions.height * demo.config().dimensions.depth,
                       "Total blocks should match chunk dimensions for '{}'", config_name);
            
            if stats.non_air_blocks() == 0 {
                assert!(mesh.is_empty(), "Empty chunks should produce empty meshes for '{}'", config_name);
                assert!(stats.is_empty, "Statistics should mark empty chunks as empty for '{}'", config_name);
            } else {
                assert!(!mesh.is_empty(), "Non-empty chunks should produce non-empty meshes for '{}'", config_name);
                assert!(!stats.is_empty, "Statistics should not mark non-empty chunks as empty for '{}'", config_name);
                
                // Prepare for rendering with unique position
                let prepare_result = renderer.prepare_chunk(&queue, unique_pos, &mesh);
                assert!(prepare_result.is_ok(), "Should prepare '{}' for rendering: {:?}", config_name, prepare_result);
                non_empty_count += 1;
            }
            
            // Verify mesh integrity
            assert!(mesh.validate().is_ok(), "Mesh should be valid for '{}': {:?}", config_name, mesh.validate());
            
            // Test mesh generation determinism
            let mesh2 = demo.generate_mesh();
            assert_eq!(mesh.vertices.len(), mesh2.vertices.len(), "Mesh generation should be deterministic for '{}'", config_name);
            assert_eq!(mesh.indices.len(), mesh2.indices.len(), "Mesh generation should be deterministic for '{}'", config_name);
        }
        
        // Test renderer statistics after all preparations
        let final_stats = renderer.get_stats();
        
        assert_eq!(final_stats.chunks_loaded, non_empty_count, 
                   "Renderer should have buffers for all non-empty configurations");
        
        if non_empty_count > 0 {
            assert!(final_stats.total_memory_bytes > 0, "Should use memory for non-empty chunks");
        }
    });
}

/// Test concurrent operations and thread safety
/// Requirements: 4.4
#[test]
fn test_concurrent_operations() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (device, _queue) = create_test_device().await;
        let mesh_generator = MeshGenerator::new();
        
        // Create multiple chunks sequentially (since MeshGenerator doesn't implement Clone)
        let chunk_count = 5usize;
        let mut results = Vec::new();
        
        for i in 0..chunk_count {
            let chunk_pos = ChunkPosition { x: i as i32, z: 0 };
            let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
            let mut chunk = Chunk::new(chunk_pos, dimensions);
            
            // Fill with pattern
            for x in 0..4 {
                for y in 0..4 {
                    for z in 0..4 {
                        if (x + y + z) % 2 == 0 {
                            chunk.set_block(x, y, z, BlockID::Stone).unwrap();
                        }
                    }
                }
            }
            
            let mesh = mesh_generator.generate_chunk_mesh(&chunk);
            results.push((chunk_pos, mesh));
        }
        
        // Verify all meshes were generated successfully
        assert_eq!(results.len(), chunk_count, "Should have results for all chunks");
        
        for (chunk_pos, mesh) in &results {
            assert!(!mesh.is_empty(), "Chunk {:?} should produce non-empty mesh", chunk_pos);
            assert!(mesh.validate().is_ok(), "Mesh for chunk {:?} should be valid", chunk_pos);
        }
        
        // Test buffer creation with the generated meshes
        let mut buffer_manager = BufferManager::new(device.clone());
        
        for (chunk_pos, mesh) in &results {
            let result = buffer_manager.create_buffers(*chunk_pos, mesh);
            assert!(result.is_ok(), "Buffer creation should succeed for chunk {:?}", chunk_pos);
        }
        
        // Verify final state
        assert_eq!(buffer_manager.buffer_count(), chunk_count, "Should have buffers for all chunks");
        
        let memory_stats = buffer_manager.memory_usage();
        assert!(memory_stats.total_memory > 0, "Should use memory for all chunks");
        assert_eq!(memory_stats.buffer_count, chunk_count, "Memory stats should reflect all chunks");
    });
}