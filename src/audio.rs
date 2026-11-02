//! Output stream on the system's default device, rebuilt if the device goes away.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{ErrorKind, FromSample, SampleFormat, SizedSample, Stream, StreamConfig};

use crate::dsp::{Params, Synth};

pub struct Player {
    _stream: Stream,
    pub sample_rate: f32,
    lost: Arc<AtomicBool>,
}

impl Player {
    pub fn start(params: Arc<Params>) -> Result<Player, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("no audio output device found")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let format = supported.sample_format();
        let config: StreamConfig = supported.into();
        let sample_rate = config.sample_rate as f32;
        let lost = Arc::new(AtomicBool::new(false));

        let stream = match format {
            SampleFormat::F32 => build::<f32>(&device, config, params, lost.clone()),
            SampleFormat::F64 => build::<f64>(&device, config, params, lost.clone()),
            SampleFormat::I16 => build::<i16>(&device, config, params, lost.clone()),
            SampleFormat::I32 => build::<i32>(&device, config, params, lost.clone()),
            SampleFormat::U16 => build::<u16>(&device, config, params, lost.clone()),
            SampleFormat::U8 => build::<u8>(&device, config, params, lost.clone()),
            other => return Err(format!("unsupported sample format {other}")),
        }?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Player {
            _stream: stream,
            sample_rate,
            lost,
        })
    }

    /// True once the device disappeared or the stream must be rebuilt.
    /// Backends that reroute to the new default device on their own never set this.
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Relaxed)
    }
}

fn build<T>(
    device: &cpal::Device,
    config: StreamConfig,
    params: Arc<Params>,
    lost: Arc<AtomicBool>,
) -> Result<Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut synth = Synth::new(&params, config.sample_rate as f32);
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| synth.fill(&params, data, channels),
            move |err| {
                if matches!(
                    err.kind(),
                    ErrorKind::DeviceNotAvailable | ErrorKind::StreamInvalidated
                ) {
                    lost.store(true, Ordering::Relaxed);
                }
            },
            None,
        )
        .map_err(|e| e.to_string())
}
