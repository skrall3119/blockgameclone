//! Performance benchmarks for terrain generation system
//!
//! These benchmarks measure the performance of terrain generation algorithms,
//! noise sampling, caching effectiveness, and memory usage to ensure optimal
//! performance characteristics and meet the <50ms per chunk target.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::time::Duration;
use world::{
    generation::terrain::{TerrainGenerator, NoiseConfiguration, NoiseCache},
    world::coordinate::ChunkCoord,
};

/// Create a terrain generator with default configuration for benchmarking
fn create_benchmark_generator() -> TerrainGenerator {
    TerrainGenerator::new(12345) // Fixed seed for consistent benchmarks
}

/// Create a terrain generator with custom configuration
fn create_custom_generator(config: NoiseConfiguration) -> TerrainGenerator {
    TerrainGenerator::with_config(12345, config)
}

/// Benchmark basic terrain generation performance
fn bench_terrain_generation_basic(c: &mut Criterion) {
    let mut group = c.benchmark_group("terrain_generation_basic");
    group.sample_size(50); // Reduce sample size for faster benchmarks
    
    let generator = create_benchmark_generator();
    
    // Benchmark single chunk generation
    group.bench_function("single_chunk_generation", |b| {
        b.iter(|| {
            let chunk_coord = ChunkCoord::new(0, 0, 0);
            let chunk = generator.generate_chunk(black_box(chunk_coord));
            black_box(chunk);
        });
    });
    
    // Benchmark multiple chunk generation
    let chunk_coords = vec![
        ChunkCoord::new(0, 0, 0),
        ChunkCoord::new(1, 0, 0),
        ChunkCoord::new(0, 0, 1),
        ChunkCoord::new(1, 0, 1),
    ];
    
    group.throughput(Throughput::Elements(chunk_coords.len() as u64));
    group.bench_function("multiple_chunk_generation", |b| {
        b.iter(|| {
            for &coord in black_box(&chunk_coords) {
                let chunk = generator.generate_chunk(coord);
                black_box(chunk);
            }
        });
    });
    
    group.finish();
}

/// Benchmark terrain generation with performance monitoring
fn bench_terrain_generation_monitored(c: &mut Criterion) {
    let mut group = c.benchmark_group("terrain_generation_monitored");
    group.sample_size(30); // Reduce sample size due to monitoring overhead
    
    let mut generator = create_benchmark_generator();
    
    // Benchmark monitored chunk generation
    group.bench_function("monitored_chunk_generation", |b| {
        b.iter(|| {
            let chunk_coord = ChunkCoord::new(0, 0, 0);
            let chunk = generator.generate_chunk_monitored(black_box(chunk_coord));
            black_box(chunk);
        });
    });
    
    // Benchmark monitored height generation
    group.bench_function("monitored_height_generation", |b| {
        b.iter(|| {
            let height = generator.get_height_at_monitored(black_box(100), black_box(200));
            black_box(height);
        });
    });
    
    group.finish();
}

/// Benchmark optimized terrain generation with caching
fn bench_terrain_generation_optimized(c: &mut Criterion) {
    let mut group = c.benchmark_group("terrain_generation_optimized");
    group.sample_size(50);
    
    let mut generator = create_benchmark_generator();
    
    // Benchmark optimized chunk generation
    group.bench_function("optimized_chunk_generation", |b| {
        b.iter(|| {
            let chunk_coord = ChunkCoord::new(0, 0, 0);
            let chunk = generator.generate_chunk_optimized(black_box(chunk_coord));
            black_box(chunk);
        });
    });
    
    // Benchmark cached height generation
    group.bench_function("cached_height_generation", |b| {
        b.iter(|| {
            let height = generator.get_height_at_cached(black_box(100), black_box(200));
            black_box(height);
        });
    });
    
    // Benchmark cached biome generation
    group.bench_function("cached_biome_generation", |b| {
        b.iter(|| {
            let biome = generator.get_biome_at_cached(black_box(100), black_box(200));
            black_box(biome);
        });
    });
    
    group.finish();
}

