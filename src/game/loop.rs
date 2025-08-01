use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};

use crate::worker::Worker;

/// Trait of a game logic, which handles some high-level game logic
pub trait GameLogic: Send + Sync {
    /// Invoke game logic
    fn invoke(&self, elapsed: f32);
}

/// Stateful [GameLogic] implementation
pub struct StatefulGameLogic<S> {
    state: S,
    delegate: Box<dyn Fn(f32, &S)>,
}

impl<S> StatefulGameLogic<S> {
    /// Creates new instance of [StatefulGameLogic] with some predefines state
    pub fn new<F>(state: S, delegate: F) -> StatefulGameLogic<S>
    where
        F: Fn(f32, &S) + 'static,
    {
        StatefulGameLogic {
            state,
            delegate: Box::new(delegate),
        }
    }
}

impl<S> GameLogic for StatefulGameLogic<S>
where
    S: Send + Sync,
{
    fn invoke(&self, elapsed: f32) {
        (self.delegate)(elapsed, &self.state)
    }
}

unsafe impl<S> Send for StatefulGameLogic<S> where S: Send + Sync {}

unsafe impl<S> Sync for StatefulGameLogic<S> where S: Send + Sync {}

/// A game loop, which stores game logics to be executed
pub struct Loop {
    logics: Mutex<BTreeMap<String, Box<dyn GameLogic>>>,
}

impl Loop {
    /// Adds game logic into the loop
    pub fn add_logic<N, L>(&self, name: N, logic: L)
    where
        N: Into<String>,
        L: GameLogic + 'static,
    {
        let mut logics = self.logics.lock().unwrap();

        logics.insert(name.into(), Box::new(logic));
    }

    /// Removes game logic from the loop
    pub fn remove_logic<N>(&self, name: N)
    where
        N: Into<String>,
    {
        let mut logics = self.logics.lock().unwrap();

        logics.remove(&name.into());
    }
}

impl Default for Loop {
    fn default() -> Self {
        Self {
            logics: Default::default(),
        }
    }
}

/// INTERNAL: Game loop worker thread function
fn worker_func(game_loop: &Loop, elapsed: f32) {
    let logics = game_loop.logics.lock().unwrap();

    for (_, logic) in logics.iter() {
        logic.invoke(elapsed);
    }
}

/// Spawns game loop worker thread
pub fn spawn_worker(r#loop: Arc<Loop>) -> Worker {
    Worker::spawn("GameLoop", move |alive| {
        const UPDATE_RATE: f32 = 1.0 / 120.0;

        let mut last_update = Instant::now();

        while alive.load(Ordering::Relaxed) {
            let elapsed = Instant::now().duration_since(last_update).as_secs_f32();

            worker_func(&r#loop, elapsed);

            last_update = Instant::now();

            if elapsed < UPDATE_RATE {
                let duration = Duration::from_secs_f32(UPDATE_RATE - elapsed);

                thread::sleep(duration);
            }
        }
    })
}
