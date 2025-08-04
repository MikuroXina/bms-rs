//! Part: Convert `Bms` to `Bmson`.

use std::{collections::HashMap, num::NonZeroU8};

use thiserror::Error;

use crate::{
    bms::prelude::*,
    bmson::{
        fin_f64::FinF64, pulse::PulseConverter, BarLine, Bga, BgaEvent, BgaHeader, BgaId, Bmson, BmsonInfo, BpmEvent, KeyChannel, KeyEvent, LongNoteType, MineChannel, MineEvent, Note, ScrollEvent, SoundChannel, StopEvent
    },
};

/// Errors on converting from `Bms` into `Bmson`.
#[derive(Debug, Error, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BmsonConvertError {
    /// The initial BPM was infinity or NaN.
    #[error("header bpm was invalid value")]
    InvalidBpm,
    /// The total percentage was infinity or NaN.
    #[error("header total was invalid value")]
    InvalidTotal,
    /// The scrolling factor was infinity or NaN.
    #[error("scrolling factor was invalid value")]
    InvalidScrollingFactor,
    /// The judge rank was infinity or NaN.
    #[error("judge rank was invalid value")]
    InvalidJudgeRank,
    /// The stop duration was infinity or NaN.
    #[error("stop duration was invalid value")]
    InvalidStopDuration,
    /// The note lane was invalid.
    #[error("note lane was invalid value")]
    InvalidNoteLane,
}

impl TryFrom<Bms> for Bmson {
    type Error = BmsonConvertError;

