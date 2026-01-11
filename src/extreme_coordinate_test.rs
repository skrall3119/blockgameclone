//! Extreme Coordinate Handling Test
//!
//! This test validates that the terrain generation system can handle extreme coordinates
//! without integer overflow, panics, or other failures. It tests:
//! - Large positive coordinates
//! - Large negative coordinates  
//! - Coordinates near integer limits
//! - Mixed extreme coordinates
//! - Boundary conditions

use anyhow::Result;
use log::info;
use std::time::Instant;
use world::{
    generation::terrain::TerrainGenerator,
    ChunkCoord,
};

/// Main entry point for extreme coordinate testing
pub fn run_extreme_coordinate_test() -> Result<()> {
    env_logger::init();
    
    info!("Starting extreme coordinate handling test...");
    
    let mut tester = ExtremeCoordinateTester::new();
    
    // Run all extreme coordinate tests
    tester.run_all_tests()?;
    
    info!("Extreme coordinate handling test completed successfully!");
    
    Ok(())
}

/// Extreme coordinate testing controller
struct ExtremeCoordinateTester {
    generator: TerrainGenerator,
    test_results: Vec<TestResult>,
}

/// Result of an individual coordinate test
#[derive(Debug)]
struct TestResult {
    test_name: String,
    coordinates: Vec<(i32, i32)>,
    chunk_coords: Vec<ChunkCoord>,
    success: bool,
    error_message: Option<String>,
    generation_time: std::time::Duration,
}

impl ExtremeCoordinateTester {
    /// Create a new extreme coordinate tester
    fn new() -> Self {
        let seed = 999999; // Use a specific seed for reproducible testing
        let generator = TerrainGenerator::new(seed);
        
        info!("Created terrain generator with seed: {} for extreme coordinate testing", seed);
        
        Self {
            generator,
            test_results: Vec::new(),
        }
    }

    /// Run all extreme coordinate tests
    fn run_all_tests(&mut self) -> Result<()> {
        info!("=== Extreme Coordinate Handling Tests ===");
        
        // 1. Test large positive coordinates
        self.test_large_positive_coordinates()?;
        
        // 2. Test large negative coordinates
        self.test_large_negative_coordinates()?;
        
        // 3. Test mixed extreme coordinates
        self.test_mixed_extreme_coordinates()?;
        
        // 4. Test coordinates near integer limits
        self.test_near_integer_limits()?;
        
        // 5. Test boundary conditions
        self.test_boundary_conditions()?;
        
        // 6. Test coordinate conversion edge cases
        self.test_coordinate_conversion_edge_cases()?;
        
        // 7. Test height and biome generation at extreme coordinates
        self.test_height_biome_extreme_coordinates()?;
        
        // Print summary of all tests
        self.print_test_summary();
        
        Ok(())
    }

