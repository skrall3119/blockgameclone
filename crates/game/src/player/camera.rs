//! First-person camera system with mouse look
//!
//! This module provides the Camera struct and related functionality
//! for first-person camera control, view matrix generation, and mouse look.

use glam::{Vec3, Mat4};
use crate::player::constants::*;

/// First-person camera with mouse look capabilities
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world space
    position: Vec3,
    /// Horizontal rotation in radians (left/right)
    yaw: f32,
    /// Vertical rotation in radians (up/down)
    pitch: f32,
    /// Field of view in radians
    fov: f32,
    /// Aspect ratio (width/height)
    aspect_ratio: f32,
    /// Near clipping plane distance
    near_plane: f32,
    /// Far clipping plane distance
    far_plane: f32,
}

impl Camera {
    /// Create a new camera at the specified position
    pub fn new(position: Vec3, aspect_ratio: f32) -> Self {
        Self {
            position,
            yaw: 0.0,
            pitch: 0.0,
            fov: CAMERA_FOV.to_radians(),
            aspect_ratio,
            near_plane: CAMERA_NEAR_PLANE,
            far_plane: CAMERA_FAR_PLANE,
        }
    }

    /// Update camera rotation based on mouse input
    /// 
    /// # Arguments
    /// * `delta_yaw` - Horizontal rotation change in radians
    /// * `delta_pitch` - Vertical rotation change in radians
    pub fn update_rotation(&mut self, delta_yaw: f32, delta_pitch: f32) {
        // Update yaw (horizontal rotation)
        self.yaw += delta_yaw;
        
        // Normalize yaw to [-π, π] range
        while self.yaw > std::f32::consts::PI {
            self.yaw -= 2.0 * std::f32::consts::PI;
        }
        while self.yaw < -std::f32::consts::PI {
            self.yaw += 2.0 * std::f32::consts::PI;
        }
        
        // Update pitch (vertical rotation) with clamping
        self.pitch += delta_pitch;
        let pitch_limit = CAMERA_PITCH_LIMIT.to_radians();
        self.pitch = self.pitch.clamp(-pitch_limit, pitch_limit);
    }

    /// Generate the view matrix for rendering
    pub fn get_view_matrix(&self) -> Mat4 {
        let forward = self.get_forward_vector();
        let target = self.position + forward;
        let up = Vec3::Y; // World up vector
        
        Mat4::look_at_lh(self.position, target, up)
    }

    /// Generate the projection matrix for rendering
    pub fn get_projection_matrix(&self) -> Mat4 {
        Mat4::perspective_lh(self.fov, self.aspect_ratio, self.near_plane, self.far_plane)
    }

