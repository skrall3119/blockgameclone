//! Terrain Generation Example
//!
//! This example demonstrates the terrain generation system including:
//! - Different biome types (Plains and Hills)
//! - Height generation with noise functions
//! - Block placement and layering
//! - Deterministic generation from seeds
//! - Performance monitoring and statistics
//! - Coordinate system integration

use anyhow::Result;
use log::info;
use std::time::{Duration, Instant};
use world::{
    generation::terrain::{TerrainGenerator, BiomeType, NoiseConfiguration},
    chunk::BlockID,
    ChunkCoord,
};

/// Main entry point for the terrain generation example
pub fn run_terrain_generation_example() -> Result<()> {
    env_logger::init();
    
    info!("Starting terrain generation example...");
    
    // Create terrain generation demo
    let mut demo = TerrainGenerationDemo::new();
    
    // Run all demonstration modes
    demo.run_all_demonstrations()?;
    
    info!("Terrain generation example completed successfully!");
    
    Ok(())
}

/// Terrain generation demonstration controller
struct TerrainGenerationDemo {
    generator: TerrainGenerator,
    performance_tracker: PerformanceTracker,
}

/// Performance tracking for terrain generation
struct PerformanceTracker {
    generation_times: Vec<Duration>,
    total_chunks_generated: u32,
    start_time: Instant,
}

impl PerformanceTracker {
    fn new() -> Self {
        Self {
            generation_times: Vec::new(),
            total_chunks_generated: 0,
            start_time: Instant::now(),
        }
    }

    fn record_generation(&mut self, duration: Duration) {
        self.generation_times.push(duration);
        self.total_chunks_generated += 1;
    }

    fn get_statistics(&self) -> GenerationStatistics {
        if self.generation_times.is_empty() {
            return GenerationStatistics::default();
        }

        let total_time: Duration = self.generation_times.iter().sum();
        let avg_time = total_time / self.generation_times.len() as u32;
        let min_time = *self.generation_times.iter().min().unwrap();
        let max_time = *self.generation_times.iter().max().unwrap();
        
        let chunks_per_second = if total_time.as_secs_f64() > 0.0 {
            self.total_chunks_generated as f64 / total_time.as_secs_f64()
        } else {
            0.0
        };

        GenerationStatistics {
            total_chunks: self.total_chunks_generated,
            avg_generation_time: avg_time,
            min_generation_time: min_time,
            max_generation_time: max_time,
            chunks_per_second,
            meets_performance_target: avg_time < Duration::from_millis(50),
        }
    }
}

#[derive(Debug, Default)]
struct GenerationStatistics {
    total_chunks: u32,
    avg_generation_time: Duration,
    min_generation_time: Duration,
    max_generation_time: Duration,
    chunks_per_second: f64,
    meets_performance_target: bool,
}

impl TerrainGenerationDemo {
    /// Create a new terrain generation demo with a known seed
    fn new() -> Self {
        let seed = 42; // Use a fixed seed for reproducible demonstrations
        let generator = TerrainGenerator::new(seed);
        let performance_tracker = PerformanceTracker::new();

        info!("Created terrain generator with seed: {}", seed);
        
        Self {
            generator,
            performance_tracker,
        }
    }

    /// Run all terrain generation demonstrations
    fn run_all_demonstrations(&mut self) -> Result<()> {
        info!("=== Terrain Generation Demonstration ===");
        
        // 1. Basic terrain generation
        self.demonstrate_basic_generation()?;
        
        // 2. Biome system demonstration
        self.demonstrate_biome_system()?;
        
        // 3. Height variation demonstration
        self.demonstrate_height_variation()?;
        
        // 4. Block layering demonstration
        self.demonstrate_block_layering()?;
        
        // 5. Deterministic generation demonstration
        self.demonstrate_deterministic_generation()?;
        
        // 6. Performance demonstration
        self.demonstrate_performance()?;
        
        // 7. Coordinate system demonstration
        self.demonstrate_coordinate_system()?;
        
        // 8. Custom configuration demonstration
        self.demonstrate_custom_configuration()?;
        
        // Print final statistics
        self.print_final_statistics();
        
        Ok(())
    }

