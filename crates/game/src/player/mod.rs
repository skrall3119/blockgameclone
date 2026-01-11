//! Player controller and related systems
//!
//! This module provides first-person movement and interaction capabilities
//! for the voxel game, including camera control, physics-based movement,
//! collision detection, and debug visualization.

use glam::{Vec3, Quat};
use engine::ecs::prelude::*;

// Re-export all public types
pub use self::controller::*;
pub use self::physics::*;
pub use self::camera::*;
pub use self::input::*;
pub use self::debug::*;

// Module declarations - these will be implemented in subsequent tasks
pub mod controller;
pub mod physics;
pub mod camera;
pub mod input;
pub mod debug;

/// Player controller error types
#[derive(Debug, thiserror::Error)]
pub enum PlayerError {
    #[error("Invalid player position: {position:?}")]
    InvalidPosition { position: Vec3 },
    
    #[error("Collision resolution failed after {attempts} attempts")]
    CollisionResolutionFailed { attempts: u32 },
    
    #[error("World interface query failed: {reason}")]
    WorldQueryFailed { reason: String },
    
    #[error("Input state error: {message}")]
    InputStateError { message: String },
    
    #[error("Physics simulation error: {message}")]
    PhysicsError { message: String },
}

/// Result type for player controller operations
pub type PlayerResult<T> = Result<T, PlayerError>;

/// Player controller constants
pub mod constants {
    /// Movement speed in blocks per second (Minecraft walking speed)
    pub const MOVEMENT_SPEED: f32 = 4.3;
    
    /// Jump height in blocks
    pub const JUMP_HEIGHT: f32 = 1.25;
    
    /// Gravity acceleration in blocks per second²
    pub const GRAVITY: f32 = 32.0;
    
    /// Terminal velocity in blocks per second
    pub const TERMINAL_VELOCITY: f32 = 78.4;
    
    /// Mouse sensitivity in degrees per pixel
    pub const MOUSE_SENSITIVITY: f32 = 0.1;
    
    /// Player AABB dimensions (width, height, depth) in blocks
    pub const PLAYER_AABB_SIZE: (f32, f32, f32) = (0.6, 1.8, 0.6);
    
    /// Collision detection margin to prevent floating point precision issues
    pub const COLLISION_MARGIN: f32 = 0.001;
    
    /// Ground detection threshold distance
    pub const GROUND_DETECTION_THRESHOLD: f32 = 0.1;
    
    /// Maximum step height the player can walk up
    pub const STEP_HEIGHT: f32 = 0.5;
    
    /// Maximum collision resolution iterations
    pub const MAX_COLLISION_RESOLUTION_STEPS: u32 = 3;
    
    /// Camera field of view in degrees
    pub const CAMERA_FOV: f32 = 70.0;
    
    /// Camera pitch limits in degrees (prevents gimbal lock)
    pub const CAMERA_PITCH_LIMIT: f32 = 89.0;
    
    /// Camera near clipping plane distance
    pub const CAMERA_NEAR_PLANE: f32 = 0.1;
    
    /// Camera far clipping plane distance
    pub const CAMERA_FAR_PLANE: f32 = 1000.0;
}

/// Player position and orientation component (from existing code)
#[derive(Component, Debug, Clone)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }
}

/// Player movement component (from existing code)
#[derive(Component, Debug, Clone, Default)]
pub struct Velocity {
    pub linear: Vec3,
    pub angular: Vec3,
}

/// Player input state (from existing code)
#[derive(Component, Debug, Clone, Default)]
pub struct PlayerInput {
    pub move_forward: bool,
    pub move_backward: bool,
    pub move_left: bool,
    pub move_right: bool,
    pub jump: bool,
    pub crouch: bool,
    pub mouse_delta: Vec3,
}

/// Player controller settings (from existing code)
#[derive(Debug, Clone)]
pub struct PlayerController {
    pub move_speed: f32,
    pub jump_force: f32,
    pub mouse_sensitivity: f32,
}

impl Default for PlayerController {
    fn default() -> Self {
        Self {
            move_speed: constants::MOVEMENT_SPEED,
            jump_force: constants::JUMP_HEIGHT * constants::GRAVITY, // Convert height to force
            mouse_sensitivity: constants::MOUSE_SENSITIVITY,
        }
    }
}