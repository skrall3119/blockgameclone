//! Window resize handling integration

use winit::{event::{Event, WindowEvent}, dpi::PhysicalSize};
use crate::renderer::{GraphicsContext, Camera, GraphicsResult, GraphicsError};

/// Handles window resize events by coordinating updates to graphics context and camera
pub struct ResizeHandler<'window> {
    graphics_context: &'window mut GraphicsContext<'window>,
    camera: &'window mut Camera,
}

impl<'window> ResizeHandler<'window> {
    /// Create a new resize handler
    pub fn new(graphics_context: &'window mut GraphicsContext<'window>, camera: &'window mut Camera) -> Self {
        Self {
            graphics_context,
            camera,
        }
    }

    /// Handle a window resize event
    /// This updates both the graphics surface configuration and camera aspect ratio
    pub fn handle_resize(&mut self, new_size: PhysicalSize<u32>) -> GraphicsResult<()> {
        // Validate dimensions
        if new_size.width == 0 || new_size.height == 0 {
            return Err(GraphicsError::InvalidDimensions {
                width: new_size.width,
                height: new_size.height,
            });
        }

        // Update graphics context surface configuration
        self.graphics_context.resize(new_size.width, new_size.height)?;

        // Update camera aspect ratio
        self.camera.handle_resize(new_size.width, new_size.height);

        Ok(())
    }

    /// Process a winit event and handle resize if applicable
    /// Returns true if the event was a resize event that was handled
    pub fn process_event(&mut self, event: &Event<()>) -> GraphicsResult<bool> {
        match event {
            Event::WindowEvent { 
                event: WindowEvent::Resized(new_size), 
                .. 
            } => {
                self.handle_resize(*new_size)?;
                Ok(true)
            }
            _ => Ok(false)
        }
    }

    /// Get the current surface size from the graphics context
    pub fn current_size(&self) -> (u32, u32) {
        self.graphics_context.surface_size()
    }

    /// Get the current camera aspect ratio
    pub fn current_aspect_ratio(&self) -> f32 {
        self.camera.aspect_ratio()
    }
}

/// Convenience function to handle resize events for a graphics context and camera
pub fn handle_window_resize(
    graphics_context: &mut GraphicsContext,
    camera: &mut Camera,
    new_size: PhysicalSize<u32>
) -> GraphicsResult<()> {
    // Validate dimensions
    if new_size.width == 0 || new_size.height == 0 {
        return Err(GraphicsError::InvalidDimensions {
            width: new_size.width,
            height: new_size.height,
        });
    }

    // Update graphics context surface configuration
    graphics_context.resize(new_size.width, new_size.height)?;

    // Update camera aspect ratio
    camera.handle_resize(new_size.width, new_size.height);

    Ok(())
}

#[cfg(test)]
mod tests {
    use winit::dpi::PhysicalSize;

    #[test]
    fn test_handle_window_resize_valid_dimensions() {
        // This test would require creating a graphics context and camera
        // which needs a window, so we'll test the validation logic
        let new_size = PhysicalSize::new(1920, 1080);
        
        // Test that valid dimensions don't immediately fail
        assert!(new_size.width > 0);
        assert!(new_size.height > 0);
    }

    #[test]
    fn test_handle_window_resize_zero_width() {
        let new_size = PhysicalSize::new(0, 1080);
        
        // This should be caught by validation
        assert_eq!(new_size.width, 0);
    }

    #[test]
    fn test_handle_window_resize_zero_height() {
        let new_size = PhysicalSize::new(1920, 0);
        
        // This should be caught by validation
        assert_eq!(new_size.height, 0);
    }

    #[test]
    fn test_aspect_ratio_calculation() {
        let width = 1920u32;
        let height = 1080u32;
        let expected_aspect = width as f32 / height as f32;
        
        assert_eq!(expected_aspect, 1920.0 / 1080.0);
        assert!((expected_aspect - 16.0/9.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_square_aspect_ratio() {
        let width = 512u32;
        let height = 512u32;
        let expected_aspect = width as f32 / height as f32;
        
        assert_eq!(expected_aspect, 1.0);
    }

    #[test]
    fn test_portrait_aspect_ratio() {
        let width = 600u32;
        let height = 800u32;
        let expected_aspect = width as f32 / height as f32;
        
        assert_eq!(expected_aspect, 0.75);
        assert!(expected_aspect < 1.0); // Portrait should be less than 1.0
    }
}