# Design Document: Chunk Data Model

## Overview

The chunk data model system provides the core data structures for representing voxel world data in fixed-size 3D regions. This system is designed around cache-friendly flat array storage, type-safe block identification, and efficient coordinate conversion utilities. The design prioritizes performance through data-oriented principles while maintaining safety through Rust's type system.

The system consists of three primary components: the Chunk struct for storing block data, the BlockID enum for type-safe block identification, and coordinate conversion utilities for translating between 3D positions and flat array indices.

## Architecture

The chunk data model follows a layered architecture within the world crate:

```
┌─────────────────────────────────────┐
│           Game Layer                │
│    (uses chunks via world API)     │
└─────────────────────────────────────┘
                    │
┌─────────────────────────────────────┐
│          World Layer                │
│  ┌─────────────┐ ┌─────────────────┐│
│  │   Chunk     │ │ Chunk Manager   ││
│  │   Manager   │ │   (future)      ││
│  └─────────────┘ └─────────────────┘│
└─────────────────────────────────────┘
                    │
┌─────────────────────────────────────┐
│        Chunk Data Model             │
│  ┌─────────────┐ ┌─────────────────┐│
│  │   Chunk     │ │    BlockID      ││
│  │   Struct    │ │     Enum        ││
│  └─────────────┘ └─────────────────┘│
└─────────────────────────────────────┘
```

The chunk data model sits at the foundation, providing data structures that higher-level systems build upon. It has no dependencies on rendering or game logic, maintaining clean separation of concerns.

## Components and Interfaces

### Chunk Struct

The core data structure representing a fixed-size 3D region of blocks:

```rust
pub struct Chunk {
    blocks: Vec<BlockID>,
    world_position: ChunkPosition,
    dimensions: ChunkDimensions,
}

pub struct ChunkPosition {
    pub x: i32,
    pub z: i32,
}

pub struct ChunkDimensions {
    pub width: usize,    // x-axis
    pub height: usize,   // y-axis  
    pub depth: usize,    // z-axis
}
```

**Key Methods:**
- `new(world_position: ChunkPosition, dimensions: ChunkDimensions) -> Self`
- `get_block(&self, x: usize, y: usize, z: usize) -> Result<BlockID, ChunkError>`
- `set_block(&mut self, x: usize, y: usize, z: usize, block: BlockID) -> Result<(), ChunkError>`
- `fill(&mut self, block: BlockID)` - Fill entire chunk with specified block
- `from_data(data: Vec<BlockID>, world_position: ChunkPosition, dimensions: ChunkDimensions) -> Result<Self, ChunkError>`

### BlockID Enum

Type-safe identifier for different block types:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum BlockID {
    Air = 0,
    Stone = 1,
    Dirt = 2,
    Grass = 3,
}
```

The enum uses `#[repr(u16)]` for efficient memory usage while allowing for 65,536 different block types. The explicit discriminant values ensure stable serialization.

**Key Methods:**
- `from_u16(value: u16) -> Option<Self>` - Safe conversion from integer
- `to_u16(self) -> u16` - Convert to integer for serialization

### Coordinate Conversion Utilities

Efficient conversion between 3D coordinates and flat array indices:

```rust
impl Chunk {
    fn coords_to_index(&self, x: usize, y: usize, z: usize) -> Result<usize, ChunkError> {
        if x >= self.dimensions.width || y >= self.dimensions.height || z >= self.dimensions.depth {
            return Err(ChunkError::OutOfBounds);
        }
        
        // Row-major order: x + y * width + z * width * height
        Ok(x + y * self.dimensions.width + z * self.dimensions.width * self.dimensions.height)
    }
    
    fn index_to_coords(&self, index: usize) -> Result<(usize, usize, usize), ChunkError> {
        let total_size = self.dimensions.width * self.dimensions.height * self.dimensions.depth;
        if index >= total_size {
            return Err(ChunkError::OutOfBounds);
        }
        
        let z = index / (self.dimensions.width * self.dimensions.height);
        let remainder = index % (self.dimensions.width * self.dimensions.height);
        let y = remainder / self.dimensions.width;
        let x = remainder % self.dimensions.width;
        
        Ok((x, y, z))
    }
}
```

### Error Handling

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkError {
    OutOfBounds,
    InvalidDimensions,
    InvalidBlockData,
}
```

## Data Models

### Memory Layout

The chunk uses a flat `Vec<BlockID>` for storage, providing several advantages:

1. **Cache Efficiency**: Contiguous memory layout improves cache performance
2. **Predictable Access**: O(1) access time via direct indexing
3. **Memory Efficiency**: No pointer overhead from nested data structures
4. **SIMD Friendly**: Flat arrays enable vectorized operations

### Coordinate System

The system uses a right-handed coordinate system:
- **X-axis**: West (-) to East (+)
- **Y-axis**: Down (-) to Up (+) 
- **Z-axis**: North (-) to South (+)

### Index Mapping

The flat array uses row-major ordering for optimal cache locality when iterating through common access patterns:

```
index = x + y * width + z * width * height
```

This ordering prioritizes:
1. X-axis iteration (most cache-friendly)
2. Y-axis iteration (moderate cache performance)
3. Z-axis iteration (least cache-friendly but still reasonable)

### Chunk Dimensions

The system supports configurable chunk dimensions through compile-time constants:

```rust
pub const CHUNK_WIDTH: usize = 16;   // or 32
pub const CHUNK_HEIGHT: usize = 256; // or 128  
pub const CHUNK_DEPTH: usize = 16;   // or 32

