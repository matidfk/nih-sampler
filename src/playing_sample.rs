use std::path::PathBuf;

pub struct PlayingSample {
    pub handle: PathBuf,
    pub position: usize,
    pub gain: f32,
}

impl PlayingSample {
    pub fn new(handle: PathBuf, gain: f32) -> Self {
        Self {
            handle,
            position: 0,
            gain,
        }
    }
}
