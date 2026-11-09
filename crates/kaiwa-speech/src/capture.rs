//! Microphone capture while the talk button is held (push-to-talk, /LF100/). The audio stream
//! lives on its own thread (cpal streams are not `Send` on every platform); the caller holds a
//! handle with the live level and gets the recording back when it stops. On Linux cpal talks to
//! PulseAudio (also PipeWire's) when it runs, so the system shares the microphone and its
//! names are the ones of the system's sound settings; without it, ALSA. The learner may pick
//! a microphone in the settings ([`input_devices`]); [`check`] says how a test recording
//! sounds.

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

/// A microphone of this system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputDevice {
    /// cpal's id (`host:device`), stable across runs; stored in the profile.
    pub id: String,
    pub name: String,
    /// The system's default input.
    pub default: bool,
}

/// Whether a device id is a microphone to offer. PulseAudio (also on PipeWire) lists a
/// "monitor" of every output, a recording of what plays, not a microphone. ALSA without a
/// sound server lists plugins (null, speex, …) and each card several times; only the
/// default and one entry per card (`sysdefault`) are offered.
pub fn listed(id: &str) -> bool {
    match id.split_once(':') {
        Some(("pulseaudio", dev)) => !dev.ends_with(".monitor"),
        Some(("alsa", dev)) => dev == "default" || dev.starts_with("sysdefault:"),
        _ => true,
    }
}

/// The microphones of this system, the default first.
pub fn input_devices() -> Vec<InputDevice> {
    let host = cpal::default_host();
    let default = host
        .default_input_device()
        .and_then(|d| d.id().ok())
        .map(|id| id.to_string());
    let mut list: Vec<InputDevice> = host
        .input_devices()
        .map(|it| {
            it.filter_map(|d| {
                let id = d.id().ok()?.to_string();
                if !listed(&id) {
                    return None;
                }
                let name = d
                    .description()
                    .map(|x| x.name().to_string())
                    .unwrap_or_else(|_| id.clone());
                Some(InputDevice {
                    default: default.as_deref() == Some(id.as_str()),
                    id,
                    name,
                })
            })
            .collect()
        })
        .unwrap_or_default();
    list.sort_by_key(|d| !d.default);
    list
}

/// The device with `id`, else the default microphone (a device that is gone falls back).
fn open_device(id: Option<&str>) -> Option<cpal::Device> {
    let host = cpal::default_host();
    id.and_then(|id| id.parse::<cpal::DeviceId>().ok())
        .and_then(|id| host.device_by_id(&id))
        .or_else(|| host.default_input_device())
}

/// How a test recording sounds, from 50 ms windows: the speech is the loud windows, the room
/// the quiet ones.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Check {
    /// dBFS of the loud windows (90th percentile).
    pub speech_db: f32,
    /// dBFS of the quiet windows (10th percentile): the room and the microphone's own noise.
    pub noise_db: f32,
    /// dBFS of the loudest 250 ms: a short answer in a long take (thinking first) still
    /// shows here, where the percentiles see only the room.
    pub loudest_db: f32,
    /// Share of samples at full scale.
    pub clipped: f32,
    pub seconds: f32,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Nothing comes in: muted, wrong device or no permission.
    Silent,
    /// Speech, but too quiet for the recognition.
    Quiet,
    /// The speech hardly stands out from the noise.
    Noisy,
    /// Too loud: the peaks are cut.
    Clipping,
    Good,
}

/// Below this nothing comes in (dBFS).
pub const SILENT_DB: f32 = -60.0;
/// Below this the speech is too quiet for the recognition (dBFS).
pub const QUIET_DB: f32 = -40.0;
/// Speech at most this far above the noise is too noisy (dB).
pub const NOISY_DB: f32 = 12.0;
/// More than this share of samples at full scale is clipping.
pub const CLIPPED: f32 = 0.001;

impl Check {
    /// Whether a voice could be in the recording: something comes in, and it is either loud
    /// enough or stands out from the room. Silence, or a faint steady hiss (an input with no
    /// microphone on it), is no voice, and the recognition would only invent sounds from it.
    pub fn voice(&self) -> bool {
        self.loudest_db >= SILENT_DB
            && (self.loudest_db >= QUIET_DB || self.loudest_db - self.noise_db >= NOISY_DB)
    }
}

pub fn check(rec: &Recording) -> Check {
    let rate = rec.sample_rate.max(1) as usize;
    let win = (rate / 20).max(1);
    let db = |w: &[f32]| {
        let rms = (w.iter().map(|s| s * s).sum::<f32>() / w.len() as f32).sqrt();
        if rms > 0.0 {
            20.0 * rms.log10()
        } else {
            -120.0
        }
    };
    let mut levels: Vec<f32> = rec
        .samples
        .chunks(win)
        .filter(|w| w.len() == win)
        .map(db)
        .collect();
    // The loudest 250 ms (five windows), as the mean power of the block.
    let block = levels.len().clamp(1, 5);
    let loudest_db = levels
        .windows(block)
        .map(|b| b.iter().map(|d| 10f32.powf(d / 10.0)).sum::<f32>() / block as f32)
        .fold(None, |m: Option<f32>, p| Some(m.map_or(p, |m| m.max(p))))
        .map_or(-120.0, |p| 10.0 * p.log10());
    levels.sort_by(f32::total_cmp);
    let at = |q: f32| {
        levels
            .get(((levels.len() as f32 - 1.0) * q).round() as usize)
            .copied()
            .unwrap_or(-120.0)
    };
    let (speech_db, noise_db) = (at(0.9), at(0.1));
    let clipped = if rec.samples.is_empty() {
        0.0
    } else {
        rec.samples.iter().filter(|s| s.abs() >= 0.99).count() as f32 / rec.samples.len() as f32
    };
    let verdict = if speech_db < SILENT_DB {
        Verdict::Silent
    } else if clipped > CLIPPED {
        Verdict::Clipping
    } else if speech_db < QUIET_DB {
        Verdict::Quiet
    } else if speech_db - noise_db < NOISY_DB {
        Verdict::Noisy
    } else {
        Verdict::Good
    };
    Check {
        speech_db,
        noise_db,
        loudest_db,
        clipped,
        seconds: rec.samples.len() as f32 / rate as f32,
        verdict,
    }
}

