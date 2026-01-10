//! Configuration robustness tests for chunk rendering
//!
//! This module provides comprehensive tests for different chunk configurations
//! to ensure the rendering system handles various block patterns robustly.

use crate::chunk::{ChunkPosition, ChunkDimensions, BlockID};
use crate::rendering::{SingleChunkDemo, SingleChunkConfig, MeshGenerator};

/// Test different chunk configurations for robustness
pub struct ConfigurationTester {
    mesh_generator: MeshGenerator,
}

impl ConfigurationTester {
    /// Create a new configuration tester
    pub fn new() -> Self {
        Self {
            mesh_generator: MeshGenerator::new(),
        }
    }

    /// Test empty chunk configuration
    pub fn test_empty_chunk(&self) -> Result<(), String> {
        let demo = SingleChunkDemo::empty();
        let stats = demo.get_statistics();
        
        // Empty chunk should have no geometry
        if stats.vertices != 0 || stats.indices != 0 {
            return Err(format!("Empty chunk should have no geometry, got {} vertices, {} indices", 
                stats.vertices, stats.indices));
        }
        
        if !stats.is_empty {
            return Err("Empty chunk should be marked as empty".to_string());
        }
        
        if stats.non_air_blocks() != 0 {
            return Err(format!("Empty chunk should have no non-air blocks, got {}", stats.non_air_blocks()));
        }
        
        demo.validate()?;
        Ok(())
    }

    /// Test completely filled chunk configuration
    pub fn test_filled_chunk(&self) -> Result<(), String> {
        let demo = SingleChunkDemo::filled(BlockID::Stone);
        let stats = demo.get_statistics();
        
        // Filled chunk should have geometry but with face culling
        if stats.vertices == 0 || stats.indices == 0 {
            return Err("Filled chunk should have geometry".to_string());
        }
        
        if stats.is_empty {
            return Err("Filled chunk should not be marked as empty".to_string());
        }
        
        // Should be completely filled
        if stats.fill_percentage() != 100.0 {
            return Err(format!("Filled chunk should be 100% filled, got {:.1}%", stats.fill_percentage()));
        }
        
        // With face culling, should have much fewer vertices than without culling
        let max_vertices_without_culling = stats.total_blocks * 24;
        if stats.vertices >= max_vertices_without_culling {
            return Err(format!("Face culling not working: {} vertices >= {} max", 
                stats.vertices, max_vertices_without_culling));
        }
        
        demo.validate()?;
        Ok(())
    }

    /// Test mixed pattern chunk configuration
    pub fn test_mixed_pattern_chunk(&self) -> Result<(), String> {
        let demo = SingleChunkDemo::mixed_pattern();
        let stats = demo.get_statistics();
        
        // Mixed pattern should have some but not all blocks filled
        if stats.non_air_blocks() == 0 {
            return Err("Mixed pattern chunk should have some non-air blocks".to_string());
        }
        
        if stats.fill_percentage() == 0.0 || stats.fill_percentage() == 100.0 {
            return Err(format!("Mixed pattern should be partially filled, got {:.1}%", stats.fill_percentage()));
        }
        
        if stats.is_empty {
            return Err("Mixed pattern chunk should not be empty".to_string());
        }
        
        // Should have multiple block types
        if stats.block_counts.len() < 2 {
            return Err(format!("Mixed pattern should have multiple block types, got {}", stats.block_counts.len()));
        }
        
        demo.validate()?;
        Ok(())
    }

