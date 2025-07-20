use std::sync::Arc;

use crate::{
    dispatch::{Dispatcher, Event},
    game_ecs::{self, ECS, StatelessSystem},
    game_logics::{AsteroidsRespawnGameLogicState, asteroids_respawn_game_logic},
    game_loop::{self, GameLoop, StatefulGameLogic},
    game_physics::{self, Physics},
    game_players::GamePlayers,
    game_systems,
    worker::Worker,
};

/// Game infrastructure
pub struct Game {
    ecs: Arc<ECS>,
    _workers: [Worker; 3],
}

impl Game {
    /// Creates new instance of [Game] with default systems and game logics
    pub fn new(event_dispatcher: &Dispatcher<Event>) -> Arc<Game> {
        let ecs = ECS::new(event_dispatcher);
        let game_loop: Arc<GameLoop> = Default::default();
        let game_players: Arc<GamePlayers> = Default::default();
        let physics = Physics::new(event_dispatcher, ecs.clone());

        ecs.add_system(
            "camera_sync_system",
            Into::<StatelessSystem>::into(game_systems::camera_sync_system),
        );

        ecs.add_system(
            "movement_system",
            Into::<StatelessSystem>::into(game_systems::movement_system),
        );

        ecs.add_system(
            "spacecraft_cooldown_system",
            Into::<StatelessSystem>::into(game_systems::spacecraft_cooldown_system),
        );

        // TODO: add asteroid rotation system

        // TODO: add entities despawn system

        // TODO: add init game logic

        // TODO: add game logic, which ties game and renderer

        game_loop.add_logic(
            "asteroids_respawn_game_logic",
            StatefulGameLogic::new(
                AsteroidsRespawnGameLogicState::new(ecs.clone(), game_players),
                asteroids_respawn_game_logic,
            ),
        );

        let game = Game {
            _workers: [
                game_ecs::spawn_worker(ecs.clone()),
                game_loop::spawn_worker(game_loop),
                game_physics::spawn_worker(physics),
            ],

            ecs,
        };

        Arc::new(game)
    }

    /// Accesses to Entity-Component-System infrastructre within a game
    pub fn ecs(&self) -> Arc<ECS> {
        self.ecs.clone()
    }
}
