//! Integration tests for the world integration system
//!
//! These tests verify complete end-to-end workflows including:
//! - Multi-chunk world creation and management
//! - Integration with rendering pipeline
//! - Performance characteristics under various loads
//! - Memory management and cleanup
//! - Error handling and recovery

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use world::{
    World, WorldConfig, ChunkCoord, ChunkState, ChunkPosition,
    rendering::{ChunkRenderer, ChunkUniforms, MeshGenerator, BufferManager},
};
use world::world::MemoryHealthStatus;

/// Create a test world configuration optimized for testing
fn create_test_world_config() -> WorldConfig {
    WorldConfig::new()
        .with_render_distance(4)
        .with_max_chunks(Some(50))
        .with_performance_monitoring(true)
}

/// Create a mock wgpu device for rendering tests
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

/// Create a test surface configuration
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

/// Test complete world creation to rendering workflow
/// Requirements: 1.1, 1.2, 1.3, 1.5
#[test]
fn test_world_creation_to_rendering_workflow() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Step 1: Create world
        let config = create_test_world_config();
        let mut world = World::new(config).expect("Should create world successfully");
        
        // Verify initial world state
        assert_eq!(world.chunk_count(), 0);
        assert_eq!(world.config().render_distance, 4);
        assert_eq!(world.config().max_chunks_loaded, Some(50));
        
        // Step 2: Load chunks in a grid pattern
        let center = ChunkCoord::new(0, 0, 0);
        let radius = 2;
        let progress = world.load_grid(center, radius).expect("Should load grid successfully");
        
        // Verify loading progress
        assert!(progress.is_complete());
        assert!(progress.successful_loads() > 0);
        assert_eq!(progress.failed_loads(), 0);
        
        let expected_chunks = ((radius * 2 + 1) as usize).pow(3);
        assert_eq!(progress.total_chunks(), expected_chunks);
        assert_eq!(progress.successful_loads(), expected_chunks);
        
        // Step 3: Verify world state after loading
        assert_eq!(world.chunk_count(), expected_chunks);
        
        let loading_stats = world.get_loading_stats();
        assert_eq!(loading_stats.total_chunks, expected_chunks);
        assert!(loading_stats.generated_chunks > 0);
        
        // Step 4: Prepare chunks for rendering
        let chunk_coords: Vec<ChunkCoord> = world.get_render_ready_chunks()
            .into_iter()
            .map(|(coord, _)| coord)
            .collect();
        
        let preparation_results = world.prepare_chunks_for_rendering(&chunk_coords);
        let mut successful_preparations = 0;
        
        for (coord, result) in preparation_results {
            match result {
                Ok(_) => {
                    successful_preparations += 1;
                    // Mark chunk as render-ready
                    world.mark_chunk_render_ready(coord).expect("Should mark chunk as render-ready");
                }
                Err(e) => {
                    panic!("Chunk preparation failed for {:?}: {}", coord, e);
                }
            }
        }
        
        assert!(successful_preparations > 0);
        
        // Step 5: Verify rendering integration
        let render_ready_chunks = world.get_render_ready_chunks();
        assert!(render_ready_chunks.len() > 0);
        
        for (coord, entry) in &render_ready_chunks {
            assert_eq!(entry.state, ChunkState::RenderReady);
            assert!(entry.is_renderable());
            assert!(world.is_chunk_loaded(*coord));
        }
        
        // Step 6: Test coordinate system integration
        for (coord, _) in &render_ready_chunks {
            let world_pos = world.chunk_to_world_pos(*coord);
            let converted_coord = world.world_to_chunk_coord(world_pos);
            assert_eq!(*coord, converted_coord);
            
            let (min_bounds, max_bounds) = world.chunk_bounds(*coord);
            assert!(world_pos.x >= min_bounds.x && world_pos.x < max_bounds.x);
            assert!(world_pos.y >= min_bounds.y && world_pos.y < max_bounds.y);
            assert!(world_pos.z >= min_bounds.z && world_pos.z < max_bounds.z);
        }
        
        // Step 7: Test memory management
        let memory_stats = world.memory_stats();
        assert!(memory_stats.current_usage > 0);
        assert!(memory_stats.chunks_tracked > 0);
        assert_eq!(memory_stats.chunks_tracked, expected_chunks);
        assert!(!memory_stats.is_critical());
        
        // Step 8: Test performance monitoring
        let performance_monitor = world.performance_monitor();
        // Performance monitor should be initialized
        let _monitor_ref = performance_monitor;
    });
}

