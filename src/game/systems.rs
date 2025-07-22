use std::sync::Arc;

use crate::game::{
    ecs::SystemArgs,
    players::{self, GamePlayers},
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
    players: Arc<GamePlayers>,
}

impl EntityDespawnSystemState {
    /// Creates new instance of [EntityDespawnSystemState]
    pub fn new(players: Arc<GamePlayers>) -> EntityDespawnSystemState {
        EntityDespawnSystemState { players }
    }
}

/// Despawns entity when its far away from any players
pub fn entity_despawn_system(args: SystemArgs, state: &EntityDespawnSystemState) {
    const MAX_DISTANCE: f32 = 150.0;

    if args.entity.asteroid().is_none() {
        return;
    }

    let players = state.players.players.read().unwrap();

    let any_near = players
        .iter()
        .filter_map(|player| {
            args.get_entity(player.spacecraft_id).map(|spacecraft| {
                args.entity
                    .transform()
                    .position
                    .distance(spacecraft.transform().position)
            })
        })
        .any(|distance| distance < MAX_DISTANCE);

    if !any_near {
        args.destroy();
    }
}

/// State for [renderer_dispatch_system]
pub struct RendererDispatchSystemState {
    // TODO
}

impl RendererDispatchSystemState {
    /// Creates new instance of [RendererDispatchSystemState]
    pub fn new() -> RendererDispatchSystemState {
        RendererDispatchSystemState {
            // TODO
        }
    }
}

/// Dispatches data from entity to renderer
pub fn renderer_dispatch_system(args: SystemArgs, state: &RendererDispatchSystemState) {
    match args.entity {
        Entity::Camera(camera) => {
            // TODO: send view data to renderer
        }

        Entity::Spacecraft(spacecraft) => {
            // TODO: send entity data to renderer
        }

        Entity::Asteroid(asteroid) => {
            // TODO: send entity data to renderer
        }

        Entity::Bullet(bullet) => {
            // TODO: send entity data to renderer
        }
    }
}
