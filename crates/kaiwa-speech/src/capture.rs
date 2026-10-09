//! Microphone capture while the talk button is held. The audio stream lives on its own
//! thread (cpal streams are not `Send` on every platform); the caller holds a handle with
//! the live level and gets the recording back when it stops.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

use crate::{SpeechError, level_of};

/// Mono samples and their rate.
#[derive(Debug, Clone, Default)]
pub struct Recording {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

pub struct Capture {
    stop: mpsc::Sender<()>,
    thread: JoinHandle<Result<Recording, SpeechError>>,
    level: Arc<AtomicU32>,
}

/// Mixes interleaved frames down to mono.
pub fn to_mono(data: &[f32], channels: usize) -> impl Iterator<Item = f32> + '_ {
    data.chunks(channels.max(1))
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
}

fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    buffer: Arc<Mutex<Vec<f32>>>,
    level: Arc<AtomicU32>,
) -> Result<cpal::Stream, SpeechError>
where
    T: SizedSample + Send + 'static,
    f32: FromSample<T>,
{
    let channels = usize::from(config.channels);
    device
        .build_input_stream::<T, _, _>(
            config,
            move |data: &[T], _| {
                let floats: Vec<f32> = data.iter().map(|s| s.to_sample::<f32>()).collect();
                let mono: Vec<f32> = to_mono(&floats, channels).collect();
                level.store(level_of(&mono).to_bits(), Ordering::Relaxed);
                if let Ok(mut b) = buffer.lock() {
                    b.extend_from_slice(&mono);
                }
            },
            |_| {},
            None,
        )
        .map_err(|e| SpeechError::Microphone(e.to_string()))
}

impl Capture {
    /// Opens the default microphone and records until [`Capture::stop`].
    pub fn start() -> Result<Self, SpeechError> {
        let (stop_tx, stop_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let level = Arc::new(AtomicU32::new(0));
        let thread_level = level.clone();
        let thread = std::thread::spawn(move || -> Result<Recording, SpeechError> {
            type Opened = (cpal::Stream, Arc<Mutex<Vec<f32>>>, u32);
            let opened: Result<Opened, SpeechError> = (|| {
                let host = cpal::default_host();
                let device = host
                    .default_input_device()
                    .ok_or(SpeechError::NoMicrophone)?;
                let supported = device
                    .default_input_config()
                    .map_err(|e| SpeechError::Microphone(e.to_string()))?;
                let rate = supported.sample_rate();
                let buffer = Arc::new(Mutex::new(Vec::new()));
                let config = supported.config();
                let stream = match supported.sample_format() {
                    SampleFormat::F32 => {
                        build::<f32>(&device, config, buffer.clone(), thread_level.clone())
                    }
                    SampleFormat::I16 => {
                        build::<i16>(&device, config, buffer.clone(), thread_level.clone())
                    }
                    SampleFormat::U16 => {
                        build::<u16>(&device, config, buffer.clone(), thread_level.clone())
                    }
                    SampleFormat::I32 => {
                        build::<i32>(&device, config, buffer.clone(), thread_level.clone())
                    }
                    other => Err(SpeechError::Microphone(format!(
                        "sample format {other} not supported"
                    ))),
                }?;
                stream
                    .play()
                    .map_err(|e| SpeechError::Microphone(e.to_string()))?;
                Ok((stream, buffer, rate))
            })();
            match opened {
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                    Err(e)
                }
                Ok((stream, buffer, rate)) => {
                    let _ = ready_tx.send(Ok(()));
                    let _ = stop_rx.recv();
                    drop(stream);
                    let samples = buffer.lock().map(|b| b.clone()).unwrap_or_default();
                    Ok(Recording {
                        samples,
                        sample_rate: rate,
                    })
                }
            }
        });
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                stop: stop_tx,
                thread,
                level,
            }),
            Ok(Err(_)) | Err(_) => Err(thread
                .join()
                .ok()
                .and_then(|r| r.err())
                .unwrap_or(SpeechError::NoMicrophone)),
        }
    }

    /// The current input level 0..1.
    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    pub fn stop(self) -> Result<Recording, SpeechError> {
        let _ = self.stop.send(());
        self.thread
            .join()
            .map_err(|_| SpeechError::Microphone("capture thread failed".into()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_frames_mix_to_mono() {
        let mono: Vec<f32> = to_mono(&[1.0, 0.0, 0.5, 0.5, -1.0, 1.0], 2).collect();
        assert_eq!(mono, vec![0.5, 0.5, 0.0]);
        let same: Vec<f32> = to_mono(&[0.25, 0.75], 1).collect();
        assert_eq!(same, vec![0.25, 0.75]);
    }
}
