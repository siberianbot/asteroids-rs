use std::sync::{Arc, atomic::Ordering};

use winit::{
    application::ApplicationHandler,
    dpi::Size,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    dispatch::{Command, Dispatcher, Event, Sender},
    game::Game,
    rendering::{backend::Backend, renderer::Renderer},
    worker::Worker,
};

#[derive(Debug)]
enum AppEvent {
    Exit,
}

struct Inner {
    command_sender: Sender<Command>,
    event_sender: Sender<Event>,
    window: Arc<Window>,
    backend: Arc<Backend>,
    _renderer: Renderer,
}

impl Inner {
    fn new(
        command_dispatcher: &Dispatcher<Command>,
        event_dispatcher: &Dispatcher<Event>,
        game: Arc<Game>,
        event_loop: &ActiveEventLoop,
    ) -> Inner {
        let window = Inner::init_window(event_loop);

        let backend = Backend::new(event_dispatcher, event_loop, window.clone());
        let renderer = Renderer::new(event_dispatcher, game, backend.clone());

        let inner = Inner {
            command_sender: command_dispatcher.create_sender(),
            event_sender: event_dispatcher.create_sender(),
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

    fn dispatch_window_event(&mut self, _: &ActiveEventLoop, event: WindowEvent) {
        match event {
            WindowEvent::Resized(size) => {
                self.event_sender.send(Event::WindowResized(size.into()));
            }

            WindowEvent::RedrawRequested => {
                self.window.request_redraw();
            }

            WindowEvent::CloseRequested => {
                self.command_sender.send(Command::Exit);
            }

            _ => {}
        }
    }
}

struct App {
    command_dispatcher: Arc<Dispatcher<Command>>,
    event_dispatcher: Arc<Dispatcher<Event>>,
    _dispatcher_worker: Worker,

    game: Arc<Game>,

    inner: Option<Inner>,
}

impl App {
    fn new(proxy: EventLoopProxy<AppEvent>) -> App {
        let command_dispatcher = Dispatcher::new();
        let event_dispatcher = Dispatcher::new();

        command_dispatcher.add_handler(move |command: &Command| match command {
            Command::Exit => proxy
                .send_event(AppEvent::Exit)
                .expect("event loop is not exist anymore"),

            _ => {}
        });

        let game = Game::new(&command_dispatcher, &event_dispatcher);

        let dispatcher_worker = {
            let command_dispatcher = command_dispatcher.clone();
            let event_dispatcher = event_dispatcher.clone();

            Worker::spawn("Dispatcher", move |alive| {
                while alive.load(Ordering::Relaxed) {
                    command_dispatcher.dispatch();
                    event_dispatcher.dispatch();
                }
            })
        };

        let app = App {
            command_dispatcher,
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
            &self.command_dispatcher,
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
