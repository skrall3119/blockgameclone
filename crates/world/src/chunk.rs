//! Chunk data structures and management

use glam::{IVec3, UVec3};

/// Chunk size constants - compile-time configurable
pub const CHUNK_SIZE_X: u32 = 16;
pub const CHUNK_SIZE_Y: u32 = 256;
pub const CHUNK_SIZE_Z: u32 = 16;
pub const CHUNK_SIZE: UVec3 = UVec3::new(CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z);

/// Total number of blocks in a chunk
pub const CHUNK_VOLUME: usize = (CHUNK_SIZE_X * CHUNK_SIZE_Y * CHUNK_SIZE_Z) as usize;

/// Chunk coordinate type
pub type ChunkCoord = IVec3;

/// Block ID type - u16 allows for 65k different block types
pub type BlockId = u16;

/// Chunk loading states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkState {
    Unloaded,
    TerrainGenerated,
    FeaturesGenerated,
    Meshed,
    Renderable,
}

/// A chunk of blocks stored as a flat array for cache efficiency
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Chunk coordinate in world space
    pub coord: ChunkCoord,
    /// Block data stored as flat array [y * CHUNK_SIZE_X * CHUNK_SIZE_Z + z * CHUNK_SIZE_X + x]
    pub blocks: Box<[BlockId; CHUNK_VOLUME]>,
    /// Current loading state
    pub state: ChunkState,
}

impl Chunk {
    /// Create a new empty chunk
    pub fn new(coord: ChunkCoord) -> Self {
        Self {
            coord,
            blocks: Box::new([0; CHUNK_VOLUME]),
            state: ChunkState::Unloaded,
        }
    }
    
    /// Get block at local chunk coordinates (0-15, 0-255, 0-15)
    pub fn get_block(&self, x: u32, y: u32, z: u32) -> BlockId {
        debug_assert!(x < CHUNK_SIZE_X && y < CHUNK_SIZE_Y && z < CHUNK_SIZE_Z);
        let index = (y * CHUNK_SIZE_X * CHUNK_SIZE_Z + z * CHUNK_SIZE_X + x) as usize;
        self.blocks[index]
    }
    
    /// Set block at local chunk coordinates
    pub fn set_block(&mut self, x: u32, y: u32, z: u32, block_id: BlockId) {
        debug_assert!(x < CHUNK_SIZE_X && y < CHUNK_SIZE_Y && z < CHUNK_SIZE_Z);
        let index = (y * CHUNK_SIZE_X * CHUNK_SIZE_Z + z * CHUNK_SIZE_X + x) as usize;
        self.blocks[index] = block_id;
    }
}