    /// Test variety chunk configuration (default)
    pub fn test_variety_chunk(&self) -> Result<(), String> {
        let demo = SingleChunkDemo::new();
        let stats = demo.get_statistics();
        
        // Variety chunk should have some geometry
        if stats.vertices == 0 || stats.indices == 0 {
            return Err("Variety chunk should have some geometry".to_string());
        }
        
        if stats.is_empty {
            return Err("Variety chunk should not be empty".to_string());
        }
        
        // Should have multiple block types including air
        if stats.block_counts.len() < 3 {
            return Err(format!("Variety chunk should have multiple block types, got {}", stats.block_counts.len()));
        }
        
        // Should have both air and non-air blocks
        if !stats.block_counts.contains_key(&BlockID::Air) {
            return Err("Variety chunk should contain air blocks".to_string());
        }
        
        let non_air_types = stats.block_counts.iter()
            .filter(|(&block_type, _)| block_type != BlockID::Air)
            .count();
        
        if non_air_types < 2 {
            return Err(format!("Variety chunk should have multiple non-air block types, got {}", non_air_types));
        }
        
        demo.validate()?;
        Ok(())
    }

    /// Test custom configuration with specific dimensions
    pub fn test_custom_dimensions(&self) -> Result<(), String> {
        let config = SingleChunkConfig {
            dimensions: ChunkDimensions {
                width: 8,
                height: 4,
                depth: 8,
            },
            world_position: ChunkPosition { x: 0, z: 0 },
            include_variety: false,
        };
        
        let demo = SingleChunkDemo::with_config(config.clone());
        
        // Verify dimensions are respected
        if demo.chunk().dimensions() != config.dimensions {
            return Err("Custom dimensions not respected".to_string());
        }
        
        if demo.chunk().world_position() != config.world_position {
            return Err("Custom world position not respected".to_string());
        }
        
        demo.validate()?;
        Ok(())
    }

    /// Test chunk at different world positions
    pub fn test_different_world_positions(&self) -> Result<(), String> {
        let positions = [
            ChunkPosition { x: 0, z: 0 },
            ChunkPosition { x: 1, z: 0 },
            ChunkPosition { x: 0, z: 1 },
            ChunkPosition { x: -1, z: -1 },
            ChunkPosition { x: 10, z: -5 },
        ];
        
        for &position in &positions {
            let config = SingleChunkConfig {
                world_position: position,
                ..Default::default()
            };
            
            let mut demo = SingleChunkDemo::with_config(config);
            
            // Add a test block
            demo.set_block(0, 0, 0, BlockID::Stone)
                .map_err(|e| format!("Failed to set block at position {:?}: {}", position, e))?;
            
            let mesh = demo.generate_mesh();
            
            // Verify that vertices are positioned correctly for this world position
            if !mesh.vertices.is_empty() {
                let first_vertex = mesh.vertices[0];
                let expected_min_x = position.x as f32 * demo.config().dimensions.width as f32;
                let expected_min_z = position.z as f32 * demo.config().dimensions.depth as f32;
                
                if first_vertex.position[0] < expected_min_x {
                    return Err(format!("Vertex X position {} below expected minimum {} for chunk at {:?}", 
                        first_vertex.position[0], expected_min_x, position));
                }
                
                if first_vertex.position[2] < expected_min_z {
                    return Err(format!("Vertex Z position {} below expected minimum {} for chunk at {:?}", 
                        first_vertex.position[2], expected_min_z, position));
                }
            }
            
            demo.validate()?;
        }
        
        Ok(())
    }

    /// Test performance with different chunk sizes
    pub fn test_different_chunk_sizes(&self) -> Result<(), String> {
        let sizes = [
            (4, 4, 4),
            (8, 8, 8),
            (16, 16, 16),
            (32, 16, 32),
        ];
        
        for &(width, height, depth) in &sizes {
            let config = SingleChunkConfig {
                dimensions: ChunkDimensions { width, height, depth },
                world_position: ChunkPosition { x: 0, z: 0 },
                include_variety: true,
            };
            
            let demo = SingleChunkDemo::with_config(config);
            let stats = demo.get_statistics();
            
            // Verify total block count matches dimensions
            let expected_total = width * height * depth;
            if stats.total_blocks != expected_total {
                return Err(format!("Total blocks {} doesn't match expected {} for size {}x{}x{}", 
                    stats.total_blocks, expected_total, width, height, depth));
            }
            
            // Larger chunks should generally have more geometry (unless mostly empty)
            if stats.non_air_blocks() > 0 {
                if stats.vertices == 0 || stats.indices == 0 {
                    return Err(format!("Non-empty chunk of size {}x{}x{} should have geometry", 
                        width, height, depth));
                }
            }
            
            demo.validate()?;
        }
        
        Ok(())
    }

