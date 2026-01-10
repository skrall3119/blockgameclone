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
        // Initialize wgpu instance with broader backend support
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends: Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            ..Default::default()
        });

        // Create surface from window
        let surface = instance
            .create_surface(window)
            .map_err(GraphicsError::SurfaceCreation)?;

        // Try high performance first, then fallback to low power
        let adapter_options_high_perf = RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        };

        let adapter_options_low_power = RequestAdapterOptions {
            power_preference: PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        };

        let adapter_options_fallback = RequestAdapterOptions {
            power_preference: PowerPreference::None,
            compatible_surface: Some(&surface),
            force_fallback_adapter: true,
        };

        let adapter = match instance.request_adapter(&adapter_options_high_perf).await {
            Ok(adapter) => {
                log::info!("Selected high performance graphics adapter");
                adapter
            }
            Err(_) => match instance.request_adapter(&adapter_options_low_power).await {
                Ok(adapter) => {
                    log::info!("Selected low power graphics adapter");
                    adapter
                }
                Err(_) => match instance.request_adapter(&adapter_options_fallback).await {
                    Ok(adapter) => {
                        log::info!("Selected fallback graphics adapter");
                        adapter
                    }
                    Err(_) => {
                        log::error!("No compatible graphics adapter found");
                        return Err(GraphicsError::NoAdapter);
                    }
                }
            }
        };

        // Log adapter information
        let adapter_info = adapter.get_info();
        log::info!("Graphics adapter: {} ({:?})", adapter_info.name, adapter_info.backend);

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

        let (device, queue) = adapter
            .request_device(&device_descriptor)
            .await
            .map_err(GraphicsError::DeviceCreation)?;

        // Configure surface with robust fallback options
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        // Choose the best present mode with fallbacks
        let present_mode = if surface_caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
            wgpu::PresentMode::Mailbox
        } else if surface_caps.present_modes.contains(&wgpu::PresentMode::Immediate) {
            wgpu::PresentMode::Immediate
        } else {
            surface_caps.present_modes[0]
        };

        // Choose the best alpha mode with fallbacks
        let alpha_mode = if surface_caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            wgpu::CompositeAlphaMode::Opaque
        } else {
            surface_caps.alpha_modes[0]
        };

        let size = window.inner_size();
        
        // Ensure minimum valid dimensions
        let width = size.width.max(1);
        let height = size.height.max(1);
        
        log::info!("Surface configuration: {}x{}, format: {:?}, present_mode: {:?}, alpha_mode: {:?}", 
                   width, height, surface_format, present_mode, alpha_mode);
        
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode,
            alpha_mode,
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
        // Try to get surface texture with timeout handling
        match self.surface.get_current_texture() {
            Ok(texture) => Ok(texture),
            Err(wgpu::SurfaceError::Timeout) => {
                // On timeout, try to reconfigure the surface and retry once
                log::warn!("Surface texture acquisition timed out, attempting surface reconfiguration");
                self.surface.configure(&self.device, &self.config);
                
                // Retry once after reconfiguration
                self.surface.get_current_texture().map_err(|err| match err {
                    wgpu::SurfaceError::Lost => GraphicsError::SurfaceLost,
                    wgpu::SurfaceError::OutOfMemory => GraphicsError::SurfaceConfiguration {
                        reason: "Out of GPU memory after reconfiguration".to_string(),
                    },
                    wgpu::SurfaceError::Timeout => GraphicsError::SurfaceConfiguration {
                        reason: "Surface texture acquisition timed out even after reconfiguration. This may indicate graphics driver issues or incompatible hardware.".to_string(),
                    },
                    wgpu::SurfaceError::Outdated => GraphicsError::SurfaceConfiguration {
                        reason: "Surface configuration is outdated after reconfiguration".to_string(),
                    },
                    wgpu::SurfaceError::Other => GraphicsError::SurfaceConfiguration {
                        reason: "Unknown surface error occurred after reconfiguration".to_string(),
                    },
                })
            }
            Err(err) => Err(match err {
                wgpu::SurfaceError::Lost => GraphicsError::SurfaceLost,
                wgpu::SurfaceError::OutOfMemory => GraphicsError::SurfaceConfiguration {
                    reason: "Out of GPU memory".to_string(),
                },
                wgpu::SurfaceError::Outdated => GraphicsError::SurfaceConfiguration {
                    reason: "Surface configuration is outdated".to_string(),
                },
                wgpu::SurfaceError::Other => GraphicsError::SurfaceConfiguration {
                    reason: "Unknown surface error occurred".to_string(),
                },
                wgpu::SurfaceError::Timeout => unreachable!(), // Already handled above
            })
        }
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