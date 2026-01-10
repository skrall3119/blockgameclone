//! Memory management system for world operations

use super::{ChunkCoord, WorldError, WorldResult};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

/// Memory management system for tracking and controlling world resource usage
#[derive(Debug)]
pub struct MemoryManager {
    /// Maximum memory budget in bytes
    memory_budget: usize,
    /// Current memory usage (atomic for thread safety)
    current_usage: AtomicUsize,
    /// Threshold for triggering cleanup (as fraction of budget)
    cleanup_threshold: f32,
    /// Memory usage per chunk for tracking
    chunk_memory: HashMap<ChunkCoord, usize>,
    /// Last access time per chunk for LRU cleanup
    chunk_access_times: HashMap<ChunkCoord, Instant>,
}

impl MemoryManager {
    /// Create a new memory manager with the given budget in bytes
    pub fn new(memory_budget: usize) -> Self {
        Self {
            memory_budget,
            current_usage: AtomicUsize::new(0),
            cleanup_threshold: 0.8, // Trigger cleanup at 80% of budget
            chunk_memory: HashMap::new(),
            chunk_access_times: HashMap::new(),
        }
    }

    /// Check if a chunk can be loaded without exceeding the memory budget
    pub fn can_load_chunk(&self, estimated_size: usize) -> bool {
        let current = self.current_usage.load(Ordering::Relaxed);
        current + estimated_size <= self.memory_budget
    }

    /// Register memory usage for a chunk
    pub fn register_chunk_memory(&mut self, coord: ChunkCoord, size: usize) -> WorldResult<()> {
        let current = self.current_usage.load(Ordering::Relaxed);
        
        if current + size > self.memory_budget {
            return Err(WorldError::OutOfMemory {
                requested: size,
                available: self.memory_budget.saturating_sub(current),
            });
        }
        
        self.current_usage.fetch_add(size, Ordering::Relaxed);
        self.chunk_memory.insert(coord, size);
        self.chunk_access_times.insert(coord, Instant::now());
        
        Ok(())
    }

    /// Unregister memory usage for a chunk
    pub fn unregister_chunk_memory(&mut self, coord: ChunkCoord) -> Option<usize> {
        if let Some(size) = self.chunk_memory.remove(&coord) {
            self.current_usage.fetch_sub(size, Ordering::Relaxed);
            self.chunk_access_times.remove(&coord);
            Some(size)
        } else {
            None
        }
    }

    /// Update the last access time for a chunk
    pub fn touch_chunk(&mut self, coord: ChunkCoord) {
        self.chunk_access_times.insert(coord, Instant::now());
    }

    /// Get current memory usage in bytes
    pub fn current_usage(&self) -> usize {
        self.current_usage.load(Ordering::Relaxed)
    }

    /// Get memory budget in bytes
    pub fn memory_budget(&self) -> usize {
        self.memory_budget
    }

    /// Get memory usage as a fraction of the budget (0.0 to 1.0+)
    pub fn usage_fraction(&self) -> f32 {
        if self.memory_budget == 0 {
            return 0.0;
        }
        
        self.current_usage() as f32 / self.memory_budget as f32
    }

    /// Check if cleanup should be triggered
    pub fn should_cleanup(&self) -> bool {
        self.usage_fraction() >= self.cleanup_threshold
    }

    /// Suggest chunks for cleanup based on LRU policy
    pub fn suggest_cleanup(&self, target_free_bytes: usize) -> Vec<ChunkCoord> {
        let mut candidates: Vec<_> = self.chunk_access_times
            .iter()
            .map(|(coord, time)| (*coord, *time))
            .collect();
        
        // Sort by access time (oldest first)
        candidates.sort_by_key(|(_, time)| *time);
        
        let mut cleanup_list = Vec::new();
        let mut freed_bytes = 0;
        
        for (coord, _) in candidates {
            if let Some(&chunk_size) = self.chunk_memory.get(&coord) {
                cleanup_list.push(coord);
                freed_bytes += chunk_size;
                
                if freed_bytes >= target_free_bytes {
                    break;
                }
            }
        }
        
        cleanup_list
    }

    /// Get memory statistics
    pub fn memory_stats(&self) -> MemoryStats {
        MemoryStats {
            total_budget: self.memory_budget,
            current_usage: self.current_usage(),
            usage_fraction: self.usage_fraction(),
            chunks_tracked: self.chunk_memory.len(),
            average_chunk_size: if self.chunk_memory.is_empty() {
                0
            } else {
                self.chunk_memory.values().sum::<usize>() / self.chunk_memory.len()
            },
            cleanup_threshold: self.cleanup_threshold,
        }
    }

    /// Set the cleanup threshold (0.0 to 1.0)
    pub fn set_cleanup_threshold(&mut self, threshold: f32) -> WorldResult<()> {
        if threshold < 0.0 || threshold > 1.0 {
            return Err(WorldError::InvalidConfiguration {
                parameter: "cleanup_threshold".to_string(),
                value: threshold.to_string(),
                reason: "Cleanup threshold must be between 0.0 and 1.0".to_string(),
            });
        }
        
        self.cleanup_threshold = threshold;
        Ok(())
    }

    /// Update the memory budget
    pub fn set_memory_budget(&mut self, new_budget: usize) -> WorldResult<()> {
        if new_budget == 0 {
            return Err(WorldError::InvalidConfiguration {
                parameter: "memory_budget".to_string(),
                value: new_budget.to_string(),
                reason: "Memory budget must be greater than 0".to_string(),
            });
        }
        
        self.memory_budget = new_budget;
        Ok(())
    }

    /// Force cleanup of specific chunks
    pub fn force_cleanup(&mut self, coords: &[ChunkCoord]) -> usize {
        let mut freed_bytes = 0;
        
        for &coord in coords {
            if let Some(size) = self.unregister_chunk_memory(coord) {
                freed_bytes += size;
            }
        }
        
        freed_bytes
    }

    /// Get the oldest accessed chunk
    pub fn oldest_chunk(&self) -> Option<(ChunkCoord, Instant)> {
        self.chunk_access_times
            .iter()
            .min_by_key(|(_, time)| *time)
            .map(|(coord, time)| (*coord, *time))
    }

    /// Get the newest accessed chunk
    pub fn newest_chunk(&self) -> Option<(ChunkCoord, Instant)> {
        self.chunk_access_times
            .iter()
            .max_by_key(|(_, time)| *time)
            .map(|(coord, time)| (*coord, *time))
    }
}

/// Statistics about memory usage
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total memory budget in bytes
    pub total_budget: usize,
    /// Current memory usage in bytes
    pub current_usage: usize,
    /// Usage as a fraction of budget (0.0 to 1.0+)
    pub usage_fraction: f32,
    /// Number of chunks being tracked
    pub chunks_tracked: usize,
    /// Average memory per chunk in bytes
    pub average_chunk_size: usize,
    /// Cleanup threshold (0.0 to 1.0)
    pub cleanup_threshold: f32,
}

impl MemoryStats {
    /// Get available memory in bytes
    pub fn available_memory(&self) -> usize {
        self.total_budget.saturating_sub(self.current_usage)
    }

    /// Check if memory usage is critical (above cleanup threshold)
    pub fn is_critical(&self) -> bool {
        self.usage_fraction >= self.cleanup_threshold
    }

    /// Get memory pressure level (0.0 = no pressure, 1.0+ = over budget)
    pub fn pressure_level(&self) -> f32 {
        self.usage_fraction / self.cleanup_threshold
    }
}