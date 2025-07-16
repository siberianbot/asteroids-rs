use std::{
    collections::{BTreeMap, BTreeSet},
    f32::consts::PI,
    ops::{Div, Mul, RangeInclusive},
    sync::{Arc, Mutex, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};

use glam::Vec2;

use crate::{
    dispatch::{Command, Dispatcher, Event, Sender},
    ecs::{self, ECS, StatelessSystem},
    entity::{Asteroid, Camera, CameraComponent, Entity, EntityId, Spacecraft, TransformComponent},
    game::entities::{
        CAMERA_DISTANCE_MULTIPLIER, CAMERA_MAX_DISTANCE, CAMERA_MIN_DISTANCE, PlayerAction,
    },
    systems,
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
    ecs: Arc<ECS>,
    _ecs_worker: Worker,
    state: State,
    asteroid_respawn_state: Mutex<AsteroidRespawnState>,
}

impl Game {
    pub fn new(
        command_dispatcher: &Dispatcher<Command>,
        event_dispatcher: &Dispatcher<Event>,
    ) -> (Arc<Game>, Worker) {
        let ecs = ECS::new(event_dispatcher);

        ecs.add_system(
            "camera_sync_system",
            Into::<StatelessSystem>::into(systems::camera_sync_system),
        );

        ecs.add_system(
            "movement_system",
            Into::<StatelessSystem>::into(systems::movement_system),
        );

        ecs.add_system(
            "spacecraft_cooldown_system",
            Into::<StatelessSystem>::into(systems::spacecraft_cooldown_system),
        );

        let spacecraft_entity_id = ecs.create_entity(Spacecraft::default());
        let camera_entity_id = ecs.create_entity(Camera {
            camera: CameraComponent {
                target: Some(spacecraft_entity_id),
                ..Default::default()
            },
            ..Default::default()
        });

        let game = Game {
            _ecs_worker: ecs::spawn_worker(ecs.clone()),
            ecs,
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

    pub fn ecs(&self) -> Arc<ECS> {
        self.ecs.clone()
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    fn update(&self, delta: f32) {
        self.respawn_asteroids(delta);
    }

    fn respawn_asteroids(&self, delta: f32) {
        let mut state = self.asteroid_respawn_state.lock().unwrap();

        state.timer -= delta;

        if state.timer > 0.0 {
            return;
        }

        let count = self
            .ecs
            .iter_entities()
            .filter_map(|(_, entity)| entity.asteroid())
            .count();

        if count >= 16 {
            return;
        }

        let distance = rand::random_range(SAFE_DISTANCE);
        let rotation = rand::random_range(0.0..=2.0 * PI);

        let position = self
            .ecs
            .visit_entity(self.state.spacecraft_id, |entity| {
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

        self.ecs.create_entity(asteroid);

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
            // Command::ToggleCameraFollow => self
            //     .entities
            //     .visit_mut(self.state.camera_id, |entity| {
            //         let camera = entity.camera_mut().unwrap();

            //         camera.follow = !camera.follow;
            //     })
            //     .expect("there is not camera entity"),

            // Command::CameraZoomIn => self
            //     .entities
            //     .visit_mut(self.state.camera_id, |entity| {
            //         let camera = entity.camera_mut().unwrap();

            //         camera.distance = camera
            //             .distance
            //             .div(CAMERA_DISTANCE_MULTIPLIER)
            //             .clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE);
            //     })
            //     .expect("there is not camera entity"),

            // Command::CameraZoomOut => self
            //     .entities
            //     .visit_mut(self.state.camera_id, |entity| {
            //         let camera = entity.camera_mut().unwrap();

            //         camera.distance = camera
            //             .distance
            //             .mul(CAMERA_DISTANCE_MULTIPLIER)
            //             .clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE);
            //     })
            //     .expect("there is not camera entity"),
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
                        self.ecs
                            .visit_entity(*entity_id, |entity| match entity {
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
                        self.ecs.destroy_entity(*entity_id);
                    }
                }
            }

            _ => {}
        }
    }

    // fn entities_despawn(context: UpdateContext<State>) {
    //     let player_position = context
    //         .get_entity(context.data().spacecraft_id)
    //         .map(|spacecraft| spacecraft.transform().position)
    //         .expect("there is no player entity");

    //     let entity_position = match context.current_entity() {
    //         Entity::Spacecraft(spacecraft) => Some(spacecraft.transform.position),
    //         Entity::Asteroid(asteroid) => Some(asteroid.transform.position),
    //         Entity::Bullet(bullet) => Some(bullet.transform.position),

    //         _ => None,
    //     };

    //     if let Some(asteroid_position) = entity_position {
    //         let distance = player_position.distance(asteroid_position);

    //         if distance >= MAX_DISTANCE {
    //             context.destroy();
    //         }
    //     }
    // }
}
