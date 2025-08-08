use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::Size,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    assets, commands, events, game, handle, input,
    rendering::{backend, renderer},
    workers,
};

#[derive(Debug)]
enum AppEvent {
    Exit,
}

struct State {
    commands: Arc<commands::Commands>,
    input: Arc<input::Input>,
    window: Arc<Window>,

    _game: Arc<game::Game>,
    _schemes: [handle::Handle; 1],
    _workers: [handle::Handle; 1],
}

impl State {
    fn new(
        workers: &workers::Workers,
        events: &events::Events,
        commands: Arc<commands::Commands>,
        input: Arc<input::Input>,
        event_loop: &ActiveEventLoop,
    ) -> State {
        let window = State::init_window(event_loop);

        let backend = backend::Backend::new(event_loop, window.clone());
        let assets = assets::Assets::new(backend.clone());
        let renderer = renderer::Renderer::new(events, backend.clone(), assets.clone());

        let inner = State {
            _game: game::Game::new(
                workers,
                events,
                commands.clone(),
                assets.clone(),
                renderer.clone(),
            ),

            _schemes: [input.add_scheme(
                input::Scheme::default()
                    .add("camera_follow", [input::Key::KbdF])
                    .add("camera_zoom_out", [input::Key::KbdQ])
                    .add("camera_zoom_in", [input::Key::KbdE])
                    .add("player_forward", [input::Key::KbdW])
                    .add("player_backward", [input::Key::KbdS])
                    .add("player_incline_left", [input::Key::KbdA])
                    .add("player_incline_right", [input::Key::KbdD])
                    .add("player_weapon_fire", [input::Key::KbdSpace]),
            )],

            _workers: [renderer::spawn_worker(workers, renderer.clone())],

            commands,
            input,
            window,
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
    workers: workers::Workers,
    commands: Arc<commands::Commands>,
    events: Arc<events::Events>,
    input: Arc<input::Input>,

    state: Option<State>,

    _commands: [commands::Registration; 1],
    _schemes: [handle::Handle; 1],
    _workers: [handle::Handle; 1],
}

impl App {
    fn new(proxy: EventLoopProxy<AppEvent>) -> App {
        let workers: workers::Workers = Default::default();
        let commands: Arc<commands::Commands> = Default::default();
        let events: Arc<events::Events> = Default::default();
        let input = input::Input::new(commands.clone());

        let app = App {
            _commands: [commands.add(
                "exit",
                commands::StatefulCommand::new(proxy, |_, proxy| {
                    let _ = proxy.send_event(AppEvent::Exit);

                    true
                }),
            )],

            _schemes: [
                input.add_scheme(input::Scheme::default().add("exit", [input::Key::KbdEscape]))
            ],

            _workers: [events::spawn_worker(&workers, events.clone())],

            state: Default::default(),

            workers,
            commands,
            events,
            input,
        };

        app
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let state = State::new(
            &self.workers,
            &self.events,
            self.commands.clone(),
            self.input.clone(),
            event_loop,
        );

        self.state = Some(state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        self.state
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
