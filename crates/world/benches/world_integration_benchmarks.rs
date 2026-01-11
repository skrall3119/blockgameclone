//! Performance benchmarks for world integration system
//!
//! These benchmarks measure the performance of multi-chunk operations, memory management,
//! world coordinate systems, and configuration management to ensure optimal performance
//! characteristics at world scale.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use world::{
    Chunk, ChunkDimensions, ChunkPosition, BlockID, DEFAULT_DIMENSIONS,
    world::{
        World, WorldConfig, ChunkCoord,
        memory::MemoryManager,
        performance::{PerformanceMonitor, MonitorConfig, MemorySample},
        coordinate::CoordinateSystem,
    }
};

/// Create a test world with specified configuration
fn create_test_world(config: WorldConfig) -> World {
    World::new(config).expect("Failed to create test world")
}

/// Create test chunks in a grid pattern
fn create_chunk_grid(size: usize, dimensions: ChunkDimensions) -> Vec<(ChunkCoord, Chunk)> {
    let mut chunks = Vec::new();
    
    for x in 0..size {
        for z in 0..size {
            let coord = ChunkCoord::new(x as i32, 0, z as i32);
            let position = ChunkPosition { x: x as i32, z: z as i32 };
            let mut chunk = Chunk::new(position, dimensions);
            
            // Fill with a pattern for realistic testing
            let fill_percentage = 0.3 + (x + z) as f32 * 0.1 % 0.4;
            let total_blocks = dimensions.width * dimensions.height * dimensions.depth;
            let blocks_to_fill = (total_blocks as f32 * fill_percentage) as usize;
            
            for i in 0..blocks_to_fill {
                let block_x = i % dimensions.width;
                let block_y = (i / dimensions.width) % dimensions.height;
                let block_z = i / (dimensions.width * dimensions.height);
                
                let block_type = match (block_x + block_y + block_z) % 4 {
                    0 => BlockID::Stone,
                    1 => BlockID::Dirt,
                    2 => BlockID::Grass,
                    _ => BlockID::Air,
                };
                
                if block_type != BlockID::Air {
                    chunk.set_block(block_x, block_y, block_z, block_type).unwrap();
                }
            }
            
            chunks.push((coord, chunk));
        }
    }
    
    chunks
}

