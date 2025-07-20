use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex, mpsc},
};

use crate::game_entity::EntityId;

#[derive(Clone)]
pub struct Sender<T> {
    tx: mpsc::Sender<T>,
}

impl<T> Sender<T> {
    pub fn send(&self, message: T) {
        self.tx.send(message).expect("message wasn't sent");
    }
}

struct Handler<T>(Box<dyn Fn(&T)>);

unsafe impl<T> Send for Handler<T> {}

impl<T, H> From<H> for Handler<T>
where
    H: Fn(&T) + 'static,
{
    fn from(handler: H) -> Self {
        let handler = Box::new(handler);

        Handler(handler)
    }
}

pub struct Dispatcher<T> {
    handlers: Mutex<Vec<Handler<T>>>,
    tx: mpsc::Sender<T>,
    rx: Mutex<mpsc::Receiver<T>>,
}

impl<T> Dispatcher<T> {
    pub fn new() -> Arc<Dispatcher<T>> {
        let (tx, rx) = mpsc::channel();

        let manager = Dispatcher {
            handlers: Mutex::default(),
            tx,
            rx: Mutex::new(rx),
        };

        Arc::new(manager)
    }

    pub fn create_sender(&self) -> Sender<T> {
        Sender {
            tx: self.tx.clone(),
        }
    }

    pub fn add_handler<H>(&self, handler: H)
    where
        H: Fn(&T) + 'static,
    {
        let handler = handler.into();

        let mut handlers = self.handlers.lock().unwrap();
        handlers.push(handler);
    }

    pub fn dispatch(&self) {
        let rx = self.rx.lock().unwrap();
        let messages: Vec<_> = rx.try_iter().collect();

        let handlers = self.handlers.lock().unwrap();
        for message in messages {
            for Handler(handler) in handlers.iter() {
                handler(&message);
            }
        }
    }
}

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum Command {
    Exit,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Event {
    WindowResized([u32; 2]),
    EntityCreated(EntityId),
    EntityDestroyed(EntityId),
    CollisionOccurred([EntityId; 2]), // TODO: use ECS/game logics for collision handling
}

#[cfg(test)]
mod tests {
    use super::Command;
    use super::Dispatcher;

    #[test]
    fn commands_test() {
        const COMMAND: Command = Command::Exit;

        let dispatcher = Dispatcher::new();

        dispatcher.add_handler(|command: &Command| {
            assert!(matches!(command, &COMMAND));
        });

        let sender = dispatcher.create_sender();
        sender.send(COMMAND);

        dispatcher.dispatch();
    }
}