    /// Get the forward direction vector based on current rotation
    pub fn get_forward_vector(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        ).normalize()
    }

    /// Get the right direction vector based on current rotation
    pub fn get_right_vector(&self) -> Vec3 {
        self.get_forward_vector().cross(Vec3::Y).normalize()
    }

    /// Get the up direction vector based on current rotation
    pub fn get_up_vector(&self) -> Vec3 {
        self.get_right_vector().cross(self.get_forward_vector()).normalize()
    }

    /// Set the camera position
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    /// Get the camera position
    pub fn get_position(&self) -> Vec3 {
        self.position
    }

    /// Get the current yaw rotation in radians
    pub fn get_yaw(&self) -> f32 {
        self.yaw
    }

    /// Get the current pitch rotation in radians
    pub fn get_pitch(&self) -> f32 {
        self.pitch
    }

    /// Set the aspect ratio (typically called when window is resized)
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }

    /// Get the current aspect ratio
    pub fn get_aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }

    /// Get the field of view in radians
    pub fn get_fov(&self) -> f32 {
        self.fov
    }

    /// Set the field of view in radians
    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov;
    }

    /// Get movement direction for forward movement relative to camera
    /// Returns a normalized vector in the XZ plane (no vertical component)
    pub fn get_movement_forward(&self) -> Vec3 {
        let forward = Vec3::new(
            self.yaw.cos(),
            0.0, // No vertical component for movement
            self.yaw.sin(),
        );
        forward.normalize()
    }

    /// Get movement direction for backward movement relative to camera
    /// Returns a normalized vector in the XZ plane (no vertical component)
    pub fn get_movement_backward(&self) -> Vec3 {
        -self.get_movement_forward()
    }

    /// Get movement direction for left strafe relative to camera
    /// Returns a normalized vector in the XZ plane (no vertical component)
    pub fn get_movement_left(&self) -> Vec3 {
        let right = self.get_movement_right();
        -right
    }

    /// Get movement direction for right strafe relative to camera
    /// Returns a normalized vector in the XZ plane (no vertical component)
    pub fn get_movement_right(&self) -> Vec3 {
        let forward = self.get_movement_forward();
        forward.cross(Vec3::Y).normalize()
    }

    /// Calculate movement vector based on input directions
    /// 
    /// # Arguments
    /// * `forward` - Forward movement input (positive = forward, negative = backward)
    /// * `right` - Right movement input (positive = right, negative = left)
    /// 
    /// # Returns
    /// Normalized movement vector in world space, or zero vector if no input
    pub fn calculate_movement_vector(&self, forward: f32, right: f32) -> Vec3 {
        if forward == 0.0 && right == 0.0 {
            return Vec3::ZERO;
        }

        let forward_dir = self.get_movement_forward();
        let right_dir = self.get_movement_right();
        
        let movement = forward_dir * forward + right_dir * right;
        
        // Normalize to prevent faster diagonal movement
        if movement.length_squared() > 0.0 {
            movement.normalize()
        } else {
            Vec3::ZERO
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        /// **Feature: player-controller, Property 1: Mouse Input Camera Rotation**
        /// **Validates: Requirements 1.1, 1.2, 1.3**
        /// 
        /// For any horizontal mouse movement delta, the camera yaw should change proportionally 
        /// to the mouse movement, and for any vertical mouse movement delta, the camera pitch 
        /// should change proportionally while being clamped to valid limits.
        #[test]
        fn test_mouse_input_camera_rotation(
            mouse_delta_x in -1000.0f32..1000.0f32,
            mouse_delta_y in -1000.0f32..1000.0f32,
            initial_yaw in -std::f32::consts::PI..std::f32::consts::PI,
            initial_pitch in -CAMERA_PITCH_LIMIT.to_radians()..CAMERA_PITCH_LIMIT.to_radians()
        ) {
            let mut camera = Camera::new(Vec3::ZERO, 16.0 / 9.0);
            
            // Set initial rotation
            camera.yaw = initial_yaw;
            camera.pitch = initial_pitch;
            
            let initial_yaw_value = camera.get_yaw();
            let initial_pitch_value = camera.get_pitch();
            
            // Apply mouse input
            camera.update_rotation(mouse_delta_x, mouse_delta_y);
            
            let final_yaw = camera.get_yaw();
            let final_pitch = camera.get_pitch();
            
            // Property 1: Yaw should change proportionally to horizontal mouse movement
            // (accounting for normalization to [-π, π])
            let expected_yaw = initial_yaw_value + mouse_delta_x;
            let normalized_expected_yaw = {
                let mut yaw = expected_yaw;
                while yaw > std::f32::consts::PI {
                    yaw -= 2.0 * std::f32::consts::PI;
                }
                while yaw < -std::f32::consts::PI {
                    yaw += 2.0 * std::f32::consts::PI;
                }
                yaw
            };
            
            prop_assert!((final_yaw - normalized_expected_yaw).abs() < 1e-6, 
                "Yaw should change proportionally to mouse X movement. Expected: {}, Got: {}", 
                normalized_expected_yaw, final_yaw);
            
            // Property 2: Pitch should change proportionally to vertical mouse movement
            // but be clamped to valid limits
            let expected_pitch = (initial_pitch_value + mouse_delta_y)
                .clamp(-CAMERA_PITCH_LIMIT.to_radians(), CAMERA_PITCH_LIMIT.to_radians());
            
            prop_assert!((final_pitch - expected_pitch).abs() < 1e-6,
                "Pitch should change proportionally to mouse Y movement with clamping. Expected: {}, Got: {}",
                expected_pitch, final_pitch);
            
            // Property 3: Pitch should always be within valid limits
            let pitch_limit = CAMERA_PITCH_LIMIT.to_radians();
            prop_assert!(final_pitch >= -pitch_limit && final_pitch <= pitch_limit,
                "Pitch should always be within limits [-{}, {}]. Got: {}",
                pitch_limit, pitch_limit, final_pitch);
        }
        
        /// **Feature: player-controller, Property 2: Camera View Matrix Updates**
        /// **Validates: Requirements 1.5**
        /// 
        /// For any camera rotation change, the view matrix should be updated to reflect 
        /// the new camera orientation, producing different view matrices for different orientations.
        #[test]
        fn test_camera_view_matrix_updates(
            yaw1 in -std::f32::consts::PI..std::f32::consts::PI,
            pitch1 in -CAMERA_PITCH_LIMIT.to_radians()..CAMERA_PITCH_LIMIT.to_radians(),
            yaw2 in -std::f32::consts::PI..std::f32::consts::PI,
            pitch2 in -CAMERA_PITCH_LIMIT.to_radians()..CAMERA_PITCH_LIMIT.to_radians(),
            position in prop::array::uniform3(-100.0f32..100.0f32)
        ) {
            let position = Vec3::from_array(position);
            let mut camera1 = Camera::new(position, 16.0 / 9.0);
            let mut camera2 = Camera::new(position, 16.0 / 9.0);
            
            // Set different rotations
            camera1.yaw = yaw1;
            camera1.pitch = pitch1;
            camera2.yaw = yaw2;
            camera2.pitch = pitch2;
            
            let matrix1 = camera1.get_view_matrix();
            let matrix2 = camera2.get_view_matrix();
            
            // Property 1: Different orientations should produce different view matrices
            // (unless the orientations are equivalent due to normalization)
            let yaw_diff = (yaw1 - yaw2).abs();
            let pitch_diff = (pitch1 - pitch2).abs();
            let orientation_different = yaw_diff > 1e-6 || pitch_diff > 1e-6;
            
            if orientation_different {
                // Check that at least one element of the matrices is different
                let matrices_different = matrix1.to_cols_array()
                    .iter()
                    .zip(matrix2.to_cols_array().iter())
                    .any(|(a, b)| (a - b).abs() > 1e-6);
                
                prop_assert!(matrices_different,
                    "Different camera orientations should produce different view matrices. \
                     Yaw1: {}, Pitch1: {}, Yaw2: {}, Pitch2: {}",
                    yaw1, pitch1, yaw2, pitch2);
            }
            
            // Property 2: View matrix should be consistent for the same orientation
            let matrix1_again = camera1.get_view_matrix();
            let matrices_equal = matrix1.to_cols_array()
                .iter()
                .zip(matrix1_again.to_cols_array().iter())
                .all(|(a, b)| (a - b).abs() < 1e-6);
            
            prop_assert!(matrices_equal,
                "Same camera orientation should produce identical view matrices");
            
            // Property 3: View matrix should reflect camera position changes
            let original_matrix = camera1.get_view_matrix();
            camera1.set_position(position + Vec3::new(1.0, 0.0, 0.0));
            let moved_matrix = camera1.get_view_matrix();
            
            let position_matrices_different = original_matrix.to_cols_array()
                .iter()
                .zip(moved_matrix.to_cols_array().iter())
                .any(|(a, b)| (a - b).abs() > 1e-6);
            
            prop_assert!(position_matrices_different,
                "Changing camera position should produce different view matrices");
        }
    }

    // Unit tests for camera vector calculations
    #[test]
    fn test_camera_vector_calculations() {
        let mut camera = Camera::new(Vec3::ZERO, 16.0 / 9.0);
        
        // Test forward vector at zero rotation
        camera.yaw = 0.0;
        camera.pitch = 0.0;
        let forward = camera.get_movement_forward();
        assert!((forward - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-6, 
            "Forward vector should point along positive X axis at zero rotation");
        
        // Test right vector at zero rotation
        let right = camera.get_movement_right();
        assert!((right - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-6,
            "Right vector should point along positive Z axis at zero rotation");
        
        // Test backward vector
        let backward = camera.get_movement_backward();
        assert!((backward - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-6,
            "Backward vector should point along negative X axis at zero rotation");
        
        // Test left vector
        let left = camera.get_movement_left();
        assert!((left - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-6,
            "Left vector should point along negative Z axis at zero rotation");
        
        // Test 90-degree rotation
        camera.yaw = std::f32::consts::PI / 2.0;
        let forward_90 = camera.get_movement_forward();
        assert!((forward_90 - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-6,
            "Forward vector should point along positive Z axis at 90-degree rotation");
        
        // Test movement vector calculation
        let movement = camera.calculate_movement_vector(1.0, 0.0);
        assert!((movement - forward_90).length() < 1e-6,
            "Movement vector should match forward direction for forward input");
        
        // Test diagonal movement normalization
        let diagonal = camera.calculate_movement_vector(1.0, 1.0);
        assert!((diagonal.length() - 1.0).abs() < 1e-6,
            "Diagonal movement should be normalized to unit length");
        
        // Test zero movement
        let zero_movement = camera.calculate_movement_vector(0.0, 0.0);
        assert!(zero_movement.length() < 1e-6,
            "Zero input should produce zero movement vector");
    }

    #[test]
    fn test_vector_normalization() {
        let camera = Camera::new(Vec3::ZERO, 16.0 / 9.0);
        
        // Test that all direction vectors are normalized
        let forward = camera.get_forward_vector();
        let right = camera.get_right_vector();
        let up = camera.get_up_vector();
        let movement_forward = camera.get_movement_forward();
        let movement_right = camera.get_movement_right();
        
        assert!((forward.length() - 1.0).abs() < 1e-6, "Forward vector should be normalized");
        assert!((right.length() - 1.0).abs() < 1e-6, "Right vector should be normalized");
        assert!((up.length() - 1.0).abs() < 1e-6, "Up vector should be normalized");
        assert!((movement_forward.length() - 1.0).abs() < 1e-6, "Movement forward vector should be normalized");
        assert!((movement_right.length() - 1.0).abs() < 1e-6, "Movement right vector should be normalized");
    }
}