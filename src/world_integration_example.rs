//! World Integration Example Application
//!
//! This example demonstrates the complete world integration system including:
//! - Multi-chunk world creation and management
//! - Dynamic chunk loading with different patterns
//! - Integration with the rendering pipeline
//! - Performance monitoring and statistics display
//! - Memory management and cleanup
//! - Frustum culling and render distance optimization

use anyhow::Result;
use log::info;
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId, WindowAttributes},
    dpi::PhysicalSize,
};
use world::glam::Vec3;

use engine::renderer::{RenderResult, RenderError, GraphicsContext, Camera};
use world::rendering::{ChunkRenderer, ChunkUniforms};
use world::{
    World, WorldConfig, LoadPattern, ChunkCoord,
};

/// Main entry point for the world integration example
pub async fn run_world_integration_example() -> Result<()> {
    env_logger::init();
    
    info!("Starting world integration example...");
    
    let event_loop = EventLoop::new()
        .map_err(|e| RenderError::Window(engine::renderer::WindowError::EventLoopCreation(e.to_string())))?;
    
    let mut app = WorldIntegrationApp::new();
    
    event_loop.run_app(&mut app)
        .map_err(|e| RenderError::Window(engine::renderer::WindowError::EventLoopCreation(e.to_string())))?;
    
    Ok(())
}

/// World integration example application
struct WorldIntegrationApp {
    window: Option<Window>,
    render_state: Option<RenderState>,
}

/// Rendering state for the world integration example
struct RenderState {
    graphics_context: GraphicsContext<'static>,
    camera: Camera,
    chunk_renderer: ChunkRenderer,
    world: World,
    demo_controller: DemoController,
    performance_tracker: PerformanceTracker,
    last_frame_time: Instant,
}

/// Controls different demonstration modes
struct DemoController {
    current_mode: DemoMode,
    center_chunk: ChunkCoord,
    render_distance: u32,
    auto_mode: bool,
    last_mode_switch: Instant,
    mode_duration: Duration,
}

/// Different demonstration modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DemoMode {
    SingleChunk,        // Load and render a single chunk
    SmallGrid,          // 3x3 grid of chunks
    MediumGrid,         // 5x5 grid of chunks
    LargeGrid,          // 7x7 grid of chunks
    DynamicLoading,     // Simulate player movement with dynamic loading
    MemoryStress,       // Test memory management under load
    FrustumCulling,     // Demonstrate frustum culling
}

impl DemoMode {
    /// Get the next demo mode in sequence
    fn next(self) -> Self {
        match self {
            DemoMode::SingleChunk => DemoMode::SmallGrid,
            DemoMode::SmallGrid => DemoMode::MediumGrid,
            DemoMode::MediumGrid => DemoMode::LargeGrid,
            DemoMode::LargeGrid => DemoMode::DynamicLoading,
            DemoMode::DynamicLoading => DemoMode::MemoryStress,
            DemoMode::MemoryStress => DemoMode::FrustumCulling,
            DemoMode::FrustumCulling => DemoMode::SingleChunk,
        }
    }

    /// Get a description of this demo mode
    fn description(self) -> &'static str {
        match self {
            DemoMode::SingleChunk => "Single Chunk - Basic world integration",
            DemoMode::SmallGrid => "Small Grid (3x3) - Multi-chunk rendering",
            DemoMode::MediumGrid => "Medium Grid (5x5) - Moderate world size",
            DemoMode::LargeGrid => "Large Grid (7x7) - Large world demonstration",
            DemoMode::DynamicLoading => "Dynamic Loading - Simulated player movement",
            DemoMode::MemoryStress => "Memory Stress - Memory management testing",
            DemoMode::FrustumCulling => "Frustum Culling - Rendering optimization",
        }
    }

    /// Get the grid radius for this mode
    fn grid_radius(self) -> u32 {
        match self {
            DemoMode::SingleChunk => 0,
            DemoMode::SmallGrid => 1,
            DemoMode::MediumGrid => 2,
            DemoMode::LargeGrid => 3,
            DemoMode::DynamicLoading => 2,
            DemoMode::MemoryStress => 4,
            DemoMode::FrustumCulling => 3,
        }
    }
}

