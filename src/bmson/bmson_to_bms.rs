//! Part: Convert `Bmson` to `Bms`.

use std::{
    num::{NonZeroU8, NonZeroU64},
    path::PathBuf,
};

use thiserror::Error;

use crate::{
    bms::prelude::*,
    bmson::{BgaId, Bmson, pulse::PulseNumber},
};

/// Warnings that occur during conversion from `Bmson` to `Bms`.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BmsonToBmsWarning {
    /// The wav object ID was out of range and default value was used.
    #[error("wav object ID was out of range, using default value")]
    WavObjIdOutOfRange,
    /// The BGA header object ID was out of range and default value was used.
    #[error("BGA header object ID was out of range, using default value")]
    BgaHeaderObjIdOutOfRange,
    /// The BGA event object ID was out of range and default value was used.
    #[error("BGA event object ID was out of range, using default value")]
    BgaEventObjIdOutOfRange,
    /// The BPM definition was out of range and default value was used.
    #[error("BPM definition was out of range, using default value")]
    BpmDefOutOfRange,
    /// The stop definition was out of range and default value was used.
    #[error("stop definition was out of range, using default value")]
    StopDefOutOfRange,
    /// The scroll definition was out of range and default value was used.
    #[error("scroll definition was out of range, using default value")]
    ScrollDefOutOfRange,
}

#[derive(Debug)]
struct ObjIdIssuer(u16);

impl ObjIdIssuer {
    const fn new() -> Self {
        Self(1)
    }
}

impl Iterator for ObjIdIssuer {
    type Item = ObjId;
    fn next(&mut self) -> Option<Self::Item> {
        const MAX_ID: u16 = 62 * 62;
        if self.0 > MAX_ID {
            return None;
        }
        let id = self.0;
        self.0 += 1;
        create_obj_id_from_u16(id).ok()
    }
}

/// Output of the conversion from `Bmson` to `Bms`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct BmsonToBmsOutput {
    /// The converted `Bms` object.
    pub bms: Bms,
    /// Warnings that occurred during the conversion.
    pub warnings: Vec<BmsonToBmsWarning>,
    /// Warnings that affect the playing of the score.
    pub playing_warnings: Vec<PlayingWarning>,
    /// Errors that make the score unplayable.
    pub playing_errors: Vec<PlayingError>,
}

