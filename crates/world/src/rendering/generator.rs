//! Mesh generation system for converting chunk data to renderable geometry

use crate::chunk::{Chunk, BlockID};
use crate::rendering::{ChunkMesh, ChunkVertex, CubeFace};

/// Directions for cube faces used in mesh generation and culling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceDirection {
    PosX = 0, // Right
    NegX = 1, // Left  
    PosY = 2, // Up
    NegY = 3, // Down
    PosZ = 4, // Forward
    NegZ = 5, // Back
}

/// Mesh generator for converting chunk data into renderable geometry
pub struct MeshGenerator {
    /// Template faces for cube generation
    cube_faces: [CubeFace; 6],
}

impl MeshGenerator {
    /// Create a new mesh generator with cube face templates
    pub fn new() -> Self {
        Self {
            cube_faces: Self::create_cube_face_templates(),
        }
    }

    /// Determine if a face should be rendered based on adjacent blocks
    /// Returns true if the face is visible (exposed to air or chunk boundary)
    pub fn should_render_face(
        &self,
        chunk: &Chunk,
        x: usize,
        y: usize,
        z: usize,
        face_direction: FaceDirection,
    ) -> bool {
        let dimensions = chunk.dimensions();
        
        // Calculate the position of the adjacent block in the given face direction
        let (adj_x, adj_y, adj_z) = match face_direction {
            FaceDirection::PosX => (x + 1, y, z),     // Right
            FaceDirection::NegX => (x.wrapping_sub(1), y, z), // Left
            FaceDirection::PosY => (x, y + 1, z),     // Up
            FaceDirection::NegY => (x, y.wrapping_sub(1), z), // Down
            FaceDirection::PosZ => (x, y, z + 1),     // Forward
            FaceDirection::NegZ => (x, y, z.wrapping_sub(1)), // Back
        };
        
        // Check if adjacent position is outside chunk bounds (chunk boundary)
        if adj_x >= dimensions.width || adj_y >= dimensions.height || adj_z >= dimensions.depth {
            return true; // Face is at chunk boundary, should be rendered
        }
        
        // Handle underflow cases (when wrapping_sub results in very large numbers)
        if adj_x == usize::MAX || adj_y == usize::MAX || adj_z == usize::MAX {
            return true; // Face is at chunk boundary, should be rendered
        }
        
        // Get the adjacent block
        match chunk.get_block(adj_x, adj_y, adj_z) {
            Ok(adjacent_block) => {
                // Face should be rendered if adjacent block is Air (transparent)
                adjacent_block == BlockID::Air
            }
            Err(_) => {
                // If we can't get the adjacent block, assume it's at boundary
                true
            }
        }
    }

    /// Get a bitmask of visible faces for a block at the given coordinates
    /// Each bit represents whether a face in the corresponding direction should be rendered
    /// Bit 0 = PosX, Bit 1 = NegX, Bit 2 = PosY, Bit 3 = NegY, Bit 4 = PosZ, Bit 5 = NegZ
    pub fn get_visible_faces(&self, chunk: &Chunk, x: usize, y: usize, z: usize) -> u8 {
        let mut visible_faces = 0u8;
        
        let face_directions = [
            FaceDirection::PosX, // Bit 0
            FaceDirection::NegX, // Bit 1
            FaceDirection::PosY, // Bit 2
            FaceDirection::NegY, // Bit 3
            FaceDirection::PosZ, // Bit 4
            FaceDirection::NegZ, // Bit 5
        ];
        
        for (i, direction) in face_directions.iter().enumerate() {
            if self.should_render_face(chunk, x, y, z, *direction) {
                visible_faces |= 1 << i;
            }
        }
        
        visible_faces
    }

    /// Generate a complete chunk mesh from chunk data
    /// Iterates through all blocks in the chunk and generates geometry for non-air blocks
    /// Uses face culling to only generate visible faces for performance optimization
    pub fn generate_chunk_mesh(&self, chunk: &Chunk) -> ChunkMesh {
        let dimensions = chunk.dimensions();
        
        // Estimate capacity based on chunk size (assuming some blocks are air and some faces are culled)
        let estimated_blocks = (dimensions.width * dimensions.height * dimensions.depth) / 4;
        let estimated_faces_per_block = 3; // Assume average of 3 visible faces per block due to culling
        let estimated_vertices = estimated_blocks * estimated_faces_per_block * 4; // 4 vertices per face
        let estimated_indices = estimated_blocks * estimated_faces_per_block * 6;  // 6 indices per face
        
        let mut mesh = ChunkMesh::with_capacity(estimated_vertices, estimated_indices);
        
        // Iterate through all blocks in the chunk
        for x in 0..dimensions.width {
            for y in 0..dimensions.height {
                for z in 0..dimensions.depth {
                    // Get the block at this position
                    if let Ok(block) = chunk.get_block(x, y, z) {
                        // Skip air blocks - they don't need geometry
                        if block != BlockID::Air {
                            // Calculate world position for this block
                            let world_pos = self.calculate_block_world_position(chunk, x, y, z);
                            
                            // Get visible faces for this block using face culling
                            let visible_faces = self.get_visible_faces(chunk, x, y, z);
                            
                            // Generate geometry only for visible faces
                            self.add_block_faces(&mut mesh, world_pos, visible_faces);
                        }
                    }
                }
            }
        }
        
        mesh
    }

    /// Calculate the world position for a block within a chunk
    fn calculate_block_world_position(&self, chunk: &Chunk, x: usize, y: usize, z: usize) -> [f32; 3] {
        let chunk_pos = chunk.world_position();
        let dimensions = chunk.dimensions();
        
        [
            (chunk_pos.x as f32 * dimensions.width as f32) + x as f32,
            y as f32,
            (chunk_pos.z as f32 * dimensions.depth as f32) + z as f32,
        ]
    }

    /// Add geometry for visible faces of a block to the mesh
    /// Uses the visible_faces bitmask to determine which faces to include
    fn add_block_faces(&self, mesh: &mut ChunkMesh, position: [f32; 3], visible_faces: u8) {
        let face_directions = [
            FaceDirection::PosX, // Bit 0
            FaceDirection::NegX, // Bit 1
            FaceDirection::PosY, // Bit 2
            FaceDirection::NegY, // Bit 3
            FaceDirection::PosZ, // Bit 4
            FaceDirection::NegZ, // Bit 5
        ];
        
        for (i, direction) in face_directions.iter().enumerate() {
            // Check if this face is visible using the bitmask
            if (visible_faces & (1 << i)) != 0 {
                let face_index = *direction as usize;
                let face = &self.cube_faces[face_index];
                mesh.add_cube_face(face, position);
            }
        }
    }

