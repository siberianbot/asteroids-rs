use std::sync::Arc;

use glam::Vec2;

use crate::{
    game::{
        ecs::SystemArgs,
        entities::{
            Bullet, BulletComponent, CameraTarget, Entity, MovementComponent, TransformComponent,
        },
        physics::Collision,
        players::Players,
    },
    scene,
};

/// State for [camera_sync_system]
pub struct CameraSyncSystemState {
    players: Arc<Players>,
}

impl CameraSyncSystemState {
    /// Creates new instance of [CameraSyncSystemState]
    pub fn new(players: Arc<Players>) -> CameraSyncSystemState {
        CameraSyncSystemState { players }
    }
}

/// Synchronizes camera position with target position
pub fn camera_sync_system(args: SystemArgs, state: &CameraSyncSystemState) {
    let position = args
        .entity
        .camera()
        .filter(|camera| camera.follow)
        .and_then(|camera| match camera.target {
            CameraTarget::None => None,

            CameraTarget::Entity(entity_id) => args
                .get_entity(entity_id)
                .map(|entity| entity.transform().position),

            CameraTarget::Player(player_id) => state
                .players
                .visit_player(&player_id, |player| player.spacecraft_id)
                .flatten()
                .and_then(|entity_id| {
                    args.get_entity(entity_id)
                        .map(|entity| entity.transform().position)
                }),
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

/// Handles spacecraft weapon fire
pub fn spacecraft_weapon_fire_system(args: SystemArgs) {
    const BULLET_VELOCITY: f32 = 8.0;
    const COOLDOWN: f32 = 0.2;

    let bullet = args
        .entity
        .spacecraft()
        .filter(|spacecraft| spacecraft.weapon_fire)
        .and_then(|spacecraft| {
            if spacecraft.weapon_cooldown > 0.0 {
                None
            } else {
                let velocity = BULLET_VELOCITY
                    * Vec2::ONE.rotate(args.entity.transform().rotation.sin_cos().into());

                let bullet = Bullet {
                    transform: TransformComponent {
                        position: args.entity.transform().position,
                        ..Default::default()
                    },
                    movement: MovementComponent {
                        velocity,
                        const_velocity: true,
                        ..Default::default()
                    },
                    bullet: BulletComponent {
                        owner: spacecraft.owner.clone(),
                    },
                    ..Default::default()
                };

                Some(bullet)
            }
        });

    if let Some(bullet) = bullet {
        args.modify(|entity| entity.spacecraft_mut().unwrap().weapon_cooldown = COOLDOWN);
        args.create(move || bullet.into());
    }
}

/// Updates spacecraft weapon cooldown
pub fn spacecraft_weapon_cooldown_system(args: SystemArgs) {
    let cooldown = args
        .entity
        .spacecraft()
        .filter(|spacecraft| spacecraft.weapon_cooldown > 0.0)
        .map(|spacecraft| {
            if spacecraft.weapon_cooldown < args.elapsed {
                0.0
            } else {
                spacecraft.weapon_cooldown - args.elapsed
            }
        });

    if let Some(cooldown) = cooldown {
        args.modify(move |entity| entity.spacecraft_mut().unwrap().weapon_cooldown = cooldown);
    }
}

/// Rotates spacecraft by its rotation velocity
pub fn spacecraft_rotation_system(args: SystemArgs) {
    let rotation = args.entity.spacecraft().map(|spacecraft| {
        args.entity.transform().rotation + args.elapsed * spacecraft.rotation_velocity
    });

    if let Some(rotation) = rotation {
        args.modify(move |entity| entity.transform_mut().rotation = rotation);
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
    players: Arc<Players>,
}

impl EntityDespawnSystemState {
    /// Creates new instance of [EntityDespawnSystemState]
    pub fn new(players: Arc<Players>) -> EntityDespawnSystemState {
        EntityDespawnSystemState { players }
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
        .players
        .iter()
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

/// Handles collisions of entities
pub fn entity_collision_system(args: SystemArgs) {
    let should_destroy = match args.entity {
        Entity::Camera(_) | Entity::Spacecraft(_) => false,
        _ => true,
    };

    if !should_destroy {
        return;
    }

    let any_collided = args
        .entity
        .collider()
        .iter()
        .flat_map(|collider| {
            collider.collisions.iter().map(|Collision(entity_id)| {
                args.get_entity(*entity_id)
                    .is_some_and(|entity| match entity {
                        Entity::Camera(_) | Entity::Spacecraft(_) => false,
                        _ => true,
                    })
            })
        })
        .any(|collided| collided);

    if any_collided {
        args.destroy();
    }
}

/// State for [scene_dispatch_system]
pub struct SceneDispatchSystemState {
    scene: Arc<scene::Scene>,
}

impl SceneDispatchSystemState {
    /// Creates new instance of [SceneDispatchSystemState]
    pub fn new(scene: Arc<scene::Scene>) -> SceneDispatchSystemState {
        SceneDispatchSystemState { scene }
    }
}

/// Dispatches scene data from entities
pub fn scene_dispatch_system(args: SystemArgs, state: &SceneDispatchSystemState) {
    match args.entity {
        Entity::Camera(camera) => {
            state
                .scene
                .dispatch::<scene::ViewSceneEntity>(args.entity_id, camera.into());
        }

        Entity::Spacecraft(spacecraft) => {
            state
                .scene
                .dispatch::<scene::ModelSceneEntity>(args.entity_id, spacecraft.into());
        }

        Entity::Asteroid(asteroid) => {
            state
                .scene
                .dispatch::<scene::ModelSceneEntity>(args.entity_id, asteroid.into());
        }

        Entity::Bullet(bullet) => {
            state
                .scene
                .dispatch::<scene::ModelSceneEntity>(args.entity_id, bullet.into());
        }
    }
}
