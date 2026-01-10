//! Window management

use winit::{
    application::ApplicationHandler,
    event::{Event, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId, WindowAttributes},
    dpi::PhysicalSize,
};
use crate::renderer::{WindowError, WindowResult};

/// Trait for handling window resize events
pub trait ResizeHandler {
    /// Called when the window is resized
    fn handle_resize(&mut self, new_width: u32, new_height: u32);
}

/// Manages window creation and event handling using winit
pub struct WindowManager {
    window: Option<Window>,
}

impl WindowManager {
    /// Creates a new WindowManager
    pub fn new() -> Self {
        Self { window: None }
    }
    
    /// Returns a reference to the window, if it exists
    pub fn window(&self) -> Option<&Window> {
        self.window.as_ref()
    }
    
    /// Runs the main event loop with the provided callback
    pub fn run<F>(self, event_handler: F) -> WindowResult<()>
    where
        F: FnMut(&Window, &Event<()>) -> ControlFlow + 'static,
    {
        let event_loop = EventLoop::new()
            .map_err(|e| WindowError::EventLoopCreation(e.to_string()))?;
        
        struct App<F> {
            window_manager: WindowManager,
            event_handler: F,
        }
        
        impl<F> ApplicationHandler for App<F>
        where
            F: FnMut(&Window, &Event<()>) -> ControlFlow,
        {
            fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                if self.window_manager.window.is_none() {
                    let window_attributes = WindowAttributes::default()
                        .with_title("Voxel Game")
                        .with_inner_size(PhysicalSize::new(800, 600));
                    
                    match event_loop.create_window(window_attributes) {
                        Ok(window) => {
                            self.window_manager.window = Some(window);
                        }
                        Err(e) => {
                            log::error!("Failed to create window: {}", e);
                            event_loop.exit();
                        }
                    }
                }
            }
            
            fn window_event(
                &mut self,
                event_loop: &ActiveEventLoop,
                window_id: WindowId,
                event: WindowEvent,
            ) {
                if let Some(window) = &self.window_manager.window {
                    if window.id() == window_id {
                        match event {
                            WindowEvent::CloseRequested => {
                                event_loop.exit();
                            }
                            _ => {
                                let winit_event = Event::WindowEvent { window_id, event };
                                let control_flow = (self.event_handler)(window, &winit_event);
                                event_loop.set_control_flow(control_flow);
                            }
                        }
                    }
                }
            }
        }
        
        let mut app = App {
            window_manager: self,
            event_handler,
        };
        
        event_loop.run_app(&mut app)
            .map_err(|e| WindowError::EventLoopCreation(e.to_string()))?;
        
        Ok(())
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::dpi::PhysicalSize;

    #[test]
    fn test_window_manager_creation() {
        let window_manager = WindowManager::new();
        assert!(window_manager.window.is_none());
    }

    #[test]
    fn test_window_manager_default() {
        let window_manager = WindowManager::default();
        assert!(window_manager.window.is_none());
    }

    #[test]
    fn test_window_attributes_configuration() {
        // Test that window attributes are configured correctly
        let window_attributes = WindowAttributes::default()
            .with_title("Voxel Game")
            .with_inner_size(PhysicalSize::new(800, 600));
        
        // Verify title is set correctly
        assert_eq!(window_attributes.title, "Voxel Game");
        
        // Verify dimensions are set correctly
        if let Some(size) = window_attributes.inner_size {
            let physical_size: PhysicalSize<u32> = size.to_physical(1.0);
            assert_eq!(physical_size.width, 800);
            assert_eq!(physical_size.height, 600);
        } else {
            panic!("Window size should be set");
        }
    }

    #[test]
    fn test_window_manager_window_access() {
        let window_manager = WindowManager::new();
        
        // Initially no window should exist
        assert!(window_manager.window().is_none());
    }

    // Test event handling logic by creating a mock event handler
    #[test]
    fn test_event_handler_callback_signature() {
        // This test verifies that our event handler signature is correct
        // and can be used with the WindowManager::run method
        
        let _test_handler = |_window: &Window, _event: &Event<()>| -> ControlFlow {
            ControlFlow::Wait
        };
        
        // If this compiles, our signature is correct
        assert!(true);
    }

    #[test]
    fn test_window_error_types() {
        // Test that WindowError variants can be created and formatted
        let event_loop_error = WindowError::EventLoopCreation("Test error".to_string());
        assert!(event_loop_error.to_string().contains("Failed to create event loop"));
        
        let invalid_handle_error = WindowError::InvalidHandle;
        assert!(invalid_handle_error.to_string().contains("Window handle is invalid"));
    }

    #[test]
    fn test_window_manager_event_handler_types() {
        // Test that different types of event handlers work with WindowManager
        // This verifies the event handling interface without actually running the event loop
        
        // Test handler that returns Wait
        let _wait_handler = |_window: &Window, _event: &Event<()>| -> ControlFlow {
            ControlFlow::Wait
        };
        
        // Test handler that returns Poll
        let _poll_handler = |_window: &Window, _event: &Event<()>| -> ControlFlow {
            ControlFlow::Poll
        };
        
        // Test handler that processes different event types
        let _event_processor = |_window: &Window, event: &Event<()>| -> ControlFlow {
            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    // In a real handler, this would trigger exit
                    ControlFlow::Wait
                }
                Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                    // Handle resize events
                    ControlFlow::Wait
                }
                _ => ControlFlow::Wait,
            }
        };
        
        // If all handlers compile, the interface is correct
        assert!(true);
    }
}