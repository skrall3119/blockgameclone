//! Single chunk rendering example application
//!
//! This example demonstrates how to render a single chunk with various block types
//! using the chunk rendering system. It serves as a proof of concept and validation
//! of the chunk rendering pipeline.

use anyhow::Result;
use log::info;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId, WindowAttributes},
    dpi::PhysicalSize,
};
use world::glam::Vec3;

use engine::renderer::{RenderResult, RenderError, GraphicsContext, Camera};
use world::rendering::{ChunkRenderer, ChunkUniforms, SingleChunkDemo};
use world::chunk::BlockID;

/// Main entry point for the single chunk rendering example
pub async fn run_single_chunk_example() -> Result<()> {
    env_logger::init();
    
    info!("Starting single chunk rendering example...");
    
    let event_loop = EventLoop::new()
        .map_err(|e| RenderError::Window(engine::renderer::WindowError::EventLoopCreation(e.to_string())))?;
    
    let mut app = SingleChunkExampleApp::new();
    
    event_loop.run_app(&mut app)
        .map_err(|e| RenderError::Window(engine::renderer::WindowError::EventLoopCreation(e.to_string())))?;
    
    Ok(())
}

/// Single chunk example application
struct SingleChunkExampleApp {
    window: Option<Window>,
    render_state: Option<RenderState>,
}

/// Rendering state for the single chunk example
struct RenderState {
    graphics_context: GraphicsContext<'static>,
    camera: Camera,
    chunk_renderer: ChunkRenderer,
    chunk_demo: SingleChunkDemo,
    current_demo_type: DemoType,
}

/// Different types of chunk demonstrations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DemoType {
    Variety,    // Chunk with various block types and patterns
    Empty,      // Empty chunk (all air)
    Filled,     // Completely filled chunk
    Mixed,      // Mixed pattern chunk
}

impl DemoType {
    /// Get the next demo type in sequence
    fn next(self) -> Self {
        match self {
            DemoType::Variety => DemoType::Empty,
            DemoType::Empty => DemoType::Filled,
            DemoType::Filled => DemoType::Mixed,
            DemoType::Mixed => DemoType::Variety,
        }
    }

    /// Get a description of this demo type
    fn description(self) -> &'static str {
        match self {
            DemoType::Variety => "Variety (layered blocks with patterns)",
            DemoType::Empty => "Empty (all air blocks)",
            DemoType::Filled => "Filled (all stone blocks)",
            DemoType::Mixed => "Mixed (pattern of different block types)",
        }
    }
}

impl SingleChunkExampleApp {
    fn new() -> Self {
        Self {
            window: None,
            render_state: None,
        }
    }

    /// Initialize rendering state when window is created
    async fn init_render_state(&mut self, window: &Window) -> RenderResult<()> {
        info!("Initializing single chunk rendering example...");
        
        // SAFETY: We're using 'static lifetime here because the window will live
        // for the entire duration of the application. This is safe because we
        // control the window lifecycle in this application.
        let window_ref: &'static Window = unsafe { std::mem::transmute(window) };
        
        let graphics_context = GraphicsContext::new(window_ref).await
            .map_err(RenderError::Graphics)?;
        
        // Initialize camera with current window aspect ratio
        let (width, height) = graphics_context.surface_size();
        let aspect_ratio = width as f32 / height as f32;
        let mut camera = Camera::new(aspect_ratio);
        
        // Position camera to view the chunk at origin
        // Move camera back and up to get a good view of the chunk
        camera.set_position(Vec3::new(8.0, 12.0, 20.0));
        camera.set_target(Vec3::new(8.0, 8.0, 8.0));
        
        info!("Camera initialized at position {:?}", camera.position());
        
        // Create chunk renderer
        info!("Creating chunk renderer...");
        let chunk_renderer = ChunkRenderer::new(
            std::sync::Arc::new(graphics_context.device.clone()), 
            &graphics_context.config
        ).map_err(|e| RenderError::OperationFailed(format!("Failed to create chunk renderer: {:?}", e)))?;
        info!("Chunk renderer created successfully");
        
        // Create initial chunk demonstration
        let chunk_demo = SingleChunkDemo::new();
        let current_demo_type = DemoType::Variety;
        
        info!("Created chunk demo: {}", current_demo_type.description());
        
        // Log chunk statistics
        let stats = chunk_demo.get_statistics();
        info!("Chunk statistics:");
        info!("  Total blocks: {}", stats.total_blocks);
        info!("  Non-air blocks: {}", stats.non_air_blocks());
        info!("  Fill percentage: {:.1}%", stats.fill_percentage());
        info!("  Vertices: {}", stats.vertices);
        info!("  Indices: {}", stats.indices);
        info!("  Triangles: {}", stats.triangles);
        info!("  Vertices per block: {:.1}", stats.vertices_per_block());
        
        // Validate the chunk demonstration
        if let Err(e) = chunk_demo.validate() {
            log::error!("Chunk validation failed: {}", e);
            return Err(RenderError::OperationFailed(format!("Chunk validation failed: {}", e)));
        }
        info!("Chunk validation passed");
        
        self.render_state = Some(RenderState {
            graphics_context,
            camera,
            chunk_renderer,
            chunk_demo,
            current_demo_type,
        });
        
        // Prepare the chunk for rendering
        self.prepare_chunk_for_rendering()?;
        
        Ok(())
    }