/// Benchmark multi-chunk loading performance at different scales
fn bench_multi_chunk_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_chunk_loading");
    
    let world_sizes = vec![
        ("3x3", 3),
        ("5x5", 5),
    ];
    
    for (size_name, grid_size) in world_sizes {
        let chunk_count = grid_size * grid_size;
        let chunks = create_chunk_grid(grid_size, DEFAULT_DIMENSIONS);
        
        group.throughput(Throughput::Elements(chunk_count as u64));
        
        // Benchmark world creation with chunks
        group.bench_with_input(
            BenchmarkId::new("world_creation", size_name),
            &chunks,
            |b, chunks| {
                b.iter(|| {
                    let config = WorldConfig::default();
                    let world = create_test_world(config);
                    
                    // Just create the world and chunks for now
                    black_box((world, chunks));
                });
            },
        );
        
        // Benchmark chunk coordinate calculations
        group.bench_with_input(
            BenchmarkId::new("coordinate_calculations", size_name),
            &chunks,
            |b, chunks| {
                b.iter(|| {
                    let mut total_distance = 0.0;
                    let center = ChunkCoord::new(0, 0, 0);
                    
                    for (coord, _) in black_box(chunks) {
                        let distance = coord.euclidean_distance(&center);
                        total_distance += distance;
                    }
                    
                    black_box(total_distance);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark memory management and cleanup performance
fn bench_memory_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_management");
    
    let chunks = create_chunk_grid(5, DEFAULT_DIMENSIONS);
    
    // Benchmark memory manager operations
    group.bench_function("memory_manager_creation", |b| {
        b.iter(|| {
            let memory_manager = MemoryManager::new(1024 * 1024 * 100); // 100MB
            black_box(memory_manager);
        });
    });
    
    // Benchmark chunk memory estimation
    group.bench_function("chunk_memory_estimation", |b| {
        let memory_manager = MemoryManager::new(1024 * 1024 * 100);
        
        b.iter(|| {
            for (_, chunk) in black_box(&chunks) {
                let estimated_size = std::mem::size_of_val(chunk) + 
                    chunk.dimensions().width * chunk.dimensions().height * chunk.dimensions().depth * 
                    std::mem::size_of::<BlockID>();
                let can_load = memory_manager.can_load_chunk(estimated_size);
                black_box(can_load);
            }
        });
    });
    
    group.finish();
}

/// Benchmark world coordinate system performance
fn bench_coordinate_system(c: &mut Criterion) {
    let mut group = c.benchmark_group("coordinate_system");
    
    let coordinate_system = CoordinateSystem::new(16); // 16x16 chunk size
    
    // Generate test coordinates
    use rand::prelude::*;
    let mut rng = StdRng::seed_from_u64(42);
    let world_positions: Vec<glam::Vec3> = (0..1000)
        .map(|_| {
            glam::Vec3::new(
                rng.gen_range(-1000.0..1000.0),
                rng.gen_range(0.0..256.0),
                rng.gen_range(-1000.0..1000.0),
            )
        })
        .collect();
    
    let chunk_coords: Vec<ChunkCoord> = (0..1000)
        .map(|_| ChunkCoord::new(
            rng.gen_range(-50..50),
            0,
            rng.gen_range(-50..50),
        ))
        .collect();
    
    group.throughput(Throughput::Elements(1000));
    
    // Benchmark world to chunk coordinate conversion
    group.bench_function("world_to_chunk_coords", |b| {
        b.iter(|| {
            let mut results = Vec::with_capacity(world_positions.len());
            for &pos in black_box(&world_positions) {
                let chunk_coord = coordinate_system.world_to_chunk_coord(pos);
                results.push(chunk_coord);
            }
            black_box(results);
        });
    });
    
    // Benchmark chunk to world coordinate conversion
    group.bench_function("chunk_to_world_coords", |b| {
        b.iter(|| {
            let mut results = Vec::with_capacity(chunk_coords.len());
            for &coord in black_box(&chunk_coords) {
                let world_pos = coordinate_system.chunk_to_world_pos(coord);
                results.push(world_pos);
            }
            black_box(results);
        });
    });
    
    // Benchmark distance calculations
    group.bench_function("chunk_distance_calculations", |b| {
        let center = ChunkCoord::new(0, 0, 0);
        
        b.iter(|| {
            let mut total_distance = 0.0;
            for &coord in black_box(&chunk_coords) {
                let distance = center.euclidean_distance(&coord);
                total_distance += distance;
            }
            black_box(total_distance);
        });
    });
    
    // Benchmark Manhattan distance calculations
    group.bench_function("manhattan_distance_calculations", |b| {
        let center = ChunkCoord::new(0, 0, 0);
        
        b.iter(|| {
            let mut total_distance = 0u32;
            for &coord in black_box(&chunk_coords) {
                let distance = center.manhattan_distance(&coord);
                total_distance += distance;
            }
            black_box(total_distance);
        });
    });
    
    group.finish();
}

/// Benchmark configuration management performance
fn bench_configuration_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("configuration_management");
    
    // Benchmark configuration creation and validation
    group.bench_function("config_creation", |b| {
        b.iter(|| {
            let config = WorldConfig::default();
            black_box(config);
        });
    });
    
    group.bench_function("config_validation", |b| {
        let config = WorldConfig::default();
        
        b.iter(|| {
            let is_valid = config.validate();
            let _ = black_box(is_valid);
        });
    });
    
    // Benchmark configuration cloning (for thread safety)
    group.bench_function("config_cloning", |b| {
        let config = WorldConfig::default();
        
        b.iter(|| {
            let cloned_config = config.clone();
            black_box(cloned_config);
        });
    });
    
    // Benchmark configuration updates
    group.bench_function("config_updates", |b| {
        b.iter(|| {
            let mut config = WorldConfig::default();
            config = config.with_render_distance(16);
            black_box(config);
        });
    });
    
    group.finish();
}

/// Benchmark performance monitoring system overhead
fn bench_performance_monitoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_monitoring");
    
    // Benchmark performance monitor creation
    group.bench_function("monitor_creation", |b| {
        b.iter(|| {
            let monitor = PerformanceMonitor::new(MonitorConfig::default());
            black_box(monitor);
        });
    });
    
    // Benchmark metric recording
    group.bench_function("metric_recording", |b| {
        let mut monitor = PerformanceMonitor::new(MonitorConfig::default());
        
        b.iter(|| {
            monitor.record_chunk_generation(black_box(std::time::Duration::from_millis(10)));
            monitor.record_chunk_meshing(black_box(std::time::Duration::from_millis(5)));
            
            let memory_sample = MemorySample::new(25, 1024 * 1024, 512 * 1024);
            monitor.record_memory_sample(black_box(memory_sample));
        });
    });
    
    group.finish();
}

/// Benchmark chunk coordinate operations
fn bench_chunk_coordinate_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunk_coordinate_operations");
    
    // Generate test coordinates
    use rand::prelude::*;
    let mut rng = StdRng::seed_from_u64(42);
    let coords: Vec<ChunkCoord> = (0..1000)
        .map(|_| ChunkCoord::new(
            rng.gen_range(-100..100),
            rng.gen_range(-10..10),
            rng.gen_range(-100..100),
        ))
        .collect();
    
    group.throughput(Throughput::Elements(1000));
    
    // Benchmark coordinate creation
    group.bench_function("coordinate_creation", |b| {
        b.iter(|| {
            let mut results = Vec::with_capacity(1000);
            for i in 0..1000 {
                let coord = ChunkCoord::new(i % 200 - 100, 0, i % 200 - 100);
                results.push(coord);
            }
            black_box(results);
        });
    });
    
    // Benchmark adjacent coordinate calculation
    group.bench_function("adjacent_coordinates", |b| {
        b.iter(|| {
            let mut total_adjacent = 0;
            for &coord in black_box(&coords) {
                let adjacent = coord.adjacent();
                total_adjacent += adjacent.len();
            }
            black_box(total_adjacent);
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_multi_chunk_loading,
    bench_memory_management,
    bench_coordinate_system,
    bench_configuration_management,
    bench_performance_monitoring,
    bench_chunk_coordinate_operations
);

criterion_main!(benches);