    fn try_from(value: Bms) -> Result<Self, Self::Error> {
        let converter = PulseConverter::new(&value);

        const EASY_WIDTH: f64 = 21.0;
        const VERY_EASY_WIDTH: f64 = EASY_WIDTH * 1.25;
        const NORMAL_WIDTH: f64 = 18.0;
        const HARD_WIDTH: f64 = 15.0;
        const VERY_HARD_WIDTH: f64 = 8.0;
        let judge_rank = FinF64::new(match value.header.rank {
            Some(JudgeLevel::OtherInt(4)) => VERY_EASY_WIDTH / NORMAL_WIDTH, // VeryEasy implementation of beatoraja.
            Some(JudgeLevel::Easy) => EASY_WIDTH / NORMAL_WIDTH,
            Some(JudgeLevel::Normal) | None => 1.0,
            Some(JudgeLevel::Hard) => HARD_WIDTH / NORMAL_WIDTH,
            Some(JudgeLevel::VeryHard) => VERY_HARD_WIDTH / NORMAL_WIDTH,
            Some(JudgeLevel::OtherInt(_)) => 1.0,
        })
        .unwrap_or_else(|| {
            // This should never happen as the values are all valid
            panic!("Internal error: judge rank is invalid")
        });

        let resolution = value.resolution_for_pulses();

        let last_obj_time = value
            .last_obj_time()
            .unwrap_or_else(|| ObjTime::new(0, 0, 4));
        let lines = (0..=last_obj_time.track.0)
            .map(|track| BarLine {
                y: converter.get_pulses_on(Track(track)),
            })
            .collect();

        let bpm_events = value
            .arrangers
            .bpm_changes
            .values()
            .map(|bpm_change| {
                Ok(BpmEvent {
                    y: converter.get_pulses_at(bpm_change.time),
                    bpm: FinF64::new(
                        bpm_change
                            .bpm
                            .clone()
                            .try_into()
                            .map_err(|_| BmsonConvertError::InvalidBpm)?,
                    )
                    .ok_or(BmsonConvertError::InvalidBpm)?,
                })
            })
            .collect::<Result<Vec<_>, BmsonConvertError>>()?;

        let stop_events = value
            .arrangers
            .stops
            .values()
            .map(|stop| {
                Ok(StopEvent {
                    y: converter.get_pulses_at(stop.time),
                    duration: stop
                        .duration
                        .clone()
                        .try_into()
                        .map_err(|_| BmsonConvertError::InvalidStopDuration)?,
                })
            })
            .collect::<Result<Vec<_>, BmsonConvertError>>()?;

        let info = BmsonInfo {
            title: value.header.title.unwrap_or_default(),
            subtitle: value.header.subtitle.unwrap_or_default(),
            artist: value.header.artist.unwrap_or_default(),
            subartists: vec![value.header.sub_artist.unwrap_or_default()],
            genre: value.header.genre.unwrap_or_default(),
            mode_hint: {
                // TODO: Support other modes
                let is_7keys = value
                    .notes
                    .all_notes()
                    .any(|note| note.key == Key::Key6 || note.key == Key::Key7);
                let is_dp = value
                    .notes
                    .all_notes()
                    .any(|note| note.side == PlayerSide::Player2);
                match (is_dp, is_7keys) {
                    (true, true) => "beat-14k".into(),
                    (true, false) => "beat-10k".into(),
                    (false, true) => "beat-7k".into(),
                    (false, false) => "beat-5k".into(),
                }
            },
            chart_name: "".into(),
            level: value.header.play_level.unwrap_or_default() as u32,
            init_bpm: FinF64::new(
                value
                    .arrangers
                    .bpm
                    .as_ref()
                    .map_or(Decimal::from(120.0), |bpm| bpm.to_owned())
                    .try_into()
                    .map_err(|_| BmsonConvertError::InvalidBpm)?,
            )
            .ok_or(BmsonConvertError::InvalidBpm)?,
            judge_rank,
            total: FinF64::new(
                value
                    .header
                    .total
                    .unwrap_or(Decimal::from(100.0))
                    .try_into()
                    .map_err(|_| BmsonConvertError::InvalidTotal)?,
            )
            .ok_or(BmsonConvertError::InvalidTotal)?,
            back_image: value
                .header
                .back_bmp
                .as_ref()
                .cloned()
                .map(|path| path.display().to_string()),
            eyecatch_image: value
                .header
                .stage_file
                .map(|path| path.display().to_string()),
            title_image: value.header.back_bmp.map(|path| path.display().to_string()),
            banner_image: value.header.banner.map(|path| path.display().to_string()),
            preview_music: None,
            resolution,
            ln_type: LongNoteType::LN,
        };

        let (sound_channels, mine_channels, key_channels) = {
            let path_root = value.notes.wav_path_root.clone().unwrap_or_default();
            let mut sound_map: HashMap<_, Vec<Note>> = HashMap::new();
            let mut mine_map: HashMap<_, Vec<MineEvent>> = HashMap::new();
            let mut key_map: HashMap<_, Vec<KeyEvent>> = HashMap::new();
            for note in value.notes.all_notes() {
                let note_lane = note
                    .kind
                    .is_playable()
                    .then_some(
                        match note.key {
                            Key::Key1 => 1,
                            Key::Key2 => 2,
                            Key::Key3 => 3,
                            Key::Key4 => 4,
                            Key::Key5 => 5,
                            Key::Key6 => 6,
                            Key::Key7 => 7,
                            Key::Scratch | Key::FreeZone => 8,
                            // TODO: Extra key convertion
                            Key::Key8
                            | Key::Key9
                            | Key::Key10
                            | Key::Key11
                            | Key::Key12
                            | Key::Key13
                            | Key::Key14
                            | Key::ScratchExtra
                            | Key::FootPedal => 0,
                        } + match note.side {
                            PlayerSide::Player1 => 0,
                            PlayerSide::Player2 => 8,
                        },
                    )
                    .map(|lane| NonZeroU8::new(lane).ok_or(BmsonConvertError::InvalidNoteLane))
                    .transpose()?;
                let pulses = converter.get_pulses_at(note.offset);
                match note.kind {
                    NoteKind::Landmine => {
                        let damage = FinF64::new(100.0).unwrap_or_else(|| {
                            // This should never happen as 100.0 is a valid FinF64 value
                            panic!("Internal error: 100.0 is not a valid FinF64")
                        });
                        mine_map.entry(note.obj).or_default().push(MineEvent {
                            x: note_lane,
                            y: pulses,
                            damage,
                        });
                    }
                    NoteKind::Invisible => {
                        key_map.entry(note.obj).or_default().push(KeyEvent {
                            x: note_lane,
                            y: pulses,
                        });
                    }
                    _ => {
                        // Normal note
                        let duration = if let Some(next_note) =
                            value.notes.next_obj_by_key(note.key, note.offset)
                        {
                            pulses.abs_diff(converter.get_pulses_at(next_note.offset))
                        } else {
                            0
                        };
                        sound_map.entry(note.obj).or_default().push(Note {
                            x: note_lane,
                            y: pulses,
                            l: duration,
                            c: false,
                            t: LongNoteType::LN,
                            up: false,
                        });
                    }
                }
            }
            let sound_channels = sound_map
                .into_iter()
                .map(|(obj, notes)| {
                    let sound_path = path_root
                        .join(value.notes.wav_files.get(&obj).cloned().unwrap_or_default());
                    SoundChannel {
                        name: sound_path.display().to_string(),
                        notes,
                    }
                })
                .collect();
            let mine_channels = mine_map
                .into_iter()
                .map(|(obj, notes)| {
                    let sound_path = path_root
                        .join(value.notes.wav_files.get(&obj).cloned().unwrap_or_default());
                    MineChannel {
                        name: sound_path.display().to_string(),
                        notes,
                    }
                })
                .collect();
            let key_channels = key_map
                .into_iter()
                .map(|(obj, notes)| {
                    let sound_path = path_root
                        .join(value.notes.wav_files.get(&obj).cloned().unwrap_or_default());
                    KeyChannel {
                        name: sound_path.display().to_string(),
                        notes,
                    }
                })
                .collect();
            (sound_channels, mine_channels, key_channels)
        };

        let bga = {
            let mut bga = Bga {
                bga_header: vec![],
                bga_events: vec![],
                layer_events: vec![],
                poor_events: vec![],
            };
            for (id, bmp) in &value.graphics.bmp_files {
                bga.bga_header.push(BgaHeader {
                    id: BgaId(id.as_u32()),
                    name: bmp.file.display().to_string(),
                });
            }
            for (&time, change) in value.graphics.bga_changes() {
                let target = match change.layer {
                    BgaLayer::Base => &mut bga.bga_events,
                    BgaLayer::Poor => &mut bga.poor_events,
                    BgaLayer::Overlay => &mut bga.layer_events,
                };
                target.push(BgaEvent {
                    y: converter.get_pulses_at(time),
                    id: BgaId(change.id.as_u32()),
                })
            }
            bga
        };

        let scroll_events = value
            .arrangers
            .scrolling_factor_changes
            .values()
            .map(|scroll| {
                Ok(ScrollEvent {
                    y: converter.get_pulses_at(scroll.time),
                    rate: FinF64::new(
                        scroll
                            .factor
                            .clone()
                            .try_into()
                            .map_err(|_| BmsonConvertError::InvalidScrollingFactor)?,
                    )
                    .ok_or(BmsonConvertError::InvalidScrollingFactor)?,
                })
            })
            .collect::<Result<Vec<_>, BmsonConvertError>>()?;

        Ok(Self {
            version: "1.0.0".into(),
            info,
            lines: Some(lines),
            bpm_events,
            stop_events,
            sound_channels,
            bga,
            scroll_events,
            mine_channels,
            key_channels,
        })
    }
}