/// Tracks performance metrics and statistics
struct PerformanceTracker {
    frame_times: Vec<Duration>,
    frame_count: u64,
    last_stats_update: Instant,
    stats_interval: Duration,
    current_fps: f32,
    avg_frame_time: Duration,
    min_frame_time: Duration,
    max_frame_time: Duration,
}

impl PerformanceTracker {
    fn new() -> Self {
        Self {
            frame_times: Vec::with_capacity(120), // Store last 2 seconds at 60fps
            frame_count: 0,
            last_stats_update: Instant::now(),
            stats_interval: Duration::from_millis(500), // Update stats every 500ms
            current_fps: 0.0,
            avg_frame_time: Duration::ZERO,
            min_frame_time: Duration::MAX,
            max_frame_time: Duration::ZERO,
        }
    }

    fn record_frame(&mut self, frame_time: Duration) {
        self.frame_count += 1;
        self.frame_times.push(frame_time);

        // Keep only recent frame times
        if self.frame_times.len() > 120 {
            self.frame_times.remove(0);
        }

        // Update min/max
        self.min_frame_time = self.min_frame_time.min(frame_time);
        self.max_frame_time = self.max_frame_time.max(frame_time);

        // Update statistics periodically
        if self.last_stats_update.elapsed() >= self.stats_interval {
            self.update_statistics();
            self.last_stats_update = Instant::now();
        }
    }

    fn update_statistics(&mut self) {
        if !self.frame_times.is_empty() {
            let total_time: Duration = self.frame_times.iter().sum();
            self.avg_frame_time = total_time / self.frame_times.len() as u32;
            
            if self.avg_frame_time.as_secs_f32() > 0.0 {
                self.current_fps = 1.0 / self.avg_frame_time.as_secs_f32();
            }
        }
    }

    fn get_stats(&self) -> PerformanceStats {
        PerformanceStats {
            fps: self.current_fps,
            avg_frame_time_ms: self.avg_frame_time.as_secs_f32() * 1000.0,
            min_frame_time_ms: self.min_frame_time.as_secs_f32() * 1000.0,
            max_frame_time_ms: self.max_frame_time.as_secs_f32() * 1000.0,
            frame_count: self.frame_count,
        }
    }
}

/// Performance statistics snapshot
#[derive(Debug, Clone)]
struct PerformanceStats {
    fps: f32,
    avg_frame_time_ms: f32,
    min_frame_time_ms: f32,
    max_frame_time_ms: f32,
    frame_count: u64,
}

impl DemoController {
    fn new() -> Self {
        Self {
            current_mode: DemoMode::SingleChunk,
            center_chunk: ChunkCoord::new(0, 0, 0),
            render_distance: 8,
            auto_mode: false,
            last_mode_switch: Instant::now(),
            mode_duration: Duration::from_secs(10), // Switch modes every 10 seconds in auto mode
        }
    }

    fn should_switch_mode(&self) -> bool {
        self.auto_mode && self.last_mode_switch.elapsed() >= self.mode_duration
    }

    fn switch_to_next_mode(&mut self) {
        self.current_mode = self.current_mode.next();
        self.last_mode_switch = Instant::now();
        info!("Switched to demo mode: {}", self.current_mode.description());
    }

    fn toggle_auto_mode(&mut self) {
        self.auto_mode = !self.auto_mode;
        info!("Auto mode: {}", if self.auto_mode { "enabled" } else { "disabled" });
        if self.auto_mode {
            self.last_mode_switch = Instant::now();
        }
    }
}

impl WorldIntegrationApp {
    fn new() -> Self {
        Self {
            window: None,
            render_state: None,
        }
    }

