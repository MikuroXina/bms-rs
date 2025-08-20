//! Fault-tolerant JSON parsing for BMSON format.
//!
//! This module provides a JSON parser that can handle malformed JSON input
//! by issuing warnings and continuing to parse with default values.

use serde_json::{Map, Value};

use super::{Bmson, BmsonInfo, BmsonOutput, BmsonWarning, fin_f64::FinF64, pulse::PulseNumber};

/// Parse a BMSON file from source text with fault tolerance.
///
/// This function uses serde_json for parsing but adds fault tolerance by
/// handling missing or invalid fields with warnings and default values.
pub fn parse_bmson(source: &str) -> BmsonOutput {
    let mut warnings = Vec::new();

    // Try to parse as JSON first
    let json_value = match serde_json::from_str::<Value>(source) {
        Ok(value) => value,
        Err(e) => {
            warnings.push(BmsonWarning::JsonParsing(format!(
                "JSON parsing failed: {}",
                e
            )));
            // Return default BMSON
            return BmsonOutput {
                bmson: create_default_bmson(&mut warnings),
                warnings,
            };
        }
    };

    // Parse the JSON value into BMSON
    match parse_bmson_from_value(json_value, &mut warnings) {
        Ok(bmson) => BmsonOutput { bmson, warnings },
        Err(_) => {
            warnings.push(BmsonWarning::JsonParsing(
                "Failed to parse BMSON structure, using defaults".to_string(),
            ));
            BmsonOutput {
                bmson: create_default_bmson(&mut warnings),
                warnings,
            }
        }
    }
}

/// Parse BMSON from a JSON Value with fault tolerance.
fn parse_bmson_from_value(value: Value, warnings: &mut Vec<BmsonWarning>) -> Result<Bmson, ()> {
    let obj = value.as_object().ok_or(())?;

    // Parse version
    let version = get_string_field(obj, "version", "1.0.0", warnings);

    // Parse info
    let info = if let Some(info_value) = obj.get("info") {
        parse_bmson_info(info_value, warnings)
    } else {
        warnings.push(BmsonWarning::MissingRequiredField("info".to_string()));
        create_default_bmson_info(warnings)
    };

    // Parse other fields with defaults
    let lines = parse_lines(obj.get("lines"), warnings);
    let bpm_events = parse_bpm_events(obj.get("bpm_events"), warnings);
    let stop_events = parse_stop_events(obj.get("stop_events"), warnings);
    let sound_channels = parse_sound_channels(obj.get("sound_channels"), warnings);
    let bga = parse_bga(obj.get("bga"), warnings);
    let scroll_events = parse_scroll_events(obj.get("scroll_events"), warnings);
    let mine_channels = parse_mine_channels(obj.get("mine_channels"), warnings);
    let key_channels = parse_key_channels(obj.get("key_channels"), warnings);

    Ok(Bmson {
        version,
        info,
        lines,
        bpm_events,
        stop_events,
        sound_channels,
        bga,
        scroll_events,
        mine_channels,
        key_channels,
    })
}

