//! Single chunk rendering demonstration
//!
//! This module provides functionality to create and render a single chunk
//! with various block types as a demonstration of the chunk rendering system.

use crate::chunk::{Chunk, ChunkPosition, ChunkDimensions, BlockID};
use crate::rendering::{ChunkRenderer, ChunkUniforms, MeshGenerator, RenderResult};

/// Configuration for single chunk rendering demonstration
#[derive(Debug, Clone)]
pub struct SingleChunkConfig {
    /// Chunk dimensions
    pub dimensions: ChunkDimensions,
    /// World position for the chunk (typically origin for demonstration)
    pub world_position: ChunkPosition,
    /// Whether to include various block types for demonstration
    pub include_variety: bool,
}

impl Default for SingleChunkConfig {
    fn default() -> Self {
        Self {
            dimensions: ChunkDimensions {
                width: 16,
                height: 16,
                depth: 16,
            },
            world_position: ChunkPosition { x: 0, z: 0 },
            include_variety: true,
        }
    }
}

/// Single chunk rendering demonstration
pub struct SingleChunkDemo {
    /// The chunk being rendered
    chunk: Chunk,
    /// Mesh generator for creating renderable geometry
    mesh_generator: MeshGenerator,
    /// Configuration for the demonstration
    config: SingleChunkConfig,
}

impl SingleChunkDemo {
    /// Create a new single chunk demonstration with default configuration
    pub fn new() -> Self {
        Self::with_config(SingleChunkConfig::default())
    }

    /// Create a new single chunk demonstration with custom configuration
    pub fn with_config(config: SingleChunkConfig) -> Self {
        let mut chunk = Chunk::new(config.world_position, config.dimensions);
        let mesh_generator = MeshGenerator::new();

        // Set up the chunk with various block types for demonstration
        if config.include_variety {
            Self::setup_demonstration_blocks(&mut chunk, &config);
        }

        Self {
            chunk,
            mesh_generator,
            config,
        }
    }

