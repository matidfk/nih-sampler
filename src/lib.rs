#![allow(unused_imports)]

use iced_baseview::Application;
use iced_editor::IcedEditor;
use layers::NoteLayer;
use loaded_sample::LoadedSample;
use map::Map;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use playing_sample::PlayingSample;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

mod iced_editor;
mod layers;
mod loaded_sample;
mod map;
mod playing_sample;
mod theme;
mod utils;

pub const TEST_SAMPLE: &str = "D:/Audio/UserPlugins/Drums/Restrains Kit Samples/BigKick-001.wav";

/// Main plugin struct
pub struct NihSampler {
    /// Plugin parameters
    params: Arc<NihSamplerParams>,
    /// Currently playing samples
    pub playing_samples: Vec<PlayingSample>,
}

impl Default for NihSampler {
    fn default() -> Self {
        Self {
            params: Arc::new(NihSamplerParams::default()),
            playing_samples: vec![],
        }
    }
}

/// Plugin parameters struct
#[derive(Params)]
pub struct NihSamplerParams {
    // #[persist = "editor-state"]
    // editor_state: Arc<ViziaState>,
    #[persist = "note-map"]
    pub note_map: Arc<Mutex<Map<NoteLayer>>>,
}

impl Default for NihSamplerParams {
    fn default() -> Self {
        Self {
            // editor_state: editor::default_state(),
            note_map: Arc::new(Mutex::new(Map::new())),
        }
    }
}

impl Plugin for NihSampler {
    const NAME: &'static str = "Nih Sampler";
    const VENDOR: &'static str = "matidfk";
    const URL: &'static str = "https://youtu.be/dQw4w9WgXcQ";
    const EMAIL: &'static str = "info@example.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const DEFAULT_INPUT_CHANNELS: u32 = 0;
    const DEFAULT_OUTPUT_CHANNELS: u32 = 2;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let a = IcedEditor::new(Arc::clone(&self.params));

        Some(Box::new(a.0))
    }

    fn accepts_bus_config(&self, config: &BusConfig) -> bool {
        // This can output to any number of channels, but it doesn't take any audio inputs
        config.num_input_channels == 0 && config.num_output_channels > 0
    }

    fn initialize(
        &mut self,
        _bus_config: &BusConfig,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        return true;
        todo!();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();
        // for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
        for channel_samples in buffer.iter_samples() {
            while let Some(event) = next_event {
                // if event.timing() > sample_id as u32 {
                //     break;
                // }
                match event {
                    NoteEvent::NoteOn { note, velocity, .. } => {
                        // convert velocity from f32 to u8
                        let velocity = (velocity * 127.0) as u8;

                        if let Some(note_layer) =
                            self.params.note_map.lock().unwrap().get_note_layer(&note)
                        {
                            if let Some(velocity_layer) =
                                note_layer.velocity_map.get_velocity_layer(&velocity)
                            {
                                if let Some(sample_layer) =
                                    velocity_layer.samples.get_random_sample()
                                {
                                    self.playing_samples
                                        .push(PlayingSample::new(sample_layer.clone()));
                                }
                            }
                        }
                    }
                    _ => (),
                }

                next_event = context.next_event();
            }

            for sample in channel_samples {
                for playing_sample in self.playing_samples.iter_mut() {
                    *sample += playing_sample.get_next_sample();
                }

                self.playing_samples.retain(|e| !e.should_be_removed());
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for NihSampler {
    const CLAP_ID: &'static str = "com.moist-plugins-gmbh.gain-gui-vizia";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A smoothed gain parameter example plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for NihSampler {
    const VST3_CLASS_ID: [u8; 16] = *b"GainGuiVIIIZIAAA";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Drum,
        Vst3SubCategory::Sampler,
        Vst3SubCategory::Instrument,
    ];
}

nih_export_clap!(NihSampler);
nih_export_vst3!(NihSampler);
