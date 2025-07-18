use std::{
    f32::consts::PI,
    ops::RangeInclusive,
    sync::{Arc, Mutex},
};

use glam::Vec2;
use rand::seq::IteratorRandom;

use crate::{
    ecs::ECS,
    entity::{Asteroid, TransformComponent},
    game_common::GamePlayers,
};

/// State for [asteroids_respawn_game_logic]
pub struct AsteroidsRespawnGameLogicState {
    passed: Mutex<f32>,
    ecs: Arc<ECS>,
    players: Arc<GamePlayers>,
}

impl AsteroidsRespawnGameLogicState {
    /// Creates new instance of [AsteroidsRespawnGameLogicState]
    pub fn new(ecs: Arc<ECS>, players: Arc<GamePlayers>) -> AsteroidsRespawnGameLogicState {
        AsteroidsRespawnGameLogicState {
            passed: Default::default(),
            ecs,
            players,
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

    let count = state
        .ecs
        .iter_entities()
        .filter_map(|(_, entity)| entity.asteroid())
        .count();

    if count >= MAX_ASTEROIDS_COUNT {
        return;
    }

    let position = state
        .players
        .players
        .read()
        .unwrap()
        .iter()
        .filter_map(|player| {
            state
                .ecs
                .visit_entity(player.spacecraft_id, |entity| entity.transform().position)
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

    state.ecs.create_entity(asteroid);
}
