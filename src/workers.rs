use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use crate::handle;

/// A cancellation token
#[derive(Clone, Default)]
pub struct CancellationToken {
    token: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Checks if token is cancelled
    pub fn is_cancelled(&self) -> bool {
        self.token.load(Ordering::Relaxed)
    }

    /// INTERNAL: cancels token
    fn cancel(&self) {
        let _ = self
            .token
            .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed);
    }
}

/// INTERNAL: worker thread with cancellation token
struct Worker {
    token: CancellationToken,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    /// INTERNAL: spawns a worker
    fn spawn<F>(name: String, func: F) -> Worker
    where
        F: FnOnce(CancellationToken) + Send + 'static,
    {
        let token: CancellationToken = Default::default();

        let handle = {
            let token = token.clone();

            thread::Builder::new()
                .name(name)
                .spawn(move || func(token))
                .expect("failed to spawn worker thread")
        };

        let worker = Worker {
            token,
            handle: Some(handle),
        };

        worker
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.token.cancel();

        if let Some(handle) = self.handle.take() {
            if let Err(_) = handle.join() {
                // TODO: notify
            }
        }
    }
}

/// Workers infrastructure
#[derive(Default)]
pub struct Workers {
    workers: Arc<Mutex<BTreeMap<String, Worker>>>,
}

impl Workers {
    /// Spawns a worker and returns its handle
    #[must_use = "returned handle removes worker on drop"]
    pub fn spawn<S, F>(&self, name: S, func: F) -> handle::Handle
    where
        S: Into<String>,
        F: FnOnce(CancellationToken) + Send + 'static,
    {
        let mut workers = self.workers.lock().unwrap();

        let name = name.into();
        let worker = Worker::spawn(name.clone(), func);

        workers.insert(name.clone(), worker);

        let workers = self.workers.clone();
        let drop = move || {
            let mut workers = workers.lock().unwrap();
            workers.remove(&name);
        };

        drop.into()
    }
}
