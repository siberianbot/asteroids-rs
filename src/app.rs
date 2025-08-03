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
    assets::Assets,
    commands::{self, Commands, StatefulCommand},
    dispatch::{Dispatcher, Event, Sender},
    game::Game,
    handle,
    input::{self, Input, Key, Scheme},
    rendering::{
        backend::Backend,
        renderer::{self, Renderer},
    },
    worker::Worker,
};

#[derive(Debug)]
enum AppEvent {
    Exit,
}

struct Inner {
    commands: Arc<Commands>,
    input: Arc<Input>,

    event_sender: Sender<Event>,
    game: Arc<Game>,
    window: Arc<Window>,

    _schemes: [handle::Handle; 1],
    _renderer_worker: Worker,
}

impl Inner {
    fn new(
        commands: Arc<Commands>,
        event_dispatcher: &Dispatcher<Event>,
        input: Arc<Input>,
        event_loop: &ActiveEventLoop,
    ) -> Inner {
        let window = Inner::init_window(event_loop);

        let backend = Backend::new(event_dispatcher, event_loop, window.clone());
        let assets = Assets::new(backend.clone());
        let renderer = Renderer::new(event_dispatcher, backend.clone(), assets.clone());

        let game = Game::new(
            event_dispatcher,
            commands.clone(),
            assets.clone(),
            renderer.clone(),
        );

        let inner = Inner {
            _schemes: [input.add_scheme(
                Scheme::default()
                    .add("camera_follow", [input::Key::KbdF])
                    .add("camera_zoom_out", [input::Key::KbdQ])
                    .add("camera_zoom_in", [input::Key::KbdE])
                    .add("player_forward", [input::Key::KbdW])
                    .add("player_backward", [input::Key::KbdS])
                    .add("player_incline_left", [input::Key::KbdA])
                    .add("player_incline_right", [input::Key::KbdD])
                    .add("player_weapon_fire", [input::Key::KbdSpace]),
            )],

            commands,
            input,

            event_sender: event_dispatcher.create_sender(),
            game,
            window,

            _renderer_worker: renderer::spawn_worker(renderer.clone()),
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
                self.input.dispatch_key_event(event);
            }

            _ => {}
        }
    }
}

struct App {
    commands: Arc<Commands>,
    event_dispatcher: Arc<Dispatcher<Event>>,
    input: Arc<Input>,

    _commands: [commands::Registration; 1],
    _schemes: [handle::Handle; 1],

    _dispatcher_worker: Worker,

    inner: Option<Inner>,
}

impl App {
    fn new(proxy: EventLoopProxy<AppEvent>) -> App {
        let commands: Arc<Commands> = Default::default();
        let event_dispatcher = Dispatcher::new();
        let input = Input::new(commands.clone());

        let dispatcher_worker = {
            let event_dispatcher = event_dispatcher.clone();

            Worker::spawn("Dispatcher", move |alive| {
                while alive.load(Ordering::Relaxed) {
                    event_dispatcher.dispatch();
                }
            })
        };

        let app = App {
            _commands: [commands.add(
                "exit",
                StatefulCommand::new(proxy, |_, proxy| {
                    let _ = proxy.send_event(AppEvent::Exit);

                    true
                }),
            )],

            _schemes: [input.add_scheme(Scheme::default().add("exit", [input::Key::KbdEscape]))],

            commands,
            event_dispatcher,
            input,

            _dispatcher_worker: dispatcher_worker,

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
            self.input.clone(),
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
