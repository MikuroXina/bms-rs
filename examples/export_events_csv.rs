//! Parse BMS/BMSON files and export `ChartEvent` to CSV format
//!
//! Usage:
//!   cargo run --example `export_events_csv` -- <`file_path`>
//!
//! Example:
//!   cargo run --example `export_events_csv` -- `tests/bms/files/lilith_mx.bms`

use std::env;

use bms_rs::bms::{default_config, parse_bms};
use bms_rs::chart_process::prelude::*;

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Convert `TimeSpan` to seconds (floating point)
fn timespan_to_seconds(ts: gametime::TimeSpan) -> f64 {
    ts.as_nanos() as f64 / NANOS_PER_SECOND as f64
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    let program_name = args.first().map(std::string::String::as_str).unwrap_or("export_events_csv");
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
    if is_bmson {
        // Parse BMSON file
        #[cfg(feature = "bmson")]
        {
            let output = bms_rs::bmson::parse_bmson(&source);
            let bmson = output.bmson.ok_or("BMSON parsing failed")?;
            let processor = BmsonProcessor::new(&bmson);
            export_events_to_csv(processor.all_events(), processor.init_bpm())?;
        }

        #[cfg(not(feature = "bmson"))]
        {
            eprintln!("Error: BMSON support requires 'bmson' feature");
            eprintln!(
                "Please use: cargo run --example export_events_csv --features bmson -- <file_path>"
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
        export_events_to_csv(processor.all_events(), processor.init_bpm())?;
    }

    Ok(())
}

/// Export `AllEventsIndex` to CSV format
fn export_events_to_csv(
    all_events: &AllEventsIndex,
    init_bpm: &bms_rs::bms::Decimal,
) -> Result<(), Box<dyn std::error::Error>> {
    let stdout = std::io::stdout();
    let writer = stdout.lock();
    let mut csv_writer = csv::Writer::from_writer(writer);

    // Write CSV header
    csv_writer.write_record([
        "event_id",
        "event_type",
        "y_coordinate",
        "activate_time_sec",
        "side",
        "key",
        "kind",
        "wav_id",
        "length",
        "continue_play_sec",
        "bpm",
        "scroll_factor",
        "speed_factor",
        "stop_duration",
        "bga_layer",
        "bmp_id",
        "bga_opacity",
        "bga_argb",
        "bgm_volume",
        "key_volume",
        "text",
        "judge_level",
        "video_seek",
        "bga_keybound",
        "option",
    ])?;

    // Get all events and output in order
    let all_events_vec: Vec<PlayheadEvent> = all_events.events_in_y_range(..);

    for event_data in all_events_vec {
        let mut record = Vec::with_capacity(25);

        // Basic information
        record.push(event_data.id().value().to_string());
        record.push(event_type_name(&event_data.event).to_string());
        record.push(format!("{:.6}", event_data.position.value()));
        record.push(format!(
            "{:.6}",
            timespan_to_seconds(event_data.activate_time)
        ));

        // Fill fields based on event type
        match &event_data.event {
            ChartEvent::Note {
                side,
                key,
                kind,
                wav_id,
                length,
                continue_play,
            } => {
                record.push(format!("{:?}", side));
                record.push(format!("{:?}", key));
                record.push(format!("{:?}", kind));
                record.push(wav_id.map(|id| id.value().to_string()).unwrap_or_default());
                record.push(
                    length
                        .as_ref()
                        .map(|y| format!("{:.6}", y.value()))
                        .unwrap_or_default(),
                );
                record.push(
                    continue_play
                        .map(|s| timespan_to_seconds(s).to_string())
                        .unwrap_or_default(),
                );
                // Fill remaining fields with empty strings (25 - 4 - 6 = 15)
                for _ in 0..15 {
                    record.push(String::new());
                }
            }
            ChartEvent::Bgm { wav_id } => {
                // Fill empty fields (25 - 4 - 1 = 20)
                for _ in 0..19 {
                    record.push(String::new());
                }
                record.push(wav_id.map(|id| id.value().to_string()).unwrap_or_default());
                for _ in 0..1 {
                    record.push(String::new());
                }
            }
            ChartEvent::BpmChange { bpm } => {
                // Fill empty fields (25 - 4 - 1 = 20)
                for _ in 0..19 {
                    record.push(String::new());
                }
                record.push(format!("{:.6}", bpm));
            }
            ChartEvent::ScrollChange { factor } => {
                // Fill empty fields (25 - 4 - 1 = 20)
                for _ in 0..20 {
                    record.push(String::new());
                }
                record.push(format!("{:.6}", factor));
            }
            ChartEvent::SpeedChange { factor } => {
                // Fill empty fields (25 - 4 - 1 = 20)
                for _ in 0..21 {
                    record.push(String::new());
                }
                record.push(format!("{:.6}", factor));
            }
            ChartEvent::Stop { duration } => {
                // Fill empty fields (25 - 4 - 1 = 20)
                for _ in 0..22 {
                    record.push(String::new());
                }
                record.push(format!("{:.6}", duration));
            }
            ChartEvent::BgaChange { layer, bmp_id } => {
                // Fill empty fields (25 - 4 - 2 = 19)
                for _ in 0..19 {
                    record.push(String::new());
                }
                record.push(format!("{:?}", layer));
                record.push(bmp_id.map(|id| id.value().to_string()).unwrap_or_default());
            }
            ChartEvent::BgaOpacityChange { layer, opacity } => {
                // Fill empty fields (25 - 4 - 2 = 19)
                for _ in 0..19 {
                    record.push(String::new());
                }
                record.push(format!("{:?}", layer));
                record.push(String::new());
                record.push(opacity.to_string());
            }
            ChartEvent::BgaArgbChange { layer, argb } => {
                // Fill empty fields (25 - 4 - 2 = 19)
                for _ in 0..19 {
                    record.push(String::new());
                }
                record.push(format!("{:?}", layer));
                record.push(String::new());
                record.push(String::new());
                record.push(format!(
                    "{:02x}{:02x}{:02x}{:02x}",
                    argb.alpha, argb.red, argb.green, argb.blue
                ));
            }
            ChartEvent::BgmVolumeChange { volume } => {
                // Fill empty fields (25 - 4 - 1 = 20)
                for _ in 0..24 {
                    record.push(String::new());
                }
                record.push(volume.to_string());
            }
            ChartEvent::KeyVolumeChange { volume } => {
                // Fill empty fields (25 - 4 - 1 = 20)
                for _ in 0..25 {
                    record.push(String::new());
                }
                record.push(volume.to_string());
            }
            ChartEvent::TextDisplay { text } => {
                // Fill empty fields (25 - 4 - 1 = 20)
                for _ in 0..26 {
                    record.push(String::new());
                }
                record.push(text.clone());
            }
            ChartEvent::JudgeLevelChange { level } => {
                // Fill empty fields (25 - 4 - 1 = 20)
                for _ in 0..27 {
                    record.push(String::new());
                }
                record.push(format!("{:?}", level));
            }
            ChartEvent::VideoSeek { seek_time } => {
                // Fill empty fields (25 - 4 - 1 = 20)
                for _ in 0..28 {
                    record.push(String::new());
                }
                record.push(seek_time.to_string());
            }
            ChartEvent::BgaKeybound { event } => {
                // Fill empty fields (25 - 4 - 1 = 20)
                for _ in 0..29 {
                    record.push(String::new());
                }
                record.push(format!("{:?}", event));
            }
            ChartEvent::OptionChange { option } => {
                // Fill empty fields (25 - 4 - 1 = 20)
                for _ in 0..30 {
                    record.push(String::new());
                }
                record.push(option.clone());
            }
            ChartEvent::BarLine => {
                // Fill empty fields (25 - 4 = 21)
                for _ in 0..21 {
                    record.push(String::new());
                }
            }
        }

        csv_writer.write_record(&record)?;
    }

    // Write initial BPM info to stderr
    eprintln!("# Initial BPM: {}", init_bpm);
    eprintln!("# Total events: {}", all_events.events_in_y_range(..).len());

    csv_writer.flush()?;

    Ok(())
}

/// Get the name of event type
const fn event_type_name(event: &ChartEvent) -> &'static str {
    match event {
        ChartEvent::Note { .. } => "Note",
        ChartEvent::Bgm { .. } => "Bgm",
        ChartEvent::BpmChange { .. } => "BpmChange",
        ChartEvent::ScrollChange { .. } => "ScrollChange",
        ChartEvent::SpeedChange { .. } => "SpeedChange",
        ChartEvent::Stop { .. } => "Stop",
        ChartEvent::BgaChange { .. } => "BgaChange",
        ChartEvent::BgaOpacityChange { .. } => "BgaOpacityChange",
        ChartEvent::BgaArgbChange { .. } => "BgaArgbChange",
        ChartEvent::BgmVolumeChange { .. } => "BgmVolumeChange",
        ChartEvent::KeyVolumeChange { .. } => "KeyVolumeChange",
        ChartEvent::TextDisplay { .. } => "TextDisplay",
        ChartEvent::JudgeLevelChange { .. } => "JudgeLevelChange",
        ChartEvent::VideoSeek { .. } => "VideoSeek",
        ChartEvent::BgaKeybound { .. } => "BgaKeybound",
        ChartEvent::OptionChange { .. } => "OptionChange",
        ChartEvent::BarLine => "BarLine",
    }
}
