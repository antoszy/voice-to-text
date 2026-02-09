use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    stream: Option<cpal::Stream>,
    device_sample_rate: u32,
}

const TARGET_SAMPLE_RATE: u32 = 16_000;

impl AudioRecorder {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("No input audio device found")?;

        let config = device.default_input_config()?;
        log::info!(
            "Audio device: {}, sample rate: {}, channels: {}",
            device.name().unwrap_or_default(),
            config.sample_rate().0,
            config.channels()
        );

        Ok(Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            device_sample_rate: config.sample_rate().0,
        })
    }

    pub fn start(&mut self) -> Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("No input audio device found")?;

        let config = device.default_input_config()?;
        self.device_sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        let samples = self.samples.clone();
        samples.lock().unwrap().clear();

        let err_fn = |err: cpal::StreamError| {
            log::error!("Audio stream error: {err}");
        };

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mono: Vec<f32> = data
                        .chunks(channels)
                        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                        .collect();
                    samples.lock().unwrap().extend_from_slice(&mono);
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let mono: Vec<f32> = data
                        .chunks(channels)
                        .map(|frame| {
                            frame.iter().map(|&s| s as f32 / i16::MAX as f32).sum::<f32>()
                                / channels as f32
                        })
                        .collect();
                    samples.lock().unwrap().extend_from_slice(&mono);
                },
                err_fn,
                None,
            )?,
            format => anyhow::bail!("Unsupported sample format: {format:?}"),
        };

        stream.play()?;
        self.stream = Some(stream);
        log::info!("Recording started");
        Ok(())
    }

    /// Stop recording and return 16kHz mono f32 samples.
    pub fn stop(&mut self) -> Vec<f32> {
        self.stream.take(); // drops the stream, stopping recording
        let raw = std::mem::take(&mut *self.samples.lock().unwrap());
        log::info!(
            "Recording stopped: {} samples at {}Hz",
            raw.len(),
            self.device_sample_rate
        );

        if self.device_sample_rate == TARGET_SAMPLE_RATE {
            raw
        } else {
            resample(&raw, self.device_sample_rate, TARGET_SAMPLE_RATE)
        }
    }
}

fn resample(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (input.len() as f64 / ratio) as usize;
    (0..output_len)
        .map(|i| {
            let src = i as f64 * ratio;
            let idx = src as usize;
            let frac = src - idx as f64;
            if idx + 1 < input.len() {
                (input[idx] as f64 * (1.0 - frac) + input[idx + 1] as f64 * frac) as f32
            } else {
                input[idx.min(input.len() - 1)]
            }
        })
        .collect()
}
