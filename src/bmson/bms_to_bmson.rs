//! Part: Convert `Bms` to `Bmson`.

use std::{
    borrow::Cow,
    collections::HashMap,
    num::{NonZeroU8, NonZeroU64},
};

use num::ToPrimitive;
use thiserror::Error;

use crate::{
    bms::prelude::*,
    bmson::{
        BarLine, Bga, BgaEvent, BgaHeader, BgaId, Bmson, BmsonInfo, BpmEvent, KeyChannel, KeyEvent,
        MineChannel, MineEvent, Note, ScrollEvent, SoundChannel, StopEvent, fin_f64::FinF64,
        pulse::PulseConverter,
    },
};

/// Warnings that occur during conversion from `Bms` to `Bmson`.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BmsToBmsonWarning {
    /// The initial BPM was invalid and default value was used.
    #[error("initial BPM was invalid, using default value")]
    InvalidBpm,
    /// The total percentage was invalid and default value was used.
    #[error("total percentage was invalid, using default value")]
    InvalidTotal,
    /// The scrolling factor was invalid and default value was used.
    #[error("scrolling factor was invalid, using default value")]
    InvalidScrollingFactor,
    /// The judge rank was invalid and default value was used.
    #[error("judge rank was invalid, using default value")]
    InvalidJudgeRank,
    /// The stop duration was invalid and default value was used.
    #[error("stop duration was invalid, using default value")]
    InvalidStopDuration,
    /// The note lane was invalid and default value was used.
    #[error("note lane was invalid, using default value")]
    InvalidNoteLane,
    /// The initial BPM was missing and default value was used.
    #[error("initial BPM was missing, using default value")]
    MissingBpm,
    /// The total percentage was missing and default value was used.
    #[error("total percentage was missing, using default value")]
    MissingTotal,
}

/// Output of the conversion from `Bms` to `Bmson`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct BmsToBmsonOutput<'a> {
    /// The converted `Bmson` object.
    pub bmson: Bmson<'a>,
    /// Warnings that occurred during the conversion.
    pub warnings: Vec<BmsToBmsonWarning>,
}

