//! Platform abstraction layer - window, input, file system, timing

pub mod window;
pub mod input;
pub mod filesystem;
pub mod timing;

// Re-export commonly used types
pub use window::{WindowManager, ResizeHandler};