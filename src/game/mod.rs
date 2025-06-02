use std::sync::{Arc, atomic::Ordering};

use entities::Entities;

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
        let entities = Entities::new(event_dispatcher);
        let worker = {
            let entities = entities.clone();

            Worker::spawn("Game", move |alive| {
                while alive.load(Ordering::Relaxed) {
                    // TODO: implement entities updating
                }
            })
        };

        let game = Game { entities, worker };

        Arc::new(game)
    }
}