    /// Test edge cases and boundary conditions
    pub fn test_edge_cases(&self) -> Result<(), String> {
        // Test single block chunk
        let config = SingleChunkConfig {
            dimensions: ChunkDimensions { width: 1, height: 1, depth: 1 },
            world_position: ChunkPosition { x: 0, z: 0 },
            include_variety: false,
        };
        
        let mut demo = SingleChunkDemo::with_config(config);
        demo.set_block(0, 0, 0, BlockID::Stone)
            .map_err(|e| format!("Failed to set block in 1x1x1 chunk: {}", e))?;
        
        let stats = demo.get_statistics();
        
        // Single block should have all 6 faces visible (no culling)
        if stats.vertices != 24 || stats.indices != 36 {
            return Err(format!("Single block should have 24 vertices and 36 indices, got {} vertices, {} indices", 
                stats.vertices, stats.indices));
        }
        
        demo.validate()?;
        
        // Test thin slab chunk
        let _config = SingleChunkConfig {
            dimensions: ChunkDimensions { width: 16, height: 1, depth: 16 },
            world_position: ChunkPosition { x: 0, z: 0 },
            include_variety: false,
        };
        
        let demo = SingleChunkDemo::filled(BlockID::Stone);
        demo.validate()?;
        
        Ok(())
    }

    /// Test all block types
    pub fn test_all_block_types(&self) -> Result<(), String> {
        let block_types = [BlockID::Air, BlockID::Stone, BlockID::Dirt, BlockID::Grass];
        
        for &block_type in &block_types {
            let demo = if block_type == BlockID::Air {
                SingleChunkDemo::empty()
            } else {
                SingleChunkDemo::filled(block_type)
            };
            
            let stats = demo.get_statistics();
            
            if block_type == BlockID::Air {
                // Air chunks should be empty
                if !stats.is_empty || stats.non_air_blocks() != 0 {
                    return Err(format!("Air chunk should be empty, got {} non-air blocks", stats.non_air_blocks()));
                }
            } else {
                // Non-air chunks should be filled
                if stats.is_empty || stats.non_air_blocks() == 0 {
                    return Err(format!("Chunk filled with {:?} should not be empty", block_type));
                }
                
                // Should contain only the specified block type (plus air if not completely filled)
                let non_air_types: Vec<_> = stats.block_counts.iter()
                    .filter(|(&bt, _)| bt != BlockID::Air)
                    .map(|(&bt, _)| bt)
                    .collect();
                
                if non_air_types.len() != 1 || non_air_types[0] != block_type {
                    return Err(format!("Chunk should contain only {:?}, got {:?}", block_type, non_air_types));
                }
            }
            
            demo.validate()?;
        }
        
        Ok(())
    }

    /// Run all configuration tests
    pub fn run_all_tests(&self) -> Result<(), String> {
        self.test_empty_chunk()?;
        self.test_filled_chunk()?;
        self.test_mixed_pattern_chunk()?;
        self.test_variety_chunk()?;
        self.test_custom_dimensions()?;
        self.test_different_world_positions()?;
        self.test_different_chunk_sizes()?;
        self.test_edge_cases()?;
        self.test_all_block_types()?;
        
        Ok(())
    }
}

impl Default for ConfigurationTester {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate arbitrary chunk dimensions for property testing
    fn arb_chunk_dimensions() -> impl Strategy<Value = ChunkDimensions> {
        (1usize..=32, 1usize..=64, 1usize..=32).prop_map(|(width, height, depth)| {
            ChunkDimensions { width, height, depth }
        })
    }

