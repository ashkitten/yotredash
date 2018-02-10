//! The audio node recieves audio input from PortAudio and analyzes it, outputting
//! the power spectrum of the audio as a Texture1d.
use portaudio::{self, Input, InputStreamCallbackArgs, InputStreamSettings, NonBlocking, PortAudio,
                Stream, StreamParameters};
use rb::{RbConsumer, RbProducer, SpscRb, RB};
use fftw::plan::{R2CPlan, R2CPlan32};
use fftw::types::{Flag, c32};
use num_traits::Zero;
use std::default::Default;
use std::thread;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use failure::Error;
use super::{Node, NodeInputs, NodeOutput};

// Only deal with a single channel, we don't want to mixdown (yet).
// Also sidesteps phase cancellation.
const CHANNELS: i32 = 1;
const FRAMES_PER_BUFFER: u32 = 256; // how many sample frames to pass to each callback
const SAMPLE_BUFFER_LENGTH: usize = FRAMES_PER_BUFFER as usize * 4;

/// The type of individual samples returned by PortAudio.
type Sample = f32;

/// Encapsulates the lifetime of the audio system, owning the PortAudio connection and stream.
pub struct AudioNode {
    /// Our connection to PortAudio.
    #[allow(dead_code)]
    pa: PortAudio,

    /// The input stream we recieve samples from.
    stream: Stream<NonBlocking, Input<Sample>>,

    /// A ringbuffer of samples, produced by the PortAudio callback and consumed by the
    /// analysis thread.
    sample_buffer: SpscRb<Sample>,

    /// The current computed complex spectrum.
    spectrum: Arc<RwLock<Vec<c32>>>,
}

impl AudioNode {
    /// Set up our connection to PortAudio
    pub fn new() -> Result<AudioNode, Error> {
        let pa = PortAudio::new()?;

        debug!("PortAudio version: {} {}", pa.version(), pa.version_text()?);

        let input = pa.default_input_device()?;
        debug!("Input metadata: {:?}", pa.device_info(input)?);

        let input_params = {
            // Just making sure we document this instead of passing in a raw true :D
            const INTERLEAVED: bool = true;

            let latency = pa.device_info(input)?.default_low_input_latency;
            StreamParameters::new(input, CHANNELS, INTERLEAVED, latency)
        };

        let input_settings = {
            let sample_rate = pa.device_info(input)?.default_sample_rate;
            InputStreamSettings::new(input_params, sample_rate, FRAMES_PER_BUFFER)
        };

        let sample_buffer = SpscRb::new(SAMPLE_BUFFER_LENGTH);
        let producer = sample_buffer.producer();
        let callback = move |InputStreamCallbackArgs { buffer, .. }| {
            // TODO: Handle overruns gracefully instead of panic!()ing.
            if let Err(_) = producer.write(&buffer) {
                warn!("xrun in producer");
            }

            portaudio::Continue
        };

        let stream = pa.open_non_blocking_stream(input_settings, callback)?;

        let mut node = AudioNode {
            stream,
            pa,
            sample_buffer,
            spectrum: Arc::new(RwLock::new(vec![c32::zero(); SAMPLE_BUFFER_LENGTH / 2 + 1])),
        };

        node.run()?;

        Ok(node)
    }

    /// Launches the audio thread.
    pub fn run(&mut self) -> Result<(), Error> {
        let consumer = self.sample_buffer.consumer();
        // TODO: Replace with Default::default() when const generics are a thing
        let mut buf: [Sample; FRAMES_PER_BUFFER as usize] =
            [Default::default(); FRAMES_PER_BUFFER as usize];

        let n = FRAMES_PER_BUFFER as usize;

        let spectrum_lock = Arc::clone(&self.spectrum);
        thread::spawn(move || {
            let mut plan: R2CPlan32 = {
                R2CPlan::new(
                    &[n],
                    &mut buf,
                    &mut *spectrum_lock.write().unwrap(),
                    Flag::Estimate,
                ).unwrap()
            };

            loop {
                if let None = consumer.read_blocking(&mut buf) {
                    warn!("urun in reciever");
                }

                {
                    if let Err(e) = plan.r2c(&mut buf, &mut *spectrum_lock.write().unwrap()) {
                        error!("fftw plan failed to execute: {:?}", e);
                    }
                }
            }
        });

        self.stream.start()?;

        Ok(())
    }
}

impl Node for AudioNode {
    fn render(&mut self, _inputs: &NodeInputs) -> Result<HashMap<String, NodeOutput>, Error> {
        let power_spectrum: Vec<f32> = {
            self.spectrum
                .read()
                .unwrap()
                .iter()
                .map(|c| c.norm())
                .collect()
        };

        let mut outputs = HashMap::new();

        Ok(outputs)
    }
}
