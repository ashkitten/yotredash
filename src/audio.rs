//! The audio subsystem.
use portaudio::{self, Input, InputStreamCallbackArgs, InputStreamSettings, NonBlocking, PortAudio,
                Stream, StreamParameters};
use rb::{RbConsumer, RbProducer, SpscRb, RB};
use std::default::Default;
use std::thread;
use failure::Error;

// Only deal with a single channel, we don't want to mixdown (yet)
// Also sidesteps phase cancellation
const CHANNELS: i32 = 1;
const SAMPLE_RATE: f64 = 44_100.0; // 44.1kHz. TODO: don't hardcode
const FRAMES_PER_BUFFER: u32 = 256; // how many sample frames to pass to each callback
const SAMPLE_BUFFER_LENGTH: usize = FRAMES_PER_BUFFER as usize * 4;

/// The type of individual samples returned by PortAudio.
type Sample = f32;

/// Encapsulates the lifetime of the audio system, owning the PortAudio connection and stream.
pub struct Audio {
    /// Our connection to PortAudio.
    pa: PortAudio,

    /// The input stream we recieve samples from.
    stream: Stream<NonBlocking, Input<Sample>>,

    /// A ringbuffer of samples, produced by the PortAudio callback and consumed by the
    /// analysis thread.
    sample_buffer: SpscRb<Sample>,
}

impl Audio {
    pub fn run(&mut self) -> Result<(), Error> {
        let consumer = self.sample_buffer.consumer();
        thread::spawn(move || {
			// TODO: Replace with Default::default() when const generics are a thing
            let mut buf: [Sample; FRAMES_PER_BUFFER as usize] =
                [Default::default(); FRAMES_PER_BUFFER as usize];

            loop {
                consumer.read_blocking(&mut buf).unwrap();

                // TODO: Analyze audio (libfftw)

				// just a test, remove this
				let x: f32 = buf.iter().sum();
            }
        });

        self.stream.start()?;

        Ok(())
    }
}

/// Set up our connection to PortAudio
pub fn setup() -> Result<Audio, Error> {
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

    let input_settings = InputStreamSettings::new(input_params, SAMPLE_RATE, FRAMES_PER_BUFFER);

    let sample_buffer = SpscRb::new(SAMPLE_BUFFER_LENGTH);
    let producer = sample_buffer.producer();
    let callback = move |InputStreamCallbackArgs { buffer, .. }| {
		// TODO: Handle overruns gracefully instead of panic!()ing.
		producer.write(&buffer).unwrap();

        portaudio::Continue
    };

    let stream = pa.open_non_blocking_stream(input_settings, callback)?;

    Ok(Audio {
        stream,
        pa,
        sample_buffer,
    })
}