/// Parse BMSON info from JSON value.
fn parse_bmson_info(value: &Value, warnings: &mut Vec<BmsonWarning>) -> BmsonInfo {
    let empty_map = Map::new();
    let obj = value.as_object().unwrap_or(&empty_map);

    let title = get_string_field(obj, "title", "Unknown", warnings);
    let subtitle = get_string_field(obj, "subtitle", "", warnings);
    let artist = get_string_field(obj, "artist", "Unknown", warnings);
    let genre = get_string_field(obj, "genre", "Unknown", warnings);
    let mode_hint = get_string_field(obj, "mode_hint", "beat-7k", warnings);
    let chart_name = get_string_field(obj, "chart_name", "", warnings);

    let level = get_u32_field(obj, "level", 1, warnings);
    let init_bpm = get_f64_field(obj, "init_bpm", 120.0, warnings);
    let judge_rank = get_f64_field(obj, "judge_rank", 100.0, warnings);
    let total = get_f64_field(obj, "total", 100.0, warnings);
    let resolution = get_u64_field(obj, "resolution", 240, warnings);

    let back_image = get_optional_string_field(obj, "back_image", warnings);
    let eyecatch_image = get_optional_string_field(obj, "eyecatch_image", warnings);
    let title_image = get_optional_string_field(obj, "title_image", warnings);
    let banner_image = get_optional_string_field(obj, "banner_image", warnings);
    let preview_music = get_optional_string_field(obj, "preview_music", warnings);

    let subartists = if let Some(subartists_value) = obj.get("subartists") {
        parse_string_array(subartists_value, warnings)
    } else {
        Vec::new()
    };

    let ln_type = if let Some(ln_type_value) = obj.get("ln_type") {
        parse_ln_mode(ln_type_value, warnings)
    } else {
        crate::bms::command::LnMode::Ln
    };

    BmsonInfo {
        title,
        subtitle,
        artist,
        subartists,
        genre,
        mode_hint,
        chart_name,
        level,
        init_bpm: FinF64::new(init_bpm).unwrap_or_else(|| {
            warnings.push(BmsonWarning::InvalidFieldValue(format!(
                "Invalid init_bpm value: {}, using default 120.0",
                init_bpm
            )));
            FinF64::new(120.0).unwrap()
        }),
        judge_rank: FinF64::new(judge_rank).unwrap_or_else(|| {
            warnings.push(BmsonWarning::InvalidFieldValue(format!(
                "Invalid judge_rank value: {}, using default 100.0",
                judge_rank
            )));
            FinF64::new(100.0).unwrap()
        }),
        total: FinF64::new(total).unwrap_or_else(|| {
            warnings.push(BmsonWarning::InvalidFieldValue(format!(
                "Invalid total value: {}, using default 100.0",
                total
            )));
            FinF64::new(100.0).unwrap()
        }),
        back_image,
        eyecatch_image,
        title_image,
        banner_image,
        preview_music,
        resolution,
        ln_type,
    }
}

/// Helper function to get a string field with default value.
fn get_string_field(
    obj: &Map<String, Value>,
    key: &str,
    default: &str,
    warnings: &mut Vec<BmsonWarning>,
) -> String {
    if let Some(value) = obj.get(key) {
        if let Some(s) = value.as_str() {
            s.to_string()
        } else {
            warnings.push(BmsonWarning::InvalidFieldType(format!(
                "Field '{}' should be a string, using default '{}'",
                key, default
            )));
            default.to_string()
        }
    } else {
        warnings.push(BmsonWarning::MissingRequiredField(key.to_string()));
        default.to_string()
    }
}

/// Helper function to get an optional string field.
fn get_optional_string_field(
    obj: &Map<String, Value>,
    key: &str,
    warnings: &mut Vec<BmsonWarning>,
) -> Option<String> {
    if let Some(value) = obj.get(key) {
        if value.is_null() {
            None
        } else if let Some(s) = value.as_str() {
            Some(s.to_string())
        } else {
            warnings.push(BmsonWarning::InvalidFieldType(format!(
                "Field '{}' should be a string or null",
                key
            )));
            None
        }
    } else {
        None
    }
}

/// Helper function to get a u32 field with default value.
fn get_u32_field(
    obj: &Map<String, Value>,
    key: &str,
    default: u32,
    warnings: &mut Vec<BmsonWarning>,
) -> u32 {
    if let Some(value) = obj.get(key) {
        if let Some(n) = value.as_u64() {
            if n <= u32::MAX as u64 {
                n as u32
            } else {
                warnings.push(BmsonWarning::InvalidFieldValue(format!(
                    "Field '{}' value {} is too large, using default {}",
                    key, n, default
                )));
                default
            }
        } else {
            warnings.push(BmsonWarning::InvalidFieldType(format!(
                "Field '{}' should be a number, using default {}",
                key, default
            )));
            default
        }
    } else {
        warnings.push(BmsonWarning::MissingRequiredField(key.to_string()));
        default
    }
}

