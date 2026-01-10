//! Render pipeline management

use crate::renderer::{ShaderError, ShaderResult};
use crate::renderer::shader::{Vertex, Uniforms, ShaderModule, TRIANGLE_VERTICES};
use wgpu::util::DeviceExt;

/// Render pipeline that manages shaders, buffers, and rendering state
pub struct ShaderPipeline {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    uniforms: Uniforms,
}

impl ShaderPipeline {
    /// Create a new shader pipeline with the basic triangle shader
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> ShaderResult<Self> {
        // Load the basic shader with error handling
        let shader_source = include_str!("../../../../assets/shaders/basic.wgsl");
        let shader_module = ShaderModule::from_embedded_wgsl(device, shader_source, Some("Basic Shader"))
            .map_err(|e| ShaderError::CompilationFailed(format!("Failed to compile basic shader: {}", e)))?;
        
        // Create uniform buffer
        let uniforms = Uniforms::new();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        // Create bind group layout for uniforms
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });
        
        // Validate surface format
        if !Self::is_supported_format(surface_format) {
            return Err(ShaderError::PipelineCreation(format!(
                "Unsupported surface format: {:?}. Supported formats are: Bgra8UnormSrgb, Rgba8UnormSrgb",
                surface_format
            )));
        }
        
        // Create render pipeline with error handling
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Basic Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader_module.module(),
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader_module.module(),
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });
        
        // Create vertex buffer with triangle data
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(TRIANGLE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        Ok(Self {
            render_pipeline,
            vertex_buffer,
            uniform_buffer,
            bind_group,
            uniforms,
        })
    }
    
    /// Check if a texture format is supported for rendering
    fn is_supported_format(format: wgpu::TextureFormat) -> bool {
        matches!(format, 
            wgpu::TextureFormat::Bgra8UnormSrgb | 
            wgpu::TextureFormat::Rgba8UnormSrgb |
            wgpu::TextureFormat::Bgra8Unorm |
            wgpu::TextureFormat::Rgba8Unorm
        )
    }
    
    /// Update the uniform buffer with new transformation matrices
    pub fn update_uniforms(&mut self, queue: &wgpu::Queue, uniforms: &Uniforms) {
        self.uniforms = *uniforms;
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }
    
    /// Render the triangle using this pipeline
    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Basic Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
        
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..TRIANGLE_VERTICES.len() as u32, 0..1);
    }
    
    /// Render a complete frame with command submission to GPU queue
    pub fn render_frame(&self, device: &wgpu::Device, queue: &wgpu::Queue, view: &wgpu::TextureView) -> ShaderResult<()> {
        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        // Encode render commands
        self.render(&mut encoder, view);
        
        // Submit commands to GPU queue
        queue.submit(std::iter::once(encoder.finish()));
        
        Ok(())
    }
    
    /// Get the current uniforms
    pub fn uniforms(&self) -> &Uniforms {
        &self.uniforms
    }
    
    /// Create a shader pipeline from custom WGSL source
    pub fn from_wgsl(
        device: &wgpu::Device, 
        surface_format: wgpu::TextureFormat,
        vertex_source: &str,
        fragment_source: &str,
        vertex_entry: &str,
        fragment_entry: &str,
    ) -> ShaderResult<Self> {
        // Validate inputs
        if vertex_source.trim().is_empty() {
            return Err(ShaderError::InvalidSource("Vertex shader source is empty".to_string()));
        }
        if fragment_source.trim().is_empty() {
            return Err(ShaderError::InvalidSource("Fragment shader source is empty".to_string()));
        }
        if vertex_entry.trim().is_empty() {
            return Err(ShaderError::InvalidSource("Vertex entry point is empty".to_string()));
        }
        if fragment_entry.trim().is_empty() {
            return Err(ShaderError::InvalidSource("Fragment entry point is empty".to_string()));
        }
        
        // Create shader modules
        let vertex_module = ShaderModule::from_wgsl(device, vertex_source, Some("Custom Vertex Shader"))
            .map_err(|e| ShaderError::CompilationFailed(format!("Vertex shader compilation failed: {}", e)))?;
            
        let fragment_module = ShaderModule::from_wgsl(device, fragment_source, Some("Custom Fragment Shader"))
            .map_err(|e| ShaderError::CompilationFailed(format!("Fragment shader compilation failed: {}", e)))?;
        
        // Validate surface format
        if !Self::is_supported_format(surface_format) {
            return Err(ShaderError::PipelineCreation(format!(
                "Unsupported surface format: {:?}",
                surface_format
            )));
        }
        
        // Create uniform buffer
        let uniforms = Uniforms::new();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Custom Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Custom Uniform Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Custom Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Custom Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });
        
        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Custom Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: vertex_module.module(),
                entry_point: Some(vertex_entry),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: fragment_module.module(),
                entry_point: Some(fragment_entry),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });
        
        // Create vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Custom Vertex Buffer"),
            contents: bytemuck::cast_slice(TRIANGLE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        Ok(Self {
            render_pipeline,
            vertex_buffer,
            uniform_buffer,
            bind_group,
            uniforms,
        })
    }
}