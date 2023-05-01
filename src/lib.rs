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
use visualizer::Visualizer;

use rtrb;

use nih_plug::prelude::*;
mod editor_vizia;
mod playing_sample;
pub mod visualizer;

/// A loaded sample stored as a vec of samples in the form:
/// [
///     [a, a, a, ...],
///     [b, b, b, ...],
/// ]
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
    pub visualizer: Arc<Visualizer>,
}

impl Default for NihSampler {
    fn default() -> Self {
        Self {
            params: Arc::new(Default::default()),
            playing_samples: vec![],
            loaded_samples: HashMap::with_capacity(64),
            consumer: RefCell::new(None),
            sample_rate: 44100.0,
            visualizer: Arc::new(Visualizer::new()),
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
            editor_state: ViziaState::new(|| (400, 700)),
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
            Arc::clone(&self.visualizer),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        nih_log!("changed sample rate to {}", buffer_config.sample_rate);

        self.sample_rate = buffer_config.sample_rate;

        let sample_list = self.params.sample_list.lock().unwrap().clone();
        for path in sample_list {
            self.load_sample(path.clone());
        }

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

        let mut amplitude = 0.0;

        for playing_sample in &mut self.playing_samples {
            match self.loaded_samples.get(&playing_sample.handle) {
                Some(loaded_sample) => {
                    for channel_samples in buffer.iter_samples() {
                        // channel_samples is [a, b, c]
                        for (channel_index, sample) in channel_samples.into_iter().enumerate() {
                            let s = loaded_sample
                                .0
                                .get(channel_index)
                                .unwrap_or(&vec![])
                                .get(playing_sample.position)
                                .unwrap_or(&0.0)
                                * playing_sample.gain;
                            *sample += s;
                            amplitude += s.abs();
                        }
                        playing_sample.position += 1;
                    }
                }
                None => {}
            }
        }

        amplitude /= buffer.samples() as f32 * buffer.channels() as f32;
        self.visualizer.store(amplitude);

        // remove samples that are done playing
        self.playing_samples
            .retain(|e| match self.loaded_samples.get(&e.handle) {
                Some(sample) => e.position < sample.0[0].len(),
                None => false,
            });

        ProcessStatus::Normal
    }
}

fn uninterleave(samples: Vec<f32>, channels: usize) -> LoadedSample {
    // input looks like:
    // [a, b, a, b, a, b, ...]
    //
    // output should be:
    // [
    //    [a, a, a, ...],
    //    [b, b, b, ...]
    // ]

    let mut new_samples = vec![Vec::with_capacity(samples.len() / channels); channels];

    for sample_chunk in samples.chunks(channels) {
        // sample_chunk is a chunk like [a, b]
        for (i, sample) in sample_chunk.into_iter().enumerate() {
            new_samples[i].push(sample.clone());
        }
    }

    LoadedSample(new_samples)
}

fn resample(samples: LoadedSample, sample_rate_in: f32, sample_rate_out: f32) -> LoadedSample {
    let samples = samples.0;
    let mut resampler = rubato::FftFixedIn::<f32>::new(
        sample_rate_in as usize,
        sample_rate_out as usize,
        samples[0].len(),
        8,
        samples.len(),
    )
    .unwrap();

    match resampler.process(&samples, None) {
        Ok(mut waves_out) => {
            // get the duration of leading silence introduced by FFT
            // https://github.com/HEnquist/rubato/blob/52cdc3eb8e2716f40bc9b444839bca067c310592/src/synchro.rs#L654
            let silence_len = resampler.output_delay();

            for channel in waves_out.iter_mut() {
                channel.drain(..silence_len);
                channel.shrink_to_fit();
            }

            LoadedSample(waves_out)
        }
        Err(_) => LoadedSample(vec![]),
    }
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
                        self.load_sample(path);
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

    /// Loads a sample at the given filepath, overwriting any sample loaded with the given path
    fn load_sample(&mut self, path: PathBuf) {
        // wav only for now
        let reader = hound::WavReader::open(&path);
        if let Ok(mut reader) = reader {
            let spec = reader.spec();
            let sample_rate = spec.sample_rate as f32;
            let channels = spec.channels as usize;

            let interleaved_samples = match spec.sample_format {
                hound::SampleFormat::Int => reader
                    .samples::<i32>()
                    .map(|s| (s.unwrap_or_default() as f32 * 256.0) / i32::MAX as f32)
                    .collect::<Vec<f32>>(),
                hound::SampleFormat::Float => reader
                    .samples::<f32>()
                    .map(|s| s.unwrap_or_default())
                    .collect::<Vec<f32>>(),
            };

            let mut samples = uninterleave(interleaved_samples, channels);

            // resample if needed
            if sample_rate != self.sample_rate {
                samples = resample(samples, sample_rate, self.sample_rate);
            }

            self.loaded_samples.insert(path.clone(), samples);
        }

        if !self.params.sample_list.lock().unwrap().contains(&path) {
            self.params.sample_list.lock().unwrap().push(path);
        }
    }

    fn remove_sample(&mut self, path: PathBuf) {
        let mut sample_list = self.params.sample_list.lock().unwrap();
        if let Some(index) = sample_list.iter().position(|e| e == &path) {
            sample_list.remove(index);
        }
        self.loaded_samples.remove(&path);
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