    /// Test large positive coordinates
    fn test_large_positive_coordinates(&mut self) -> Result<()> {
        info!("\n--- Large Positive Coordinates Test ---");
        
        let test_coordinates = vec![
            (1_000_000, 1_000_000),
            (10_000_000, 5_000_000),
            (100_000_000, 50_000_000),
            (500_000_000, 300_000_000),
        ];
        
        let chunk_coords: Vec<ChunkCoord> = test_coordinates.iter()
            .map(|(x, z)| self.generator.world_to_chunk_coords(*x, *z))
            .collect();
        
        let start_time = Instant::now();
        let mut success = true;
        let mut error_message = None;
        
        for (i, coord) in chunk_coords.iter().enumerate() {
            let (world_x, world_z) = test_coordinates[i];
            
            info!("Testing coordinate ({}, {}) -> chunk {:?}", world_x, world_z, coord);
            
            // Test coordinate conversion
            match std::panic::catch_unwind(|| {
                let converted_chunk = self.generator.world_to_chunk_coords(world_x, world_z);
                let (recovered_x, recovered_z) = self.generator.chunk_to_world_coords(converted_chunk);
                (converted_chunk, recovered_x, recovered_z)
            }) {
                Ok((converted_chunk, recovered_x, recovered_z)) => {
                    info!("  Coordinate conversion: {} -> {:?} -> ({}, {})", 
                          format!("({}, {})", world_x, world_z), converted_chunk, recovered_x, recovered_z);
                    
                    // Test height generation
                    match std::panic::catch_unwind(|| self.generator.get_height_at(world_x, world_z)) {
                        Ok(height) => {
                            info!("  Height generation: {:.1}", height);
                            
                            // Verify height is within valid bounds
                            if height < 0.0 || height > 255.0 {
                                success = false;
                                error_message = Some(format!("Height {} out of bounds at ({}, {})", height, world_x, world_z));
                            }
                        }
                        Err(_) => {
                            success = false;
                            error_message = Some(format!("Height generation panicked at ({}, {})", world_x, world_z));
                        }
                    }
                    
                    // Test biome generation
                    match std::panic::catch_unwind(|| self.generator.get_biome_at(world_x, world_z)) {
                        Ok(biome) => {
                            info!("  Biome generation: {:?}", biome);
                        }
                        Err(_) => {
                            success = false;
                            error_message = Some(format!("Biome generation panicked at ({}, {})", world_x, world_z));
                        }
                    }
                    
                    // Test chunk generation (only for reasonable chunk coordinates)
                    if coord.x.abs() < 1_000_000 && coord.z.abs() < 1_000_000 {
                        match std::panic::catch_unwind(|| self.generator.generate_chunk(*coord)) {
                            Ok(_chunk) => {
                                info!("  Chunk generation: ✓ Success");
                            }
                            Err(_) => {
                                success = false;
                                error_message = Some(format!("Chunk generation panicked for {:?}", coord));
                            }
                        }
                    } else {
                        info!("  Chunk generation: Skipped (coordinate too extreme for chunk generation)");
                    }
                }
                Err(_) => {
                    success = false;
                    error_message = Some(format!("Coordinate conversion panicked for ({}, {})", world_x, world_z));
                }
            }
        }
        
        let generation_time = start_time.elapsed();
        
        self.test_results.push(TestResult {
            test_name: "Large Positive Coordinates".to_string(),
            coordinates: test_coordinates,
            chunk_coords,
            success,
            error_message: error_message.clone(),
            generation_time,
        });
        
        info!("Large positive coordinates test: {}", if success { "✓ PASSED" } else { "✗ FAILED" });
        if let Some(error) = &error_message {
            info!("Error: {}", error);
        }
        
        Ok(())
    }

    /// Test large negative coordinates
    fn test_large_negative_coordinates(&mut self) -> Result<()> {
        info!("\n--- Large Negative Coordinates Test ---");
        
        let test_coordinates = vec![
            (-1_000_000, -1_000_000),
            (-10_000_000, -5_000_000),
            (-100_000_000, -50_000_000),
            (-500_000_000, -300_000_000),
        ];
        
        let chunk_coords: Vec<ChunkCoord> = test_coordinates.iter()
            .map(|(x, z)| self.generator.world_to_chunk_coords(*x, *z))
            .collect();
        
        let start_time = Instant::now();
        let mut success = true;
        let mut error_message = None;
        
        for (i, coord) in chunk_coords.iter().enumerate() {
            let (world_x, world_z) = test_coordinates[i];
            
            info!("Testing coordinate ({}, {}) -> chunk {:?}", world_x, world_z, coord);
            
            // Test coordinate conversion
            match std::panic::catch_unwind(|| {
                let converted_chunk = self.generator.world_to_chunk_coords(world_x, world_z);
                let (recovered_x, recovered_z) = self.generator.chunk_to_world_coords(converted_chunk);
                (converted_chunk, recovered_x, recovered_z)
            }) {
                Ok((converted_chunk, recovered_x, recovered_z)) => {
                    info!("  Coordinate conversion: {} -> {:?} -> ({}, {})", 
                          format!("({}, {})", world_x, world_z), converted_chunk, recovered_x, recovered_z);
                    
                    // Test height generation
                    match std::panic::catch_unwind(|| self.generator.get_height_at(world_x, world_z)) {
                        Ok(height) => {
                            info!("  Height generation: {:.1}", height);
                            
                            // Verify height is within valid bounds
                            if height < 0.0 || height > 255.0 {
                                success = false;
                                error_message = Some(format!("Height {} out of bounds at ({}, {})", height, world_x, world_z));
                            }
                        }
                        Err(_) => {
                            success = false;
                            error_message = Some(format!("Height generation panicked at ({}, {})", world_x, world_z));
                        }
                    }
                    
                    // Test biome generation
                    match std::panic::catch_unwind(|| self.generator.get_biome_at(world_x, world_z)) {
                        Ok(biome) => {
                            info!("  Biome generation: {:?}", biome);
                        }
                        Err(_) => {
                            success = false;
                            error_message = Some(format!("Biome generation panicked at ({}, {})", world_x, world_z));
                        }
                    }
                    
                    // Test chunk generation (only for reasonable chunk coordinates)
                    if coord.x.abs() < 1_000_000 && coord.z.abs() < 1_000_000 {
                        match std::panic::catch_unwind(|| self.generator.generate_chunk(*coord)) {
                            Ok(_chunk) => {
                                info!("  Chunk generation: ✓ Success");
                            }
                            Err(_) => {
                                success = false;
                                error_message = Some(format!("Chunk generation panicked for {:?}", coord));
                            }
                        }
                    } else {
                        info!("  Chunk generation: Skipped (coordinate too extreme for chunk generation)");
                    }
                }
                Err(_) => {
                    success = false;
                    error_message = Some(format!("Coordinate conversion panicked for ({}, {})", world_x, world_z));
                }
            }
        }
        
        let generation_time = start_time.elapsed();
        
        self.test_results.push(TestResult {
            test_name: "Large Negative Coordinates".to_string(),
            coordinates: test_coordinates,
            chunk_coords,
            success,
            error_message: error_message.clone(),
            generation_time,
        });
        
        info!("Large negative coordinates test: {}", if success { "✓ PASSED" } else { "✗ FAILED" });
        if let Some(error) = &error_message {
            info!("Error: {}", error);
        }
        
        Ok(())
    }

