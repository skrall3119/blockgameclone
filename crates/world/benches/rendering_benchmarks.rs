//! Performance benchmarks for chunk rendering system
//!
//! These benchmarks measure the performance of mesh generation, GPU buffer operations,
//! and rendering pipeline components to ensure optimal performance characteristics.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::sync::Arc;
use world::{
    Chunk, ChunkDimensions, ChunkPosition, BlockID,
    rendering::{
        MeshGenerator, BufferManager, SingleChunkDemo,
        ChunkMesh, ChunkVertex
    }
};

/// Create a test chunk with the specified dimensions and fill pattern
fn create_test_chunk(dimensions: ChunkDimensions, fill_percentage: f32) -> Chunk {
    let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
    
    let total_blocks = dimensions.width * dimensions.height * dimensions.depth;
    let blocks_to_fill = (total_blocks as f32 * fill_percentage) as usize;
    let mut filled = 0;
    
    for x in 0..dimensions.width {
        for y in 0..dimensions.height {
            for z in 0..dimensions.depth {
                if filled < blocks_to_fill {
                    let block_type = match (x + y + z) % 4 {
                        0 => BlockID::Stone,
                        1 => BlockID::Dirt,
                        2 => BlockID::Grass,
                        _ => BlockID::Stone,
                    };
                    chunk.set_block(x, y, z, block_type).unwrap();
                    filled += 1;
                } else {
                    break;
                }
            }
            if filled >= blocks_to_fill { break; }
        }
        if filled >= blocks_to_fill { break; }
    }
    
    chunk
}

