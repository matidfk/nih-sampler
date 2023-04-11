use nih_plug_vizia::ViziaState;
use rand::prelude::*;
use std::{
    cell::RefCell,
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use rtrb;

use nih_plug::prelude::*;
mod editor_vizia;

pub struct LoadedSample(Vec<f32>);

pub struct PlayingSample {
    pub handle: PathBuf,
    pub position: usize,
}

#[derive(Clone)]
pub enum ThreadMessage {
    LoadSample(PathBuf),
    RemoveSample(PathBuf),
}

/// Main plugin struct
pub struct NihSampler {
    pub params: Arc<NihSamplerParams>,
    pub playing_samples: Vec<PlayingSample>,
    pub loaded_samples: HashMap<PathBuf, LoadedSample>,
    pub consumer: RefCell<Option<rtrb::Consumer<ThreadMessage>>>,
}

impl Default for NihSampler {
    fn default() -> Self {
        Self {
            params: Arc::new(Default::default()),
            playing_samples: vec![],
            loaded_samples: HashMap::with_capacity(64),
            consumer: RefCell::new(None),
        }
    }
}

/// Plugin parameters struct
#[derive(Params)]
pub struct NihSamplerParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
    #[persist = "sample-list"]
    sample_list: Mutex<Vec<PathBuf>>,

    #[id = "note"]
    pub note: IntParam,
    #[id = "min-velocity"]
    pub min_velocity: IntParam,
    #[id = "max-velocity"]
    pub max_velocity: IntParam,
}

impl Default for NihSamplerParams {
    fn default() -> Self {
        Self {
            editor_state: ViziaState::new(|| (400, 400)),
            sample_list: Mutex::new(vec![]),
            note: IntParam::new("Note", 40, IntRange::Linear { min: 0, max: 127 }),
            min_velocity: IntParam::new("Min velocity", 0, IntRange::Linear { min: 0, max: 127 }),
            max_velocity: IntParam::new("Max velocity", 127, IntRange::Linear { min: 0, max: 127 }),
        }
    }
}

impl Plugin for NihSampler {
    const NAME: &'static str = "Nih Sampler";
    const VENDOR: &'static str = "matidfk";
    const URL: &'static str = "https://youtu.be/dQw4w9WgXcQ";
    const EMAIL: &'static str = "info@example.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic;

    type SysExMessage = ();
    type BackgroundTask = ();

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let (producer, consumer) = rtrb::RingBuffer::new(10);
        self.consumer.replace(Some(consumer));

        editor_vizia::create(
            self.params.clone(),
            self.params.editor_state.clone(),
            Arc::new(Mutex::new(producer)),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        let sample_list = self.params.sample_list.lock().unwrap().clone();

        for path in sample_list.iter() {
            self.load_sample(path.clone()).unwrap_or(());
        }
        return true;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut consumer = self.consumer.take();
        if let Some(consumer) = &mut consumer {
            while let Ok(message) = consumer.pop() {
                match message {
                    ThreadMessage::LoadSample(path) => {
                        self.load_sample(path).unwrap_or(());
                    }
                    ThreadMessage::RemoveSample(path) => {
                        self.remove_sample(path);
                    }
                }
            }

            for playing_sample in &mut self.playing_samples {
                let data = &self.loaded_samples[&playing_sample.handle].0;
                for channel_samples in buffer.iter_samples() {
                    for sample in channel_samples {
                        *sample += data.get(playing_sample.position).unwrap_or(&0.0);
                        playing_sample.position += 1;
                    }
                }
            }
        }

        self.consumer.replace(consumer);

        let mut next_event = context.next_event();

        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            while let Some(event) = next_event {
                if event.timing() > sample_id as u32 {
                    break;
                }
                match event {
                    NoteEvent::NoteOn { note, velocity, .. }
                        if note == self.params.note.value() as u8
                            && (velocity * 127.0) as u8
                                >= self.params.min_velocity.value() as u8
                            && (velocity * 127.0) as u8
                                <= self.params.max_velocity.value() as u8 =>
                    {
                        // None if no samples are loaded
                        if let Some((path, _sample_data)) =
                            self.loaded_samples.iter().choose(&mut thread_rng())
                        {
                            let playing_sample = PlayingSample {
                                handle: path.clone(),
                                position: 0,
                            };

                            self.playing_samples.push(playing_sample);
                        }
                    }
                    event => context.send_event(event),
                    // _ => {}
                }
                next_event = context.next_event();
            }
        }

        // remove samples that are done playing
        self.playing_samples
            .retain(|e| e.position < self.loaded_samples[&e.handle].0.len());

        ProcessStatus::Normal
    }
}

impl NihSampler {
    fn load_sample(&mut self, path: PathBuf) -> Result<(), ()> {
        if !self.loaded_samples.contains_key(&path) {
            let reader = hound::WavReader::open(&path);
            if let Ok(mut reader) = reader {
                let spec = reader.spec();
                let _sample_rate = spec.sample_rate as f32;

                let samples = match spec.sample_format {
                    hound::SampleFormat::Int => reader
                        .samples::<i32>()
                        .map(|s| (s.unwrap_or_default() as f32 * 256.0) / i32::MAX as f32)
                        .collect::<Vec<f32>>(),
                    hound::SampleFormat::Float => reader
                        .samples::<f32>()
                        .map(|s| s.unwrap_or_default())
                        .collect::<Vec<f32>>(),
                };

                // resample if needed
                // if sample_rate != self.sample_rate {
                // let mut resampler = rubato::FftFixedIn::<f32>::new(
                //     sample_rate as usize,
                //     self.sample_rate as usize,
                //     samples.len(),
                //     2,
                //     spec.channels as usize,
                // )
                // .unwrap();
                // let out = resampler.process(&[samples], None).unwrap_or_default();
                // out[1];
                // }

                self.loaded_samples
                    .insert(path.clone(), LoadedSample(samples));
            }

            if !self.params.sample_list.lock().unwrap().contains(&path) {
                self.params.sample_list.lock().unwrap().push(path);
            }

            Ok(())
        } else {
            Err(())
        }
    }

    fn remove_sample(&mut self, path: PathBuf) {
        let mut sample_list = self.params.sample_list.lock().unwrap();
        if let Some(index) = sample_list.iter().position(|e| e == &path) {
            sample_list.remove(index);
        }
    }
}

impl ClapPlugin for NihSampler {
    const CLAP_ID: &'static str = "com.moist-plugins-gmbh.the-moistest-plugin-ever";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A simple random-selection sampler");
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
    const VST3_CLASS_ID: [u8; 16] = *b"NihSamplerrrrrrr";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Drum,
        Vst3SubCategory::Sampler,
        Vst3SubCategory::Instrument,
    ];
}

nih_export_clap!(NihSampler);
nih_export_vst3!(NihSampler);
