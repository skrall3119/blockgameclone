//! Camera system for view and projection transformations

use glam::{Mat4, Vec3};

/// Camera for 3D scene rendering with view and projection transformations
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world space
    position: Vec3,
    /// Target point the camera is looking at
    target: Vec3,
    /// Up vector for camera orientation
    up: Vec3,
    /// Field of view in radians
    fov: f32,
    /// Aspect ratio (width / height)
    aspect_ratio: f32,
    /// Near clipping plane distance
    near: f32,
    /// Far clipping plane distance
    far: f32,
}

impl Camera {
    /// Create a new camera with default settings
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 3.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: std::f32::consts::FRAC_PI_4, // 45 degrees
            aspect_ratio,
            near: 0.1,
            far: 100.0,
        }
    }

    /// Create a camera with custom parameters
    pub fn with_params(
        position: Vec3,
        target: Vec3,
        up: Vec3,
        fov: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            position,
            target,
            up,
            fov,
            aspect_ratio,
            near,
            far,
        }
    }

    /// Calculate the view matrix using right-handed coordinate system
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Calculate the projection matrix using right-handed coordinate system
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect_ratio, self.near, self.far)
    }

    /// Get the combined view-projection matrix
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Update the aspect ratio (typically called on window resize)
    /// This maintains camera state consistency by only updating the projection matrix
    pub fn update_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }

    /// Handle window resize by updating aspect ratio and ensuring state consistency
    /// This is a convenience method that wraps update_aspect_ratio with validation
    pub fn handle_resize(&mut self, new_width: u32, new_height: u32) {
        if new_height > 0 {
            let new_aspect_ratio = new_width as f32 / new_height as f32;
            self.update_aspect_ratio(new_aspect_ratio);
        }
    }

    /// Get camera position
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Set camera position
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    /// Get camera target
    pub fn target(&self) -> Vec3 {
        self.target
    }

    /// Set camera target
    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
    }

    /// Get camera up vector
    pub fn up(&self) -> Vec3 {
        self.up
    }

    /// Set camera up vector
    pub fn set_up(&mut self, up: Vec3) {
        self.up = up;
    }

    /// Get field of view in radians
    pub fn fov(&self) -> f32 {
        self.fov
    }

    /// Set field of view in radians
    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov;
    }

    /// Get aspect ratio
    pub fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }

    /// Get near clipping plane distance
    pub fn near(&self) -> f32 {
        self.near
    }

    /// Set near clipping plane distance
    pub fn set_near(&mut self, near: f32) {
        self.near = near;
    }

    /// Get far clipping plane distance
    pub fn far(&self) -> f32 {
        self.far
    }

    /// Set far clipping plane distance
    pub fn set_far(&mut self, far: f32) {
        self.far = far;
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(16.0 / 9.0) // Default 16:9 aspect ratio
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_camera_creation() {
        let camera = Camera::new(16.0 / 9.0);
        assert_eq!(camera.position(), Vec3::new(0.0, 0.0, 3.0));
        assert_eq!(camera.target(), Vec3::ZERO);
        assert_eq!(camera.up(), Vec3::Y);
        assert_eq!(camera.aspect_ratio(), 16.0 / 9.0);
    }

    #[test]
    fn test_camera_with_params() {
        let position = Vec3::new(1.0, 2.0, 3.0);
        let target = Vec3::new(0.0, 1.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let fov = std::f32::consts::FRAC_PI_3; // 60 degrees
        let aspect_ratio = 4.0 / 3.0;
        let near = 0.5;
        let far = 200.0;

        let camera = Camera::with_params(position, target, up, fov, aspect_ratio, near, far);
        
        assert_eq!(camera.position(), position);
        assert_eq!(camera.target(), target);
        assert_eq!(camera.up(), up);
        assert_eq!(camera.fov(), fov);
        assert_eq!(camera.aspect_ratio(), aspect_ratio);
        assert_eq!(camera.near(), near);
        assert_eq!(camera.far(), far);
    }

    #[test]
    fn test_view_matrix_calculation() {
        let camera = Camera::new(16.0 / 9.0);
        let view_matrix = camera.view_matrix();
        
        // The view matrix should be a valid 4x4 matrix
        // We can't easily test the exact values without complex math,
        // but we can verify it's not the identity matrix
        assert_ne!(view_matrix, Mat4::IDENTITY);
    }

    #[test]
    fn test_projection_matrix_calculation() {
        let camera = Camera::new(16.0 / 9.0);
        let proj_matrix = camera.projection_matrix();
        
        // The projection matrix should be a valid 4x4 matrix
        assert_ne!(proj_matrix, Mat4::IDENTITY);
    }

    #[test]
    fn test_view_projection_matrix_composition() {
        let camera = Camera::new(16.0 / 9.0);
        let view_proj = camera.view_projection_matrix();
        let expected = camera.projection_matrix() * camera.view_matrix();
        
        // The view-projection matrix should equal projection * view
        assert_eq!(view_proj, expected);
    }

    #[test]
    fn test_aspect_ratio_update() {
        let mut camera = Camera::new(16.0 / 9.0);
        let original_aspect = camera.aspect_ratio();
        
        camera.update_aspect_ratio(4.0 / 3.0);
        
        assert_ne!(camera.aspect_ratio(), original_aspect);
        assert_eq!(camera.aspect_ratio(), 4.0 / 3.0);
    }

    #[test]
    fn test_handle_resize() {
        let mut camera = Camera::new(16.0 / 9.0);
        
        // Test normal resize
        camera.handle_resize(1920, 1080);
        assert_eq!(camera.aspect_ratio(), 1920.0 / 1080.0);
        
        // Test different aspect ratio
        camera.handle_resize(800, 600);
        assert_eq!(camera.aspect_ratio(), 800.0 / 600.0);
        
        // Test square aspect ratio
        camera.handle_resize(512, 512);
        assert_eq!(camera.aspect_ratio(), 1.0);
    }

    #[test]
    fn test_handle_resize_zero_height() {
        let mut camera = Camera::new(16.0 / 9.0);
        let original_aspect = camera.aspect_ratio();
        
        // Should not change aspect ratio when height is 0
        camera.handle_resize(800, 0);
        assert_eq!(camera.aspect_ratio(), original_aspect);
    }

    #[test]
    fn test_resize_maintains_other_properties() {
        let mut camera = Camera::new(16.0 / 9.0);
        let original_position = camera.position();
        let original_target = camera.target();
        let original_up = camera.up();
        let original_fov = camera.fov();
        let original_near = camera.near();
        let original_far = camera.far();
        
        // Resize should only change aspect ratio
        camera.handle_resize(1024, 768);
        
        assert_eq!(camera.position(), original_position);
        assert_eq!(camera.target(), original_target);
        assert_eq!(camera.up(), original_up);
        assert_eq!(camera.fov(), original_fov);
        assert_eq!(camera.near(), original_near);
        assert_eq!(camera.far(), original_far);
        // Only aspect ratio should change
        assert_ne!(camera.aspect_ratio(), 16.0 / 9.0);
        assert_eq!(camera.aspect_ratio(), 1024.0 / 768.0);
    }

    #[test]
    fn test_camera_setters() {
        let mut camera = Camera::new(16.0 / 9.0);
        
        let new_position = Vec3::new(5.0, 10.0, 15.0);
        let new_target = Vec3::new(1.0, 2.0, 3.0);
        let new_up = Vec3::new(0.0, 0.0, 1.0);
        let new_fov = std::f32::consts::FRAC_PI_2; // 90 degrees
        
        camera.set_position(new_position);
        camera.set_target(new_target);
        camera.set_up(new_up);
        camera.set_fov(new_fov);
        camera.set_near(0.01);
        camera.set_far(1000.0);
        
        assert_eq!(camera.position(), new_position);
        assert_eq!(camera.target(), new_target);
        assert_eq!(camera.up(), new_up);
        assert_eq!(camera.fov(), new_fov);
        assert_eq!(camera.near(), 0.01);
        assert_eq!(camera.far(), 1000.0);
    }
}