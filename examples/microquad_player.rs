//! Microquad BMS/BMSON Chart Player
//!
//! A simple BMS/BMSON chart player supporting 7+1k key layout.
//! Uses the microquad framework for visualization and audio playback.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use bms_rs::chart_process::BaseBpm;
use bms_rs::chart_process::prelude::*;
use bms_rs::{bms::prelude::*, chart_process::PlayheadEvent};
use clap::Parser;
use gametime::{TimeSpan, TimeStamp};
use kira::{
    AudioManager, AudioManagerSettings, Capacities, DefaultBackend,
    sound::static_sound::StaticSoundData,
};
use macroquad::prelude::Color;
use macroquad::prelude::*;
use rayon::prelude::*;
use strict_num_extended::{FinF64, PositiveF64};

/// Default BPM value
const DEFAULT_BPM: PositiveF64 = PositiveF64::new_const(120.0);

fn window_conf() -> Conf {
    Conf {
        window_title: "BMS Player".to_owned(),
        platform: miniquad::conf::Platform {
            linux_backend: miniquad::conf::LinuxBackend::WaylandWithX11Fallback,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() -> Result<(), String> {
    // 1. Parse command line arguments
    let config = Config::parse();

    if !config.chart_path.exists() {
        return Err(format!("File not found: {}", config.chart_path.display()));
    }

    println!("Loading chart: {}", config.chart_path.display());

    // 2. Load chart
    let (chart, base_bpm) = load_chart(&config.chart_path)?;
    println!("Chart loaded successfully");

    // 3. Extract base path (for audio file path resolution)
    let base_path = config.chart_path.parent().unwrap_or_else(|| Path::new("."));

    // 4. Calculate VisibleRangePerBpm
    let reaction_time = TimeSpan::from_duration(Duration::from_millis(config.reaction_time_ms));
    let visible_range = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);

    // 5. Preload audio in parallel using rayon
    println!("Loading audio files...");
    let audio_data_map = load_audio_files_parallel(chart.resources().wav_files(), base_path);
    println!(
        "Audio loading completed: {} files loaded",
        audio_data_map.len()
    );

    // 6. Start ChartPlayer
    let start_time = TimeStamp::now();
    let mut chart_player = ChartPlayer::start(chart, visible_range, start_time);
    // Set visibility range to [-0.5, 1.0) to show events past judgment line
    chart_player.set_visibility_range(FinF64::NEG_HALF..FinF64::ONE);
    println!("Player started");

    // 6.5. Initialize audio playback system
    let mut audio_manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings {
        capacities: Capacities {
            sub_track_capacity: 512,
            send_track_capacity: 16,
            clock_capacity: 8,
            modulator_capacity: 16,
            listener_capacity: 8,
        },
        internal_buffer_size: 256,
        ..Default::default()
    })
    .map_err(|e| format!("Failed to initialize audio: {}", e))?;
    println!("Audio system initialized");

    // Track played events to prevent duplicate audio playback
    let mut played_events = HashSet::new();

    // 7. Main loop
    println!("Starting playback...");
    let mut next_print_time = start_time;
    let mut missed_sounds = 0u32;
    loop {
        // Update playback state
        let now = TimeStamp::now();
        let events = chart_player.update(now);

        // Print playback status once per second
        if now >= next_print_time {
            let state = chart_player.playback_state();
            let elapsed = now
                .checked_elapsed_since(start_time)
                .unwrap_or(TimeSpan::ZERO);
            println!(
                "[Playback] Time: {:.1}s | BPM: {:.1} | Y: {:.2} | Speed: {:.2} | Scroll: {:.2} | Missed: {}",
                elapsed.as_secs_f64(),
                state.current_bpm.as_f64(),
                state.progressed_y().as_f64(),
                state.current_speed.as_f64(),
                state.current_scroll.as_f64(),
                missed_sounds,
            );
            next_print_time += TimeSpan::SECOND;
        }

        // Process all triggered events (play audio once per event)
        for event in &events {
            let wav_id = match event.event() {
                ChartEvent::Note { wav_id, .. } | ChartEvent::Bgm { wav_id } => wav_id,
                _ => continue,
            };

            let Some(id) = wav_id else { continue };
            let Some(audio) = audio_data_map.get(id) else {
                continue;
            };

            // Skip if this event has already been played
            if played_events.contains(&event.id()) {
                continue;
            }

            if let Err(e) = audio_manager.play(audio.clone()) {
                if matches!(e, kira::PlaySoundError::SoundLimitReached) {
                    missed_sounds += 1;
                } else {
                    eprintln!("Failed to play audio: {}", e);
                }
            } else {
                // Mark as played only on success
                played_events.insert(event.id());
            }
        }

        // Rendering
        // Clear background
        macroquad::prelude::clear_background(COLOR_BG);

        // Render tracks and judgment line
        render_tracks();

        // Render visible notes
        render_notes(&mut chart_player, &events);

        // Render info text (BPM, progress, etc.)
        render_info(&chart_player);

        macroquad::prelude::next_frame().await;
    }
}

/// Configuration parameters
#[derive(Parser, Debug)]
#[command(name = "microquad_player")]
#[command(about = "A simple BMS/BMSON chart player", long_about = None)]
struct Config {
    /// Chart file path
    #[arg(value_name = "FILE")]
    chart_path: PathBuf,

    /// Reaction time (milliseconds)
    #[arg(short, long, default_value = "550", value_name = "MILLISECONDS")]
    reaction_time_ms: u64,
}

/// Load chart file
///
/// Automatically detect format (BMS/BMSON) based on file extension and parse.
///
/// # Arguments
///
/// * `path` - Chart file path
///
/// # Returns
///
/// Returns parsed `PlayableChart` and base BPM value.
fn load_chart(path: &Path) -> Result<(PlayableChart, BaseBpm), String> {
    // Read file content
    // First read as bytes
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Use Shift-JIS encoding
    let content = {
        let bytes: &[u8] = &bytes;
        encoding_rs::SHIFT_JIS.decode(bytes).0.to_string()
    };

    // Determine format based on extension
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or("Invalid file extension")?;

    let (chart, base_bpm) = match extension.to_lowercase().as_str() {
        "bms" | "bme" | "bml" | "pms" => {
            // Parse using BmsProcessor
            let output = parse_bms(&content, default_config());
            let bms = output.bms.map_err(|e| format!("Parse error: {:?}", e))?;

            // First generate base BPM from BMS
            let base_bpm = StartBpmGenerator
                .generate(&bms)
                .unwrap_or(BaseBpm::new(DEFAULT_BPM));

            // Use KeyLayoutBeat mapper (supports 7+1k)
            let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms)
                .map_err(|e| format!("Failed to parse chart: {}", e))?;
            (chart, base_bpm)
        }
        "bmson" => {
            // BMSON format
            #[cfg(feature = "bmson")]
            {
                let bmson = serde_json::from_str(&content)
                    .map_err(|e| format!("JSON parse error: {}", e))?;

                // First generate base BPM from BMSON
                let base_bpm = StartBpmGenerator
                    .generate(&bmson)
                    .unwrap_or(BaseBpm::new(DEFAULT_BPM));

                let chart = BmsonProcessor::parse(&bmson);
                (chart, base_bpm)
            }
            #[cfg(not(feature = "bmson"))]
            return Err("BMSON feature not enabled".to_string());
        }
        _ => return Err(format!("Unsupported format: {}", extension)),
    };

    Ok((chart, base_bpm))
}

