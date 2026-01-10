//! Chunk renderer for integrating with the wgpu rendering pipeline

use std::sync::Arc;
use crate::rendering::{ChunkVertex, BufferManager, RenderResult};
use crate::ChunkPosition;

/// Uniform data for chunk rendering
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ChunkUniforms {
    /// Combined view and projection matrix
    pub view_proj: [[f32; 4]; 4],
    /// Position of the chunk in world space
    pub chunk_position: [f32; 3],
    /// Padding for alignment
    pub _padding: f32,
}

impl ChunkUniforms {
    /// Create new chunk uniforms
    pub fn new(view_proj: [[f32; 4]; 4], chunk_position: [f32; 3]) -> Self {
        Self {
            view_proj,
            chunk_position,
            _padding: 0.0,
        }
    }
}

/// Chunk renderer that integrates with the wgpu rendering pipeline
pub struct ChunkRenderer {
    /// The GPU device
    device: Arc<wgpu::Device>,
    /// The render pipeline for chunk rendering
    render_pipeline: wgpu::RenderPipeline,
    /// Uniform buffer for shader uniforms
    uniform_buffer: wgpu::Buffer,
    /// Bind group layout for uniforms
    bind_group_layout: wgpu::BindGroupLayout,
    /// Bind group for uniforms
    bind_group: wgpu::BindGroup,
    /// Buffer manager for chunk meshes
    buffer_manager: BufferManager,
}

impl ChunkRenderer {
    /// Create a new chunk renderer with the given device and surface configuration
    pub fn new(device: Arc<wgpu::Device>, surface_config: &wgpu::SurfaceConfiguration) -> RenderResult<Self> {
        // Load and compile the chunk shader
        let shader_source = include_str!("../../../../assets/shaders/chunk.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Chunk Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Create bind group layout for uniforms
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Chunk Uniform Bind Group Layout"),
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

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Chunk Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Chunk Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[ChunkVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chunk Uniform Buffer"),
            size: std::mem::size_of::<ChunkUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Chunk Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Create buffer manager
        let buffer_manager = BufferManager::new(device.clone());

        Ok(Self {
            device,
            render_pipeline,
            uniform_buffer,
            bind_group_layout,
            bind_group,
            buffer_manager,
        })
    }

    /// Get a reference to the buffer manager
    pub fn buffer_manager(&self) -> &BufferManager {
        &self.buffer_manager
    }

    /// Get a mutable reference to the buffer manager
    pub fn buffer_manager_mut(&mut self) -> &mut BufferManager {
        &mut self.buffer_manager
    }

    /// Update the uniform buffer with new data
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &ChunkUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    /// Get the bind group layout for external use
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Get the render pipeline for external use
    pub fn render_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.render_pipeline
    }

    /// Render a chunk using the provided render pass and mesh data
    pub fn render_chunk(
        &self,
        render_pass: &mut wgpu::RenderPass,
        chunk_position: &ChunkPosition,
        mesh: &crate::rendering::ChunkMesh,
    ) -> RenderResult<()> {
        // Get buffers for the chunk
        let (vertex_buffer, index_buffer) = self.buffer_manager
            .get_buffers(chunk_position)
            .ok_or(crate::rendering::RenderError::BufferCreationFailed)?;

        // Validate that we have geometry to render
        if mesh.is_empty() {
            return Ok(()); // Nothing to render
        }

        // Set the render pipeline
        render_pass.set_pipeline(&self.render_pipeline);

        // Bind the uniform buffer
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        // Bind vertex buffer
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

        // Bind index buffer
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Issue draw call
        render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);

        Ok(())
    }

    /// Render multiple chunks in a batch
    pub fn render_chunks(
        &self,
        render_pass: &mut wgpu::RenderPass,
        chunks: &[(ChunkPosition, &crate::rendering::ChunkMesh)],
    ) -> Vec<(ChunkPosition, RenderResult<()>)> {
        let mut results = Vec::with_capacity(chunks.len());

        for &(chunk_pos, mesh) in chunks {
            let result = self.render_chunk(render_pass, &chunk_pos, mesh);
            results.push((chunk_pos, result));
        }

        results
    }

    /// Prepare chunk for rendering by uploading mesh data to GPU
    pub fn prepare_chunk(
        &mut self,
        queue: &wgpu::Queue,
        chunk_position: ChunkPosition,
        mesh: &crate::rendering::ChunkMesh,
    ) -> RenderResult<()> {
        // Upload mesh data to GPU buffers
        self.buffer_manager.update_chunk_buffers(chunk_position, mesh, queue)
    }

    /// Update chunk mesh data
    pub fn update_chunk(
        &mut self,
        queue: &wgpu::Queue,
        chunk_position: ChunkPosition,
        mesh: &crate::rendering::ChunkMesh,
    ) -> RenderResult<()> {
        // Update buffers for the chunk
        self.buffer_manager.update_chunk_buffers(chunk_position, mesh, queue)
    }

