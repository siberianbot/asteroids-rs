use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::Size,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

struct Inner {
    window: Arc<Window>,
}

impl Inner {
    fn new(event_loop: &ActiveEventLoop) -> Inner {
        let window = Inner::init_window(event_loop);

        let inner = Inner { window };

        inner
    }

    fn init_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
        let attributes = WindowAttributes::default()
            .with_title("Asteroids")
            .with_inner_size(Size::Logical([1280.0, 720.0].into()));

        let window = event_loop
            .create_window(attributes)
            .expect("failed to create window");

        Arc::new(window)
    }

    fn dispatch_window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => {
                // TODO: redraw

                self.window.request_redraw();
            }

            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            _ => {}
        }
    }
}

struct App {
    inner: Option<Inner>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.inner = Some(Inner::new(event_loop));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        self.inner
            .as_mut()
            .expect("there is no inner app state")
            .dispatch_window_event(event_loop, event);
    }
}

pub fn run() {
    let mut app = App::default();

    let event_loop = EventLoop::builder()
        .build()
        .expect("failed to create event loop for viewport");

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).expect("application failure");
}