    /// Demonstrate basic terrain generation
    fn demonstrate_basic_generation(&mut self) -> Result<()> {
        info!("\n--- Basic Terrain Generation ---");
        
        // Generate a single chunk at origin
        let coord = ChunkCoord::new(0, 0, 0);
        let start_time = Instant::now();
        let chunk = self.generator.generate_chunk(coord);
        let generation_time = start_time.elapsed();
        
        self.performance_tracker.record_generation(generation_time);
        
        info!("Generated chunk at {:?} in {:.2}ms", coord, generation_time.as_millis());
        
        // Analyze the generated chunk
        let mut block_counts = std::collections::HashMap::new();
        let dimensions = chunk.dimensions();
        
        for x in 0..dimensions.width {
            for y in 0..dimensions.height {
                for z in 0..dimensions.depth {
                    if let Ok(block) = chunk.get_block(x, y, z) {
                        *block_counts.entry(block).or_insert(0) += 1;
                    }
                }
            }
        }
        
        info!("Chunk analysis:");
        info!("  Dimensions: {}x{}x{}", dimensions.width, dimensions.height, dimensions.depth);
        info!("  Block distribution:");
        for (block_type, count) in &block_counts {
            let percentage = (*count as f32 / (dimensions.width * dimensions.height * dimensions.depth) as f32) * 100.0;
            info!("    {:?}: {} blocks ({:.1}%)", block_type, count, percentage);
        }
        
        // Sample some specific positions to show terrain structure
        info!("  Sample blocks:");
        let sample_positions = [(0, 0, 0), (8, 64, 8), (15, 128, 15), (7, 200, 7)];
        for (x, y, z) in sample_positions {
            if let Ok(block) = chunk.get_block(x, y, z) {
                info!("    ({}, {}, {}): {:?}", x, y, z, block);
            }
        }
        
        Ok(())
    }

    /// Demonstrate the biome system
    fn demonstrate_biome_system(&mut self) -> Result<()> {
        info!("\n--- Biome System Demonstration ---");
        
        // Sample biomes across a region
        let sample_region = 50; // 50x50 region
        let mut biome_counts = std::collections::HashMap::new();
        let mut biome_positions = Vec::new();
        
        for x in -sample_region..=sample_region {
            for z in -sample_region..=sample_region {
                let biome = self.generator.get_biome_at(x, z);
                *biome_counts.entry(biome).or_insert(0) += 1;
                biome_positions.push((x, z, biome));
            }
        }
        
        info!("Biome distribution in {}x{} region:", sample_region * 2 + 1, sample_region * 2 + 1);
        let total_samples = biome_positions.len();
        for (biome, count) in &biome_counts {
            let percentage = (*count as f32 / total_samples as f32) * 100.0;
            info!("  {:?}: {} positions ({:.1}%)", biome, count, percentage);
        }
        
        // Show biome-specific height characteristics
        info!("Biome height characteristics:");
        for biome_type in [BiomeType::Plains, BiomeType::Hills] {
            let mut heights = Vec::new();
            
            // Collect heights for this biome type
            for (x, z, biome) in &biome_positions {
                if *biome == biome_type {
                    let height = self.generator.get_height_at(*x, *z);
                    heights.push(height);
                }
            }
            
            if !heights.is_empty() {
                let avg_height = heights.iter().sum::<f32>() / heights.len() as f32;
                let min_height = heights.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                let max_height = heights.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                
                // Calculate variance
                let variance = heights.iter()
                    .map(|h| (h - avg_height).powi(2))
                    .sum::<f32>() / heights.len() as f32;
                let std_dev = variance.sqrt();
                
                info!("  {:?}:", biome_type);
                info!("    Average height: {:.1}", avg_height);
                info!("    Height range: {:.1} - {:.1}", min_height, max_height);
                info!("    Standard deviation: {:.1}", std_dev);
                info!("    Height modifier: {:.1}", biome_type.height_modifier());
                info!("    Surface block: {:?}", biome_type.surface_block());
            }
        }
        
        Ok(())
    }

