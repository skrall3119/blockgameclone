//! Performance monitoring system for world operations

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Configuration for performance monitoring
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Maximum number of frame time samples to keep
    pub max_frame_samples: usize,
    /// Maximum number of memory samples to keep
    pub max_memory_samples: usize,
    /// Interval between memory sampling
    pub memory_sample_interval: Duration,
    /// Whether to enable detailed chunk statistics
    pub detailed_chunk_stats: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            max_frame_samples: 120, // 2 seconds at 60 FPS
            max_memory_samples: 60,  // 1 minute at 1 sample per second
            memory_sample_interval: Duration::from_secs(1),
            detailed_chunk_stats: true,
        }
    }
}

/// Memory usage sample at a specific point in time
#[derive(Debug, Clone)]
pub struct MemorySample {
    /// When this sample was taken
    pub timestamp: Instant,
    /// Number of chunks loaded
    pub chunk_count: usize,
    /// Total system memory used (estimated)
    pub total_memory: usize,
    /// GPU memory used (estimated)
    pub gpu_memory: usize,
}

impl MemorySample {
    /// Create a new memory sample
    pub fn new(chunk_count: usize, total_memory: usize, gpu_memory: usize) -> Self {
        Self {
            timestamp: Instant::now(),
            chunk_count,
            total_memory,
            gpu_memory,
        }
    }
}

/// Statistics about chunk operations
#[derive(Debug, Clone)]
pub struct ChunkStats {
    /// Number of chunks currently loaded
    pub chunks_loaded: usize,
    /// Number of chunks generated this session
    pub chunks_generated: usize,
    /// Number of chunks meshed this session
    pub chunks_meshed: usize,
    /// Average time to generate a chunk
    pub generation_time_avg: Duration,
    /// Average time to mesh a chunk
    pub meshing_time_avg: Duration,
    /// Total time spent on chunk operations
    pub total_chunk_time: Duration,
}

impl Default for ChunkStats {
    fn default() -> Self {
        Self {
            chunks_loaded: 0,
            chunks_generated: 0,
            chunks_meshed: 0,
            generation_time_avg: Duration::ZERO,
            meshing_time_avg: Duration::ZERO,
            total_chunk_time: Duration::ZERO,
        }
    }
}

impl ChunkStats {
    /// Update generation time statistics
    pub fn record_generation_time(&mut self, duration: Duration) {
        self.chunks_generated += 1;
        self.total_chunk_time += duration;
        
        // Calculate running average
        let total_samples = self.chunks_generated as f64;
        let current_avg = self.generation_time_avg.as_secs_f64();
        let new_sample = duration.as_secs_f64();
        let new_avg = (current_avg * (total_samples - 1.0) + new_sample) / total_samples;
        
        self.generation_time_avg = Duration::from_secs_f64(new_avg);
    }

    /// Update meshing time statistics
    pub fn record_meshing_time(&mut self, duration: Duration) {
        self.chunks_meshed += 1;
        self.total_chunk_time += duration;
        
        // Calculate running average
        let total_samples = self.chunks_meshed as f64;
        let current_avg = self.meshing_time_avg.as_secs_f64();
        let new_sample = duration.as_secs_f64();
        let new_avg = (current_avg * (total_samples - 1.0) + new_sample) / total_samples;
        
        self.meshing_time_avg = Duration::from_secs_f64(new_avg);
    }
}

/// Performance monitoring system for world operations
#[derive(Debug)]
pub struct PerformanceMonitor {
    /// Configuration for monitoring behavior
    config: MonitorConfig,
    /// Frame time samples for FPS calculation
    frame_times: VecDeque<Duration>,
    /// Memory usage samples over time
    memory_samples: VecDeque<MemorySample>,
    /// Statistics about chunk operations
    chunk_stats: ChunkStats,
    /// Last time memory was sampled
    last_memory_sample: Instant,
    /// Start time for session statistics
    session_start: Instant,
}

impl PerformanceMonitor {
    /// Create a new performance monitor with the given configuration
    pub fn new(config: MonitorConfig) -> Self {
        let now = Instant::now();
        Self {
            frame_times: VecDeque::with_capacity(config.max_frame_samples),
            memory_samples: VecDeque::with_capacity(config.max_memory_samples),
            config,
            chunk_stats: ChunkStats::default(),
            last_memory_sample: now,
            session_start: now,
        }
    }

    /// Record a frame time for FPS calculation
    pub fn record_frame_time(&mut self, frame_time: Duration) {
        self.frame_times.push_back(frame_time);
        
        // Keep only the most recent samples
        while self.frame_times.len() > self.config.max_frame_samples {
            self.frame_times.pop_front();
        }
    }

    /// Record a memory sample
    pub fn record_memory_sample(&mut self, sample: MemorySample) {
        self.memory_samples.push_back(sample);
        self.last_memory_sample = Instant::now();
        
        // Keep only the most recent samples
        while self.memory_samples.len() > self.config.max_memory_samples {
            self.memory_samples.pop_front();
        }
    }

    /// Update chunk statistics
    pub fn update_chunk_stats(&mut self, chunks_loaded: usize) {
        self.chunk_stats.chunks_loaded = chunks_loaded;
    }

    /// Record chunk generation time
    pub fn record_chunk_generation(&mut self, duration: Duration) {
        self.chunk_stats.record_generation_time(duration);
    }

    /// Record chunk meshing time
    pub fn record_chunk_meshing(&mut self, duration: Duration) {
        self.chunk_stats.record_meshing_time(duration);
    }

    /// Calculate current FPS based on recent frame times
    pub fn current_fps(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        
        let total_time: Duration = self.frame_times.iter().sum();
        let avg_frame_time = total_time.as_secs_f32() / self.frame_times.len() as f32;
        
        if avg_frame_time > 0.0 {
            1.0 / avg_frame_time
        } else {
            0.0
        }
    }

    /// Get the most recent memory sample
    pub fn current_memory_usage(&self) -> Option<&MemorySample> {
        self.memory_samples.back()
    }

    /// Get current chunk statistics
    pub fn chunk_statistics(&self) -> &ChunkStats {
        &self.chunk_stats
    }

    /// Get session duration
    pub fn session_duration(&self) -> Duration {
        Instant::now() - self.session_start
    }

    /// Check if it's time to take a memory sample
    pub fn should_sample_memory(&self) -> bool {
        Instant::now() - self.last_memory_sample >= self.config.memory_sample_interval
    }

    /// Get average frame time over the current sample window
    pub fn average_frame_time(&self) -> Duration {
        if self.frame_times.is_empty() {
            return Duration::ZERO;
        }
        
        let total: Duration = self.frame_times.iter().sum();
        total / self.frame_times.len() as u32
    }

    /// Get memory usage trend (positive = increasing, negative = decreasing)
    pub fn memory_trend(&self) -> f32 {
        if self.memory_samples.len() < 2 {
            return 0.0;
        }
        
        let recent = &self.memory_samples[self.memory_samples.len() - 1];
        let older = &self.memory_samples[self.memory_samples.len() - 2];
        
        let recent_total = recent.total_memory as f32;
        let older_total = older.total_memory as f32;
        
        if older_total > 0.0 {
            (recent_total - older_total) / older_total
        } else {
            0.0
        }
    }

    /// Reset all statistics (useful for benchmarking)
    pub fn reset(&mut self) {
        self.frame_times.clear();
        self.memory_samples.clear();
        self.chunk_stats = ChunkStats::default();
        self.session_start = Instant::now();
        self.last_memory_sample = Instant::now();
    }
}