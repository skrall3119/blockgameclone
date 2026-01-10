//! Graphics abstraction layer

pub mod context;
pub mod buffer;
pub mod texture;
pub mod shader;
pub mod pipeline;
pub mod camera;
pub mod resize;
pub mod app;
pub mod integration_example;
pub mod error;

// Re-export commonly used types
pub use error::{
    RenderError, RenderResult,
    WindowError, WindowResult,
    GraphicsError, GraphicsResult,
    ShaderError, ShaderResult,
    BufferError, BufferResult,
    TextureError, TextureResult,
};
pub use camera::Camera;
pub use context::GraphicsContext;
pub use resize::{ResizeHandler, handle_window_resize};
pub use app::RenderApp;
pub use integration_example::{ResizeIntegrationExample, run_resize_example};