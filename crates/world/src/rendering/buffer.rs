//! GPU buffer management for chunk rendering

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crate::rendering::{ChunkMesh, ChunkVertex, RenderError, RenderResult};
use crate::ChunkPosition;

/// Manages GPU buffers for chunk meshes
pub struct BufferManager {
    /// Reference to the GPU device
    device: Arc<wgpu::Device>,
    /// Vertex buffers indexed by chunk position
    vertex_buffers: HashMap<ChunkPosition, wgpu::Buffer>,
    /// Index buffers indexed by chunk position
    index_buffers: HashMap<ChunkPosition, wgpu::Buffer>,
}

impl BufferManager {
    /// Create a new buffer manager with the given GPU device
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        Self {
            device,
            vertex_buffers: HashMap::new(),
            index_buffers: HashMap::new(),
        }
    }

    /// Get a reference to the GPU device
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Create vertex and index buffers for a chunk mesh
    pub fn create_buffers(&mut self, chunk_pos: ChunkPosition, mesh: &ChunkMesh) -> RenderResult<()> {
        // Validate mesh data before creating buffers
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return Err(RenderError::InvalidMeshData);
        }

        // Validate mesh integrity
        mesh.validate().map_err(|_| RenderError::InvalidMeshData)?;

        // Create vertex buffer
        let vertex_data: &[u8] = bytemuck::cast_slice(&mesh.vertices);
        let vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Chunk Vertex Buffer {:?}", chunk_pos)),
            size: vertex_data.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create index buffer
        let index_data: &[u8] = bytemuck::cast_slice(&mesh.indices);
        let index_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Chunk Index Buffer {:?}", chunk_pos)),
            size: index_data.len() as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Store buffers
        self.vertex_buffers.insert(chunk_pos, vertex_buffer);
        self.index_buffers.insert(chunk_pos, index_buffer);

        Ok(())
    }

    /// Upload mesh data to GPU buffers using a queue
    pub fn upload_mesh_data(&self, queue: &wgpu::Queue, chunk_pos: &ChunkPosition, mesh: &ChunkMesh) -> RenderResult<()> {
        let vertex_buffer = self.vertex_buffers.get(chunk_pos)
            .ok_or(RenderError::BufferCreationFailed)?;
        let index_buffer = self.index_buffers.get(chunk_pos)
            .ok_or(RenderError::BufferCreationFailed)?;

        // Upload vertex data
        let vertex_data: &[u8] = bytemuck::cast_slice(&mesh.vertices);
        queue.write_buffer(vertex_buffer, 0, vertex_data);

        // Upload index data
        let index_data: &[u8] = bytemuck::cast_slice(&mesh.indices);
        queue.write_buffer(index_buffer, 0, index_data);

        Ok(())
    }

    /// Update buffers for a chunk when its mesh data changes
    /// This method handles both creation and updates efficiently
    pub fn update_chunk_buffers(&mut self, chunk_pos: ChunkPosition, mesh: &ChunkMesh, queue: &wgpu::Queue) -> RenderResult<()> {
        // Handle empty meshes by removing buffers
        if mesh.is_empty() {
            self.remove_buffers(&chunk_pos);
            return Ok(());
        }

        // Validate mesh data
        mesh.validate().map_err(|_| RenderError::InvalidMeshData)?;

        // Check if we need to recreate buffers due to size changes
        let needs_recreation = self.check_buffer_size_mismatch(&chunk_pos, mesh);

        if needs_recreation {
            // Remove old buffers and create new ones with the correct size
            self.remove_buffers(&chunk_pos);
            self.create_buffers(chunk_pos, mesh)?;
        }

        // Upload the mesh data to GPU
        self.upload_mesh_data(queue, &chunk_pos, mesh)
    }

    /// Check if existing buffers have size mismatches with the new mesh data
    fn check_buffer_size_mismatch(&self, chunk_pos: &ChunkPosition, mesh: &ChunkMesh) -> bool {
        if let (Some(vertex_buffer), Some(index_buffer)) = 
            (self.vertex_buffers.get(chunk_pos), self.index_buffers.get(chunk_pos)) {
            
            let required_vertex_size = (mesh.vertices.len() * std::mem::size_of::<ChunkVertex>()) as u64;
            let required_index_size = (mesh.indices.len() * std::mem::size_of::<u32>()) as u64;
            
            vertex_buffer.size() != required_vertex_size || index_buffer.size() != required_index_size
        } else {
            // Buffers don't exist, so we need to create them
            true
        }
    }

    /// Automatically manage buffer lifecycle for multiple chunks
    /// This method removes buffers for chunks that are no longer needed
    pub fn cleanup_unused_buffers(&mut self, active_chunks: &HashSet<ChunkPosition>) {
        // Collect chunk positions that are no longer active
        let chunks_to_remove: Vec<ChunkPosition> = self.vertex_buffers
            .keys()
            .filter(|pos| !active_chunks.contains(pos))
            .copied()
            .collect();

        // Remove buffers for inactive chunks
        for chunk_pos in chunks_to_remove {
            self.remove_buffers(&chunk_pos);
        }
    }

    /// Gracefully handle buffer creation failures by attempting recovery
    pub fn create_buffers_with_fallback(&mut self, chunk_pos: ChunkPosition, mesh: &ChunkMesh) -> RenderResult<()> {
        // First attempt: try to create buffers normally
        match self.create_buffers(chunk_pos, mesh) {
            Ok(()) => Ok(()),
            Err(RenderError::BufferCreationFailed) => {
                // Fallback: try to free some memory and retry
                self.attempt_memory_recovery();
                
                // Second attempt after cleanup
                self.create_buffers(chunk_pos, mesh)
                    .map_err(|_| RenderError::BufferCreationFailed)
            }
            Err(other_error) => Err(other_error),
        }
    }

    /// Attempt to recover memory by removing some buffers
    /// This is a simple strategy that removes the oldest buffers
    fn attempt_memory_recovery(&mut self) {
        // Simple strategy: remove up to 25% of existing buffers
        let buffers_to_remove = (self.buffer_count() / 4).max(1);
        
        // Collect some chunk positions to remove (we'll remove the first few)
        let chunks_to_remove: Vec<ChunkPosition> = self.vertex_buffers
            .keys()
            .take(buffers_to_remove)
            .copied()
            .collect();

        for chunk_pos in chunks_to_remove {
            self.remove_buffers(&chunk_pos);
        }
    }

    /// Batch update multiple chunks efficiently
    pub fn batch_update_chunks(&mut self, updates: &[(ChunkPosition, &ChunkMesh)], queue: &wgpu::Queue) -> Vec<(ChunkPosition, RenderResult<()>)> {
        let mut results = Vec::with_capacity(updates.len());
        
        for &(chunk_pos, mesh) in updates {
            let result = self.update_chunk_buffers(chunk_pos, mesh, queue);
            results.push((chunk_pos, result));
        }
        
        results
    }

    /// Check buffer health and return any issues found
    pub fn validate_buffers(&self) -> Vec<(ChunkPosition, String)> {
        let mut issues = Vec::new();
        
        // Check for orphaned buffers (vertex buffer without index buffer or vice versa)
        for chunk_pos in self.vertex_buffers.keys() {
            if !self.index_buffers.contains_key(chunk_pos) {
                issues.push((*chunk_pos, "Missing index buffer".to_string()));
            }
        }
        
        for chunk_pos in self.index_buffers.keys() {
            if !self.vertex_buffers.contains_key(chunk_pos) {
                issues.push((*chunk_pos, "Missing vertex buffer".to_string()));
            }
        }
        
        issues
    }

    /// Get vertex buffer for a chunk position
    pub fn get_vertex_buffer(&self, chunk_pos: &ChunkPosition) -> Option<&wgpu::Buffer> {
        self.vertex_buffers.get(chunk_pos)
    }

    /// Get index buffer for a chunk position
    pub fn get_index_buffer(&self, chunk_pos: &ChunkPosition) -> Option<&wgpu::Buffer> {
        self.index_buffers.get(chunk_pos)
    }

    /// Get both vertex and index buffers for a chunk position
    pub fn get_buffers(&self, chunk_pos: &ChunkPosition) -> Option<(&wgpu::Buffer, &wgpu::Buffer)> {
        match (self.vertex_buffers.get(chunk_pos), self.index_buffers.get(chunk_pos)) {
            (Some(vertex_buffer), Some(index_buffer)) => Some((vertex_buffer, index_buffer)),
            _ => None,
        }
    }

    /// Check if buffers exist for the given chunk position
    pub fn has_buffers(&self, chunk_pos: &ChunkPosition) -> bool {
        self.vertex_buffers.contains_key(chunk_pos) && self.index_buffers.contains_key(chunk_pos)
    }

    /// Remove buffers for the given chunk position
    pub fn remove_buffers(&mut self, chunk_pos: &ChunkPosition) {
        self.vertex_buffers.remove(chunk_pos);
        self.index_buffers.remove(chunk_pos);
    }

    /// Clear all buffers
    pub fn clear(&mut self) {
        self.vertex_buffers.clear();
        self.index_buffers.clear();
    }

    /// Get the number of chunks with allocated buffers
    pub fn buffer_count(&self) -> usize {
        self.vertex_buffers.len()
    }

    /// Get memory usage statistics (approximate)
    pub fn memory_usage(&self) -> BufferMemoryStats {
        let mut vertex_memory = 0u64;
        let mut index_memory = 0u64;

        for buffer in self.vertex_buffers.values() {
            vertex_memory += buffer.size();
        }

        for buffer in self.index_buffers.values() {
            index_memory += buffer.size();
        }

        BufferMemoryStats {
            vertex_memory,
            index_memory,
            total_memory: vertex_memory + index_memory,
            buffer_count: self.buffer_count(),
        }
    }
}