    /// Initialize rendering state when window is created
    async fn init_render_state(&mut self, window: &Window) -> RenderResult<()> {
        info!("Initializing world integration example...");
        
        // SAFETY: We're using 'static lifetime here because the window will live
        // for the entire duration of the application.
        let window_ref: &'static Window = unsafe { std::mem::transmute(window) };
        
        let graphics_context = GraphicsContext::new(window_ref).await
            .map_err(RenderError::Graphics)?;
        
        // Initialize camera with current window aspect ratio
        let (width, height) = graphics_context.surface_size();
        let aspect_ratio = width as f32 / height as f32;
        let mut camera = Camera::new(aspect_ratio);
        
        // Position camera to view the world from above and at an angle
        camera.set_position(Vec3::new(32.0, 48.0, 64.0));
        camera.set_target(Vec3::new(0.0, 16.0, 0.0));
        
        info!("Camera initialized at position {:?}", camera.position());
        
        // Create chunk renderer
        info!("Creating chunk renderer...");
        let chunk_renderer = ChunkRenderer::new(
            std::sync::Arc::new(graphics_context.device.clone()), 
            &graphics_context.config
        ).map_err(|e| RenderError::OperationFailed(format!("Failed to create chunk renderer: {:?}", e)))?;
        info!("Chunk renderer created successfully");
        
        // Create world with configuration optimized for demonstration
        let world_config = WorldConfig::new()
            .with_render_distance(8)
            .with_max_chunks(Some(200)) // Allow up to 200 chunks for large demonstrations
            .with_performance_monitoring(true);
        
        let world = World::new(world_config)
            .map_err(|e| RenderError::OperationFailed(format!("Failed to create world: {:?}", e)))?;
        
        info!("World created with configuration: render_distance={}, max_chunks={:?}", 
              world.config().render_distance, world.config().max_chunks_loaded);
        
        // Initialize demo controller and performance tracker
        let demo_controller = DemoController::new();
        let performance_tracker = PerformanceTracker::new();
        
        self.render_state = Some(RenderState {
            graphics_context,
            camera,
            chunk_renderer,
            world,
            demo_controller,
            performance_tracker,
            last_frame_time: Instant::now(),
        });
        
        // Load initial chunks for the first demo mode
        self.setup_demo_mode()?;
        
        Ok(())
    }

    /// Setup chunks for the current demo mode
    fn setup_demo_mode(&mut self) -> RenderResult<()> {
        if let Some(render_state) = &mut self.render_state {
            let mode = render_state.demo_controller.current_mode;
            let center = render_state.demo_controller.center_chunk;
            
            info!("Setting up demo mode: {}", mode.description());
            
            // Clear existing chunks if switching modes
            let current_chunks: Vec<ChunkCoord> = render_state.world.get_render_ready_chunks()
                .into_iter()
                .map(|(coord, _)| coord)
                .collect();
            for coord in current_chunks {
                render_state.world.remove_chunk(coord);
            }
            
            // Load chunks based on demo mode
            let loading_result = match mode {
                DemoMode::SingleChunk => {
                    render_state.world.load_single_chunk(center)
                }
                DemoMode::SmallGrid | DemoMode::MediumGrid | DemoMode::LargeGrid | 
                DemoMode::DynamicLoading | DemoMode::FrustumCulling => {
                    let radius = mode.grid_radius();
                    render_state.world.load_grid(center, radius)
                }
                DemoMode::MemoryStress => {
                    // Load a larger grid to test memory management
                    render_state.world.load_grid(center, 4)
                }
            };
            
            match loading_result {
                Ok(progress) => {
                    info!("Chunk loading completed:");
                    info!("  Total chunks: {}", progress.total_chunks());
                    info!("  Successful loads: {}", progress.successful_loads());
                    info!("  Failed loads: {}", progress.failed_loads());
                    info!("  Completion: {:.1}%", progress.completion_percentage() * 100.0);
                    
                    if progress.failed_loads() > 0 {
                        let failed_chunks = progress.failed_chunks();
                        info!("  Failed chunk coordinates: {:?}", failed_chunks);
                    }
                }
                Err(e) => {
                    log::error!("Failed to load chunks for demo mode: {}", e);
                    return Err(RenderError::OperationFailed(format!("Chunk loading failed: {}", e)));
                }
            }
            
            // Prepare all loaded chunks for rendering
            self.prepare_chunks_for_rendering()?;
            
            // Log world statistics
            self.log_world_statistics();
        }
        
        Ok(())
    }

