//! Graphics context management

use crate::renderer::{GraphicsError, GraphicsResult};
use winit::window::Window;
use wgpu::{
    Instance, Surface, Adapter, Device, Queue, SurfaceConfiguration,
    Backends, PowerPreference, RequestAdapterOptions, DeviceDescriptor,
    Features, Limits, TextureUsages,
};

/// Graphics context that manages wgpu resources
pub struct GraphicsContext<'window> {
    pub instance: Instance,
    pub surface: Surface<'window>,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
}

impl<'window> GraphicsContext<'window> {
    /// Create a new graphics context for the given window
    pub async fn new(window: &'window Window) -> GraphicsResult<Self> {
        // Initialize wgpu instance with Vulkan backend preference
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends: Backends::VULKAN | Backends::DX12 | Backends::METAL,
            flags: wgpu::InstanceFlags::default(),
            ..Default::default()
        });

        // Create surface from window
        let surface = instance
            .create_surface(window)
            .map_err(GraphicsError::SurfaceCreation)?;

        // Request adapter with Vulkan preference
        let adapter_options = RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        };

        let adapter = match instance.request_adapter(&adapter_options).await {
            Ok(adapter) => adapter,
            Err(_) => return Err(GraphicsError::NoAdapter),
        };

        // Get required features and limits
        let required_features = Features::empty();
        let required_limits = Limits::default();

        // Check if adapter supports required features
        let adapter_features = adapter.features();
        let missing_features: Vec<String> = required_features
            .difference(adapter_features)
            .iter()
            .map(|f| format!("{:?}", f))
            .collect();

        if !missing_features.is_empty() {
            return Err(GraphicsError::MissingFeatures { missing_features });
        }

        // Create device and queue
        let device_descriptor = DeviceDescriptor {
            required_features,
            required_limits,
            ..Default::default()
        };

        let (device, queue): (Device, Queue) = adapter
            .request_device(&device_descriptor)
            .await
            .map_err(GraphicsError::DeviceCreation)?;

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let size = window.inner_size();
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        Ok(GraphicsContext {
            instance,
            surface,
            adapter,
            device,
            queue,
            config,
        })
    }

    /// Resize the surface configuration
    pub fn resize(&mut self, new_width: u32, new_height: u32) -> GraphicsResult<()> {
        if new_width == 0 || new_height == 0 {
            return Err(GraphicsError::InvalidDimensions {
                width: new_width,
                height: new_height,
            });
        }

        self.config.width = new_width;
        self.config.height = new_height;
        
        // Handle potential surface configuration errors
        self.surface.configure(&self.device, &self.config);
        
        Ok(())
    }

    /// Get the current surface texture for rendering
    pub fn get_current_texture(&self) -> GraphicsResult<wgpu::SurfaceTexture> {
        self.surface.get_current_texture().map_err(|err| match err {
            wgpu::SurfaceError::Lost => GraphicsError::SurfaceLost,
            wgpu::SurfaceError::OutOfMemory => GraphicsError::SurfaceConfiguration {
                reason: "Out of GPU memory".to_string(),
            },
            wgpu::SurfaceError::Timeout => GraphicsError::SurfaceConfiguration {
                reason: "Surface texture acquisition timed out".to_string(),
            },
            wgpu::SurfaceError::Outdated => GraphicsError::SurfaceConfiguration {
                reason: "Surface configuration is outdated".to_string(),
            },
            wgpu::SurfaceError::Other => GraphicsError::SurfaceConfiguration {
                reason: "Unknown surface error occurred".to_string(),
            },
        })
    }

    /// Get the surface format
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Get the current surface size
    pub fn surface_size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Recreate the surface configuration (useful for surface recovery)
    pub fn reconfigure_surface(&mut self) -> GraphicsResult<()> {
        // Validate current dimensions
        if self.config.width == 0 || self.config.height == 0 {
            return Err(GraphicsError::InvalidDimensions {
                width: self.config.width,
                height: self.config.height,
            });
        }

        // Reconfigure with current settings
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }

    /// Check if the adapter supports the required features and limits
    pub fn validate_adapter_capabilities(&self, required_features: Features, required_limits: &Limits) -> GraphicsResult<()> {
        let adapter_features = self.adapter.features();
        let missing_features: Vec<String> = required_features
            .difference(adapter_features)
            .iter()
            .map(|f| format!("{:?}", f))
            .collect();

        if !missing_features.is_empty() {
            return Err(GraphicsError::MissingFeatures { missing_features });
        }

        let adapter_limits = self.adapter.limits();
        
        // Check some key limits (add more as needed)
        if adapter_limits.max_texture_dimension_2d < required_limits.max_texture_dimension_2d {
            return Err(GraphicsError::InsufficientLimits {
                required: format!("max_texture_dimension_2d: {}", required_limits.max_texture_dimension_2d),
                available: format!("max_texture_dimension_2d: {}", adapter_limits.max_texture_dimension_2d),
            });
        }

        Ok(())
    }
}