/// Helper function to get a u64 field with default value.
fn get_u64_field(
    obj: &Map<String, Value>,
    key: &str,
    default: u64,
    warnings: &mut Vec<BmsonWarning>,
) -> u64 {
    if let Some(value) = obj.get(key) {
        if let Some(n) = value.as_u64() {
            n
        } else {
            warnings.push(BmsonWarning::InvalidFieldType(format!(
                "Field '{}' should be a number, using default {}",
                key, default
            )));
            default
        }
    } else {
        warnings.push(BmsonWarning::MissingRequiredField(key.to_string()));
        default
    }
}

/// Helper function to get a f64 field with default value.
fn get_f64_field(
    obj: &Map<String, Value>,
    key: &str,
    default: f64,
    warnings: &mut Vec<BmsonWarning>,
) -> f64 {
    if let Some(value) = obj.get(key) {
        if let Some(n) = value.as_f64() {
            // Check if the value is finite (not NaN or infinity)
            if n.is_finite() {
                n
            } else {
                warnings.push(BmsonWarning::InvalidFieldValue(format!(
                    "Field '{}' value {} is not finite, using default {}",
                    key, n, default
                )));
                default
            }
        } else {
            warnings.push(BmsonWarning::InvalidFieldType(format!(
                "Field '{}' should be a number, using default {}",
                key, default
            )));
            default
        }
    } else {
        warnings.push(BmsonWarning::MissingRequiredField(key.to_string()));
        default
    }
}

/// Parse a string array from JSON value.
fn parse_string_array(value: &Value, warnings: &mut Vec<BmsonWarning>) -> Vec<String> {
    if let Some(array) = value.as_array() {
        let mut result = Vec::new();
        for item in array {
            if let Some(s) = item.as_str() {
                result.push(s.to_string());
            } else {
                warnings.push(BmsonWarning::InvalidFieldType(
                    "Array item should be a string".to_string(),
                ));
            }
        }
        result
    } else {
        warnings.push(BmsonWarning::InvalidFieldType(
            "Field should be an array".to_string(),
        ));
        Vec::new()
    }
}

/// Parse LnMode from JSON value.
fn parse_ln_mode(value: &Value, warnings: &mut Vec<BmsonWarning>) -> crate::bms::command::LnMode {
    if let Some(s) = value.as_str() {
        match s {
            "LN" => crate::bms::command::LnMode::Ln,
            "CN" => crate::bms::command::LnMode::Cn,
            "HCN" => crate::bms::command::LnMode::Hcn,
            _ => {
                warnings.push(BmsonWarning::InvalidFieldValue(format!(
                    "Invalid ln_type value: '{}', using default LN",
                    s
                )));
                crate::bms::command::LnMode::Ln
            }
        }
    } else {
        warnings.push(BmsonWarning::InvalidFieldType(
            "ln_type should be a string".to_string(),
        ));
        crate::bms::command::LnMode::Ln
    }
}

/// Parse lines from JSON value.
fn parse_lines(
    value: Option<&Value>,
    warnings: &mut Vec<BmsonWarning>,
) -> Option<Vec<super::BarLine>> {
    if let Some(value) = value {
        if value.is_null() {
            None
        } else if let Some(array) = value.as_array() {
            let mut lines = Vec::new();
            for item in array {
                if let Some(obj) = item.as_object() {
                    let y = get_u64_field(obj, "y", 0, warnings);
                    lines.push(super::BarLine { y: PulseNumber(y) });
                } else {
                    warnings.push(BmsonWarning::InvalidFieldType(
                        "Bar line should be an object".to_string(),
                    ));
                }
            }
            Some(lines)
        } else {
            warnings.push(BmsonWarning::InvalidFieldType(
                "lines should be an array or null".to_string(),
            ));
            None
        }
    } else {
        None
    }
}

