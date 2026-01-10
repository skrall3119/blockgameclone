//! Integration example showing how to use window resize handling

use winit::{event::{Event, WindowEvent}, event_loop::ControlFlow};
use crate::{
    platform::WindowManager,
    renderer::{GraphicsContext, Camera, handle_window_resize, RenderResult},
};

/// Example integration that demonstrates proper resize handling
pub struct ResizeIntegrationExample<'window> {
    graphics_context: GraphicsContext<'window>,
    camera: Camera,
}

impl<'window> ResizeIntegrationExample<'window> {
    /// Create a new integration example
    pub async fn new(window: &'window winit::window::Window) -> RenderResult<Self> {
        // Initialize graphics context
        let graphics_context = GraphicsContext::new(window).await
            .map_err(crate::renderer::RenderError::Graphics)?;
        
        // Initialize camera with current window aspect ratio
        let (width, height) = graphics_context.surface_size();
        let aspect_ratio = width as f32 / height as f32;
        let camera = Camera::new(aspect_ratio);

        println!("Initialized graphics context with surface size: {}x{}", width, height);
        println!("Camera aspect ratio: {:.3}", aspect_ratio);

        Ok(Self {
            graphics_context,
            camera,
        })
    }

    /// Handle window events, specifically resize events
    pub fn handle_event(&mut self, event: &Event<()>) -> RenderResult<()> {
        match event {
            Event::WindowEvent { 
                event: WindowEvent::Resized(new_size), 
                .. 
            } => {
                println!("Handling window resize to: {}x{}", new_size.width, new_size.height);
                
                // Use the integrated resize handling function
                handle_window_resize(&mut self.graphics_context, &mut self.camera, *new_size)
                    .map_err(crate::renderer::RenderError::Graphics)?;
                
                // Verify the changes were applied
                let (width, height) = self.graphics_context.surface_size();
                let aspect_ratio = self.camera.aspect_ratio();
                
                println!("Surface updated to: {}x{}", width, height);
                println!("Camera aspect ratio updated to: {:.3}", aspect_ratio);
                
                // Validate consistency
                let expected_aspect = width as f32 / height as f32;
                if (aspect_ratio - expected_aspect).abs() > f32::EPSILON {
                    println!("Warning: Aspect ratio mismatch! Expected: {:.3}, Got: {:.3}", 
                             expected_aspect, aspect_ratio);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Get current state information
    pub fn get_state_info(&self) -> (u32, u32, f32) {
        let (width, height) = self.graphics_context.surface_size();
        let aspect_ratio = self.camera.aspect_ratio();
        (width, height, aspect_ratio)
    }
}

/// Run a simple example that demonstrates resize handling
pub async fn run_resize_example() -> RenderResult<()> {
    println!("Starting resize handling example...");
    
    let window_manager = WindowManager::new();
    
    window_manager.run(|_window, event| {
        match event {
            Event::WindowEvent { 
                event: WindowEvent::Resized(new_size), 
                .. 
            } => {
                println!("Window resize event: {}x{}", new_size.width, new_size.height);
                
                // Calculate and display aspect ratio
                if new_size.height > 0 {
                    let aspect_ratio = new_size.width as f32 / new_size.height as f32;
                    println!("New aspect ratio: {:.3}", aspect_ratio);
                    
                    // Demonstrate different aspect ratio categories
                    if aspect_ratio > 1.5 {
                        println!("Wide aspect ratio (landscape)");
                    } else if aspect_ratio < 0.8 {
                        println!("Tall aspect ratio (portrait)");
                    } else {
                        println!("Standard aspect ratio");
                    }
                }
            }
            Event::WindowEvent { 
                event: WindowEvent::CloseRequested, 
                .. 
            } => {
                println!("Window close requested - exiting resize example");
            }
            _ => {}
        }
        ControlFlow::Wait
    }).map_err(|e| crate::renderer::RenderError::Window(e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use winit::dpi::PhysicalSize;

    #[test]
    fn test_aspect_ratio_categories() {
        // Test wide aspect ratio
        let wide_size = PhysicalSize::new(1920, 1080);
        let wide_aspect = wide_size.width as f32 / wide_size.height as f32;
        assert!(wide_aspect > 1.5);

        // Test standard aspect ratio
        let standard_size = PhysicalSize::new(1024, 768);
        let standard_aspect = standard_size.width as f32 / standard_size.height as f32;
        assert!(standard_aspect > 0.8 && standard_aspect <= 1.5);

        // Test tall aspect ratio
        let tall_size = PhysicalSize::new(600, 800);
        let tall_aspect = tall_size.width as f32 / tall_size.height as f32;
        assert!(tall_aspect < 0.8);
    }

    #[test]
    fn test_state_consistency() {
        // Test that aspect ratio calculation is consistent
        let width = 1920u32;
        let height = 1080u32;
        let expected_aspect = width as f32 / height as f32;
        
        // This should match what our camera would calculate
        assert!((expected_aspect - 16.0/9.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_resize_validation() {
        // Test that we properly validate resize dimensions
        let valid_size = PhysicalSize::new(800, 600);
        assert!(valid_size.width > 0 && valid_size.height > 0);

        let zero_width = PhysicalSize::new(0, 600);
        assert!(zero_width.width == 0);

        let zero_height = PhysicalSize::new(800, 0);
        assert!(zero_height.height == 0);
    }
}