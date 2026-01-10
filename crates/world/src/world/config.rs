//! World configuration types and validation

use super::{ChunkCoord, WorldError, WorldResult};
use std::time::Duration;

/// Version of the configuration format for migration support
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigVersion {
    /// Version 1.0 - Initial configuration format
    V1_0,
    /// Version 1.1 - Added performance monitoring toggle
    V1_1,
    /// Version 1.2 - Added chunk unload delay configuration
    V1_2,
}

impl Default for ConfigVersion {
    fn default() -> Self {
        ConfigVersion::V1_2 // Latest version
    }
}

/// Configuration for world behavior and parameters
#[derive(Debug, Clone)]
pub struct WorldConfig {
    /// Configuration format version for migration support
    pub version: ConfigVersion,
    /// Maximum distance from player where chunks are loaded and rendered
    pub render_distance: u32,
    /// Pattern for initial chunk loading
    pub initial_load_pattern: LoadPattern,
    /// Maximum number of chunks to keep loaded (None = unlimited)
    pub max_chunks_loaded: Option<usize>,
    /// Delay before unloading unused chunks
    pub chunk_unload_delay: Duration,
    /// Whether to enable performance monitoring
    pub performance_monitoring: bool,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            version: ConfigVersion::default(),
            render_distance: super::DEFAULT_RENDER_DISTANCE,
            initial_load_pattern: LoadPattern::Single(ChunkCoord::new(0, 0, 0)),
            max_chunks_loaded: Some(1000),
            chunk_unload_delay: Duration::from_secs(super::CHUNK_UNLOAD_DELAY_SECONDS),
            performance_monitoring: true,
        }
    }
}

impl WorldConfig {
    /// Create a new world configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration with a specific version (for testing migration)
    pub fn new_with_version(version: ConfigVersion) -> Self {
        let mut config = Self::default();
        config.version = version;
        config
    }

    /// Set the render distance
    pub fn with_render_distance(mut self, distance: u32) -> Self {
        self.render_distance = distance;
        self
    }

    /// Set the initial loading pattern
    pub fn with_initial_load_pattern(mut self, pattern: LoadPattern) -> Self {
        self.initial_load_pattern = pattern;
        self
    }

    /// Set the maximum number of loaded chunks
    pub fn with_max_chunks(mut self, max_chunks: Option<usize>) -> Self {
        self.max_chunks_loaded = max_chunks;
        self
    }

    /// Set the chunk unload delay
    pub fn with_unload_delay(mut self, delay: Duration) -> Self {
        self.chunk_unload_delay = delay;
        self
    }

    /// Enable or disable performance monitoring
    pub fn with_performance_monitoring(mut self, enabled: bool) -> Self {
        self.performance_monitoring = enabled;
        self
    }

    /// Set the configuration version
    pub fn with_version(mut self, version: ConfigVersion) -> Self {
        self.version = version;
        self
    }

    /// Validate the configuration parameters
    pub fn validate(&self) -> WorldResult<()> {
        if self.render_distance == 0 {
            return Err(WorldError::InvalidConfiguration {
                parameter: "render_distance".to_string(),
                value: self.render_distance.to_string(),
                reason: "Render distance must be greater than 0".to_string(),
            });
        }

        if self.render_distance > 64 {
            return Err(WorldError::InvalidConfiguration {
                parameter: "render_distance".to_string(),
                value: self.render_distance.to_string(),
                reason: "Render distance should not exceed 64 for performance reasons".to_string(),
            });
        }

        if let Some(max_chunks) = self.max_chunks_loaded {
            if max_chunks == 0 {
                return Err(WorldError::InvalidConfiguration {
                    parameter: "max_chunks_loaded".to_string(),
                    value: max_chunks.to_string(),
                    reason: "Maximum chunks must be greater than 0 if specified".to_string(),
                });
            }
        }

        if self.chunk_unload_delay.as_secs() > 3600 {
            return Err(WorldError::InvalidConfiguration {
                parameter: "chunk_unload_delay".to_string(),
                value: format!("{:?}", self.chunk_unload_delay),
                reason: "Chunk unload delay should not exceed 1 hour".to_string(),
            });
        }

        Ok(())
    }

    /// Check if this configuration is compatible with another configuration
    /// This is useful for determining if a configuration change requires world restart
    pub fn is_compatible_with(&self, other: &WorldConfig) -> bool {
        // Configurations are compatible if they don't require fundamental changes
        // to the world structure or chunk management
        
        // Version compatibility - newer versions should be backward compatible
        if !self.is_version_compatible_with(other.version) {
            return false;
        }

        // Render distance changes are always compatible (can be adapted at runtime)
        // Max chunks changes are compatible if not decreasing below current usage
        // Performance monitoring changes are always compatible
        // Chunk unload delay changes are always compatible
        
        true // Most configuration changes are compatible
    }

    /// Check if this configuration version is compatible with another version
    pub fn is_version_compatible_with(&self, other_version: ConfigVersion) -> bool {
        use ConfigVersion::*;
        
        match (self.version, other_version) {
            // Same versions are always compatible
            (v1, v2) if v1 == v2 => true,
            
            // V1_2 can handle all previous versions
            (V1_2, V1_1) | (V1_2, V1_0) => true,
            
            // V1_1 can handle V1_0
            (V1_1, V1_0) => true,
            
            // Older versions cannot handle newer versions
            _ => false,
        }
    }

