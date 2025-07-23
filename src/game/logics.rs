use std::{
    f32::consts::PI,
    ops::RangeInclusive,
    sync::{Arc, Mutex},
};

use glam::Vec2;
use rand::seq::IteratorRandom;

use crate::game::{
    ecs::ECS,
    entities::{Asteroid, TransformComponent},
    state::State,
};

/// State for [asteroids_respawn_game_logic]
pub struct AsteroidsRespawnGameLogicState {
    passed: Mutex<f32>,
    ecs: Arc<ECS>,
    game_state: Arc<State>,
}

impl AsteroidsRespawnGameLogicState {
    /// Creates new instance of [AsteroidsRespawnGameLogicState]
    pub fn new(ecs: Arc<ECS>, game_state: Arc<State>) -> AsteroidsRespawnGameLogicState {
        AsteroidsRespawnGameLogicState {
            passed: Default::default(),
            ecs,
            game_state,
        }
    }
}

/// Game logic for respawning asteroids
pub fn asteroids_respawn_game_logic(elapsed: f32, state: &AsteroidsRespawnGameLogicState) {
    const RESPAWN_THRESHOLD: f32 = 1.0;
    const MAX_ASTEROIDS_COUNT: usize = 64;
    const DISTANCE_RANGE: RangeInclusive<f32> = 15.0..=100.0;
    const ROTATION_RANGE: RangeInclusive<f32> = 0.0..=2.0 * PI;

    let mut passed = state.passed.lock().unwrap();

    *passed += elapsed;

    if *passed < RESPAWN_THRESHOLD {
        return;
    }

    *passed = 0.0;

    let mut entities = state.ecs.write();

    let count = entities
        .iter()
        .filter_map(|(_, entity)| entity.asteroid())
        .count();

    if count >= MAX_ASTEROIDS_COUNT {
        return;
    }

    let position = state
        .game_state
        .iter_players()
        .filter_map(|(_, player)| {
            player.spacecraft_id.and_then(|spacecraft_id| {
                entities
                    .get(spacecraft_id)
                    .map(|entity| entity.transform().position)
            })
        })
        .choose(&mut rand::rng())
        .unwrap_or_else(|| Vec2::ZERO);

    let distance = rand::random_range(DISTANCE_RANGE);
    let rotation = rand::random_range(ROTATION_RANGE);
    let position = position + distance * Vec2::ONE.rotate(rotation.sin_cos().into());

    let asteroid = Asteroid {
        transform: TransformComponent {
            position,
            ..Default::default()
        },
        ..Default::default()
    };

    entities.create(asteroid);
}