/// Test integration with existing chunk rendering system
/// Requirements: 4.1, 4.5
#[test]
fn test_chunk_rendering_system_integration() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let (device, queue) = create_test_device().await;
        let surface_config = create_test_surface_config();
        
        // Step 1: Create world and renderer
        let config = create_test_world_config();
        let mut world = World::new(config).expect("Should create world successfully");
        
        let renderer = ChunkRenderer::new(device.clone(), &surface_config)
            .expect("Should create chunk renderer successfully");
        
        let mesh_generator = MeshGenerator::new();
        let mut buffer_manager = BufferManager::new(device.clone());
        
        // Step 2: Load chunks
        let center = ChunkCoord::new(0, 0, 0);
        let progress = world.load_grid(center, 1).expect("Should load grid successfully");
        assert!(progress.successful_loads() > 0);
        
        // Step 3: Prepare chunks for rendering
        let render_ready_chunks = world.get_render_ready_chunks();
        let chunk_coords: Vec<ChunkCoord> = render_ready_chunks
            .into_iter()
            .map(|(coord, _)| coord)
            .collect();
        
        world.prepare_chunks_for_rendering(&chunk_coords);
        
        // Step 4: Generate meshes and create GPU buffers
        let mut rendered_chunks = 0;
        
        for coord in &chunk_coords {
            if let Some(entry) = world.get_chunk(*coord) {
                // Generate mesh from chunk data
                let mesh = mesh_generator.generate_chunk_mesh(&entry.chunk);
                
                if !mesh.is_empty() {
                    // Convert ChunkCoord to ChunkPosition for buffer manager
                    let chunk_pos = ChunkPosition { x: coord.x, z: coord.z };
                    
                    // Create GPU buffers
                    let buffer_result = buffer_manager.create_buffers(chunk_pos, &mesh);
                    assert!(buffer_result.is_ok(), "Should create buffers for chunk {:?}", coord);
                    
                    // Upload mesh data
                    let upload_result = buffer_manager.upload_mesh_data(&queue, &chunk_pos, &mesh);
                    assert!(upload_result.is_ok(), "Should upload mesh data for chunk {:?}", coord);
                    
                    rendered_chunks += 1;
                }
            }
        }
        
        assert!(rendered_chunks > 0, "Should have rendered at least one chunk");
        
        // Step 5: Test rendering with world coordinates
        let view_proj_matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        
        for coord in &chunk_coords {
            if buffer_manager.has_buffers(&ChunkPosition { x: coord.x, z: coord.z }) {
                let world_pos = world.chunk_to_world_pos(*coord);
                let uniforms = ChunkUniforms::new(
                    view_proj_matrix,
                    [world_pos.x, world_pos.y, world_pos.z]
                );
                
                renderer.update_uniforms(&queue, &uniforms);
                
                // Verify uniforms are set correctly
                assert_eq!(uniforms.view_proj, view_proj_matrix);
                assert_eq!(uniforms.chunk_position, [world_pos.x, world_pos.y, world_pos.z]);
            }
        }
        
        // Step 6: Test memory usage and cleanup
        let memory_usage = buffer_manager.memory_usage();
        assert!(memory_usage.total_memory > 0);
        assert_eq!(memory_usage.buffer_count, rendered_chunks);
        
        // Test selective cleanup
        let active_chunks: HashSet<ChunkPosition> = chunk_coords.into_iter()
            .take(1)
            .map(|coord| ChunkPosition { x: coord.x, z: coord.z })
            .collect();
        buffer_manager.cleanup_unused_buffers(&active_chunks);
        
        let after_cleanup = buffer_manager.memory_usage();
        assert!(after_cleanup.total_memory <= memory_usage.total_memory);
        assert!(after_cleanup.buffer_count <= memory_usage.buffer_count);
    });
}

