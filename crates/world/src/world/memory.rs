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

    /// Check for potential memory leaks by detecting chunks that haven't been accessed recently
    pub fn detect_potential_leaks(&self, max_idle_duration: std::time::Duration) -> Vec<ChunkCoord> {
        let now = Instant::now();
        self.chunk_access_times
            .iter()
            .filter_map(|(coord, last_access)| {
                if now.duration_since(*last_access) > max_idle_duration {
                    Some(*coord)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Validate memory bounds and detect inconsistencies
    pub fn validate_memory_bounds(&self) -> Result<(), String> {
        let current = self.current_usage();
        
        // Check if current usage exceeds budget
        if current > self.memory_budget {
            return Err(format!(
                "Memory usage {} exceeds budget {}",
                current, self.memory_budget
            ));
        }
        
        // Check if tracked memory matches atomic counter
        let tracked_total: usize = self.chunk_memory.values().sum();
        if tracked_total != current {
            return Err(format!(
                "Memory tracking inconsistency: tracked {} != atomic {}",
                tracked_total, current
            ));
        }
        
        // Check for chunks with zero memory (potential leak)
        let zero_memory_chunks: Vec<_> = self.chunk_memory
            .iter()
            .filter(|(_, &size)| size == 0)
            .map(|(coord, _)| *coord)
            .collect();
        
        if !zero_memory_chunks.is_empty() {
            return Err(format!(
                "Found {} chunks with zero memory allocation: {:?}",
                zero_memory_chunks.len(), zero_memory_chunks
            ));
        }
        
        // Check for orphaned access times (chunks in access_times but not in chunk_memory)
        let orphaned_access_times: Vec<_> = self.chunk_access_times
            .keys()
            .filter(|coord| !self.chunk_memory.contains_key(coord))
            .copied()
            .collect();
        
        if !orphaned_access_times.is_empty() {
            return Err(format!(
                "Found {} orphaned access time entries: {:?}",
                orphaned_access_times.len(), orphaned_access_times
            ));
        }
        
        // Check for orphaned memory entries (chunks in chunk_memory but not in access_times)
        let orphaned_memory_entries: Vec<_> = self.chunk_memory
            .keys()
            .filter(|coord| !self.chunk_access_times.contains_key(coord))
            .copied()
            .collect();
        
        if !orphaned_memory_entries.is_empty() {
            return Err(format!(
                "Found {} orphaned memory entries: {:?}",
                orphaned_memory_entries.len(), orphaned_memory_entries
            ));
        }
        
        Ok(())
    }

    /// Perform automatic leak detection and cleanup
    pub fn cleanup_potential_leaks(&mut self, max_idle_duration: std::time::Duration) -> Vec<ChunkCoord> {
        let potential_leaks = self.detect_potential_leaks(max_idle_duration);
        let mut cleaned_up = Vec::new();
        
        for coord in potential_leaks {
            if self.unregister_chunk_memory(coord).is_some() {
                cleaned_up.push(coord);
            }
        }
        
        cleaned_up
    }

    /// Set strict memory bounds checking
    pub fn set_strict_bounds_checking(&mut self, enabled: bool) {
        // This could be used to enable/disable additional bounds checking
        // For now, we always perform bounds checking, but this could be extended
        // to add more expensive validation in debug builds
    }

    /// Get memory usage per chunk (for debugging and validation)
    pub fn chunk_memory(&self) -> &HashMap<ChunkCoord, usize> {
        &self.chunk_memory
    }

    /// Get chunk access times (for debugging and validation)
    pub fn chunk_access_times(&self) -> &HashMap<ChunkCoord, Instant> {
        &self.chunk_access_times
    }

    /// Get memory fragmentation statistics
    pub fn fragmentation_stats(&self) -> FragmentationStats {
        if self.chunk_memory.is_empty() {
            return FragmentationStats {
                total_chunks: 0,
                min_chunk_size: 0,
                max_chunk_size: 0,
                average_chunk_size: 0,
                size_variance: 0.0,
            };
        }
        
        let sizes: Vec<usize> = self.chunk_memory.values().copied().collect();
        let min_size = *sizes.iter().min().unwrap();
        let max_size = *sizes.iter().max().unwrap();
        let total_size: usize = sizes.iter().sum();
        let average_size = total_size / sizes.len();
        
        // Calculate variance
        let variance = if sizes.len() > 1 {
            let sum_squared_diff: f64 = sizes
                .iter()
                .map(|&size| {
                    let diff = size as f64 - average_size as f64;
                    diff * diff
                })
                .sum();
            sum_squared_diff / sizes.len() as f64
        } else {
            0.0
        };
        
        FragmentationStats {
            total_chunks: sizes.len(),
            min_chunk_size: min_size,
            max_chunk_size: max_size,
            average_chunk_size: average_size,
            size_variance: variance,
        }
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

/// Statistics about memory fragmentation
#[derive(Debug, Clone)]
pub struct FragmentationStats {
    /// Total number of chunks
    pub total_chunks: usize,
    /// Minimum chunk size in bytes
    pub min_chunk_size: usize,
    /// Maximum chunk size in bytes
    pub max_chunk_size: usize,
    /// Average chunk size in bytes
    pub average_chunk_size: usize,
    /// Variance in chunk sizes
    pub size_variance: f64,
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

impl FragmentationStats {
    /// Check if memory is highly fragmented
    pub fn is_fragmented(&self) -> bool {
        if self.total_chunks <= 1 {
            return false;
        }
        
        // Consider fragmented if variance is high relative to average
        let coefficient_of_variation = (self.size_variance.sqrt() / self.average_chunk_size as f64).abs();
        coefficient_of_variation > 0.5 // 50% coefficient of variation threshold
    }

    /// Get the size range (max - min)
    pub fn size_range(&self) -> usize {
        self.max_chunk_size.saturating_sub(self.min_chunk_size)
    }

    /// Get fragmentation score (0.0 = no fragmentation, 1.0 = high fragmentation)
    pub fn fragmentation_score(&self) -> f32 {
        if self.total_chunks <= 1 || self.average_chunk_size == 0 {
            return 0.0;
        }
        
        let coefficient_of_variation = (self.size_variance.sqrt() / self.average_chunk_size as f64).abs();
        (coefficient_of_variation as f32).min(1.0)
    }
}

/// Memory health status levels
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryHealthStatus {
    /// Memory is operating normally
    Healthy,
    /// Memory has warnings but is still functional
    Warning,
    /// Memory is in critical state and needs immediate attention
    Critical,
}

/// Comprehensive memory health report
#[derive(Debug, Clone)]
pub struct MemoryHealthReport {
    /// Overall health status
    pub status: MemoryHealthStatus,
    /// Current memory statistics
    pub memory_stats: MemoryStats,
    /// Memory fragmentation statistics
    pub fragmentation_stats: FragmentationStats,
    /// Number of potential memory leaks detected
    pub potential_leaks: usize,
    /// Critical issues that need immediate attention
    pub issues: Vec<String>,
    /// Warnings that should be monitored
    pub warnings: Vec<String>,
}

impl MemoryHealthReport {
    /// Check if the memory system is healthy
    pub fn is_healthy(&self) -> bool {
        self.status == MemoryHealthStatus::Healthy
    }

    /// Check if there are any critical issues
    pub fn has_critical_issues(&self) -> bool {
        self.status == MemoryHealthStatus::Critical
    }

    /// Get a summary of all issues and warnings
    pub fn summary(&self) -> String {
        let mut summary = format!("Memory Health: {:?}\n", self.status);
        
        if !self.issues.is_empty() {
            summary.push_str("Critical Issues:\n");
            for issue in &self.issues {
                summary.push_str(&format!("  - {}\n", issue));
            }
        }
        
        if !self.warnings.is_empty() {
            summary.push_str("Warnings:\n");
            for warning in &self.warnings {
                summary.push_str(&format!("  - {}\n", warning));
            }
        }
        
        summary.push_str(&format!(
            "Memory Usage: {:.1}% ({} / {} bytes)\n",
            self.memory_stats.usage_fraction * 100.0,
            self.memory_stats.current_usage,
            self.memory_stats.total_budget
        ));
        
        if self.fragmentation_stats.total_chunks > 0 {
            summary.push_str(&format!(
                "Fragmentation: {:.1}% ({} chunks)\n",
                self.fragmentation_stats.fragmentation_score() * 100.0,
                self.fragmentation_stats.total_chunks
            ));
        }
        
        if self.potential_leaks > 0 {
            summary.push_str(&format!("Potential Leaks: {}\n", self.potential_leaks));
        }
        
        summary
    }
}