pub const DEFAULT_DIMENSIONS: ChunkDimensions = ChunkDimensions {
    width: CHUNK_WIDTH,
    height: CHUNK_HEIGHT,
    depth: CHUNK_DEPTH,
};
```

### Block Storage

Each block is stored as a single `BlockID` enum value (2 bytes with `u16` representation). For a 16×16×256 chunk:
- Total blocks: 65,536
- Memory usage: 131,072 bytes (128 KB per chunk)
- Cache lines: ~2,048 cache lines (assuming 64-byte cache lines)

This provides excellent memory density while maintaining type safety.

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Coordinate Round-Trip Consistency
*For any* valid 3D coordinates within chunk bounds, converting to flat array index and back to coordinates should produce the original coordinates.
**Validates: Requirements 4.1, 4.2**

### Property 2: Chunk Dimensions Consistency  
*For any* chunk created with specific dimensions, the chunk should store exactly width × height × depth blocks and maintain those dimensions.
**Validates: Requirements 1.1, 1.4**

### Property 3: Block Storage and Retrieval
*For any* valid coordinates and block type, setting a block at those coordinates and then getting the block should return the same block type.
**Validates: Requirements 2.1, 6.1, 6.2**

### Property 4: Bounds Validation
*For any* coordinates outside chunk dimensions, all coordinate-based operations (get, set, coordinate conversion) should return appropriate errors rather than accessing invalid memory.
**Validates: Requirements 2.4, 2.5, 4.3, 4.5, 6.3, 6.4**

### Property 5: Default Initialization
*For any* newly created chunk, all blocks should be initialized to Air unless explicitly specified otherwise.
**Validates: Requirements 5.1**

### Property 6: World Position Persistence
*For any* chunk created with specific world coordinates, the chunk should maintain those coordinates throughout its lifetime.
**Validates: Requirements 1.5, 5.2**

### Property 7: Bulk Fill Operations
*For any* block type, filling an entire chunk with that block type should result in all blocks in the chunk being set to that type.
**Validates: Requirements 5.3**

### Property 8: Data Initialization Validation
*For any* block data array, creating a chunk from that data should succeed if and only if the data length matches the expected chunk dimensions.
**Validates: Requirements 5.4, 5.5**

### Property 9: BlockID Serialization Round-Trip
*For any* valid BlockID value, converting to integer representation and back should produce the original BlockID.
**Validates: Requirements 3.3**

### Property 10: Batch Operations Consistency
*For any* set of coordinate-block pairs, batch setting those blocks should produce the same result as setting each block individually.
**Validates: Requirements 6.5**

## Error Handling

The chunk data model uses Rust's `Result` type for comprehensive error handling:

### ChunkError Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkError {
    OutOfBounds,
    InvalidDimensions, 
    InvalidBlockData,
}
```

### Error Handling Strategy

1. **Bounds Checking**: All coordinate-based operations validate bounds before array access
2. **Graceful Degradation**: Invalid operations return errors rather than panicking
3. **Type Safety**: BlockID enum prevents invalid block type assignments
4. **Validation**: Constructor methods validate input parameters before creating chunks

### Error Recovery

- **OutOfBounds**: Caller should validate coordinates or handle gracefully
- **InvalidDimensions**: Indicates programming error, should be fixed at compile time
- **InvalidBlockData**: Caller should validate data before chunk creation

## Testing Strategy

The chunk data model will be validated through a dual testing approach combining unit tests for specific examples and property-based tests for universal correctness guarantees.

### Unit Testing Focus

Unit tests will verify:
- Specific chunk size configurations (16×16×256, 32×32×128)
- Required BlockID variants (Air, Stone, Dirt, Grass)
- Error conditions with known invalid inputs
- Integration between coordinate conversion and block access
- Memory layout and size requirements

### Property-Based Testing Focus

Property-based tests will verify universal properties across randomized inputs:
- Coordinate conversion round-trip consistency across all valid coordinates
- Block storage and retrieval across all coordinate-block combinations
- Bounds validation across all possible out-of-bounds coordinates
- Initialization behavior across different chunk configurations
- Serialization consistency across all BlockID values

### Testing Configuration

- **Property Test Iterations**: Minimum 100 iterations per property test
- **Test Framework**: Use `proptest` crate for property-based testing in Rust
- **Tag Format**: Each property test tagged with **Feature: chunk-data-model, Property {number}: {property_text}**
- **Coverage**: Both unit and property tests are required for comprehensive validation

### Performance Testing

While not part of correctness validation, performance characteristics will be measured:
- Memory usage per chunk (target: ~128KB for 16×16×256)
- Access time benchmarks (should be constant time)
- Cache performance analysis for different access patterns