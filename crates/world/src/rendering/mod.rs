//! Chunk rendering system for converting chunk data into GPU-renderable meshes
//!
//! This module provides the functionality to transform chunk block data into
//! optimized 3D meshes suitable for GPU rendering. It handles mesh generation,
//! face culling optimization, and GPU buffer management.

pub mod error;
pub mod mesh;
pub mod generator;
pub mod buffer;
pub mod renderer;
pub mod example;
pub mod configuration_tests;

// Re-export core types for convenience
pub use error::{RenderError, RenderResult};
pub use mesh::{ChunkMesh, ChunkVertex, CubeFace};
pub use generator::{MeshGenerator, FaceDirection};
pub use buffer::{BufferManager, BufferMemoryStats};
pub use renderer::{ChunkRenderer, ChunkUniforms, ChunkRenderStats};
pub use example::{SingleChunkDemo, SingleChunkConfig, ChunkStatistics};
pub use configuration_tests::ConfigurationTester;

// Rendering constants
pub const VERTICES_PER_CUBE: usize = 8;
pub const TRIANGLES_PER_CUBE: usize = 12;
pub const FACES_PER_CUBE: usize = 6;
pub const VERTICES_PER_FACE: usize = 4;
pub const INDICES_PER_FACE: usize = 6;

/// Face directions for cube mesh generation and culling
pub const FACE_DIRECTIONS: [FaceDirection; 6] = [
    FaceDirection::PosX, // Right
    FaceDirection::NegX, // Left
    FaceDirection::PosY, // Up
    FaceDirection::NegY, // Down
    FaceDirection::PosZ, // Forward
    FaceDirection::NegZ, // Back
];