/// Find audio file with extension fallback support
///
/// # Arguments
///
/// * `path` - Original path (with or without extension)
/// * `extensions` - List of extensions to try
///
/// # Returns
///
/// Found file path, or None if not found
fn find_audio_with_extensions(path: &Path, extensions: &[&str]) -> Option<PathBuf> {
    // Get path without extension
    let stem = path.with_extension("");

    // Try each extension in order
    for ext in extensions {
        let candidate = stem.with_extension(ext);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

/// Preload all audio files in parallel using rayon
///
/// # Arguments
///
/// * `audio_files` - Audio file ID to path mapping
/// * `base_path` - Base path for audio files (usually chart file directory)
///
/// # Returns
///
/// Returns loaded audio mapping, failed audio files will be skipped.
fn load_audio_files_parallel(
    audio_files: &HashMap<WavId, PathBuf>,
    base_path: &Path,
) -> HashMap<WavId, StaticSoundData> {
    audio_files
        .par_iter()
        .filter_map(|(wav_id, path)| {
            let full_path = base_path.join(path);

            // Find file with extension fallback
            let found_path =
                find_audio_with_extensions(&full_path, &["ogg", "flac", "wav", "mp3"])?;

            match StaticSoundData::from_file(&found_path)
                .map_err(|e| format!("Failed to load audio: {}", e))
            {
                Ok(data) => Some((*wav_id, data)),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to load audio {:?} (ID: {:?}): {}",
                        found_path, wav_id, e
                    );
                    None
                }
            }
        })
        .collect()
}

