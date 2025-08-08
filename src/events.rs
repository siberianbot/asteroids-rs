use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
};

use crate::{game::entities::EntityId, handle, workers};

/// Enumeration of possible events
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// Entity was created
    EntityCreated(EntityId),

    /// Entity was destroyed
    EntityDestroyed(EntityId),
}

/// Event sender
#[derive(Clone)]
pub struct Sender {
    tx: mpsc::Sender<Event>,
}

impl Sender {
    /// Sends event
    pub fn send(&self, event: Event) {
        if let Err(_) = self.tx.send(event) {
            // TODO: notify
        }
    }
}

/// INTERNAL: event handler
struct Handler(Box<dyn Fn(&Event)>);

unsafe impl Send for Handler {}

/// Events infrastructure
pub struct Events {
    handler_counter: AtomicUsize,
    handlers: Arc<Mutex<BTreeMap<usize, Handler>>>,
    tx: mpsc::Sender<Event>,
    rx: Mutex<mpsc::Receiver<Event>>,
}

impl Events {
    /// Gets instance of [Sender]
    pub fn get_sender(&self) -> Sender {
        Sender {
            tx: self.tx.clone(),
        }
    }

    /// Adds handler delegate
    #[must_use = "returned handle removes handler delegate on drop"]
    pub fn add_handler<F>(&self, delegate: F) -> handle::Handle
    where
        F: Fn(&Event) + 'static,
    {
        let handler_id = self.handler_counter.fetch_add(1, Ordering::Relaxed);

        let mut handlers = self.handlers.lock().unwrap();
        handlers.insert(handler_id, Handler(Box::new(delegate)));

        let handlers = self.handlers.clone();
        let drop = move || {
            handlers.lock().unwrap().remove(&handler_id);
        };

        drop.into()
    }
}

impl Default for Events {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            handler_counter: Default::default(),
            handlers: Default::default(),
            tx,
            rx: Mutex::new(rx),
        }
    }
}

/// INTERNAL: events worker thread function
fn worker_func(events: &Events) {
    let rx = events.rx.lock().unwrap();
    let messages: Vec<_> = rx.try_iter().collect();

    let handlers = events.handlers.lock().unwrap();
    for message in messages {
        for (_, Handler(delegate)) in handlers.iter() {
            delegate(&message);
        }
    }
}

/// Spawns events worker thread
pub fn spawn_worker(workers: &workers::Workers, events: Arc<Events>) -> handle::Handle {
    workers.spawn("Events", move |token| {
        while !token.is_cancelled() {
            worker_func(&events);
        }
    })
}