impl Bms {
    /// Convert `Bmson` to `Bms`.
    pub fn from_bmson(value: Bmson) -> BmsonToBmsOutput {
        let mut bms = Self::default();
        let mut warnings = Vec::new();
        let mut wav_obj_id_issuer = ObjIdIssuer::new();
        let mut bga_header_obj_id_issuer = ObjIdIssuer::new();
        let mut bpm_def_obj_id_issuer = ObjIdIssuer::new();
        let mut stop_def_obj_id_issuer = ObjIdIssuer::new();
        let mut scroll_def_obj_id_issuer = ObjIdIssuer::new();

        let resolution =
            NonZeroU64::new(value.info.resolution.get()).expect("resolution should be non-zero");

        // Convert info to header
        bms.header.title = Some(value.info.title.into_owned());
        bms.header.subtitle = Some(value.info.subtitle.into_owned());
        bms.header.artist = Some(value.info.artist.into_owned());
        bms.header.sub_artist = value
            .info
            .subartists
            .first()
            .map(|s| s.clone().into_owned());
        bms.header.genre = Some(value.info.genre.into_owned());
        bms.header.play_level = Some(value.info.level as u8);
        bms.header.total = Some(Decimal::from(value.info.total.as_f64()));
        bms.header.back_bmp = value.info.back_image.map(|s| PathBuf::from(s.into_owned()));
        bms.header.stage_file = value
            .info
            .eyecatch_image
            .map(|s| PathBuf::from(s.into_owned()));
        bms.header.banner = value
            .info
            .banner_image
            .map(|s| PathBuf::from(s.into_owned()));
        bms.header.preview_music = value
            .info
            .preview_music
            .map(|s| PathBuf::from(s.into_owned()));

        // Convert judge rank
        let judge_rank_value = (value.info.judge_rank.as_f64() * 18.0) as i64;
        bms.header.rank = Some(JudgeLevel::OtherInt(judge_rank_value));

        // Convert initial BPM
        bms.arrangers.bpm = Some(Decimal::from(value.info.init_bpm.as_f64()));

        // Convert resolution
        bms.arrangers.section_len_changes.insert(
            Track(0),
            SectionLenChangeObj {
                track: Track(0),
                length: Decimal::from(resolution.get()),
            },
        );

        // Convert BPM events
        for bpm_event in value.bpm_events {
            let time = convert_pulse_to_obj_time(bpm_event.y, resolution);
            let bpm = Decimal::from(bpm_event.bpm.as_f64());

            // Add to scope_defines
            let bpm_def_id = bpm_def_obj_id_issuer.next().unwrap_or_else(|| {
                warnings.push(BmsonToBmsWarning::BpmDefOutOfRange);
                ObjId::null()
            });
            bms.scope_defines.bpm_defs.insert(bpm_def_id, bpm.clone());

            bms.arrangers
                .bpm_changes
                .insert(time, BpmChangeObj { time, bpm });
        }

        // Convert stop events
        for stop_event in value.stop_events {
            let time = convert_pulse_to_obj_time(stop_event.y, resolution);
            let duration = Decimal::from(stop_event.duration);

            // Add to scope_defines
            let stop_def_id = stop_def_obj_id_issuer.next().unwrap_or_else(|| {
                warnings.push(BmsonToBmsWarning::StopDefOutOfRange);
                ObjId::null()
            });
            bms.scope_defines
                .stop_defs
                .insert(stop_def_id, duration.clone());

            bms.arrangers.stops.insert(time, StopObj { time, duration });
        }

        // Convert scroll events
        for scroll_event in value.scroll_events {
            let time = convert_pulse_to_obj_time(scroll_event.y, resolution);
            let factor = Decimal::from(scroll_event.rate.as_f64());

            // Add to scope_defines
            let scroll_def_id = scroll_def_obj_id_issuer.next().unwrap_or_else(|| {
                warnings.push(BmsonToBmsWarning::ScrollDefOutOfRange);
                ObjId::null()
            });
            bms.scope_defines
                .scroll_defs
                .insert(scroll_def_id, factor.clone());

            bms.arrangers
                .scrolling_factor_changes
                .insert(time, ScrollingFactorObj { time, factor });
        }

        // Convert sound channels to notes
        for sound_channel in value.sound_channels {
            let wav_path = PathBuf::from(sound_channel.name.into_owned());
            let obj_id = wav_obj_id_issuer.next().unwrap_or_else(|| {
                warnings.push(BmsonToBmsWarning::WavObjIdOutOfRange);
                ObjId::null()
            });
            bms.notes.wav_files.insert(obj_id, wav_path);

            for note in sound_channel.notes {
                let time = convert_pulse_to_obj_time(note.y, resolution);
                let (key, side) = convert_lane_to_key_side(note.x);
                let kind = if note.l > 0 {
                    NoteKind::Long
                } else {
                    NoteKind::Visible
                };

                let obj = WavObj {
                    offset: time,
                    channel_id: KeyLayoutBeat::new(side, kind, key).to_channel_id(),
                    wav_id: obj_id,
                };
                bms.notes.push_note(obj);
            }
        }

        // Convert mine channels
        for mine_channel in value.mine_channels {
            let wav_path = PathBuf::from(mine_channel.name.into_owned());
            let obj_id = wav_obj_id_issuer.next().unwrap_or_else(|| {
                warnings.push(BmsonToBmsWarning::WavObjIdOutOfRange);
                ObjId::null()
            });
            bms.notes.wav_files.insert(obj_id, wav_path);

            for mine_event in mine_channel.notes {
                let time = convert_pulse_to_obj_time(mine_event.y, resolution);
                let (key, side) = convert_lane_to_key_side(mine_event.x);

                let obj = WavObj {
                    offset: time,
                    channel_id: KeyLayoutBeat::new(side, NoteKind::Landmine, key).to_channel_id(),
                    wav_id: obj_id,
                };
                bms.notes.push_note(obj);
            }
        }

        // Convert key channels (invisible notes)
        for key_channel in value.key_channels {
            let wav_path = PathBuf::from(key_channel.name.into_owned());
            let obj_id = wav_obj_id_issuer.next().unwrap_or_else(|| {
                warnings.push(BmsonToBmsWarning::WavObjIdOutOfRange);
                ObjId::null()
            });
            bms.notes.wav_files.insert(obj_id, wav_path);

            for key_event in key_channel.notes {
                let time = convert_pulse_to_obj_time(key_event.y, resolution);
                let (key, side) = convert_lane_to_key_side(key_event.x);

                let obj = WavObj {
                    offset: time,
                    channel_id: KeyLayoutBeat::new(side, NoteKind::Invisible, key).to_channel_id(),
                    wav_id: obj_id,
                };
                bms.notes.push_note(obj);
            }
        }

        // Convert BGA
        // First, create a mapping from BgaId to ObjId for bga_headers
        let mut bga_id_to_obj_id = std::collections::HashMap::new();

        for bga_header in value.bga.bga_header {
            let bmp_path = PathBuf::from(bga_header.name.into_owned());
            let obj_id = bga_header_obj_id_issuer.next().unwrap_or_else(|| {
                warnings.push(BmsonToBmsWarning::BgaHeaderObjIdOutOfRange);
                ObjId::null()
            });
            bga_id_to_obj_id.insert(bga_header.id, obj_id);
            bms.graphics.bmp_files.insert(
                obj_id,
                Bmp {
                    file: bmp_path,
                    transparent_color: Argb::default(),
                },
            );
        }

        // Helper function to get obj_id for bga events
        let mut get_bga_obj_id = |bga_id: &BgaId| -> ObjId {
            bga_id_to_obj_id.get(bga_id).copied().unwrap_or_else(|| {
                warnings.push(BmsonToBmsWarning::BgaEventObjIdOutOfRange);
                ObjId::null()
            })
        };

        for bga_event in value.bga.bga_events {
            let time = convert_pulse_to_obj_time(bga_event.y, resolution);
            let obj_id = get_bga_obj_id(&bga_event.id);
            bms.graphics.bga_changes.insert(
                time,
                BgaObj {
                    time,
                    id: obj_id,
                    layer: BgaLayer::Base,
                },
            );
        }

        for bga_event in value.bga.layer_events {
            let time = convert_pulse_to_obj_time(bga_event.y, resolution);
            let obj_id = get_bga_obj_id(&bga_event.id);
            bms.graphics.bga_changes.insert(
                time,
                BgaObj {
                    time,
                    id: obj_id,
                    layer: BgaLayer::Overlay,
                },
            );
        }

        for bga_event in value.bga.poor_events {
            let time = convert_pulse_to_obj_time(bga_event.y, resolution);
            let obj_id = get_bga_obj_id(&bga_event.id);
            bms.graphics.bga_changes.insert(
                time,
                BgaObj {
                    time,
                    id: obj_id,
                    layer: BgaLayer::Poor,
                },
            );
        }

        let PlayingCheckOutput {
            playing_warnings,
            playing_errors,
        } = bms.check_playing();

        BmsonToBmsOutput {
            bms,
            warnings,
            playing_warnings,
            playing_errors,
        }
    }
}

