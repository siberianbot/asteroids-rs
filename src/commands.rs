use std::sync::{Arc, Mutex, mpsc};

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum Command {
    Exit,
}

pub struct Sender {
    tx: mpsc::Sender<Command>,
}

impl Sender {
    pub fn send(&self, command: Command) {
        self.tx.send(command).expect("command wasn't sent");
    }
}

pub struct Handler(Box<dyn Fn(&Command)>);

impl<H> From<H> for Handler
where
    H: Fn(&Command) + 'static,
{
    fn from(handler: H) -> Self {
        let handler = Box::new(handler);

        Handler(handler)
    }
}

pub struct Manager {
    handlers: Mutex<Vec<Handler>>,
    tx: mpsc::Sender<Command>,
    rx: mpsc::Receiver<Command>,
}

impl Manager {
    pub fn new() -> Arc<Manager> {
        let (tx, rx) = mpsc::channel();

        let manager = Manager {
            handlers: Mutex::default(),
            tx,
            rx,
        };

        Arc::new(manager)
    }

    pub fn create_sender(&self) -> Sender {
        Sender {
            tx: self.tx.clone(),
        }
    }

    pub fn add_handler<H>(&self, handler: H)
    where
        H: Into<Handler>,
    {
        let handler = handler.into();

        let mut handlers = self.handlers.lock().unwrap();
        handlers.push(handler);
    }

    pub fn dispatch(&self) {
        let commands: Vec<_> = self.rx.try_iter().collect();

        let handlers = self.handlers.lock().unwrap();
        for command in commands {
            for Handler(handler) in handlers.iter() {
                handler(&command);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::Command;

    use super::Manager;

    #[test]
    fn commands_test() {
        const COMMAND: Command = Command::Exit;

        let manager = Manager::new();

        manager.add_handler(|command: &Command| {
            assert!(matches!(command, &COMMAND));
        });

        let sender = manager.create_sender();
        sender.send(COMMAND);

        manager.dispatch();
    }
}