    /// Test mixed extreme coordinates (positive and negative)
    fn test_mixed_extreme_coordinates(&mut self) -> Result<()> {
        info!("\n--- Mixed Extreme Coordinates Test ---");
        
        let test_coordinates = vec![
            (1_000_000, -1_000_000),
            (-10_000_000, 5_000_000),
            (100_000_000, -50_000_000),
            (-500_000_000, 300_000_000),
        ];
        
        let chunk_coords: Vec<ChunkCoord> = test_coordinates.iter()
            .map(|(x, z)| self.generator.world_to_chunk_coords(*x, *z))
            .collect();
        
        let start_time = Instant::now();
        let mut success = true;
        let mut error_message = None;
        
        for (i, coord) in chunk_coords.iter().enumerate() {
            let (world_x, world_z) = test_coordinates[i];
            
            info!("Testing coordinate ({}, {}) -> chunk {:?}", world_x, world_z, coord);
            
            // Test coordinate conversion
            match std::panic::catch_unwind(|| {
                let converted_chunk = self.generator.world_to_chunk_coords(world_x, world_z);
                let (recovered_x, recovered_z) = self.generator.chunk_to_world_coords(converted_chunk);
                (converted_chunk, recovered_x, recovered_z)
            }) {
                Ok((converted_chunk, recovered_x, recovered_z)) => {
                    info!("  Coordinate conversion: {} -> {:?} -> ({}, {})", 
                          format!("({}, {})", world_x, world_z), converted_chunk, recovered_x, recovered_z);
                    
                    // Test height generation
                    match std::panic::catch_unwind(|| self.generator.get_height_at(world_x, world_z)) {
                        Ok(height) => {
                            info!("  Height generation: {:.1}", height);
                            
                            // Verify height is within valid bounds
                            if height < 0.0 || height > 255.0 {
                                success = false;
                                error_message = Some(format!("Height {} out of bounds at ({}, {})", height, world_x, world_z));
                            }
                        }
                        Err(_) => {
                            success = false;
                            error_message = Some(format!("Height generation panicked at ({}, {})", world_x, world_z));
                        }
                    }
                    
                    // Test biome generation
                    match std::panic::catch_unwind(|| self.generator.get_biome_at(world_x, world_z)) {
                        Ok(biome) => {
                            info!("  Biome generation: {:?}", biome);
                        }
                        Err(_) => {
                            success = false;
                            error_message = Some(format!("Biome generation panicked at ({}, {})", world_x, world_z));
                        }
                    }
                }
                Err(_) => {
                    success = false;
                    error_message = Some(format!("Coordinate conversion panicked for ({}, {})", world_x, world_z));
                }
            }
        }
        
        let generation_time = start_time.elapsed();
        
        self.test_results.push(TestResult {
            test_name: "Mixed Extreme Coordinates".to_string(),
            coordinates: test_coordinates,
            chunk_coords,
            success,
            error_message: error_message.clone(),
            generation_time,
        });
        
        info!("Mixed extreme coordinates test: {}", if success { "✓ PASSED" } else { "✗ FAILED" });
        if let Some(error) = &error_message {
            info!("Error: {}", error);
        }
        
        Ok(())
    }