    /// Create a chunk with empty configuration (all air blocks)
    pub fn empty() -> Self {
        let config = SingleChunkConfig {
            include_variety: false,
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Create a chunk completely filled with the specified block type
    pub fn filled(block_type: BlockID) -> Self {
        let config = SingleChunkConfig {
            include_variety: false,
            ..Default::default()
        };
        let mut demo = Self::with_config(config);
        demo.chunk.fill(block_type);
        demo
    }

    /// Create a chunk with mixed block types in a pattern
    pub fn mixed_pattern() -> Self {
        let config = SingleChunkConfig {
            include_variety: false,
            ..Default::default()
        };
        let mut demo = Self::with_config(config.clone());
        Self::setup_mixed_pattern(&mut demo.chunk, &config);
        demo
    }

    /// Set up demonstration blocks with various types and patterns
    fn setup_demonstration_blocks(chunk: &mut Chunk, config: &SingleChunkConfig) {
        let dims = config.dimensions;

        // Create a layered structure to demonstrate different block types
        for x in 0..dims.width {
            for z in 0..dims.depth {
                for y in 0..dims.height {
                    let block_type = match y {
                        // Bottom layer: Stone foundation
                        0..=2 => BlockID::Stone,
                        // Middle layers: Dirt
                        3..=6 => BlockID::Dirt,
                        // Top layer: Grass
                        7 => BlockID::Grass,
                        // Create some interesting patterns in upper layers
                        8..=10 => {
                            // Checkerboard pattern with stone and air
                            if (x + z) % 2 == 0 {
                                BlockID::Stone
                            } else {
                                BlockID::Air
                            }
                        }
                        // Sparse blocks in upper layers
                        11..=13 => {
                            if x % 4 == 0 && z % 4 == 0 {
                                BlockID::Dirt
                            } else {
                                BlockID::Air
                            }
                        }
                        // Top layers: mostly air with occasional blocks
                        _ => {
                            if x == dims.width / 2 && z == dims.depth / 2 {
                                BlockID::Grass
                            } else {
                                BlockID::Air
                            }
                        }
                    };

                    if let Err(e) = chunk.set_block(x, y, z, block_type) {
                        log::warn!("Failed to set block at ({}, {}, {}): {}", x, y, z, e);
                    }
                }
            }
        }
    }

    /// Set up a mixed pattern for testing different configurations
    fn setup_mixed_pattern(chunk: &mut Chunk, config: &SingleChunkConfig) {
        let dims = config.dimensions;

        // Create quarters with different block types
        for x in 0..dims.width {
            for z in 0..dims.depth {
                for y in 0..dims.height / 2 {
                    let block_type = match (x < dims.width / 2, z < dims.depth / 2) {
                        (true, true) => BlockID::Stone,   // Top-left quarter
                        (true, false) => BlockID::Dirt,   // Top-right quarter
                        (false, true) => BlockID::Grass,  // Bottom-left quarter
                        (false, false) => {
                            // Bottom-right quarter: mixed pattern
                            if (x + z + y) % 3 == 0 {
                                BlockID::Stone
                            } else {
                                BlockID::Air
                            }
                        }
                    };

                    if let Err(e) = chunk.set_block(x, y, z, block_type) {
                        log::warn!("Failed to set block at ({}, {}, {}): {}", x, y, z, e);
                    }
                }
            }
        }
    }

    /// Get a reference to the chunk
    pub fn chunk(&self) -> &Chunk {
        &self.chunk
    }

    /// Get a mutable reference to the chunk
    pub fn chunk_mut(&mut self) -> &mut Chunk {
        &mut self.chunk
    }

    /// Get the configuration
    pub fn config(&self) -> &SingleChunkConfig {
        &self.config
    }

    /// Generate mesh for the current chunk
    pub fn generate_mesh(&self) -> crate::rendering::ChunkMesh {
        self.mesh_generator.generate_chunk_mesh(&self.chunk)
    }

    /// Prepare the chunk for rendering by uploading mesh data to GPU
    pub fn prepare_for_rendering(
        &self,
        renderer: &mut ChunkRenderer,
        queue: &wgpu::Queue,
    ) -> RenderResult<()> {
        let mesh = self.generate_mesh();
        renderer.prepare_chunk(queue, self.config.world_position, &mesh)
    }

    /// Render the chunk using the provided renderer and render pass
    pub fn render(
        &self,
        renderer: &ChunkRenderer,
        render_pass: &mut wgpu::RenderPass,
        view_proj_matrix: [[f32; 4]; 4],
    ) -> RenderResult<()> {
        let mesh = self.generate_mesh();
        
        // Create uniforms for this chunk
        let chunk_world_pos = [
            self.config.world_position.x as f32 * self.config.dimensions.width as f32,
            0.0,
            self.config.world_position.z as f32 * self.config.dimensions.depth as f32,
        ];
        
        // Update uniforms for this chunk
        let _uniforms = ChunkUniforms::new(view_proj_matrix, chunk_world_pos);
        
        // Render the chunk
        renderer.render_chunk(render_pass, &self.config.world_position, &mesh)
    }

    /// Get statistics about the chunk
    pub fn get_statistics(&self) -> ChunkStatistics {
        let dims = self.config.dimensions;
        let mut block_counts = std::collections::HashMap::new();
        let mut total_blocks = 0;

        // Count blocks by type
        for x in 0..dims.width {
            for y in 0..dims.height {
                for z in 0..dims.depth {
                    if let Ok(block) = self.chunk.get_block(x, y, z) {
                        *block_counts.entry(block).or_insert(0) += 1;
                        total_blocks += 1;
                    }
                }
            }
        }

        let mesh = self.generate_mesh();

        ChunkStatistics {
            total_blocks,
            block_counts,
            vertices: mesh.vertices.len(),
            indices: mesh.indices.len(),
            triangles: mesh.triangle_count(),
            is_empty: mesh.is_empty(),
        }
    }

    /// Validate the chunk and mesh generation
    pub fn validate(&self) -> Result<(), String> {
        // Validate chunk dimensions
        let dims = self.chunk.dimensions();
        if dims.width == 0 || dims.height == 0 || dims.depth == 0 {
            return Err("Invalid chunk dimensions: all dimensions must be > 0".to_string());
        }

        // Generate mesh and validate it
        let mesh = self.generate_mesh();
        mesh.validate()?;

        // Validate that mesh generation is deterministic
        let mesh2 = self.generate_mesh();
        if mesh.vertices.len() != mesh2.vertices.len() || mesh.indices.len() != mesh2.indices.len() {
            return Err("Mesh generation is not deterministic".to_string());
        }

        Ok(())
    }

    /// Update a block in the chunk
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: BlockID) -> Result<(), crate::chunk::ChunkError> {
        self.chunk.set_block(x, y, z, block)
    }

    /// Get a block from the chunk
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> Result<BlockID, crate::chunk::ChunkError> {
        self.chunk.get_block(x, y, z)
    }

    /// Clear the chunk (set all blocks to air)
    pub fn clear(&mut self) {
        self.chunk.fill(BlockID::Air);
    }

    /// Fill the chunk with the specified block type
    pub fn fill(&mut self, block_type: BlockID) {
        self.chunk.fill(block_type);
    }
}

impl Default for SingleChunkDemo {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about a chunk for debugging and validation
#[derive(Debug, Clone)]
pub struct ChunkStatistics {
    /// Total number of blocks in the chunk
    pub total_blocks: usize,
    /// Count of each block type
    pub block_counts: std::collections::HashMap<BlockID, usize>,
    /// Number of vertices in the generated mesh
    pub vertices: usize,
    /// Number of indices in the generated mesh
    pub indices: usize,
    /// Number of triangles in the generated mesh
    pub triangles: usize,
    /// Whether the mesh is empty
    pub is_empty: bool,
}

impl ChunkStatistics {
    /// Get the number of non-air blocks
    pub fn non_air_blocks(&self) -> usize {
        self.block_counts.iter()
            .filter(|(&block_type, _)| block_type != BlockID::Air)
            .map(|(_, &count)| count)
            .sum()
    }

