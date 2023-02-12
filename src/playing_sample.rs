use crate::loaded_sample::LoadedSample;

pub struct PlayingSample {
    sample: LoadedSample,
    current_sample_index: usize,
}

impl PlayingSample {
    pub fn new(sample: LoadedSample) -> Self {
        Self {
            sample,
            current_sample_index: 0,
        }
    }

    pub fn get_next_sample(&mut self) -> f32 {
        let sample = self.sample.data[self.current_sample_index];
        self.current_sample_index += 1;
        sample
    }

    pub fn should_be_removed(&self) -> bool {
        self.current_sample_index >= self.sample.data.len()
    }
}

