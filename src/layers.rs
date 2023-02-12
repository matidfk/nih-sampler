use nih_plug::prelude::Params;
use serde::{Serialize, Deserialize};

use crate::{loaded_sample::LoadedSample, map::Map};
#[derive(Debug, Params, Serialize, Deserialize)]
pub struct NoteLayer {
    pub note: u8,
    pub velocity_map: Map<VelocityLayer>,
}

impl NoteLayer {
    pub fn new(note: u8) -> Self {
        Self {
            note,
            velocity_map: Map::new(),
        }
    }
}

#[derive(Debug, Params, Serialize, Deserialize)]
pub struct VelocityLayer {
    pub max_velocity: u8,
    pub samples: Map<LoadedSample>,
}

impl VelocityLayer {
    pub fn new(max_velocity: u8) -> Self {
        Self {
            max_velocity,
            samples: Map::new(),
        }
    }
}