    /// Get the percentage of non-air blocks
    pub fn fill_percentage(&self) -> f32 {
        if self.total_blocks == 0 {
            0.0
        } else {
            (self.non_air_blocks() as f32 / self.total_blocks as f32) * 100.0
        }
    }

    /// Get the average vertices per non-air block (for face culling analysis)
    pub fn vertices_per_block(&self) -> f32 {
        let non_air = self.non_air_blocks();
        if non_air == 0 {
            0.0
        } else {
            self.vertices as f32 / non_air as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_chunk_demo_creation() {
        let demo = SingleChunkDemo::new();
        assert_eq!(demo.config.world_position, ChunkPosition { x: 0, z: 0 });
        assert_eq!(demo.config.dimensions.width, 16);
        assert_eq!(demo.config.dimensions.height, 16);
        assert_eq!(demo.config.dimensions.depth, 16);
    }

    #[test]
    fn test_empty_chunk_demo() {
        let demo = SingleChunkDemo::empty();
        let stats = demo.get_statistics();
        
        assert_eq!(stats.non_air_blocks(), 0);
        assert_eq!(stats.fill_percentage(), 0.0);
        assert!(stats.is_empty);
        assert_eq!(stats.vertices, 0);
        assert_eq!(stats.indices, 0);
    }

    #[test]
    fn test_filled_chunk_demo() {
        let demo = SingleChunkDemo::filled(BlockID::Stone);
        let stats = demo.get_statistics();
        
        assert_eq!(stats.non_air_blocks(), 16 * 16 * 16);
        assert_eq!(stats.fill_percentage(), 100.0);
        assert!(!stats.is_empty);
        assert!(stats.vertices > 0);
        assert!(stats.indices > 0);
    }

    #[test]
    fn test_mixed_pattern_chunk_demo() {
        let demo = SingleChunkDemo::mixed_pattern();
        let stats = demo.get_statistics();
        
        // Mixed pattern should have some non-air blocks but not be completely filled
        assert!(stats.non_air_blocks() > 0);
        assert!(stats.fill_percentage() > 0.0);
        assert!(stats.fill_percentage() < 100.0);
        assert!(!stats.is_empty);
    }

    #[test]
    fn test_chunk_validation() {
        let demo = SingleChunkDemo::new();
        assert!(demo.validate().is_ok());
        
        let empty_demo = SingleChunkDemo::empty();
        assert!(empty_demo.validate().is_ok());
        
        let filled_demo = SingleChunkDemo::filled(BlockID::Dirt);
        assert!(filled_demo.validate().is_ok());
    }

    #[test]
    fn test_chunk_modification() {
        let mut demo = SingleChunkDemo::empty();
        
        // Initially should be empty
        assert_eq!(demo.get_block(5, 5, 5).unwrap(), BlockID::Air);
        
        // Set a block
        demo.set_block(5, 5, 5, BlockID::Stone).unwrap();
        assert_eq!(demo.get_block(5, 5, 5).unwrap(), BlockID::Stone);
        
        // Clear the chunk
        demo.clear();
        assert_eq!(demo.get_block(5, 5, 5).unwrap(), BlockID::Air);
        
        // Fill the chunk
        demo.fill(BlockID::Grass);
        assert_eq!(demo.get_block(5, 5, 5).unwrap(), BlockID::Grass);
    }

    #[test]
    fn test_mesh_generation_consistency() {
        let demo = SingleChunkDemo::new();
        
        let mesh1 = demo.generate_mesh();
        let mesh2 = demo.generate_mesh();
        
        // Mesh generation should be deterministic
        assert_eq!(mesh1.vertices.len(), mesh2.vertices.len());
        assert_eq!(mesh1.indices.len(), mesh2.indices.len());
        assert_eq!(mesh1.triangle_count(), mesh2.triangle_count());
    }

    #[test]
    fn test_statistics_calculation() {
        let demo = SingleChunkDemo::filled(BlockID::Stone);
        let stats = demo.get_statistics();
        
        assert_eq!(stats.total_blocks, 16 * 16 * 16);
        assert_eq!(stats.block_counts.get(&BlockID::Stone), Some(&(16 * 16 * 16)));
        assert_eq!(stats.block_counts.get(&BlockID::Air), None);
        assert_eq!(stats.non_air_blocks(), 16 * 16 * 16);
        assert_eq!(stats.fill_percentage(), 100.0);
        
        // With face culling, a completely filled chunk should have fewer vertices than without culling
        let max_vertices_without_culling = 16 * 16 * 16 * 24; // 24 vertices per block
        assert!(stats.vertices < max_vertices_without_culling);
        assert!(stats.vertices > 0);
    }

    #[test]
    fn test_custom_configuration() {
        let config = SingleChunkConfig {
            dimensions: ChunkDimensions {
                width: 8,
                height: 8,
                depth: 8,
            },
            world_position: ChunkPosition { x: 2, z: -1 },
            include_variety: false,
        };
        
        let demo = SingleChunkDemo::with_config(config.clone());
        assert_eq!(demo.config.dimensions, config.dimensions);
        assert_eq!(demo.config.world_position, config.world_position);
        assert_eq!(demo.config.include_variety, config.include_variety);
    }

    #[test]
    fn test_face_culling_effectiveness() {
        // Compare empty vs filled chunk to verify face culling
        let empty_demo = SingleChunkDemo::empty();
        let filled_demo = SingleChunkDemo::filled(BlockID::Stone);
        
        let empty_stats = empty_demo.get_statistics();
        let filled_stats = filled_demo.get_statistics();
        
        // Empty chunk should have no geometry
        assert_eq!(empty_stats.vertices, 0);
        assert_eq!(empty_stats.indices, 0);
        
        // Filled chunk should have geometry but much less than without culling
        // A 16x16x16 filled chunk should only render the outer faces
        assert!(filled_stats.vertices > 0);
        assert!(filled_stats.vertices < 16 * 16 * 16 * 24); // Much less than all faces
        
        // The vertices per block should be much less than 24 due to face culling
        assert!(filled_stats.vertices_per_block() < 24.0);
    }
}