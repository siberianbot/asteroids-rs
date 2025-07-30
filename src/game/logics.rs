use std::{
    f32::consts::PI,
    iter::once,
    ops::RangeInclusive,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use glam::Vec2;
use rand::seq::IteratorRandom;
use vulkano::pipeline::graphics::input_assembly::PrimitiveTopology;

use crate::{
    assets::{self, MeshAssetDef, types::Vertex},
    game::{ecs::ECS, entities, state::State},
    rendering::{self, renderer},
};

/// State for [init_game_logic]
pub struct InitGameLogicState {
    assets: Arc<assets::Assets>,
    renderer: Arc<renderer::Renderer>,
    ecs: Arc<ECS>,
    game_state: Arc<State>,
    initialized: AtomicBool,
}

impl InitGameLogicState {
    /// Creates new instance of [InitGameLogicState]
    pub fn new(
        assets: Arc<assets::Assets>,
        renderer: Arc<renderer::Renderer>,
        ecs: Arc<ECS>,
        game_state: Arc<State>,
    ) -> InitGameLogicState {
        InitGameLogicState {
            assets,
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

    state.assets.load(
        entities::consts::ENTITY_PIPELINE_ASSET_REF.into(),
        assets::PipelineAssetDef {
            topology: PrimitiveTopology::TriangleList,
            shaders: vec![
                (
                    rendering::backend::ShaderStage::Vertex,
                    Box::new(assets::shaders::entity::vs::load),
                ),
                (
                    rendering::backend::ShaderStage::Fragment,
                    Box::new(assets::shaders::entity::fs::load),
                ),
            ],
        },
    );

    state.assets.load(
        entities::consts::SPACECRAFT_MESH_ASSET_REF.into(),
        assets::MeshAssetDef {
            vertices: assets::models::spacecraft::VERTICES.into(),
            indices: assets::models::spacecraft::INDICES.into(),
        },
    );

    state.assets.load(
        entities::consts::BULLET_MESH_ASSET_REF.into(),
        assets::MeshAssetDef {
            vertices: assets::models::bullet::VERTICES.into(),
            indices: assets::models::bullet::INDICES.into(),
        },
    );

    let player_id = state.game_state.new_player();

    let camera = entities::Camera {
        camera: entities::CameraComponent {
            target: entities::CameraTarget::Player(player_id),
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
    assets: Arc<assets::Assets>,
    ecs: Arc<ECS>,
    game_state: Arc<State>,
}

impl AsteroidsRespawnGameLogicState {
    /// Creates new instance of [AsteroidsRespawnGameLogicState]
    pub fn new(
        assets: Arc<assets::Assets>,
        ecs: Arc<ECS>,
        game_state: Arc<State>,
    ) -> AsteroidsRespawnGameLogicState {
        AsteroidsRespawnGameLogicState {
            passed: Default::default(),
            assets,
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

    let asteroid = entities::Asteroid {
        transform: entities::TransformComponent {
            position,
            ..Default::default()
        },
        ..Default::default()
    };

    let asteroid_mesh_def = MeshAssetDef {
        vertices: once(Vertex::default())
            .chain(
                asteroid
                    .asteroid
                    .body
                    .iter()
                    .copied()
                    .map(|vertex| Vertex { position: vertex }),
            )
            .collect(),

        indices: (1..=entities::consts::ASTEROID_SEGMENTS_COUNT)
            .flat_map(|index| {
                let next_index = (index + 1) % entities::consts::ASTEROID_SEGMENTS_COUNT;

                [0, index as u32, next_index.max(1) as u32]
            })
            .collect(),
    };

    state
        .assets
        .load(asteroid.render.mesh.clone(), asteroid_mesh_def);

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

            let spacecraft_id = state.ecs.write().create(entities::Spacecraft::default());

            player.spacecraft_id = Some(spacecraft_id);
        });
}