    /// Generate a complete cube mesh at the given position (without face culling)
    /// Returns a mesh with 8 vertices and 12 triangles (2 per face)
    /// This method is kept for backward compatibility and testing
    pub fn generate_cube_mesh(&self, position: [f32; 3]) -> ChunkMesh {
        let mut mesh = ChunkMesh::with_capacity(24, 36); // 24 vertices, 36 indices (6 faces * 4 vertices, 6 faces * 6 indices)
        
        // Add all 6 faces of the cube (no culling)
        for face in &self.cube_faces {
            mesh.add_cube_face(face, position);
        }
        
        mesh
    }

    /// Generate a cube mesh with face culling at the given position
    /// Only generates geometry for faces specified in the visible_faces bitmask
    pub fn generate_cube_mesh_with_culling(&self, position: [f32; 3], visible_faces: u8) -> ChunkMesh {
        let mut mesh = ChunkMesh::with_capacity(24, 36); // Maximum capacity for all faces
        
        self.add_block_faces(&mut mesh, position, visible_faces);
        
        mesh
    }

    /// Get a reference to the cube face templates
    pub fn get_cube_faces(&self) -> &[CubeFace; 6] {
        &self.cube_faces
    }

    /// Create the template cube faces used for mesh generation
    fn create_cube_face_templates() -> [CubeFace; 6] {
        // Define cube face templates with correct vertices, normals, and texture coordinates
        // Each face has 4 vertices and 6 indices (2 triangles)
        
        // PosX face (Right) - facing positive X direction
        let pos_x_vertices = [
            ChunkVertex::new([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0]), // bottom-left
            ChunkVertex::new([1.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0]), // top-left
            ChunkVertex::new([1.0, 1.0, 1.0], [1.0, 0.0, 0.0], [1.0, 0.0]), // top-right
            ChunkVertex::new([1.0, 0.0, 1.0], [1.0, 0.0, 0.0], [1.0, 1.0]), // bottom-right
        ];
        let pos_x_indices = [0, 1, 2, 0, 2, 3];
        
        // NegX face (Left) - facing negative X direction
        let neg_x_vertices = [
            ChunkVertex::new([0.0, 0.0, 1.0], [-1.0, 0.0, 0.0], [0.0, 1.0]), // bottom-left
            ChunkVertex::new([0.0, 1.0, 1.0], [-1.0, 0.0, 0.0], [0.0, 0.0]), // top-left
            ChunkVertex::new([0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [1.0, 0.0]), // top-right
            ChunkVertex::new([0.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [1.0, 1.0]), // bottom-right
        ];
        let neg_x_indices = [0, 1, 2, 0, 2, 3];
        
        // PosY face (Top) - facing positive Y direction
        let pos_y_vertices = [
            ChunkVertex::new([0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0]), // bottom-left
            ChunkVertex::new([0.0, 1.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0]), // top-left
            ChunkVertex::new([1.0, 1.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0]), // top-right
            ChunkVertex::new([1.0, 1.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0]), // bottom-right
        ];
        let pos_y_indices = [0, 1, 2, 0, 2, 3];
        
        // NegY face (Bottom) - facing negative Y direction
        let neg_y_vertices = [
            ChunkVertex::new([0.0, 0.0, 1.0], [0.0, -1.0, 0.0], [0.0, 1.0]), // bottom-left
            ChunkVertex::new([0.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0]), // top-left
            ChunkVertex::new([1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0]), // top-right
            ChunkVertex::new([1.0, 0.0, 1.0], [0.0, -1.0, 0.0], [1.0, 1.0]), // bottom-right
        ];
        let neg_y_indices = [0, 1, 2, 0, 2, 3];
        
        // PosZ face (Front) - facing positive Z direction
        let pos_z_vertices = [
            ChunkVertex::new([0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 1.0]), // bottom-left
            ChunkVertex::new([0.0, 1.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0]), // top-left
            ChunkVertex::new([1.0, 1.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.0]), // top-right
            ChunkVertex::new([1.0, 0.0, 1.0], [0.0, 0.0, 1.0], [1.0, 1.0]), // bottom-right
        ];
        let pos_z_indices = [0, 1, 2, 0, 2, 3];
        
        // NegZ face (Back) - facing negative Z direction
        let neg_z_vertices = [
            ChunkVertex::new([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0]), // bottom-left
            ChunkVertex::new([1.0, 1.0, 0.0], [0.0, 0.0, -1.0], [0.0, 0.0]), // top-left
            ChunkVertex::new([0.0, 1.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0]), // top-right
            ChunkVertex::new([0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [1.0, 1.0]), // bottom-right
        ];
        let neg_z_indices = [0, 1, 2, 0, 2, 3];
        
        [
            CubeFace::new(pos_x_vertices, pos_x_indices, [1.0, 0.0, 0.0]),   // PosX
            CubeFace::new(neg_x_vertices, neg_x_indices, [-1.0, 0.0, 0.0]),  // NegX
            CubeFace::new(pos_y_vertices, pos_y_indices, [0.0, 1.0, 0.0]),   // PosY
            CubeFace::new(neg_y_vertices, neg_y_indices, [0.0, -1.0, 0.0]),  // NegY
            CubeFace::new(pos_z_vertices, pos_z_indices, [0.0, 0.0, 1.0]),   // PosZ
            CubeFace::new(neg_z_vertices, neg_z_indices, [0.0, 0.0, -1.0]),  // NegZ
        ]
    }
}

impl Default for MeshGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::{TRIANGLES_PER_CUBE, FACES_PER_CUBE};
    use crate::chunk::{Chunk, ChunkPosition, ChunkDimensions, BlockID};

    #[test]
    fn test_cube_vertex_count() {
        let generator = MeshGenerator::new();
        let mesh = generator.generate_cube_mesh([0.0, 0.0, 0.0]);
        
        // A cube should have 8 unique vertices, but our implementation creates 4 vertices per face
        // for 6 faces, resulting in 24 vertices total (some duplicated for proper normals)
        assert_eq!(mesh.vertices.len(), 24); // 6 faces * 4 vertices per face
        assert_eq!(mesh.indices.len(), 36); // 6 faces * 6 indices per face (2 triangles * 3 vertices)
    }

    #[test]
    fn test_chunk_mesh_generation_empty_chunk() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        let chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        
        let mesh = generator.generate_chunk_mesh(&chunk);
        
        // Empty chunk (all air blocks) should produce empty mesh
        assert_eq!(mesh.vertices.len(), 0);
        assert_eq!(mesh.indices.len(), 0);
        assert!(mesh.is_empty());
    }

    #[test]
    fn test_chunk_mesh_generation_single_block() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        
        // Place a single stone block at (1, 1, 1)
        chunk.set_block(1, 1, 1, BlockID::Stone).unwrap();
        
        let mesh = generator.generate_chunk_mesh(&chunk);
        
        // Single block should produce same geometry as single cube
        assert_eq!(mesh.vertices.len(), 24); // 6 faces * 4 vertices per face
        assert_eq!(mesh.indices.len(), 36);  // 6 faces * 6 indices per face
        
        // Verify that vertices are positioned correctly at (1, 1, 1)
        for vertex in &mesh.vertices {
            let pos = vertex.position;
            assert!(pos[0] >= 1.0 && pos[0] <= 2.0); // X: [1, 2]
            assert!(pos[1] >= 1.0 && pos[1] <= 2.0); // Y: [1, 2]
            assert!(pos[2] >= 1.0 && pos[2] <= 2.0); // Z: [1, 2]
        }
    }

