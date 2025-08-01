use std::sync::Arc;

use crate::{
    assets,
    commands::{Commands, Registration, StatefulCommand},
    dispatch::{Dispatcher, Event},
    game::{
        controller::Controller,
        ecs::{ECS, StatefulSystem, StatelessSystem},
        r#loop::{GameLoop, StatefulGameLogic},
        physics::Physics,
        state::State,
    },
    rendering::renderer,
    worker::Worker,
};

pub mod ecs;
pub mod entities;

mod commands;
mod controller;
mod logics;
mod r#loop;
mod physics;
mod state;
mod systems;

/// Game infrastructure
pub struct Game {
    _commands: [Registration; 3],
    _workers: [Worker; 3],
}

impl Game {
    /// Creates new instance of [Game] with default systems and game logics
    pub fn new(
        event_dispatcher: &Dispatcher<Event>,
        commands: Arc<Commands>,
        assets: Arc<assets::Assets>,
        renderer: Arc<renderer::Renderer>,
    ) -> Arc<Game> {
        let ecs = ECS::new(event_dispatcher);
        let game_loop: Arc<GameLoop> = Default::default();
        let game_state: Arc<State> = State::new(event_dispatcher);
        let controller = Controller::new(ecs.clone());
        let physics = Physics::new(event_dispatcher, ecs.clone());

        ecs.add_system(
            "camera_sync_system",
            StatefulSystem::new(
                systems::CameraSyncSystemState::new(game_state.clone()),
                systems::camera_sync_system,
            ),
        );

        ecs.add_system(
            "movement_system",
            Into::<StatelessSystem>::into(systems::movement_system),
        );

        ecs.add_system(
            "spacecraft_cooldown_system",
            Into::<StatelessSystem>::into(systems::spacecraft_cooldown_system),
        );

        ecs.add_system(
            "asteroid_rotation_system",
            Into::<StatelessSystem>::into(systems::asteroid_rotation_system),
        );

        ecs.add_system(
            "renderer_dispatch_system",
            StatefulSystem::new(
                systems::RendererDispatchSystemState::new(renderer.clone()),
                systems::renderer_dispatch_system,
            ),
        );

        ecs.add_system(
            "entity_despawn_system",
            StatefulSystem::new(
                systems::EntityDespawnSystemState::new(game_state.clone()),
                systems::entity_despawn_system,
            ),
        );

        game_loop.add_logic(
            "init_game_logic",
            StatefulGameLogic::new(
                logics::InitGameLogicState::new(
                    assets.clone(),
                    renderer.clone(),
                    ecs.clone(),
                    game_state.clone(),
                    controller.clone(),
                ),
                logics::init_game_logic,
            ),
        );

        game_loop.add_logic(
            "asteroids_respawn_game_logic",
            StatefulGameLogic::new(
                logics::AsteroidsRespawnGameLogicState::new(
                    assets.clone(),
                    ecs.clone(),
                    game_state.clone(),
                ),
                logics::asteroids_respawn_game_logic,
            ),
        );

        game_loop.add_logic(
            "players_respawn_game_logic",
            StatefulGameLogic::new(
                logics::PlayersRespawnGameLogicState::new(ecs.clone(), game_state.clone()),
                logics::players_respawn_game_logic,
            ),
        );

        let game = Game {
            _commands: [
                commands.add(
                    "camera_follow",
                    StatefulCommand::new(controller.clone(), commands::camera_follow_command),
                ),
                commands.add(
                    "camera_zoom_out",
                    StatefulCommand::new(controller.clone(), commands::camera_zoom_out_command),
                ),
                commands.add(
                    "camera_zoom_in",
                    StatefulCommand::new(controller.clone(), commands::camera_zoom_in_command),
                ),
            ],

            _workers: [
                ecs::spawn_worker(ecs),
                r#loop::spawn_worker(game_loop),
                physics::spawn_worker(physics),
            ],
        };

        Arc::new(game)
    }
}