    /// Prepare all loaded chunks for rendering
    fn prepare_chunks_for_rendering(&mut self) -> RenderResult<()> {
        if let Some(render_state) = &mut self.render_state {
            let chunk_coords: Vec<ChunkCoord> = render_state.world.get_render_ready_chunks()
                .into_iter()
                .map(|(coord, _)| coord)
                .collect();
            
            info!("Preparing {} chunks for rendering...", chunk_coords.len());
            
            let preparation_results = render_state.world.prepare_chunks_for_rendering(&chunk_coords);
            
            let mut successful_preparations = 0;
            let mut failed_preparations = 0;
            
            for (coord, result) in preparation_results {
                match result {
                    Ok(state_changed) => {
                        successful_preparations += 1;
                        if state_changed {
                            // Mark chunk as render-ready
                            if let Err(e) = render_state.world.mark_chunk_render_ready(coord) {
                                log::warn!("Failed to mark chunk {:?} as render-ready: {}", coord, e);
                            }
                        }
                    }
                    Err(e) => {
                        failed_preparations += 1;
                        log::warn!("Failed to prepare chunk {:?} for rendering: {}", coord, e);
                    }
                }
            }
            
            info!("Chunk preparation completed: {} successful, {} failed", 
                  successful_preparations, failed_preparations);
        }
        
        Ok(())
    }

    /// Log comprehensive world statistics
    fn log_world_statistics(&self) {
        if let Some(render_state) = &self.render_state {
            let loading_stats = render_state.world.get_loading_stats();
            let memory_stats = render_state.world.memory_stats();
            let performance_stats = render_state.performance_tracker.get_stats();
            
            info!("=== World Statistics ===");
            info!("Chunks:");
            info!("  Total loaded: {}", loading_stats.total_chunks);
            info!("  Render ready: {}", loading_stats.render_ready_chunks);
            info!("  Generated: {}", loading_stats.generated_chunks);
            info!("  Meshed: {}", loading_stats.meshed_chunks);
            info!("  Loading: {}", loading_stats.loading_chunks);
            info!("  Error: {}", loading_stats.error_chunks);
            
            info!("Memory:");
            info!("  Current usage: {:.2} MB", memory_stats.current_usage as f32 / 1024.0 / 1024.0);
            info!("  Total budget: {:.2} MB", memory_stats.total_budget as f32 / 1024.0 / 1024.0);
            info!("  Usage percentage: {:.1}%", memory_stats.usage_fraction * 100.0);
            info!("  Chunks tracked: {}", memory_stats.chunks_tracked);
            
            info!("Performance:");
            info!("  FPS: {:.1}", performance_stats.fps);
            info!("  Avg frame time: {:.2} ms", performance_stats.avg_frame_time_ms);
            info!("  Frame count: {}", performance_stats.frame_count);
            
            // Check for any issues
            if loading_stats.error_chunks > 0 {
                log::warn!("Warning: {} chunks are in error state", loading_stats.error_chunks);
            }
            
            if memory_stats.is_critical() {
                log::warn!("Warning: Memory usage is critical ({:.1}%)", memory_stats.usage_fraction * 100.0);
            }
            
            info!("========================");
        }
    }

