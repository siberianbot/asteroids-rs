use std::sync::Arc;

use crate::{
    assets,
    commands::{Commands, Registration, StatefulCommand},
    dispatch::{Dispatcher, Event},
    game::{
        controller::Controller,
        ecs::{ECS, StatefulSystem, StatelessSystem},
        r#loop::{Loop, StatefulGameLogic},
        physics::Physics,
        players::Players,
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
mod players;
mod systems;

/// Game infrastructure
pub struct Game {
    _commands: [Registration; 8],
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
        let r#loop: Arc<Loop> = Default::default();
        let players = Players::new(event_dispatcher);
        let controller = Controller::new(ecs.clone(), players.clone());
        let physics = Physics::new(event_dispatcher, ecs.clone());

        ecs.add_system(
            "camera_sync_system",
            StatefulSystem::new(
                systems::CameraSyncSystemState::new(players.clone()),
                systems::camera_sync_system,
            ),
        );

        ecs.add_system(
            "movement_system",
            Into::<StatelessSystem>::into(systems::movement_system),
        );

        ecs.add_system(
            "spacecraft_weapon_fire_system",
            Into::<StatelessSystem>::into(systems::spacecraft_weapon_fire_system),
        );

        ecs.add_system(
            "spacecraft_weapon_cooldown_system",
            Into::<StatelessSystem>::into(systems::spacecraft_weapon_cooldown_system),
        );

        ecs.add_system(
            "spacecraft_rotation_system",
            Into::<StatelessSystem>::into(systems::spacecraft_rotation_system),
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
                systems::EntityDespawnSystemState::new(players.clone()),
                systems::entity_despawn_system,
            ),
        );

        r#loop.add_logic(
            "init_game_logic",
            StatefulGameLogic::new(
                logics::InitGameLogicState::new(
                    assets.clone(),
                    renderer.clone(),
                    ecs.clone(),
                    players.clone(),
                    controller.clone(),
                ),
                logics::init_game_logic,
            ),
        );

        r#loop.add_logic(
            "asteroids_respawn_game_logic",
            StatefulGameLogic::new(
                logics::AsteroidsRespawnGameLogicState::new(
                    assets.clone(),
                    ecs.clone(),
                    players.clone(),
                ),
                logics::asteroids_respawn_game_logic,
            ),
        );

        r#loop.add_logic(
            "players_respawn_game_logic",
            StatefulGameLogic::new(
                logics::PlayersRespawnGameLogicState::new(ecs.clone(), players.clone()),
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
                commands.add(
                    "player_forward",
                    StatefulCommand::new(controller.clone(), commands::player_forward_command),
                ),
                commands.add(
                    "player_backward",
                    StatefulCommand::new(controller.clone(), commands::player_backward_command),
                ),
                commands.add(
                    "player_incline_left",
                    StatefulCommand::new(controller.clone(), commands::player_incline_left_command),
                ),
                commands.add(
                    "player_incline_right",
                    StatefulCommand::new(
                        controller.clone(),
                        commands::player_incline_right_command,
                    ),
                ),
                commands.add(
                    "player_weapon_fire",
                    StatefulCommand::new(controller.clone(), commands::player_weapon_fire_command),
                ),
            ],

            _workers: [
                ecs::spawn_worker(ecs),
                r#loop::spawn_worker(r#loop),
                physics::spawn_worker(physics),
            ],
        };

        Arc::new(game)
    }
}
