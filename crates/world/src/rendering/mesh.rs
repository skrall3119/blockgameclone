//! Mesh data structures for chunk rendering

use bytemuck::{Pod, Zeroable};

/// Vertex data for chunk mesh rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ChunkVertex {
    /// 3D position in world space
    pub position: [f32; 3],
    /// Surface normal for lighting calculations
    pub normal: [f32; 3],
    /// Texture coordinates for block textures
    pub tex_coords: [f32; 2],
}

impl ChunkVertex {
    /// Create a new vertex with the given position, normal, and texture coordinates
    pub fn new(position: [f32; 3], normal: [f32; 3], tex_coords: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            tex_coords,
        }
    }

    /// Vertex buffer layout for wgpu
    pub const fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 3] = [
            // Position
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
            // Normal
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            },
            // Texture coordinates
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
            },
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ChunkVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

/// A single face of a cube with its vertices and indices
#[derive(Debug, Clone)]
pub struct CubeFace {
    /// The four vertices that make up this face
    pub vertices: [ChunkVertex; 4],
    /// The six indices that define the two triangles of this face
    pub indices: [u32; 6],
    /// The normal vector for this face
    pub normal: [f32; 3],
}

impl CubeFace {
    /// Create a new cube face with the given vertices, indices, and normal
    pub fn new(vertices: [ChunkVertex; 4], indices: [u32; 6], normal: [f32; 3]) -> Self {
        Self {
            vertices,
            indices,
            normal,
        }
    }
}

/// Complete mesh data for a chunk
#[derive(Debug, Clone)]
pub struct ChunkMesh {
    /// All vertices in the mesh
    pub vertices: Vec<ChunkVertex>,
    /// All indices defining triangles in the mesh
    pub indices: Vec<u32>,
    /// GPU vertex buffer (if uploaded)
    pub vertex_buffer: Option<wgpu::Buffer>,
    /// GPU index buffer (if uploaded)
    pub index_buffer: Option<wgpu::Buffer>,
    /// Number of indices to render
    pub index_count: u32,
}

impl ChunkMesh {
    /// Create a new empty chunk mesh
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
        }
    }

    /// Create a chunk mesh with pre-allocated capacity
    pub fn with_capacity(vertex_capacity: usize, index_capacity: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(vertex_capacity),
            indices: Vec::with_capacity(index_capacity),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
        }
    }

    /// Add vertices and indices to the mesh
    pub fn add_geometry(&mut self, vertices: &[ChunkVertex], indices: &[u32]) {
        let vertex_offset = self.vertices.len() as u32;
        
        self.vertices.extend_from_slice(vertices);
        
        // Adjust indices by the current vertex offset
        for &index in indices {
            self.indices.push(index + vertex_offset);
        }
        
        self.index_count = self.indices.len() as u32;
    }

    /// Add a single cube face to the mesh with proper vertex offset handling
    pub fn add_cube_face(&mut self, face: &CubeFace, position: [f32; 3]) {
        let vertex_offset = self.vertices.len() as u32;
        
        // Transform face vertices to the specified position
        let transformed_vertices: Vec<ChunkVertex> = face.vertices
            .iter()
            .map(|vertex| {
                ChunkVertex::new(
                    [
                        vertex.position[0] + position[0],
                        vertex.position[1] + position[1],
                        vertex.position[2] + position[2],
                    ],
                    vertex.normal,
                    vertex.tex_coords,
                )
            })
            .collect();
        
        self.vertices.extend(transformed_vertices);
        
        // Adjust face indices by the current vertex offset
        for &index in &face.indices {
            self.indices.push(index + vertex_offset);
        }
        
        self.index_count = self.indices.len() as u32;
    }

    /// Combine another mesh into this mesh with proper vertex offset handling
    pub fn combine_mesh(&mut self, other: &ChunkMesh) {
        if other.is_empty() {
            return;
        }
        
        let vertex_offset = self.vertices.len() as u32;
        
        // Add all vertices from the other mesh
        self.vertices.extend_from_slice(&other.vertices);
        
        // Add all indices from the other mesh, adjusting for vertex offset
        for &index in &other.indices {
            self.indices.push(index + vertex_offset);
        }
        
        self.index_count = self.indices.len() as u32;
    }

    /// Reserve additional capacity for vertices and indices
    pub fn reserve(&mut self, additional_vertices: usize, additional_indices: usize) {
        self.vertices.reserve(additional_vertices);
        self.indices.reserve(additional_indices);
    }

    /// Optimize the mesh by removing duplicate vertices (basic implementation)
    /// Note: This is a simple optimization that may not be suitable for all use cases
    pub fn optimize(&mut self) {
        // For now, we'll keep this simple and just ensure the index count is correct
        // More sophisticated optimization (like vertex deduplication) can be added later
        self.index_count = self.indices.len() as u32;
        
        // Validate that all indices are within bounds
        let vertex_count = self.vertices.len() as u32;
        for (i, &index) in self.indices.iter().enumerate() {
            if index >= vertex_count {
                panic!("Invalid index {} at position {} for {} vertices", index, i, vertex_count);
            }
        }
    }

    /// Get the number of triangles in this mesh
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Validate mesh data integrity
    pub fn validate(&self) -> Result<(), String> {
        // Check that indices are valid
        let vertex_count = self.vertices.len() as u32;
        for (i, &index) in self.indices.iter().enumerate() {
            if index >= vertex_count {
                return Err(format!("Invalid index {} at position {} for {} vertices", index, i, vertex_count));
            }
        }
        
        // Check that index count matches actual indices
        if self.index_count != self.indices.len() as u32 {
            return Err(format!("Index count mismatch: stored {} but actual {}", self.index_count, self.indices.len()));
        }
        
        // Check that indices form complete triangles
        if self.indices.len() % 3 != 0 {
            return Err(format!("Index count {} is not divisible by 3", self.indices.len()));
        }
        
        Ok(())
    }

    /// Clear all mesh data
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.vertex_buffer = None;
        self.index_buffer = None;
        self.index_count = 0;
    }

    /// Check if the mesh is empty
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() || self.indices.is_empty()
    }
}

