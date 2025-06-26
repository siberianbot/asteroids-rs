use std::{
    f32::consts::PI,
    ops::{Div, Mul},
    sync::{Arc, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};

use entities::{Entities, Entity, UpdateContext};
use glam::Vec2;

use crate::{
    dispatch::{Command, Dispatcher, Event},
    game::entities::{Camera, EntityId, PlayerMovement, Spacecraft},
    worker::Worker,
};

pub mod entities;

pub struct Game {
    entities: Arc<Entities>,
    player_entity_id: EntityId,
    camera_entity_id: EntityId,
}

impl Game {
    pub fn new(
        command_dispatcher: &Dispatcher<Command>,
        event_dispatcher: &Dispatcher<Event>,
    ) -> (Arc<Game>, Worker) {
        let entities = Entities::new(
            event_dispatcher,
            [
                Self::camera_sync,
                Self::camera_zoom,
                Self::entities_movement,
                Self::spacecraft_movement_handle,
            ],
        );

        let player_entity_id = entities.create(Spacecraft::default());
        let camera_entity_id = entities.create(Camera {
            target: player_entity_id,
            ..Default::default()
        });

        let game = Game {
            entities,
            player_entity_id,
            camera_entity_id,
        };

        let game = Arc::new(game);

        let worker = {
            let game = game.clone();

            Worker::spawn("Game", move |alive| {
                const RATE: f32 = 1.0 / 120.0;

                let mut last_update = Instant::now();

                while alive.load(Ordering::Relaxed) {
                    let delta = Instant::now().duration_since(last_update).as_secs_f32();

                    game.entities.update(delta);

                    last_update = Instant::now();

                    if delta < RATE {
                        thread::sleep(Duration::from_secs_f32(RATE - delta));
                    }
                }
            })
        };

        {
            let game = game.clone();

            command_dispatcher.add_handler(move |command| {
                game.handle_command(command);
            });
        }

        (game.clone(), worker)
    }

    pub fn entities(&self) -> Arc<Entities> {
        self.entities.clone()
    }

    pub fn camera_entity_id(&self) -> EntityId {
        self.camera_entity_id
    }

    fn handle_command(&self, command: &Command) {
        match command {
            Command::PlayerMovementDown(movement) => self
                .entities
                .visit_mut(self.player_entity_id, |entity| {
                    entity.to_spacecraft_mut().movement |= *movement;
                })
                .expect("there is not player entity"),

            Command::PlayerMovementUp(movement) => self
                .entities
                .visit_mut(self.player_entity_id, |entity| {
                    entity.to_spacecraft_mut().movement &= !*movement;
                })
                .expect("there is not player entity"),

            Command::ToggleCameraFollow => self
                .entities
                .visit_mut(self.camera_entity_id, |entity| {
                    let camera = entity.to_camera_mut();

                    camera.follow = !camera.follow;
                })
                .expect("there is not camera entity"),

            Command::CameraZoomIn => self
                .entities
                .visit_mut(self.camera_entity_id, |entity| {
                    let camera = entity.to_camera_mut();

                    camera.target_distance = camera.target_distance.div(2.0).clamp(1.0, 16.0);
                })
                .expect("there is not camera entity"),

            Command::CameraZoomOut => self
                .entities
                .visit_mut(self.camera_entity_id, |entity| {
                    let camera = entity.to_camera_mut();

                    camera.target_distance = camera.target_distance.mul(2.0).clamp(1.0, 16.0);
                })
                .expect("there is not camera entity"),

            _ => {}
        }
    }

    fn camera_sync(context: UpdateContext) {
        let position = context
            .current_entity()
            .as_camera()
            .filter(|camera| camera.follow)
            .and_then(|camera| {
                context
                    .get_entity(camera.target)
                    .and_then(|target| match target {
                        Entity::Spacecraft(spacecraft) => Some(spacecraft.position),
                        Entity::Asteroid(asteroid) => Some(asteroid.position),

                        _ => None,
                    })
            });

        if let Some(position) = position {
            context.modify(|entity| {
                entity.to_camera_mut().position = position;
            });
        }
    }

    fn camera_zoom(context: UpdateContext) {
        const ZOOM_EPSILON: f32 = 0.1;

        let distance = context.current_entity().as_camera().map(|camera| {
            let diff = camera.target_distance - camera.distance;

            if diff.abs() < ZOOM_EPSILON {
                return camera.target_distance;
            }

            camera.distance + context.delta() * diff
        });

        if let Some(distance) = distance {
            context.modify(|entity| {
                entity.to_camera_mut().distance = distance;
            });
        }
    }

    fn entities_movement(context: UpdateContext) {
        const BREAKING_MULTIPLIER: f32 = 0.5;
        const BREAKING_EPSILON: f32 = 0.01;

        match context.current_entity() {
            Entity::Asteroid(asteroid) => {
                let position = asteroid.position + context.delta() * asteroid.velocity;

                context.modify(|entity| entity.to_asteroid_mut().position = position);
            }

            Entity::Spacecraft(spacecraft) => {
                let acceleration = if spacecraft.acceleration.length() < BREAKING_EPSILON {
                    -1.0 * spacecraft.velocity * BREAKING_MULTIPLIER
                } else {
                    spacecraft.acceleration
                };

                let velocity = spacecraft.velocity + context.delta() * acceleration;
                let position = spacecraft.position + context.delta() * spacecraft.velocity;

                context.modify(|entity| {
                    let spacecraft = entity.to_spacecraft_mut();

                    spacecraft.velocity = velocity;
                    spacecraft.position = position;
                });
            }

            _ => {}
        }
    }

    fn spacecraft_movement_handle(context: UpdateContext) {
        const VEC: Vec2 = Vec2::new(1.0, 0.0);
        const ACCELERATION: f32 = 2.0;
        const DECELERATION: f32 = -1.0;
        const ROTATION_VELOCITY: f32 = PI;

        struct Changes {
            acceleration: Vec2,
            rotation: f32,
        }

        let changes = context.current_entity().as_spacecraft().map(|spacecraft| {
            let mut changes = Changes {
                acceleration: Vec2::ZERO,
                rotation: spacecraft.rotation,
            };

            let acceleration_vec = VEC.rotate(spacecraft.rotation.sin_cos().into());

            if spacecraft.movement.contains(PlayerMovement::ACCELERATE) {
                changes.acceleration += ACCELERATION * acceleration_vec;
            }

            if spacecraft.movement.contains(PlayerMovement::DECELERATE) {
                changes.acceleration += DECELERATION * acceleration_vec;
            }

            if spacecraft.movement.contains(PlayerMovement::INCLINE_LEFT) {
                changes.rotation += context.delta() * ROTATION_VELOCITY;
            }

            if spacecraft.movement.contains(PlayerMovement::INCLINE_RIGHT) {
                changes.rotation -= context.delta() * ROTATION_VELOCITY;
            }

            // TODO: map rotation to [0; 2pi]

            changes
        });

        if let Some(changes) = changes {
            context.modify(|entity| {
                let spacecraft = entity.to_spacecraft_mut();

                spacecraft.acceleration = changes.acceleration;
                spacecraft.rotation = changes.rotation;
            });
        }
    }
}
