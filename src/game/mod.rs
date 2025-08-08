use std::sync::Arc;

use crate::{assets, commands as app_commands, events, handle, rendering::renderer, workers};

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
    _systems: [handle::Handle; 9],
    _logics: [handle::Handle; 3],
    _commands: [handle::Handle; 8],
    _workers: [handle::Handle; 3],
}

impl Game {
    /// Creates new instance of [Game] with default systems and game logics
    pub fn new(
        workers: &workers::Workers,
        events: &events::Events,
        commands: Arc<app_commands::Commands>,
        assets: Arc<assets::Assets>,
        renderer: Arc<renderer::Renderer>,
    ) -> Arc<Game> {
        let ecs = ecs::ECS::new(events);
        let r#loop: Arc<r#loop::Loop> = Default::default();
        let players = players::Players::new(events);
        let controller = controller::Controller::new(ecs.clone(), players.clone());
        let physics = physics::Physics::new(ecs.clone());

        let game = Game {
            _systems: [
                ecs.add_system(
                    "camera_sync_system",
                    ecs::StatefulSystem::new(
                        systems::CameraSyncSystemState::new(players.clone()),
                        systems::camera_sync_system,
                    ),
                ),
                ecs.add_system(
                    "movement_system",
                    Into::<ecs::StatelessSystem>::into(systems::movement_system),
                ),
                ecs.add_system(
                    "spacecraft_weapon_fire_system",
                    Into::<ecs::StatelessSystem>::into(systems::spacecraft_weapon_fire_system),
                ),
                ecs.add_system(
                    "spacecraft_weapon_cooldown_system",
                    Into::<ecs::StatelessSystem>::into(systems::spacecraft_weapon_cooldown_system),
                ),
                ecs.add_system(
                    "spacecraft_rotation_system",
                    Into::<ecs::StatelessSystem>::into(systems::spacecraft_rotation_system),
                ),
                ecs.add_system(
                    "asteroid_rotation_system",
                    Into::<ecs::StatelessSystem>::into(systems::asteroid_rotation_system),
                ),
                ecs.add_system(
                    "renderer_dispatch_system",
                    ecs::StatefulSystem::new(
                        systems::RendererDispatchSystemState::new(renderer.clone()),
                        systems::renderer_dispatch_system,
                    ),
                ),
                ecs.add_system(
                    "entity_despawn_system",
                    ecs::StatefulSystem::new(
                        systems::EntityDespawnSystemState::new(players.clone()),
                        systems::entity_despawn_system,
                    ),
                ),
                ecs.add_system(
                    "entity_collision_system",
                    Into::<ecs::StatelessSystem>::into(systems::entity_collision_system),
                ),
            ],

            _logics: [
                r#loop.add_logic(
                    "init_game_logic",
                    r#loop::StatefulGameLogic::new(
                        logics::InitGameLogicState::new(
                            assets.clone(),
                            renderer.clone(),
                            ecs.clone(),
                            players.clone(),
                            controller.clone(),
                        ),
                        logics::init_game_logic,
                    ),
                ),
                r#loop.add_logic(
                    "asteroids_respawn_game_logic",
                    r#loop::StatefulGameLogic::new(
                        logics::AsteroidsRespawnGameLogicState::new(
                            assets.clone(),
                            ecs.clone(),
                            players.clone(),
                        ),
                        logics::asteroids_respawn_game_logic,
                    ),
                ),
                r#loop.add_logic(
                    "players_respawn_game_logic",
                    r#loop::StatefulGameLogic::new(
                        logics::PlayersRespawnGameLogicState::new(ecs.clone(), players.clone()),
                        logics::players_respawn_game_logic,
                    ),
                ),
            ],

            _commands: [
                commands.add(
                    "camera_follow",
                    app_commands::StatefulCommand::new(
                        controller.clone(),
                        commands::camera_follow_command,
                    ),
                ),
                commands.add(
                    "camera_zoom_out",
                    app_commands::StatefulCommand::new(
                        controller.clone(),
                        commands::camera_zoom_out_command,
                    ),
                ),
                commands.add(
                    "camera_zoom_in",
                    app_commands::StatefulCommand::new(
                        controller.clone(),
                        commands::camera_zoom_in_command,
                    ),
                ),
                commands.add(
                    "player_forward",
                    app_commands::StatefulCommand::new(
                        controller.clone(),
                        commands::player_forward_command,
                    ),
                ),
                commands.add(
                    "player_backward",
                    app_commands::StatefulCommand::new(
                        controller.clone(),
                        commands::player_backward_command,
                    ),
                ),
                commands.add(
                    "player_incline_left",
                    app_commands::StatefulCommand::new(
                        controller.clone(),
                        commands::player_incline_left_command,
                    ),
                ),
                commands.add(
                    "player_incline_right",
                    app_commands::StatefulCommand::new(
                        controller.clone(),
                        commands::player_incline_right_command,
                    ),
                ),
                commands.add(
                    "player_weapon_fire",
                    app_commands::StatefulCommand::new(
                        controller.clone(),
                        commands::player_weapon_fire_command,
                    ),
                ),
            ],

            _workers: [
                ecs::spawn_worker(workers, ecs),
                r#loop::spawn_worker(workers, r#loop),
                physics::spawn_worker(workers, physics),
            ],
        };

        Arc::new(game)
    }
}