    /// Handle demo mode switching and updates
    fn update_demo_mode(&mut self) -> RenderResult<()> {
        let should_switch = if let Some(render_state) = &self.render_state {
            render_state.demo_controller.should_switch_mode()
        } else {
            false
        };

        if should_switch {
            if let Some(render_state) = &mut self.render_state {
                render_state.demo_controller.switch_to_next_mode();
            }
            self.setup_demo_mode()?;
        }

        // Handle special demo mode behaviors
        let current_mode = if let Some(render_state) = &self.render_state {
            render_state.demo_controller.current_mode
        } else {
            return Ok(());
        };

        match current_mode {
            DemoMode::DynamicLoading => {
                self.simulate_dynamic_loading()?;
            }
            DemoMode::MemoryStress => {
                self.simulate_memory_stress()?;
            }
            _ => {}
        }
        
        Ok(())
    }

    /// Simulate dynamic loading by moving the center chunk
    fn simulate_dynamic_loading(&mut self) -> RenderResult<()> {
        let (time, current_center) = if let Some(render_state) = &self.render_state {
            (render_state.last_frame_time.elapsed().as_secs_f32(), 
             render_state.demo_controller.center_chunk)
        } else {
            return Ok(());
        };

        // Move center chunk in a circular pattern
        let radius = 2.0;
        let speed = 0.5;
        
        let x = (time * speed).cos() * radius;
        let z = (time * speed).sin() * radius;
        
        let new_center = ChunkCoord::new(x as i32, 0, z as i32);
        
        if new_center != current_center {
            if let Some(render_state) = &mut self.render_state {
                render_state.demo_controller.center_chunk = new_center;
                
                // Load chunks around new center
                let pattern = LoadPattern::Grid { 
                    center: new_center, 
                    radius: render_state.demo_controller.current_mode.grid_radius() 
                };
                
                if let Ok(progress) = render_state.world.load_chunks(pattern) {
                    if progress.successful_loads() > 0 {
                        // Need to prepare chunks, but we can't call self method here
                        // So we'll do it inline
                        let chunk_coords: Vec<ChunkCoord> = render_state.world.get_render_ready_chunks()
                            .into_iter()
                            .map(|(coord, _)| coord)
                            .collect();
                        
                        let _preparation_results = render_state.world.prepare_chunks_for_rendering(&chunk_coords);
                    }
                }
                
                // Clean up distant chunks to manage memory
                let _ = render_state.world.cleanup_memory_if_needed();
            }
        }
        
        Ok(())
    }

    /// Simulate memory stress by loading and unloading chunks
    fn simulate_memory_stress(&mut self) -> RenderResult<()> {
        if let Some(render_state) = &mut self.render_state {
            // Periodically load new chunks and clean up old ones
            let time = render_state.last_frame_time.elapsed().as_secs_f32();
            
            if (time as u32) % 3 == 0 { // Every 3 seconds
                // Load some random chunks
                let random_coords = vec![
                    ChunkCoord::new((time as i32) % 10 - 5, 0, (time as i32 * 2) % 10 - 5),
                    ChunkCoord::new((time as i32 * 3) % 10 - 5, 0, (time as i32) % 10 - 5),
                ];
                
                let pattern = LoadPattern::Custom(random_coords);
                let _ = render_state.world.load_chunks(pattern);
                
                // Force memory cleanup
                let _ = render_state.world.cleanup_memory_if_needed();
            }
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

    /// Render a frame with world integration
    fn render_frame(&mut self) -> RenderResult<()> {
        let frame_start = Instant::now();
        
        // Update demo mode
        self.update_demo_mode()?;

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
                label: Some("World Integration Render Encoder"),
            });
            
            // Begin render pass
            {
                let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("World Integration Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.2,
                                g: 0.3,
                                b: 0.8,
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
                
                // Render all render-ready chunks
                let render_ready_chunks = render_state.world.get_render_ready_chunks();
                
                for (coord, _entry) in render_ready_chunks {
                    // Calculate world position for this chunk
                    let world_pos = render_state.world.chunk_to_world_pos(coord);
                    let chunk_uniforms = ChunkUniforms::new(
                        view_proj_array,
                        [world_pos.x, world_pos.y, world_pos.z]
                    );
                    
                    // Update uniforms for this chunk
                    render_state.chunk_renderer.update_uniforms(
                        &render_state.graphics_context.queue,
                        &chunk_uniforms
                    );
                    
                    // Render the chunk (this would normally use the chunk's mesh data)
                    // For now, we'll use a placeholder rendering approach
                    // In a complete implementation, this would render the actual chunk mesh
                }
            }
            
            // Submit commands
            render_state.graphics_context.queue.submit(std::iter::once(encoder.finish()));
            
            // Present the frame
            surface_texture.present();
            
            // Record performance metrics
            let frame_time = frame_start.elapsed();
            render_state.performance_tracker.record_frame(frame_time);
            render_state.last_frame_time = frame_start;
        }
        Ok(())
    }

