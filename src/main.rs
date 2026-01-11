use anyhow::Result;
use log::info;
use engine::renderer::{RenderResult, RenderError, GraphicsContext, Camera};
use engine::renderer::pipeline::ShaderPipeline;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId, WindowAttributes},
    dpi::PhysicalSize,
};

mod terrain_generation_example;
mod world_integration_example;

fn main() -> Result<()> {
    env_logger::init();
    
    // Check command line arguments for example selection
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 {
        match args[1].as_str() {
            "terrain" => {
                info!("Running terrain generation example...");
                return terrain_generation_example::run_terrain_generation_example();
            }
            "world" => {
                info!("Running world integration example...");
                return pollster::block_on(world_integration_example::run_world_integration_example());
            }
            "help" | "--help" | "-h" => {
                println!("Voxel Game Examples");
                println!("Usage: cargo run [example]");
                println!();
                println!("Available examples:");
                println!("  terrain  - Terrain generation demonstration");
                println!("  world    - World integration demonstration");
                println!("  (none)   - Run main voxel game application");
                return Ok(());
            }
            _ => {
                println!("Unknown example: {}", args[1]);
                println!("Use 'cargo run help' to see available examples.");
                return Ok(());
            }
        }
    }
    
    info!("Starting voxel game...");
    
    // Run the main application loop
    pollster::block_on(run_application())?;
    
    Ok(())
}

/// Main application loop that integrates window manager with graphics context
async fn run_application() -> RenderResult<()> {
    info!("Initializing event loop...");
    let event_loop = EventLoop::new()
        .map_err(|e| RenderError::Window(engine::renderer::WindowError::EventLoopCreation(e.to_string())))?;
    
    let mut app = VoxelGameApp::new();
    
    event_loop.run_app(&mut app)
        .map_err(|e| RenderError::Window(engine::renderer::WindowError::EventLoopCreation(e.to_string())))?;
    
    Ok(())
}

/// Main application that handles window creation and rendering
struct VoxelGameApp {
    window: Option<Window>,
    render_state: Option<RenderState>,
}

/// Rendering state that manages graphics context, camera, and pipeline
struct RenderState {
    graphics_context: GraphicsContext<'static>,
    camera: Camera,
    shader_pipeline: ShaderPipeline,
}

impl VoxelGameApp {
    fn new() -> Self {
        Self {
            window: None,
            render_state: None,
        }
    }
    
    /// Initialize rendering state when window is created
    async fn init_render_state(&mut self, window: &Window) -> RenderResult<()> {
        info!("Initializing graphics context...");
        
        // SAFETY: We're using 'static lifetime here because the window will live
        // for the entire duration of the application. This is safe because we
        // control the window lifecycle in this application.
        let window_ref: &'static Window = unsafe { std::mem::transmute(window) };
        
        let graphics_context = GraphicsContext::new(window_ref).await
            .map_err(RenderError::Graphics)?;
        
        // Initialize camera with current window aspect ratio
        let (width, height) = graphics_context.surface_size();
        let aspect_ratio = width as f32 / height as f32;
        let camera = Camera::new(aspect_ratio);
        info!("Camera initialized with aspect ratio: {:.3}", aspect_ratio);
        
        // Create shader pipeline
        info!("Creating shader pipeline...");
        let surface_format = graphics_context.surface_format();
        let shader_pipeline = ShaderPipeline::new(&graphics_context.device, surface_format)
            .map_err(RenderError::Shader)?;
        info!("Shader pipeline created successfully");
        
        self.render_state = Some(RenderState {
            graphics_context,
            camera,
            shader_pipeline,
        });
        
        // Validate that all components are properly integrated
        self.validate_integration()?;
        
        Ok(())
    }
    
    /// Validate that all components are properly integrated
    fn validate_integration(&self) -> RenderResult<()> {
        if let Some(render_state) = &self.render_state {
            // Verify graphics context and camera aspect ratios match
            let (width, height) = render_state.graphics_context.surface_size();
            let expected_aspect = width as f32 / height as f32;
            let camera_aspect = render_state.camera.aspect_ratio();
            
            if (expected_aspect - camera_aspect).abs() > f32::EPSILON {
                log::warn!(
                    "Aspect ratio mismatch: graphics context {:.3}, camera {:.3}",
                    expected_aspect, camera_aspect
                );
            }
            
            // Verify surface format compatibility
            let surface_format = render_state.graphics_context.surface_format();
            info!("Surface format: {:?}", surface_format);
            
            info!("All components successfully integrated and validated");
        }
        Ok(())
    }
    