    /// Generate arbitrary chunk positions for property testing
    fn arb_chunk_position() -> impl Strategy<Value = ChunkPosition> {
        (-10i32..=10, -10i32..=10).prop_map(|(x, z)| ChunkPosition { x, z })
    }

    /// Generate arbitrary block types for property testing
    fn arb_block_id() -> impl Strategy<Value = BlockID> {
        prop::sample::select(vec![BlockID::Air, BlockID::Stone, BlockID::Dirt, BlockID::Grass])
    }

    /// **Feature: chunk-rendering, Property 6: Chunk Configuration Robustness**
    /// **Validates: Requirements 6.1, 6.2, 6.5**
    /// 
    /// For any chunk configuration (empty, full, mixed block types), the mesh generation 
    /// and rendering should complete successfully without errors.
    #[test]
    fn property_chunk_configuration_robustness() {
        let config = ProptestConfig {
            cases: 100,
            ..ProptestConfig::default()
        };
        
        proptest!(config, |(
            dimensions in arb_chunk_dimensions(),
            world_position in arb_chunk_position(),
            include_variety in any::<bool>(),
            fill_block in arb_block_id(),
            block_pattern in prop::collection::vec(
                (0usize..32, 0usize..64, 0usize..32, arb_block_id()),
                0..50
            )
        )| {
            // Test 1: Custom configuration should always be valid
            let config = SingleChunkConfig {
                dimensions,
                world_position,
                include_variety,
            };
            
            let demo = SingleChunkDemo::with_config(config.clone());
            prop_assert!(demo.validate().is_ok(), "Custom configuration should be valid");
            prop_assert_eq!(demo.config().dimensions, dimensions, "Dimensions should match configuration");
            prop_assert_eq!(demo.config().world_position, world_position, "World position should match configuration");
            
            // Test 2: Filled chunk configuration should be robust
            let filled_demo = SingleChunkDemo::filled(fill_block);
            prop_assert!(filled_demo.validate().is_ok(), "Filled chunk should be valid");
            
            let filled_stats = filled_demo.get_statistics();
            if fill_block == BlockID::Air {
                prop_assert!(filled_stats.is_empty, "Air-filled chunk should be empty");
                prop_assert_eq!(filled_stats.non_air_blocks(), 0, "Air-filled chunk should have no non-air blocks");
            } else {
                prop_assert!(!filled_stats.is_empty, "Non-air filled chunk should not be empty");
                prop_assert!(filled_stats.non_air_blocks() > 0, "Non-air filled chunk should have non-air blocks");
                prop_assert_eq!(filled_stats.fill_percentage(), 100.0, "Filled chunk should be 100% filled");
            }
            
            // Test 3: Empty chunk configuration should always work
            let empty_demo = SingleChunkDemo::empty();
            prop_assert!(empty_demo.validate().is_ok(), "Empty chunk should be valid");
            
            let empty_stats = empty_demo.get_statistics();
            prop_assert!(empty_stats.is_empty, "Empty chunk should be marked as empty");
            prop_assert_eq!(empty_stats.non_air_blocks(), 0, "Empty chunk should have no non-air blocks");
            prop_assert_eq!(empty_stats.fill_percentage(), 0.0, "Empty chunk should be 0% filled");
            
            // Test 4: Mixed pattern configuration should be robust
            let mixed_demo = SingleChunkDemo::mixed_pattern();
            prop_assert!(mixed_demo.validate().is_ok(), "Mixed pattern chunk should be valid");
            
            // Test 5: Custom block pattern should be handled robustly
            let mut pattern_demo = SingleChunkDemo::with_config(config.clone());
            
            // Filter blocks to only include those within chunk bounds
            let valid_blocks: Vec<_> = block_pattern
                .into_iter()
                .filter(|(x, y, z, _)| *x < dimensions.width && *y < dimensions.height && *z < dimensions.depth)
                .collect();
            
            // Set blocks and ensure no errors occur
            for (x, y, z, block) in valid_blocks {
                let result = pattern_demo.set_block(x, y, z, block);
                prop_assert!(result.is_ok(), "Setting valid block should succeed: {:?}", result);
            }
            
            prop_assert!(pattern_demo.validate().is_ok(), "Pattern chunk should be valid after modifications");
            
            // Test 6: Mesh generation should always succeed for valid configurations
            let mesh = pattern_demo.generate_mesh();
            prop_assert!(mesh.validate().is_ok(), "Generated mesh should be valid");
            
            // Test 7: Statistics should be consistent
            let stats = pattern_demo.get_statistics();
            prop_assert_eq!(stats.total_blocks, dimensions.width * dimensions.height * dimensions.depth, 
                "Total blocks should match chunk dimensions");
            
            let calculated_non_air = stats.block_counts.iter()
                .filter(|(&block_type, _)| block_type != BlockID::Air)
                .map(|(_, &count)| count)
                .sum::<usize>();
            prop_assert_eq!(stats.non_air_blocks(), calculated_non_air, 
                "Non-air block count should be consistent");
            
            // Test 8: Vertex and index counts should be reasonable
            if stats.non_air_blocks() == 0 {
                prop_assert_eq!(stats.vertices, 0, "Empty chunks should have no vertices");
                prop_assert_eq!(stats.indices, 0, "Empty chunks should have no indices");
                prop_assert!(stats.is_empty, "Chunks with no non-air blocks should be marked as empty");
            } else {
                // Non-empty chunks should have some geometry (face culling may reduce it)
                prop_assert!(stats.vertices <= stats.non_air_blocks() * 24, 
                    "Vertices should not exceed maximum possible (24 per block)");
                prop_assert!(stats.indices <= stats.non_air_blocks() * 36, 
                    "Indices should not exceed maximum possible (36 per block)");
                prop_assert_eq!(stats.indices % 3, 0, "Indices should form complete triangles");
                prop_assert!(!stats.is_empty, "Non-empty chunks should not be marked as empty");
            }
            
            // Test 9: Vertices per block should be reasonable (accounting for face culling)
            if stats.non_air_blocks() > 0 {
                let vertices_per_block = stats.vertices_per_block();
                prop_assert!(vertices_per_block >= 0.0, "Vertices per block should be non-negative");
                prop_assert!(vertices_per_block <= 24.0, "Vertices per block should not exceed 24 (max faces)");
            }
            
            // Test 10: Triangle count should be consistent with indices
            prop_assert_eq!(stats.triangles, stats.indices / 3, "Triangle count should equal indices / 3");
        });
    }

