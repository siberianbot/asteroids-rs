use std::{
    sync::{Arc, atomic::Ordering},
    time::Instant,
};

use entities::{Entities, Entity, UpdateContext};

use crate::{
    dispatch::{Command, Dispatcher, Event},
    worker::Worker,
};

pub mod entities;

pub struct Game {
    entities: Arc<Entities>,
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

        let game = Game { entities, worker };

        Arc::new(game)
    }

    fn camera_sync(context: UpdateContext) {
        if let Entity::Camera(camera) = context.current_entity() {
            let target = context
                .get_entity(camera.target)
                .expect("unknown camera target");

            let position = match target {
                Entity::Spacecraft(spacecraft) => spacecraft.position,
                Entity::Asteroid(asteroid) => asteroid.position,

                _ => panic!("unexpected camera target"),
            };

            context.modify(|entity| {
                entity.to_camera_mut().position = position;
            });
        }
    }
}