// Screen size configuration
const SCREEN_WIDTH: f32 = 800.0;
const SCREEN_HEIGHT: f32 = 600.0;

// Track configuration
const TRACK_COUNT: usize = 8;
const TRACK_WIDTH: f32 = 60.0;
const TRACK_SPACING: f32 = 5.0;
const TOTAL_TRACKS_WIDTH: f32 = TRACK_COUNT as f32 * (TRACK_WIDTH + TRACK_SPACING);

// Judgment line configuration
const JUDGMENT_LINE_Y: f32 = 500.0;

// Color configuration
const COLOR_BG: Color = Color::from_rgba(30, 30, 30, 255);
const COLOR_TRACK_LINE: Color = Color::from_rgba(100, 100, 100, 255);
const COLOR_JUDGMENT_LINE: Color = Color::from_rgba(255, 255, 255, 255);
const COLOR_BAR_LINE: Color = Color::from_rgba(80, 80, 80, 180);
const COLOR_NOTE_WHITE: Color = Color::from_rgba(255, 255, 255, 255);
const COLOR_NOTE_BLUE: Color = Color::from_rgba(100, 149, 237, 255);
const COLOR_NOTE_SCRATCH: Color = Color::from_rgba(255, 0, 0, 255);
const COLOR_NOTE_MINE: Color = Color::from_rgba(255, 255, 0, 255);

/// Get all supported keys (7+1k layout)
#[must_use]
const fn supported_keys() -> [Key; 8] {
    [
        Key::Scratch(1),
        Key::Key(1),
        Key::Key(2),
        Key::Key(3),
        Key::Key(4),
        Key::Key(5),
        Key::Key(6),
        Key::Key(7),
    ]
}

/// Map Key to track index (0-7)
///
/// Only supports 7+1k: `Key::Key(1..=7)` and `Key::Scratch(1)`
///
/// # Returns
///
/// Returns corresponding track index, or None if not supported.
#[must_use]
const fn key_to_index(key: Key) -> Option<usize> {
    match key {
        Key::Scratch(1) => Some(0),
        Key::Key(1) => Some(1),
        Key::Key(2) => Some(2),
        Key::Key(3) => Some(3),
        Key::Key(4) => Some(4),
        Key::Key(5) => Some(5),
        Key::Key(6) => Some(6),
        Key::Key(7) => Some(7),
        _ => None,
    }
}

/// Calculate track X coordinate
#[must_use]
fn track_x(key: Key) -> Option<f32> {
    let start_x = (SCREEN_WIDTH - TOTAL_TRACKS_WIDTH) / 2.0;
    let index = key_to_index(key)?;
    Some(start_x + index as f32 * (TRACK_WIDTH + TRACK_SPACING))
}

/// Render tracks and judgment line
fn render_tracks() {
    let start_x = (SCREEN_WIDTH - TOTAL_TRACKS_WIDTH) / 2.0;

    for key in supported_keys() {
        let x = track_x(key).expect("supported keys should have valid track position");

        // Draw track lines
        macroquad::prelude::draw_line(x, 0.0, x, SCREEN_HEIGHT, 2.0, COLOR_TRACK_LINE);

        // Draw track right border
        macroquad::prelude::draw_line(
            x + TRACK_WIDTH,
            0.0,
            x + TRACK_WIDTH,
            SCREEN_HEIGHT,
            1.0,
            COLOR_TRACK_LINE,
        );
    }

    // Draw judgment line
    macroquad::prelude::draw_line(
        start_x - 10.0,
        JUDGMENT_LINE_Y,
        start_x + TOTAL_TRACKS_WIDTH + 10.0,
        JUDGMENT_LINE_Y,
        3.0,
        COLOR_JUDGMENT_LINE,
    );
}

