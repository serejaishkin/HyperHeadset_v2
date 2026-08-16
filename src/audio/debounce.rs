use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use std::thread;

#[derive(Clone)]
pub struct DebouncedEQ {
    last_change: Arc<Mutex<Instant>>,
    pending_bands: Arc<Mutex<Option<[f32; 10]>>>,
    delay_ms: u64,
}

impl DebouncedEQ {
    pub fn new(delay_ms: u64) -> Self {
        Self {
            last_change: Arc::new(Mutex::new(Instant::now())),
            pending_bands: Arc::new(Mutex::new(None)),
            delay_ms,
        }
    }

    pub fn schedule(&self, bands: [f32; 10]) {
        *self.pending_bands.lock() = Some(bands);
        *self.last_change.lock() = Instant::now();
    }

    pub fn spawn_worker<F>(self, apply_fn: F)
    where F: Fn([f32; 10]) + Send + 'static {
        let last_change = self.last_change.clone();
        let pending = self.pending_bands.clone();
        let delay = Duration::from_millis(self.delay_ms);
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(50));
                let elapsed = last_change.lock().elapsed();
                if elapsed >= delay {
                    if let Some(bands) = pending.lock().take() {
                        apply_fn(bands);
                    }
                }
            }
        });
    }
}