/// Create a mock wgpu device for benchmarking
async fn create_benchmark_device() -> (Arc<wgpu::Device>, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
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

/// Benchmark mesh generation performance for different chunk sizes
fn bench_mesh_generation_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("mesh_generation_by_size");
    group.sample_size(10); // Reduce sample size for faster benchmarks
    
    let mesh_generator = MeshGenerator::new();
    let chunk_sizes = vec![
        (4, 4, 4),
        (8, 8, 8),
        (16, 16, 16),
        // Removed larger sizes for faster benchmarking
    ];
    
    for (width, height, depth) in chunk_sizes {
        let dimensions = ChunkDimensions { width, height, depth };
        let total_blocks = width * height * depth;
        
        // Test with 50% fill to have realistic face culling
        let chunk = create_test_chunk(dimensions, 0.5);
        
        group.throughput(Throughput::Elements(total_blocks as u64));
        group.bench_with_input(
            BenchmarkId::new("generate_chunk_mesh", format!("{}x{}x{}", width, height, depth)),
            &chunk,
            |b, chunk| {
                b.iter(|| {
                    let mesh = mesh_generator.generate_chunk_mesh(black_box(chunk));
                    black_box(mesh);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark mesh generation performance for different fill percentages
fn bench_mesh_generation_by_fill(c: &mut Criterion) {
    let mut group = c.benchmark_group("mesh_generation_by_fill");
    group.sample_size(10); // Reduce sample size for faster benchmarks
    
    let mesh_generator = MeshGenerator::new();
    let dimensions = ChunkDimensions { width: 8, height: 8, depth: 8 }; // Smaller for faster benchmarks
    let fill_percentages = vec![0.25, 0.5, 0.75]; // Fewer test cases
    
    for fill_percentage in fill_percentages {
        let chunk = create_test_chunk(dimensions, fill_percentage);
        let filled_blocks = (dimensions.width * dimensions.height * dimensions.depth) as f32 * fill_percentage;
        
        group.throughput(Throughput::Elements(filled_blocks as u64));
        group.bench_with_input(
            BenchmarkId::new("generate_chunk_mesh", format!("{}%_fill", (fill_percentage * 100.0) as u32)),
            &chunk,
            |b, chunk| {
                b.iter(|| {
                    let mesh = mesh_generator.generate_chunk_mesh(black_box(chunk));
                    black_box(mesh);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark face culling effectiveness
fn bench_face_culling_effectiveness(c: &mut Criterion) {
    let mut group = c.benchmark_group("face_culling_effectiveness");
    
    let mesh_generator = MeshGenerator::new();
    let dimensions = ChunkDimensions { width: 16, height: 16, depth: 16 };
    
    // Create different scenarios for face culling
    let scenarios = vec![
        ("isolated_blocks", 0.1), // Sparse blocks (minimal culling)
        ("half_filled", 0.5),     // Mixed pattern (moderate culling)
        ("mostly_filled", 0.9),   // Dense blocks (heavy culling)
        ("completely_filled", 1.0), // Solid chunk (maximum culling)
    ];
    
    for (scenario_name, fill_percentage) in scenarios {
        let chunk = create_test_chunk(dimensions, fill_percentage);
        
        group.bench_function(scenario_name, |b| {
            b.iter(|| {
                let mesh = mesh_generator.generate_chunk_mesh(black_box(&chunk));
                // Measure both generation time and resulting mesh size
                black_box((mesh.vertices.len(), mesh.indices.len()));
            });
        });
    }
    
    group.finish();
}

/// Benchmark GPU buffer operations
fn bench_gpu_buffer_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (device, queue) = rt.block_on(create_benchmark_device());
    
    let mut group = c.benchmark_group("gpu_buffer_operations");
    
    let mesh_generator = MeshGenerator::new();
    let dimensions = ChunkDimensions { width: 16, height: 16, depth: 16 };
    let chunk = create_test_chunk(dimensions, 0.5);
    let mesh = mesh_generator.generate_chunk_mesh(&chunk);
    
    // Benchmark buffer creation
    group.bench_function("buffer_creation", |b| {
        b.iter(|| {
            let mut buffer_manager = BufferManager::new(device.clone());
            let chunk_pos = ChunkPosition { x: 0, z: 0 };
            let result = buffer_manager.create_buffers(chunk_pos, black_box(&mesh));
            black_box(result);
        });
    });
    
    // Benchmark buffer upload
    let mut buffer_manager = BufferManager::new(device.clone());
    let chunk_pos = ChunkPosition { x: 0, z: 0 };
    buffer_manager.create_buffers(chunk_pos, &mesh).unwrap();
    
    group.bench_function("buffer_upload", |b| {
        b.iter(|| {
            let result = buffer_manager.upload_mesh_data(black_box(&queue), black_box(&chunk_pos), black_box(&mesh));
            black_box(result);
        });
    });
    
    // Benchmark buffer update (including recreation if needed)
    group.bench_function("buffer_update", |b| {
        b.iter(|| {
            let result = buffer_manager.update_chunk_buffers(chunk_pos, black_box(&mesh), black_box(&queue));
            black_box(result);
        });
    });
    
    group.finish();
}

/// Benchmark memory usage and allocation patterns
fn bench_memory_usage_patterns(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (device, _queue) = rt.block_on(create_benchmark_device());
    
    let mut group = c.benchmark_group("memory_usage_patterns");
    
    let mesh_generator = MeshGenerator::new();
    let dimensions = ChunkDimensions { width: 8, height: 8, depth: 8 };
    
    // Benchmark batch buffer creation
    let chunk_count = 10;
    let mut chunks_and_meshes = Vec::new();
    
    for i in 0..chunk_count {
        let chunk_pos = ChunkPosition { x: i, z: 0 };
        let chunk = create_test_chunk(dimensions, 0.5);
        let mesh = mesh_generator.generate_chunk_mesh(&chunk);
        chunks_and_meshes.push((chunk_pos, mesh));
    }
    
    group.throughput(Throughput::Elements(chunk_count as u64));
    group.bench_function("batch_buffer_creation", |b| {
        b.iter(|| {
            let mut buffer_manager = BufferManager::new(device.clone());
            for (chunk_pos, mesh) in black_box(&chunks_and_meshes) {
                let result = buffer_manager.create_buffers(*chunk_pos, mesh);
                black_box(result);
            }
            let stats = buffer_manager.memory_usage();
            black_box(stats);
        });
    });
    
    // Benchmark memory cleanup
    let mut buffer_manager = BufferManager::new(device.clone());
    for (chunk_pos, mesh) in &chunks_and_meshes {
        buffer_manager.create_buffers(*chunk_pos, mesh).unwrap();
    }
    
    group.bench_function("memory_cleanup", |b| {
        b.iter(|| {
            let active_chunks = std::collections::HashSet::new(); // Empty set = cleanup all
            buffer_manager.cleanup_unused_buffers(black_box(&active_chunks));
            let stats = buffer_manager.memory_usage();
            black_box(stats);
            
            // Recreate buffers for next iteration
            for (chunk_pos, mesh) in &chunks_and_meshes {
                buffer_manager.create_buffers(*chunk_pos, mesh).unwrap();
            }
        });
    });
    
    group.finish();
}

/// Benchmark SingleChunkDemo performance
fn bench_single_chunk_demo_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_chunk_demo_performance");
    
    // Benchmark different demo configurations
    let configurations: Vec<(&str, Box<dyn Fn() -> SingleChunkDemo>)> = vec![
        ("empty", Box::new(|| SingleChunkDemo::empty())),
        ("filled_stone", Box::new(|| SingleChunkDemo::filled(BlockID::Stone))),
        ("mixed_pattern", Box::new(|| SingleChunkDemo::mixed_pattern())),
        ("variety", Box::new(|| SingleChunkDemo::new())),
    ];
    
    for (config_name, create_demo) in &configurations {
        group.bench_function(format!("create_{}", config_name), |b| {
            b.iter(|| {
                let demo = create_demo();
                black_box(demo);
            });
        });
        
        let demo = create_demo();
        group.bench_function(format!("generate_mesh_{}", config_name), |b| {
            b.iter(|| {
                let mesh = demo.generate_mesh();
                black_box(mesh);
            });
        });
        
        group.bench_function(format!("get_statistics_{}", config_name), |b| {
            b.iter(|| {
                let stats = demo.get_statistics();
                black_box(stats);
            });
        });
    }
    
    group.finish();
}

/// Benchmark cube mesh generation (basic building block)
fn bench_cube_mesh_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("cube_mesh_generation");
    
    let mesh_generator = MeshGenerator::new();
    
    // Benchmark single cube generation
    group.bench_function("single_cube", |b| {
        b.iter(|| {
            let mesh = mesh_generator.generate_cube_mesh(black_box([0.0, 0.0, 0.0]));
            black_box(mesh);
        });
    });
    
    // Benchmark cube generation with face culling
    let face_culling_scenarios = vec![
        ("no_faces", 0b000000),
        ("one_face", 0b000001),
        ("three_faces", 0b000111),
        ("all_faces", 0b111111),
    ];
    
    for (scenario_name, visible_faces) in face_culling_scenarios {
        group.bench_function(format!("cube_with_culling_{}", scenario_name), |b| {
            b.iter(|| {
                let mesh = mesh_generator.generate_cube_mesh_with_culling(
                    black_box([0.0, 0.0, 0.0]), 
                    black_box(visible_faces)
                );
                black_box(mesh);
            });
        });
    }
    
    group.finish();
}

/// Benchmark mesh validation and integrity checks
fn bench_mesh_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("mesh_validation");
    
    let mesh_generator = MeshGenerator::new();
    let dimensions = ChunkDimensions { width: 16, height: 16, depth: 16 };
    let chunk = create_test_chunk(dimensions, 0.5);
    let mesh = mesh_generator.generate_chunk_mesh(&chunk);
    
    group.bench_function("mesh_validate", |b| {
        b.iter(|| {
            let result = mesh.validate();
            black_box(result);
        });
    });
    
    group.bench_function("mesh_is_empty", |b| {
        b.iter(|| {
            let result = mesh.is_empty();
            black_box(result);
        });
    });
    
    group.bench_function("mesh_triangle_count", |b| {
        b.iter(|| {
            let count = mesh.triangle_count();
            black_box(count);
        });
    });
    
    // Benchmark mesh creation and addition
    group.bench_function("mesh_add_geometry", |b| {
        b.iter(|| {
            let mut new_mesh = ChunkMesh::new();
            let vertices = vec![
                ChunkVertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
                ChunkVertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
                ChunkVertex::new([1.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 1.0]),
            ];
            let indices = vec![0, 1, 2];
            new_mesh.add_geometry(black_box(&vertices), black_box(&indices));
            black_box(new_mesh);
        });
    });
    
    group.finish();
}

/// Benchmark face culling algorithms
fn bench_face_culling_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("face_culling_algorithms");
    
    let mesh_generator = MeshGenerator::new();
    let dimensions = ChunkDimensions { width: 16, height: 16, depth: 16 };
    
    // Create chunks with different patterns for face culling testing
    let test_scenarios = vec![
        ("sparse_blocks", create_test_chunk(dimensions, 0.1)),
        ("checkerboard", {
            let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
            for x in 0..dimensions.width {
                for y in 0..dimensions.height {
                    for z in 0..dimensions.depth {
                        if (x + y + z) % 2 == 0 {
                            chunk.set_block(x, y, z, BlockID::Stone).unwrap();
                        }
                    }
                }
            }
            chunk
        }),
        ("solid_cube", create_test_chunk(dimensions, 1.0)),
    ];
    
    for (scenario_name, chunk) in test_scenarios {
        // Benchmark individual face visibility checks
        group.bench_function(format!("should_render_face_{}", scenario_name), |b| {
            b.iter(|| {
                let mut visible_count = 0;
                for x in 0..dimensions.width {
                    for y in 0..dimensions.height {
                        for z in 0..dimensions.depth {
                            if chunk.get_block(x, y, z).unwrap() != BlockID::Air {
                                let visible = mesh_generator.should_render_face(
                                    black_box(&chunk), 
                                    black_box(x), 
                                    black_box(y), 
                                    black_box(z), 
                                    black_box(world::rendering::FaceDirection::PosX)
                                );
                                if visible { visible_count += 1; }
                            }
                        }
                    }
                }
                black_box(visible_count);
            });
        });
        
        // Benchmark visible faces bitmask generation
        group.bench_function(format!("get_visible_faces_{}", scenario_name), |b| {
            b.iter(|| {
                let mut total_faces = 0u32;
                for x in 0..dimensions.width {
                    for y in 0..dimensions.height {
                        for z in 0..dimensions.depth {
                            if chunk.get_block(x, y, z).unwrap() != BlockID::Air {
                                let visible_faces = mesh_generator.get_visible_faces(
                                    black_box(&chunk), 
                                    black_box(x), 
                                    black_box(y), 
                                    black_box(z)
                                );
                                total_faces += visible_faces.count_ones();
                            }
                        }
                    }
                }
                black_box(total_faces);
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_mesh_generation_by_size,
    bench_mesh_generation_by_fill,
    bench_face_culling_effectiveness,
    bench_cube_mesh_generation,
    bench_mesh_validation
);

criterion_main!(benches);