    #[test]
    fn test_chunk_mesh_generation_multiple_blocks() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        
        // Place multiple blocks
        chunk.set_block(0, 0, 0, BlockID::Stone).unwrap();
        chunk.set_block(1, 0, 0, BlockID::Dirt).unwrap();
        chunk.set_block(0, 1, 0, BlockID::Grass).unwrap();
        
        let mesh = generator.generate_chunk_mesh(&chunk);
        
        // With face culling enabled, adjacent faces will be culled
        // Block at (0,0,0) and (1,0,0) are adjacent in X direction - 2 faces culled
        // Block at (0,0,0) and (0,1,0) are adjacent in Y direction - 2 faces culled
        // So we have: 3 blocks * 6 faces - 4 culled faces = 14 faces
        // 14 faces * 4 vertices per face = 56 vertices
        // 14 faces * 6 indices per face = 84 indices
        assert_eq!(mesh.vertices.len(), 56); // 14 faces * 4 vertices per face
        assert_eq!(mesh.indices.len(), 84);  // 14 faces * 6 indices per face
        
        // Verify mesh integrity
        assert!(mesh.validate().is_ok());
    }

    #[test]
    fn test_block_world_position_calculation() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 16, height: 256, depth: 16 };
        
        // Test chunk at origin
        let chunk_origin = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        let pos_origin = generator.calculate_block_world_position(&chunk_origin, 5, 10, 7);
        assert_eq!(pos_origin, [5.0, 10.0, 7.0]);
        
        // Test chunk at offset position
        let chunk_offset = Chunk::new(ChunkPosition { x: 2, z: -1 }, dimensions);
        let pos_offset = generator.calculate_block_world_position(&chunk_offset, 5, 10, 7);
        assert_eq!(pos_offset, [37.0, 10.0, -9.0]); // (2*16 + 5, 10, -1*16 + 7)
    }

    #[test]
    fn test_chunk_mesh_with_different_chunk_positions() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        
        // Create chunks at different world positions
        let mut chunk1 = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        let mut chunk2 = Chunk::new(ChunkPosition { x: 1, z: 1 }, dimensions);
        
        // Place identical blocks in both chunks
        chunk1.set_block(0, 0, 0, BlockID::Stone).unwrap();
        chunk2.set_block(0, 0, 0, BlockID::Stone).unwrap();
        
        let mesh1 = generator.generate_chunk_mesh(&chunk1);
        let mesh2 = generator.generate_chunk_mesh(&chunk2);
        
        // Both meshes should have same structure
        assert_eq!(mesh1.vertices.len(), mesh2.vertices.len());
        assert_eq!(mesh1.indices.len(), mesh2.indices.len());
        
        // But vertices should be at different world positions
        let pos1 = mesh1.vertices[0].position;
        let pos2 = mesh2.vertices[0].position;
        
        // Chunk2 should be offset by chunk dimensions
        assert_ne!(pos1, pos2);
        assert_eq!(pos2[0] - pos1[0], 4.0); // width offset
        assert_eq!(pos2[2] - pos1[2], 4.0); // depth offset
        assert_eq!(pos2[1], pos1[1]);       // height should be same
    }

    #[test]
    fn test_cube_triangle_count() {
        let generator = MeshGenerator::new();
        let mesh = generator.generate_cube_mesh([0.0, 0.0, 0.0]);
        
        // Each face has 2 triangles, and we have 6 faces
        let triangle_count = mesh.indices.len() / 3;
        assert_eq!(triangle_count, TRIANGLES_PER_CUBE);
        assert_eq!(triangle_count, 12); // 6 faces * 2 triangles per face
    }

    #[test]
    fn test_cube_face_count() {
        let generator = MeshGenerator::new();
        let cube_faces = generator.get_cube_faces();
        
        // Should have exactly 6 faces
        assert_eq!(cube_faces.len(), FACES_PER_CUBE);
        assert_eq!(cube_faces.len(), 6);
    }

    #[test]
    fn test_cube_face_normals() {
        let generator = MeshGenerator::new();
        let cube_faces = generator.get_cube_faces();
        
        // Test that each face has the correct normal vector
        let expected_normals = [
            [1.0, 0.0, 0.0],   // PosX (Right)
            [-1.0, 0.0, 0.0],  // NegX (Left)
            [0.0, 1.0, 0.0],   // PosY (Up)
            [0.0, -1.0, 0.0],  // NegY (Down)
            [0.0, 0.0, 1.0],   // PosZ (Forward)
            [0.0, 0.0, -1.0],  // NegZ (Back)
        ];
        
        for (i, face) in cube_faces.iter().enumerate() {
            assert_eq!(face.normal, expected_normals[i]);
            
            // Also verify that all vertices in the face have the same normal
            for vertex in &face.vertices {
                assert_eq!(vertex.normal, expected_normals[i]);
            }
        }
    }

    #[test]
    fn test_texture_coordinate_assignment() {
        let generator = MeshGenerator::new();
        let cube_faces = generator.get_cube_faces();
        
        // Each face should have texture coordinates that cover the full texture (0.0 to 1.0)
        for face in cube_faces {
            let mut has_0_0 = false;
            let mut has_1_0 = false;
            let mut has_1_1 = false;
            let mut has_0_1 = false;
            
            for vertex in &face.vertices {
                let tex_coords = vertex.tex_coords;
                
                // Texture coordinates should be in valid range [0.0, 1.0]
                assert!(tex_coords[0] >= 0.0 && tex_coords[0] <= 1.0);
                assert!(tex_coords[1] >= 0.0 && tex_coords[1] <= 1.0);
                
                // Check for the four corners of texture space
                if (tex_coords[0] - 0.0).abs() < f32::EPSILON && (tex_coords[1] - 0.0).abs() < f32::EPSILON {
                    has_0_0 = true;
                }
                if (tex_coords[0] - 1.0).abs() < f32::EPSILON && (tex_coords[1] - 0.0).abs() < f32::EPSILON {
                    has_1_0 = true;
                }
                if (tex_coords[0] - 1.0).abs() < f32::EPSILON && (tex_coords[1] - 1.0).abs() < f32::EPSILON {
                    has_1_1 = true;
                }
                if (tex_coords[0] - 0.0).abs() < f32::EPSILON && (tex_coords[1] - 1.0).abs() < f32::EPSILON {
                    has_0_1 = true;
                }
            }
            
            // Each face should have all four texture coordinate corners
            assert!(has_0_0, "Face missing texture coordinate (0,0)");
            assert!(has_1_0, "Face missing texture coordinate (1,0)");
            assert!(has_1_1, "Face missing texture coordinate (1,1)");
            assert!(has_0_1, "Face missing texture coordinate (0,1)");
        }
    }

    #[test]
    fn test_cube_mesh_at_different_positions() {
        let generator = MeshGenerator::new();
        
        // Test cube generation at different positions
        let positions = [
            [0.0, 0.0, 0.0],
            [1.0, 2.0, 3.0],
            [-5.0, 10.0, -2.0],
        ];
        
        for position in positions {
            let mesh = generator.generate_cube_mesh(position);
            
            // Verify mesh structure
            assert_eq!(mesh.vertices.len(), 24);
            assert_eq!(mesh.indices.len(), 36);
            
            // Verify that vertices are positioned correctly relative to the given position
            for vertex in &mesh.vertices {
                // Each vertex position should be within the unit cube offset by the position
                assert!(vertex.position[0] >= position[0] && vertex.position[0] <= position[0] + 1.0);
                assert!(vertex.position[1] >= position[1] && vertex.position[1] <= position[1] + 1.0);
                assert!(vertex.position[2] >= position[2] && vertex.position[2] <= position[2] + 1.0);
            }
        }
    }

    #[test]
    fn test_cube_indices_validity() {
        let generator = MeshGenerator::new();
        let mesh = generator.generate_cube_mesh([0.0, 0.0, 0.0]);
        
        // All indices should be valid (within the vertex array bounds)
        let vertex_count = mesh.vertices.len() as u32;
        for &index in &mesh.indices {
            assert!(index < vertex_count, "Index {} is out of bounds for {} vertices", index, vertex_count);
        }
        
        // Indices should form valid triangles (groups of 3)
        assert_eq!(mesh.indices.len() % 3, 0);
        
        // Each triangle should have 3 different indices (no degenerate triangles)
        for triangle in mesh.indices.chunks(3) {
            assert_ne!(triangle[0], triangle[1]);
            assert_ne!(triangle[1], triangle[2]);
            assert_ne!(triangle[0], triangle[2]);
        }
    }

    #[test]
    fn test_should_render_face_chunk_boundaries() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        
        // Place a block at the edge of the chunk
        chunk.set_block(0, 0, 0, BlockID::Stone).unwrap(); // Corner block
        chunk.set_block(3, 2, 3, BlockID::Stone).unwrap(); // Opposite corner
        
        // Test faces at chunk boundaries - should all be visible
        assert!(generator.should_render_face(&chunk, 0, 0, 0, FaceDirection::NegX)); // Left boundary
        assert!(generator.should_render_face(&chunk, 0, 0, 0, FaceDirection::NegY)); // Bottom boundary
        assert!(generator.should_render_face(&chunk, 0, 0, 0, FaceDirection::NegZ)); // Back boundary
        
        assert!(generator.should_render_face(&chunk, 3, 2, 3, FaceDirection::PosX)); // Right boundary
        assert!(generator.should_render_face(&chunk, 3, 2, 3, FaceDirection::PosY)); // Top boundary
        assert!(generator.should_render_face(&chunk, 3, 2, 3, FaceDirection::PosZ)); // Front boundary
    }

    #[test]
    fn test_should_render_face_adjacent_air_blocks() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        
        // Place a block surrounded by air
        chunk.set_block(2, 2, 2, BlockID::Stone).unwrap();
        
        // All faces should be visible because they're adjacent to air
        assert!(generator.should_render_face(&chunk, 2, 2, 2, FaceDirection::PosX));
        assert!(generator.should_render_face(&chunk, 2, 2, 2, FaceDirection::NegX));
        assert!(generator.should_render_face(&chunk, 2, 2, 2, FaceDirection::PosY));
        assert!(generator.should_render_face(&chunk, 2, 2, 2, FaceDirection::NegY));
        assert!(generator.should_render_face(&chunk, 2, 2, 2, FaceDirection::PosZ));
        assert!(generator.should_render_face(&chunk, 2, 2, 2, FaceDirection::NegZ));
    }

    #[test]
    fn test_should_render_face_adjacent_solid_blocks() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        
        // Create a 2x2x2 solid cube of blocks
        for x in 1..=2 {
            for y in 1..=2 {
                for z in 1..=2 {
                    chunk.set_block(x, y, z, BlockID::Stone).unwrap();
                }
            }
        }
        
        // Internal faces between solid blocks should not be visible
        assert!(!generator.should_render_face(&chunk, 1, 1, 1, FaceDirection::PosX)); // Adjacent to (2,1,1)
        assert!(!generator.should_render_face(&chunk, 2, 1, 1, FaceDirection::NegX)); // Adjacent to (1,1,1)
        assert!(!generator.should_render_face(&chunk, 1, 1, 1, FaceDirection::PosY)); // Adjacent to (1,2,1)
        assert!(!generator.should_render_face(&chunk, 1, 2, 1, FaceDirection::NegY)); // Adjacent to (1,1,1)
        assert!(!generator.should_render_face(&chunk, 1, 1, 1, FaceDirection::PosZ)); // Adjacent to (1,1,2)
        assert!(!generator.should_render_face(&chunk, 1, 1, 2, FaceDirection::NegZ)); // Adjacent to (1,1,1)
        
        // External faces should still be visible (adjacent to air)
        assert!(generator.should_render_face(&chunk, 1, 1, 1, FaceDirection::NegX)); // Adjacent to air at (0,1,1)
        assert!(generator.should_render_face(&chunk, 1, 1, 1, FaceDirection::NegY)); // Adjacent to air at (1,0,1)
        assert!(generator.should_render_face(&chunk, 1, 1, 1, FaceDirection::NegZ)); // Adjacent to air at (1,1,0)
    }

    #[test]
    fn test_get_visible_faces_bitmask() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        
        // Place a single block surrounded by air
        chunk.set_block(2, 2, 2, BlockID::Stone).unwrap();
        
        let visible_faces = generator.get_visible_faces(&chunk, 2, 2, 2);
        
        // All 6 faces should be visible (all bits set)
        assert_eq!(visible_faces, 0b111111); // All 6 bits set
        
        // Test individual bits
        assert_eq!(visible_faces & (1 << 0), 1 << 0); // PosX
        assert_eq!(visible_faces & (1 << 1), 1 << 1); // NegX
        assert_eq!(visible_faces & (1 << 2), 1 << 2); // PosY
        assert_eq!(visible_faces & (1 << 3), 1 << 3); // NegY
        assert_eq!(visible_faces & (1 << 4), 1 << 4); // PosZ
        assert_eq!(visible_faces & (1 << 5), 1 << 5); // NegZ
    }

    #[test]
    fn test_get_visible_faces_partial_occlusion() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        
        // Place two adjacent blocks
        chunk.set_block(1, 1, 1, BlockID::Stone).unwrap();
        chunk.set_block(2, 1, 1, BlockID::Stone).unwrap(); // Adjacent in PosX direction
        
        let visible_faces_1 = generator.get_visible_faces(&chunk, 1, 1, 1);
        let visible_faces_2 = generator.get_visible_faces(&chunk, 2, 1, 1);
        
        // Block 1 should not have PosX face visible (bit 0 should be 0)
        assert_eq!(visible_faces_1 & (1 << 0), 0); // PosX face hidden
        
        // Block 2 should not have NegX face visible (bit 1 should be 0)
        assert_eq!(visible_faces_2 & (1 << 1), 0); // NegX face hidden
        
        // Other faces should still be visible for both blocks
        assert_ne!(visible_faces_1 & (1 << 1), 0); // NegX face visible
        assert_ne!(visible_faces_1 & (1 << 2), 0); // PosY face visible
        assert_ne!(visible_faces_2 & (1 << 0), 0); // PosX face visible
        assert_ne!(visible_faces_2 & (1 << 2), 0); // PosY face visible
    }

    #[test]
    fn test_should_render_face_air_block() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        let chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        
        // Air blocks should not have any faces rendered (this test is for completeness)
        // In practice, the mesh generation should skip air blocks entirely
        let visible_faces = generator.get_visible_faces(&chunk, 1, 1, 1);
        
        // Even though all faces would be "visible" for an air block, 
        // the mesh generation should skip air blocks before calling this
        assert_eq!(visible_faces, 0b111111); // All faces would be visible if we rendered air
    }

    #[test]
    fn test_face_culling_with_different_block_types() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        
        // Place different types of solid blocks adjacent to each other
        chunk.set_block(1, 1, 1, BlockID::Stone).unwrap();
        chunk.set_block(2, 1, 1, BlockID::Dirt).unwrap();
        chunk.set_block(1, 2, 1, BlockID::Grass).unwrap();
        
        // Faces between different solid block types should still be culled
        assert!(!generator.should_render_face(&chunk, 1, 1, 1, FaceDirection::PosX)); // Stone -> Dirt
        assert!(!generator.should_render_face(&chunk, 2, 1, 1, FaceDirection::NegX)); // Dirt -> Stone
        assert!(!generator.should_render_face(&chunk, 1, 1, 1, FaceDirection::PosY)); // Stone -> Grass
        assert!(!generator.should_render_face(&chunk, 1, 2, 1, FaceDirection::NegY)); // Grass -> Stone
        
        // Faces adjacent to air should still be visible
        assert!(generator.should_render_face(&chunk, 1, 1, 1, FaceDirection::NegX)); // Stone -> Air
        assert!(generator.should_render_face(&chunk, 1, 1, 1, FaceDirection::NegZ)); // Stone -> Air
    }

    #[test]
    fn test_chunk_mesh_generation_with_face_culling() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        
        // Create a 2x2x2 solid cube of blocks
        for x in 1..=2 {
            for y in 1..=2 {
                for z in 1..=2 {
                    chunk.set_block(x, y, z, BlockID::Stone).unwrap();
                }
            }
        }
        
        let mesh = generator.generate_chunk_mesh(&chunk);
        
        // With face culling, internal faces should be removed
        // Each block in a 2x2x2 cube has some faces culled:
        // - Corner blocks: 3 faces culled (internal), 3 faces visible (external)
        // - Edge blocks: more faces culled
        // - Internal blocks: all faces culled (but there are no fully internal blocks in 2x2x2)
        
        // The exact count depends on the configuration, but it should be less than 8 blocks * 24 vertices
        let total_blocks = 8;
        let max_vertices_without_culling = total_blocks * 24; // 192 vertices
        
        // With culling, we should have fewer vertices
        assert!(mesh.vertices.len() < max_vertices_without_culling);
        assert!(mesh.vertices.len() > 0); // But still some vertices for external faces
        
        // Verify mesh integrity
        assert!(mesh.validate().is_ok());
        assert_eq!(mesh.indices.len() % 3, 0); // Should form complete triangles
    }

    #[test]
    fn test_single_block_face_culling() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        
        // Place a single block surrounded by air
        chunk.set_block(2, 2, 2, BlockID::Stone).unwrap();
        
        let mesh = generator.generate_chunk_mesh(&chunk);
        
        // Single block surrounded by air should have all 6 faces visible
        assert_eq!(mesh.vertices.len(), 24); // 6 faces * 4 vertices per face
        assert_eq!(mesh.indices.len(), 36);  // 6 faces * 6 indices per face
        
        // Should be same as generating a cube without culling
        let cube_mesh = generator.generate_cube_mesh([2.0, 2.0, 2.0]);
        assert_eq!(mesh.vertices.len(), cube_mesh.vertices.len());
        assert_eq!(mesh.indices.len(), cube_mesh.indices.len());
    }

    #[test]
    fn test_adjacent_blocks_face_culling() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        
        // Place two adjacent blocks
        chunk.set_block(1, 1, 1, BlockID::Stone).unwrap();
        chunk.set_block(2, 1, 1, BlockID::Stone).unwrap();
        
        let mesh = generator.generate_chunk_mesh(&chunk);
        
        // Two adjacent blocks should have fewer faces than two separate blocks
        // Each block loses 1 face (the adjacent face), so we lose 2 faces total
        // Expected: (2 blocks * 6 faces - 2 culled faces) * 4 vertices per face = 40 vertices
        assert_eq!(mesh.vertices.len(), 40); // 10 faces * 4 vertices per face
        assert_eq!(mesh.indices.len(), 60);  // 10 faces * 6 indices per face
        
        // Verify mesh integrity
        assert!(mesh.validate().is_ok());
    }

    #[test]
    fn test_generate_cube_mesh_with_culling() {
        let generator = MeshGenerator::new();
        
        // Test with all faces visible
        let all_faces_visible = 0b111111; // All 6 bits set
        let mesh_all = generator.generate_cube_mesh_with_culling([0.0, 0.0, 0.0], all_faces_visible);
        assert_eq!(mesh_all.vertices.len(), 24); // 6 faces * 4 vertices
        assert_eq!(mesh_all.indices.len(), 36);  // 6 faces * 6 indices
        
        // Test with no faces visible
        let no_faces_visible = 0b000000; // No bits set
        let mesh_none = generator.generate_cube_mesh_with_culling([0.0, 0.0, 0.0], no_faces_visible);
        assert_eq!(mesh_none.vertices.len(), 0);
        assert_eq!(mesh_none.indices.len(), 0);
        
        // Test with only PosX face visible (bit 0)
        let only_pos_x = 0b000001;
        let mesh_pos_x = generator.generate_cube_mesh_with_culling([0.0, 0.0, 0.0], only_pos_x);
        assert_eq!(mesh_pos_x.vertices.len(), 4); // 1 face * 4 vertices
        assert_eq!(mesh_pos_x.indices.len(), 6);  // 1 face * 6 indices
        
        // Test with multiple faces visible (PosX and NegY: bits 0 and 3)
        let pos_x_and_neg_y = 0b001001;
        let mesh_multiple = generator.generate_cube_mesh_with_culling([0.0, 0.0, 0.0], pos_x_and_neg_y);
        assert_eq!(mesh_multiple.vertices.len(), 8);  // 2 faces * 4 vertices
        assert_eq!(mesh_multiple.indices.len(), 12); // 2 faces * 6 indices
    }

    #[test]
    fn test_face_culling_performance_optimization() {
        let generator = MeshGenerator::new();
        let dimensions = ChunkDimensions { width: 4, height: 4, depth: 4 };
        let mut chunk = Chunk::new(ChunkPosition { x: 0, z: 0 }, dimensions);
        
        // Fill entire chunk with solid blocks
        for x in 0..dimensions.width {
            for y in 0..dimensions.height {
                for z in 0..dimensions.depth {
                    chunk.set_block(x, y, z, BlockID::Stone).unwrap();
                }
            }
        }
        
        let mesh = generator.generate_chunk_mesh(&chunk);
        
        // In a completely filled chunk, only the outer faces should be visible
        // A 4x4x4 chunk has 6 faces: 2 faces of 4x4 = 32 faces each, 4 faces of 4x4 = 32 faces each
        // Total outer faces: 6 * 16 = 96 faces
        let expected_faces = 96;
        let expected_vertices = expected_faces * 4;
        let expected_indices = expected_faces * 6;
        
        assert_eq!(mesh.vertices.len(), expected_vertices);
        assert_eq!(mesh.indices.len(), expected_indices);
        
        // This should be much less than without culling
        let total_blocks = dimensions.width * dimensions.height * dimensions.depth;
        let vertices_without_culling = total_blocks * 24;
        assert!(mesh.vertices.len() < vertices_without_culling);
        
        // Verify mesh integrity
        assert!(mesh.validate().is_ok());
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use crate::chunk::{ChunkDimensions, ChunkPosition};

    /// **Feature: chunk-rendering, Property 1: Cube Mesh Structure**
    /// **Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5**
    /// 
    /// For any single block cube generation, the resulting mesh should contain exactly 8 vertices, 
    /// 6 faces, and 12 triangles with correct normals and texture coordinates.
    #[test]
    fn property_cube_mesh_structure() {
        proptest!(|(
            x in -100.0f32..100.0f32,
            y in -100.0f32..100.0f32,
            z in -100.0f32..100.0f32
        )| {
            let generator = MeshGenerator::new();
            let position = [x, y, z];
            let mesh = generator.generate_cube_mesh(position);
            
            // Property 1.1: Should have 6 faces
            let cube_faces = generator.get_cube_faces();
            prop_assert_eq!(cube_faces.len(), 6);
            
            // Property 1.2: Should generate vertices (24 total, 4 per face for proper normals)
            prop_assert_eq!(mesh.vertices.len(), 24); // 6 faces * 4 vertices per face
            
            // Property 1.3: Should generate 12 triangles (2 per face)
            let triangle_count = mesh.indices.len() / 3;
            prop_assert_eq!(triangle_count, 12);
            prop_assert_eq!(mesh.indices.len(), 36); // 12 triangles * 3 indices per triangle
            
            // Property 1.4: All vertices should have valid normals (unit length)
            for vertex in &mesh.vertices {
                let normal = vertex.normal;
                let length_squared = normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2];
                let length = length_squared.sqrt();
                prop_assert!((length - 1.0).abs() < 0.001, "Normal vector should be unit length, got length {}", length);
            }
            
            // Property 1.5: All vertices should have valid texture coordinates [0.0, 1.0]
            for vertex in &mesh.vertices {
                let tex_coords = vertex.tex_coords;
                prop_assert!(tex_coords[0] >= 0.0 && tex_coords[0] <= 1.0, "Texture U coordinate {} out of range [0,1]", tex_coords[0]);
                prop_assert!(tex_coords[1] >= 0.0 && tex_coords[1] <= 1.0, "Texture V coordinate {} out of range [0,1]", tex_coords[1]);
            }
            
            // Additional structural properties
            
            // All vertices should be positioned within the unit cube at the given position
            for vertex in &mesh.vertices {
                let pos = vertex.position;
                prop_assert!(pos[0] >= position[0] && pos[0] <= position[0] + 1.0, "Vertex X position {} outside cube bounds [{}, {}]", pos[0], position[0], position[0] + 1.0);
                prop_assert!(pos[1] >= position[1] && pos[1] <= position[1] + 1.0, "Vertex Y position {} outside cube bounds [{}, {}]", pos[1], position[1], position[1] + 1.0);
                prop_assert!(pos[2] >= position[2] && pos[2] <= position[2] + 1.0, "Vertex Z position {} outside cube bounds [{}, {}]", pos[2], position[2], position[2] + 1.0);
            }
            
            // All indices should be valid
            let vertex_count = mesh.vertices.len() as u32;
            for &index in &mesh.indices {
                prop_assert!(index < vertex_count, "Index {} out of bounds for {} vertices", index, vertex_count);
            }
            
            // Indices should form valid triangles (no degenerate triangles)
            for triangle in mesh.indices.chunks(3) {
                prop_assert_ne!(triangle[0], triangle[1], "Degenerate triangle with duplicate indices");
                prop_assert_ne!(triangle[1], triangle[2], "Degenerate triangle with duplicate indices");
                prop_assert_ne!(triangle[0], triangle[2], "Degenerate triangle with duplicate indices");
            }
            
            // Each face should have exactly one of the 6 standard normals
            let expected_normals = [
                [1.0, 0.0, 0.0],   // PosX
                [-1.0, 0.0, 0.0],  // NegX
                [0.0, 1.0, 0.0],   // PosY
                [0.0, -1.0, 0.0],  // NegY
                [0.0, 0.0, 1.0],   // PosZ
                [0.0, 0.0, -1.0],  // NegZ
            ];
            
            // Count occurrences of each normal (should be 4 vertices per normal)
            for expected_normal in &expected_normals {
                let count = mesh.vertices.iter()
                    .filter(|v| {
                        (v.normal[0] - expected_normal[0]).abs() < 0.001 &&
                        (v.normal[1] - expected_normal[1]).abs() < 0.001 &&
                        (v.normal[2] - expected_normal[2]).abs() < 0.001
                    })
                    .count();
                prop_assert_eq!(count, 4, "Expected 4 vertices with normal {:?}, found {}", expected_normal, count);
            }
        });
    }

    /// **Feature: chunk-rendering, Property 2: Chunk Mesh Assembly**
    /// **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5**
    /// 
    /// For any chunk with non-air blocks, generating a chunk mesh should process all non-air blocks 
    /// and position them correctly in world coordinates within a single combined mesh.
    #[test]
    fn property_chunk_mesh_assembly() {
        proptest!(|(
            chunk_x in -10i32..10i32,
            chunk_z in -10i32..10i32,
            width in 2usize..8usize,
            height in 2usize..8usize,
            depth in 2usize..8usize,
            blocks in prop::collection::vec(
                (0usize..8, 0usize..8, 0usize..8, prop::sample::select(vec![BlockID::Air, BlockID::Stone, BlockID::Dirt, BlockID::Grass])),
                0..20
            )
        )| {
            let generator = MeshGenerator::new();
            let dimensions = ChunkDimensions { width, height, depth };
            let chunk_pos = ChunkPosition { x: chunk_x, z: chunk_z };
            let mut chunk = Chunk::new(chunk_pos, dimensions);
            
            // Filter blocks to only include those within chunk bounds
            let valid_blocks: Vec<_> = blocks
                .into_iter()
                .filter(|(x, y, z, _)| *x < width && *y < height && *z < depth)
                .collect();
            
            // Set blocks in chunk and track final state (accounting for overwrites)
            for (x, y, z, block) in &valid_blocks {
                chunk.set_block(*x, *y, *z, *block).unwrap();
            }
            
            // Count actual non-air blocks in the final chunk state
            let mut actual_non_air_count = 0;
            for x in 0..width {
                for y in 0..height {
                    for z in 0..depth {
                        if let Ok(block) = chunk.get_block(x, y, z) {
                            if block != BlockID::Air {
                                actual_non_air_count += 1;
                            }
                        }
                    }
                }
            }
            
            let mesh = generator.generate_chunk_mesh(&chunk);
            
            // Property 2.1: Should process all non-air blocks
            if actual_non_air_count == 0 {
                // Empty chunk should produce empty mesh
                prop_assert!(mesh.is_empty(), "Empty chunk should produce empty mesh");
                prop_assert_eq!(mesh.vertices.len(), 0);
                prop_assert_eq!(mesh.indices.len(), 0);
            } else {
                // Non-empty chunk should produce non-empty mesh
                prop_assert!(!mesh.is_empty(), "Non-empty chunk should produce non-empty mesh");
                
                // Property 2.2: With face culling, each non-air block contributes <= 24 vertices and <= 36 indices
                // (some faces may be culled)
                prop_assert!(mesh.vertices.len() <= actual_non_air_count * 24, "Expected <= {} vertices for {} blocks, got {}", actual_non_air_count * 24, actual_non_air_count, mesh.vertices.len());
                prop_assert!(mesh.indices.len() <= actual_non_air_count * 36, "Expected <= {} indices for {} blocks, got {}", actual_non_air_count * 36, actual_non_air_count, mesh.indices.len());
                
                // Should have at least some geometry for non-air blocks
                prop_assert!(mesh.vertices.len() > 0, "Non-empty chunk should have some vertices");
                prop_assert!(mesh.indices.len() > 0, "Non-empty chunk should have some indices");
            }
            
            // Property 2.3: All vertices should be positioned correctly in world coordinates
            for vertex in &mesh.vertices {
                let pos = vertex.position;
                
                // Calculate expected world coordinate bounds for this chunk
                let min_x = chunk_x as f32 * width as f32;
                let max_x = min_x + width as f32;
                let min_y = 0.0;
                let max_y = height as f32;
                let min_z = chunk_z as f32 * depth as f32;
                let max_z = min_z + depth as f32;
                
                prop_assert!(pos[0] >= min_x && pos[0] <= max_x, "Vertex X position {} outside chunk bounds [{}, {}]", pos[0], min_x, max_x);
                prop_assert!(pos[1] >= min_y && pos[1] <= max_y, "Vertex Y position {} outside chunk bounds [{}, {}]", pos[1], min_y, max_y);
                prop_assert!(pos[2] >= min_z && pos[2] <= max_z, "Vertex Z position {} outside chunk bounds [{}, {}]", pos[2], min_z, max_z);
            }
            
            // Property 2.4: All indices should be valid and form complete triangles
            let vertex_count = mesh.vertices.len() as u32;
            for &index in &mesh.indices {
                prop_assert!(index < vertex_count, "Index {} out of bounds for {} vertices", index, vertex_count);
            }
            
            // Property 2.5: Mesh should be compatible with rendering pipeline
            prop_assert_eq!(mesh.indices.len() % 3, 0, "Index count {} should be divisible by 3 for triangles", mesh.indices.len());
            prop_assert_eq!(mesh.index_count, mesh.indices.len() as u32, "Index count mismatch: stored {} but actual {}", mesh.index_count, mesh.indices.len());
            
            // Validate mesh integrity
            prop_assert!(mesh.validate().is_ok(), "Mesh validation failed: {:?}", mesh.validate());
            
            // All vertices should have valid normals and texture coordinates
            for vertex in &mesh.vertices {
                let normal = vertex.normal;
                let length_squared = normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2];
                let length = length_squared.sqrt();
                prop_assert!((length - 1.0).abs() < 0.001, "Normal vector should be unit length, got length {}", length);
                
                let tex_coords = vertex.tex_coords;
                prop_assert!(tex_coords[0] >= 0.0 && tex_coords[0] <= 1.0, "Texture U coordinate {} out of range [0,1]", tex_coords[0]);
                prop_assert!(tex_coords[1] >= 0.0 && tex_coords[1] <= 1.0, "Texture V coordinate {} out of range [0,1]", tex_coords[1]);
            }
        });
    }

    /// **Feature: chunk-rendering, Property 3: Face Culling Correctness**
    /// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5**
    /// 
    /// For any two adjacent solid blocks, the faces between them should not be included in the generated mesh, 
    /// while faces exposed to air or chunk boundaries should be included.
    #[test]
    fn property_face_culling_correctness() {
        proptest!(|(
            width in 3usize..8usize,
            height in 3usize..8usize,
            depth in 3usize..8usize,
            blocks in prop::collection::vec(
                (1usize..7, 1usize..7, 1usize..7, prop::sample::select(vec![BlockID::Stone, BlockID::Dirt, BlockID::Grass])),
                1..15
            )
        )| {
            let generator = MeshGenerator::new();
            let dimensions = ChunkDimensions { width, height, depth };
            let chunk_pos = ChunkPosition { x: 0, z: 0 };
            let mut chunk = Chunk::new(chunk_pos, dimensions);
            
            // Filter blocks to only include those within chunk bounds (leaving border for boundary testing)
            let valid_blocks: Vec<_> = blocks
                .into_iter()
                .filter(|(x, y, z, _)| *x < width - 1 && *y < height - 1 && *z < depth - 1)
                .collect();
            
            // Set blocks in chunk
            for (x, y, z, block) in &valid_blocks {
                chunk.set_block(*x, *y, *z, *block).unwrap();
            }
            
            // Test face culling properties for each block
            for (x, y, z, _block) in &valid_blocks {
                let x = *x;
                let y = *y;
                let z = *z;
                
                // Property 3.1 & 3.2: Adjacent solid blocks should have faces culled between them
                let face_directions = [
                    (FaceDirection::PosX, x + 1, y, z),     // Right
                    (FaceDirection::NegX, x.wrapping_sub(1), y, z),     // Left
                    (FaceDirection::PosY, x, y + 1, z),     // Up
                    (FaceDirection::NegY, x, y.wrapping_sub(1), z),     // Down
                    (FaceDirection::PosZ, x, y, z + 1),     // Forward
                    (FaceDirection::NegZ, x, y, z.wrapping_sub(1)),     // Back
                ];
                
                for (face_dir, adj_x, adj_y, adj_z) in face_directions {
                    // Skip underflow cases
                    if adj_x == usize::MAX || adj_y == usize::MAX || adj_z == usize::MAX {
                        // Property 3.4: Faces at chunk boundaries should be visible
                        let should_render = generator.should_render_face(&chunk, x, y, z, face_dir);
                        prop_assert!(should_render, "Face {:?} at ({},{},{}) should be visible at chunk boundary", face_dir, x, y, z);
                        continue;
                    }
                    
                    if adj_x < width && adj_y < height && adj_z < depth {
                        if let Ok(adjacent_block) = chunk.get_block(adj_x, adj_y, adj_z) {
                            let should_render = generator.should_render_face(&chunk, x, y, z, face_dir);
                            
                            if adjacent_block == BlockID::Air {
                                // Property 3.5: Faces adjacent to air should be visible
                                prop_assert!(should_render, "Face {:?} at ({},{},{}) should be visible when adjacent to air at ({},{},{})", face_dir, x, y, z, adj_x, adj_y, adj_z);
                            } else {
                                // Property 3.1: Faces between solid blocks should be culled
                                prop_assert!(!should_render, "Face {:?} at ({},{},{}) should be culled when adjacent to solid block at ({},{},{})", face_dir, x, y, z, adj_x, adj_y, adj_z);
                            }
                        }
                    } else {
                        // Property 3.4: Faces at chunk boundaries should be visible
                        let should_render = generator.should_render_face(&chunk, x, y, z, face_dir);
                        prop_assert!(should_render, "Face {:?} at ({},{},{}) should be visible at chunk boundary", face_dir, x, y, z);
                    }
                }
                
                // Property 3.3: Check all 6 directions for face visibility determination
                let visible_faces = generator.get_visible_faces(&chunk, x, y, z);
                
                // Visible faces bitmask should be consistent with individual face checks
                let face_directions_ordered = [
                    FaceDirection::PosX, // Bit 0
                    FaceDirection::NegX, // Bit 1
                    FaceDirection::PosY, // Bit 2
                    FaceDirection::NegY, // Bit 3
                    FaceDirection::PosZ, // Bit 4
                    FaceDirection::NegZ, // Bit 5
                ];
                
                for (i, face_dir) in face_directions_ordered.iter().enumerate() {
                    let should_render = generator.should_render_face(&chunk, x, y, z, *face_dir);
                    let bit_set = (visible_faces & (1 << i)) != 0;
                    prop_assert_eq!(should_render, bit_set, "Face visibility mismatch for {:?} at ({},{},{}): individual check = {}, bitmask bit {} = {}", face_dir, x, y, z, should_render, i, bit_set);
                }
            }
            
            // Generate mesh and verify face culling is applied
            let mesh = generator.generate_chunk_mesh(&chunk);
            
            if !valid_blocks.is_empty() {
                // Mesh should have some geometry but less than without culling
                prop_assert!(mesh.vertices.len() > 0, "Non-empty chunk should produce some geometry");
                
                // With face culling, should have fewer vertices than maximum possible
                let max_vertices_without_culling = valid_blocks.len() * 24;
                prop_assert!(mesh.vertices.len() <= max_vertices_without_culling, "Face culling should not increase vertex count: got {} vertices for {} blocks (max {})", mesh.vertices.len(), valid_blocks.len(), max_vertices_without_culling);
                
                // Verify mesh integrity
                prop_assert!(mesh.validate().is_ok(), "Mesh with face culling should be valid");
            }
        });
    }
}