//! Shader compilation and management

use crate::renderer::{ShaderError, ShaderResult};
use bytemuck::{Pod, Zeroable};
use std::mem;

/// Vertex data structure matching the WGSL shader input
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    /// Returns the vertex buffer layout descriptor for wgpu
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position attribute at location 0
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Color attribute at location 1
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

/// Uniform data structure for transformation matrices
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
}

impl Uniforms {
    /// Create new uniforms with identity matrix
    pub fn new() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
    
    /// Create uniforms from camera view-projection matrix
    pub fn from_camera(camera: &crate::renderer::Camera) -> Self {
        Self {
            view_proj: camera.view_projection_matrix().to_cols_array_2d(),
        }
    }
    
    /// Update the view-projection matrix
    pub fn update_view_proj(&mut self, view_proj: glam::Mat4) {
        self.view_proj = view_proj.to_cols_array_2d();
    }
}

impl Default for Uniforms {
    fn default() -> Self {
        Self::new()
    }
}

/// Triangle geometry for basic rendering test
pub const TRIANGLE_VERTICES: &[Vertex] = &[
    Vertex { position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },    // Top - Red
    Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },  // Bottom Left - Green  
    Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },   // Bottom Right - Blue
];

/// Shader module management
pub struct ShaderModule {
    module: wgpu::ShaderModule,
}

impl ShaderModule {
    /// Create a new shader module from WGSL source code
    pub fn from_wgsl(device: &wgpu::Device, source: &str, label: Option<&str>) -> ShaderResult<Self> {
        // Validate that source is not empty
        if source.trim().is_empty() {
            return Err(ShaderError::InvalidSource("Shader source is empty".to_string()));
        }
        
        // Basic validation for WGSL syntax
        if !source.contains("@vertex") && !source.contains("@fragment") {
            return Err(ShaderError::InvalidSource(
                "Shader source must contain at least one @vertex or @fragment function".to_string()
            ));
        }
        
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label,
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        
        Ok(Self { module })
    }
    
    /// Create a shader module from embedded WGSL source
    pub fn from_embedded_wgsl(device: &wgpu::Device, source: &str, label: Option<&str>) -> ShaderResult<Self> {
        Self::from_wgsl(device, source, label)
    }
    
    /// Get the underlying wgpu shader module
    pub fn module(&self) -> &wgpu::ShaderModule {
        &self.module
    }
}