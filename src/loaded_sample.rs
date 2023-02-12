use std::path::PathBuf;

use nih_plug_vizia::vizia::prelude::Data;
use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize, Debug, Data)]
pub struct LoadedSample {
    pub data: Vec<f32>,
    pub path: PathBuf,
    pub volume: f32,
}

impl LoadedSample {
    pub fn new(path: PathBuf) -> Self {
        Self {
            data: load_wav(&path),
            path,
            volume: 1.0
        }
    }
}
pub fn load_wav(path: &PathBuf) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).unwrap();
    let spec = reader.spec();
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.unwrap_or_default())
            .collect::<Vec<_>>(),

        hound::SampleFormat::Int => reader
            .samples::<i32>()
            .map(|s| s.unwrap_or_default() as f32 * 256.0 / i32::MAX as f32)
            .collect::<Vec<_>>(),
    };

    samples
}
