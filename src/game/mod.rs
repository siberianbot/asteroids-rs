use std::{
    collections::{BTreeMap, BTreeSet},
    f32::consts::PI,
    ops::{Div, Mul, RangeInclusive},
    sync::{Arc, Mutex, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};

use entities::{Entities, UpdateContext};
use glam::Vec2;

use crate::{
    dispatch::{Command, Dispatcher, Event, Sender},
    entity::{Asteroid, Camera, CameraComponent, Entity, Spacecraft, TransformComponent},
    game::entities::{
        CAMERA_DISTANCE_MULTIPLIER, CAMERA_MAX_DISTANCE, CAMERA_MIN_DISTANCE, EntityId,
        PlayerAction,
    },
    worker::Worker,
};

pub mod entities;

const MAX_DISTANCE: f32 = 100.0;
const SAFE_DISTANCE: RangeInclusive<f32> = 15.0..=MAX_DISTANCE;
const FIRE_COOLDOWN: f32 = 0.5;
const BULLET_VELOCITY: f32 = 12.5;

pub struct State {
    pub spacecraft_id: EntityId,
    pub camera_id: EntityId,

    command_sender: Sender<Command>,
}

struct AsteroidRespawnState {
    timer: f32,
}

pub struct Game {
    entities: Arc<Entities<State>>,
    state: State,
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
                Self::spacecraft_action_handle,
                Self::spacecraft_fire_cooldown,
                Self::entities_despawn,
            ],
        );

        let spacecraft_entity_id = entities.create(Spacecraft::default());
        let camera_entity_id = entities.create(Camera {
            camera: CameraComponent {
                target: Some(spacecraft_entity_id),
                ..Default::default()
            },
            ..Default::default()
        });

        let game = Game {
            entities,
            state: State {
                spacecraft_id: spacecraft_entity_id,
                camera_id: camera_entity_id,
                command_sender: command_dispatcher.create_sender(),
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

        {
            let game = game.clone();

            event_dispatcher.add_handler(move |event| {
                game.handle_event(event);
            });
        }

        (game.clone(), worker)
    }

    pub fn entities(&self) -> Arc<Entities<State>> {
        self.entities.clone()
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    fn update(&self, delta: f32) {
        self.entities.update(delta, &self.state);

        self.respawn_asteroids(delta);
    }

    fn respawn_asteroids(&self, delta: f32) {
        let mut state = self.asteroid_respawn_state.lock().unwrap();

        state.timer -= delta;

        if state.timer > 0.0 {
            return;
        }

        let count = self
            .entities
            .iter()
            .filter_map(|(_, entity)| entity.asteroid())
            .count();

        if count >= 16 {
            return;
        }

        let distance = rand::random_range(SAFE_DISTANCE);
        let rotation = rand::random_range(0.0..=2.0 * PI);

        let position = self
            .entities
            .visit(self.state.spacecraft_id, |entity| {
                entity.transform().position + distance * Vec2::ONE.rotate(rotation.sin_cos().into())
            })
            .expect("there is no player entity");

        let asteroid = Asteroid {
            transform: TransformComponent {
                position,
                ..Default::default()
            },
            ..Asteroid::default()
        };

        self.entities.create(asteroid);

        state.timer = 1.0;
    }

    fn handle_command(&self, command: &Command) {
        match command {
            // Command::PlayerActionDown(action) => self
            //     .entities
            //     .visit_mut(self.state.spacecraft_id, |entity| {
            //         entity.to_spacecraft_mut().action |= *action;
            //     })
            //     .expect("there is not player entity"),

            // Command::PlayerActionUp(action) => self
            //     .entities
            //     .visit_mut(self.state.spacecraft_id, |entity| {
            //         entity.to_spacecraft_mut().action &= !*action;
            //     })
            //     .expect("there is not player entity"),

            // Command::PlayerFire => {
            //     let (position, rotation) = self
            //         .entities
            //         .visit_mut(self.state.spacecraft_id, |entity| {
            //             let spacecraft = entity.to_spacecraft_mut();
            //             spacecraft.fire_cooldown = FIRE_COOLDOWN;

            //             (spacecraft.position, spacecraft.rotation)
            //         })
            //         .expect("there is not player entity");

            //     let bullet = Bullet {
            //         position,
            //         velocity: BULLET_VELOCITY
            //             * Vec2::new(1.0, 0.0).rotate(rotation.sin_cos().into()),
            //         owner_id: self.state.spacecraft_id,

            //         ..Default::default()
            //     };

            //     self.entities.create(bullet);
            // }
            Command::ToggleCameraFollow => self
                .entities
                .visit_mut(self.state.camera_id, |entity| {
                    let camera = entity.camera_mut().unwrap();

                    camera.follow = !camera.follow;
                })
                .expect("there is not camera entity"),

            Command::CameraZoomIn => self
                .entities
                .visit_mut(self.state.camera_id, |entity| {
                    let camera = entity.camera_mut().unwrap();

                    camera.distance = camera
                        .distance
                        .div(CAMERA_DISTANCE_MULTIPLIER)
                        .clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE);
                })
                .expect("there is not camera entity"),

            Command::CameraZoomOut => self
                .entities
                .visit_mut(self.state.camera_id, |entity| {
                    let camera = entity.camera_mut().unwrap();

                    camera.distance = camera
                        .distance
                        .mul(CAMERA_DISTANCE_MULTIPLIER)
                        .clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE);
                })
                .expect("there is not camera entity"),

            _ => {}
        }
    }

    fn handle_event(&self, event: &Event) {
        #[derive(PartialEq, Eq, PartialOrd, Ord)]
        enum Collider {
            Asteroid,
            Bullet,
        }

        match event {
            Event::CollisionStarted(collision) => {
                let colliders = collision
                    .iter()
                    .filter_map(|entity_id| {
                        self.entities
                            .visit(*entity_id, |entity| match entity {
                                Entity::Asteroid(_) => Some(Collider::Asteroid),
                                Entity::Bullet(_) => Some(Collider::Bullet),
                                _ => None,
                            })
                            .flatten()
                    })
                    .collect::<BTreeSet<_>>();

                if colliders.contains(&Collider::Asteroid) && colliders.contains(&Collider::Bullet)
                {
                    for entity_id in collision {
                        self.entities.destroy(*entity_id);
                    }
                }
            }

            _ => {}
        }
    }

    fn camera_sync(context: UpdateContext<State>) {
        let position = context
            .current_entity()
            .camera()
            .filter(|camera| camera.follow)
            .and_then(|camera| {
                context
                    .get_entity(camera.target.unwrap())
                    .and_then(|target| match target {
                        Entity::Spacecraft(spacecraft) => Some(spacecraft.transform.position),
                        Entity::Asteroid(asteroid) => Some(asteroid.transform.position),

                        _ => None,
                    })
            });

        if let Some(position) = position {
            context.modify(|entity| {
                entity.transform_mut().position = position;
            });
        }
    }

    fn camera_zoom(context: UpdateContext<State>) {
        const ZOOM_EPSILON: f32 = 0.1;
        const ZOOM_SPEED: f32 = 2.0;

        let distance = context
            .current_entity()
            .camera()
            .map(|camera| camera.distance);

        if let Some(distance) = distance {
            context.modify(|entity| {
                entity.camera_mut().unwrap().distance = distance;
            });
        }
    }

    fn entities_movement(context: UpdateContext<State>) {
        const BREAKING_MULTIPLIER: f32 = 0.5;
        const BREAKING_EPSILON: f32 = 0.01;

        match context.current_entity() {
            Entity::Asteroid(asteroid) => {
                let position =
                    asteroid.transform.position + context.delta() * asteroid.movement.velocity;
                let rotation = asteroid.transform.rotation
                    + context.delta() * asteroid.asteroid.rotation_velocity;

                context.modify(|entity| {
                    let asteroid = entity.transform_mut();

                    asteroid.position = position;
                    asteroid.rotation = rotation;
                });
            }

            Entity::Spacecraft(spacecraft) => {
                let acceleration = if spacecraft.movement.acceleration.length() < BREAKING_EPSILON {
                    -1.0 * spacecraft.movement.velocity * BREAKING_MULTIPLIER
                } else {
                    spacecraft.movement.acceleration
                };

                let velocity = spacecraft.movement.velocity + context.delta() * acceleration;
                let position =
                    spacecraft.transform.position + context.delta() * spacecraft.movement.velocity;

                context.modify(|entity| {
                    entity.movement_mut().unwrap().velocity = velocity;
                    entity.transform_mut().position = position;
                });
            }

            Entity::Bullet(bullet) => {
                let position =
                    bullet.transform.position + context.delta() * bullet.movement.velocity;

                context.modify(|entity| {
                    let bullet = entity.transform_mut();

                    bullet.position = position;
                });
            }

            _ => {}
        }
    }

    fn spacecraft_action_handle(context: UpdateContext<State>) {
        const VEC: Vec2 = Vec2::new(1.0, 0.0);
        const ACCELERATION: f32 = 2.0;
        const DECELERATION: f32 = -1.0;
        const ROTATION_VELOCITY: f32 = PI;

        struct Changes {
            acceleration: Vec2,
            rotation: f32,
        }

        // let changes = context.current_entity().as_spacecraft().map(|spacecraft| {
        //     let mut changes = Changes {
        //         acceleration: Vec2::ZERO,
        //         rotation: spacecraft.rotation,
        //     };

        //     let acceleration_vec = VEC.rotate(spacecraft.rotation.sin_cos().into());

        //     if spacecraft.action.contains(PlayerAction::ACCELERATE) {
        //         changes.acceleration += ACCELERATION * acceleration_vec;
        //     }

        //     if spacecraft.action.contains(PlayerAction::DECELERATE) {
        //         changes.acceleration += DECELERATION * acceleration_vec;
        //     }

        //     if spacecraft.action.contains(PlayerAction::INCLINE_LEFT) {
        //         changes.rotation += context.delta() * ROTATION_VELOCITY;
        //     }

        //     if spacecraft.action.contains(PlayerAction::INCLINE_RIGHT) {
        //         changes.rotation -= context.delta() * ROTATION_VELOCITY;
        //     }

        //     if spacecraft.action.contains(PlayerAction::FIRE) && spacecraft.fire_cooldown == 0.0 {
        //         context.data().command_sender.send(Command::PlayerFire);
        //     }

        //     // TODO: map rotation to [0; 2pi]

        //     changes
        // });

        // if let Some(changes) = changes {
        //     context.modify(|entity| {
        //         let spacecraft = entity.to_spacecraft_mut();

        //         spacecraft.acceleration = changes.acceleration;
        //         spacecraft.rotation = changes.rotation;
        //     });
        // }
    }

    fn spacecraft_fire_cooldown(context: UpdateContext<State>) {
        let fire_cooldown = context.current_entity().spacecraft().map(|spacecraft| {
            if spacecraft.cooldown <= 0.0 {
                0.0
            } else {
                spacecraft.cooldown - context.delta()
            }
        });

        if let Some(fire_cooldown) = fire_cooldown {
            context.modify(move |entity| entity.spacecraft_mut().unwrap().cooldown = fire_cooldown);
        }
    }

    fn entities_despawn(context: UpdateContext<State>) {
        let player_position = context
            .get_entity(context.data().spacecraft_id)
            .map(|spacecraft| spacecraft.transform().position)
            .expect("there is no player entity");

        let entity_position = match context.current_entity() {
            Entity::Spacecraft(spacecraft) => Some(spacecraft.transform.position),
            Entity::Asteroid(asteroid) => Some(asteroid.transform.position),
            Entity::Bullet(bullet) => Some(bullet.transform.position),

            _ => None,
        };

        if let Some(asteroid_position) = entity_position {
            let distance = player_position.distance(asteroid_position);

            if distance >= MAX_DISTANCE {
                context.destroy();
            }
        }
    }
}