/// Converts a pulse number to [`ObjTime`]
fn convert_pulse_to_obj_time(pulse: PulseNumber, resolution: NonZeroU64) -> ObjTime {
    // Simple conversion: assume 4/4 time signature and convert pulses to track/time
    let pulses_per_measure = resolution.get() * 4; // 4 quarter notes per measure
    let track = pulse.0 / pulses_per_measure;
    let remaining_pulses = pulse.0 % pulses_per_measure;

    // Convert remaining pulses to fraction
    let numerator = remaining_pulses;
    let denominator = pulses_per_measure;

    let denominator = NonZeroU64::new(denominator).expect("resolution should be non-zero");
    ObjTime::new(track, numerator, denominator)
}

/// Converts a lane number to [`Key`] and [`PlayerSide`]
fn convert_lane_to_key_side(lane: Option<NonZeroU8>) -> (Key, PlayerSide) {
    let lane_value = lane.map_or(0, |l| l.get());

    // Handle player sides
    let (adjusted_lane, side) = if lane_value > 8 {
        (lane_value - 8, PlayerSide::Player2)
    } else {
        (lane_value, PlayerSide::Player1)
    };

    // Convert lane to key
    let key = match adjusted_lane {
        1 => Key::Key(1),
        2 => Key::Key(2),
        3 => Key::Key(3),
        4 => Key::Key(4),
        5 => Key::Key(5),
        6 => Key::Key(6),
        7 => Key::Key(7),
        8 => Key::Scratch(1),
        _ => Key::Key(1), // Default fallback
    };

    (key, side)
}

/// Creates an [`ObjId`] from `u16`
fn create_obj_id_from_u16(value: u16) -> Result<ObjId, ()> {
    let mut chars = ['0'; 2];
    let first = (value / 62) as u8;
    let second = (value % 62) as u8;

    chars[0] = match first {
        0..=9 => (b'0' + first) as char,
        10..=35 => (b'A' + (first - 10)) as char,
        36..=61 => (b'a' + (first - 36)) as char,
        _ => return Err(()),
    };

    chars[1] = match second {
        0..=9 => (b'0' + second) as char,
        10..=35 => (b'A' + (second - 10)) as char,
        36..=61 => (b'a' + (second - 36)) as char,
        _ => return Err(()),
    };

    chars.try_into().map_err(|_| ())
}