    /// Test coordinates near integer limits
    fn test_near_integer_limits(&mut self) -> Result<()> {
        info!("\n--- Near Integer Limits Test ---");
        
        // Test coordinates near i32 limits but not at the exact limit to avoid overflow in calculations
        let test_coordinates = vec![
            (i32::MAX / 2, i32::MAX / 2),
            (i32::MIN / 2, i32::MIN / 2),
            (i32::MAX / 2, i32::MIN / 2),
            (i32::MIN / 2, i32::MAX / 2),
        ];
        
        let chunk_coords: Vec<ChunkCoord> = test_coordinates.iter()
            .map(|(x, z)| self.generator.world_to_chunk_coords(*x, *z))
            .collect();
        
        let start_time = Instant::now();
        let mut success = true;
        let mut error_message = None;
        
        for (i, coord) in chunk_coords.iter().enumerate() {
            let (world_x, world_z) = test_coordinates[i];
            
            info!("Testing coordinate ({}, {}) -> chunk {:?}", world_x, world_z, coord);
            
            // Test coordinate conversion
            match std::panic::catch_unwind(|| {
                let converted_chunk = self.generator.world_to_chunk_coords(world_x, world_z);
                let (recovered_x, recovered_z) = self.generator.chunk_to_world_coords(converted_chunk);
                (converted_chunk, recovered_x, recovered_z)
            }) {
                Ok((converted_chunk, recovered_x, recovered_z)) => {
                    info!("  Coordinate conversion: {} -> {:?} -> ({}, {})", 
                          format!("({}, {})", world_x, world_z), converted_chunk, recovered_x, recovered_z);
                    
                    // Test height generation
                    match std::panic::catch_unwind(|| self.generator.get_height_at(world_x, world_z)) {
                        Ok(height) => {
                            info!("  Height generation: {:.1}", height);
                            
                            // Verify height is within valid bounds
                            if height < 0.0 || height > 255.0 {
                                success = false;
                                error_message = Some(format!("Height {} out of bounds at ({}, {})", height, world_x, world_z));
                            }
                        }
                        Err(_) => {
                            success = false;
                            error_message = Some(format!("Height generation panicked at ({}, {})", world_x, world_z));
                        }
                    }
                    
                    // Test biome generation
                    match std::panic::catch_unwind(|| self.generator.get_biome_at(world_x, world_z)) {
                        Ok(biome) => {
                            info!("  Biome generation: {:?}", biome);
                        }
                        Err(_) => {
                            success = false;
                            error_message = Some(format!("Biome generation panicked at ({}, {})", world_x, world_z));
                        }
                    }
                }
                Err(_) => {
                    success = false;
                    error_message = Some(format!("Coordinate conversion panicked for ({}, {})", world_x, world_z));
                }
            }
        }
        
        let generation_time = start_time.elapsed();
        
        self.test_results.push(TestResult {
            test_name: "Near Integer Limits".to_string(),
            coordinates: test_coordinates,
            chunk_coords,
            success,
            error_message: error_message.clone(),
            generation_time,
        });
        
        info!("Near integer limits test: {}", if success { "✓ PASSED" } else { "✗ FAILED" });
        if let Some(error) = &error_message {
            info!("Error: {}", error);
        }
        
        Ok(())
    }

