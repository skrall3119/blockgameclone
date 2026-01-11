//! World generation, chunk management, and storage systems
//! 
//! This crate handles all world-related functionality including terrain generation,
//! chunk management, and save/load operations. Contains no rendering code.

pub mod chunk;
pub mod generation;
pub mod storage;
pub mod rendering;
pub mod world;
pub mod integration_test;

// Re-export core chunk types for convenience
pub use chunk::{
    Chunk, ChunkDimensions, ChunkPosition, BlockID, ChunkError,
    DEFAULT_DIMENSIONS, CHUNK_WIDTH, CHUNK_HEIGHT, CHUNK_DEPTH,
};

// Re-export rendering types for convenience
pub use rendering::{
    RenderError, RenderResult, ChunkMesh, ChunkVertex, CubeFace,
    MeshGenerator, FaceDirection, BufferManager, ChunkRenderer, ChunkUniforms,
};

// Re-export world integration types for convenience
pub use world::{
    World, WorldConfig, WorldError, WorldResult, ChunkCoord, ChunkEntry, ChunkState,
    LoadPattern, CoordinateSystem, PerformanceMonitor, MemoryManager,
};

// Re-export external dependencies
pub use glam;
pub use engine;