    /// Demonstrate height variation across terrain
    fn demonstrate_height_variation(&mut self) -> Result<()> {
        info!("\n--- Height Variation Demonstration ---");
        
        // Generate height map for a region
        let region_size = 32;
        let mut heights = Vec::new();
        let mut height_map = Vec::new();
        
        for z in 0..region_size {
            let mut row = Vec::new();
            for x in 0..region_size {
                let height = self.generator.get_height_at(x, z);
                heights.push(height);
                row.push(height);
            }
            height_map.push(row);
        }
        
        // Calculate height statistics
        let avg_height = heights.iter().sum::<f32>() / heights.len() as f32;
        let min_height = heights.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_height = heights.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let height_range = max_height - min_height;
        
        info!("Height map analysis ({}x{} region):", region_size, region_size);
        info!("  Average height: {:.1}", avg_height);
        info!("  Height range: {:.1} - {:.1} (span: {:.1})", min_height, max_height, height_range);
        
        // Show terrain smoothness by checking adjacent height differences
        let mut height_differences = Vec::new();
        for z in 0..(region_size - 1) {
            for x in 0..(region_size - 1) {
                let current = height_map[z as usize][x as usize];
                let right = height_map[z as usize][(x + 1) as usize];
                let down = height_map[(z + 1) as usize][x as usize];
                
                height_differences.push((current - right).abs());
                height_differences.push((current - down).abs());
            }
        }
        
        let avg_height_diff = height_differences.iter().sum::<f32>() / height_differences.len() as f32;
        let max_height_diff = height_differences.iter().fold(f32::NEG_INFINITY, |a: f32, &b| a.max(b));
        
        info!("  Terrain smoothness:");
        info!("    Average adjacent height difference: {:.2}", avg_height_diff);
        info!("    Maximum adjacent height difference: {:.2}", max_height_diff);
        info!("    Smoothness rating: {}", if avg_height_diff < 2.0 { "Smooth" } else if avg_height_diff < 5.0 { "Moderate" } else { "Rough" });
        
        // Display a small ASCII height map
        info!("  ASCII height map (8x8 sample, 'o' = low, 'X' = high):");
        for z in 0..8 {
            let mut line = String::from("    ");
            for x in 0..8 {
                let height = height_map[z * 4][x * 4]; // Sample every 4th position
                let normalized = ((height - min_height) / height_range * 4.0) as u32;
                let char = match normalized {
                    0 => 'o',
                    1 => '.',
                    2 => '+',
                    3 => '#',
                    _ => 'X',
                };
                line.push(char);
            }
            info!("{}", line);
        }
        
        Ok(())
    }