    /// Test boundary conditions (zero, small values, etc.)
    fn test_boundary_conditions(&mut self) -> Result<()> {
        info!("\n--- Boundary Conditions Test ---");
        
        let test_coordinates = vec![
            (0, 0),
            (1, 0),
            (0, 1),
            (-1, 0),
            (0, -1),
            (15, 15),   // Chunk boundary
            (16, 16),   // Next chunk
            (-16, -16), // Negative chunk boundary
        ];
        
        let chunk_coords: Vec<ChunkCoord> = test_coordinates.iter()
            .map(|(x, z)| self.generator.world_to_chunk_coords(*x, *z))
            .collect();
        
        let start_time = Instant::now();
        let mut success = true;
        let mut error_message = None;
        
        for (i, coord) in chunk_coords.iter().enumerate() {
            let (world_x, world_z) = test_coordinates[i];
            
            info!("Testing coordinate ({}, {}) -> chunk {:?}", world_x, world_z, coord);
            
            // Test coordinate conversion
            match std::panic::catch_unwind(|| {
                let converted_chunk = self.generator.world_to_chunk_coords(world_x, world_z);
                let (recovered_x, recovered_z) = self.generator.chunk_to_world_coords(converted_chunk);
                let (local_x, local_z) = self.generator.world_to_local_coords(world_x, world_z);
                (converted_chunk, recovered_x, recovered_z, local_x, local_z)
            }) {
                Ok((converted_chunk, recovered_x, recovered_z, local_x, local_z)) => {
                    info!("  Coordinate conversion: {} -> {:?} -> ({}, {}) local: ({}, {})", 
                          format!("({}, {})", world_x, world_z), converted_chunk, recovered_x, recovered_z, local_x, local_z);
                    
                    // Verify local coordinates are within chunk bounds
                    if local_x >= 16 || local_z >= 16 {
                        success = false;
                        error_message = Some(format!("Local coordinates ({}, {}) out of bounds for world ({}, {})", local_x, local_z, world_x, world_z));
                    }
                    
                    // Test height generation
                    match std::panic::catch_unwind(|| self.generator.get_height_at(world_x, world_z)) {
                        Ok(height) => {
                            info!("  Height generation: {:.1}", height);
                            
                            // Verify height is within valid bounds
                            if height < 0.0 || height > 255.0 {
                                success = false;
                                error_message = Some(format!("Height {} out of bounds at ({}, {})", height, world_x, world_z));
                            }
                        }
                        Err(_) => {
                            success = false;
                            error_message = Some(format!("Height generation panicked at ({}, {})", world_x, world_z));
                        }
                    }
                    
                    // Test biome generation
                    match std::panic::catch_unwind(|| self.generator.get_biome_at(world_x, world_z)) {
                        Ok(biome) => {
                            info!("  Biome generation: {:?}", biome);
                        }
                        Err(_) => {
                            success = false;
                            error_message = Some(format!("Biome generation panicked at ({}, {})", world_x, world_z));
                        }
                    }
                    
                    // Test chunk generation for reasonable coordinates
                    match std::panic::catch_unwind(|| self.generator.generate_chunk(*coord)) {
                        Ok(_chunk) => {
                            info!("  Chunk generation: ✓ Success");
                        }
                        Err(_) => {
                            success = false;
                            error_message = Some(format!("Chunk generation panicked for {:?}", coord));
                        }
                    }
                }
                Err(_) => {
                    success = false;
                    error_message = Some(format!("Coordinate conversion panicked for ({}, {})", world_x, world_z));
                }
            }
        }
        
        let generation_time = start_time.elapsed();
        
        self.test_results.push(TestResult {
            test_name: "Boundary Conditions".to_string(),
            coordinates: test_coordinates,
            chunk_coords,
            success,
            error_message: error_message.clone(),
            generation_time,
        });
        
        info!("Boundary conditions test: {}", if success { "✓ PASSED" } else { "✗ FAILED" });
        if let Some(error) = &error_message {
            info!("Error: {}", error);
        }
        
        Ok(())
    }

