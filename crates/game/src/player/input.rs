//! Input handling system for keyboard and mouse
//!
//! This module provides the InputHandler struct and related functionality
//! for processing keyboard and mouse input for player movement and camera control.

use std::collections::HashMap;
use glam::Vec3;
use winit::keyboard::KeyCode;

use crate::player::constants;

/// Input handler for keyboard and mouse events
#[derive(Debug, Clone)]
pub struct InputHandler {
    /// Current movement key states
    movement_keys: MovementKeys,
    /// Mouse sensitivity for camera rotation
    mouse_sensitivity: f32,
    /// Current state of all tracked keys
    key_states: HashMap<KeyCode, bool>,
}

/// Movement key states for WASD and Space
#[derive(Debug, Clone, Default)]
pub struct MovementKeys {
    /// W key - move forward
    pub forward: bool,
    /// S key - move backward
    pub backward: bool,
    /// A key - strafe left
    pub left: bool,
    /// D key - strafe right
    pub right: bool,
    /// Space key - jump
    pub jump: bool,
}

impl InputHandler {
    /// Create a new input handler with default settings
    pub fn new() -> Self {
        Self {
            movement_keys: MovementKeys::default(),
            mouse_sensitivity: constants::MOUSE_SENSITIVITY,
            key_states: HashMap::new(),
        }
    }

    /// Create a new input handler with custom mouse sensitivity
    pub fn with_sensitivity(mouse_sensitivity: f32) -> Self {
        Self {
            movement_keys: MovementKeys::default(),
            mouse_sensitivity,
            key_states: HashMap::new(),
        }
    }

    /// Handle keyboard input events
    pub fn handle_keyboard_input(&mut self, key: KeyCode, pressed: bool) {
        // Update key state tracking
        self.key_states.insert(key, pressed);

        // Update movement keys based on input
        match key {
            KeyCode::KeyW => self.movement_keys.forward = pressed,
            KeyCode::KeyS => self.movement_keys.backward = pressed,
            KeyCode::KeyA => self.movement_keys.left = pressed,
            KeyCode::KeyD => self.movement_keys.right = pressed,
            KeyCode::Space => self.movement_keys.jump = pressed,
            _ => {} // Ignore other keys for now
        }
    }

    /// Handle mouse motion events and return scaled deltas
    pub fn handle_mouse_motion(&mut self, delta_x: f32, delta_y: f32) -> (f32, f32) {
        // Apply sensitivity scaling to mouse movement
        let scaled_x = delta_x * self.mouse_sensitivity;
        let scaled_y = delta_y * self.mouse_sensitivity;
        
        (scaled_x, scaled_y)
    }

    /// Get the current movement vector based on camera orientation
    /// 
    /// This method generates camera-relative movement vectors, handles diagonal movement
    /// normalization, and supports simultaneous key press combinations.
    pub fn get_movement_vector(&self, camera_forward: Vec3, camera_right: Vec3) -> Vec3 {
        let mut movement = Vec3::ZERO;

        // Calculate movement based on pressed keys (camera-relative)
        if self.movement_keys.forward {
            movement += camera_forward;
        }
        if self.movement_keys.backward {
            movement -= camera_forward;
        }
        if self.movement_keys.right {
            movement += camera_right;
        }
        if self.movement_keys.left {
            movement -= camera_right;
        }

        // Normalize diagonal movement to prevent faster diagonal movement
        // This ensures consistent movement speed regardless of direction
        if movement.length() > 0.0 {
            movement = movement.normalize();
        }

        movement
    }

    /// Get a normalized movement vector with explicit speed scaling
    /// 
    /// This variant allows for explicit movement speed control while maintaining
    /// proper diagonal movement normalization.
    pub fn get_movement_vector_with_speed(&self, camera_forward: Vec3, camera_right: Vec3, speed: f32) -> Vec3 {
        let normalized_movement = self.get_movement_vector(camera_forward, camera_right);
        normalized_movement * speed
    }

    /// Check if any movement keys are currently pressed
    pub fn has_movement_input(&self) -> bool {
        self.movement_keys.forward || 
        self.movement_keys.backward || 
        self.movement_keys.left || 
        self.movement_keys.right
    }