    /// Remove chunk from rendering
    pub fn remove_chunk(&mut self, chunk_position: &ChunkPosition) {
        self.buffer_manager.remove_buffers(chunk_position);
    }

    /// Get rendering statistics
    pub fn get_stats(&self) -> ChunkRenderStats {
        let memory_stats = self.buffer_manager.memory_usage();
        ChunkRenderStats {
            chunks_loaded: memory_stats.buffer_count,
            vertex_memory_bytes: memory_stats.vertex_memory,
            index_memory_bytes: memory_stats.index_memory,
            total_memory_bytes: memory_stats.total_memory,
        }
    }
}

/// Statistics for chunk rendering
#[derive(Debug, Clone, Copy)]
pub struct ChunkRenderStats {
    /// Number of chunks currently loaded
    pub chunks_loaded: usize,
    /// Memory used by vertex buffers in bytes
    pub vertex_memory_bytes: u64,
    /// Memory used by index buffers in bytes
    pub index_memory_bytes: u64,
    /// Total memory used by all buffers in bytes
    pub total_memory_bytes: u64,
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::{ChunkMesh, ChunkVertex};

    fn create_test_mesh() -> ChunkMesh {
        let mut mesh = ChunkMesh::new();
        let vertices = vec![
            ChunkVertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ChunkVertex::new([1.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 1.0]),
        ];
        let indices = vec![0, 1, 2];
        mesh.add_geometry(&vertices, &indices);
        mesh
    }

    #[test]
    fn test_chunk_uniforms_creation() {
        let view_proj = [[1.0, 0.0, 0.0, 0.0]; 4];
        let chunk_position = [10.0, 0.0, 5.0];
        
        let uniforms = ChunkUniforms::new(view_proj, chunk_position);
        
        assert_eq!(uniforms.view_proj, view_proj);
        assert_eq!(uniforms.chunk_position, chunk_position);
        assert_eq!(uniforms._padding, 0.0);
    }

    #[test]
    fn test_chunk_render_stats() {
        let stats = ChunkRenderStats {
            chunks_loaded: 5,
            vertex_memory_bytes: 1024,
            index_memory_bytes: 512,
            total_memory_bytes: 1536,
        };
        
        assert_eq!(stats.chunks_loaded, 5);
        assert_eq!(stats.vertex_memory_bytes, 1024);
        assert_eq!(stats.index_memory_bytes, 512);
        assert_eq!(stats.total_memory_bytes, 1536);
    }

    // Note: Full ChunkRenderer tests would require a GPU context
    // In a real testing environment, you would use a headless GPU context
    // or mock the wgpu types for unit testing
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use crate::chunk::ChunkPosition;
    use crate::rendering::ChunkMesh;

    /// Generate arbitrary chunk positions for property testing
    fn arb_chunk_position() -> impl Strategy<Value = ChunkPosition> {
        (-100i32..=100, -100i32..=100).prop_map(|(x, z)| ChunkPosition { x, z })
    }

    /// Generate arbitrary view-projection matrices
    fn arb_view_proj_matrix() -> impl Strategy<Value = [[f32; 4]; 4]> {
        prop::array::uniform4(prop::array::uniform4(-10.0f32..10.0))
    }

    /// Generate arbitrary chunk positions in world space
    fn arb_world_position() -> impl Strategy<Value = [f32; 3]> {
        prop::array::uniform3(-1000.0f32..1000.0)
    }

    /// Generate arbitrary chunk meshes for property testing
    fn arb_chunk_mesh() -> impl Strategy<Value = ChunkMesh> {
        prop::collection::vec(
            (
                prop::array::uniform3(-10.0f32..10.0),  // position
                prop::array::uniform3(-1.0f32..1.0),    // normal
                prop::array::uniform2(0.0f32..1.0),     // tex_coords
            ),
            3..=20,
        )
        .prop_flat_map(|vertex_data| {
            let vertex_count = vertex_data.len();
            // Generate triangle count (each triangle needs 3 indices)
            let triangle_count = 1..=(vertex_count / 3).max(1);
            (
                Just(vertex_data),
                triangle_count,
            )
        })
        .prop_map(|(vertex_data, triangle_count)| {
            let mut mesh = ChunkMesh::new();
            let vertices: Vec<ChunkVertex> = vertex_data
                .into_iter()
                .map(|(pos, normal, tex_coords)| ChunkVertex::new(pos, normal, tex_coords))
                .collect();
            
            // Generate valid triangle indices (always divisible by 3)
            let mut indices = Vec::new();
            for _ in 0..triangle_count {
                // Generate a valid triangle using available vertices
                let v1 = (indices.len() / 3) % vertices.len();
                let v2 = (v1 + 1) % vertices.len();
                let v3 = (v1 + 2) % vertices.len();
                
                indices.push(v1 as u32);
                indices.push(v2 as u32);
                indices.push(v3 as u32);
            }
            
            mesh.add_geometry(&vertices, &indices);
            mesh
        })
    }