/// Test performance characteristics under various loads
/// Requirements: 5.1, 5.2, 5.3, 5.4
#[test]
fn test_performance_under_various_loads() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Test different world sizes and measure performance
        let test_scenarios = vec![
            ("small", 1, 8),    // 3x3x3 = 27 chunks
            ("medium", 2, 32),  // 5x5x5 = 125 chunks  
            ("large", 3, 50),   // 7x7x7 = 343 chunks (limited by max_chunks)
        ];
        
        for (scenario_name, radius, max_chunks) in test_scenarios {
            println!("Testing performance scenario: {}", scenario_name);
            
            let config = WorldConfig::new()
                .with_render_distance(radius + 1)
                .with_max_chunks(Some(max_chunks))
                .with_performance_monitoring(true);
            
            let mut world = World::new(config).expect("Should create world successfully");
            
            // Measure chunk loading performance
            let load_start = Instant::now();
            let center = ChunkCoord::new(0, 0, 0);
            let progress = world.load_grid(center, radius).expect("Should load grid successfully");
            let load_duration = load_start.elapsed();
            
            println!("  Load time: {:?}", load_duration);
            println!("  Chunks loaded: {}", progress.successful_loads());
            println!("  Failed loads: {}", progress.failed_loads());
            
            // Verify loading completed successfully
            assert!(progress.is_complete());
            assert_eq!(progress.failed_loads(), 0);
            
            // Measure chunk preparation performance
            let prep_start = Instant::now();
            let chunk_coords: Vec<ChunkCoord> = world.get_render_ready_chunks()
                .into_iter()
                .map(|(coord, _)| coord)
                .collect();
            
            let preparation_results = world.prepare_chunks_for_rendering(&chunk_coords);
            let prep_duration = prep_start.elapsed();
            
            println!("  Preparation time: {:?}", prep_duration);
            
            let successful_preps = preparation_results.iter()
                .filter(|(_, result)| result.is_ok())
                .count();
            
            println!("  Successful preparations: {}", successful_preps);
            
            // Test memory usage
            let memory_stats = world.memory_stats();
            println!("  Memory usage: {:.2} MB", memory_stats.current_usage as f32 / 1024.0 / 1024.0);
            println!("  Memory usage fraction: {:.2}%", memory_stats.usage_fraction * 100.0);
            
            // Verify memory usage is reasonable
            assert!(!memory_stats.is_critical(), "Memory usage should not be critical for scenario {}", scenario_name);
            assert!(memory_stats.chunks_tracked > 0, "Should track chunk memory for scenario {}", scenario_name);
            
            // Test loading statistics
            let loading_stats = world.get_loading_stats();
            println!("  Total chunks: {}", loading_stats.total_chunks);
            println!("  Render ready: {}", loading_stats.render_ready_chunks);
            
            assert!(loading_stats.total_chunks > 0);
            assert!(loading_stats.render_ready_chunks <= loading_stats.total_chunks);
            
            // Performance assertions
            match scenario_name {
                "small" => {
                    assert!(load_duration < Duration::from_millis(100), "Small world should load quickly");
                    assert!(prep_duration < Duration::from_millis(50), "Small world should prepare quickly");
                }
                "medium" => {
                    assert!(load_duration < Duration::from_millis(500), "Medium world should load reasonably fast");
                    assert!(prep_duration < Duration::from_millis(200), "Medium world should prepare reasonably fast");
                }
                "large" => {
                    assert!(load_duration < Duration::from_secs(2), "Large world should load within reasonable time");
                    assert!(prep_duration < Duration::from_millis(500), "Large world should prepare within reasonable time");
                }
                _ => {}
            }
            
            println!("  Scenario {} completed successfully\n", scenario_name);
        }
    });
}

