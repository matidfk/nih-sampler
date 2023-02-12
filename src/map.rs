use iced_baseview::{
    alignment::Horizontal,
    widget::{Button, Column, Text},
    Length,
};
use nih_plug::prelude::Params;
use rand::seq::SliceRandom;
use serde::{Serialize, Deserialize};

use crate::{
    layers::{NoteLayer, VelocityLayer}, loaded_sample::LoadedSample,
};

#[derive(Debug, Params, Serialize, Deserialize)]
pub struct Map<T: 'static + Send + Sync> {
    pub vec: Vec<T>,
    selected: Option<usize>,
}

impl<T: 'static + Send + Sync> Map<T> {
    pub fn new() -> Self {
        Self {
            vec: vec![],
            selected: None,
        }
    }
    pub fn is_selected(&self, index: usize) -> bool {
        match self.selected {
            Some(selected) => index == selected,
            None => false,
        }
    }
    pub fn select(&mut self, index: usize) {
        self.selected = Some(index);
    }
    pub fn remove_selected(&mut self) {
        if let Some(selected) = self.selected {
            self.vec.remove(selected);
            self.selected = None;
        }
        
    }
    pub fn selected(&self) -> Option<&T> {
        match self.selected {
            None => None,
            Some(selected) => Some(&self.vec[selected]),
        }
    }
    pub fn selected_mut(&mut self) -> Option<&mut T> {
        match self.selected {
            None => None,
            Some(selected) => Some(&mut self.vec[selected]),
        }
    }
}

impl Map<NoteLayer> {
    pub fn get_note_layer(&self, note: &u8) -> Option<&NoteLayer> {
        self.vec.iter().find(|e| &e.note == note)
    }
}

impl Map<VelocityLayer> {
    pub fn get_velocity_layer(&self, velocity: &u8) -> Option<&VelocityLayer> {
        if self.vec.len() == 0 {
            return None;
        }

        for layer in &self.vec {
            if &layer.max_velocity < velocity {
                // too small
                continue;
            } else {
                return Some(&layer);
            }
        }
        return None;
    }
}

impl Map<LoadedSample> {
    pub fn get_random_sample(&self) -> Option<&LoadedSample> {
        self.vec.choose(&mut rand::thread_rng())
    }
}