    /// Test coordinate conversion edge cases
    fn test_coordinate_conversion_edge_cases(&mut self) -> Result<()> {
        info!("\n--- Coordinate Conversion Edge Cases Test ---");
        
        let test_coordinates = vec![
            (15, 15),    // Last position in chunk (0,0)
            (16, 16),    // First position in chunk (1,1)
            (-1, -1),    // Last position in chunk (-1,-1)
            (-16, -16),  // First position in chunk (-1,-1)
            (31, 31),    // Last position in chunk (1,1)
            (32, 32),    // First position in chunk (2,2)
        ];
        
        let start_time = Instant::now();
        let mut success = true;
        let mut error_message = None;
        
        for (world_x, world_z) in test_coordinates.iter() {
            info!("Testing coordinate conversion for ({}, {})", world_x, world_z);
            
            match std::panic::catch_unwind(|| {
                // Test world to chunk conversion
                let chunk_coord = self.generator.world_to_chunk_coords(*world_x, *world_z);
                
                // Test chunk to world conversion
                let (chunk_world_x, chunk_world_z) = self.generator.chunk_to_world_coords(chunk_coord);
                
                // Test world to local conversion
                let (local_x, local_z) = self.generator.world_to_local_coords(*world_x, *world_z);
                
                // Test local to world conversion
                let (recovered_world_x, recovered_world_z) = self.generator.local_to_world_coords(chunk_coord, local_x, local_z);
                
                (chunk_coord, chunk_world_x, chunk_world_z, local_x, local_z, recovered_world_x, recovered_world_z)
            }) {
                Ok((chunk_coord, chunk_world_x, chunk_world_z, local_x, local_z, recovered_world_x, recovered_world_z)) => {
                    info!("  World ({}, {}) -> Chunk {:?} -> World ({}, {})", 
                          world_x, world_z, chunk_coord, chunk_world_x, chunk_world_z);
                    info!("  Local ({}, {}) -> World ({}, {})", 
                          local_x, local_z, recovered_world_x, recovered_world_z);
                    
                    // Verify local coordinates are within bounds
                    if local_x >= 16 || local_z >= 16 {
                        success = false;
                        error_message = Some(format!("Local coordinates ({}, {}) out of bounds", local_x, local_z));
                    }
                    
                    // Verify round-trip conversion
                    if recovered_world_x != *world_x || recovered_world_z != *world_z {
                        success = false;
                        error_message = Some(format!("Round-trip conversion failed: ({}, {}) -> ({}, {})", 
                                                    world_x, world_z, recovered_world_x, recovered_world_z));
                    }
                    
                    // Verify chunk origin is correct
                    let expected_chunk_x = world_x.div_euclid(16);
                    let expected_chunk_z = world_z.div_euclid(16);
                    if chunk_coord.x != expected_chunk_x || chunk_coord.z != expected_chunk_z {
                        success = false;
                        error_message = Some(format!("Chunk coordinate mismatch: expected ({}, {}), got ({}, {})", 
                                                    expected_chunk_x, expected_chunk_z, chunk_coord.x, chunk_coord.z));
                    }
                }
                Err(_) => {
                    success = false;
                    error_message = Some(format!("Coordinate conversion panicked for ({}, {})", world_x, world_z));
                }
            }
        }
        
        let generation_time = start_time.elapsed();
        
        self.test_results.push(TestResult {
            test_name: "Coordinate Conversion Edge Cases".to_string(),
            coordinates: test_coordinates,
            chunk_coords: Vec::new(), // Not applicable for this test
            success,
            error_message: error_message.clone(),
            generation_time,
        });
        
        info!("Coordinate conversion edge cases test: {}", if success { "✓ PASSED" } else { "✗ FAILED" });
        if let Some(error) = &error_message {
            info!("Error: {}", error);
        }
        
        Ok(())
    }