impl Bms {
    /// Convert `Bms` to `Bmson`.
    pub fn to_bmson<'a>(self) -> BmsToBmsonOutput<'a> {
        const EASY_WIDTH: f64 = 21.0;
        const VERY_EASY_WIDTH: f64 = EASY_WIDTH * 1.25;
        const NORMAL_WIDTH: f64 = 18.0;
        const HARD_WIDTH: f64 = 15.0;
        const VERY_HARD_WIDTH: f64 = 8.0;

        let mut warnings = Vec::new();
        let converter = PulseConverter::new(&self);
        let judge_rank = FinF64::new(match self.header.rank {
            Some(JudgeLevel::OtherInt(4)) => VERY_EASY_WIDTH / NORMAL_WIDTH, // VeryEasy implementation of beatoraja.
            Some(JudgeLevel::Easy) => EASY_WIDTH / NORMAL_WIDTH,
            Some(JudgeLevel::Normal | JudgeLevel::OtherInt(_)) | None => 1.0,
            Some(JudgeLevel::Hard) => HARD_WIDTH / NORMAL_WIDTH,
            Some(JudgeLevel::VeryHard) => VERY_HARD_WIDTH / NORMAL_WIDTH,
        })
        .unwrap_or_else(|| {
            warnings.push(BmsToBmsonWarning::InvalidJudgeRank);
            FinF64::new(1.0).expect("Internal error: 1.0 is not a valid FinF64")
        });

        let resolution = self.resolution_for_pulses();

        let last_obj_time = self.last_obj_time().unwrap_or_else(|| {
            ObjTime::new(
                0,
                0,
                NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
            )
        });
        let lines = (0..=last_obj_time.track.0)
            .map(|track| BarLine {
                y: converter.get_pulses_on(Track(track)),
            })
            .collect();

        let bpm_events =
            self.arrangers
                .bpm_changes
                .values()
                .map(|bpm_change| BpmEvent {
                    y: converter.get_pulses_at(bpm_change.time),
                    bpm: FinF64::new(bpm_change.bpm.clone().to_f64().unwrap_or(120.0))
                        .unwrap_or_else(|| {
                            warnings.push(BmsToBmsonWarning::InvalidBpm);
                            FinF64::new(120.0).expect("Internal error: 120.0 is not a valid FinF64")
                        }),
                })
                .collect();

        let stop_events = self
            .arrangers
            .stops
            .values()
            .filter_map(|stop| {
                stop.duration.to_f64().map(|f64_value| StopEvent {
                    y: converter.get_pulses_at(stop.time),
                    duration: f64_value as u64,
                })
            })
            .collect();

        let info = BmsonInfo {
            title: Cow::Owned(self.header.title.unwrap_or_default()),
            subtitle: Cow::Owned(self.header.subtitle.unwrap_or_default()),
            artist: Cow::Owned(self.header.artist.unwrap_or_default()),
            subartists: vec![Cow::Owned(self.header.sub_artist.unwrap_or_default())],
            genre: Cow::Owned(self.header.genre.unwrap_or_default()),
            mode_hint: {
                // TODO: Support other modes
                let is_7keys = self.notes.all_notes().any(|note| {
                    note.channel_id
                        .try_into_map::<KeyLayoutBeat>()
                        .is_some_and(|map| matches!(map.key(), Key::Key(6 | 7)))
                });
                let is_dp = self.notes.all_notes().any(|note| {
                    note.channel_id
                        .try_into_map::<KeyLayoutBeat>()
                        .is_some_and(|map| map.side() == PlayerSide::Player2)
                });
                match (is_dp, is_7keys) {
                    (true, true) => "beat-14k".into(),
                    (true, false) => "beat-10k".into(),
                    (false, true) => "beat-7k".into(),
                    (false, false) => "beat-5k".into(),
                }
            },
            chart_name: Cow::Owned(String::new()),
            level: self.header.play_level.unwrap_or_default() as u32,
            init_bpm: {
                let bpm_value = if let Some(bpm) = self.arrangers.bpm.as_ref() {
                    bpm.to_f64().unwrap_or(120.0)
                } else {
                    warnings.push(BmsToBmsonWarning::MissingBpm);
                    120.0
                };
                FinF64::new(bpm_value).unwrap_or_else(|| {
                    warnings.push(BmsToBmsonWarning::InvalidBpm);
                    FinF64::new(120.0).expect("Internal error: 120.0 is not a valid FinF64")
                })
            },
            judge_rank,
            total: {
                let total_value = if let Some(total) = self.header.total.as_ref() {
                    total.to_f64().unwrap_or(100.0)
                } else {
                    warnings.push(BmsToBmsonWarning::MissingTotal);
                    100.0
                };
                FinF64::new(total_value).unwrap_or_else(|| {
                    warnings.push(BmsToBmsonWarning::InvalidTotal);
                    FinF64::new(100.0).expect("Internal error: 100.0 is not a valid FinF64")
                })
            },
            back_image: self
                .header
                .back_bmp
                .clone()
                .map(|path| Cow::Owned(path.display().to_string())),
            eyecatch_image: self
                .header
                .stage_file
                .map(|path| Cow::Owned(path.display().to_string())),
            title_image: self
                .header
                .back_bmp
                .map(|path| Cow::Owned(path.display().to_string())),
            banner_image: self
                .header
                .banner
                .map(|path| Cow::Owned(path.display().to_string())),
            preview_music: None,
            resolution: NonZeroU64::new(resolution).expect("resolution should be non-zero"),
            ln_type: self.header.ln_mode,
        };

        let (sound_channels, mine_channels, key_channels) = {
            let path_root = self.notes.wav_path_root.clone().unwrap_or_default();
            let mut sound_map: HashMap<_, Vec<Note>> = HashMap::new();
            let mut mine_map: HashMap<_, Vec<MineEvent>> = HashMap::new();
            let mut key_map: HashMap<_, Vec<KeyEvent>> = HashMap::new();
            for note in self.notes.all_notes() {
                let note_lane = note
                    .channel_id
                    .try_into_map::<KeyLayoutBeat>()
                    .filter(|map| map.kind().is_playable())
                    .map(|map|
                        match map.key() {
                            Key::Key(1) => 1,
                            Key::Key(2) => 2,
                            Key::Key(3) => 3,
                            Key::Key(4) => 4,
                            Key::Key(5) => 5,
                            Key::Key(6) => 6,
                            Key::Key(7) => 7,
                            Key::Scratch(_) | Key::FreeZone => 8,
                            // TODO: Extra key convertion
                            Key::Key(_) | Key::FootPedal => 0,
                        } + match map.side() {
                            PlayerSide::Player1 => 0,
                            PlayerSide::Player2 => 8,
                        }
                    )
                    .and_then(NonZeroU8::new);

                let pulses = converter.get_pulses_at(note.offset);
                match note
                    .channel_id
                    .try_into_map::<KeyLayoutBeat>()
                    .map(|map| map.kind())
                {
                    Some(NoteKind::Landmine) => {
                        let damage = FinF64::new(100.0).unwrap_or_else(|| {
                            // This should never happen as 100.0 is a valid FinF64 value
                            panic!("Internal error: 100.0 is not a valid FinF64")
                        });
                        mine_map.entry(note.wav_id).or_default().push(MineEvent {
                            x: note_lane,
                            y: pulses,
                            damage,
                        });
                    }
                    Some(NoteKind::Invisible) | None => {
                        key_map.entry(note.wav_id).or_default().push(KeyEvent {
                            x: note_lane,
                            y: pulses,
                        });
                    }
                    Some(NoteKind::Long) => {
                        let duration = self
                            .notes
                            .next_obj_by_key(note.channel_id, note.offset)
                            .map_or(0, |next_note| {
                                pulses.abs_diff(converter.get_pulses_at(next_note.offset))
                            });
                        sound_map.entry(note.wav_id).or_default().push(Note {
                            x: note_lane,
                            y: pulses,
                            l: duration,
                            c: false,
                            t: Some(self.header.ln_mode),
                            up: Some(false),
                        });
                    }
                    Some(NoteKind::Visible) => {
                        sound_map.entry(note.wav_id).or_default().push(Note {
                            x: note_lane,
                            y: pulses,
                            l: 0,
                            c: false,
                            t: Some(self.header.ln_mode),
                            up: Some(false),
                        });
                    }
                }
            }
            let sound_channels = sound_map
                .into_iter()
                .map(|(obj, notes)| {
                    let sound_path =
                        path_root.join(self.notes.wav_files.get(&obj).cloned().unwrap_or_default());
                    SoundChannel {
                        name: Cow::Owned(sound_path.display().to_string()),
                        notes,
                    }
                })
                .collect();
            let mine_channels = mine_map
                .into_iter()
                .map(|(obj, notes)| {
                    let sound_path =
                        path_root.join(self.notes.wav_files.get(&obj).cloned().unwrap_or_default());
                    MineChannel {
                        name: Cow::Owned(sound_path.display().to_string()),
                        notes,
                    }
                })
                .collect();
            let key_channels = key_map
                .into_iter()
                .map(|(obj, notes)| {
                    let sound_path =
                        path_root.join(self.notes.wav_files.get(&obj).cloned().unwrap_or_default());
                    KeyChannel {
                        name: Cow::Owned(sound_path.display().to_string()),
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
            for (id, bmp) in &self.graphics.bmp_files {
                bga.bga_header.push(BgaHeader {
                    id: BgaId(id.as_u32()),
                    name: Cow::Owned(bmp.file.display().to_string()),
                });
            }
            for (&time, change) in self.graphics.bga_changes() {
                let target = match change.layer {
                    BgaLayer::Base => &mut bga.bga_events,
                    BgaLayer::Poor => &mut bga.poor_events,
                    BgaLayer::Overlay | BgaLayer::Overlay2 => &mut bga.layer_events,
                };
                target.push(BgaEvent {
                    y: converter.get_pulses_at(time),
                    id: BgaId(change.id.as_u32()),
                });
            }
            bga
        };

        let scroll_events = self
            .arrangers
            .scrolling_factor_changes
            .values()
            .filter_map(|scroll| {
                let rate = FinF64::new(scroll.factor.clone().to_f64().unwrap_or(1.0));
                if rate.is_none() {
                    warnings.push(BmsToBmsonWarning::InvalidScrollingFactor);
                    return None;
                }
                Some(ScrollEvent {
                    y: converter.get_pulses_at(scroll.time),
                    rate: rate.unwrap(),
                })
            })
            .collect();

        let bmson = Bmson {
            version: Cow::Borrowed("1.0.0"),
            info,
            lines: Some(lines),
            bpm_events,
            stop_events,
            sound_channels,
            bga,
            scroll_events,
            mine_channels,
            key_channels,
        };

        BmsToBmsonOutput { bmson, warnings }
    }
}