/// Render visible notes
///
/// # Arguments
///
/// * `player` - `ChartPlayer` instance
fn render_notes(player: &mut ChartPlayer, _events: &[PlayheadEvent]) {
    let visible = player.visible_events();

    for (event, display_range) in visible {
        // Render measure lines
        if matches!(event.event(), ChartEvent::BarLine) {
            let ratio = display_range.start().value().as_f64();
            let y = JUDGMENT_LINE_Y - (ratio as f32 * JUDGMENT_LINE_Y);

            // Draw measure line across all tracks
            let start_x = (SCREEN_WIDTH - TOTAL_TRACKS_WIDTH) / 2.0;
            macroquad::prelude::draw_line(
                start_x,
                y,
                start_x + TOTAL_TRACKS_WIDTH,
                y,
                2.0,
                COLOR_BAR_LINE,
            );
            continue;
        }

        // Only render Note events
        if let ChartEvent::Note { key, kind, .. } = event.event() {
            // Filter non-7+1k keys
            let x = match track_x(*key) {
                Some(x) => x,
                None => continue,
            };

            // DisplayRatio: 0.0 = judgment line, 1.0 = visible area top
            // Convert to screen coordinates: Y = judgment line - (ratio * visible height)
            let ratio_start = display_range.start().value().as_f64();
            let ratio_end = display_range.end().value().as_f64();

            let y_start = JUDGMENT_LINE_Y - (ratio_start as f32 * JUDGMENT_LINE_Y);
            let y_end = JUDGMENT_LINE_Y - (ratio_end as f32 * JUDGMENT_LINE_Y);

            // Select color based on track type
            let color = match key {
                Key::Scratch(_) => COLOR_NOTE_SCRATCH,
                Key::Key(n) if n % 2 == 1 => COLOR_NOTE_WHITE,
                Key::Key(_) => COLOR_NOTE_BLUE,
                _ => COLOR_NOTE_WHITE,
            };

            // Draw notes
            match kind {
                NoteKind::Visible | NoteKind::Invisible => {
                    // Normal note (rectangle)
                    let note_height = 10.0;
                    macroquad::prelude::draw_rectangle(
                        x + 2.0,
                        y_start - note_height,
                        TRACK_WIDTH - 4.0,
                        note_height,
                        color,
                    );
                }
                NoteKind::Long => {
                    // Long note (rectangle from start to end)
                    let height = y_start - y_end;
                    macroquad::prelude::draw_rectangle(
                        x + 2.0,
                        y_end,
                        TRACK_WIDTH - 4.0,
                        height,
                        color,
                    );

                    // Long note head
                    macroquad::prelude::draw_rectangle(
                        x + 2.0,
                        y_start - 5.0,
                        TRACK_WIDTH - 4.0,
                        5.0,
                        color,
                    );
                }
                NoteKind::Landmine => {
                    // Landmine note (circle)
                    macroquad::prelude::draw_circle(
                        x + TRACK_WIDTH / 2.0,
                        y_start,
                        TRACK_WIDTH / 3.0,
                        COLOR_NOTE_MINE,
                    );
                }
            }
        }
    }
}

/// Render info text
///
/// # Arguments
///
/// * `player` - `ChartPlayer` instance
fn render_info(player: &ChartPlayer) {
    let state = player.playback_state();

    // Display BPM
    let bpm = state.current_bpm.as_f64();
    macroquad::prelude::draw_text(
        &format!("BPM: {:.1}", bpm),
        10.0,
        20.0,
        20.0,
        Color::from_rgba(255, 255, 255, 255),
    );

    // Display progress (current Y coordinate)
    let y = state.progressed_y().as_f64();
    macroquad::prelude::draw_text(
        &format!("Position: {:.2}", y),
        10.0,
        50.0,
        20.0,
        Color::from_rgba(255, 255, 255, 255),
    );

    // Display key hints
    macroquad::prelude::draw_text(
        "7+1k Layout: S | 1 | 2 | 3 | 4 | 5 | 6 | 7",
        10.0,
        SCREEN_HEIGHT - 30.0,
        16.0,
        Color::from_rgba(128, 128, 128, 255),
    );
}