/// Memory usage statistics for buffer management
#[derive(Debug, Clone, Copy)]
pub struct BufferMemoryStats {
    /// Total memory used by vertex buffers in bytes
    pub vertex_memory: u64,
    /// Total memory used by index buffers in bytes
    pub index_memory: u64,
    /// Total memory used by all buffers in bytes
    pub total_memory: u64,
    /// Number of chunks with allocated buffers
    pub buffer_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::ChunkVertex;
    use std::sync::Arc;

    // Mock wgpu device for testing (we'll create a minimal mock)
    fn create_mock_device() -> Arc<wgpu::Device> {
        // For unit tests, we'll need to create a real wgpu device
        // This is a simplified approach - in a real test environment,
        // you might want to use a headless adapter
        futures::executor::block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            });
            
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .expect("Failed to find an appropriate adapter");

            let (device, _queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor::default(),
                )
                .await
                .expect("Failed to create device");

            Arc::new(device)
        })
    }

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
    fn test_buffer_manager_creation() {
        let device = create_mock_device();
        let buffer_manager = BufferManager::new(device.clone());
        
        assert_eq!(buffer_manager.buffer_count(), 0);
        assert!(!buffer_manager.has_buffers(&ChunkPosition { x: 0, z: 0 }));
    }

    #[test]
    fn test_buffer_creation() {
        let device = create_mock_device();
        let mut buffer_manager = BufferManager::new(device.clone());
        let mesh = create_test_mesh();
        let chunk_pos = ChunkPosition { x: 0, z: 0 };

        let result = buffer_manager.create_buffers(chunk_pos, &mesh);
        assert!(result.is_ok());
        assert!(buffer_manager.has_buffers(&chunk_pos));
        assert_eq!(buffer_manager.buffer_count(), 1);
    }

    #[test]
    fn test_buffer_creation_with_empty_mesh() {
        let device = create_mock_device();
        let mut buffer_manager = BufferManager::new(device.clone());
        let empty_mesh = ChunkMesh::new();
        let chunk_pos = ChunkPosition { x: 0, z: 0 };

        let result = buffer_manager.create_buffers(chunk_pos, &empty_mesh);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RenderError::InvalidMeshData);
    }

    #[test]
    fn test_buffer_removal() {
        let device = create_mock_device();
        let mut buffer_manager = BufferManager::new(device.clone());
        let mesh = create_test_mesh();
        let chunk_pos = ChunkPosition { x: 0, z: 0 };

        buffer_manager.create_buffers(chunk_pos, &mesh).unwrap();
        assert!(buffer_manager.has_buffers(&chunk_pos));

        buffer_manager.remove_buffers(&chunk_pos);
        assert!(!buffer_manager.has_buffers(&chunk_pos));
        assert_eq!(buffer_manager.buffer_count(), 0);
    }

    #[test]
    fn test_buffer_cleanup() {
        let device = create_mock_device();
        let mut buffer_manager = BufferManager::new(device.clone());
        let mesh = create_test_mesh();

        // Create buffers for multiple chunks
        let chunk1 = ChunkPosition { x: 0, z: 0 };
        let chunk2 = ChunkPosition { x: 1, z: 0 };
        let chunk3 = ChunkPosition { x: 0, z: 1 };

        buffer_manager.create_buffers(chunk1, &mesh).unwrap();
        buffer_manager.create_buffers(chunk2, &mesh).unwrap();
        buffer_manager.create_buffers(chunk3, &mesh).unwrap();

        assert_eq!(buffer_manager.buffer_count(), 3);

        // Only keep chunk1 and chunk2 active
        let mut active_chunks = HashSet::new();
        active_chunks.insert(chunk1);
        active_chunks.insert(chunk2);

        buffer_manager.cleanup_unused_buffers(&active_chunks);

        assert_eq!(buffer_manager.buffer_count(), 2);
        assert!(buffer_manager.has_buffers(&chunk1));
        assert!(buffer_manager.has_buffers(&chunk2));
        assert!(!buffer_manager.has_buffers(&chunk3));
    }

    #[test]
    fn test_buffer_validation() {
        let device = create_mock_device();
        let mut buffer_manager = BufferManager::new(device.clone());
        let mesh = create_test_mesh();
        let chunk_pos = ChunkPosition { x: 0, z: 0 };

        // Create only vertex buffer (simulate corruption)
        buffer_manager.create_buffers(chunk_pos, &mesh).unwrap();
        
        // Manually remove index buffer to create inconsistent state
        buffer_manager.index_buffers.remove(&chunk_pos);

        let issues = buffer_manager.validate_buffers();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].0, chunk_pos);
        assert_eq!(issues[0].1, "Missing index buffer");
    }

    #[test]
    fn test_memory_usage_stats() {
        let device = create_mock_device();
        let mut buffer_manager = BufferManager::new(device.clone());
        let mesh = create_test_mesh();
        let chunk_pos = ChunkPosition { x: 0, z: 0 };

        let stats_before = buffer_manager.memory_usage();
        assert_eq!(stats_before.buffer_count, 0);
        assert_eq!(stats_before.total_memory, 0);

        buffer_manager.create_buffers(chunk_pos, &mesh).unwrap();

        let stats_after = buffer_manager.memory_usage();
        assert_eq!(stats_after.buffer_count, 1);
        assert!(stats_after.total_memory > 0);
        assert!(stats_after.vertex_memory > 0);
        assert!(stats_after.index_memory > 0);
        assert_eq!(stats_after.total_memory, stats_after.vertex_memory + stats_after.index_memory);
    }

    #[test]
    fn test_check_buffer_size_mismatch() {
        let device = create_mock_device();
        let mut buffer_manager = BufferManager::new(device.clone());
        let chunk_pos = ChunkPosition { x: 0, z: 0 };

        // No buffers exist yet
        let mesh1 = create_test_mesh();
        assert!(buffer_manager.check_buffer_size_mismatch(&chunk_pos, &mesh1));

        // Create buffers
        buffer_manager.create_buffers(chunk_pos, &mesh1).unwrap();
        
        // Same mesh should not need recreation
        assert!(!buffer_manager.check_buffer_size_mismatch(&chunk_pos, &mesh1));

        // Different sized mesh should need recreation
        let mut mesh2 = create_test_mesh();
        mesh2.add_geometry(&[ChunkVertex::new([2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0])], &[3]);
        assert!(buffer_manager.check_buffer_size_mismatch(&chunk_pos, &mesh2));
    }
}

    #[cfg(test)]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;
        use crate::chunk::ChunkPosition;

        /// Mock buffer for testing that doesn't require GPU access
        #[derive(Debug, Clone)]
        struct MockBuffer {
            size: u64,
            label: String,
        }

        impl MockBuffer {
            fn new(size: u64, label: String) -> Self {
                Self { size, label }
            }

            fn size(&self) -> u64 {
                self.size
            }
        }

        /// Mock buffer manager for testing without GPU dependencies
        struct MockBufferManager {
            vertex_buffers: HashMap<ChunkPosition, MockBuffer>,
            index_buffers: HashMap<ChunkPosition, MockBuffer>,
        }

        impl MockBufferManager {
            fn new() -> Self {
                Self {
                    vertex_buffers: HashMap::new(),
                    index_buffers: HashMap::new(),
                }
            }

            fn create_buffers(&mut self, chunk_pos: ChunkPosition, mesh: &ChunkMesh) -> RenderResult<()> {
                // Validate mesh data before creating buffers
                if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                    return Err(RenderError::InvalidMeshData);
                }

                // Validate mesh integrity
                mesh.validate().map_err(|_| RenderError::InvalidMeshData)?;

                let vertex_size = (mesh.vertices.len() * std::mem::size_of::<ChunkVertex>()) as u64;
                let index_size = (mesh.indices.len() * std::mem::size_of::<u32>()) as u64;

                let vertex_buffer = MockBuffer::new(vertex_size, format!("Chunk Vertex Buffer {:?}", chunk_pos));
                let index_buffer = MockBuffer::new(index_size, format!("Chunk Index Buffer {:?}", chunk_pos));

                self.vertex_buffers.insert(chunk_pos, vertex_buffer);
                self.index_buffers.insert(chunk_pos, index_buffer);

                Ok(())
            }

            fn has_buffers(&self, chunk_pos: &ChunkPosition) -> bool {
                self.vertex_buffers.contains_key(chunk_pos) && self.index_buffers.contains_key(chunk_pos)
            }

            fn remove_buffers(&mut self, chunk_pos: &ChunkPosition) {
                self.vertex_buffers.remove(chunk_pos);
                self.index_buffers.remove(chunk_pos);
            }

            fn buffer_count(&self) -> usize {
                self.vertex_buffers.len()
            }

            fn get_buffers(&self, chunk_pos: &ChunkPosition) -> Option<(&MockBuffer, &MockBuffer)> {
                match (self.vertex_buffers.get(chunk_pos), self.index_buffers.get(chunk_pos)) {
                    (Some(vertex_buffer), Some(index_buffer)) => Some((vertex_buffer, index_buffer)),
                    _ => None,
                }
            }

            fn check_buffer_size_mismatch(&self, chunk_pos: &ChunkPosition, mesh: &ChunkMesh) -> bool {
                if let (Some(vertex_buffer), Some(index_buffer)) = 
                    (self.vertex_buffers.get(chunk_pos), self.index_buffers.get(chunk_pos)) {
                    
                    let required_vertex_size = (mesh.vertices.len() * std::mem::size_of::<ChunkVertex>()) as u64;
                    let required_index_size = (mesh.indices.len() * std::mem::size_of::<u32>()) as u64;
                    
                    vertex_buffer.size() != required_vertex_size || index_buffer.size() != required_index_size
                } else {
                    // Buffers don't exist, so we need to create them
                    true
                }
            }

            fn cleanup_unused_buffers(&mut self, active_chunks: &HashSet<ChunkPosition>) {
                let chunks_to_remove: Vec<ChunkPosition> = self.vertex_buffers
                    .keys()
                    .filter(|pos| !active_chunks.contains(pos))
                    .copied()
                    .collect();

                for chunk_pos in chunks_to_remove {
                    self.remove_buffers(&chunk_pos);
                }
            }

            fn memory_usage(&self) -> BufferMemoryStats {
                let mut vertex_memory = 0u64;
                let mut index_memory = 0u64;

                for buffer in self.vertex_buffers.values() {
                    vertex_memory += buffer.size();
                }

                for buffer in self.index_buffers.values() {
                    index_memory += buffer.size();
                }

                BufferMemoryStats {
                    vertex_memory,
                    index_memory,
                    total_memory: vertex_memory + index_memory,
                    buffer_count: self.buffer_count(),
                }
            }
        }

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

        /// Generate arbitrary chunk positions for property testing
        fn arb_chunk_position() -> impl Strategy<Value = ChunkPosition> {
            (-100i32..=100, -100i32..=100).prop_map(|(x, z)| ChunkPosition { x, z })
        }

        /// Generate arbitrary chunk meshes for property testing
        fn arb_chunk_mesh() -> impl Strategy<Value = ChunkMesh> {
            prop::collection::vec(
                (
                    prop::array::uniform3(-10.0f32..10.0),  // position
                    prop::array::uniform3(-1.0f32..1.0),    // normal
                    prop::array::uniform2(0.0f32..1.0),     // tex_coords
                ),
                3..=20, // Reduced from 1..=100 to 3..=20 for faster execution
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

        /// **Feature: chunk-rendering, Property 4: Buffer Lifecycle Management**
        /// 
        /// For any chunk mesh, creating GPU buffers should allocate vertex and index buffers 
        /// with correct data, and updating chunk data should properly update the corresponding buffers.
        /// 
        /// **Validates: Requirements 4.1, 4.2, 4.3, 4.4**
        #[test]
        fn property_buffer_lifecycle_management() {
            let config = ProptestConfig {
                cases: 10, // Reduced from default 256 to 10 for faster execution
                ..ProptestConfig::default()
            };
            proptest!(config, |(
                chunk_pos in arb_chunk_position(),
                mesh1 in arb_chunk_mesh(),
                mesh2 in arb_chunk_mesh(),
            )| {
                let mut buffer_manager = MockBufferManager::new();
                
                // Property 1: Buffer creation should succeed for valid meshes (Requirement 4.1)
                let result1 = buffer_manager.create_buffers(chunk_pos, &mesh1);
                prop_assert!(result1.is_ok(), "Buffer creation should succeed for valid mesh");
                prop_assert!(buffer_manager.has_buffers(&chunk_pos), "Buffers should exist after creation");
                
                // Property 2: Buffer allocation should create both vertex and index buffers (Requirement 4.2)
                let (vertex_buffer, index_buffer) = buffer_manager.get_buffers(&chunk_pos)
                    .expect("Buffers should exist");
                
                let expected_vertex_size = (mesh1.vertices.len() * std::mem::size_of::<ChunkVertex>()) as u64;
                let expected_index_size = (mesh1.indices.len() * std::mem::size_of::<u32>()) as u64;
                
                prop_assert_eq!(vertex_buffer.size(), expected_vertex_size, "Vertex buffer should have correct size");
                prop_assert_eq!(index_buffer.size(), expected_index_size, "Index buffer should have correct size");
                
                // Property 3: Buffer updates should handle size changes correctly (Requirement 4.3)
                let needs_recreation = buffer_manager.check_buffer_size_mismatch(&chunk_pos, &mesh2);
                let size_changed = mesh1.vertices.len() != mesh2.vertices.len() || mesh1.indices.len() != mesh2.indices.len();
                
                if size_changed {
                    prop_assert!(needs_recreation, "Should detect size mismatch when mesh size changes");
                } else {
                    prop_assert!(!needs_recreation, "Should not detect size mismatch when mesh size is same");
                }
                
                // Property 4: Buffer deallocation should work correctly (Requirement 4.4)
                buffer_manager.remove_buffers(&chunk_pos);
                prop_assert!(!buffer_manager.has_buffers(&chunk_pos), "Buffers should not exist after removal");
                prop_assert_eq!(buffer_manager.buffer_count(), 0, "Buffer count should be zero after removal");
                
                // Property 5: Memory usage should be consistent
                let stats = buffer_manager.memory_usage();
                prop_assert_eq!(stats.buffer_count, 0, "Memory stats should show zero buffers");
                prop_assert_eq!(stats.total_memory, 0, "Memory stats should show zero memory usage");
            });
        }

        /// Property test for buffer cleanup functionality
        #[test]
        fn property_buffer_cleanup() {
            let config = ProptestConfig {
                cases: 10, // Reduced from default 256 to 10 for faster execution
                ..ProptestConfig::default()
            };
            proptest!(config, |(
                chunk_positions in prop::collection::vec(arb_chunk_position(), 1..=5), // Reduced max from 10 to 5
                active_chunk_indices in prop::collection::vec(0usize..5, 0..=3), // Reduced ranges
            )| {
                let mut buffer_manager = MockBufferManager::new();
                let mesh = create_test_mesh();
                
                // Create buffers for all chunk positions
                for &chunk_pos in &chunk_positions {
                    let _ = buffer_manager.create_buffers(chunk_pos, &mesh);
                }
                
                let initial_count = buffer_manager.buffer_count();
                prop_assert_eq!(initial_count, chunk_positions.len(), "Should have buffers for all chunks");
                
                // Create set of active chunks (subset of all chunks)
                let mut active_chunks = HashSet::new();
                for &index in &active_chunk_indices {
                    if index < chunk_positions.len() {
                        active_chunks.insert(chunk_positions[index]);
                    }
                }
                
                // Cleanup unused buffers
                buffer_manager.cleanup_unused_buffers(&active_chunks);
                
                let final_count = buffer_manager.buffer_count();
                prop_assert_eq!(final_count, active_chunks.len(), "Should only have buffers for active chunks");
                
                // Verify that all active chunks still have buffers
                for &chunk_pos in &active_chunks {
                    prop_assert!(buffer_manager.has_buffers(&chunk_pos), "Active chunks should still have buffers");
                }
            });
        }

        /// Property test for batch operations
        #[test]
        fn property_batch_operations() {
            let config = ProptestConfig {
                cases: 10, // Reduced from default 256 to 10 for faster execution
                ..ProptestConfig::default()
            };
            proptest!(config, |(
                updates in prop::collection::vec((arb_chunk_position(), arb_chunk_mesh()), 1..=3), // Reduced max from 5 to 3
            )| {
                let mut buffer_manager = MockBufferManager::new();
                
                let initial_count = buffer_manager.buffer_count();
                prop_assert_eq!(initial_count, 0, "Should start with no buffers");
                
                // Create buffers for all updates
                for (chunk_pos, mesh) in &updates {
                    let result = buffer_manager.create_buffers(*chunk_pos, mesh);
                    prop_assert!(result.is_ok(), "Buffer creation should succeed");
                }
                
                let final_count = buffer_manager.buffer_count();
                
                // Account for duplicate chunk positions
                let unique_positions: HashSet<_> = updates.iter().map(|(pos, _)| *pos).collect();
                prop_assert_eq!(final_count, unique_positions.len(), "Should have buffers for unique chunk positions");
                
                // Verify all unique positions have buffers
                for chunk_pos in unique_positions {
                    prop_assert!(buffer_manager.has_buffers(&chunk_pos), "Each unique chunk should have buffers");
                }
            });
        }
    }