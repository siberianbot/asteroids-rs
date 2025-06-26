use std::{
    f32::consts::PI,
    ops::{Div, Mul, RangeInclusive},
    sync::{Arc, Mutex, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};

use entities::{Entities, Entity, UpdateContext};
use glam::Vec2;

use crate::{
    dispatch::{Command, Dispatcher, Event},
    game::entities::{
        Asteroid, CAMERA_DISTANCE_MULTIPLIER, CAMERA_MAX_DISTANCE, CAMERA_MIN_DISTANCE, Camera,
        EntityId, PlayerMovement, Spacecraft,
    },
    worker::Worker,
};

pub mod entities;

pub struct PlayerState {
    pub spacecraft_id: EntityId,
    pub camera_id: EntityId,
}

struct AsteroidRespawnState {
    timer: f32,
}

pub struct Game {
    entities: Arc<Entities<PlayerState>>,
    player_state: PlayerState,
    asteroid_respawn_state: Mutex<AsteroidRespawnState>,
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
                Self::asteroids_despawn,
            ],
        );

        let spacecraft_entity_id = entities.create(Spacecraft::default());
        let camera_entity_id = entities.create(Camera {
            target: spacecraft_entity_id,
            ..Default::default()
        });

        let game = Game {
            entities,
            player_state: PlayerState {
                spacecraft_id: spacecraft_entity_id,
                camera_id: camera_entity_id,
            },
            asteroid_respawn_state: Mutex::new(AsteroidRespawnState { timer: 0.0 }),
        };

        let game = Arc::new(game);

        let worker = {
            let game = game.clone();

            Worker::spawn("Game", move |alive| {
                const RATE: f32 = 1.0 / 120.0;

                let mut last_update = Instant::now();

                while alive.load(Ordering::Relaxed) {
                    let delta = Instant::now().duration_since(last_update).as_secs_f32();

                    game.update(delta);

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

    pub fn entities(&self) -> Arc<Entities<PlayerState>> {
        self.entities.clone()
    }

    pub fn player_state(&self) -> &PlayerState {
        &self.player_state
    }

    fn update(&self, delta: f32) {
        self.entities.update(delta, &self.player_state);

        self.respawn_asteroids(delta);
    }

    fn respawn_asteroids(&self, delta: f32) {
        const SAFE_DISTANCE: RangeInclusive<f32> = 10.0..=20.0;

        let mut state = self.asteroid_respawn_state.lock().unwrap();

        state.timer -= delta;

        if state.timer > 0.0 {
            return;
        }

        let count = self
            .entities
            .iter()
            .filter_map(|(_, entity)| entity.as_asteroid())
            .count();

        if count >= 16 {
            return;
        }

        let distance = rand::random_range(SAFE_DISTANCE);
        let rotation = rand::random_range(0.0..=2.0 * PI);

        let position = self
            .entities
            .visit(self.player_state.spacecraft_id, |entity| {
                entity.as_spacecraft().map(|spacecraft| {
                    spacecraft.position + distance * Vec2::ONE.rotate(rotation.sin_cos().into())
                })
            })
            .flatten()
            .expect("there is no player entity");

        let asteroid = Asteroid {
            position,
            ..Asteroid::default()
        };

        self.entities.create(asteroid);

        state.timer = 1.0;
    }

    fn handle_command(&self, command: &Command) {
        match command {
            Command::PlayerMovementDown(movement) => self
                .entities
                .visit_mut(self.player_state.spacecraft_id, |entity| {
                    entity.to_spacecraft_mut().movement |= *movement;
                })
                .expect("there is not player entity"),

            Command::PlayerMovementUp(movement) => self
                .entities
                .visit_mut(self.player_state.spacecraft_id, |entity| {
                    entity.to_spacecraft_mut().movement &= !*movement;
                })
                .expect("there is not player entity"),

            Command::ToggleCameraFollow => self
                .entities
                .visit_mut(self.player_state.camera_id, |entity| {
                    let camera = entity.to_camera_mut();

                    camera.follow = !camera.follow;
                })
                .expect("there is not camera entity"),

            Command::CameraZoomIn => self
                .entities
                .visit_mut(self.player_state.camera_id, |entity| {
                    let camera = entity.to_camera_mut();

                    camera.target_distance = camera
                        .target_distance
                        .div(CAMERA_DISTANCE_MULTIPLIER)
                        .clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE);
                })
                .expect("there is not camera entity"),

            Command::CameraZoomOut => self
                .entities
                .visit_mut(self.player_state.camera_id, |entity| {
                    let camera = entity.to_camera_mut();

                    camera.target_distance = camera
                        .target_distance
                        .mul(CAMERA_DISTANCE_MULTIPLIER)
                        .clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE);
                })
                .expect("there is not camera entity"),

            _ => {}
        }
    }

    fn camera_sync(context: UpdateContext<PlayerState>) {
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

    fn camera_zoom(context: UpdateContext<PlayerState>) {
        const ZOOM_EPSILON: f32 = 0.1;
        const ZOOM_SPEED: f32 = 2.0;

        let distance = context.current_entity().as_camera().map(|camera| {
            let diff = camera.target_distance - camera.distance;

            if diff.abs() < ZOOM_EPSILON {
                return camera.target_distance;
            }

            camera.distance + context.delta() * ZOOM_SPEED * diff
        });

        if let Some(distance) = distance {
            context.modify(|entity| {
                entity.to_camera_mut().distance = distance;
            });
        }
    }

    fn entities_movement(context: UpdateContext<PlayerState>) {
        const BREAKING_MULTIPLIER: f32 = 0.5;
        const BREAKING_EPSILON: f32 = 0.01;

        match context.current_entity() {
            Entity::Asteroid(asteroid) => {
                let position = asteroid.position + context.delta() * asteroid.velocity;
                let rotation = asteroid.rotation + context.delta() * asteroid.rotation_velocity;

                context.modify(|entity| {
                    let asteroid = entity.to_asteroid_mut();

                    asteroid.position = position;
                    asteroid.rotation = rotation;
                });
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

    fn spacecraft_movement_handle(context: UpdateContext<PlayerState>) {
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

    fn asteroids_despawn(context: UpdateContext<PlayerState>) {
        let player_position = context
            .get_entity(context.data().spacecraft_id)
            .and_then(|entity| entity.as_spacecraft())
            .map(|spacecraft| spacecraft.position)
            .expect("there is no player entity");

        let asteroid_position = context
            .current_entity()
            .as_asteroid()
            .map(|asteroid| asteroid.position);

        if let Some(asteroid_position) = asteroid_position {
            let distance = player_position.distance(asteroid_position);

            if distance >= 25.0 {
                context.destroy();
            }
        }
    }
}
