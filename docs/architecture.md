# Architecture Documentation

## Overview

This voxel game follows a layered architecture with clear separation of concerns:

- **Platform Layer**: Window management, input handling, file system access
- **Engine Layer**: Core systems with no game-specific knowledge
- **World Layer**: Chunk management, terrain generation, storage
- **Game Layer**: Game rules, block definitions, player logic

## Crate Dependencies

```
game → world → engine → platform libraries
```

## Key Design Principles

1. **Data-Oriented Design**: Structures optimized for cache efficiency
2. **Multithreaded by Default**: Heavy operations run off main thread
3. **Engine/Game Separation**: Core engine is reusable
4. **Data-Driven Content**: Blocks and items defined in JSON files

## Chunk System

- **Size**: 16×16×256 blocks per chunk
- **Storage**: Flat arrays indexed as `[y * width * depth + z * width + x]`
- **States**: Unloaded → Terrain Generated → Features Generated → Meshed → Renderable

## Performance Considerations

- Chunks use boxed arrays to avoid stack overflow
- Block IDs are u16 to support 65k block types while staying cache-friendly
- World generation and meshing happen on background threads
- Save data uses LZ4/Zstd compression for chunk storage