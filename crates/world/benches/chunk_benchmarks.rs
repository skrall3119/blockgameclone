use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box as std_black_box;
use world::chunk::{Chunk, ChunkPosition, ChunkDimensions, BlockID, DEFAULT_DIMENSIONS};

/// Benchmark block access patterns
fn bench_block_access_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("block_access_patterns");
    
    // Create a chunk for testing
    let mut chunk = Chunk::new(
        ChunkPosition { x: 0, z: 0 },
        DEFAULT_DIMENSIONS,
    );
    
    // Fill chunk with some test data
    chunk.fill(BlockID::Stone);
    
    // Sequential access pattern (cache-friendly)
    group.bench_function("sequential_read", |b| {
        b.iter(|| {
            let mut sum = 0u32;
            for x in 0..DEFAULT_DIMENSIONS.width {
                for y in 0..DEFAULT_DIMENSIONS.height {
                    for z in 0..DEFAULT_DIMENSIONS.depth {
                        let block = chunk.get_block(x, y, z).unwrap();
                        sum += block.to_u16() as u32;
                    }
                }
            }
            black_box(sum)
        })
    });
    
    // Random access pattern (cache-unfriendly)
    group.bench_function("random_read", |b| {
        use rand::prelude::*;
        let mut rng = StdRng::seed_from_u64(42);
        let coords: Vec<(usize, usize, usize)> = (0..1000)
            .map(|_| {
                (
                    rng.gen_range(0..DEFAULT_DIMENSIONS.width),
                    rng.gen_range(0..DEFAULT_DIMENSIONS.height),
                    rng.gen_range(0..DEFAULT_DIMENSIONS.depth),
                )
            })
            .collect();
        
        b.iter(|| {
            let mut sum = 0u32;
            for &(x, y, z) in &coords {
                let block = chunk.get_block(x, y, z).unwrap();
                sum += block.to_u16() as u32;
            }
            black_box(sum)
        })
    });
    
    // Strided access pattern (moderate cache performance)
    group.bench_function("strided_read", |b| {
        b.iter(|| {
            let mut sum = 0u32;
            let stride = 4;
            for x in (0..DEFAULT_DIMENSIONS.width).step_by(stride) {
                for y in (0..DEFAULT_DIMENSIONS.height).step_by(stride) {
                    for z in (0..DEFAULT_DIMENSIONS.depth).step_by(stride) {
                        let block = chunk.get_block(x, y, z).unwrap();
                        sum += block.to_u16() as u32;
                    }
                }
            }
            black_box(sum)
        })
    });
    
    // Sequential write pattern
    group.bench_function("sequential_write", |b| {
        b.iter(|| {
            let mut chunk_copy = chunk.clone();
            for x in 0..DEFAULT_DIMENSIONS.width {
                for y in 0..DEFAULT_DIMENSIONS.height {
                    for z in 0..DEFAULT_DIMENSIONS.depth {
                        let block_type = match (x + y + z) % 4 {
                            0 => BlockID::Air,
                            1 => BlockID::Stone,
                            2 => BlockID::Dirt,
                            _ => BlockID::Grass,
                        };
                        chunk_copy.set_block(x, y, z, block_type).unwrap();
                    }
                }
            }
            black_box(chunk_copy)
        })
    });
    
    // Random write pattern
    group.bench_function("random_write", |b| {
        use rand::prelude::*;
        let mut rng = StdRng::seed_from_u64(42);
        let coords: Vec<(usize, usize, usize, BlockID)> = (0..1000)
            .map(|_| {
                let block_type = match rng.gen_range(0..4) {
                    0 => BlockID::Air,
                    1 => BlockID::Stone,
                    2 => BlockID::Dirt,
                    _ => BlockID::Grass,
                };
                (
                    rng.gen_range(0..DEFAULT_DIMENSIONS.width),
                    rng.gen_range(0..DEFAULT_DIMENSIONS.height),
                    rng.gen_range(0..DEFAULT_DIMENSIONS.depth),
                    block_type,
                )
            })
            .collect();
        
        b.iter(|| {
            let mut chunk_copy = chunk.clone();
            for &(x, y, z, block) in &coords {
                chunk_copy.set_block(x, y, z, block).unwrap();
            }
            black_box(chunk_copy)
        })
    });
    
    group.finish();
}

