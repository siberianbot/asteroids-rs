use std::{
    sync::{Arc, atomic::Ordering},
    time::Instant,
};

use entities::{Entities, Entity, UpdateContext};

use crate::{
    dispatch::{Command, Dispatcher, Event},
    game::entities::{Camera, EntityId},
    worker::Worker,
};

pub mod entities;

pub struct Game {
    entities: Arc<Entities>,
    camera_entity_id: EntityId,
    worker: Worker,
}

impl Game {
    pub fn new(
        command_dispatcher: &Dispatcher<Command>,
        event_dispatcher: &Dispatcher<Event>,
    ) -> Arc<Game> {
        let entities = Entities::new(event_dispatcher, [Self::camera_sync]);

        let worker = {
            let entities = entities.clone();

            Worker::spawn("Game", move |alive| {
                let mut last_update = Instant::now();

                while alive.load(Ordering::Relaxed) {
                    let delta = Instant::now().duration_since(last_update).as_secs_f32();
                    entities.update(delta);

                    last_update = Instant::now();
                }
            })
        };

        let game = Game {
            camera_entity_id: entities.create(Camera::default()),
            entities,
            worker,
        };

        Arc::new(game)
    }

    pub fn entities(&self) -> Arc<Entities> {
        self.entities.clone()
    }

    pub fn camera_entity_id(&self) -> EntityId {
        self.camera_entity_id
    }

    fn camera_sync(context: UpdateContext) {
        if let Entity::Camera(camera) = context.current_entity() {
            let position = context
                .get_entity(camera.target)
                .and_then(|target| match target {
                    Entity::Spacecraft(spacecraft) => Some(spacecraft.position),
                    Entity::Asteroid(asteroid) => Some(asteroid.position),

                    _ => None,
                });

            if let Some(position) = position {
                context.modify(|entity| {
                    entity.to_camera_mut().position = position;
                });
            }
        }
    }
}
