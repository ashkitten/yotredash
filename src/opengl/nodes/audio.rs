//! The audio node recieves audio input from PortAudio and analyzes it, outputting
//! the power spectrum of the audio as a Texture1d.
use portaudio::{self, Input, InputStreamCallbackArgs, InputStreamSettings, NonBlocking, PortAudio,
                Stream, StreamParameters};
use rb::{RbConsumer, RbProducer, SpscRb, RB};
use fftw::plan::{R2CPlan, R2CPlan32};
use fftw::types::{Flag, c32};
use glium::backend::Facade;
use glium::texture::Texture1d;
use num_traits::Zero;
use std::thread;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use failure::Error;
use super::{Node, NodeInputs, NodeOutput};

// Only deal with a single channel, we don't want to mixdown (yet).
// Also sidesteps phase cancellation.
const CHANNELS: i32 = 1;
const FRAMES_PER_BUFFER: u32 = 2048; // how many sample frames to pass to each callback
const SAMPLE_BUFFER_LENGTH: usize = FRAMES_PER_BUFFER as usize * 8;
const FFT_SIZE: usize = 2048;
const SPECTRUM_LENGTH: usize = FFT_SIZE / 2;
const SMOOTHING: f32 = 0.8;
const MIN_DB: f32 = -30.0;
const MAX_DB: f32 = 20.0;

/// The type of individual samples returned by PortAudio.
type Sample = f32;

/// Computes a Blackman window of size `size` with α=`alpha`.
#[allow(non_snake_case)]
fn blackman(size: usize, alpha: f32) -> Vec<f32> {
    use std::f32::consts::PI;

    let N = size as f32;
    let alpha_0 = (1.0 - alpha) / 2.0;
    let alpha_1 = 0.5;
    let alpha_2 = alpha / 2.0;

    let w = |n: f32| {
        alpha_0 - alpha_1 * ((2.0 * PI * n) / (N - 1.0)).cos()
            + alpha_2 * ((4.0 * PI * n) / (N - 1.0)).cos()
    };

    (0..size).map(|n| w(n as f32)).collect::<Vec<f32>>()
}

/// Encapsulates the lifetime of the audio system, owning the PortAudio connection and stream.
pub struct AudioNode {
    /// Our connection to PortAudio.
    #[allow(dead_code)]
    pa: PortAudio,

    /// Our OpenGL context.
    facade: Rc<Facade>,

    /// The input stream we recieve samples from.
    stream: Stream<NonBlocking, Input<Sample>>,

    /// A ringbuffer of samples, produced by the PortAudio callback and consumed by the
    /// analysis thread.
    sample_buffer: SpscRb<Sample>,

    /// The current computed complex spectrum (X).
    spectrum: Arc<RwLock<Vec<c32>>>,

    /// The last computed smoothed spectrum; retained for smoothing (X̂).
    spectrum_smoothed: Vec<f32>,

    /// The current spectrum texture.
    spectrum_texture: Rc<Texture1d>,
}

impl AudioNode {
    /// Set up our connection to PortAudio
    pub fn new(facade: &Rc<Facade>) -> Result<AudioNode, Error> {
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
                warn!("orun in producer");
            }

            portaudio::Continue
        };

        let stream = pa.open_non_blocking_stream(input_settings, callback)?;

        let mut node = AudioNode {
            stream,
            pa,
            sample_buffer,
            facade: Rc::clone(facade),
            spectrum: Arc::new(RwLock::new(vec![c32::zero(); SPECTRUM_LENGTH])),
            spectrum_smoothed: vec![0.0; SPECTRUM_LENGTH],
            spectrum_texture: Rc::new(Texture1d::empty(&**facade, SPECTRUM_LENGTH as u32)?),
        };

        node.run()?;

        Ok(node)
    }

    /// Launches the audio thread.
    pub fn run(&mut self) -> Result<(), Error> {
        let consumer = self.sample_buffer.consumer();
        // TODO: Replace with Default::default() when const generics are a thing
        let mut buf: [Sample; FFT_SIZE as usize] = [Default::default(); FFT_SIZE as usize];

        let n = FFT_SIZE as usize;

        let spectrum_lock = Arc::clone(&self.spectrum);
        thread::spawn(move || {
            // Use the window from §1.8.6 of the Web Audio API specification
            let window = blackman(n, 0.16);

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

                // window the buffer
                for i in 0..FRAMES_PER_BUFFER as usize {
                    buf[i] *= window[i];
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
        self.spectrum_smoothed = (&self.spectrum).read().unwrap()
            .iter()
            .zip(&self.spectrum_smoothed) // zip in old smoothed spectrum
            .map(|(x, x_old)| SMOOTHING * x_old + (1.0 - SMOOTHING) * x.norm())
            .collect();

        let spectrum_normalized: Vec<f32> = self.spectrum_smoothed
            .iter()
            .map(|x| (20.0 * x.log10() - MIN_DB) / (MAX_DB - MIN_DB))
            .collect();

        self.spectrum_texture = Rc::new(Texture1d::new(&*self.facade, spectrum_normalized)?);

        let mut outputs = HashMap::new();
        outputs.insert(
            "spectrum".to_string(),
            NodeOutput::Texture1d(Rc::clone(&self.spectrum_texture)),
        );
        Ok(outputs)
    }
}
