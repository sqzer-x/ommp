use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, select, tick};
use rodio::buffer::SamplesBuffer;
use rodio::mixer::Mixer;
use rodio::{Decoder, OutputStreamBuilder, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::event::{AudioEvent, Event};

#[derive(Debug, Clone)]
pub enum PlayerCommand {
    Play(PathBuf),
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
    Seek(f64),
}

pub struct AudioEngine {
    cmd_tx: Sender<PlayerCommand>,
    _handle: std::thread::JoinHandle<()>,
}

impl AudioEngine {
    pub fn new(event_tx: Sender<Event>) -> Result<Self> {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();

        let handle = std::thread::spawn(move || {
            player_thread(cmd_rx, event_tx);
        });

        Ok(Self {
            cmd_tx,
            _handle: handle,
        })
    }

    pub fn send(&self, cmd: PlayerCommand) {
        let _ = self.cmd_tx.send(cmd);
    }
}

/// One flat loop over every command, with the sink and the playback clock held
/// as plain locals.
///
/// This used to be two loops: an idle one that matched only Play and Stop and
/// threw the rest away (`Ok(_) => {}`), and a playing one that recursed into
/// itself on every track change. That cost a stack frame and a still-connected
/// `Sink` per track, and — because volume lived only inside the playing loop —
/// every track that ended on its own started the next one at rodio's default
/// full volume.
fn player_thread(cmd_rx: Receiver<PlayerCommand>, event_tx: Sender<Event>) {
    let stream = match OutputStreamBuilder::open_default_stream() {
        Ok(s) => s,
        Err(e) => {
            let _ = event_tx.send(Event::Audio(AudioEvent::DeviceError(e.to_string())));
            return;
        }
    };

    let mixer = stream.mixer().clone();
    let position_ticker = tick(Duration::from_millis(250));

    let mut sink: Option<Sink> = None;
    // Survives across tracks and idle periods; applied to every sink we build.
    let mut volume: f32 = 1.0;
    let mut duration: f64 = 0.0;
    let mut play_start: Option<Instant> = None;
    let mut accumulated_secs: f64 = 0.0;
    let mut is_paused = false;

    loop {
        select! {
            recv(cmd_rx) -> msg => {
                match msg {
                    Ok(PlayerCommand::Play(path)) => {
                        if let Some(old) = sink.take() {
                            old.stop();
                        }
                        accumulated_secs = 0.0;
                        is_paused = false;
                        match open_and_play(&mixer, &path, volume) {
                            Ok((new_sink, new_duration)) => {
                                sink = Some(new_sink);
                                duration = new_duration;
                                play_start = Some(Instant::now());
                                let _ = event_tx.send(Event::Audio(AudioEvent::Playing));
                            }
                            Err(e) => {
                                duration = 0.0;
                                play_start = None;
                                let _ = event_tx.send(Event::Audio(AudioEvent::TrackError(e)));
                            }
                        }
                    }
                    Ok(PlayerCommand::Pause) => {
                        if let Some(ref s) = sink {
                            if !is_paused {
                                s.pause();
                                if let Some(start) = play_start.take() {
                                    accumulated_secs += start.elapsed().as_secs_f64();
                                }
                                is_paused = true;
                                let _ = event_tx.send(Event::Audio(AudioEvent::Paused));
                            }
                        }
                    }
                    Ok(PlayerCommand::Resume) => {
                        if let Some(ref s) = sink {
                            if is_paused {
                                s.play();
                                play_start = Some(Instant::now());
                                is_paused = false;
                                let _ = event_tx.send(Event::Audio(AudioEvent::Playing));
                            }
                        }
                    }
                    Ok(PlayerCommand::Stop) => {
                        if let Some(old) = sink.take() {
                            old.stop();
                        }
                        play_start = None;
                        accumulated_secs = 0.0;
                        is_paused = false;
                        let _ = event_tx.send(Event::Audio(AudioEvent::Stopped));
                    }
                    Ok(PlayerCommand::SetVolume(vol)) => {
                        // Remembered even with nothing playing, so the volume
                        // restored at startup applies to the first track.
                        volume = vol.clamp(0.0, 1.0);
                        if let Some(ref s) = sink {
                            s.set_volume(volume);
                        }
                    }
                    Ok(PlayerCommand::Seek(secs)) => {
                        if let Some(ref s) = sink {
                            if s.try_seek(Duration::from_secs_f64(secs)).is_ok() {
                                accumulated_secs = secs;
                                if !is_paused {
                                    play_start = Some(Instant::now());
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            recv(position_ticker) -> _ => {
                let finished = sink.as_ref().is_some_and(|s| s.empty()) && !is_paused;
                if finished {
                    sink = None;
                    play_start = None;
                    accumulated_secs = 0.0;
                    let _ = event_tx.send(Event::Audio(AudioEvent::TrackFinished));
                } else if sink.is_some() {
                    let pos = if is_paused {
                        accumulated_secs
                    } else if let Some(start) = play_start {
                        accumulated_secs + start.elapsed().as_secs_f64()
                    } else {
                        accumulated_secs
                    };
                    // Only clamp when the decoder actually reported a length —
                    // VBR MP3s without a Xing header give 0.0, and clamping to
                    // that pins the elapsed time at 0:00 for the whole track.
                    let position_secs = if duration > 0.0 { pos.min(duration) } else { pos };
                    let _ = event_tx.send(Event::Audio(AudioEvent::PositionUpdate {
                        position_secs,
                        duration_secs: duration,
                    }));
                }
            }
        }
    }
}

fn open_and_play(mixer: &Mixer, path: &PathBuf, volume: f32) -> Result<(Sink, f64), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    // First try rodio's Decoder
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        if let Ok(source) = Decoder::new(reader) {
            let duration = Source::total_duration(&source)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            return Ok((start_sink(mixer, source, volume), duration));
        }
    }

    // Rodio failed — try extension-specific rodio decoders
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        let result = match ext.as_str() {
            "mp3" => Decoder::new_mp3(reader).ok(),
            "flac" => Decoder::new_flac(reader).ok(),
            "wav" => Decoder::new_wav(reader).ok(),
            "ogg" => Decoder::new_vorbis(reader).ok(),
            _ => None,
        };
        if let Some(source) = result {
            let duration = Source::total_duration(&source)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            return Ok((start_sink(mixer, source, volume), duration));
        }
    }

    // Fall back to symphonia direct decoding for m4a/mp4/etc
    decode_with_symphonia(mixer, path, volume)
}

/// Build a playing sink at the player's current volume. A fresh `Sink` starts at
/// rodio's default 1.0, so skipping the `set_volume` here is what made every
/// naturally-ended track hand off to the next one at full blast.
fn start_sink<S>(mixer: &Mixer, source: S, volume: f32) -> Sink
where
    S: Source + Send + 'static,
{
    let sink = Sink::connect_new(mixer);
    sink.set_volume(volume);
    sink.append(source);
    sink.play();
    sink
}

/// Decode using symphonia directly, buffer the entire track, and play via rodio Sink.
fn decode_with_symphonia(mixer: &Mixer, path: &Path, volume: f32) -> Result<(Sink, f64), String> {
    let file = File::open(path).map_err(|e| format!("Open: {}", e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("Probe: {}", e))?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| "No audio track found".to_string())?;

    let track_id = track.id;
    let codec_params = track.codec_params.clone();
    let sample_rate = codec_params.sample_rate.unwrap_or(44100);
    let channels = codec_params.channels.map(|c| c.count()).unwrap_or(2) as u16;

    // Calculate duration from track params
    let duration_secs = codec_params
        .n_frames
        .map(|n| n as f64 / sample_rate as f64)
        .or_else(|| {
            codec_params
                .time_base
                .and_then(|tb| codec_params.n_frames.map(|n| tb.calc_time(n).seconds as f64))
        })
        .unwrap_or(0.0);

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Codec: {}", e))?;

    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(_) => break,
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let spec = *decoded.spec();
        let num_frames = decoded.frames();
        let mut sample_buf = SampleBuffer::<f32>::new(num_frames as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        all_samples.extend_from_slice(sample_buf.samples());
    }

    if all_samples.is_empty() {
        return Err("No audio data decoded".to_string());
    }

    let buffer = SamplesBuffer::new(channels, sample_rate, all_samples);
    let actual_duration = Source::total_duration(&buffer)
        .map(|d| d.as_secs_f64())
        .unwrap_or(duration_secs);

    Ok((start_sink(mixer, buffer, volume), actual_duration))
}
