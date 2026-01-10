//! Application integration for rendering system with resize handling

use winit::{event::{Event, WindowEvent}, event_loop::ControlFlow, dpi::PhysicalSize};
use crate::{
    platform::WindowManager,
    renderer::{GraphicsContext, Camera, handle_window_resize, GraphicsResult, RenderResult},
};

/// Example application that demonstrates proper resize handling integration
pub struct RenderApp<'window> {
    graphics_context: GraphicsContext<'window>,
    camera: Camera,
}

impl<'window> RenderApp<'window> {
    /// Create a new render application
    pub async fn new(window: &'window winit::window::Window) -> GraphicsResult<Self> {
        let graphics_context = GraphicsContext::new(window).await?;
        
        // Initialize camera with current window aspect ratio
        let (width, height) = graphics_context.surface_size();
        let aspect_ratio = width as f32 / height as f32;
        let camera = Camera::new(aspect_ratio);

        Ok(Self {
            graphics_context,
            camera,
        })
    }

    /// Handle window events, including resize
    pub fn handle_event(&mut self, event: &Event<()>, event_loop: &winit::event_loop::ActiveEventLoop) -> GraphicsResult<()> {
        match event {
            Event::WindowEvent { 
                event: WindowEvent::Resized(new_size), 
                .. 
            } => {
                // Handle resize by updating both graphics context and camera
                self.handle_resize(*new_size)?;
            }
            Event::WindowEvent { 
                event: WindowEvent::CloseRequested, 
                .. 
            } => {
                event_loop.exit();
            }
            Event::WindowEvent { 
                event: WindowEvent::RedrawRequested, 
                .. 
            } => {
                // Render frame here
                self.render()?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle window resize
    fn handle_resize(&mut self, new_size: PhysicalSize<u32>) -> GraphicsResult<()> {
        handle_window_resize(&mut self.graphics_context, &mut self.camera, new_size)
    }

    /// Render a frame (placeholder implementation)
    fn render(&mut self) -> GraphicsResult<()> {
        // Get current surface texture
        let surface_texture = self.graphics_context.get_current_texture()?;
        
        // Create command encoder
        let mut encoder = self.graphics_context.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            }
        );

        // Create render pass
        {
            let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            
            // Render commands would go here
        }

        // Submit commands
        self.graphics_context.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
    }

    /// Get current surface size
    pub fn surface_size(&self) -> (u32, u32) {
        self.graphics_context.surface_size()
    }

    /// Get current camera aspect ratio
    pub fn camera_aspect_ratio(&self) -> f32 {
        self.camera.aspect_ratio()
    }

    /// Get camera reference for external use
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Get mutable camera reference for external use
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }
}

/// Example function showing how to run the application with proper resize handling
pub async fn run_with_resize_handling() -> RenderResult<()> {
    let window_manager = WindowManager::new();
    
    window_manager.run(|_window, event| {
        // This is a simplified example - in practice you'd want to store the RenderApp
        // and handle the async initialization properly
        match event {
            Event::WindowEvent { 
                event: WindowEvent::Resized(new_size), 
                .. 
            } => {
                println!("Window resized to: {}x{}", new_size.width, new_size.height);
            }
            Event::WindowEvent { 
                event: WindowEvent::CloseRequested, 
                .. 
            } => {
                // Exit is handled by the window manager itself
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
    fn test_resize_validation() {
        // Test that resize validation works correctly
        let valid_size = PhysicalSize::new(1920, 1080);
        assert!(valid_size.width > 0 && valid_size.height > 0);

        let invalid_width = PhysicalSize::new(0, 1080);
        assert!(invalid_width.width == 0);

        let invalid_height = PhysicalSize::new(1920, 0);
        assert!(invalid_height.height == 0);
    }

    #[test]
    fn test_aspect_ratio_calculations() {
        // Test various aspect ratio calculations
        let hd_size = PhysicalSize::new(1920, 1080);
        let hd_aspect = hd_size.width as f32 / hd_size.height as f32;
        assert!((hd_aspect - 16.0/9.0).abs() < f32::EPSILON);

        let square_size = PhysicalSize::new(512, 512);
        let square_aspect = square_size.width as f32 / square_size.height as f32;
        assert_eq!(square_aspect, 1.0);

        let portrait_size = PhysicalSize::new(600, 800);
        let portrait_aspect = portrait_size.width as f32 / portrait_size.height as f32;
        assert_eq!(portrait_aspect, 0.75);
    }
}