    /// Switch to the next demo mode manually
    fn switch_demo_mode(&mut self) -> RenderResult<()> {
        if let Some(render_state) = &mut self.render_state {
            render_state.demo_controller.switch_to_next_mode();
            self.setup_demo_mode()?;
        }
        Ok(())
    }

    /// Toggle auto mode for automatic demo switching
    fn toggle_auto_mode(&mut self) {
        if let Some(render_state) = &mut self.render_state {
            render_state.demo_controller.toggle_auto_mode();
        }
    }

    /// Print current statistics to console
    fn print_statistics(&self) {
        if let Some(render_state) = &self.render_state {
            let loading_stats = render_state.world.get_loading_stats();
            let memory_stats = render_state.world.memory_stats();
            let performance_stats = render_state.performance_tracker.get_stats();
            
            println!("\n=== World Integration Statistics ===");
            println!("Demo Mode: {}", render_state.demo_controller.current_mode.description());
            println!("Auto Mode: {}", if render_state.demo_controller.auto_mode { "ON" } else { "OFF" });
            println!();
            println!("Chunks: {} total, {} render-ready, {} loading", 
                     loading_stats.total_chunks, 
                     loading_stats.render_ready_chunks,
                     loading_stats.loading_chunks);
            println!("Memory: {:.1} MB / {:.1} MB ({:.1}%)", 
                     memory_stats.current_usage as f32 / 1024.0 / 1024.0,
                     memory_stats.total_budget as f32 / 1024.0 / 1024.0,
                     memory_stats.usage_fraction * 100.0);
            println!("Performance: {:.1} FPS, {:.2} ms avg frame time", 
                     performance_stats.fps, 
                     performance_stats.avg_frame_time_ms);
            println!("=====================================\n");
        }
    }
}

impl Drop for WorldIntegrationApp {
    fn drop(&mut self) {
        info!("Cleaning up world integration example application");
        
        // Log final statistics
        if let Some(render_state) = &self.render_state {
            let final_stats = render_state.performance_tracker.get_stats();
            info!("Final performance statistics:");
            info!("  Total frames rendered: {}", final_stats.frame_count);
            info!("  Average FPS: {:.1}", final_stats.fps);
            info!("  Average frame time: {:.2} ms", final_stats.avg_frame_time_ms);
        }
    }
}

impl ApplicationHandler for WorldIntegrationApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = WindowAttributes::default()
                .with_title("World Integration Example - SPACE: next demo, A: auto mode, S: stats")
                .with_inner_size(PhysicalSize::new(1200, 800));
            
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
                                    if let Err(e) = self.switch_demo_mode() {
                                        log::error!("Error switching demo mode: {}", e);
                                    }
                                }
                                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyA) => {
                                    self.toggle_auto_mode();
                                }
                                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyS) => {
                                    self.print_statistics();
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

/// Convenience function to run the world integration example
pub fn main() -> Result<()> {
    pollster::block_on(run_world_integration_example())
}