    /// Prepare the current chunk for rendering by uploading mesh data to GPU
    fn prepare_chunk_for_rendering(&mut self) -> RenderResult<()> {
        if let Some(render_state) = &mut self.render_state {
            info!("Preparing chunk for rendering...");
            
            render_state.chunk_demo.prepare_for_rendering(
                &mut render_state.chunk_renderer,
                &render_state.graphics_context.queue,
            ).map_err(|e| RenderError::OperationFailed(format!("Failed to prepare chunk: {:?}", e)))?;
            
            info!("Chunk prepared for rendering successfully");
        }
        Ok(())
    }

    /// Switch to the next demonstration type
    fn switch_demo_type(&mut self) -> RenderResult<()> {
        if let Some(render_state) = &mut self.render_state {
            let new_demo_type = render_state.current_demo_type.next();
            
            info!("Switching to demo type: {}", new_demo_type.description());
            
            // Create new chunk demonstration based on type
            render_state.chunk_demo = match new_demo_type {
                DemoType::Variety => SingleChunkDemo::new(),
                DemoType::Empty => SingleChunkDemo::empty(),
                DemoType::Filled => SingleChunkDemo::filled(BlockID::Stone),
                DemoType::Mixed => SingleChunkDemo::mixed_pattern(),
            };
            
            render_state.current_demo_type = new_demo_type;
            
            // Log new statistics
            let stats = render_state.chunk_demo.get_statistics();
            info!("New chunk statistics:");
            info!("  Non-air blocks: {}", stats.non_air_blocks());
            info!("  Fill percentage: {:.1}%", stats.fill_percentage());
            info!("  Vertices: {}", stats.vertices);
            info!("  Triangles: {}", stats.triangles);
            
            // Prepare new chunk for rendering
            self.prepare_chunk_for_rendering()?;
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
            
            // Create depth texture for 3D rendering
            let depth_texture = render_state.graphics_context.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width: surface_texture.texture.width(),
                    height: surface_texture.texture.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            
            let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
            
            // Create command encoder
            let mut encoder = render_state.graphics_context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
            
            // Begin render pass
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                
                // Update uniforms with camera matrices
                let view_proj_matrix = render_state.camera.view_projection_matrix();
                let view_proj_array: [[f32; 4]; 4] = view_proj_matrix.to_cols_array_2d();
                render_state.chunk_renderer.update_uniforms(
                    &render_state.graphics_context.queue,
                    &ChunkUniforms::new(view_proj_array, [0.0, 0.0, 0.0])
                );
                
                // Render the chunk
                render_state.chunk_demo.render(
                    &render_state.chunk_renderer,
                    &mut render_pass,
                    view_proj_array,
                ).map_err(|e| RenderError::OperationFailed(format!("Failed to render chunk: {:?}", e)))?;
            }
            
            // Submit commands
            render_state.graphics_context.queue.submit(std::iter::once(encoder.finish()));
            
            // Present the frame
            surface_texture.present();
        }
        Ok(())
    }
}

impl Drop for SingleChunkExampleApp {
    fn drop(&mut self) {
        info!("Cleaning up single chunk example application");
    }
}

impl ApplicationHandler for SingleChunkExampleApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = WindowAttributes::default()
                .with_title("Single Chunk Rendering Example - Press SPACE to switch demos")
                .with_inner_size(PhysicalSize::new(1024, 768));
            
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
                    WindowEvent::KeyboardInput { event, .. } => {
                        if event.state == winit::event::ElementState::Pressed {
                            match event.physical_key {
                                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Space) => {
                                    if let Err(e) = self.switch_demo_type() {
                                        log::error!("Error switching demo type: {}", e);
                                    }
                                }
                                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape) => {
                                    info!("Escape pressed - shutting down");
                                    event_loop.exit();
                                }
                                _ => {}
                            }
                        }
                        false
                    }
                    WindowEvent::RedrawRequested => {
                        if let Err(e) = self.render_frame() {
                            log::error!("Render error: {}", e);
                        }
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

/// Convenience function to run the single chunk example
pub fn main() -> Result<()> {
    pollster::block_on(run_single_chunk_example())
}