//! The speaker: a queue of mono samples that the output stream drains. Like the microphone,
//! the stream lives on its own thread. Sentences are queued as they are made, so the first
//! one plays while the next is synthesized; [`Player::silence`] cuts the character off when
//! the learner interrupts.

use std::collections::VecDeque;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

use crate::SpeechError;

type Queue = Arc<Mutex<VecDeque<f32>>>;

pub struct Player {
    queue: Queue,
    rate: u32,
    stop: mpsc::Sender<()>,
    thread: Option<JoinHandle<()>>,
}

/// Linear resampling, enough for speech.
pub fn resample(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || samples.is_empty() {
        return samples.to_vec();
    }
    let n = (samples.len() as u64 * u64::from(to) / u64::from(from)) as usize;
    let step = f64::from(from) / f64::from(to);
    (0..n)
        .map(|i| {
            let pos = i as f64 * step;
            let j = pos as usize;
            let frac = (pos - j as f64) as f32;
            let a = samples[j.min(samples.len() - 1)];
            let b = samples[(j + 1).min(samples.len() - 1)];
            a + (b - a) * frac
        })
        .collect()
}

fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    queue: Queue,
) -> Result<cpal::Stream, SpeechError>
where
    T: SizedSample + FromSample<f32> + Send + 'static,
{
    let channels = usize::from(config.channels).max(1);
    device
        .build_output_stream::<T, _, _>(
            config,
            move |data: &mut [T], _| {
                let mut q = queue.lock().ok();
                for frame in data.chunks_mut(channels) {
                    let s = q.as_mut().and_then(|q| q.pop_front()).unwrap_or(0.0);
                    for out in frame {
                        *out = T::from_sample(s);
                    }
                }
            },
            |_| {},
            None,
        )
        .map_err(|e| SpeechError::Speaker(e.to_string()))
}

impl Player {
    /// Opens the default speaker; it plays silence until something is queued.
    pub fn start() -> Result<Self, SpeechError> {
        let queue: Queue = Arc::new(Mutex::new(VecDeque::new()));
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, SpeechError>>();
        let thread_queue = queue.clone();
        let thread = std::thread::spawn(move || {
            let opened = (|| {
                let device = cpal::default_host()
                    .default_output_device()
                    .ok_or(SpeechError::NoSpeaker)?;
                let supported = device
                    .default_output_config()
                    .map_err(|e| SpeechError::Speaker(e.to_string()))?;
                let rate = supported.sample_rate();
                let config = supported.config();
                let q = thread_queue.clone();
                let stream = match supported.sample_format() {
                    SampleFormat::F32 => build::<f32>(&device, config, q),
                    SampleFormat::I16 => build::<i16>(&device, config, q),
                    SampleFormat::U16 => build::<u16>(&device, config, q),
                    SampleFormat::I32 => build::<i32>(&device, config, q),
                    other => Err(SpeechError::Speaker(format!(
                        "sample format {other} not supported"
                    ))),
                }?;
                stream
                    .play()
                    .map_err(|e| SpeechError::Speaker(e.to_string()))?;
                Ok((stream, rate))
            })();
            match opened {
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
                Ok((stream, rate)) => {
                    let _ = ready_tx.send(Ok(rate));
                    let _ = stop_rx.recv();
                    drop(stream);
                }
            }
        });
        match ready_rx.recv() {
            Ok(Ok(rate)) => Ok(Self {
                queue,
                rate,
                stop: stop_tx,
                thread: Some(thread),
            }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => Err(SpeechError::NoSpeaker),
        }
    }

    /// Queues mono samples at `rate` behind what is already playing.
    pub fn play(&self, samples: &[f32], rate: u32) {
        let resampled = resample(samples, rate, self.rate);
        if let Ok(mut q) = self.queue.lock() {
            q.extend(resampled);
        }
    }

    /// Whether anything is still to be heard.
    pub fn playing(&self) -> bool {
        self.queue.lock().map(|q| !q.is_empty()).unwrap_or(false)
    }

    /// Stops at once and drops what was queued.
    pub fn silence(&self) {
        if let Ok(mut q) = self.queue.lock() {
            q.clear();
        }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampling_keeps_the_duration() {
        let one_second = vec![0.25; 24_000];
        let out = resample(&one_second, 24_000, 48_000);
        assert_eq!(out.len(), 48_000);
        assert!(out.iter().all(|s| (s - 0.25).abs() < 1e-6));
        assert_eq!(resample(&one_second, 24_000, 16_000).len(), 16_000);
    }

    #[test]
    fn resampling_interpolates() {
        assert_eq!(resample(&[0.0, 1.0], 1, 2), vec![0.0, 0.5, 1.0, 1.0]);
    }
}