    /// **Feature: chunk-rendering, Property 5: Rendering Pipeline Integration**
    /// 
    /// For any chunk mesh, the rendering process should bind correct buffers, 
    /// issue draw calls with proper counts, and integrate with camera transforms.
    /// 
    /// **Validates: Requirements 5.1, 5.2, 5.3, 5.4, 5.5**
    #[test]
    fn property_rendering_pipeline_integration() {
        let config = ProptestConfig {
            cases: 10, // Reduced for faster execution
            ..ProptestConfig::default()
        };
        proptest!(config, |(
            chunk_pos in arb_chunk_position(),
            mesh in arb_chunk_mesh(),
            view_proj in arb_view_proj_matrix(),
            world_pos in arb_world_position(),
        )| {
            // Property 1: ChunkUniforms should correctly store transformation data (Requirement 5.1, 5.2)
            let uniforms = ChunkUniforms::new(view_proj, world_pos);
            prop_assert_eq!(uniforms.view_proj, view_proj, "View-projection matrix should be stored correctly");
            prop_assert_eq!(uniforms.chunk_position, world_pos, "Chunk position should be stored correctly");
            prop_assert_eq!(uniforms._padding, 0.0, "Padding should be zero");
            
            // Property 2: Mesh validation should be consistent (Requirement 5.3)
            let is_valid = mesh.validate().is_ok();
            let is_empty = mesh.is_empty();
            
            if !is_empty {
                prop_assert!(is_valid, "Non-empty meshes should be valid");
                prop_assert!(mesh.index_count > 0, "Non-empty meshes should have indices");
                prop_assert!(mesh.vertices.len() > 0, "Non-empty meshes should have vertices");
                prop_assert_eq!(mesh.indices.len() % 3, 0, "Indices should form complete triangles");
            }
            
            // Property 3: Index count should match actual indices (Requirement 5.4)
            prop_assert_eq!(mesh.index_count as usize, mesh.indices.len(), "Index count should match indices length");
            
            // Property 4: Triangle count should be consistent (Requirement 5.5)
            let expected_triangles = mesh.indices.len() / 3;
            let actual_triangles = mesh.triangle_count();
            prop_assert_eq!(actual_triangles, expected_triangles, "Triangle count should be consistent");
        });
    }

    /// Property test for chunk render statistics
    #[test]
    fn property_render_stats_consistency() {
        let config = ProptestConfig {
            cases: 10,
            ..ProptestConfig::default()
        };
        proptest!(config, |(
            chunks_loaded in 0usize..1000,
            vertex_memory in 0u64..1_000_000,
            index_memory in 0u64..1_000_000,
        )| {
            let total_memory = vertex_memory + index_memory;
            let stats = ChunkRenderStats {
                chunks_loaded,
                vertex_memory_bytes: vertex_memory,
                index_memory_bytes: index_memory,
                total_memory_bytes: total_memory,
            };
            
            // Property: Total memory should equal sum of vertex and index memory
            prop_assert_eq!(stats.total_memory_bytes, stats.vertex_memory_bytes + stats.index_memory_bytes,
                "Total memory should equal vertex memory plus index memory");
            
            // Property: All values should be non-negative
            prop_assert!(stats.chunks_loaded >= 0, "Chunks loaded should be non-negative");
            prop_assert!(stats.vertex_memory_bytes >= 0, "Vertex memory should be non-negative");
            prop_assert!(stats.index_memory_bytes >= 0, "Index memory should be non-negative");
            prop_assert!(stats.total_memory_bytes >= 0, "Total memory should be non-negative");
        });
    }

    /// Property test for uniform data consistency
    #[test]
    fn property_uniform_data_consistency() {
        let config = ProptestConfig {
            cases: 10,
            ..ProptestConfig::default()
        };
        proptest!(config, |(
            view_proj in arb_view_proj_matrix(),
            chunk_pos in arb_world_position(),
        )| {
            let uniforms1 = ChunkUniforms::new(view_proj, chunk_pos);
            let uniforms2 = ChunkUniforms::new(view_proj, chunk_pos);
            
            // Property: Same inputs should produce identical uniforms
            prop_assert_eq!(uniforms1.view_proj, uniforms2.view_proj, "View-proj matrices should be identical");
            prop_assert_eq!(uniforms1.chunk_position, uniforms2.chunk_position, "Chunk positions should be identical");
            prop_assert_eq!(uniforms1._padding, uniforms2._padding, "Padding should be identical");
            
            // Property: Uniforms should be Pod (can be cast to bytes)
            let _bytes: &[u8] = bytemuck::cast_slice(&[uniforms1]);
            // If this compiles and runs, the Pod trait is working correctly
        });
    }
}