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
    game::entities::{
        CAMERA_DISTANCE_MULTIPLIER, CAMERA_MAX_DISTANCE, CAMERA_MIN_DISTANCE, PlayerAction,
    },
    game_common::{GamePlayer, GamePlayers},
    game_ecs::{self, ECS, StatelessSystem},
    game_entity::{
        Asteroid, Camera, CameraComponent, Entity, EntityId, Spacecraft, TransformComponent,
    },
    game_logics::{AsteroidsRespawnGameLogicState, asteroids_respawn_game_logic},
    game_loop::{self, GameLoop, StatefulGameLogic},
    game_systems,
    worker::Worker,
};

pub mod entities;

const MAX_DISTANCE: f32 = 100.0;
const SAFE_DISTANCE: RangeInclusive<f32> = 15.0..=MAX_DISTANCE;
const FIRE_COOLDOWN: f32 = 0.5;
const BULLET_VELOCITY: f32 = 12.5;

pub struct Game {
    ecs: Arc<ECS>,
    _ecs_worker: Worker,
    game_loop: Arc<GameLoop>,
    _game_loop_worker: Worker,

    camera_id: EntityId,
    game_players: Arc<GamePlayers>,
}

impl Game {
    pub fn new(
        command_dispatcher: &Dispatcher<Command>,
        event_dispatcher: &Dispatcher<Event>,
    ) -> Arc<Game> {
        let ecs = ECS::new(event_dispatcher);

        ecs.add_system(
            "camera_sync_system",
            Into::<StatelessSystem>::into(game_systems::camera_sync_system),
        );

        ecs.add_system(
            "movement_system",
            Into::<StatelessSystem>::into(game_systems::movement_system),
        );

        ecs.add_system(
            "spacecraft_cooldown_system",
            Into::<StatelessSystem>::into(game_systems::spacecraft_cooldown_system),
        );

        let game_loop: Arc<GameLoop> = Default::default();
        let game_players: Arc<GamePlayers> = Default::default();

        game_loop.add_logic(
            "asteroids_respawn_game_logic",
            StatefulGameLogic::new(
                AsteroidsRespawnGameLogicState::new(ecs.clone(), game_players.clone()),
                asteroids_respawn_game_logic,
            ),
        );

        let spacecraft_entity_id = ecs.create_entity(Spacecraft::default());
        let camera_entity_id = ecs.create_entity(Camera {
            camera: CameraComponent {
                target: Some(spacecraft_entity_id),
                ..Default::default()
            },
            ..Default::default()
        });

        game_players.players.write().unwrap().push(GamePlayer {
            spacecraft_id: spacecraft_entity_id,
        });

        let game = Game {
            _ecs_worker: game_ecs::spawn_worker(ecs.clone()),
            ecs,
            _game_loop_worker: game_loop::spawn_worker(game_loop.clone()),
            game_loop,

            camera_id: camera_entity_id,
            game_players,
        };

        let game = Arc::new(game);

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

        game
    }

    pub fn ecs(&self) -> Arc<ECS> {
        self.ecs.clone()
    }

    pub fn camera_id(&self) -> EntityId {
        self.camera_id
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
