//! Parse BMS/BMSON files and export `ChartEvent` to TOML format, grouped by y-coordinate
//!
//! Usage:
//!   cargo run --example `export_events_toml` -- <`file_path`>
//!
//! Example:
//!   cargo run --example `export_events_toml` -- `tests/bms/files/lilith_mx.bms`

use std::collections::BTreeMap;
use std::env;

use bms_rs::bms::{default_config, parse_bms};
use bms_rs::chart_process::prelude::*;
use serde::Serialize;

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Convert `TimeSpan` to seconds (floating point)
fn timespan_to_seconds(ts: gametime::TimeSpan) -> f64 {
    ts.as_nanos() as f64 / NANOS_PER_SECOND as f64
}

#[derive(Debug, Clone, Serialize)]
struct NoteEvent {
    side: String,
    key: String,
    kind: String,
    wav_id: Option<String>,
    length: Option<String>,
    continue_play_sec: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct BgmEvent {
    wav_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct BpmChangeEvent {
    bpm: String,
}

#[derive(Debug, Clone, Serialize)]
struct ScrollChangeEvent {
    factor: String,
}

#[derive(Debug, Clone, Serialize)]
struct SpeedChangeEvent {
    factor: String,
}

#[derive(Debug, Clone, Serialize)]
struct StopEvent {
    duration: String,
}

#[derive(Debug, Clone, Serialize)]
struct BgaChangeEvent {
    layer: String,
    bmp_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct BgaOpacityChangeEvent {
    layer: String,
    opacity: String,
}

#[derive(Debug, Clone, Serialize)]
struct BgaArgbChangeEvent {
    layer: String,
    argb: String,
}

#[derive(Debug, Clone, Serialize)]
struct VolumeChangeEvent {
    volume: String,
}

#[derive(Debug, Clone, Serialize)]
struct TextDisplayEvent {
    text: String,
}

#[derive(Debug, Clone, Serialize)]
struct JudgeLevelChangeEvent {
    level: String,
}

#[derive(Debug, Clone, Serialize)]
struct VideoSeekEvent {
    seek_time: String,
}

#[derive(Debug, Clone, Serialize)]
struct BgaKeyboundEvent {
    event: String,
}

#[derive(Debug, Clone, Serialize)]
struct OptionChangeEvent {
    option: String,
}

#[derive(Debug, Clone, Serialize)]
struct EventsAtY {
    y_coordinate: String,
    activate_time_sec: String,
    notes: Vec<NoteEvent>,
    bgms: Vec<BgmEvent>,
    bpm_changes: Vec<BpmChangeEvent>,
    scroll_changes: Vec<ScrollChangeEvent>,
    speed_changes: Vec<SpeedChangeEvent>,
    stops: Vec<StopEvent>,
    bga_changes: Vec<BgaChangeEvent>,
    bga_opacity_changes: Vec<BgaOpacityChangeEvent>,
    bga_argb_changes: Vec<BgaArgbChangeEvent>,
    bgm_volume_changes: Vec<VolumeChangeEvent>,
    key_volume_changes: Vec<VolumeChangeEvent>,
    text_displays: Vec<TextDisplayEvent>,
    judge_level_changes: Vec<JudgeLevelChangeEvent>,
    video_seeks: Vec<VideoSeekEvent>,
    bga_keybounds: Vec<BgaKeyboundEvent>,
    option_changes: Vec<OptionChangeEvent>,
    bar_lines: u32,
}

#[derive(Debug, Serialize)]
struct TomlOutput {
    initial_bpm: String,
    total_events: usize,
    #[serde(serialize_with = "serialize_ordered_map")]
    events_by_y: BTreeMap<String, EventsAtY>,
}

/// Helper function to serialize `BTreeMap` with ordered keys
fn serialize_ordered_map<S>(
    map: &BTreeMap<String, EventsAtY>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeMap;
    let mut map_serializer = serializer.serialize_map(Some(map.len()))?;
    for (key, value) in map {
        map_serializer.serialize_entry(key, value)?;
    }
    map_serializer.end()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    let program_name = args.first().map(std::string::String::as_str).unwrap_or("export_events_toml");
    if args.len() < 2 {
        eprintln!("Usage: {} <bms/bmson_file_path>", program_name);
        eprintln!("Example: {} tests/bms/files/lilith_mx.bms", program_name);
        std::process::exit(1);
    }

    let file_path = args.get(1).expect("args[1] should exist after length check");
    let source = std::fs::read_to_string(file_path)?;

    // Determine file type by extension
    let is_bmson =
        file_path.to_lowercase().ends_with(".bmson") || file_path.to_lowercase().ends_with(".json");

    // Parse file and export
    let (all_events, init_bpm) = if is_bmson {
        // Parse BMSON file
        #[cfg(feature = "bmson")]
        {
            let output = bms_rs::bmson::parse_bmson(&source);
            let bmson = output.bmson.ok_or("BMSON parsing failed")?;
            let processor = BmsonProcessor::new(&bmson);
            (processor.all_events().clone(), processor.init_bpm().clone())
        }

        #[cfg(not(feature = "bmson"))]
        {
            eprintln!("Error: BMSON support requires 'bmson' feature");
            eprintln!(
                "Please use: cargo run --example export_events_toml --features bmson -- <file_path>"
            );
            std::process::exit(1);
        }
    } else {
        // Parse BMS file
        let output = parse_bms(&source, default_config());
        let bms = output
            .bms
            .map_err(|e| format!("BMS parsing failed: {:?}", e))?;
        let processor =
            BmsProcessor::<bms_rs::bms::command::channel::mapper::KeyLayoutBeat>::new(&bms);
        (processor.all_events().clone(), processor.init_bpm().clone())
    };

    export_events_to_toml(&all_events, &init_bpm)?;

    Ok(())
}

/// Export `AllEventsIndex` to TOML format, grouped by y-coordinate
fn export_events_to_toml(
    all_events: &AllEventsIndex,
    init_bpm: &bms_rs::bms::Decimal,
) -> Result<(), Box<dyn std::error::Error>> {
    let all_events_vec = all_events.events_in_y_range(..);

    // Group events by y-coordinate
    let mut events_by_y: BTreeMap<String, EventsAtY> = BTreeMap::new();

    for event_data in &all_events_vec {
        let y_key = format!("{:.6}", event_data.position.value());
        let activate_time = format!("{:.6}", timespan_to_seconds(event_data.activate_time));

        let entry = events_by_y.entry(y_key.clone()).or_insert(EventsAtY {
            y_coordinate: y_key,
            activate_time_sec: activate_time,
            notes: Vec::new(),
            bgms: Vec::new(),
            bpm_changes: Vec::new(),
            scroll_changes: Vec::new(),
            speed_changes: Vec::new(),
            stops: Vec::new(),
            bga_changes: Vec::new(),
            bga_opacity_changes: Vec::new(),
            bga_argb_changes: Vec::new(),
            bgm_volume_changes: Vec::new(),
            key_volume_changes: Vec::new(),
            text_displays: Vec::new(),
            judge_level_changes: Vec::new(),
            video_seeks: Vec::new(),
            bga_keybounds: Vec::new(),
            option_changes: Vec::new(),
            bar_lines: 0,
        });

        match &event_data.event {
            ChartEvent::Note {
                side,
                key,
                kind,
                wav_id,
                length,
                continue_play,
            } => {
                entry.notes.push(NoteEvent {
                    side: format!("{:?}", side),
                    key: format!("{:?}", key),
                    kind: format!("{:?}", kind),
                    wav_id: wav_id.map(|id| id.value().to_string()),
                    length: length.as_ref().map(|y| format!("{:.6}", y.value())),
                    continue_play_sec: continue_play.map(|s| timespan_to_seconds(s).to_string()),
                });
            }
            ChartEvent::Bgm { wav_id } => {
                entry.bgms.push(BgmEvent {
                    wav_id: wav_id.map(|id| id.value().to_string()),
                });
            }
            ChartEvent::BpmChange { bpm } => {
                entry.bpm_changes.push(BpmChangeEvent {
                    bpm: format!("{:.6}", bpm),
                });
            }
            ChartEvent::ScrollChange { factor } => {
                entry.scroll_changes.push(ScrollChangeEvent {
                    factor: format!("{:.6}", factor),
                });
            }
            ChartEvent::SpeedChange { factor } => {
                entry.speed_changes.push(SpeedChangeEvent {
                    factor: format!("{:.6}", factor),
                });
            }
            ChartEvent::Stop { duration } => {
                entry.stops.push(StopEvent {
                    duration: format!("{:.6}", duration),
                });
            }
            ChartEvent::BgaChange { layer, bmp_id } => {
                entry.bga_changes.push(BgaChangeEvent {
                    layer: format!("{:?}", layer),
                    bmp_id: bmp_id.map(|id| id.value().to_string()),
                });
            }
            ChartEvent::BgaOpacityChange { layer, opacity } => {
                entry.bga_opacity_changes.push(BgaOpacityChangeEvent {
                    layer: format!("{:?}", layer),
                    opacity: opacity.to_string(),
                });
            }
            ChartEvent::BgaArgbChange { layer, argb } => {
                entry.bga_argb_changes.push(BgaArgbChangeEvent {
                    layer: format!("{:?}", layer),
                    argb: format!(
                        "{:02x}{:02x}{:02x}{:02x}",
                        argb.alpha, argb.red, argb.green, argb.blue
                    ),
                });
            }
            ChartEvent::BgmVolumeChange { volume } => {
                entry.bgm_volume_changes.push(VolumeChangeEvent {
                    volume: volume.to_string(),
                });
            }
            ChartEvent::KeyVolumeChange { volume } => {
                entry.key_volume_changes.push(VolumeChangeEvent {
                    volume: volume.to_string(),
                });
            }
            ChartEvent::TextDisplay { text } => {
                entry
                    .text_displays
                    .push(TextDisplayEvent { text: text.clone() });
            }
            ChartEvent::JudgeLevelChange { level } => {
                entry.judge_level_changes.push(JudgeLevelChangeEvent {
                    level: format!("{:?}", level),
                });
            }
            ChartEvent::VideoSeek { seek_time } => {
                entry.video_seeks.push(VideoSeekEvent {
                    seek_time: seek_time.to_string(),
                });
            }
            ChartEvent::BgaKeybound { event } => {
                entry.bga_keybounds.push(BgaKeyboundEvent {
                    event: format!("{:?}", event),
                });
            }
            ChartEvent::OptionChange { option } => {
                entry.option_changes.push(OptionChangeEvent {
                    option: option.clone(),
                });
            }
            ChartEvent::BarLine => {
                entry.bar_lines += 1;
            }
        }
    }

    // Create TOML output structure
    let output = TomlOutput {
        initial_bpm: format!("{:.6}", init_bpm),
        total_events: all_events_vec.len(),
        events_by_y,
    };

    // Serialize to TOML
    let toml_string = toml::to_string_pretty(&output)?;

    // Print to stdout
    println!("{}", toml_string);

    // Write summary to stderr
    eprintln!("# Initial BPM: {}", init_bpm);
    eprintln!("# Total events: {}", all_events_vec.len());
    eprintln!("# Unique y-coordinates: {}", output.events_by_y.len());

    Ok(())
}
