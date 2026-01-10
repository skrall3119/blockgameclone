//! Coordinate system types and transformations for world management

use glam::Vec3;
use std::hash::{Hash, Hasher};

/// 3D chunk coordinates in the world grid
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkCoord {
    /// Create a new chunk coordinate
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Create a chunk coordinate at the origin
    pub fn origin() -> Self {
        Self::new(0, 0, 0)
    }

    /// Calculate the Manhattan distance to another chunk coordinate
    pub fn manhattan_distance(&self, other: &ChunkCoord) -> u32 {
        ((self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()) as u32
    }

    /// Calculate the Euclidean distance to another chunk coordinate
    pub fn euclidean_distance(&self, other: &ChunkCoord) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        let dz = (self.z - other.z) as f32;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Get all adjacent chunk coordinates (6-connected)
    pub fn adjacent(&self) -> [ChunkCoord; 6] {
        [
            ChunkCoord::new(self.x + 1, self.y, self.z),
            ChunkCoord::new(self.x - 1, self.y, self.z),
            ChunkCoord::new(self.x, self.y + 1, self.z),
            ChunkCoord::new(self.x, self.y - 1, self.z),
            ChunkCoord::new(self.x, self.y, self.z + 1),
            ChunkCoord::new(self.x, self.y, self.z - 1),
        ]
    }
}

impl Hash for ChunkCoord {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
        self.z.hash(state);
    }
}

impl std::fmt::Display for ChunkCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// Block coordinates within a chunk (0 to chunk_size-1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockCoord {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl BlockCoord {
    /// Create a new block coordinate
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// Check if the coordinate is valid for the given chunk size
    pub fn is_valid(&self, chunk_size: u32) -> bool {
        self.x < chunk_size && self.y < chunk_size && self.z < chunk_size
    }
}

/// Coordinate system for transforming between world space and chunk space
#[derive(Debug, Clone)]
pub struct CoordinateSystem {
    /// Size of each chunk in blocks
    chunk_size: u32,
    /// Size of each chunk as a float for calculations
    chunk_size_f: f32,
}

impl CoordinateSystem {
    /// Create a new coordinate system with the given chunk size
    pub fn new(chunk_size: u32) -> Self {
        Self {
            chunk_size,
            chunk_size_f: chunk_size as f32,
        }
    }

    /// Get the chunk size
    pub fn chunk_size(&self) -> u32 {
        self.chunk_size
    }

    /// Convert world position to chunk coordinate
    pub fn world_to_chunk_coord(&self, world_pos: Vec3) -> ChunkCoord {
        ChunkCoord::new(
            (world_pos.x / self.chunk_size_f).floor() as i32,
            (world_pos.y / self.chunk_size_f).floor() as i32,
            (world_pos.z / self.chunk_size_f).floor() as i32,
        )
    }

    /// Convert chunk coordinate to world position (chunk origin)
    pub fn chunk_to_world_pos(&self, chunk_coord: ChunkCoord) -> Vec3 {
        Vec3::new(
            chunk_coord.x as f32 * self.chunk_size_f,
            chunk_coord.y as f32 * self.chunk_size_f,
            chunk_coord.z as f32 * self.chunk_size_f,
        )
    }

    /// Convert world position to chunk coordinate and local block coordinate
    pub fn world_to_local_block(&self, world_pos: Vec3) -> (ChunkCoord, BlockCoord) {
        let chunk_coord = self.world_to_chunk_coord(world_pos);
        let chunk_origin = self.chunk_to_world_pos(chunk_coord);
        let local_pos = world_pos - chunk_origin;
        
        let block_coord = BlockCoord::new(
            (local_pos.x.max(0.0) as u32).min(self.chunk_size - 1),
            (local_pos.y.max(0.0) as u32).min(self.chunk_size - 1),
            (local_pos.z.max(0.0) as u32).min(self.chunk_size - 1),
        );
        
        (chunk_coord, block_coord)
    }

    /// Get the bounding box of a chunk in world coordinates
    pub fn chunk_bounds(&self, chunk_coord: ChunkCoord) -> (Vec3, Vec3) {
        let min = self.chunk_to_world_pos(chunk_coord);
        let max = min + Vec3::splat(self.chunk_size_f);
        (min, max)
    }

    /// Check if two chunks are adjacent (share a face)
    pub fn are_adjacent(&self, coord1: ChunkCoord, coord2: ChunkCoord) -> bool {
        let dx = (coord1.x - coord2.x).abs();
        let dy = (coord1.y - coord2.y).abs();
        let dz = (coord1.z - coord2.z).abs();
        
        // Adjacent chunks differ by 1 in exactly one dimension
        (dx == 1 && dy == 0 && dz == 0) ||
        (dx == 0 && dy == 1 && dz == 0) ||
        (dx == 0 && dy == 0 && dz == 1)
    }

    /// Get all chunk coordinates within a given radius from a center
    pub fn chunks_in_radius(&self, center: ChunkCoord, radius: u32) -> Vec<ChunkCoord> {
        let mut chunks = Vec::new();
        let r = radius as i32;
        
        for x in -r..=r {
            for y in -r..=r {
                for z in -r..=r {
                    let coord = ChunkCoord::new(
                        center.x + x,
                        center.y + y,
                        center.z + z,
                    );
                    
                    if coord.manhattan_distance(&center) <= radius {
                        chunks.push(coord);
                    }
                }
            }
        }
        
        chunks
    }
}