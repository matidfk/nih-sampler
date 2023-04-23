use crate::playing_sample::PlayingSample;
use nih_plug_vizia::ViziaState;
use rand::prelude::*;
use rubato::Resampler;
use std::{
    cell::RefCell,
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use rtrb;

use nih_plug::prelude::*;
mod editor_vizia;
mod playing_sample;

/// A loaded sample stored as a vec of channels
pub struct LoadedSample(Vec<Vec<f32>>);

#[derive(Clone)]
pub enum ThreadMessage {
    LoadSample(PathBuf),
    RemoveSample(PathBuf),
}

/// Main plugin struct
pub struct NihSampler {
    pub params: Arc<NihSamplerParams>,
    pub playing_samples: Vec<PlayingSample>,
    pub sample_rate: f32,
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
            sample_rate: 44100.0,
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

    #[id = "min-volume"]
    pub min_volume: FloatParam,
    #[id = "max-volume"]
    pub max_volume: FloatParam,
}

impl Default for NihSamplerParams {
    fn default() -> Self {
        Self {
            editor_state: ViziaState::new(|| (400, 400)),
            sample_list: Mutex::new(vec![]),
            note: IntParam::new("Note", 40, IntRange::Linear { min: 0, max: 127 }),
            min_velocity: IntParam::new("Min velocity", 0, IntRange::Linear { min: 0, max: 127 }),
            max_velocity: IntParam::new("Max velocity", 127, IntRange::Linear { min: 0, max: 127 }),
            min_volume: FloatParam::new(
                "Min volume",
                util::db_to_gain(0.0),
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            max_volume: FloatParam::new(
                "Max volume",
                util::db_to_gain(0.0),
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
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
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        let sample_list = self.params.sample_list.lock().unwrap().clone();

        for path in sample_list.iter() {
            self.load_sample(path.clone()).unwrap_or(());
        }

        self.sample_rate = buffer_config.sample_rate;

        return true;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.proess_messages();
        self.process_midi(context, buffer);

        for playing_sample in &mut self.playing_samples {
            let data = &self.loaded_samples[&playing_sample.handle].0;
            for channel_samples in buffer.iter_samples() {
                for (channel_index, sample) in channel_samples.into_iter().enumerate() {
                    *sample += data
                        .get(channel_index)
                        .unwrap_or(&vec![])
                        .get(playing_sample.position)
                        .unwrap_or(&0.0)
                        * playing_sample.gain;
                }
                playing_sample.position += 1;
            }
        }

        // remove samples that are done playing
        self.playing_samples
            .retain(|e| e.position < self.loaded_samples[&e.handle].0.len());

        ProcessStatus::Normal
    }
}

fn uninterleave(samples: Vec<f32>, channels: usize) -> Vec<Vec<f32>> {
    let mut new_samples = vec![vec![]; channels];
    let mut i = 0;
    let mut iter = samples.into_iter();

    while let Some(sample) = iter.next() {
        new_samples[i % channels].push(sample);
        i += 1;
    }

    new_samples
}

impl NihSampler {
    fn velocity_to_gain(&self, velocity: u8) -> f32 {
        // this is just mapping from the velocity range to volume range
        self.params.min_volume.value()
            + (self.params.max_volume.value() - self.params.min_volume.value())
                * (velocity - self.params.min_velocity.value() as u8) as f32
                / (self.params.max_velocity.value() - self.params.min_velocity.value()) as f32
    }

    fn proess_messages(&mut self) {
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
        }

        self.consumer.replace(consumer);
    }

    fn process_midi(&mut self, context: &mut impl ProcessContext<Self>, buffer: &mut Buffer) {
        let mut next_event = context.next_event();
        for (sample_id, _channel_samples) in buffer.iter_samples().enumerate() {
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
                            let playing_sample = PlayingSample::new(
                                path.clone(),
                                self.velocity_to_gain((velocity * 127.0) as u8),
                            );

                            self.playing_samples.push(playing_sample);
                        }
                    }
                    event => context.send_event(event),
                    // _ => {}
                }
                next_event = context.next_event();
            }
        }
    }

    fn load_sample(&mut self, path: PathBuf) -> Result<(), ()> {
        if !self.loaded_samples.contains_key(&path) {
            // wav only for now
            let reader = hound::WavReader::open(&path);
            if let Ok(mut reader) = reader {
                let spec = reader.spec();
                let sample_rate = spec.sample_rate as f32;
                let channels = spec.channels as usize;

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

                let samples = uninterleave(samples, channels);

                // resample if needed
                if sample_rate != self.sample_rate {
                    let mut resampler = rubato::FftFixedIn::<f32>::new(
                        sample_rate as usize,
                        self.sample_rate as usize,
                        samples.len(),
                        2,
                        spec.channels as usize,
                    )
                    .unwrap();

                    let chunksize = resampler.input_frames_next();

                    let num_chunks = samples.len() / (spec.channels as usize * chunksize);

                    let mut new_samples = vec![vec![]; channels];
                    for _chunk in 0..num_chunks {
                        //TODO samples to vec vec not vec
                        let waves_out = resampler.process(&samples, None).unwrap();
                        for (channel_index, channel) in waves_out.into_iter().enumerate() {
                            for (sample_index, sample) in channel.into_iter().enumerate() {
                                new_samples[channel_index].push(sample);
                            }
                        }
                    }
                    let samples = new_samples;
                }

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