/// Benchmark noise configuration effects on performance
fn bench_noise_configuration_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("noise_configuration_performance");
    group.sample_size(30);
    
    let configurations = vec![
        ("low_detail", NoiseConfiguration::new(0.005, 16.0, 2, 0.002, 1).unwrap()),
        ("default", NoiseConfiguration::default()),
        ("high_detail", NoiseConfiguration::new(0.02, 64.0, 6, 0.01, 3).unwrap()),
    ];
    
    for (config_name, config) in configurations {
        let generator = create_custom_generator(config);
        
        group.bench_with_input(
            BenchmarkId::new("chunk_generation", config_name),
            &generator,
            |b, gen| {
                b.iter(|| {
                    let chunk_coord = ChunkCoord::new(0, 0, 0);
                    let chunk = gen.generate_chunk(black_box(chunk_coord));
                    black_box(chunk);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark noise cache effectiveness
fn bench_noise_cache_effectiveness(c: &mut Criterion) {
    let mut group = c.benchmark_group("noise_cache_effectiveness");
    
    let mut generator = create_benchmark_generator();
    
    // Benchmark cache hit performance
    group.bench_function("cache_hit_performance", |b| {
        // Pre-populate cache
        for x in 0..10 {
            for z in 0..10 {
                let _ = generator.get_height_at_cached(x * 10, z * 10);
            }
        }
        
        b.iter(|| {
            // Access cached values
            for x in 0..10 {
                for z in 0..10 {
                    let height = generator.get_height_at_cached(black_box(x * 10), black_box(z * 10));
                    black_box(height);
                }
            }
        });
    });
    
    // Benchmark cache miss performance
    group.bench_function("cache_miss_performance", |b| {
        generator.clear_cache(); // Ensure cache is empty
        
        b.iter(|| {
            // Access new values (cache misses)
            for x in 100..110 {
                for z in 100..110 {
                    let height = generator.get_height_at_cached(black_box(x * 10), black_box(z * 10));
                    black_box(height);
                }
            }
            generator.clear_cache(); // Clear for next iteration
        });
    });
    
    // Benchmark cache vs non-cached performance
    let test_coords = vec![(100, 200), (150, 250), (200, 300), (250, 350)];
    
    group.bench_function("non_cached_height_generation", |b| {
        b.iter(|| {
            for &(x, z) in black_box(&test_coords) {
                let height = generator.get_height_at(x, z);
                black_box(height);
            }
        });
    });
    
    group.bench_function("cached_height_generation_comparison", |b| {
        b.iter(|| {
            for &(x, z) in black_box(&test_coords) {
                let height = generator.get_height_at_cached(x, z);
                black_box(height);
            }
        });
    });
    
    group.finish();
}

/// Benchmark memory usage and allocation patterns
fn bench_memory_usage_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage_patterns");
    
    // Benchmark terrain generator creation
    group.bench_function("generator_creation", |b| {
        b.iter(|| {
            let generator = create_benchmark_generator();
            black_box(generator);
        });
    });
    
    // Benchmark noise cache creation and usage
    group.bench_function("noise_cache_creation", |b| {
        b.iter(|| {
            let cache = NoiseCache::new(1024);
            black_box(cache);
        });
    });
    
    // Benchmark memory allocation during chunk generation
    group.bench_function("chunk_generation_allocations", |b| {
        let mut generator = create_benchmark_generator();
        
        b.iter(|| {
            let chunk_coord = ChunkCoord::new(0, 0, 0);
            let chunk = generator.generate_chunk_optimized(black_box(chunk_coord));
            black_box(chunk);
        });
    });
    
    group.finish();
}

/// Benchmark performance target validation (< 50ms per chunk)
fn bench_performance_target_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_target_validation");
    group.measurement_time(Duration::from_secs(10)); // Longer measurement for accuracy
    group.sample_size(100);
    
    let mut generator = create_benchmark_generator();
    
    // Test different generation methods against the 50ms target
    let test_methods = vec![
        ("standard_generation", false, false),
        ("monitored_generation", true, false),
        ("optimized_generation", false, true),
    ];
    
    for (method_name, use_monitoring, use_optimization) in test_methods {
        group.bench_function(method_name, |b| {
            b.iter_custom(|iters| {
                let start = std::time::Instant::now();
                
                for i in 0..iters {
                    let chunk_coord = ChunkCoord::new((i % 10) as i32, 0, (i / 10) as i32);
                    
                    let _chunk = if use_optimization {
                        generator.generate_chunk_optimized(chunk_coord)
                    } else if use_monitoring {
                        generator.generate_chunk_monitored(chunk_coord)
                    } else {
                        generator.generate_chunk(chunk_coord)
                    };
                }
                
                start.elapsed()
            });
        });
    }
    
    group.finish();
}

/// Benchmark scalability with different chunk counts
fn bench_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalability");
    
    let chunk_counts = vec![1, 4, 9, 16]; // 1x1, 2x2, 3x3, 4x4 grids
    
    for &chunk_count in &chunk_counts {
        let side_length = (chunk_count as f64).sqrt() as i32;
        let mut coords = Vec::new();
        
        for x in 0..side_length {
            for z in 0..side_length {
                coords.push(ChunkCoord::new(x, 0, z));
            }
        }
        
        group.throughput(Throughput::Elements(chunk_count as u64));
        group.bench_with_input(
            BenchmarkId::new("chunk_grid_generation", chunk_count),
            &coords,
            |b, coords| {
                let mut generator = create_benchmark_generator();
                
                b.iter(|| {
                    for &coord in black_box(coords) {
                        let chunk = generator.generate_chunk_optimized(coord);
                        black_box(chunk);
                    }
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark different biome configurations
fn bench_biome_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("biome_performance");
    
    let mut generator = create_benchmark_generator();
    
    // Benchmark biome generation at different scales
    let scales = vec![
        ("local", 1, 10),      // 10x10 area
        ("chunk", 16, 1),      // Single chunk
        ("region", 32, 1),     // Large area
    ];
    
    for (scale_name, step_size, iterations) in scales {
        group.bench_function(format!("biome_generation_{}", scale_name), |b| {
            b.iter(|| {
                for i in 0..iterations {
                    for x in (0..100).step_by(step_size) {
                        for z in (0..100).step_by(step_size) {
                            let biome = generator.get_biome_at_cached(
                                black_box(x + i * 100), 
                                black_box(z + i * 100)
                            );
                            black_box(biome);
                        }
                    }
                }
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_terrain_generation_basic,
    bench_terrain_generation_monitored,
    bench_terrain_generation_optimized,
    bench_noise_configuration_performance,
    bench_noise_cache_effectiveness,
    bench_memory_usage_patterns,
    bench_performance_target_validation,
    bench_scalability,
    bench_biome_performance
);

criterion_main!(benches);