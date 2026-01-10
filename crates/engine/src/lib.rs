//! Core engine systems - graphics, ECS, platform abstraction
//! 
//! This crate provides the foundational systems for the voxel game engine,
//! with no knowledge of voxels or game-specific concepts.

pub mod ecs;
pub mod platform;
pub mod renderer;
pub mod util;

pub use glam;
pub use bevy_ecs;
pub use bytemuck;