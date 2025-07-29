use std::sync::Arc;

use crate::{
    game::{ecs::SystemArgs, state::State},
    rendering::renderer,
};

use super::entities::Entity;

/// Synchronizes camera position with target position
pub fn camera_sync_system(args: SystemArgs) {
    let position = args
        .entity
        .camera()
        .filter(|camera| camera.follow)
        .and_then(|camera| camera.target)
        .and_then(|target| {
            args.get_entity(target)
                .map(|entity| entity.transform().position)
        });

    if let Some(position) = position {
        args.modify(move |entity| entity.transform_mut().position = position);
    }
}

/// Controls entities movement
pub fn movement_system(args: SystemArgs) {
    const BREAKING_ACCELERATION_EPSILON: f32 = 0.01;
    const BREAKING_VELOCITY_MULTIPLIER: f32 = 0.5;

    let position_velocity = args
        .entity
        .movement()
        .map(|movement| match movement.const_velocity {
            true => (
                args.entity.transform().position + args.elapsed * movement.velocity,
                movement.velocity,
            ),

            false if movement.acceleration.length() > BREAKING_ACCELERATION_EPSILON => (
                args.entity.transform().position + args.elapsed * movement.velocity,
                movement.velocity + args.elapsed * movement.acceleration,
            ),

            false => (
                args.entity.transform().position + args.elapsed * movement.velocity,
                movement.velocity - args.elapsed * BREAKING_VELOCITY_MULTIPLIER * movement.velocity,
            ),
        });

    if let Some((position, velocity)) = position_velocity {
        args.modify(move |entity| {
            entity.transform_mut().position = position;
            entity.movement_mut().unwrap().velocity = velocity;
        });
    }
}

/// Updates spacecraft weapon cooldown
pub fn spacecraft_cooldown_system(args: SystemArgs) {
    let cooldown = args
        .entity
        .spacecraft()
        .filter(|spacecraft| spacecraft.cooldown > 0.0)
        .map(|spacecraft| {
            if spacecraft.cooldown < args.elapsed {
                0.0
            } else {
                spacecraft.cooldown - args.elapsed
            }
        });

    if let Some(cooldown) = cooldown {
        args.modify(move |entity| entity.spacecraft_mut().unwrap().cooldown = cooldown);
    }
}

/// Rotates asteroid by its rotation velocity
pub fn asteroid_rotation_system(args: SystemArgs) {
    let rotation = args.entity.asteroid().map(|asteroid| {
        args.entity.transform().rotation + args.elapsed * asteroid.rotation_velocity
    });

    if let Some(rotation) = rotation {
        args.modify(move |entity| entity.transform_mut().rotation = rotation);
    }
}

/// State for [entity_despawn_system]
pub struct EntityDespawnSystemState {
    game_state: Arc<State>,
}

impl EntityDespawnSystemState {
    /// Creates new instance of [EntityDespawnSystemState]
    pub fn new(game_state: Arc<State>) -> EntityDespawnSystemState {
        EntityDespawnSystemState { game_state }
    }
}

/// Despawns entity when its far away from any players
pub fn entity_despawn_system(args: SystemArgs, state: &EntityDespawnSystemState) {
    const MAX_DISTANCE: f32 = 150.0;

    let should_despawn = match args.entity {
        Entity::Camera(_) | Entity::Spacecraft(_) => false,
        _ => true,
    };

    if !should_despawn {
        return;
    }

    let any_near = state
        .game_state
        .iter_players()
        .filter_map(|(_, player)| {
            player.spacecraft_id.and_then(|spacecraft_id| {
                args.get_entity(spacecraft_id).map(|spacecraft| {
                    args.entity
                        .transform()
                        .position
                        .distance(spacecraft.transform().position)
                })
            })
        })
        .any(|distance| distance < MAX_DISTANCE);

    if !any_near {
        args.destroy();
    }
}

/// State for [renderer_dispatch_system]
pub struct RendererDispatchSystemState {
    renderer: Arc<renderer::Renderer>,
}

impl RendererDispatchSystemState {
    /// Creates new instance of [RendererDispatchSystemState]
    pub fn new(renderer: Arc<renderer::Renderer>) -> RendererDispatchSystemState {
        RendererDispatchSystemState { renderer }
    }
}

/// Dispatches data from entity to renderer
pub fn renderer_dispatch_system(args: SystemArgs, state: &RendererDispatchSystemState) {
    match args.entity {
        Entity::Camera(camera) => {
            state
                .renderer
                .dispatch::<renderer::ViewRenderData>(args.entity_id, camera.into());
        }

        Entity::Spacecraft(spacecraft) => {
            state
                .renderer
                .dispatch::<renderer::ModelRenderData>(args.entity_id, spacecraft.into());
        }

        Entity::Asteroid(asteroid) => {
            state
                .renderer
                .dispatch::<renderer::ModelRenderData>(args.entity_id, asteroid.into());
        }

        Entity::Bullet(bullet) => {
            state
                .renderer
                .dispatch::<renderer::ModelRenderData>(args.entity_id, bullet.into());
        }
    }
}