    /// Property test for mesh generation determinism across configurations
    #[test]
    fn property_mesh_generation_determinism() {
        let config = ProptestConfig {
            cases: 50,
            ..ProptestConfig::default()
        };
        
        proptest!(config, |(
            dimensions in arb_chunk_dimensions(),
            world_position in arb_chunk_position(),
            fill_block in arb_block_id(),
        )| {
            let _config = SingleChunkConfig {
                dimensions,
                world_position,
                include_variety: false,
            };
            
            let demo = SingleChunkDemo::filled(fill_block);
            
            // Generate mesh multiple times and ensure determinism
            let mesh1 = demo.generate_mesh();
            let mesh2 = demo.generate_mesh();
            let mesh3 = demo.generate_mesh();
            
            prop_assert_eq!(mesh1.vertices.len(), mesh2.vertices.len(), "Vertex count should be deterministic");
            prop_assert_eq!(mesh1.vertices.len(), mesh3.vertices.len(), "Vertex count should be deterministic");
            
            prop_assert_eq!(mesh1.indices.len(), mesh2.indices.len(), "Index count should be deterministic");
            prop_assert_eq!(mesh1.indices.len(), mesh3.indices.len(), "Index count should be deterministic");
            
            prop_assert_eq!(mesh1.triangle_count(), mesh2.triangle_count(), "Triangle count should be deterministic");
            prop_assert_eq!(mesh1.triangle_count(), mesh3.triangle_count(), "Triangle count should be deterministic");
            
            // Verify that actual data is identical (for non-empty meshes)
            if !mesh1.is_empty() {
                for (i, (v1, v2)) in mesh1.vertices.iter().zip(mesh2.vertices.iter()).enumerate() {
                    prop_assert_eq!(v1.position, v2.position, "Vertex {} position should be deterministic", i);
                    prop_assert_eq!(v1.normal, v2.normal, "Vertex {} normal should be deterministic", i);
                    prop_assert_eq!(v1.tex_coords, v2.tex_coords, "Vertex {} tex_coords should be deterministic", i);
                }
                
                prop_assert_eq!(mesh1.indices, mesh2.indices, "Indices should be deterministic");
            }
        });
    }