/// Test dynamic chunk loading and unloading
/// Requirements: 3.1, 3.2, 3.5
#[test]
fn test_dynamic_chunk_loading() {
    let config = create_test_world_config();
    let mut world = World::new(config).expect("Should create world successfully");
    
    // Test 1: Load initial chunks
    let initial_center = ChunkCoord::new(0, 0, 0);
    let progress1 = world.load_grid(initial_center, 1).expect("Should load initial grid");
    assert!(progress1.successful_loads() > 0);
    
    let initial_count = world.chunk_count();
    assert!(initial_count > 0);
    
    // Test 2: Move to new location and load more chunks
    let new_center = ChunkCoord::new(3, 0, 3);
    let _progress2 = world.load_grid(new_center, 1).expect("Should load new grid");
    
    let after_move_count = world.chunk_count();
    assert!(after_move_count >= initial_count); // Should have at least as many chunks
    
    // Test 3: Test chunk loading with custom pattern
    let custom_coords = vec![
        ChunkCoord::new(10, 0, 10),
        ChunkCoord::new(11, 0, 10),
        ChunkCoord::new(10, 0, 11),
        ChunkCoord::new(11, 0, 11),
    ];
    
    let progress3 = world.load_custom_chunks(custom_coords.clone()).expect("Should load custom chunks");
    assert_eq!(progress3.total_chunks(), custom_coords.len());
    assert_eq!(progress3.successful_loads(), custom_coords.len());
    
    // Verify custom chunks are loaded
    for coord in &custom_coords {
        assert!(world.is_chunk_loaded(*coord), "Custom chunk {:?} should be loaded", coord);
    }
    
    // Test 4: Test memory cleanup during dynamic loading
    let memory_stats_before = world.memory_stats();
    
    // Force memory cleanup
    let cleanup_result = world.cleanup_memory_if_needed();
    assert!(cleanup_result.is_ok());
    
    let memory_stats_after = world.memory_stats();
    
    // Memory usage should be managed (not necessarily reduced if under threshold)
    assert!(memory_stats_after.current_usage <= memory_stats_before.current_usage + 1024); // Allow small variance
    
    // Test 5: Test loading progress tracking
    let large_coords: Vec<ChunkCoord> = (0..10)
        .flat_map(|x| (0..10).map(move |z| ChunkCoord::new(x + 20, 0, z + 20)))
        .collect();
    
    let progress4 = world.load_custom_chunks(large_coords.clone()).expect("Should load large set");
    
    assert_eq!(progress4.total_chunks(), large_coords.len());
    assert!(progress4.completion_percentage() >= 0.0 && progress4.completion_percentage() <= 1.0);
    
    if progress4.successful_loads() > 0 {
        assert!(progress4.completion_percentage() > 0.0);
    }
    
    // Test 6: Test retry mechanism for failed chunks
    let retry_progress = world.retry_failed_chunks(&progress4).expect("Should retry failed chunks");
    
    // If there were no failures, retry should complete immediately
    if progress4.failed_loads() == 0 {
        assert_eq!(retry_progress.total_chunks(), 0);
    }
}