    /// Demonstrate block layering system
    fn demonstrate_block_layering(&mut self) -> Result<()> {
        info!("\n--- Block Layering Demonstration ---");
        
        // Generate a chunk and analyze vertical structure
        let coord = ChunkCoord::new(1, 0, 1);
        let start_time = Instant::now();
        let chunk = self.generator.generate_chunk(coord);
        let generation_time = start_time.elapsed();
        
        self.performance_tracker.record_generation(generation_time);
        
        // Analyze several columns to show layering
        let sample_columns = [(0, 0), (8, 8), (15, 15)];
        
        for (x, z) in sample_columns {
            info!("Column analysis at local ({}, {}):", x, z);
            
            // Get world coordinates for this column
            let world_x = coord.x * 16 + x as i32;
            let world_z = coord.z * 16 + z as i32;
            let terrain_height = self.generator.get_height_at(world_x, world_z);
            let biome = self.generator.get_biome_at(world_x, world_z);
            
            info!("  World position: ({}, {})", world_x, world_z);
            info!("  Terrain height: {:.1}", terrain_height);
            info!("  Biome: {:?}", biome);
            
            // Analyze vertical structure
            let mut layers = Vec::new();
            let mut current_block = None;
            let mut layer_start = 0;
            
            for y in 0..256 {
                if let Ok(block) = chunk.get_block(x, y, z) {
                    if current_block != Some(block) {
                        if let Some(prev_block) = current_block {
                            layers.push((prev_block, layer_start, y - 1));
                        }
                        current_block = Some(block);
                        layer_start = y;
                    }
                }
            }
            
            // Add final layer
            if let Some(block) = current_block {
                layers.push((block, layer_start, 255));
            }
            
            info!("  Layer structure:");
            for (block_type, start_y, end_y) in layers {
                let thickness = end_y - start_y + 1;
                info!("    Y {}-{}: {:?} ({} blocks thick)", start_y, end_y, block_type, thickness);
            }
            
            // Verify layering rules
            let surface_y = terrain_height as usize;
            if surface_y < 256 {
                if let Ok(surface_block) = chunk.get_block(x, surface_y, z) {
                    let expected_surface = biome.surface_block();
                    let surface_correct = surface_block == expected_surface;
                    info!("  Surface block validation: {} (expected {:?}, got {:?})", 
                          if surface_correct { "✓" } else { "✗" }, expected_surface, surface_block);
                }
                
                // Check subsurface layers
                if surface_y > 0 {
                    if let Ok(subsurface_block) = chunk.get_block(x, surface_y - 1, z) {
                        let subsurface_correct = subsurface_block == BlockID::Dirt;
                        info!("  Subsurface block validation: {} (expected Dirt, got {:?})", 
                              if subsurface_correct { "✓" } else { "✗" }, subsurface_block);
                    }
                }
                
                // Check air above surface
                if surface_y + 1 < 256 {
                    if let Ok(air_block) = chunk.get_block(x, surface_y + 1, z) {
                        let air_correct = air_block == BlockID::Air;
                        info!("  Air block validation: {} (expected Air, got {:?})", 
                              if air_correct { "✓" } else { "✗" }, air_block);
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Demonstrate deterministic generation
    fn demonstrate_deterministic_generation(&mut self) -> Result<()> {
        info!("\n--- Deterministic Generation Demonstration ---");
        
        let seed = 12345;
        let test_coords = [
            ChunkCoord::new(0, 0, 0),
            ChunkCoord::new(5, 0, -3),
            ChunkCoord::new(-2, 0, 7),
        ];
        
        info!("Testing deterministic generation with seed: {}", seed);
        
        for coord in test_coords {
            // Generate chunk with first generator
            let generator1 = TerrainGenerator::new(seed);
            let start_time = Instant::now();
            let chunk1 = generator1.generate_chunk(coord);
            let generation_time1 = start_time.elapsed();
            
            // Generate same chunk with second generator (same seed)
            let generator2 = TerrainGenerator::new(seed);
            let start_time = Instant::now();
            let chunk2 = generator2.generate_chunk(coord);
            let generation_time2 = start_time.elapsed();
            
            self.performance_tracker.record_generation(generation_time1);
            self.performance_tracker.record_generation(generation_time2);
            
            // Compare chunks block by block
            let mut identical_blocks = 0;
            let mut total_blocks = 0;
            let mut sample_differences = Vec::new();
            
            let dimensions = chunk1.dimensions();
            for x in 0..dimensions.width {
                for y in 0..dimensions.height {
                    for z in 0..dimensions.depth {
                        total_blocks += 1;
                        
                        let block1 = chunk1.get_block(x, y, z);
                        let block2 = chunk2.get_block(x, y, z);
                        
                        match (block1.clone(), block2.clone()) {
                            (Ok(b1), Ok(b2)) => {
                                if b1 == b2 {
                                    identical_blocks += 1;
                                } else {
                                    sample_differences.push((x, y, z, b1, b2));
                                    if sample_differences.len() >= 5 { // Limit sample size
                                        break;
                                    }
                                }
                            }
                            _ => {
                                sample_differences.push((x, y, z, block1.unwrap_or(BlockID::Air), block2.unwrap_or(BlockID::Air)));
                            }
                        }
                    }
                }
            }
            
            let match_percentage = (identical_blocks as f32 / total_blocks as f32) * 100.0;
            
            info!("Chunk {:?} determinism test:", coord);
            info!("  Generation times: {:.2}ms, {:.2}ms", generation_time1.as_millis(), generation_time2.as_millis());
            info!("  Block matches: {}/{} ({:.2}%)", identical_blocks, total_blocks, match_percentage);
            
            if match_percentage == 100.0 {
                info!("  Result: ✓ Perfect determinism");
            } else {
                info!("  Result: ✗ Determinism failed");
                info!("  Sample differences:");
                for (x, y, z, b1, b2) in sample_differences.iter().take(3) {
                    info!("    ({}, {}, {}): {:?} vs {:?}", x, y, z, b1, b2);
                }
            }
            
            // Test height and biome determinism
            let sample_positions = [(0, 0), (8, 8), (15, 15)];
            let mut height_matches = 0;
            let mut biome_matches = 0;
            
            for (local_x, local_z) in sample_positions {
                let world_x = coord.x * 16 + local_x;
                let world_z = coord.z * 16 + local_z;
                
                let height1 = generator1.get_height_at(world_x, world_z);
                let height2 = generator2.get_height_at(world_x, world_z);
                let biome1 = generator1.get_biome_at(world_x, world_z);
                let biome2 = generator2.get_biome_at(world_x, world_z);
                
                if (height1 - height2).abs() < f32::EPSILON {
                    height_matches += 1;
                }
                if biome1 == biome2 {
                    biome_matches += 1;
                }
            }
            
            info!("  Height determinism: {}/{} matches", height_matches, sample_positions.len());
            info!("  Biome determinism: {}/{} matches", biome_matches, sample_positions.len());
        }
        
        Ok(())
    }

    /// Demonstrate performance characteristics
    fn demonstrate_performance(&mut self) -> Result<()> {
        info!("\n--- Performance Demonstration ---");
        
        // Generate multiple chunks to test performance
        let test_chunks = [
            ChunkCoord::new(0, 0, 0),
            ChunkCoord::new(1, 0, 0),
            ChunkCoord::new(0, 0, 1),
            ChunkCoord::new(-1, 0, 0),
            ChunkCoord::new(0, 0, -1),
            ChunkCoord::new(2, 0, 2),
            ChunkCoord::new(-2, 0, -2),
            ChunkCoord::new(3, 0, -1),
            ChunkCoord::new(-1, 0, 3),
            ChunkCoord::new(5, 0, 5),
        ];
        
        info!("Generating {} chunks for performance testing...", test_chunks.len());
        
        let overall_start = Instant::now();
        let mut generation_times = Vec::new();
        
        for (i, coord) in test_chunks.iter().enumerate() {
            let start_time = Instant::now();
            let _chunk = self.generator.generate_chunk(*coord);
            let generation_time = start_time.elapsed();
            
            generation_times.push(generation_time);
            self.performance_tracker.record_generation(generation_time);
            
            info!("  Chunk {} ({:?}): {:.2}ms", i + 1, coord, generation_time.as_millis());
        }
        
        let total_time = overall_start.elapsed();
        
        // Calculate performance statistics
        let avg_time = generation_times.iter().sum::<Duration>() / generation_times.len() as u32;
        let min_time = *generation_times.iter().min().unwrap();
        let max_time = *generation_times.iter().max().unwrap();
        let chunks_per_second = test_chunks.len() as f64 / total_time.as_secs_f64();
        
        info!("Performance summary:");
        info!("  Total time: {:.2}ms", total_time.as_millis());
        info!("  Average time per chunk: {:.2}ms", avg_time.as_millis());
        info!("  Fastest chunk: {:.2}ms", min_time.as_millis());
        info!("  Slowest chunk: {:.2}ms", max_time.as_millis());
        info!("  Generation rate: {:.1} chunks/second", chunks_per_second);
        
        // Check performance target (< 50ms per chunk)
        let target_met = avg_time < Duration::from_millis(50);
        info!("  Performance target (<50ms): {} (avg: {:.2}ms)", 
              if target_met { "✓ MET" } else { "✗ MISSED" }, avg_time.as_millis());
        
        // Memory usage estimation
        let chunk_size = 16 * 256 * 16; // blocks per chunk
        let estimated_memory_per_chunk = chunk_size * std::mem::size_of::<BlockID>();
        let total_estimated_memory = estimated_memory_per_chunk * test_chunks.len();
        
        info!("  Estimated memory usage:");
        info!("    Per chunk: {:.2} KB", estimated_memory_per_chunk as f64 / 1024.0);
        info!("    Total for {} chunks: {:.2} KB", test_chunks.len(), total_estimated_memory as f64 / 1024.0);
        
        Ok(())
    }

    /// Demonstrate coordinate system integration
    fn demonstrate_coordinate_system(&mut self) -> Result<()> {
        info!("\n--- Coordinate System Demonstration ---");
        
        // Test coordinate conversions
        let test_chunks = [
            ChunkCoord::new(0, 0, 0),
            ChunkCoord::new(5, 0, -3),
            ChunkCoord::new(-2, 0, 7),
            ChunkCoord::new(-10, 0, -5),
        ];
        
        for chunk_coord in test_chunks {
            info!("Chunk coordinate: {:?}", chunk_coord);
            
            // Test chunk to world coordinate conversion
            let (world_x, world_z) = self.generator.chunk_to_world_coords(chunk_coord);
            info!("  World origin: ({}, {})", world_x, world_z);
            
            // Test world to chunk coordinate conversion
            let recovered_chunk = self.generator.world_to_chunk_coords(world_x, world_z);
            info!("  Recovered chunk: {:?}", recovered_chunk);
            
            // Verify round-trip conversion
            let conversion_correct = recovered_chunk.x == chunk_coord.x && recovered_chunk.z == chunk_coord.z;
            info!("  Round-trip conversion: {}", if conversion_correct { "✓" } else { "✗" });
            
            // Test local coordinate conversion
            let test_local_positions = [(0, 0), (8, 8), (15, 15)];
            for (local_x, local_z) in test_local_positions {
                let (world_pos_x, world_pos_z) = self.generator.local_to_world_coords(chunk_coord, local_x, local_z);
                let (recovered_local_x, recovered_local_z) = self.generator.world_to_local_coords(world_pos_x, world_pos_z);
                
                let local_conversion_correct = recovered_local_x == local_x && recovered_local_z == local_z;
                info!("    Local ({}, {}) -> World ({}, {}) -> Local ({}, {}): {}", 
                      local_x, local_z, world_pos_x, world_pos_z, recovered_local_x, recovered_local_z,
                      if local_conversion_correct { "✓" } else { "✗" });
            }
            
            // Test terrain generation at chunk boundaries
            info!("  Boundary continuity test:");
            let boundary_positions = [
                (world_x, world_z),           // Chunk origin
                (world_x + 15, world_z),      // Eastern edge
                (world_x, world_z + 15),      // Southern edge
                (world_x + 15, world_z + 15), // Southeast corner
            ];
            
            for (wx, wz) in boundary_positions {
                let height = self.generator.get_height_at(wx, wz);
                let biome = self.generator.get_biome_at(wx, wz);
                info!("    ({}, {}): height={:.1}, biome={:?}", wx, wz, height, biome);
            }
        }
        
        Ok(())
    }

    /// Demonstrate custom noise configuration
    fn demonstrate_custom_configuration(&mut self) -> Result<()> {
        info!("\n--- Custom Configuration Demonstration ---");
        
        // Test different noise configurations
        let configurations = [
            ("Default", NoiseConfiguration::default()),
            ("High Frequency", NoiseConfiguration::new(0.02, 32.0, 4, 0.01, 2).unwrap()),
            ("Low Amplitude", NoiseConfiguration::new(0.01, 16.0, 4, 0.005, 2).unwrap()),
            ("Many Octaves", NoiseConfiguration::new(0.01, 32.0, 6, 0.005, 3).unwrap()),
            ("Large Biomes", NoiseConfiguration::new(0.01, 32.0, 4, 0.002, 2).unwrap()),
        ];
        
        let test_coord = ChunkCoord::new(2, 0, 2);
        let seed = 98765;
        
        for (name, config) in configurations {
            info!("Configuration: {}", name);
            info!("  Height: freq={:.3}, amp={:.1}, octaves={}", 
                  config.height_frequency, config.height_amplitude, config.height_octaves);
            info!("  Biome: freq={:.3}, octaves={}", 
                  config.biome_frequency, config.biome_octaves);
            
            // Create generator with custom configuration
            let custom_generator = TerrainGenerator::with_config(seed, config);
            
            // Generate chunk and analyze
            let start_time = Instant::now();
            let _chunk = custom_generator.generate_chunk(test_coord);
            let generation_time = start_time.elapsed();
            
            self.performance_tracker.record_generation(generation_time);
            
            // Analyze height characteristics
            let sample_positions = [(0, 0), (4, 4), (8, 8), (12, 12), (15, 15)];
            let mut heights = Vec::new();
            let mut biomes = Vec::new();
            
            for (local_x, local_z) in sample_positions {
                let world_x = test_coord.x * 16 + local_x as i32;
                let world_z = test_coord.z * 16 + local_z as i32;
                
                let height = custom_generator.get_height_at(world_x, world_z);
                let biome = custom_generator.get_biome_at(world_x, world_z);
                
                heights.push(height);
                biomes.push(biome);
            }
            
            let avg_height = heights.iter().sum::<f32>() / heights.len() as f32;
            let min_height = heights.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            let max_height = heights.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            let height_variance = heights.iter()
                .map(|h| (h - avg_height).powi(2))
                .sum::<f32>() / heights.len() as f32;
            
            let plains_count = biomes.iter().filter(|&&b| b == BiomeType::Plains).count();
            let hills_count = biomes.iter().filter(|&&b| b == BiomeType::Hills).count();
            
            info!("  Results:");
            info!("    Generation time: {:.2}ms", generation_time.as_millis());
            info!("    Height range: {:.1} - {:.1} (avg: {:.1})", min_height, max_height, avg_height);
            info!("    Height variance: {:.2}", height_variance);
            info!("    Biome distribution: {} Plains, {} Hills", plains_count, hills_count);
        }
        
        Ok(())
    }

    /// Print final statistics for the entire demonstration
    fn print_final_statistics(&self) {
        info!("\n=== Final Terrain Generation Statistics ===");
        
        let stats = self.performance_tracker.get_statistics();
        
        info!("Overall Performance:");
        info!("  Total chunks generated: {}", stats.total_chunks);
        info!("  Average generation time: {:.2}ms", stats.avg_generation_time.as_millis());
        info!("  Fastest generation: {:.2}ms", stats.min_generation_time.as_millis());
        info!("  Slowest generation: {:.2}ms", stats.max_generation_time.as_millis());
        info!("  Generation rate: {:.1} chunks/second", stats.chunks_per_second);
        info!("  Performance target: {} (target: <50ms)", 
              if stats.meets_performance_target { "✓ MET" } else { "✗ MISSED" });
        
        let total_runtime = self.performance_tracker.start_time.elapsed();
        info!("  Total demonstration time: {:.2}s", total_runtime.as_secs_f64());
        
        // Terrain generator seed information
        info!("Terrain Generator:");
        info!("  Seed: {}", self.generator.get_seed());
        
        info!("===========================================");
    }
}

/// Convenience function to run the terrain generation example
pub fn main() -> Result<()> {
    run_terrain_generation_example()
}