impl Capture {
    /// Opens the default microphone and records until [`Capture::stop`].
    pub fn start() -> Result<Self, SpeechError> {
        Self::start_on(None)
    }

    /// Opens the microphone `device` (an id from [`input_devices`]; gone or `None`: the
    /// default) and records until [`Capture::stop`].
    pub fn start_on(device: Option<&str>) -> Result<Self, SpeechError> {
        let device = device.map(str::to_string);
        let (stop_tx, stop_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let level = Arc::new(AtomicU32::new(0));
        let thread_level = level.clone();
        let thread = std::thread::spawn(move || -> Result<Recording, SpeechError> {
            type Opened = (cpal::Stream, Arc<Mutex<Vec<f32>>>, u32);
            let opened: Result<Opened, SpeechError> = (|| {
                let device = open_device(device.as_deref()).ok_or(SpeechError::NoMicrophone)?;
                let supported = crate::usable_config(
                    device
                        .default_input_config()
                        .map_err(|e| SpeechError::Microphone(e.to_string()))?,
                    || device.supported_input_configs().into_iter().flatten(),
                );
                let rate = supported.sample_rate();
                let buffer = Arc::new(Mutex::new(Vec::new()));
                let mut config = supported.config();
                config.buffer_size = crate::buffer_for(&device, &supported, 20);
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

    /// One second of quiet room, then one second of a tone at `amp` (speech stands in).
    fn take(noise: f32, amp: f32) -> Recording {
        let rate = 16_000;
        let mut samples: Vec<f32> = (0..rate)
            .map(|i| noise * if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        samples.extend((0..rate).map(|i| amp * (i as f32 * 0.07).sin()));
        Recording {
            samples,
            sample_rate: rate as u32,
        }
    }

    #[test]
    fn a_test_recording_gets_a_verdict() {
        assert_eq!(check(&Recording::default()).verdict, Verdict::Silent);
        assert_eq!(check(&take(0.0, 0.0)).verdict, Verdict::Silent);
        assert_eq!(check(&take(0.001, 0.3)).verdict, Verdict::Good);
        assert_eq!(check(&take(0.0005, 0.005)).verdict, Verdict::Quiet);
        assert_eq!(check(&take(0.1, 0.2)).verdict, Verdict::Noisy);
        assert_eq!(
            check(&take(0.001, 1.5).clamped()).verdict,
            Verdict::Clipping
        );
        let c = check(&take(0.001, 0.3));
        assert!((c.seconds - 2.0).abs() < 1e-3);
        assert!(c.speech_db > -15.0 && c.noise_db < -55.0, "{c:?}");
    }

    #[test]
    fn silence_and_faint_hiss_are_no_voice() {
        // No microphone: digital silence, or the faint steady noise of an empty input.
        assert!(!check(&take(0.0, 0.0)).voice());
        assert!(!check(&take(0.0001, 0.0)).voice());
        assert!(!check(&take(0.003, 0.004)).voice());
        // Quiet but clear speech, and loud speech in a noisy room, still go to the recognition.
        assert!(check(&take(0.0005, 0.005)).voice());
        assert!(check(&take(0.1, 0.2)).voice());
        assert!(check(&take(0.001, 0.3)).voice());
        // Thinking 10 s in a quiet room, then a short clear answer and the pause to the end of
        // the turn: a voice, though the speech is under a tenth of the take.
        let rate = 16_000;
        let mut samples = vec![0.0001f32; rate * 10];
        samples.extend((0..rate).map(|i| 0.2 * (i as f32 * 0.07).sin()));
        samples.extend(std::iter::repeat_n(0.0001, rate * 6 / 5));
        let late = Recording {
            samples,
            sample_rate: rate as u32,
        };
        assert!(check(&late).voice(), "{:?}", check(&late));
    }

    impl Recording {
        fn clamped(mut self) -> Self {
            self.samples
                .iter_mut()
                .for_each(|s| *s = s.clamp(-1.0, 1.0));
            self
        }
    }

    #[test]
    fn only_microphones_are_offered() {
        assert!(listed(
            "pulseaudio:alsa_input.usb-RODE_XCM-50-00.mono-fallback"
        ));
        assert!(!listed(
            "pulseaudio:alsa_output.usb-Focusrite-00.analog-stereo.monitor"
        ));
        assert!(listed("alsa:default"));
        assert!(listed("alsa:sysdefault:CARD=USB"));
        for plugin in [
            "alsa:null",
            "alsa:speex",
            "alsa:hw:CARD=1,DEV=0",
            "alsa:front:CARD=USB,DEV=0",
        ] {
            assert!(!listed(plugin), "{plugin}");
        }
        assert!(listed("coreaudio:BuiltInMicrophoneDevice"));
    }

    #[test]
    fn devices_list_without_panicking() {
        // CI has no sound card: an empty list, never a panic.
        let list = input_devices();
        assert!(list.iter().filter(|d| d.default).count() <= 1);
    }
}