/// Benchmark batch operations vs individual operations
fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");
    
    let chunk = Chunk::new(
        ChunkPosition { x: 0, z: 0 },
        DEFAULT_DIMENSIONS,
    );
    
    // Generate test data
    use rand::prelude::*;
    let mut rng = StdRng::seed_from_u64(42);
    let coords: Vec<(usize, usize, usize)> = (0..1000)
        .map(|_| {
            (
                rng.gen_range(0..DEFAULT_DIMENSIONS.width),
                rng.gen_range(0..DEFAULT_DIMENSIONS.height),
                rng.gen_range(0..DEFAULT_DIMENSIONS.depth),
            )
        })
        .collect();
    
    let blocks: Vec<((usize, usize, usize), BlockID)> = coords
        .iter()
        .map(|&(x, y, z)| {
            let block_type = match rng.gen_range(0..4) {
                0 => BlockID::Air,
                1 => BlockID::Stone,
                2 => BlockID::Dirt,
                _ => BlockID::Grass,
            };
            ((x, y, z), block_type)
        })
        .collect();
    
    // Benchmark individual get operations
    group.bench_function("individual_get", |b| {
        b.iter(|| {
            let mut results = Vec::with_capacity(coords.len());
            for &(x, y, z) in &coords {
                results.push(chunk.get_block(x, y, z));
            }
            black_box(results)
        })
    });
    
    // Benchmark batch get operations
    group.bench_function("batch_get", |b| {
        b.iter(|| {
            let results = chunk.get_blocks(&coords);
            black_box(results)
        })
    });
    
    // Benchmark individual set operations
    group.bench_function("individual_set", |b| {
        b.iter(|| {
            let mut chunk_copy = chunk.clone();
            for &((x, y, z), block) in &blocks {
                let _ = chunk_copy.set_block(x, y, z, block);
            }
            black_box(chunk_copy)
        })
    });
    
    // Benchmark batch set operations
    group.bench_function("batch_set", |b| {
        b.iter(|| {
            let mut chunk_copy = chunk.clone();
            let _ = chunk_copy.set_blocks(&blocks);
            black_box(chunk_copy)
        })
    });
    
    group.finish();
}

/// Benchmark memory usage and allocation patterns
fn bench_memory_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_operations");
    
    // Benchmark chunk creation
    group.bench_function("chunk_creation", |b| {
        b.iter(|| {
            let chunk = Chunk::new(
                ChunkPosition { x: 0, z: 0 },
                DEFAULT_DIMENSIONS,
            );
            black_box(chunk)
        })
    });
    
    // Benchmark chunk cloning
    group.bench_function("chunk_clone", |b| {
        let chunk = Chunk::new(
            ChunkPosition { x: 0, z: 0 },
            DEFAULT_DIMENSIONS,
        );
        
        b.iter(|| {
            let cloned = chunk.clone();
            black_box(cloned)
        })
    });
    
    // Benchmark chunk fill operation
    group.bench_function("chunk_fill", |b| {
        b.iter(|| {
            let mut chunk = Chunk::new(
                ChunkPosition { x: 0, z: 0 },
                DEFAULT_DIMENSIONS,
            );
            chunk.fill(BlockID::Stone);
            black_box(chunk)
        })
    });
    
    // Benchmark chunk creation from data
    group.bench_function("chunk_from_data", |b| {
        let total_blocks = DEFAULT_DIMENSIONS.width * DEFAULT_DIMENSIONS.height * DEFAULT_DIMENSIONS.depth;
        let data = vec![BlockID::Stone; total_blocks];
        
        b.iter(|| {
            let chunk = Chunk::from_data(
                data.clone(),
                ChunkPosition { x: 0, z: 0 },
                DEFAULT_DIMENSIONS,
            ).unwrap();
            black_box(chunk)
        })
    });
    
    group.finish();
}