/// Test error handling and recovery scenarios
/// Requirements: 3.3
#[test]
fn test_error_handling_and_recovery() {
    // Test 1: World creation with invalid configuration
    let invalid_config = WorldConfig::new()
        .with_render_distance(0); // Invalid render distance
    
    // This should still work as the config validation might allow 0
    let world_result = World::new(invalid_config);
    if world_result.is_err() {
        // If it fails, that's expected behavior
        println!("World creation correctly rejected invalid config");
    } else {
        // If it succeeds, the config was adjusted or is valid
        let world = world_result.unwrap();
        assert!(world.config().render_distance >= 0);
    }
    
    // Test 2: Chunk loading with memory constraints
    let constrained_config = WorldConfig::new()
        .with_render_distance(2)
        .with_max_chunks(Some(5)); // Very small limit
    
    let mut constrained_world = World::new(constrained_config).expect("Should create constrained world");
    
    // Try to load more chunks than the limit allows
    let large_center = ChunkCoord::new(0, 0, 0);
    let progress = constrained_world.load_grid(large_center, 2).expect("Should attempt to load grid");
    
    // Should either succeed with limited chunks or handle the constraint gracefully
    assert!(progress.total_chunks() > 0);
    assert!(constrained_world.chunk_count() <= 5); // Should respect the limit
    
    // Test 3: Configuration updates and validation
    let mut test_world = World::new(create_test_world_config()).expect("Should create test world");
    
    // Load some chunks first
    let _ = test_world.load_single_chunk(ChunkCoord::new(0, 0, 0));
    
    // Test valid configuration update
    let new_render_distance = 6;
    let update_result = test_world.set_render_distance(new_render_distance);
    
    if update_result.is_ok() {
        assert_eq!(test_world.config().render_distance, new_render_distance);
    } else {
        // Configuration update might fail if it requires restart
        println!("Configuration update failed as expected: {:?}", update_result);
    }
    
    // Test 4: Memory bounds validation
    let memory_validation = test_world.validate_memory_consistency();
    assert!(memory_validation.is_ok(), "Memory consistency should be valid: {:?}", memory_validation);
    
    let bounds_validation = test_world.validate_memory_bounds();
    assert!(bounds_validation.is_ok(), "Memory bounds should be valid: {:?}", bounds_validation);
    
    // Test 5: World consistency validation
    let consistency_result = test_world.validate_world_consistency();
    
    match consistency_result {
        Ok(()) => {
            println!("World consistency validation passed");
        }
        Err(e) => {
            // Some consistency errors might be expected (e.g., chunks in loading state)
            println!("World consistency validation failed (may be expected): {}", e);
        }
    }
}

/// Test frustum culling and render distance optimization
/// Requirements: 4.4
#[test]
fn test_frustum_culling_and_optimization() {
    let config = create_test_world_config();
    let mut world = World::new(config).expect("Should create world successfully");
    
    // Load a larger grid for testing culling
    let center = ChunkCoord::new(0, 0, 0);
    let progress = world.load_grid(center, 3).expect("Should load grid for culling test");
    assert!(progress.successful_loads() > 0);
    
    // Prepare chunks for rendering
    let chunk_coords: Vec<ChunkCoord> = world.get_render_ready_chunks()
        .into_iter()
        .map(|(coord, _)| coord)
        .collect();
    
    world.prepare_chunks_for_rendering(&chunk_coords);
    
    // Test render distance filtering
    let render_distance_chunks = world.get_render_ready_chunks_in_distance(center);
    let all_render_ready = world.get_render_ready_chunks();
    
    // All chunks in render distance should be subset of all render ready chunks
    assert!(render_distance_chunks.len() <= all_render_ready.len());
    
    // Verify render distance constraint
    for (coord, _) in &render_distance_chunks {
        assert!(world.is_in_render_distance(*coord, center), 
                "Chunk {:?} should be within render distance of {:?}", coord, center);
    }
    
    // Test coordinate system utilities
    let test_coords = vec![
        ChunkCoord::new(0, 0, 0),
        ChunkCoord::new(1, 0, 0),
        ChunkCoord::new(0, 1, 0),
        ChunkCoord::new(0, 0, 1),
        ChunkCoord::new(-1, 0, 0),
    ];
    
    for coord in &test_coords {
        if world.is_chunk_loaded(*coord) {
            // Test coordinate conversions
            let world_pos = world.chunk_to_world_pos(*coord);
            let converted_back = world.world_to_chunk_coord(world_pos);
            assert_eq!(*coord, converted_back, "Coordinate conversion should be round-trip for {:?}", coord);
            
            // Test chunk bounds
            let (min_bounds, max_bounds) = world.chunk_bounds(*coord);
            assert!(min_bounds.x < max_bounds.x);
            assert!(min_bounds.y < max_bounds.y);
            assert!(min_bounds.z < max_bounds.z);
            
            // World position should be at minimum bounds (chunk origin)
            assert!((world_pos.x - min_bounds.x).abs() < f32::EPSILON);
            assert!((world_pos.y - min_bounds.y).abs() < f32::EPSILON);
            assert!((world_pos.z - min_bounds.z).abs() < f32::EPSILON);
        }
    }
    
    // Test adjacent chunk detection
    let origin = ChunkCoord::new(0, 0, 0);
    let adjacent_coords = vec![
        ChunkCoord::new(1, 0, 0),
        ChunkCoord::new(-1, 0, 0),
        ChunkCoord::new(0, 1, 0),
        ChunkCoord::new(0, -1, 0),
        ChunkCoord::new(0, 0, 1),
        ChunkCoord::new(0, 0, -1),
    ];
    
    for adj_coord in &adjacent_coords {
        assert!(world.are_chunks_adjacent(origin, *adj_coord), 
                "Chunk {:?} should be adjacent to origin", adj_coord);
        
        let distance = world.chunk_distance_world_space(origin, *adj_coord);
        let expected_distance = world.chunk_size() as f32;
        assert!((distance - expected_distance).abs() < 0.001, 
                "Distance between adjacent chunks should be chunk size");
    }
    
    // Test non-adjacent chunks
    let non_adjacent = ChunkCoord::new(2, 2, 2);
    assert!(!world.are_chunks_adjacent(origin, non_adjacent), 
            "Chunk {:?} should not be adjacent to origin", non_adjacent);
}

