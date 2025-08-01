use std::{
    ops::{Div, Mul},
    sync::{Arc, RwLock},
};

use crate::game::{ecs::ECS, entities::EntityId, players::PlayerId};

/// [crate::game::entities::Camera] zoom direction
pub enum CameraZoomDirection {
    In,
    Out,
}

/// Controller: dispatches commands to entities and players
pub struct Controller {
    ecs: Arc<ECS>,

    player_id: RwLock<Option<PlayerId>>,
    camera_id: RwLock<Option<EntityId>>,
}

impl Controller {
    /// Creates new instance of [Controller]
    pub fn new(ecs: Arc<ECS>) -> Arc<Controller> {
        let controller = Controller {
            ecs,
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
}