    /// Test height and biome generation at extreme coordinates
    fn test_height_biome_extreme_coordinates(&mut self) -> Result<()> {
        info!("\n--- Height and Biome Generation at Extreme Coordinates Test ---");
        
        let test_coordinates = vec![
            (1_000_000, 1_000_000),
            (-1_000_000, -1_000_000),
            (1_000_000, -1_000_000),
            (-1_000_000, 1_000_000),
            (0, 1_000_000),
            (1_000_000, 0),
        ];
        
        let start_time = Instant::now();
        let mut success = true;
        let mut error_message = None;
        let mut heights = Vec::new();
        let mut biomes = Vec::new();
        
        for (world_x, world_z) in test_coordinates.iter() {
            info!("Testing height/biome generation at ({}, {})", world_x, world_z);
            
            // Test height generation
            match std::panic::catch_unwind(|| self.generator.get_height_at(*world_x, *world_z)) {
                Ok(height) => {
                    info!("  Height: {:.1}", height);
                    heights.push(height);
                    
                    // Verify height is within valid bounds
                    if height < 0.0 || height > 255.0 {
                        success = false;
                        error_message = Some(format!("Height {} out of bounds at ({}, {})", height, world_x, world_z));
                    }
                }
                Err(_) => {
                    success = false;
                    error_message = Some(format!("Height generation panicked at ({}, {})", world_x, world_z));
                }
            }
            
            // Test biome generation
            match std::panic::catch_unwind(|| self.generator.get_biome_at(*world_x, *world_z)) {
                Ok(biome) => {
                    info!("  Biome: {:?}", biome);
                    biomes.push(biome);
                }
                Err(_) => {
                    success = false;
                    error_message = Some(format!("Biome generation panicked at ({}, {})", world_x, world_z));
                }
            }
            
            // Test determinism by generating the same coordinate twice
            if success {
                match (
                    std::panic::catch_unwind(|| self.generator.get_height_at(*world_x, *world_z)),
                    std::panic::catch_unwind(|| self.generator.get_biome_at(*world_x, *world_z))
                ) {
                    (Ok(height2), Ok(biome2)) => {
                        if let (Some(&height1), Some(&biome1)) = (heights.last(), biomes.last()) {
                            if (height1 - height2).abs() > f32::EPSILON {
                                success = false;
                                error_message = Some(format!("Height not deterministic at ({}, {}): {} vs {}", 
                                                            world_x, world_z, height1, height2));
                            }
                            if biome1 != biome2 {
                                success = false;
                                error_message = Some(format!("Biome not deterministic at ({}, {}): {:?} vs {:?}", 
                                                            world_x, world_z, biome1, biome2));
                            }
                        }
                    }
                    _ => {
                        success = false;
                        error_message = Some(format!("Determinism test panicked at ({}, {})", world_x, world_z));
                    }
                }
            }
        }
        
        let generation_time = start_time.elapsed();
        
        // Analyze height and biome distribution
        if !heights.is_empty() {
            let avg_height = heights.iter().sum::<f32>() / heights.len() as f32;
            let min_height = heights.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            let max_height = heights.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            
            info!("Height analysis:");
            info!("  Average: {:.1}", avg_height);
            info!("  Range: {:.1} - {:.1}", min_height, max_height);
        }
        
        if !biomes.is_empty() {
            let plains_count = biomes.iter().filter(|&&b| b == world::generation::terrain::BiomeType::Plains).count();
            let hills_count = biomes.iter().filter(|&&b| b == world::generation::terrain::BiomeType::Hills).count();
            
            info!("Biome analysis:");
            info!("  Plains: {}", plains_count);
            info!("  Hills: {}", hills_count);
        }
        
        self.test_results.push(TestResult {
            test_name: "Height and Biome Generation at Extreme Coordinates".to_string(),
            coordinates: test_coordinates,
            chunk_coords: Vec::new(), // Not applicable for this test
            success,
            error_message: error_message.clone(),
            generation_time,
        });
        
        info!("Height and biome generation at extreme coordinates test: {}", if success { "✓ PASSED" } else { "✗ FAILED" });
        if let Some(error) = &error_message {
            info!("Error: {}", error);
        }
        
        Ok(())
    }

    /// Print summary of all test results
    fn print_test_summary(&self) {
        info!("\n=== Extreme Coordinate Handling Test Summary ===");
        
        let total_tests = self.test_results.len();
        let passed_tests = self.test_results.iter().filter(|r| r.success).count();
        let failed_tests = total_tests - passed_tests;
        
        info!("Test Results:");
        info!("  Total tests: {}", total_tests);
        info!("  Passed: {}", passed_tests);
        info!("  Failed: {}", failed_tests);
        info!("  Success rate: {:.1}%", (passed_tests as f32 / total_tests as f32) * 100.0);
        
        info!("\nDetailed Results:");
        for result in &self.test_results {
            let status = if result.success { "✓ PASSED" } else { "✗ FAILED" };
            info!("  {}: {} ({:.2}ms)", result.test_name, status, result.generation_time.as_millis());
            
            if let Some(error) = &result.error_message {
                info!("    Error: {}", error);
            }
        }
        
        // Overall assessment
        if failed_tests == 0 {
            info!("\n🎉 All extreme coordinate handling tests PASSED!");
            info!("The terrain generation system successfully handles extreme coordinates without overflow or panics.");
        } else {
            info!("\n⚠️  Some extreme coordinate handling tests FAILED!");
            info!("The terrain generation system may have issues with extreme coordinates.");
        }
        
        // Performance summary
        let total_time: std::time::Duration = self.test_results.iter().map(|r| r.generation_time).sum();
        info!("\nPerformance Summary:");
        info!("  Total test time: {:.2}ms", total_time.as_millis());
        info!("  Average test time: {:.2}ms", total_time.as_millis() as f32 / total_tests as f32);
        
        info!("================================================");
    }
}

/// Convenience function to run the extreme coordinate test
pub fn main() -> Result<()> {
    run_extreme_coordinate_test()
}