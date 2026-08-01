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

fn player_thread(cmd_rx: Receiver<PlayerCommand>, event_tx: Sender<Event>) {
    let stream = match OutputStreamBuilder::open_default_stream() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open audio output: {}", e);
            return;
        }
    };

    let mixer = stream.mixer().clone();
    let position_ticker = tick(Duration::from_millis(250));

    loop {
        select! {
            recv(cmd_rx) -> msg => {
                match msg {
                    Ok(PlayerCommand::Play(path)) => {
                        match open_and_play(&mixer, &path) {
                            Ok((sink, duration)) => {
                                let _ = event_tx.send(Event::Audio(AudioEvent::Playing));
                                run_playback_loop(
                                    sink, &mixer, &cmd_rx, &event_tx,
                                    &position_ticker, duration,
                                );
                            }
                            Err(e) => {
                                let _ = event_tx.send(Event::Audio(AudioEvent::TrackError(e)));
                            }
                        }
                    }
                    Ok(PlayerCommand::Stop) => {
                        let _ = event_tx.send(Event::Audio(AudioEvent::Stopped));
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
            recv(position_ticker) -> _ => {}
        }
    }
}

fn open_and_play(mixer: &Mixer, path: &PathBuf) -> Result<(Sink, f64), String> {
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
            let sink = Sink::connect_new(mixer);
            sink.append(source);
            sink.play();
            return Ok((sink, duration));
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
            let sink = Sink::connect_new(mixer);
            sink.append(source);
            sink.play();
            return Ok((sink, duration));
        }
    }

    // Fall back to symphonia direct decoding for m4a/mp4/etc
    decode_with_symphonia(mixer, path)
}

/// Decode using symphonia directly, buffer the entire track, and play via rodio Sink.
fn decode_with_symphonia(mixer: &Mixer, path: &Path) -> Result<(Sink, f64), String> {
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

    let sink = Sink::connect_new(mixer);
    sink.append(buffer);
    sink.play();
    Ok((sink, actual_duration))
}

fn run_playback_loop(
    sink: Sink,
    mixer: &Mixer,
    cmd_rx: &Receiver<PlayerCommand>,
    event_tx: &Sender<Event>,
    position_ticker: &Receiver<Instant>,
    mut duration: f64,
) {
    let mut play_start: Option<Instant> = Some(Instant::now());
    let mut accumulated_secs: f64 = 0.0;
    let mut is_paused = false;

    loop {
        select! {
            recv(cmd_rx) -> msg => {
                match msg {
                    Ok(PlayerCommand::Play(path)) => {
                        sink.stop();
                        match open_and_play(mixer, &path) {
                            Ok((new_sink, new_dur)) => {
                                new_sink.set_volume(sink.volume());
                                duration = new_dur;
                                let _ = event_tx.send(Event::Audio(AudioEvent::Playing));
                                run_playback_loop(
                                    new_sink, mixer, cmd_rx, event_tx,
                                    position_ticker, duration,
                                );
                            }
                            Err(e) => {
                                let _ = event_tx.send(Event::Audio(AudioEvent::TrackError(e)));
                            }
                        }
                        return;
                    }
                    Ok(PlayerCommand::Pause) => {
                        if !is_paused {
                            sink.pause();
                            if let Some(start) = play_start.take() {
                                accumulated_secs += start.elapsed().as_secs_f64();
                            }
                            is_paused = true;
                            let _ = event_tx.send(Event::Audio(AudioEvent::Paused));
                        }
                    }
                    Ok(PlayerCommand::Resume) => {
                        if is_paused {
                            sink.play();
                            play_start = Some(Instant::now());
                            is_paused = false;
                            let _ = event_tx.send(Event::Audio(AudioEvent::Playing));
                        }
                    }
                    Ok(PlayerCommand::Stop) => {
                        sink.stop();
                        let _ = event_tx.send(Event::Audio(AudioEvent::Stopped));
                        return;
                    }
                    Ok(PlayerCommand::SetVolume(vol)) => {
                        sink.set_volume(vol);
                    }
                    Ok(PlayerCommand::Seek(secs)) => {
                        if sink.try_seek(Duration::from_secs_f64(secs)).is_ok() {
                            accumulated_secs = secs;
                            if !is_paused {
                                play_start = Some(Instant::now());
                            }
                        }
                    }
                    Err(_) => return,
                }
            }
            recv(position_ticker) -> _ => {
                if sink.empty() && !is_paused {
                    let _ = event_tx.send(Event::Audio(AudioEvent::TrackFinished));
                    return;
                }

                let pos = if is_paused {
                    accumulated_secs
                } else if let Some(start) = play_start {
                    accumulated_secs + start.elapsed().as_secs_f64()
                } else {
                    accumulated_secs
                };

                let _ = event_tx.send(Event::Audio(AudioEvent::PositionUpdate {
                    position_secs: pos.min(duration),
                    duration_secs: duration,
                }));
            }
        }
    }
}
