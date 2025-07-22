use std::{
    ops::{Div, Mul},
    sync::{Arc, atomic::Ordering},
};

use winit::{
    application::ApplicationHandler,
    dpi::Size,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    commands::{self, Commands},
    dispatch::{Dispatcher, Event, Sender},
    game::Game,
    input::{self},
    rendering::{backend::Backend, renderer::Renderer},
    worker::Worker,
};

#[derive(Debug)]
enum AppEvent {
    Exit,
}

pub const CAMERA_MIN_DISTANCE: f32 = 1.0;
pub const CAMERA_MAX_DISTANCE: f32 = 32.0;
pub const CAMERA_DISTANCE_MULTIPLIER: f32 = 2.0;

struct Inner {
    commands: Arc<Commands>,
    event_sender: Sender<Event>,
    input_manager: input::Manager,
    window: Arc<Window>,
    backend: Arc<Backend>,
    _renderer: Renderer,
}

impl Inner {
    fn new(
        commands: Arc<Commands>,
        event_dispatcher: &Dispatcher<Event>,
        game: Arc<Game>,
        event_loop: &ActiveEventLoop,
    ) -> Inner {
        let window = Inner::init_window(event_loop);

        let backend = Backend::new(event_dispatcher, event_loop, window.clone());
        let renderer = Renderer::new(event_dispatcher, game.clone(), backend.clone());

        let inner = Inner {
            event_sender: event_dispatcher.create_sender(),
            input_manager: Self::init_input(commands.clone(), game.clone()),
            commands,
            window,
            backend,
            _renderer: renderer,
        };

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

    fn init_input(commands: Arc<Commands>, game: Arc<Game>) -> input::Manager {
        let manager = input::Manager::new(commands);

        manager.set_key_map(input::Key::KbdEscape, "exit");
        manager.set_key_map(input::Key::KbdF, "camera_follow");
        manager.set_key_map(input::Key::KbdQ, "camera_zoom_out");
        manager.set_key_map(input::Key::KbdE, "camera_zoom_in");

        manager
    }

    fn dispatch_window_event(&mut self, _: &ActiveEventLoop, event: WindowEvent) {
        match event {
            WindowEvent::Resized(size) => {
                self.event_sender.send(Event::WindowResized(size.into()));
            }

            WindowEvent::RedrawRequested => {
                self.window.request_redraw();
            }

            WindowEvent::CloseRequested => {
                self.commands.invoke("exit", &[]);
            }

            WindowEvent::KeyboardInput { event, .. } => {
                self.input_manager.dispatch_key_event(event);
                self.input_manager.dispatch();
            }

            _ => {}
        }
    }
}

struct App {
    commands: Arc<Commands>,
    _command_registrations: [commands::Registration; 1],

    event_dispatcher: Arc<Dispatcher<Event>>,
    _dispatcher_worker: Worker,

    game: Arc<Game>,

    inner: Option<Inner>,
}

impl App {
    fn new(proxy: EventLoopProxy<AppEvent>) -> App {
        let commands: Arc<Commands> = Default::default();

        let event_dispatcher = Dispatcher::new();

        let game = Game::new(&event_dispatcher);

        let dispatcher_worker = {
            let event_dispatcher = event_dispatcher.clone();

            Worker::spawn("Dispatcher", move |alive| {
                while alive.load(Ordering::Relaxed) {
                    event_dispatcher.dispatch();
                }
            })
        };

        let app = App {
            _command_registrations: [commands.add("exit", move |_| {
                let _ = proxy.send_event(AppEvent::Exit);

                true
            })],
            commands,

            event_dispatcher,
            _dispatcher_worker: dispatcher_worker,

            game,

            inner: Default::default(),
        };

        app
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let inner = Inner::new(
            self.commands.clone(),
            &self.event_dispatcher,
            self.game.clone(),
            event_loop,
        );

        self.inner = Some(inner);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        self.inner
            .as_mut()
            .expect("there is no inner app state")
            .dispatch_window_event(event_loop, event);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::Exit => event_loop.exit(),
        }
    }
}

pub fn run() {
    let event_loop = EventLoop::with_user_event()
        .build()
        .expect("failed to create event loop for viewport");

    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy);

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).expect("application failure");
}
