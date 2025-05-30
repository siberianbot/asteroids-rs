use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

pub struct Worker {
    alive: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn spawn<S, F>(name: S, func: F) -> Worker
    where
        S: Into<String>,
        F: FnOnce(Arc<AtomicBool>) + Send + 'static,
    {
        let alive = Arc::new(AtomicBool::new(true));
        let handle = {
            let alive = alive.clone();

            thread::Builder::new()
                .name(name.into())
                .spawn(move || func(alive))
                .expect("failed to spawn worker thread")
        };

        let worker = Worker {
            alive,
            handle: Some(handle),
        };

        worker
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Relaxed);

        if let Some(handle) = self.handle.take() {
            handle.join().expect("worker thread failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicBool, Ordering},
    };

    use super::Worker;

    #[test]
    fn worker_test() {
        let value = Arc::new(AtomicBool::new(false));
        let start_barrier = Arc::new(Barrier::new(2));
        let end_barrier = Arc::new(Barrier::new(2));

        let worker = {
            let value = value.clone();
            let start_barrier = start_barrier.clone();
            let end_barrier = end_barrier.clone();

            Worker::spawn("test", move |_| {
                start_barrier.wait();

                value.store(true, Ordering::Relaxed);

                end_barrier.wait();
            })
        };

        start_barrier.wait();
        end_barrier.wait();

        drop(worker);

        let value = value.load(Ordering::Relaxed);
        assert!(value);
    }
}
