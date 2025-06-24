use std::{
    f32::consts::PI,
    sync::{Arc, atomic::Ordering},
    time::Instant,
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
                let mut last_update = Instant::now();

                while alive.load(Ordering::Relaxed) {
                    let delta = Instant::now().duration_since(last_update).as_secs_f32();
                    game.entities.update(delta);

                    last_update = Instant::now();
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
                    entity.to_spacecraft_mut().movement |= *movement
                })
                .expect("there is not player entity"),

            Command::PlayerMovementUp(movement) => self
                .entities
                .visit_mut(self.player_entity_id, |entity| {
                    entity.to_spacecraft_mut().movement &= !*movement
                })
                .expect("there is not player entity"),

            _ => {}
        }
    }

    fn camera_sync(context: UpdateContext) {
        let position = context.current_entity().as_camera().and_then(|camera| {
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

    fn entities_movement(context: UpdateContext) {
        match context.current_entity() {
            Entity::Asteroid(asteroid) => {
                let position = asteroid.position + context.delta() * asteroid.velocity;

                context.modify(|entity| entity.to_asteroid_mut().position = position);
            }

            Entity::Spacecraft(spacecraft) => {
                let velocity = spacecraft.velocity + context.delta() * spacecraft.acceleration;
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
        const ROTATION_VELOCITY: f32 = 8.0 * PI;

        struct Changes {
            acceleration: Vec2,
            rotation: f32,
        }

        let changes = context.current_entity().as_spacecraft().map(|spacecraft| {
            let mut changes = Changes {
                acceleration: spacecraft.acceleration,
                rotation: spacecraft.rotation,
            };

            // TODO: acceleration

            if spacecraft.movement.contains(PlayerMovement::INCLINE_LEFT) {
                changes.rotation -= context.delta() * ROTATION_VELOCITY;

                // if changes.rotation > 2.0 * PI {
                //     changes.rotation -= 2.0 * PI;
                // }
            }

            if spacecraft.movement.contains(PlayerMovement::INCLINE_RIGHT) {
                changes.rotation += context.delta() * ROTATION_VELOCITY;

                // if changes.rotation < 0.0 {
                //     changes.rotation += 2.0 * PI;
                // }
            }

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
