use std::sync::atomic::{AtomicUsize, Ordering};

use nih_plug::prelude::AtomicF32;

pub const BUFFER_SIZE: usize = 512;

// works like a ring buffer to avoid having to shift all the contents
pub struct Visualizer {
    pub data: [AtomicF32; BUFFER_SIZE],
    pub current_index: AtomicUsize,
}

impl Visualizer {
    pub fn new() -> Self {
        Self {
            data: std::array::from_fn(|_| AtomicF32::new(0.0)),
            current_index: AtomicUsize::new(0),
        }
    }

    pub fn store(&self, value: f32) {
        self.data[self.current_index.load(Ordering::Relaxed)].store(value, Ordering::Relaxed);
        self.current_index.store(
            (self.current_index.load(Ordering::Relaxed) + 1) % BUFFER_SIZE,
            Ordering::Relaxed,
        );
    }

    pub fn get(&self, index: usize) -> f32 {
        self.data[(self.current_index.load(Ordering::Relaxed) + index) % BUFFER_SIZE]
            .load(Ordering::Relaxed)
    }
}