/// Parse BPM events from JSON value.
fn parse_bpm_events(
    value: Option<&Value>,
    warnings: &mut Vec<BmsonWarning>,
) -> Vec<super::BpmEvent> {
    if let Some(value) = value {
        if let Some(array) = value.as_array() {
            let mut events = Vec::new();
            for item in array {
                if let Some(obj) = item.as_object() {
                    let y = get_u64_field(obj, "y", 0, warnings);
                    let bpm = get_f64_field(obj, "bpm", 120.0, warnings);
                    events.push(super::BpmEvent {
                        y: PulseNumber(y),
                        bpm: FinF64::new(bpm).unwrap_or_else(|| {
                            warnings.push(BmsonWarning::InvalidFieldValue(format!(
                                "Invalid BPM value: {}, using default 120.0",
                                bpm
                            )));
                            FinF64::new(120.0).unwrap()
                        }),
                    });
                } else {
                    warnings.push(BmsonWarning::InvalidFieldType(
                        "BPM event should be an object".to_string(),
                    ));
                }
            }
            events
        } else {
            warnings.push(BmsonWarning::InvalidFieldType(
                "bpm_events should be an array".to_string(),
            ));
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

/// Parse stop events from JSON value.
fn parse_stop_events(
    value: Option<&Value>,
    warnings: &mut Vec<BmsonWarning>,
) -> Vec<super::StopEvent> {
    if let Some(value) = value {
        if let Some(array) = value.as_array() {
            let mut events = Vec::new();
            for item in array {
                if let Some(obj) = item.as_object() {
                    let y = get_u64_field(obj, "y", 0, warnings);
                    let duration = get_u64_field(obj, "duration", 0, warnings);
                    events.push(super::StopEvent {
                        y: PulseNumber(y),
                        duration,
                    });
                } else {
                    warnings.push(BmsonWarning::InvalidFieldType(
                        "Stop event should be an object".to_string(),
                    ));
                }
            }
            events
        } else {
            warnings.push(BmsonWarning::InvalidFieldType(
                "stop_events should be an array".to_string(),
            ));
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

/// Parse sound channels from JSON value.
fn parse_sound_channels(
    value: Option<&Value>,
    warnings: &mut Vec<BmsonWarning>,
) -> Vec<super::SoundChannel> {
    if let Some(value) = value {
        if let Some(array) = value.as_array() {
            let mut channels = Vec::new();
            for item in array {
                if let Some(obj) = item.as_object() {
                    let name = get_string_field(obj, "name", "", warnings);
                    let notes = if let Some(notes_value) = obj.get("notes") {
                        parse_notes(notes_value, warnings)
                    } else {
                        Vec::new()
                    };
                    channels.push(super::SoundChannel { name, notes });
                } else {
                    warnings.push(BmsonWarning::InvalidFieldType(
                        "Sound channel should be an object".to_string(),
                    ));
                }
            }
            channels
        } else {
            warnings.push(BmsonWarning::InvalidFieldType(
                "sound_channels should be an array".to_string(),
            ));
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

/// Parse notes from JSON value.
fn parse_notes(value: &Value, warnings: &mut Vec<BmsonWarning>) -> Vec<super::Note> {
    if let Some(array) = value.as_array() {
        let mut notes = Vec::new();
        for item in array {
            if let Some(obj) = item.as_object() {
                let y = get_u64_field(obj, "y", 0, warnings);
                let x = if let Some(x_value) = obj.get("x") {
                    if x_value.is_null() {
                        None
                    } else if let Some(n) = x_value.as_u64() {
                        if n == 0 {
                            None
                        } else if n <= u8::MAX as u64 {
                            std::num::NonZeroU8::new(n as u8)
                        } else {
                            warnings.push(BmsonWarning::InvalidFieldValue(format!(
                                "Note x value {} is too large",
                                n
                            )));
                            None
                        }
                    } else {
                        warnings.push(BmsonWarning::InvalidFieldType(
                            "Note x should be a number or null".to_string(),
                        ));
                        None
                    }
                } else {
                    None
                };
                let l = get_u64_field(obj, "l", 0, warnings);
                let c = if let Some(c_value) = obj.get("c") {
                    c_value.as_bool().unwrap_or(false)
                } else {
                    false
                };
                let t = obj.get("t").map(|t_value| parse_ln_mode(t_value, warnings));
                let up = if let Some(up_value) = obj.get("up") {
                    up_value.as_bool()
                } else {
                    None
                };
                notes.push(super::Note {
                    y: PulseNumber(y),
                    x,
                    l,
                    c,
                    t,
                    up,
                });
            } else {
                warnings.push(BmsonWarning::InvalidFieldType(
                    "Note should be an object".to_string(),
                ));
            }
        }
        notes
    } else {
        warnings.push(BmsonWarning::InvalidFieldType(
            "Notes should be an array".to_string(),
        ));
        Vec::new()
    }
}

/// Parse BGA from JSON value.
fn parse_bga(value: Option<&Value>, warnings: &mut Vec<BmsonWarning>) -> super::Bga {
    if let Some(value) = value {
        if let Some(obj) = value.as_object() {
            let bga_header = if let Some(header_value) = obj.get("bga_header") {
                parse_bga_headers(header_value, warnings)
            } else {
                Vec::new()
            };
            let bga_events = if let Some(events_value) = obj.get("bga_events") {
                parse_bga_events(events_value, warnings)
            } else {
                Vec::new()
            };
            let layer_events = if let Some(events_value) = obj.get("layer_events") {
                parse_bga_events(events_value, warnings)
            } else {
                Vec::new()
            };
            let poor_events = if let Some(events_value) = obj.get("poor_events") {
                parse_bga_events(events_value, warnings)
            } else {
                Vec::new()
            };
            super::Bga {
                bga_header,
                bga_events,
                layer_events,
                poor_events,
            }
        } else {
            warnings.push(BmsonWarning::InvalidFieldType(
                "BGA should be an object".to_string(),
            ));
            super::Bga::default()
        }
    } else {
        super::Bga::default()
    }
}

/// Parse BGA headers from JSON value.
fn parse_bga_headers(value: &Value, warnings: &mut Vec<BmsonWarning>) -> Vec<super::BgaHeader> {
    if let Some(array) = value.as_array() {
        let mut headers = Vec::new();
        for item in array {
            if let Some(obj) = item.as_object() {
                let id = get_u32_field(obj, "id", 0, warnings);
                let name = get_string_field(obj, "name", "", warnings);
                headers.push(super::BgaHeader {
                    id: super::BgaId(id),
                    name,
                });
            } else {
                warnings.push(BmsonWarning::InvalidFieldType(
                    "BGA header should be an object".to_string(),
                ));
            }
        }
        headers
    } else {
        warnings.push(BmsonWarning::InvalidFieldType(
            "BGA headers should be an array".to_string(),
        ));
        Vec::new()
    }
}

/// Parse BGA events from JSON value.
fn parse_bga_events(value: &Value, warnings: &mut Vec<BmsonWarning>) -> Vec<super::BgaEvent> {
    if let Some(array) = value.as_array() {
        let mut events = Vec::new();
        for item in array {
            if let Some(obj) = item.as_object() {
                let y = get_u64_field(obj, "y", 0, warnings);
                let id = get_u32_field(obj, "id", 0, warnings);
                events.push(super::BgaEvent {
                    y: PulseNumber(y),
                    id: super::BgaId(id),
                });
            } else {
                warnings.push(BmsonWarning::InvalidFieldType(
                    "BGA event should be an object".to_string(),
                ));
            }
        }
        events
    } else {
        warnings.push(BmsonWarning::InvalidFieldType(
            "BGA events should be an array".to_string(),
        ));
        Vec::new()
    }
}

/// Parse scroll events from JSON value.
fn parse_scroll_events(
    value: Option<&Value>,
    warnings: &mut Vec<BmsonWarning>,
) -> Vec<super::ScrollEvent> {
    if let Some(value) = value {
        if let Some(array) = value.as_array() {
            let mut events = Vec::new();
            for item in array {
                if let Some(obj) = item.as_object() {
                    let y = get_u64_field(obj, "y", 0, warnings);
                    let rate = get_f64_field(obj, "rate", 1.0, warnings);
                    events.push(super::ScrollEvent {
                        y: PulseNumber(y),
                        rate: FinF64::new(rate).unwrap_or_else(|| {
                            warnings.push(BmsonWarning::InvalidFieldValue(format!(
                                "Invalid scroll rate: {}, using default 1.0",
                                rate
                            )));
                            FinF64::new(1.0).unwrap()
                        }),
                    });
                } else {
                    warnings.push(BmsonWarning::InvalidFieldType(
                        "Scroll event should be an object".to_string(),
                    ));
                }
            }
            events
        } else {
            warnings.push(BmsonWarning::InvalidFieldType(
                "scroll_events should be an array".to_string(),
            ));
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

/// Parse mine channels from JSON value.
fn parse_mine_channels(
    value: Option<&Value>,
    warnings: &mut Vec<BmsonWarning>,
) -> Vec<super::MineChannel> {
    if let Some(value) = value {
        if let Some(array) = value.as_array() {
            let mut channels = Vec::new();
            for item in array {
                if let Some(obj) = item.as_object() {
                    let name = get_string_field(obj, "name", "", warnings);
                    let notes = if let Some(notes_value) = obj.get("notes") {
                        parse_mine_events(notes_value, warnings)
                    } else {
                        Vec::new()
                    };
                    channels.push(super::MineChannel { name, notes });
                } else {
                    warnings.push(BmsonWarning::InvalidFieldType(
                        "Mine channel should be an object".to_string(),
                    ));
                }
            }
            channels
        } else {
            warnings.push(BmsonWarning::InvalidFieldType(
                "mine_channels should be an array".to_string(),
            ));
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

/// Parse mine events from JSON value.
fn parse_mine_events(value: &Value, warnings: &mut Vec<BmsonWarning>) -> Vec<super::MineEvent> {
    if let Some(array) = value.as_array() {
        let mut events = Vec::new();
        for item in array {
            if let Some(obj) = item.as_object() {
                let x = if let Some(x_value) = obj.get("x") {
                    if x_value.is_null() {
                        None
                    } else if let Some(n) = x_value.as_u64() {
                        if n == 0 {
                            None
                        } else if n <= u8::MAX as u64 {
                            std::num::NonZeroU8::new(n as u8)
                        } else {
                            warnings.push(BmsonWarning::InvalidFieldValue(format!(
                                "Mine x value {} is too large",
                                n
                            )));
                            None
                        }
                    } else {
                        warnings.push(BmsonWarning::InvalidFieldType(
                            "Mine x should be a number or null".to_string(),
                        ));
                        None
                    }
                } else {
                    None
                };
                let y = get_u64_field(obj, "y", 0, warnings);
                let damage = get_f64_field(obj, "damage", 1.0, warnings);
                events.push(super::MineEvent {
                    x,
                    y: PulseNumber(y),
                    damage: FinF64::new(damage).unwrap_or_else(|| {
                        warnings.push(BmsonWarning::InvalidFieldValue(format!(
                            "Invalid mine damage: {}, using default 1.0",
                            damage
                        )));
                        FinF64::new(1.0).unwrap()
                    }),
                });
            } else {
                warnings.push(BmsonWarning::InvalidFieldType(
                    "Mine event should be an object".to_string(),
                ));
            }
        }
        events
    } else {
        warnings.push(BmsonWarning::InvalidFieldType(
            "Mine events should be an array".to_string(),
        ));
        Vec::new()
    }
}

/// Parse key channels from JSON value.
fn parse_key_channels(
    value: Option<&Value>,
    warnings: &mut Vec<BmsonWarning>,
) -> Vec<super::KeyChannel> {
    if let Some(value) = value {
        if let Some(array) = value.as_array() {
            let mut channels = Vec::new();
            for item in array {
                if let Some(obj) = item.as_object() {
                    let name = get_string_field(obj, "name", "", warnings);
                    let notes = if let Some(notes_value) = obj.get("notes") {
                        parse_key_events(notes_value, warnings)
                    } else {
                        Vec::new()
                    };
                    channels.push(super::KeyChannel { name, notes });
                } else {
                    warnings.push(BmsonWarning::InvalidFieldType(
                        "Key channel should be an object".to_string(),
                    ));
                }
            }
            channels
        } else {
            warnings.push(BmsonWarning::InvalidFieldType(
                "key_channels should be an array".to_string(),
            ));
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

/// Parse key events from JSON value.
fn parse_key_events(value: &Value, warnings: &mut Vec<BmsonWarning>) -> Vec<super::KeyEvent> {
    if let Some(array) = value.as_array() {
        let mut events = Vec::new();
        for item in array {
            if let Some(obj) = item.as_object() {
                let x = if let Some(x_value) = obj.get("x") {
                    if x_value.is_null() {
                        None
                    } else if let Some(n) = x_value.as_u64() {
                        if n == 0 {
                            None
                        } else if n <= u8::MAX as u64 {
                            std::num::NonZeroU8::new(n as u8)
                        } else {
                            warnings.push(BmsonWarning::InvalidFieldValue(format!(
                                "Key x value {} is too large",
                                n
                            )));
                            None
                        }
                    } else {
                        warnings.push(BmsonWarning::InvalidFieldType(
                            "Key x should be a number or null".to_string(),
                        ));
                        None
                    }
                } else {
                    None
                };
                let y = get_u64_field(obj, "y", 0, warnings);
                events.push(super::KeyEvent {
                    x,
                    y: PulseNumber(y),
                });
            } else {
                warnings.push(BmsonWarning::InvalidFieldType(
                    "Key event should be an object".to_string(),
                ));
            }
        }
        events
    } else {
        warnings.push(BmsonWarning::InvalidFieldType(
            "Key events should be an array".to_string(),
        ));
        Vec::new()
    }
}

/// Create a default BMSON object.
fn create_default_bmson(warnings: &mut Vec<BmsonWarning>) -> Bmson {
    Bmson {
        version: "1.0.0".to_string(),
        info: create_default_bmson_info(warnings),
        lines: None,
        bpm_events: Vec::new(),
        stop_events: Vec::new(),
        sound_channels: Vec::new(),
        bga: super::Bga::default(),
        scroll_events: Vec::new(),
        mine_channels: Vec::new(),
        key_channels: Vec::new(),
    }
}

/// Create a default BMSON info object.
fn create_default_bmson_info(warnings: &mut Vec<BmsonWarning>) -> BmsonInfo {
    BmsonInfo {
        title: "Unknown".to_string(),
        subtitle: String::new(),
        artist: "Unknown".to_string(),
        subartists: Vec::new(),
        genre: "Unknown".to_string(),
        mode_hint: "beat-7k".to_string(),
        chart_name: String::new(),
        level: 1,
        init_bpm: FinF64::new(120.0).unwrap_or_else(|| {
            warnings.push(BmsonWarning::InvalidFieldValue(
                "Invalid init_bpm, using default 120.0".to_string(),
            ));
            FinF64::new(120.0).unwrap()
        }),
        judge_rank: FinF64::new(100.0).unwrap(),
        total: FinF64::new(100.0).unwrap(),
        back_image: None,
        eyecatch_image: None,
        title_image: None,
        banner_image: None,
        preview_music: None,
        resolution: 240,
        ln_type: crate::bms::command::LnMode::Ln,
    }
}
