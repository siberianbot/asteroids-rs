use std::{
    f32::consts::PI,
    ops::{Div, Mul},
    sync::{Arc, RwLock},
};

use glam::Vec2;

use crate::{
    consts::VEC2_RIGHT,
    game::{
        ecs::ECS,
        entities::EntityId,
        players::{PlayerId, Players},
    },
};

/// [crate::game::entities::Camera] zoom direction
pub enum CameraZoomDirection {
    In,
    Out,
}

/// [crate::game::entities::Spacecraft] acceleration direction
pub enum SpacecraftAccelerationDirection {
    Forward,
    Backward,
}

/// [crate::game::entities::Spacecraft] incline direction
pub enum SpacecraftInclineDirection {
    Left,
    Right,
}

/// Controller: dispatches commands to entities and players
pub struct Controller {
    ecs: Arc<ECS>,
    players: Arc<Players>,

    player_id: RwLock<Option<PlayerId>>,
    camera_id: RwLock<Option<EntityId>>,
}

impl Controller {
    /// Creates new instance of [Controller]
    pub fn new(ecs: Arc<ECS>, players: Arc<Players>) -> Arc<Controller> {
        let controller = Controller {
            ecs,
            players,
            player_id: Default::default(),
            camera_id: Default::default(),
        };

        Arc::new(controller)
    }

    /// Sets [PlayerId] of controllable player
    pub fn set_player(&self, player_id: Option<PlayerId>) {
        *self.player_id.write().unwrap() = player_id;
    }

    /// Sets [EntityId] of controllable [crate::game::entities::Camera] entity
    pub fn set_camera(&self, camera_id: Option<EntityId>) {
        *self.camera_id.write().unwrap() = camera_id;
    }

    /// Toggles following behavior current camera
    pub fn camera_follow_toggle(&self) {
        if let Some(camera_id) = self.camera_id.read().unwrap().clone() {
            self.ecs.write().modify(camera_id, |entity| {
                if let Some(camera) = entity.camera_mut() {
                    camera.follow = !camera.follow;
                }
            });
        }
    }

    /// Controls zoom of current camera
    pub fn camera_zoom(&self, direction: CameraZoomDirection) {
        const MIN_DISTANCE: f32 = 1.0;
        const MAX_DISTANCE: f32 = 32.0;
        const DISTANCE_MULTIPLIER: f32 = 2.0;

        if let Some(camera_id) = self.camera_id.read().unwrap().clone() {
            self.ecs.write().modify(camera_id, |entity| {
                if let Some(camera) = entity.camera_mut() {
                    camera.distance = match direction {
                        CameraZoomDirection::In => camera.distance.div(DISTANCE_MULTIPLIER),
                        CameraZoomDirection::Out => camera.distance.mul(DISTANCE_MULTIPLIER),
                    };

                    camera.distance = camera.distance.clamp(MIN_DISTANCE, MAX_DISTANCE);
                }
            });
        }
    }

    /// Gives acceleration to current player's spacecraft
    pub fn player_accelerate(&self, direction: SpacecraftAccelerationDirection) {
        const ACCELERATION: f32 = 2.0;
        const DECELERATION: f32 = -1.0;

        if let Some(player_id) = self.player_id.read().unwrap().clone() {
            self.players
                .visit_player(&player_id, |player| player.spacecraft_id)
                .flatten()
                .and_then(|spacecraft_id| {
                    self.ecs.write().modify(spacecraft_id, |entity| {
                        if entity.spacecraft().is_none() {
                            return;
                        }

                        let mut acceleration =
                            VEC2_RIGHT.rotate(entity.transform().rotation.sin_cos().into());

                        acceleration *= match direction {
                            SpacecraftAccelerationDirection::Forward => ACCELERATION,
                            SpacecraftAccelerationDirection::Backward => DECELERATION,
                        };

                        if let Some(movement) = entity.movement_mut() {
                            movement.acceleration = acceleration;
                        }
                    })
                });
        }
    }

    /// Discards acceleration of current player's spacecraft
    pub fn player_stop_accelerate(&self) {
        if let Some(player_id) = self.player_id.read().unwrap().clone() {
            self.players
                .visit_player(&player_id, |player| player.spacecraft_id)
                .flatten()
                .and_then(|spacecraft_id| {
                    self.ecs.write().modify(spacecraft_id, |entity| {
                        if entity.spacecraft().is_none() {
                            return;
                        }

                        if let Some(movement) = entity.movement_mut() {
                            movement.acceleration = Vec2::ZERO;
                        }
                    })
                });
        }
    }

    /// Inclines current player's spacecraft
    pub fn player_incline(&self, direction: SpacecraftInclineDirection) {
        const ROTATION_VELOCITY: f32 = PI;

        if let Some(player_id) = self.player_id.read().unwrap().clone() {
            self.players
                .visit_player(&player_id, |player| player.spacecraft_id)
                .flatten()
                .and_then(|spacecraft_id| {
                    self.ecs.write().modify(spacecraft_id, |entity| {
                        entity.spacecraft_mut().map(|spacecraft| {
                            spacecraft.rotation_velocity = ROTATION_VELOCITY
                                * match direction {
                                    SpacecraftInclineDirection::Left => 1.0,
                                    SpacecraftInclineDirection::Right => -1.0,
                                }
                        });
                    })
                });
        }
    }

    /// Discards incline of current player's spacecraft
    pub fn player_stop_incline(&self) {
        if let Some(player_id) = self.player_id.read().unwrap().clone() {
            self.players
                .visit_player(&player_id, |player| player.spacecraft_id)
                .flatten()
                .and_then(|spacecraft_id| {
                    self.ecs.write().modify(spacecraft_id, |entity| {
                        entity
                            .spacecraft_mut()
                            .map(|spacecraft| spacecraft.rotation_velocity = 0.0);
                    })
                });
        }
    }

    /// Fires a weapon of current player's spacecraft
    pub fn player_weapon_fire(&self) {
        if let Some(player_id) = self.player_id.read().unwrap().clone() {
            self.players
                .visit_player(&player_id, |player| player.spacecraft_id)
                .flatten()
                .and_then(|spacecraft_id| {
                    self.ecs.write().modify(spacecraft_id, |entity| {
                        entity
                            .spacecraft_mut()
                            .map(|spacecraft| spacecraft.weapon_fire = true);
                    })
                });
        }
    }

    /// Stops weapon fire of current player's spacecraft
    pub fn player_stop_weapon_fire(&self) {
        if let Some(player_id) = self.player_id.read().unwrap().clone() {
            self.players
                .visit_player(&player_id, |player| player.spacecraft_id)
                .flatten()
                .and_then(|spacecraft_id| {
                    self.ecs.write().modify(spacecraft_id, |entity| {
                        entity
                            .spacecraft_mut()
                            .map(|spacecraft| spacecraft.weapon_fire = false);
                    })
                });
        }
    }
}
