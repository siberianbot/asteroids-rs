use std::{
    f32::consts::PI,
    ops::RangeInclusive,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use glam::Vec2;
use rand::seq::IteratorRandom;

use crate::{
    game::{
        ecs::ECS,
        entities::{
            Asteroid, Camera, CameraComponent, CameraTarget, Spacecraft, TransformComponent,
        },
        state::State,
    },
    rendering::renderer,
};

/// State for [init_game_logic]
pub struct InitGameLogicState {
    renderer: Arc<renderer::Renderer>,
    ecs: Arc<ECS>,
    game_state: Arc<State>,
    initialized: AtomicBool,
}

impl InitGameLogicState {
    /// Creates new instance of [InitGameLogicState]
    pub fn new(
        renderer: Arc<renderer::Renderer>,
        ecs: Arc<ECS>,
        game_state: Arc<State>,
    ) -> InitGameLogicState {
        InitGameLogicState {
            renderer,
            ecs,
            game_state,
            initialized: Default::default(),
        }
    }
}

/// Game logic for single time initialization
pub fn init_game_logic(_: f32, state: &InitGameLogicState) {
    if state.initialized.load(Ordering::Relaxed) {
        return;
    }

    state.initialized.store(true, Ordering::Relaxed);

    let player_id = state.game_state.new_player();

    let camera = Camera {
        camera: CameraComponent {
            target: CameraTarget::Player(player_id),
            ..Default::default()
        },
        ..Default::default()
    };
    let camera_id = state.ecs.write().create(camera);

    state.renderer.set_view(Some(camera_id));
}

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

/// State for [players_respawn_game_logic]
pub struct PlayersRespawnGameLogicState {
    ecs: Arc<ECS>,
    game_state: Arc<State>,
}

impl PlayersRespawnGameLogicState {
    /// Creates new instance for [PlayersRespawnGameLogicState]
    pub fn new(ecs: Arc<ECS>, game_state: Arc<State>) -> PlayersRespawnGameLogicState {
        PlayersRespawnGameLogicState { ecs, game_state }
    }
}

/// Game logic for respawning players
pub fn players_respawn_game_logic(elapsed: f32, state: &PlayersRespawnGameLogicState) {
    state
        .game_state
        .iter_players_mut()
        .filter(|(_, player)| player.spacecraft_id.is_none())
        .for_each(|(_, player)| {
            player.respawn_timer -= elapsed;

            if player.respawn_timer > 0.0 {
                return;
            }

            let spacecraft_id = state.ecs.write().create(Spacecraft::default());

            player.spacecraft_id = Some(spacecraft_id);
        });
}
