# Voxel Game

A Minecraft-inspired voxel sandbox game built from scratch in Rust with modern architecture.

## Features

- Single-player voxel world with chunked terrain generation
- Block placement and breaking mechanics  
- Deterministic world generation (same seed = same world)
- Extensible data-driven block and item systems
- Performance-focused architecture with multithreading

## Technology Stack

- **Language**: Rust (stable)
- **Graphics**: wgpu (Vulkan-first)
- **Math**: glam
- **ECS**: Bevy ECS (standalone)
- **Windowing**: winit
- **Shaders**: WGSL

## Project Structure

```
voxel-game/
├── crates/
│   ├── engine/     # Core engine (no voxel knowledge)
│   ├── world/      # World/chunk system (no rendering)
│   ├── game/       # Game rules and logic
│   └── tools/      # Development tools
├── assets/         # Game assets (textures, shaders, data)
└── src/           # Application entry point
```

## Building and Running

```bash
# Build the project
cargo build

# Run the game
cargo run

# Run tests
cargo test

# Build optimized release
cargo build --release
```

## Architecture

The project follows a layered architecture with clear separation of concerns:

- **Platform Layer**: Window, input, file system, timing
- **Engine Layer**: Renderer, ECS, core systems
- **World Layer**: Chunks, terrain generation, storage
- **Game Layer**: Blocks, items, player logic

See [docs/architecture.md](docs/architecture.md) for detailed technical documentation.