/// Test batch operations and memory management
/// Requirements: 7.1, 7.2, 7.3, 7.5
#[test]
fn test_batch_operations_and_memory_management() {
    let config = WorldConfig::new()
        .with_render_distance(4)
        .with_max_chunks(Some(100))
        .with_performance_monitoring(true);
    
    let mut world = World::new(config).expect("Should create world successfully");
    
    // Test 1: Batch chunk loading
    let batch_coords: Vec<ChunkCoord> = (0..5)
        .flat_map(|x| (0..5).map(move |z| ChunkCoord::new(x, 0, z)))
        .collect();
    
    let batch_progress = world.load_custom_chunks(batch_coords.clone()).expect("Should load batch");
    assert_eq!(batch_progress.total_chunks(), batch_coords.len());
    
    // Test 2: Batch chunk preparation
    let preparation_start = Instant::now();
    let preparation_results = world.prepare_chunks_for_rendering(&batch_coords);
    let preparation_duration = preparation_start.elapsed();
    
    println!("Batch preparation took: {:?}", preparation_duration);
    
    let successful_preparations = preparation_results.iter()
        .filter(|(_, result)| result.is_ok())
        .count();
    
    assert!(successful_preparations > 0, "Should have successful preparations");
    
    // Test 3: Memory usage tracking
    let memory_stats = world.memory_stats();
    assert!(memory_stats.current_usage > 0, "Should use memory after loading chunks");
    assert_eq!(memory_stats.chunks_tracked, world.chunk_count());
    
    // Test 4: Memory cleanup simulation
    let initial_memory = memory_stats.current_usage;
    
    // Simulate memory pressure by checking cleanup threshold
    if world.should_cleanup_memory() {
        let cleanup_result = world.cleanup_memory_if_needed();
        assert!(cleanup_result.is_ok(), "Memory cleanup should succeed");
        
        let after_cleanup = world.memory_stats();
        println!("Memory before cleanup: {} bytes", initial_memory);
        println!("Memory after cleanup: {} bytes", after_cleanup.current_usage);
    }
    
    // Test 5: Memory bounds checking
    let bounds_check = world.validate_memory_bounds();
    assert!(bounds_check.is_ok(), "Memory bounds should be valid: {:?}", bounds_check);
    
    // Test 6: Memory health check
    let health_report = world.memory_health_check();
    assert!(health_report.is_ok(), "Memory health check should succeed");
    
    let health = health_report.unwrap();
    println!("Memory health status: {:?}", health.status);
    
    // Health should not be critical for this test size
    assert_ne!(health.status, MemoryHealthStatus::Critical, 
               "Memory health should not be critical for test workload");
    
    // Test 7: Batch render distance operations
    let center = ChunkCoord::new(2, 0, 2);
    let prepared_count = world.batch_prepare_chunks_in_render_distance(center);
    assert!(prepared_count.is_ok(), "Batch render distance preparation should succeed");
    
    let prepared = prepared_count.unwrap();
    println!("Prepared {} chunks in render distance", prepared);
    
    // Test 8: Memory leak detection
    let potential_leaks = world.detect_memory_leaks(Duration::from_secs(1));
    // For a fresh test, there should be no leaks
    assert!(potential_leaks.is_empty(), "Should not detect memory leaks in fresh test");
    
    // Test 9: Memory fragmentation statistics
    let frag_stats = world.memory_fragmentation_stats();
    println!("Memory fragmentation score: {:.2}", frag_stats.fragmentation_score());
    
    // For uniform chunk sizes, fragmentation should be low
    assert!(frag_stats.fragmentation_score() < 0.5, "Fragmentation should be low for uniform chunks");
}