/// Benchmark different chunk sizes
fn bench_chunk_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunk_sizes");
    
    let sizes = vec![
        ("16x16x256", ChunkDimensions { width: 16, height: 256, depth: 16 }),
        ("32x32x128", ChunkDimensions { width: 32, height: 128, depth: 32 }),
        ("64x64x64", ChunkDimensions { width: 64, height: 64, depth: 64 }),
    ];
    
    for (name, dimensions) in sizes {
        group.throughput(Throughput::Elements(
            (dimensions.width * dimensions.height * dimensions.depth) as u64
        ));
        
        // Benchmark creation
        group.bench_with_input(
            BenchmarkId::new("creation", name),
            &dimensions,
            |b, dims| {
                b.iter(|| {
                    let chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, *dims);
                    black_box(chunk)
                })
            },
        );
        
        // Benchmark sequential access
        group.bench_with_input(
            BenchmarkId::new("sequential_access", name),
            &dimensions,
            |b, dims| {
                let chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, *dims);
                b.iter(|| {
                    let mut sum = 0u32;
                    for x in 0..dims.width {
                        for y in 0..dims.height {
                            for z in 0..dims.depth {
                                let block = chunk.get_block(x, y, z).unwrap();
                                sum += block.to_u16() as u32;
                            }
                        }
                    }
                    black_box(sum)
                })
            },
        );
        
        // Benchmark fill operation
        group.bench_with_input(
            BenchmarkId::new("fill", name),
            &dimensions,
            |b, dims| {
                b.iter(|| {
                    let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, *dims);
                    chunk.fill(BlockID::Stone);
                    black_box(chunk)
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark coordinate conversion operations
fn bench_coordinate_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("coordinate_conversion");
    
    let chunk = Chunk::new(
        ChunkPosition { x: 0, z: 0 },
        DEFAULT_DIMENSIONS,
    );
    
    // Generate test coordinates
    use rand::prelude::*;
    let mut rng = StdRng::seed_from_u64(42);
    let coords: Vec<(usize, usize, usize)> = (0..10000)
        .map(|_| {
            (
                rng.gen_range(0..DEFAULT_DIMENSIONS.width),
                rng.gen_range(0..DEFAULT_DIMENSIONS.height),
                rng.gen_range(0..DEFAULT_DIMENSIONS.depth),
            )
        })
        .collect();
    
    let total_blocks = DEFAULT_DIMENSIONS.width * DEFAULT_DIMENSIONS.height * DEFAULT_DIMENSIONS.depth;
    let indices: Vec<usize> = (0..10000)
        .map(|_| rng.gen_range(0..total_blocks))
        .collect();
    
    group.throughput(Throughput::Elements(10000));
    
    // Note: These benchmarks access private methods through a test-only interface
    // In a real implementation, you might want to make these methods public for benchmarking
    // or create a benchmark-specific interface
    
    group.bench_function("coords_to_index", |b| {
        b.iter(|| {
            let mut sum = 0usize;
            for &(x, y, z) in &coords {
                // We'll use get_block as a proxy since coords_to_index is private
                // This includes the overhead of array access but gives us the conversion cost
                if let Ok(block) = chunk.get_block(x, y, z) {
                    sum += block.to_u16() as usize;
                }
            }
            black_box(sum)
        })
    });
    
    group.finish();
}

/// Memory usage measurement (not a benchmark, but useful for analysis)
fn measure_memory_usage() {
    println!("\n=== Memory Usage Analysis ===");
    
    let sizes = vec![
        ("16x16x256", ChunkDimensions { width: 16, height: 256, depth: 16 }),
        ("32x32x128", ChunkDimensions { width: 32, height: 128, depth: 32 }),
        ("64x64x64", ChunkDimensions { width: 64, height: 64, depth: 64 }),
    ];
    
    for (name, dimensions) in sizes {
        let total_blocks = dimensions.width * dimensions.height * dimensions.depth;
        let block_size = std::mem::size_of::<BlockID>();
        let chunk_overhead = std::mem::size_of::<ChunkPosition>() + std::mem::size_of::<ChunkDimensions>() + std::mem::size_of::<Vec<BlockID>>();
        let total_memory = total_blocks * block_size + chunk_overhead;
        
        println!("Chunk size {}: {} blocks, {} bytes per block, ~{} KB total memory", 
                 name, total_blocks, block_size, total_memory / 1024);
        
        // Create actual chunk to verify
        let chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        let actual_size = std::mem::size_of_val(&chunk) + chunk.dimensions().width * chunk.dimensions().height * chunk.dimensions().depth * block_size;
        println!("  Actual measured size: ~{} KB", actual_size / 1024);
    }
    
    println!("BlockID size: {} bytes", std::mem::size_of::<BlockID>());
    println!("ChunkPosition size: {} bytes", std::mem::size_of::<ChunkPosition>());
    println!("ChunkDimensions size: {} bytes", std::mem::size_of::<ChunkDimensions>());
    println!("Vec<BlockID> overhead: {} bytes", std::mem::size_of::<Vec<BlockID>>());
}

criterion_group!(
    benches,
    bench_block_access_patterns,
    bench_batch_operations,
    bench_memory_operations,
    bench_chunk_sizes,
    bench_coordinate_conversion
);

criterion_main!(benches);

// Run memory analysis when benchmarks are executed
#[ctor::ctor]
fn init() {
    measure_memory_usage();
}