    /// Migrate this configuration to the latest version
    pub fn migrate_to_latest(&mut self) -> WorldResult<bool> {
        let original_version = self.version;
        
        match self.version {
            ConfigVersion::V1_0 => {
                // Migrate from V1_0 to V1_1: add performance monitoring (default enabled)
                self.performance_monitoring = true;
                self.version = ConfigVersion::V1_1;
                
                // Continue to next migration
                self.migrate_v1_1_to_v1_2()?;
            }
            ConfigVersion::V1_1 => {
                // Migrate from V1_1 to V1_2: add chunk unload delay
                self.migrate_v1_1_to_v1_2()?;
            }
            ConfigVersion::V1_2 => {
                // Already at latest version
                return Ok(false);
            }
        }
        
        // Validate the migrated configuration
        self.validate()?;
        
        Ok(original_version != self.version)
    }

    /// Migrate from V1_1 to V1_2
    fn migrate_v1_1_to_v1_2(&mut self) -> WorldResult<()> {
        // Add chunk unload delay with default value
        self.chunk_unload_delay = Duration::from_secs(super::CHUNK_UNLOAD_DELAY_SECONDS);
        self.version = ConfigVersion::V1_2;
        Ok(())
    }

    /// Create a configuration from a legacy format (for backward compatibility)
    pub fn from_legacy_v1_0(
        render_distance: u32,
        initial_load_pattern: LoadPattern,
        max_chunks_loaded: Option<usize>,
    ) -> WorldResult<Self> {
        let mut config = Self {
            version: ConfigVersion::V1_0,
            render_distance,
            initial_load_pattern,
            max_chunks_loaded,
            chunk_unload_delay: Duration::from_secs(30), // Default for V1_0
            performance_monitoring: false, // Default for V1_0
        };
        
        // Migrate to latest version
        config.migrate_to_latest()?;
        
        Ok(config)
    }

    /// Create a configuration from V1_1 format
    pub fn from_v1_1(
        render_distance: u32,
        initial_load_pattern: LoadPattern,
        max_chunks_loaded: Option<usize>,
        performance_monitoring: bool,
    ) -> WorldResult<Self> {
        let mut config = Self {
            version: ConfigVersion::V1_1,
            render_distance,
            initial_load_pattern,
            max_chunks_loaded,
            chunk_unload_delay: Duration::from_secs(30), // Default for V1_1
            performance_monitoring,
        };
        
        // Migrate to latest version
        config.migrate_to_latest()?;
        
        Ok(config)
    }

    /// Get the configuration differences between this and another configuration
    /// Returns a list of parameter names that differ
    pub fn get_differences(&self, other: &WorldConfig) -> Vec<String> {
        let mut differences = Vec::new();
        
        if self.version != other.version {
            differences.push("version".to_string());
        }
        
        if self.render_distance != other.render_distance {
            differences.push("render_distance".to_string());
        }
        
        if self.initial_load_pattern != other.initial_load_pattern {
            differences.push("initial_load_pattern".to_string());
        }
        
        if self.max_chunks_loaded != other.max_chunks_loaded {
            differences.push("max_chunks_loaded".to_string());
        }
        
        if self.chunk_unload_delay != other.chunk_unload_delay {
            differences.push("chunk_unload_delay".to_string());
        }
        
        if self.performance_monitoring != other.performance_monitoring {
            differences.push("performance_monitoring".to_string());
        }
        
        differences
    }

    /// Check if a configuration change requires a world restart
    pub fn requires_restart(&self, new_config: &WorldConfig) -> bool {
        // Most configuration changes can be applied at runtime
        // Only fundamental changes would require a restart
        
        // Version changes that aren't backward compatible require restart
        if !new_config.is_version_compatible_with(self.version) {
            return true;
        }
        
        // Initial load pattern changes don't require restart (only affects new worlds)
        // Render distance changes can be adapted at runtime
        // Max chunks changes can be adapted at runtime
        // Performance monitoring can be toggled at runtime
        // Chunk unload delay can be changed at runtime
        
        false // Currently no changes require restart
    }
}

/// Patterns for loading chunks in the world
#[derive(Debug, Clone, PartialEq)]
pub enum LoadPattern {
    /// Load a single chunk at the specified coordinates
    Single(ChunkCoord),
    /// Load chunks in a grid pattern around a center point
    Grid {
        /// Center of the grid
        center: ChunkCoord,
        /// Radius of the grid (in chunks)
        radius: u32,
    },
    /// Load chunks at custom specified coordinates
    Custom(Vec<ChunkCoord>),
}

impl LoadPattern {
    /// Get all chunk coordinates that should be loaded for this pattern
    pub fn get_coordinates(&self) -> Vec<ChunkCoord> {
        match self {
            LoadPattern::Single(coord) => vec![*coord],
            LoadPattern::Grid { center, radius } => {
                let mut coords = Vec::new();
                let r = *radius as i32;
                
                for x in -r..=r {
                    for y in -r..=r {
                        for z in -r..=r {
                            coords.push(ChunkCoord::new(
                                center.x + x,
                                center.y + y,
                                center.z + z,
                            ));
                        }
                    }
                }
                
                coords
            }
            LoadPattern::Custom(coords) => coords.clone(),
        }
    }

    /// Get the estimated number of chunks this pattern will load
    pub fn estimated_chunk_count(&self) -> usize {
        match self {
            LoadPattern::Single(_) => 1,
            LoadPattern::Grid { radius, .. } => {
                let diameter = (radius * 2 + 1) as usize;
                diameter * diameter * diameter
            }
            LoadPattern::Custom(coords) => coords.len(),
        }
    }
}