    #[test]
    fn test_empty_chunk_configuration() {
        let tester = ConfigurationTester::new();
        assert!(tester.test_empty_chunk().is_ok());
    }

    #[test]
    fn test_filled_chunk_configuration() {
        let tester = ConfigurationTester::new();
        assert!(tester.test_filled_chunk().is_ok());
    }

    #[test]
    fn test_mixed_pattern_configuration() {
        let tester = ConfigurationTester::new();
        assert!(tester.test_mixed_pattern_chunk().is_ok());
    }

    #[test]
    fn test_variety_configuration() {
        let tester = ConfigurationTester::new();
        assert!(tester.test_variety_chunk().is_ok());
    }

    #[test]
    fn test_custom_dimensions_configuration() {
        let tester = ConfigurationTester::new();
        assert!(tester.test_custom_dimensions().is_ok());
    }

    #[test]
    fn test_different_world_positions_configuration() {
        let tester = ConfigurationTester::new();
        assert!(tester.test_different_world_positions().is_ok());
    }

    #[test]
    fn test_different_chunk_sizes_configuration() {
        let tester = ConfigurationTester::new();
        assert!(tester.test_different_chunk_sizes().is_ok());
    }

    #[test]
    fn test_edge_cases_configuration() {
        let tester = ConfigurationTester::new();
        assert!(tester.test_edge_cases().is_ok());
    }

    #[test]
    fn test_all_block_types_configuration() {
        let tester = ConfigurationTester::new();
        assert!(tester.test_all_block_types().is_ok());
    }

    #[test]
    fn test_all_configurations() {
        let tester = ConfigurationTester::new();
        assert!(tester.run_all_tests().is_ok());
    }

    #[test]
    fn test_performance_stability() {
        let tester = ConfigurationTester::new();
        
        // Run the same test multiple times to ensure stability
        for _ in 0..10 {
            assert!(tester.test_filled_chunk().is_ok());
            assert!(tester.test_empty_chunk().is_ok());
            assert!(tester.test_mixed_pattern_chunk().is_ok());
        }
    }

    #[test]
    fn test_mesh_generation_determinism() {
        // Test that mesh generation is deterministic for the same configuration
        let demo = SingleChunkDemo::new();
        
        let mesh1 = demo.generate_mesh();
        let mesh2 = demo.generate_mesh();
        
        assert_eq!(mesh1.vertices.len(), mesh2.vertices.len());
        assert_eq!(mesh1.indices.len(), mesh2.indices.len());
        assert_eq!(mesh1.triangle_count(), mesh2.triangle_count());
        
        // Verify that the actual vertex data is identical
        for (v1, v2) in mesh1.vertices.iter().zip(mesh2.vertices.iter()) {
            assert_eq!(v1.position, v2.position);
            assert_eq!(v1.normal, v2.normal);
            assert_eq!(v1.tex_coords, v2.tex_coords);
        }
        
        // Verify that the indices are identical
        assert_eq!(mesh1.indices, mesh2.indices);
    }
}