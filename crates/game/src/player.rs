//! Player controller and related systems

use glam::{Vec3, Quat};
use engine::ecs::prelude::*;

/// Player position and orientation component
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

/// Player movement component
#[derive(Component, Debug, Clone, Default)]
pub struct Velocity {
    pub linear: Vec3,
    pub angular: Vec3,
}

/// Player input state
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

/// Player controller settings
#[derive(Debug, Clone)]
pub struct PlayerController {
    pub move_speed: f32,
    pub jump_force: f32,
    pub mouse_sensitivity: f32,
}

impl Default for PlayerController {
    fn default() -> Self {
        Self {
            move_speed: 5.0,
            jump_force: 10.0,
            mouse_sensitivity: 0.002,
        }
    }
}