    /// Get the raw movement direction without normalization
    /// 
    /// This is useful for debugging or when you need to know the exact
    /// combination of movement inputs without normalization applied.
    pub fn get_raw_movement_vector(&self, camera_forward: Vec3, camera_right: Vec3) -> Vec3 {
        let mut movement = Vec3::ZERO;

        if self.movement_keys.forward {
            movement += camera_forward;
        }
        if self.movement_keys.backward {
            movement -= camera_forward;
        }
        if self.movement_keys.right {
            movement += camera_right;
        }
        if self.movement_keys.left {
            movement -= camera_right;
        }

        movement
    }

    /// Check if jump key is currently pressed
    pub fn is_jump_pressed(&self) -> bool {
        self.movement_keys.jump
    }

    /// Get the current mouse sensitivity
    pub fn get_mouse_sensitivity(&self) -> f32 {
        self.mouse_sensitivity
    }

    /// Set the mouse sensitivity
    pub fn set_mouse_sensitivity(&mut self, sensitivity: f32) {
        self.mouse_sensitivity = sensitivity;
    }

    /// Get the current movement keys state
    pub fn get_movement_keys(&self) -> &MovementKeys {
        &self.movement_keys
    }

    /// Check if a specific key is currently pressed
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.key_states.get(&key).copied().unwrap_or(false)
    }

    /// Reset all input states (useful when window loses focus)
    pub fn reset_input_states(&mut self) {
        self.movement_keys = MovementKeys::default();
        self.key_states.clear();
    }

    /// Get all currently pressed keys
    pub fn get_pressed_keys(&self) -> Vec<KeyCode> {
        self.key_states
            .iter()
            .filter_map(|(key, &pressed)| if pressed { Some(*key) } else { None })
            .collect()
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use glam::Vec3;
    use winit::keyboard::KeyCode;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: player-controller, Property 11: Input Handling Consistency**
        /// **Validates: Requirements 7.1, 7.2, 7.3, 7.4**
        /// 
        /// For any input events (mouse movement, keyboard keys), the input handler should 
        /// capture and process them correctly, including proper handling of multiple 
        /// simultaneous key presses and frame rate independence.
        #[test]
        fn test_input_handling_consistency(
            // Mouse movement deltas
            mouse_delta_x in -1000.0f32..1000.0f32,
            mouse_delta_y in -1000.0f32..1000.0f32,
            // Key press combinations
            w_pressed in any::<bool>(),
            a_pressed in any::<bool>(),
            s_pressed in any::<bool>(),
            d_pressed in any::<bool>(),
            space_pressed in any::<bool>(),
            // Mouse sensitivity
            sensitivity in 0.01f32..10.0f32,
            // Camera vectors for movement calculation
            camera_forward_x in -1.0f32..1.0f32,
            camera_forward_z in -1.0f32..1.0f32,
            camera_right_x in -1.0f32..1.0f32,
            camera_right_z in -1.0f32..1.0f32,
        ) {
            let mut input_handler = InputHandler::with_sensitivity(sensitivity);
            
            // Test mouse motion handling
            let (scaled_x, scaled_y) = input_handler.handle_mouse_motion(mouse_delta_x, mouse_delta_y);
            
            // Mouse scaling should be consistent with sensitivity
            prop_assert!((scaled_x - mouse_delta_x * sensitivity).abs() < f32::EPSILON);
            prop_assert!((scaled_y - mouse_delta_y * sensitivity).abs() < f32::EPSILON);
            
            // Test keyboard input handling
            input_handler.handle_keyboard_input(KeyCode::KeyW, w_pressed);
            input_handler.handle_keyboard_input(KeyCode::KeyA, a_pressed);
            input_handler.handle_keyboard_input(KeyCode::KeyS, s_pressed);
            input_handler.handle_keyboard_input(KeyCode::KeyD, d_pressed);
            input_handler.handle_keyboard_input(KeyCode::Space, space_pressed);
            
            // Key states should match input
            prop_assert_eq!(input_handler.get_movement_keys().forward, w_pressed);
            prop_assert_eq!(input_handler.get_movement_keys().left, a_pressed);
            prop_assert_eq!(input_handler.get_movement_keys().backward, s_pressed);
            prop_assert_eq!(input_handler.get_movement_keys().right, d_pressed);
            prop_assert_eq!(input_handler.get_movement_keys().jump, space_pressed);
            prop_assert_eq!(input_handler.is_jump_pressed(), space_pressed);
            
            // Test movement vector calculation with normalized camera vectors
            let camera_forward = Vec3::new(camera_forward_x, 0.0, camera_forward_z).normalize_or_zero();
            let camera_right = Vec3::new(camera_right_x, 0.0, camera_right_z).normalize_or_zero();
            
            let movement = input_handler.get_movement_vector(camera_forward, camera_right);
            
            // Movement vector should be normalized if any movement keys are pressed
            let any_movement = w_pressed || a_pressed || s_pressed || d_pressed;
            if any_movement && camera_forward.length() > 0.0 && camera_right.length() > 0.0 {
                // Movement should be normalized (length <= 1.0 due to floating point precision)
                prop_assert!(movement.length() <= 1.0 + f32::EPSILON);
            } else if !any_movement {
                // No movement should result in zero vector
                prop_assert_eq!(movement, Vec3::ZERO);
            }
            
            // Test key state tracking
            prop_assert_eq!(input_handler.is_key_pressed(KeyCode::KeyW), w_pressed);
            prop_assert_eq!(input_handler.is_key_pressed(KeyCode::KeyA), a_pressed);
            prop_assert_eq!(input_handler.is_key_pressed(KeyCode::KeyS), s_pressed);
            prop_assert_eq!(input_handler.is_key_pressed(KeyCode::KeyD), d_pressed);
            prop_assert_eq!(input_handler.is_key_pressed(KeyCode::Space), space_pressed);
            
            // Test sensitivity getter/setter
            prop_assert_eq!(input_handler.get_mouse_sensitivity(), sensitivity);
            
            let new_sensitivity = sensitivity * 2.0;
            input_handler.set_mouse_sensitivity(new_sensitivity);
            prop_assert_eq!(input_handler.get_mouse_sensitivity(), new_sensitivity);
            
            // Test input state reset
            input_handler.reset_input_states();
            prop_assert!(!input_handler.get_movement_keys().forward);
            prop_assert!(!input_handler.get_movement_keys().left);
            prop_assert!(!input_handler.get_movement_keys().backward);
            prop_assert!(!input_handler.get_movement_keys().right);
            prop_assert!(!input_handler.get_movement_keys().jump);
            prop_assert!(!input_handler.is_jump_pressed());
            prop_assert!(input_handler.get_pressed_keys().is_empty());
        }

        /// **Feature: player-controller, Property 3: Movement Direction Consistency**
        /// **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5**
        /// 
        /// For any camera orientation and movement key combination (WASD), the resulting 
        /// movement vector should be correctly oriented relative to the camera's forward 
        /// and right vectors.
        #[test]
        fn test_movement_direction_consistency(
            // Camera orientation (yaw and pitch for generating forward/right vectors)
            camera_yaw in 0.0f32..360.0f32,
            camera_pitch in -89.0f32..89.0f32,
            // Movement key combinations
            w_pressed in any::<bool>(),
            a_pressed in any::<bool>(),
            s_pressed in any::<bool>(),
            d_pressed in any::<bool>(),
        ) {
            let mut input_handler = InputHandler::new();
            
            // Set up movement keys
            input_handler.handle_keyboard_input(KeyCode::KeyW, w_pressed);
            input_handler.handle_keyboard_input(KeyCode::KeyA, a_pressed);
            input_handler.handle_keyboard_input(KeyCode::KeyS, s_pressed);
            input_handler.handle_keyboard_input(KeyCode::KeyD, d_pressed);
            
            // Generate camera vectors from yaw and pitch
            let yaw_rad = camera_yaw.to_radians();
            let pitch_rad = camera_pitch.to_radians();
            
            // Calculate camera forward vector (standard FPS camera)
            let camera_forward = Vec3::new(
                yaw_rad.sin() * pitch_rad.cos(),
                -pitch_rad.sin(),
                -yaw_rad.cos() * pitch_rad.cos()
            ).normalize();
            
            // Calculate camera right vector (perpendicular to forward, in XZ plane)
            let camera_right = Vec3::new(yaw_rad.cos(), 0.0, yaw_rad.sin()).normalize();
            
            // Get movement vector
            let movement = input_handler.get_movement_vector(camera_forward, camera_right);
            
            // Test movement direction consistency
            let any_movement = w_pressed || a_pressed || s_pressed || d_pressed;
            
            if any_movement {
                // Movement should be normalized (length = 1.0)
                if movement.length() > 0.0 {
                    prop_assert!((movement.length() - 1.0).abs() < f32::EPSILON * 10.0);
                }
                
                // Test individual movement directions
                if w_pressed && !s_pressed && !a_pressed && !d_pressed {
                    // Forward only - movement should align with camera forward
                    let dot_product = movement.dot(camera_forward);
                    prop_assert!(dot_product > 0.99, "Forward movement not aligned with camera forward");
                }
                
                if s_pressed && !w_pressed && !a_pressed && !d_pressed {
                    // Backward only - movement should be opposite to camera forward
                    let dot_product = movement.dot(camera_forward);
                    prop_assert!(dot_product < -0.99, "Backward movement not opposite to camera forward");
                }
                
                if d_pressed && !w_pressed && !s_pressed && !a_pressed {
                    // Right only - movement should align with camera right
                    let dot_product = movement.dot(camera_right);
                    prop_assert!(dot_product > 0.99, "Right movement not aligned with camera right");
                }
                
                if a_pressed && !w_pressed && !s_pressed && !d_pressed {
                    // Left only - movement should be opposite to camera right
                    let dot_product = movement.dot(camera_right);
                    prop_assert!(dot_product < -0.99, "Left movement not opposite to camera right");
                }
                
                // Test diagonal movements
                if w_pressed && d_pressed && !s_pressed && !a_pressed {
                    // Forward + Right diagonal
                    let expected = (camera_forward + camera_right).normalize();
                    let dot_product = movement.dot(expected);
                    prop_assert!(dot_product > 0.99, "Forward+Right diagonal not correct");
                }
                
                if w_pressed && a_pressed && !s_pressed && !d_pressed {
                    // Forward + Left diagonal
                    let expected = (camera_forward - camera_right).normalize();
                    let dot_product = movement.dot(expected);
                    prop_assert!(dot_product > 0.99, "Forward+Left diagonal not correct");
                }
                
                // Test opposing movements (should cancel out)
                if w_pressed && s_pressed && !a_pressed && !d_pressed {
                    // Forward + Backward should cancel
                    prop_assert_eq!(movement, Vec3::ZERO, "Forward+Backward should cancel");
                }
                
                if a_pressed && d_pressed && !w_pressed && !s_pressed {
                    // Left + Right should cancel
                    prop_assert_eq!(movement, Vec3::ZERO, "Left+Right should cancel");
                }
            } else {
                // No movement keys pressed - should result in zero movement
                prop_assert_eq!(movement, Vec3::ZERO);
            }
        }
    }

    #[test]
    fn test_input_handler_creation() {
        let handler = InputHandler::new();
        assert_eq!(handler.get_mouse_sensitivity(), constants::MOUSE_SENSITIVITY);
        assert!(!handler.is_jump_pressed());
        assert_eq!(handler.get_movement_vector(Vec3::Z, Vec3::X), Vec3::ZERO);
    }

    #[test]
    fn test_movement_key_combinations() {
        let mut handler = InputHandler::new();
        
        // Test forward + right diagonal movement
        handler.handle_keyboard_input(KeyCode::KeyW, true);
        handler.handle_keyboard_input(KeyCode::KeyD, true);
        
        let forward = Vec3::new(0.0, 0.0, -1.0); // Forward is negative Z
        let right = Vec3::new(1.0, 0.0, 0.0);    // Right is positive X
        
        let movement = handler.get_movement_vector(forward, right);
        
        // Should be normalized diagonal movement
        assert!((movement.length() - 1.0).abs() < f32::EPSILON);
        assert!(movement.x > 0.0); // Moving right
        assert!(movement.z < 0.0); // Moving forward
    }

    #[test]
    fn test_mouse_sensitivity_scaling() {
        let mut handler = InputHandler::with_sensitivity(2.0);
        
        let (scaled_x, scaled_y) = handler.handle_mouse_motion(10.0, -5.0);
        
        assert_eq!(scaled_x, 20.0);  // 10.0 * 2.0
        assert_eq!(scaled_y, -10.0); // -5.0 * 2.0
    }

    #[test]
    fn test_movement_vector_with_speed() {
        let mut handler = InputHandler::new();
        handler.handle_keyboard_input(KeyCode::KeyW, true);
        
        let forward = Vec3::new(0.0, 0.0, -1.0);
        let right = Vec3::new(1.0, 0.0, 0.0);
        let speed = 5.0;
        
        let movement = handler.get_movement_vector_with_speed(forward, right, speed);
        
        assert!((movement.length() - speed).abs() < f32::EPSILON);
        assert_eq!(movement, forward * speed);
    }

    #[test]
    fn test_has_movement_input() {
        let mut handler = InputHandler::new();
        
        // No movement initially
        assert!(!handler.has_movement_input());
        
        // Test each movement key
        handler.handle_keyboard_input(KeyCode::KeyW, true);
        assert!(handler.has_movement_input());
        
        handler.handle_keyboard_input(KeyCode::KeyW, false);
        assert!(!handler.has_movement_input());
        
        handler.handle_keyboard_input(KeyCode::KeyA, true);
        assert!(handler.has_movement_input());
    }

    #[test]
    fn test_raw_movement_vector() {
        let mut handler = InputHandler::new();
        
        // Test diagonal movement without normalization
        handler.handle_keyboard_input(KeyCode::KeyW, true);
        handler.handle_keyboard_input(KeyCode::KeyD, true);
        
        let forward = Vec3::new(0.0, 0.0, -1.0);
        let right = Vec3::new(1.0, 0.0, 0.0);
        
        let raw_movement = handler.get_raw_movement_vector(forward, right);
        let normalized_movement = handler.get_movement_vector(forward, right);
        
        // Raw movement should be longer than normalized for diagonal
        assert!(raw_movement.length() > normalized_movement.length());
        assert!((raw_movement.length() - std::f32::consts::SQRT_2).abs() < f32::EPSILON);
        assert!((normalized_movement.length() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_simultaneous_key_combinations() {
        let mut handler = InputHandler::new();
        let forward = Vec3::new(0.0, 0.0, -1.0);
        let right = Vec3::new(1.0, 0.0, 0.0);
        
        // Test all possible combinations
        let test_cases = [
            // (W, A, S, D, expected_direction)
            (true, false, false, false), // Forward only
            (false, true, false, false), // Left only  
            (false, false, true, false), // Backward only
            (false, false, false, true), // Right only
            (true, true, false, false),  // Forward + Left diagonal
            (true, false, false, true),  // Forward + Right diagonal
            (false, true, true, false),  // Backward + Left diagonal
            (false, false, true, true),  // Backward + Right diagonal
            (true, false, true, false),  // Forward + Backward (should cancel)
            (false, true, false, true),  // Left + Right (should cancel)
        ];
        
        for (w, a, s, d) in test_cases {
            handler.reset_input_states();
            handler.handle_keyboard_input(KeyCode::KeyW, w);
            handler.handle_keyboard_input(KeyCode::KeyA, a);
            handler.handle_keyboard_input(KeyCode::KeyS, s);
            handler.handle_keyboard_input(KeyCode::KeyD, d);
            
            let movement = handler.get_movement_vector(forward, right);
            
            // Movement should be normalized if any movement exists
            if handler.has_movement_input() {
                if movement.length() > 0.0 {
                    assert!((movement.length() - 1.0).abs() < f32::EPSILON, 
                           "Movement not normalized for combination W:{} A:{} S:{} D:{}", w, a, s, d);
                }
            }
        }
    }
}