    /// Handle window resize
    fn handle_resize(&mut self, new_size: PhysicalSize<u32>) -> RenderResult<()> {
        if let Some(render_state) = &mut self.render_state {
            info!("Handling window resize to: {}x{}", new_size.width, new_size.height);
            
            // Update graphics context surface configuration
            render_state.graphics_context.resize(new_size.width, new_size.height)
                .map_err(RenderError::Graphics)?;
            
            // Update camera aspect ratio
            if new_size.height > 0 {
                let aspect_ratio = new_size.width as f32 / new_size.height as f32;
                render_state.camera.update_aspect_ratio(aspect_ratio);
                info!("Camera aspect ratio updated to: {:.3}", aspect_ratio);
                
                // Validate that resize was handled correctly
                let (width, height) = render_state.graphics_context.surface_size();
                if width != new_size.width || height != new_size.height {
                    log::warn!(
                        "Surface size mismatch after resize: expected {}x{}, got {}x{}",
                        new_size.width, new_size.height, width, height
                    );
                }
            }
        }
        Ok(())
    }
    
    /// Render a frame
    fn render_frame(&mut self) -> RenderResult<()> {
        if let Some(render_state) = &mut self.render_state {
            // Get current surface texture with error recovery
            let surface_texture = match render_state.graphics_context.get_current_texture() {
                Ok(texture) => texture,
                Err(engine::renderer::GraphicsError::SurfaceLost) => {
                    info!("Surface lost, attempting to reconfigure");
                    render_state.graphics_context.reconfigure_surface()
                        .map_err(RenderError::Graphics)?;
                    render_state.graphics_context.get_current_texture()
                        .map_err(RenderError::Graphics)?
                }
                Err(e) => return Err(RenderError::Graphics(e)),
            };
            
            // Create texture view
            let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
            
            // Update shader uniforms with camera matrices - this connects camera to shaders
            let uniforms = engine::renderer::shader::Uniforms::from_camera(&render_state.camera);
            render_state.shader_pipeline.update_uniforms(&render_state.graphics_context.queue, &uniforms);
            
            // Render frame using shader pipeline - this integrates triangle rendering
            render_state.shader_pipeline.render_frame(
                &render_state.graphics_context.device,
                &render_state.graphics_context.queue,
                &view
            ).map_err(RenderError::Shader)?;
            
            // Present the frame
            surface_texture.present();
        }
        Ok(())
    }
}

impl Drop for VoxelGameApp {
    fn drop(&mut self) {
        info!("Cleaning up application resources");
        // Explicit cleanup - wgpu resources are automatically cleaned up
        // when they go out of scope, but we can log the cleanup
        if self.render_state.is_some() {
            info!("Cleaning up render state");
        }
        if self.window.is_some() {
            info!("Cleaning up window");
        }
    }
}

impl ApplicationHandler for VoxelGameApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = WindowAttributes::default()
                .with_title("Voxel Game")
                .with_inner_size(PhysicalSize::new(800, 600));
            
            match event_loop.create_window(window_attributes) {
                Ok(window) => {
                    info!("Window created successfully");
                    
                    // Initialize render state asynchronously
                    match pollster::block_on(self.init_render_state(&window)) {
                        Ok(()) => {
                            info!("Render state initialized successfully");
                            self.window = Some(window);
                        }
                        Err(e) => {
                            log::error!("Failed to initialize render state: {}", e);
                            event_loop.exit();
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to create window: {}", e);
                    event_loop.exit();
                }
            }
        }
    }
    
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let should_request_redraw = if let Some(window) = &self.window {
            if window.id() == window_id {
                match event {
                    WindowEvent::CloseRequested => {
                        info!("Window close requested - shutting down");
                        event_loop.exit();
                        false
                    }
                    WindowEvent::Resized(new_size) => {
                        if let Err(e) = self.handle_resize(new_size) {
                            log::error!("Error handling resize: {}", e);
                        }
                        false
                    }
                    WindowEvent::RedrawRequested => {
                        if let Err(e) = self.render_frame() {
                            log::error!("Render error: {}", e);
                        }
                        
                        // Request next frame
                        true
                    }
                    _ => false
                }
            } else {
                false
            }
        } else {
            false
        };
        
        // Request redraw outside of the borrow
        if should_request_redraw {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
    
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Request redraw to maintain continuous rendering
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}