impl Default for ChunkMesh {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_add_geometry_vertex_offset() {
        let mut mesh = ChunkMesh::new();
        
        // Add first set of vertices and indices
        let vertices1 = vec![
            ChunkVertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 1.0]),
        ];
        let indices1 = vec![0, 1, 2];
        
        mesh.add_geometry(&vertices1, &indices1);
        
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
        
        // Add second set of vertices and indices
        let vertices2 = vec![
            ChunkVertex::new([2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ChunkVertex::new([3.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
        ];
        let indices2 = vec![0, 1, 2]; // These should be offset to [3, 4, 5]
        
        mesh.add_geometry(&vertices2, &indices2);
        
        assert_eq!(mesh.vertices.len(), 5);
        assert_eq!(mesh.indices, vec![0, 1, 2, 3, 4, 5]); // Second set offset by 3
        assert_eq!(mesh.index_count, 6);
    }

    #[test]
    fn test_mesh_combine_mesh() {
        let mut mesh1 = ChunkMesh::new();
        let mut mesh2 = ChunkMesh::new();
        
        // Setup first mesh
        let vertices1 = vec![
            ChunkVertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
        ];
        let indices1 = vec![0, 1];
        mesh1.add_geometry(&vertices1, &indices1);
        
        // Setup second mesh
        let vertices2 = vec![
            ChunkVertex::new([2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ChunkVertex::new([3.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
        ];
        let indices2 = vec![0, 1];
        mesh2.add_geometry(&vertices2, &indices2);
        
        // Combine mesh2 into mesh1
        mesh1.combine_mesh(&mesh2);
        
        assert_eq!(mesh1.vertices.len(), 4);
        assert_eq!(mesh1.indices, vec![0, 1, 2, 3]); // Second mesh indices offset by 2
        assert_eq!(mesh1.index_count, 4);
    }

    #[test]
    fn test_mesh_combine_empty_mesh() {
        let mut mesh1 = ChunkMesh::new();
        let mesh2 = ChunkMesh::new(); // Empty mesh
        
        let vertices1 = vec![
            ChunkVertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
        ];
        let indices1 = vec![0];
        mesh1.add_geometry(&vertices1, &indices1);
        
        let original_vertex_count = mesh1.vertices.len();
        let original_index_count = mesh1.indices.len();
        
        mesh1.combine_mesh(&mesh2);
        
        // Should remain unchanged
        assert_eq!(mesh1.vertices.len(), original_vertex_count);
        assert_eq!(mesh1.indices.len(), original_index_count);
    }

    #[test]
    fn test_mesh_add_cube_face() {
        let mut mesh = ChunkMesh::new();
        
        // Create a simple cube face
        let vertices = [
            ChunkVertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 1.0]),
            ChunkVertex::new([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 1.0]),
        ];
        let indices = [0, 1, 2, 0, 2, 3];
        let face = CubeFace::new(vertices, indices, [0.0, 1.0, 0.0]);
        
        // Add face at position (5, 10, 15)
        mesh.add_cube_face(&face, [5.0, 10.0, 15.0]);
        
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
        
        // Verify vertices are transformed to the correct position
        for vertex in &mesh.vertices {
            let pos = vertex.position;
            assert!(pos[0] >= 5.0 && pos[0] <= 6.0); // X: [5, 6]
            assert!(pos[1] >= 10.0 && pos[1] <= 11.0); // Y: [10, 11] 
            assert!(pos[2] >= 15.0 && pos[2] <= 16.0); // Z: [15, 16]
        }
    }

    #[test]
    fn test_mesh_validation() {
        let mut mesh = ChunkMesh::new();
        
        // Valid mesh
        let vertices = vec![
            ChunkVertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 1.0]),
        ];
        let indices = vec![0, 1, 2];
        mesh.add_geometry(&vertices, &indices);
        
        assert!(mesh.validate().is_ok());
        
        // Invalid mesh - add invalid index
        mesh.indices.push(10); // Index out of bounds
        mesh.index_count = mesh.indices.len() as u32;
        
        assert!(mesh.validate().is_err());
    }

    #[test]
    fn test_mesh_triangle_count() {
        let mut mesh = ChunkMesh::new();
        
        let vertices = vec![
            ChunkVertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 1.0]),
            ChunkVertex::new([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 1.0]),
        ];
        let indices = vec![0, 1, 2, 0, 2, 3]; // 2 triangles
        mesh.add_geometry(&vertices, &indices);
        
        assert_eq!(mesh.triangle_count(), 2);
    }

    #[test]
    fn test_mesh_optimize() {
        let mut mesh = ChunkMesh::new();
        
        let vertices = vec![
            ChunkVertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 1.0]),
        ];
        let indices = vec![0, 1, 2];
        mesh.add_geometry(&vertices, &indices);
        
        // Should not panic and should maintain correct index count
        mesh.optimize();
        assert_eq!(mesh.index_count, 3);
    }

    #[test]
    fn test_mesh_reserve_capacity() {
        let mut mesh = ChunkMesh::new();
        
        // Reserve capacity
        mesh.reserve(100, 300);
        
        // Capacity should be at least what we reserved
        assert!(mesh.vertices.capacity() >= 100);
        assert!(mesh.indices.capacity() >= 300);
    }
}