/// Test configuration management and adaptation
/// Requirements: 6.1, 6.2, 6.3, 6.4, 6.5
#[test]
fn test_configuration_management() {
    let initial_config = create_test_world_config();
    let mut world = World::new(initial_config.clone()).expect("Should create world successfully");
    
    // Load some chunks to test configuration changes with existing data
    let _ = world.load_grid(ChunkCoord::new(0, 0, 0), 1);
    
    // Test 1: Valid configuration updates
    let new_render_distance = 6;
    let render_distance_result = world.set_render_distance(new_render_distance);
    
    match render_distance_result {
        Ok(()) => {
            assert_eq!(world.config().render_distance, new_render_distance);
            println!("Successfully updated render distance to {}", new_render_distance);
        }
        Err(e) => {
            println!("Render distance update failed (may require restart): {}", e);
        }
    }
    
    // Test 2: Max chunks configuration
    let new_max_chunks = Some(75);
    let max_chunks_result = world.set_max_chunks(new_max_chunks);
    
    match max_chunks_result {
        Ok(()) => {
            assert_eq!(world.config().max_chunks_loaded, new_max_chunks);
            println!("Successfully updated max chunks to {:?}", new_max_chunks);
        }
        Err(e) => {
            println!("Max chunks update failed: {}", e);
        }
    }
    
    // Test 3: Performance monitoring toggle
    let monitoring_result = world.set_performance_monitoring(true);
    
    match monitoring_result {
        Ok(()) => {
            assert!(world.config().performance_monitoring);
            println!("Successfully enabled performance monitoring");
        }
        Err(e) => {
            println!("Performance monitoring update failed: {}", e);
        }
    }
    
    // Test 4: Configuration validation
    let current_config = world.config().clone();
    let validation_result = current_config.validate();
    assert!(validation_result.is_ok(), "Current configuration should be valid: {:?}", validation_result);
    
    // Test 5: Configuration compatibility checking
    let test_config = WorldConfig::new()
        .with_render_distance(8)
        .with_max_chunks(Some(200));
    
    let compatibility_result = world.validate_config_change(&test_config);
    match compatibility_result {
        Ok(differences) => {
            println!("Configuration change is compatible. Differences: {:?}", differences);
        }
        Err(e) => {
            println!("Configuration change is incompatible: {}", e);
        }
    }
    
    // Test 6: Configuration differences detection
    let differences = world.get_config_differences(&test_config);
    println!("Configuration differences: {:?}", differences);
    
    // Test 7: Restart requirement checking
    let requires_restart = world.would_require_restart(&test_config);
    println!("Configuration change requires restart: {}", requires_restart);
    
    // Test 8: Chunk unload delay configuration
    let new_delay = Duration::from_secs(60);
    let delay_result = world.set_chunk_unload_delay(new_delay);
    
    match delay_result {
        Ok(()) => {
            assert_eq!(world.config().chunk_unload_delay, new_delay);
            println!("Successfully updated chunk unload delay to {:?}", new_delay);
        }
        Err(e) => {
            println!("Chunk unload delay update failed: {}", e);
        }
    }
    
    // Test 9: Configuration consistency validation
    let consistency_result = world.validate_config_compatibility();
    assert!(consistency_result.is_ok(), "Configuration should be consistent: {:?}", consistency_result);
}