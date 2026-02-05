//! Unparse Bms model into `Vec<Token>` without duplicate parsing logic.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

use num::Integer;

use crate::bms::prelude::*;

impl Bms {
    /// Convert Bms to `Vec<Token>` (in conventional order: header -> definitions -> resources -> messages).
    /// - Avoid duplicate parsing: directly construct Tokens using model data;
    /// - For messages requiring `ObjId`, prioritize reusing existing definitions; if missing, allocate new `ObjId` and add definition Token (only reflected in returned Token list).
    #[must_use]
    pub fn unparse<'a, T: KeyLayoutMapper>(&'a self) -> Vec<Token<'a>> {
        let mut tokens: Vec<Token<'a>> = Vec::new();

        // Others section lines FIRST to preserve order equality on roundtrip
        self.unparse_headers(&mut tokens);

        // Check if any of the used IDs require base62 (not base36 but valid base62)
        let mut base62_checker = Base62Checker::new();

        self.unparse_messages::<T>(&mut tokens, &mut base62_checker);

        // Add ObjIds from definition tokens that are not covered by managers
        // ExWav definitions

        base62_checker.check(self.wav.exwav_defs.keys().copied());

        // WavCmd events (use wav_index ObjId)

        base62_checker.check(self.wav.wavcmd_events.values().map(|ev| ev.wav_index));

        // AtBga definitions (use both id and source_bmp ObjIds)

        {
            base62_checker.check(self.bmp.atbga_defs.keys().copied());
            base62_checker.check(self.bmp.atbga_defs.values().map(|def| def.source_bmp));
        }

        // Bga definitions (use both id and source_bmp ObjIds)

        {
            base62_checker.check(self.bmp.bga_defs.keys().copied());
            base62_checker.check(self.bmp.bga_defs.values().map(|def| def.source_bmp));
        }

        // Argb definitions

        base62_checker.check(self.bmp.argb_defs.keys().copied());

        // SwBga events

        base62_checker.check(self.bmp.swbga_events.keys().copied());

        // Wav resource files
        base62_checker.check(self.wav.wav_files.keys().copied());

        // Bmp/ExBmp resource files
        base62_checker.check(self.bmp.bmp_files.keys().copied());

        // Add Base62 token if needed
        if base62_checker.into_using_base62() {
            tokens.push(Token::Header {
                name: "BASE".into(),
                args: "62".into(),
            });
        }

        tokens
    }

    fn unparse_headers<'a>(&'a self, tokens: &mut Vec<Token<'a>>) {
        // Options
        if let Some(options) = self.option.options.as_ref() {
            for option in options {
                tokens.push(Token::Header {
                    name: "OPTION".into(),
                    args: option.into(),
                });
            }
        }
        // Octave mode
        if self.metadata.is_octave {
            tokens.push(Token::Header {
                name: "OCT/FP".into(),
                args: "".into(),
            });
        }
        // CDDA events
        for cdda in &self.resources.cdda {
            tokens.push(Token::Header {
                name: "CDDA".into(),
                args: cdda.to_string().into(),
            });
        }
        // Extended character events
        for ExtChrEvent {
            sprite_num,
            bmp_num,
            start_x,
            start_y,
            end_x,
            end_y,
            offset_x,
            offset_y,
            abs_x,
            abs_y,
        } in &self.sprite.extchr_events
        {
            use itertools::Itertools;

            let buf = [sprite_num, bmp_num, start_x, start_y, end_x, end_y]
                .into_iter()
                .copied()
                .chain(
                    offset_x
                        .zip(*offset_y)
                        .map(Into::<[i32; 2]>::into)
                        .into_iter()
                        .flatten(),
                )
                .chain(
                    abs_x
                        .zip(*abs_y)
                        .map(Into::<[i32; 2]>::into)
                        .into_iter()
                        .flatten(),
                )
                .join(" ");
            tokens.push(Token::Header {
                name: "EXTCHR".into(),
                args: buf.into(),
            });
        }
        // Change options
        for (id, option) in &self.option.change_options {
            tokens.push(Token::Header {
                name: format!("CHANGEOPTION{id}").into(),
                args: option.as_str().into(),
            });
        }
        // Divide property
        if let Some(divide_prop) = self.metadata.divide_prop.as_ref() {
            tokens.push(Token::Header {
                name: "DIVIDEPROP".into(),
                args: divide_prop.as_str().into(),
            });
        }
        // Materials path
        if let Some(materials_path) = self.resources.materials_path.as_ref()
            && !materials_path.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::Header {
                name: "MATERIALS".into(),
                args: materials_path.display().to_string().into(),
            });
        }

        for line in &self.repr.non_command_lines {
            tokens.push(Token::NotACommand(line.as_str()));
        }

        // Header
        if let Some(player) = self.metadata.player {
            tokens.push(Token::Header {
                name: "PLAYER".into(),
                args: player.to_string().into(),
            });
        }
        if let Some(maker) = self.music_info.maker.as_deref() {
            tokens.push(Token::Header {
                name: "MAKER".into(),
                args: maker.into(),
            });
        }
        if let Some(genre) = self.music_info.genre.as_deref() {
            tokens.push(Token::Header {
                name: "GENRE".into(),
                args: genre.into(),
            });
        }
        if let Some(title) = self.music_info.title.as_deref() {
            tokens.push(Token::Header {
                name: "TITLE".into(),
                args: title.into(),
            });
        }
        if let Some(artist) = self.music_info.artist.as_deref() {
            tokens.push(Token::Header {
                name: "ARTIST".into(),
                args: artist.into(),
            });
        }
        if let Some(sub_artist) = self.music_info.sub_artist.as_deref() {
            tokens.push(Token::Header {
                name: "SUBARTIST".into(),
                args: sub_artist.into(),
            });
        }
        if let Some(bpm) = self.bpm.bpm.as_ref() {
            tokens.push(Token::Header {
                name: "BPM".into(),
                args: bpm.to_string().into(),
            });
        }
        if let Some(play_level) = self.metadata.play_level {
            tokens.push(Token::Header {
                name: "PLAYLEVEL".into(),
                args: play_level.to_string().into(),
            });
        }
        if let Some(rank) = self.judge.rank {
            tokens.push(Token::Header {
                name: "RANK".into(),
                args: rank.to_string().into(),
            });
        }
        if let Some(subtitle) = self.music_info.subtitle.as_deref() {
            tokens.push(Token::Header {
                name: "SUBTITLE".into(),
                args: subtitle.into(),
            });
        }
        if let Some(stage_file) = self.sprite.stage_file.as_ref()
            && !stage_file.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::Header {
                name: "STAGEFILE".into(),
                args: stage_file.display().to_string().into(),
            });
        }
        if let Some(back_bmp) = self.sprite.back_bmp.as_ref()
            && !back_bmp.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::Header {
                name: "BACKBMP".into(),
                args: back_bmp.display().to_string().into(),
            });
        }
        if let Some(banner) = self.sprite.banner.as_ref()
            && !banner.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::Header {
                name: "BANNER".into(),
                args: banner.display().to_string().into(),
            });
        }
        if let Some(difficulty) = self.metadata.difficulty {
            tokens.push(Token::Header {
                name: "DIFFICULTY".into(),
                args: difficulty.to_string().into(),
            });
        }
        if let Some(preview) = self.music_info.preview_music.as_ref()
            && !preview.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::Header {
                name: "PREVIEW".into(),
                args: preview.display().to_string().into(),
            });
        }
        if let Some(movie) = self.video.video_file.as_ref()
            && !movie.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::Header {
                name: "MOVIE".into(),
                args: movie.display().to_string().into(),
            });
        }
        if let Some(comment_lines) = self.music_info.comment.as_ref() {
            for line in comment_lines {
                tokens.push(Token::NotACommand(line.as_str()));
            }
        }
        if let Some(total) = self.judge.total.as_ref() {
            tokens.push(Token::Header {
                name: "TOTAL".into(),
                args: total.to_string().into(),
            });
        }
        if let Some(email) = self.metadata.email.as_deref() {
            tokens.push(Token::Header {
                name: "EMAIL".into(),
                args: email.into(),
            });
        }
        if let Some(url) = self.metadata.url.as_deref() {
            tokens.push(Token::Header {
                name: "URL".into(),
                args: url.into(),
            });
        }

        // LnType
        if self.repr.ln_type == LnType::Mgq {
            tokens.push(Token::Header {
                name: "LNTYPE".into(),
                args: "2".into(),
            });
        }
        // LnMode
        if self.repr.ln_mode != LnMode::default() {
            tokens.push(Token::Header {
                name: "LNMODE".into(),
                args: (self.repr.ln_mode as u8).to_string().into(),
            });
        }

        tokens.extend(
            self.wav
                .wav_files
                .iter()
                .filter(|(_, path)| !path.as_path().as_os_str().is_empty())
                .map(|(id, path)| {
                    (
                        *id,
                        Token::Header {
                            name: format!("WAV{id}").into(),
                            args: path.display().to_string().into(),
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        // PoorBga mode

        if self.bmp.poor_bga_mode != PoorMode::default() {
            tokens.push(Token::Header {
                name: "POORBGA".into(),
                args: self.bmp.poor_bga_mode.as_str().into(),
            });
        }

        // Add basic definitions

        if let Some(base_bpm) = self.bpm.base_bpm.as_ref() {
            tokens.push(Token::Header {
                name: "BASEBPM".into(),
                args: base_bpm.to_string().into(),
            });
        }

        tokens.extend(
            self.bpm
                .bpm_defs
                .iter()
                .map(|(id, v)| {
                    (
                        *id,
                        Token::Header {
                            name: format!("BPM{id}").into(),
                            args: v.to_string().into(),
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        self.unparse_def_tokens(tokens);

        self.unparse_resource_tokens(tokens);
    }

    fn unparse_def_tokens<'a>(&'a self, tokens: &mut Vec<Token<'a>>) {
        // Definitions in scope (existing ones first)
        // Use iterator chains to efficiently collect all definition tokens
        tokens.extend(
            self.stop
                .stop_defs
                .iter()
                .map(|(id, v)| {
                    (
                        *id,
                        Token::Header {
                            name: format!("STOP{id}").into(),
                            args: v.to_string().into(),
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        tokens.extend(
            self.video
                .seek_defs
                .iter()
                .map(|(id, v)| {
                    (
                        *id,
                        Token::Header {
                            name: format!("SEEK{id}").into(),
                            args: v.to_string().into(),
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        tokens.extend(
            self.scroll
                .scroll_defs
                .iter()
                .map(|(id, v)| {
                    (
                        *id,
                        Token::Header {
                            name: format!("SCROLL{id}").into(),
                            args: v.to_string().into(),
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        tokens.extend(
            self.speed
                .speed_defs
                .iter()
                .map(|(id, v)| {
                    (
                        *id,
                        Token::Header {
                            name: format!("SPEED{id}").into(),
                            args: v.to_string().into(),
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        tokens.extend(
            self.text
                .texts
                .iter()
                .map(|(id, text)| {
                    (
                        *id,
                        Token::Header {
                            name: format!("TEXT{id}").into(),
                            args: text.as_str().into(),
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        tokens.extend(
            self.judge
                .exrank_defs
                .iter()
                .map(|(id, exrank)| {
                    (
                        *id,
                        Token::Header {
                            name: format!("EXRANK{id}").into(),
                            args: exrank.judge_level.to_string().into(),
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        {
            tokens.extend(
                self.wav
                    .exwav_defs
                    .iter()
                    .map(|(id, def)| {
                        (
                            *id,
                            def.frequency.map_or_else(
                                || Token::Header {
                                    name: format!("EXWAV{id}").into(),
                                    args: format!(
                                        "pv {} {} {}",
                                        def.pan.as_ref(),
                                        def.volume.as_ref(),
                                        def.path.display()
                                    )
                                    .into(),
                                },
                                |freq| Token::Header {
                                    name: format!("EXWAV{id}").into(),
                                    args: format!(
                                        "pvf {} {} {} {}",
                                        def.pan.as_ref(),
                                        def.volume.as_ref(),
                                        freq.value(),
                                        def.path.display()
                                    )
                                    .into(),
                                },
                            ),
                        )
                    })
                    .collect::<BTreeMap<_, _>>()
                    .into_values(),
            );

            // wavcmd_events should be sorted by wav_index for consistent output
            let mut wavcmd_events: Vec<_> = self.wav.wavcmd_events.values().collect();
            wavcmd_events.sort_by_key(|ev| ev.wav_index);
            tokens.extend(wavcmd_events.into_iter().map(|ev| Token::Header {
                name: "WAVCMD".into(),
                args: format!("{} {} {}", ev.param.to_str(), ev.wav_index, ev.value).into(),
            }));

            tokens.extend(
                self.bmp
                    .atbga_defs
                    .iter()
                    .map(|(id, def)| {
                        (
                            *id,
                            Token::Header {
                                name: format!("@BGA{id}").into(),
                                args: format!(
                                    "{} {} {} {} {} {} {}",
                                    def.source_bmp,
                                    def.trim_top_left.x,
                                    def.trim_top_left.y,
                                    def.trim_size.width,
                                    def.trim_size.height,
                                    def.draw_point.x,
                                    def.draw_point.y,
                                )
                                .into(),
                            },
                        )
                    })
                    .collect::<BTreeMap<_, _>>()
                    .into_values(),
            );

            tokens.extend(
                self.bmp
                    .bga_defs
                    .iter()
                    .map(|(id, def)| {
                        (
                            *id,
                            Token::Header {
                                name: format!("BGA{id}").into(),
                                args: format!(
                                    "{} {} {} {} {} {} {}",
                                    def.source_bmp,
                                    def.trim_top_left.x,
                                    def.trim_top_left.y,
                                    def.trim_bottom_right.x,
                                    def.trim_bottom_right.y,
                                    def.draw_point.x,
                                    def.draw_point.y,
                                )
                                .into(),
                            },
                        )
                    })
                    .collect::<BTreeMap<_, _>>()
                    .into_values(),
            );

            tokens.extend(
                self.bmp
                    .argb_defs
                    .iter()
                    .map(
                        |(
                            id,
                            Argb {
                                alpha,
                                red,
                                green,
                                blue,
                            },
                        )| {
                            (
                                *id,
                                Token::Header {
                                    name: format!("ARGB{id}").into(),
                                    args: format!("{alpha},{red},{green},{blue}").into(),
                                },
                            )
                        },
                    )
                    .collect::<BTreeMap<_, _>>()
                    .into_values(),
            );

            // SWBGA events, sorted by ObjId for consistent output
            let mut swbga_events: Vec<_> = self.bmp.swbga_events.iter().collect();
            swbga_events.sort_by_key(|(id, _)| *id);
            tokens.extend(
                swbga_events
                    .into_iter()
                    .map(|(id, SwBgaEvent { frame_rate, total_time, line, loop_mode, argb: Argb { alpha, red, green, blue }, pattern })| Token::Header {
                        name: format!("SWBGA{id}").into(),
                        args: format!(
                            "{frame_rate}:{total_time}:{line}:{}:{alpha},{red},{green},{blue} {pattern}",
                            if *loop_mode { 1 } else { 0 }
                        ).into(),
                    }),
            );
        }
    }

    fn unparse_resource_tokens<'a>(&'a self, tokens: &mut Vec<Token<'a>>) {
        // Resources - Use iterator chains to efficiently collect resource tokens
        // Add basic resource tokens
        if let Some(path_root) = self.metadata.wav_path_root.as_ref() {
            tokens.push(Token::Header {
                name: "PATH_WAV".into(),
                args: path_root.display().to_string().into(),
            });
        }

        {
            if let Some(midi_file) = self.resources.midi_file.as_ref()
                && !midi_file.as_path().as_os_str().is_empty()
            {
                tokens.push(Token::Header {
                    name: "MIDIFILE".into(),
                    args: midi_file.display().to_string().into(),
                });
            }
            if let Some(materials_wav) = self.resources.materials_wav.first()
                && !materials_wav.as_path().as_os_str().is_empty()
            {
                tokens.push(Token::Header {
                    name: "MATERIALSWAV".into(),
                    args: materials_wav.display().to_string().into(),
                });
            }
        }

        if let Some(video_file) = self.video.video_file.as_ref()
            && !video_file.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::Header {
                name: "VIDEOFILE".into(),
                args: video_file.display().to_string().into(),
            });
        }

        {
            if let Some(colors) = self.video.video_colors {
                tokens.push(Token::Header {
                    name: "VIDEOCOLORS".into(),
                    args: colors.to_string().into(),
                });
            }
            if let Some(delay) = self.video.video_dly.as_ref() {
                tokens.push(Token::Header {
                    name: "VIDEODLY".into(),
                    args: delay.to_string().into(),
                });
            }
            if let Some(fps) = self.video.video_fs.as_ref() {
                tokens.push(Token::Header {
                    name: "VIDEOF/S".into(),
                    args: fps.to_string().into(),
                });
            }
            if let Some(char_file) = self.sprite.char_file.as_ref()
                && !char_file.as_path().as_os_str().is_empty()
            {
                tokens.push(Token::Header {
                    name: "CHARFILE".into(),
                    args: char_file.display().to_string().into(),
                });
            }
            if let Some(materials_bmp) = self.resources.materials_bmp.first()
                && !materials_bmp.as_path().as_os_str().is_empty()
            {
                tokens.push(Token::Header {
                    name: "MATERIALSBMP".into(),
                    args: materials_bmp.display().to_string().into(),
                });
            }
        }

        // VolWav as an expansion command
        if self.volume.volume != Volume::default() {
            tokens.push(Token::Header {
                name: "VOLWAV".into(),
                args: self.volume.volume.relative_percent.to_string().into(),
            });
        }
    }

    fn unparse_messages<'a, T: KeyLayoutMapper>(
        &'a self,
        tokens: &mut Vec<Token<'a>>,
        checker: &mut Base62Checker,
    ) {
        // Collect late definition tokens and message tokens
        let mut late_def_tokens: Vec<Token<'a>> = Vec::new();
        let mut message_tokens: Vec<Token<'a>> = Vec::new();

        // Messages: Section length - Use iterator chain to collect tokens (sorted by track for consistent output)
        let mut section_len_tokens: Vec<_> = self
            .section_len
            .section_len_changes
            .values()
            .map(|obj| Token::Message {
                track: obj.track,
                channel: Channel::SectionLen,
                message: Cow::Owned(obj.length.to_string()),
            })
            .collect();
        section_len_tokens.sort_by_key(|token| match token {
            Token::Message { track, .. } => *track,
            _ => Track(0),
        });
        message_tokens.extend(section_len_tokens);

        // Helper closures for mapping definitions
        // Note: We use f64::to_bits() as key since FinF64 doesn't implement Hash

        let bpm_value_to_id: HashMap<u64, ObjId> = self
            .bpm
            .bpm_defs
            .iter()
            .map(|(k, v)| {
                (
                    v.value()
                        .as_ref()
                        .expect("parsed value should be valid")
                        .as_f64()
                        .to_bits(),
                    *k,
                )
            })
            .collect();
        let stop_value_to_id: HashMap<u64, ObjId> = self
            .stop
            .stop_defs
            .iter()
            .map(|(k, v)| {
                (
                    v.value()
                        .as_ref()
                        .expect("parsed value should be valid")
                        .as_f64()
                        .to_bits(),
                    *k,
                )
            })
            .collect();
        let scroll_value_to_id: HashMap<u64, ObjId> = self
            .scroll
            .scroll_defs
            .iter()
            .map(|(k, v)| {
                (
                    v.value()
                        .as_ref()
                        .expect("parsed value should be valid")
                        .as_f64()
                        .to_bits(),
                    *k,
                )
            })
            .collect();
        let speed_value_to_id: HashMap<u64, ObjId> = self
            .speed
            .speed_defs
            .iter()
            .map(|(k, v)| {
                (
                    v.value()
                        .as_ref()
                        .expect("parsed value should be valid")
                        .as_f64()
                        .to_bits(),
                    *k,
                )
            })
            .collect();
        let text_value_to_id: HashMap<&'a str, ObjId> = self
            .text
            .texts
            .iter()
            .map(|(k, v)| (v.as_str(), *k))
            .collect();
        let exrank_value_to_id: HashMap<&'a JudgeLevel, ObjId> = self
            .judge
            .exrank_defs
            .iter()
            .map(|(k, v)| (&v.judge_level, *k))
            .collect();

        let seek_value_to_id: HashMap<u64, ObjId> = self
            .video
            .seek_defs
            .iter()
            .map(|(k, v)| {
                (
                    v.value()
                        .as_ref()
                        .expect("parsed value should be valid")
                        .as_f64()
                        .to_bits(),
                    *k,
                )
            })
            .collect();

        // Messages: BPM change (#xxx08 or #xxx03)
        let mut bpm_message_tokens = Vec::new();

        // Process U8 type BPM changes
        let EventProcessingResult {
            message_tokens: bpm_u8_message_tokens,
            ..
        } = build_event_messages(
            self.bpm.bpm_changes_u8.iter(),
            None::<(
                fn(ObjId, &()) -> Token,
                fn(&_) -> &(),
                &mut ObjIdManager<()>,
            )>,
            |_ev| Channel::BpmChangeU8,
            |bpm, _id| {
                let s = format!("{:02X}", bpm);
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            },
        );
        bpm_message_tokens.extend(bpm_u8_message_tokens);

        // Process other type BPM changes using build_event_messages_owned
        let mut bpm_manager: HashMap<u64, ObjId> = bpm_value_to_id;
        let EventProcessingResult {
            late_def_tokens: other_late_def_tokens,
            message_tokens: other_message_tokens,
        } = build_event_messages_owned(
            self.bpm.bpm_changes.iter(),
            Some((
                |id, bpm_bits: &u64| Token::Header {
                    name: format!("BPM{id}").into(),
                    args: f64::from_bits(*bpm_bits).to_string().into(),
                },
                |ev: &'a BpmChangeObj| ev.bpm.as_f64().to_bits(),
                &mut bpm_manager,
            )),
            |_ev| Channel::BpmChange,
            |_ev, id| {
                let id = id.unwrap_or(ObjId::null());
                id.into_chars()
            },
        );

        // Update id_manager with the results
        late_def_tokens.extend(other_late_def_tokens);
        bpm_message_tokens.extend(other_message_tokens);

        message_tokens.extend(bpm_message_tokens);

        // Messages: STOP (#xxx09)
        let mut stop_manager: HashMap<u64, ObjId> = stop_value_to_id;
        let EventProcessingResult {
            late_def_tokens: stop_late_def_tokens,
            message_tokens: stop_message_tokens,
        } = build_event_messages_owned(
            self.stop.stops.iter(),
            Some((
                |id, duration_bits: &u64| Token::Header {
                    name: format!("STOP{id}").into(),
                    args: f64::from_bits(*duration_bits).to_string().into(),
                },
                |ev: &'a StopObj| ev.duration.as_f64().to_bits(),
                &mut stop_manager,
            )),
            |_ev| Channel::Stop,
            |_ev, id| {
                let id = id.unwrap_or(ObjId::null());
                id.into_chars()
            },
        );
        late_def_tokens.extend(stop_late_def_tokens);
        message_tokens.extend(stop_message_tokens);

        // Messages: SCROLL (#xxxSC)
        let mut scroll_manager: HashMap<u64, ObjId> = scroll_value_to_id;
        let EventProcessingResult {
            late_def_tokens: scroll_late_def_tokens,
            message_tokens: scroll_message_tokens,
        } = build_event_messages_owned(
            self.scroll.scrolling_factor_changes.iter(),
            Some((
                |id, factor_bits: &u64| Token::Header {
                    name: format!("SCROLL{id}").into(),
                    args: f64::from_bits(*factor_bits).to_string().into(),
                },
                |ev: &'a ScrollingFactorObj| ev.factor.as_f64().to_bits(),
                &mut scroll_manager,
            )),
            |_ev| Channel::Scroll,
            |_ev, id| {
                let id = id.unwrap_or(ObjId::null());
                id.into_chars()
            },
        );
        late_def_tokens.extend(scroll_late_def_tokens);
        message_tokens.extend(scroll_message_tokens);

        // Messages: SPEED (#xxxSP)
        let mut speed_manager: HashMap<u64, ObjId> = speed_value_to_id;
        let EventProcessingResult {
            late_def_tokens: speed_late_def_tokens,
            message_tokens: speed_message_tokens,
        } = build_event_messages_owned(
            self.speed.speed_factor_changes.iter(),
            Some((
                |id, factor_bits: &u64| Token::Header {
                    name: format!("SPEED{id}").into(),
                    args: f64::from_bits(*factor_bits).to_string().into(),
                },
                |ev: &'a SpeedObj| ev.factor.as_f64().to_bits(),
                &mut speed_manager,
            )),
            |_ev| Channel::Speed,
            |_ev, id| {
                let id = id.unwrap_or(ObjId::null());
                id.into_chars()
            },
        );
        late_def_tokens.extend(speed_late_def_tokens);
        message_tokens.extend(speed_message_tokens);

        {
            // STP events, sorted by time for consistent output
            let mut stp_events: Vec<_> = self.stop.stp_events.values().collect();
            stp_events.sort_by_key(|ev| ev.time);
            tokens.extend(stp_events.into_iter().map(|ev| {
                Token::Header {
                    name: "STP".into(),
                    args: format!(
                        "{:03}.{:03} {}",
                        ev.time.track(),
                        ev.time.numerator() * ev.time.denominator_u64() / 1000,
                        ev.duration.as_millis()
                    )
                    .into(),
                }
            }));
        }

        // Messages: BGA changes (#xxx04/#xxx07/#xxx06/#xxx0A)
        let EventProcessingResult {
            message_tokens: bga_message_tokens,
            ..
        } = build_event_messages(
            self.bmp.bga_changes.iter(),
            None::<(
                fn(ObjId, &'a ()) -> Token<'a>,
                fn(&_) -> &'a (),
                &mut ObjIdManager<()>,
            )>,
            |bga| bga.layer.to_channel(),
            |bga, _id| {
                let s = bga.id.to_string();
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            },
        );
        message_tokens.extend(bga_message_tokens);

        {
            // Messages: BGA opacity changes (#xxx0B/#xxx0C/#xxx0D/#xxx0E)
            for (layer, opacity_changes) in &self.bmp.bga_opacity_changes {
                let EventProcessingResult {
                    message_tokens: opacity_message_tokens,
                    ..
                } = build_event_messages(
                    opacity_changes.iter(),
                    None::<(
                        fn(ObjId, &'a ()) -> Token<'a>,
                        fn(&_) -> &'a (),
                        &mut ObjIdManager<()>,
                    )>,
                    |_ev| match layer {
                        BgaLayer::Base => Channel::BgaBaseOpacity,
                        BgaLayer::Poor => Channel::BgaPoorOpacity,
                        BgaLayer::Overlay => Channel::BgaLayerOpacity,
                        BgaLayer::Overlay2 => Channel::BgaLayer2Opacity,
                    },
                    |ev, _id| {
                        let s = format!("{:02X}", ev.opacity);
                        let mut chars = s.chars();
                        [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
                    },
                );
                message_tokens.extend(opacity_message_tokens);
            }

            // Messages: BGA ARGB changes (#xxxA1/#xxxA2/#xxxA3/#xxxA4)
            for (layer, argb_changes) in &self.bmp.bga_argb_changes {
                let EventProcessingResult {
                    message_tokens: argb_message_tokens,
                    ..
                } = build_event_messages(
                    argb_changes.iter(),
                    None::<(
                        fn(ObjId, &'a ()) -> Token<'a>,
                        fn(&_) -> &'a (),
                        &mut ObjIdManager<()>,
                    )>,
                    |_ev| match layer {
                        BgaLayer::Base => Channel::BgaBaseArgb,
                        BgaLayer::Poor => Channel::BgaPoorArgb,
                        BgaLayer::Overlay => Channel::BgaLayerArgb,
                        BgaLayer::Overlay2 => Channel::BgaLayer2Argb,
                    },
                    |ev, _id| {
                        let s = format!("{:02X}", ev.argb.alpha);
                        let mut chars = s.chars();
                        [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
                    },
                );
                message_tokens.extend(argb_message_tokens);
            }
        }

        // Messages: BGM (#xxx01) and Notes (various #xx)
        // Use build_event_messages to process note and BGM objects
        // We need to preserve the original insertion order, so we process each object individually
        let EventProcessingResult {
            message_tokens: notes_message_tokens,
            ..
        } = build_event_messages(
            self.wav
                .notes
                .all_notes_insertion_order()
                .map(|obj| (&obj.offset, obj)),
            None::<(
                fn(ObjId, &()) -> Token,
                fn(&_) -> &(),
                &mut ObjIdManager<()>,
            )>,
            |obj| {
                // Channel mapping: determine channel based on channel_id
                obj.channel_id
                    .try_into_map::<T>()
                    .map_or(Channel::Bgm, |_map| Channel::Note {
                        channel_id: obj.channel_id,
                    })
            },
            |obj, _id| {
                let s = obj.wav_id.to_string();
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            }, // Message formatting: use wav_id
        );

        message_tokens.extend(notes_message_tokens);

        // Messages: BGM volume (#97)
        let EventProcessingResult {
            message_tokens: bgm_volume_message_tokens,
            ..
        } = build_event_messages(
            self.volume.bgm_volume_changes.iter(),
            None::<(
                fn(ObjId, &'a ()) -> Token<'a>,
                fn(&_) -> &'a (),
                &mut ObjIdManager<()>,
            )>,
            |_ev| Channel::BgmVolume,
            |ev, _id| {
                let s = format!("{:02X}", ev.volume);
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            },
        );
        message_tokens.extend(bgm_volume_message_tokens);

        // Messages: KEY volume (#98)
        let EventProcessingResult {
            message_tokens: key_volume_message_tokens,
            ..
        } = build_event_messages(
            self.volume.key_volume_changes.iter(),
            None::<(
                fn(ObjId, &'a ()) -> Token<'a>,
                fn(&_) -> &'a (),
                &mut ObjIdManager<()>,
            )>,
            |_ev| Channel::KeyVolume,
            |ev, _id| {
                let s = format!("{:02X}", ev.volume);
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            },
        );
        message_tokens.extend(key_volume_message_tokens);

        // Messages: TEXT (#99)
        let mut text_manager =
            ObjIdManager::from_entries(text_value_to_id.iter().map(|(k, v)| (*k, *v)));
        let EventProcessingResult {
            late_def_tokens: text_late_def_tokens,
            message_tokens: text_message_tokens,
        } = build_event_messages(
            self.text.text_events.iter(),
            Some((
                |id, text: &'a str| Token::Header {
                    name: format!("TEXT{id}").into(),
                    args: text.into(),
                },
                |ev: &'a TextObj| ev.text.as_str(),
                &mut text_manager,
            )),
            |_ev| Channel::Text,
            |_ev, id| {
                let id = id.unwrap_or(ObjId::null());
                id.into_chars()
            },
        );
        checker.check(text_manager.into_assigned_ids());
        late_def_tokens.extend(text_late_def_tokens);
        message_tokens.extend(text_message_tokens);

        let mut exrank_manager =
            ObjIdManager::from_entries(exrank_value_to_id.iter().map(|(k, v)| (*k, *v)));
        let EventProcessingResult {
            late_def_tokens: judge_late_def_tokens,
            message_tokens: judge_message_tokens,
        } = build_event_messages(
            self.judge.judge_events.iter(),
            Some((
                |id, judge_level: &JudgeLevel| Token::Header {
                    name: format!("EXRANK{id}").into(),
                    args: judge_level.to_string().into(),
                },
                |ev: &'a JudgeObj| &ev.judge_level,
                &mut exrank_manager,
            )),
            |_ev| Channel::Judge,
            |_ev, id| {
                let id = id.unwrap_or(ObjId::null());
                id.into_chars()
            },
        );
        checker.check(exrank_manager.into_assigned_ids());
        late_def_tokens.extend(judge_late_def_tokens);
        message_tokens.extend(judge_message_tokens);

        if let Some(poor_bmp) = self.bmp.poor_bmp.as_ref()
            && !poor_bmp.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::Header {
                name: "BMP00".into(),
                args: poor_bmp.display().to_string().into(),
            });
        }

        tokens.extend(
            self.bmp
                .bmp_files
                .iter()
                .filter(|(_, bmp)| !bmp.file.as_path().as_os_str().is_empty())
                .map(|(id, bmp)| {
                    (
                        *id,
                        if bmp.transparent_color == Argb::default() {
                            Token::Header {
                                name: format!("BMP{id}").into(),
                                args: bmp.file.display().to_string().into(),
                            }
                        } else {
                            Token::Header {
                                name: format!("EXBMP{id}").into(),
                                args: format!(
                                    "{},{},{},{} {}",
                                    bmp.transparent_color.alpha,
                                    bmp.transparent_color.red,
                                    bmp.transparent_color.green,
                                    bmp.transparent_color.blue,
                                    bmp.file.display()
                                )
                                .into(),
                            }
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        {
            // Messages: SEEK (#xxx05)
            let mut seek_manager: HashMap<u64, ObjId> = seek_value_to_id;
            let EventProcessingResult {
                late_def_tokens: seek_late_def_tokens,
                message_tokens: seek_message_tokens,
            } = build_event_messages_owned(
                self.video.seek_events.iter(),
                Some((
                    |id, position_bits: &u64| Token::Header {
                        name: format!("SEEK{id}").into(),
                        args: f64::from_bits(*position_bits).to_string().into(),
                    },
                    |ev: &'a SeekObj| ev.position.as_f64().to_bits(),
                    &mut seek_manager,
                )),
                |_ev| Channel::Seek,
                |_ev, id| {
                    let id = id.unwrap_or(ObjId::null());
                    let s = id.to_string();
                    let mut chars = s.chars();
                    [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
                },
            );
            late_def_tokens.extend(seek_late_def_tokens);
            message_tokens.extend(seek_message_tokens);

            // Messages: BGA keybound (#xxxA5)
            let EventProcessingResult {
                message_tokens: bga_keybound_message_tokens,
                ..
            } = build_event_messages(
                self.bmp.bga_keybound_events.iter(),
                None::<(
                    fn(ObjId, &'a ()) -> Token<'a>,
                    fn(&_) -> &'a (),
                    &mut ObjIdManager<()>,
                )>,
                |_ev| Channel::BgaKeybound,
                |ev, _id| {
                    let s = format!("{:02X}", ev.event.line);
                    let mut chars = s.chars();
                    [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
                },
            );
            message_tokens.extend(bga_keybound_message_tokens);

            // Messages: OPTION (#xxxA6)
            let EventProcessingResult {
                message_tokens: option_message_tokens,
                ..
            } = build_event_messages(
                self.option.option_events.iter(),
                None::<(
                    fn(ObjId, &'a ()) -> Token<'a>,
                    fn(&_) -> &'a (),
                    &mut ObjIdManager<()>,
                )>,
                |_ev| Channel::OptionChange,
                |_ev, _id| {
                    let s = format!("{:02X}", 0);
                    let mut chars = s.chars();
                    [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
                }, // Option events don't use values
            );
            checker.check(seek_manager.values().copied());
            message_tokens.extend(option_message_tokens);
        };

        // Assembly: header/definitions/resources/others -> late definitions -> messages
        if !late_def_tokens.is_empty() {
            tokens.extend(late_def_tokens);
        }
        if !message_tokens.is_empty() {
            tokens.extend(message_tokens);
        }
    }
}

#[derive(Debug, Default)]
struct Base62Checker {
    using_base62: bool,
}

impl Base62Checker {
    const fn new() -> Self {
        Self {
            using_base62: false,
        }
    }

    fn check(&mut self, iter: impl IntoIterator<Item = ObjId>) {
        if !self.using_base62 && iter.into_iter().any(|id| !id.is_base36() && id.is_base62()) {
            self.using_base62 = true;
        }
    }

    const fn into_using_base62(self) -> bool {
        self.using_base62
    }
}

/// A unit of event processing containing all necessary information for token generation
#[derive(Debug, Clone)]
struct EventUnit<'a, Event> {
    time: ObjTime,
    event: &'a Event,
    channel: Channel,
    id: Option<ObjId>,
}

/// Complete result from `build_messages_event` containing all processing outputs
struct EventProcessingResult<'a> {
    message_tokens: Vec<Token<'a>>,
    late_def_tokens: Vec<Token<'a>>,
}

/// Generic function to process message types with optional ID allocation
///
/// This function processes time-indexed events from an iterator and converts them into message tokens.
/// It supports both ID allocation mode (using `token_creator` and `key_extractor`) and direct mode (without ID allocation).
///
/// # PROCESSING FLOW OVERVIEW:
/// 1. **GROUP EVENTS**: Events are grouped by track, channel, and non-strictly increasing time
/// 2. **SPLIT INTO MESSAGE SEGMENTS**: Each group is further split into message segments with stricter rules:
///    - Strictly increasing time (prevents overlaps)
///    - Consistent denominators (ensures accurate representation)
/// 3. **GENERATE TOKENS**: Each message segment becomes one `Token::Message` with all events encoded
///
/// Arguments:
///     events: An iterator yielding (&time, &event) pairs to process
///     `id_allocation`: Optional tuple containing (`token_creator`, `key_extractor`, `id_manager`) for ID allocation mode
///     `channel_mapper`: Function to map events to channels
///     `message_formatter`: Function to format events into [char; 2]
///
/// Returns:
///     `EventProcessingResult` containing `message_tokens`, `late_def_tokens`, and updated maps
///
/// The function leverages Rust's iterator chains for efficient processing and supports
/// both ID-based and direct value-based event processing.
fn build_event_messages<
    'a,
    Event: 'a,
    Key: 'a + ?Sized + std::hash::Hash + Eq,
    EventIterator,
    TokenCreator,
    KeyExtractor,
    ChannelMapper,
    MessageFormatter,
>(
    event_iter: EventIterator,
    mut id_allocation: Option<(TokenCreator, KeyExtractor, &mut ObjIdManager<'a, Key>)>,
    channel_mapper: ChannelMapper,
    message_formatter: MessageFormatter,
) -> EventProcessingResult<'a>
where
    EventIterator: Iterator<Item = (&'a ObjTime, &'a Event)>,
    TokenCreator: Fn(ObjId, &'a Key) -> Token<'a>,
    KeyExtractor: Fn(&'a Event) -> &'a Key,
    ChannelMapper: Fn(&'a Event) -> Channel,
    MessageFormatter: Fn(&'a Event, Option<ObjId>) -> [char; 2],
{
    let mut late_def_tokens: Vec<Token<'a>> = Vec::new();

    // Process events based on whether id_allocation tuple is provided
    // Keep original order from event_iter instead of grouping by track/channel
    let processed_events: Vec<EventUnit<'a, Event>> = event_iter
        .map(|(&time, event)| {
            let id = id_allocation
                .as_mut()
                .and_then(|(token_creator, key_extractor, manager)| {
                    let key = key_extractor(event);
                    let is_assigned = manager.is_assigned(key);
                    let id = manager.get_or_new_id(key);
                    if !is_assigned && let Some(new) = id {
                        late_def_tokens.push(token_creator(new, key));
                    }
                    id
                });
            EventUnit {
                time,
                event,
                channel: channel_mapper(event),
                id,
            }
        })
        .collect();

    // === STEP 1: GROUP EVENTS BY TRACK, CHANNEL, AND TIME ===
    // Group events by adjacent same track, channel and non-strictly increasing time
    //
    // This creates the first level of grouping where events that share:
    // - Preserve the original event iterator order
    // - Same track number
    // - Same channel type
    // - Non-strictly increasing time (last_time <= current_time)
    // ...are grouped together. This is the foundation for efficient message generation.
    let grouped_events = group_events_by_track_channel_time(processed_events);

    // === STEP 2: SPLIT GROUPS INTO MESSAGE SEGMENTS ===
    // Split each group into message segments based on time ordering and denominator consistency
    //
    // This creates the second level of grouping with stricter rules:
    // - Not preserve the original event iterator order
    // - Time must be strictly increasing (last_time < current_time)
    // - Denominators must be the same starting from the second element
    // - First element can have 0 numerator, or the same denominator as elements after it
    //
    // The purpose is to ensure that events within a message segment can be represented
    // in a single message string without conflicts or information loss.
    let message_segmented_events: Vec<Vec<_>> = grouped_events
        .into_iter()
        .flat_map(split_group_into_message_segments)
        .collect();

    // === STEP 3: GENERATE MESSAGE TOKENS FROM MESSAGE SEGMENTS ===
    // Generate message tokens: each message segment generates one Token::Message
    //
    // This is the final step where each message segment is converted into a single Token::Message.
    // The process ensures that all events in a message segment are represented in one message string
    // with correct timing and without information loss.
    let message_tokens: Vec<Token<'a>> = message_segmented_events
        .into_iter()
        .map(|message_segment| {
            convert_message_segment_to_token(message_segment, &message_formatter)
        })
        .collect();

    EventProcessingResult {
        message_tokens,
        late_def_tokens,
    }
}

/// A version of `build_event_messages` that supports owned keys (like u64 instead of &u64)
fn build_event_messages_owned<
    'a,
    Event: 'a,
    Key: std::hash::Hash + Eq + Clone,
    EventIterator,
    TokenCreator,
    KeyExtractor,
    ChannelMapper,
    MessageFormatter,
>(
    event_iter: EventIterator,
    mut id_allocation: Option<(TokenCreator, KeyExtractor, &mut HashMap<Key, ObjId>)>,
    channel_mapper: ChannelMapper,
    message_formatter: MessageFormatter,
) -> EventProcessingResult<'a>
where
    EventIterator: Iterator<Item = (&'a ObjTime, &'a Event)>,
    TokenCreator: Fn(ObjId, &Key) -> Token<'a>,
    KeyExtractor: Fn(&'a Event) -> Key,
    ChannelMapper: Fn(&'a Event) -> Channel,
    MessageFormatter: Fn(&'a Event, Option<ObjId>) -> [char; 2],
{
    let mut late_def_tokens: Vec<Token<'a>> = Vec::new();

    // Process events based on whether id_allocation tuple is provided
    let processed_events: Vec<EventUnit<'a, Event>> = event_iter
        .map(|(&time, event)| {
            let id = id_allocation
                .as_mut()
                .and_then(|(token_creator, key_extractor, manager)| {
                    let key = key_extractor(event);
                    let _is_assigned = manager.contains_key(&key);

                    manager.get(&key).copied().or_else(|| {
                        let new_id =
                            ObjId::all_values().find(|id| !manager.values().any(|&v| v == *id));
                        if let Some(new_id) = new_id {
                            manager.insert(key.clone(), new_id);
                            late_def_tokens.push(token_creator(new_id, &key));
                        }
                        new_id
                    })
                });
            EventUnit {
                time,
                event,
                channel: channel_mapper(event),
                id,
            }
        })
        .collect();

    let grouped_events = group_events_by_track_channel_time(processed_events);
    let message_segmented_events: Vec<Vec<_>> = grouped_events
        .into_iter()
        .flat_map(split_group_into_message_segments)
        .collect();
    let message_tokens: Vec<Token<'a>> = message_segmented_events
        .into_iter()
        .map(|message_segment| {
            convert_message_segment_to_token(message_segment, &message_formatter)
        })
        .collect();

    EventProcessingResult {
        message_tokens,
        late_def_tokens,
    }
}

/// Group events by track, channel, and non-strictly increasing time
fn group_events_by_track_channel_time<'a, Event>(
    processed_events: Vec<EventUnit<'a, Event>>,
) -> Vec<Vec<EventUnit<'a, Event>>> {
    let mut groups = Vec::new();
    let mut current_group = Vec::new();

    for event_unit in processed_events {
        let should_join = current_group
            .last()
            .map(|last_unit: &EventUnit<'a, Event>| {
                event_unit.time.track() == last_unit.time.track()
                    && last_unit.channel == event_unit.channel
                    && last_unit.time <= event_unit.time
            })
            .unwrap_or(false);

        if should_join {
            current_group.push(event_unit);
        } else {
            if !current_group.is_empty() {
                groups.push(current_group);
            }
            current_group = vec![event_unit];
        }
    }

    if !current_group.is_empty() {
        groups.push(current_group);
    }
    groups
}

/// Split a group into message segments based on time ordering and denominator consistency
fn split_group_into_message_segments<'a, Event>(
    group: Vec<EventUnit<'a, Event>>,
) -> Vec<Vec<EventUnit<'a, Event>>> {
    let mut message_segments = Vec::new();
    let mut current_message_segment = Vec::new();

    for event_unit in group {
        let should_join = current_message_segment
            .last()
            .map(|last_unit: &EventUnit<'a, Event>| {
                // MESSAGE SEGMENT JOINING RULES:
                // 1. Time must be strictly increasing (prevents overlapping events)
                // 2. Denominators must be compatible:
                //    - If current message segment is empty, accept any denominator
                //    - Otherwise, denominators must share a factor relationship (either is a factor of the other)
                //    - Reference denominator is the maximum denominator currently in the message segment
                (last_unit.time < event_unit.time)
                    && (current_message_segment.is_empty()
                        || is_denominator_compatible(&event_unit, &current_message_segment))
            })
            .unwrap_or(true); // Empty message segment always accepts the first event

        if should_join {
            current_message_segment.push(event_unit);
        } else {
            if !current_message_segment.is_empty() {
                message_segments.push(current_message_segment);
            }
            current_message_segment = vec![event_unit];
        }
    }

    if !current_message_segment.is_empty() {
        message_segments.push(current_message_segment);
    }
    message_segments
}

/// Check if an event unit's denominator is compatible with the current message segment
/// Two denominators are compatible if either is a factor of the other
fn is_denominator_compatible<'a, Event>(
    event_unit: &EventUnit<'a, Event>,
    message_segment: &[EventUnit<'a, Event>],
) -> bool {
    // Find the maximum denominator from the current message segment as reference
    let reference_denominator = message_segment
        .iter()
        .map(|unit| unit.time.denominator_u64())
        .max()
        .unwrap_or(1);

    // Check if the event unit's denominator shares a common factor relationship
    let event_denominator = event_unit.time.denominator_u64();
    reference_denominator.is_multiple_of(event_denominator)
        || event_denominator.is_multiple_of(reference_denominator)
}

/// Convert a message segment of events into a single `Token::Message`
fn convert_message_segment_to_token<'a, Event, MessageFormatter>(
    message_segment: Vec<EventUnit<'a, Event>>,
    message_formatter: &MessageFormatter,
) -> Token<'a>
where
    MessageFormatter: Fn(&'a Event, Option<ObjId>) -> [char; 2],
{
    if message_segment.is_empty() {
        return Token::Message {
            track: Track(0),
            channel: Channel::Bgm,
            message: Cow::Borrowed(""),
        };
    }

    // EXTRACT METADATA FROM MESSAGE SEGMENT
    // All events in message segment should have same track and channel (guaranteed by grouping logic)
    let Some(first_event) = message_segment.first() else {
        return Token::Message {
            track: Track(0),
            channel: Channel::Bgm,
            message: Cow::Borrowed(""),
        };
    };
    let (track, channel) = (first_event.time.track(), first_event.channel);

    // CALCULATE MESSAGE LENGTH
    // Find the least common multiple (LCM) of all denominators to determine message length - this ensures
    // all events in the message segment can be accurately positioned in the message string.
    // Example: if we have events at 1/3 and 1/5, LCM(3,5)=15, so we need length 15 to represent them both accurately.
    let denominators: Vec<u64> = message_segment
        .iter()
        .map(|event_unit| event_unit.time.denominator_u64())
        .collect();
    let lcm_denom = lcm_slice(&denominators);

    let message_len = lcm_denom as usize;
    let mut message_parts: Vec<String> = vec!["00".to_string(); message_len];

    // PLACE EVENTS IN MESSAGE STRING
    // For each event in the message segment, calculate its exact position in the message
    // and place its value there. The time_idx calculation converts fractional time
    // to array index using the formula: (numerator * lcm_denom / denominator)
    for event_unit in message_segment {
        let EventUnit {
            event, id, time, ..
        } = event_unit;
        let chars = message_formatter(event, id);
        let denom_u64 = time.denominator_u64();

        // Calculate exact position: convert fraction to index in the message array
        // Example: time=3/4, lcm_denom=4: (3 * 4 / 4) = 3, so place at index 3
        // Example: time=1/3, lcm_denom=15: (1 * 15 / 3) = 5, so place at index 5
        let time_idx = (time.numerator() * (lcm_denom / denom_u64)) as usize;

        // Ensure we don't go out of bounds (safety check)
        let Some(slot) = message_parts.get_mut(time_idx) else {
            continue;
        };
        *slot = chars.iter().collect::<String>();
    }

    Token::Message {
        track,
        channel,
        message: Cow::Owned(message_parts.join("")),
    }
}

/// Calculate the least common multiple (LCM) of a slice of u64 values
/// Returns 1 if the slice is empty
fn lcm_slice(denominators: &[u64]) -> u64 {
    denominators.iter().fold(1, |acc, denom| acc.lcm(denom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lcm_slice() {
        // Test empty slice
        assert_eq!(lcm_slice(&[]), 1);

        // Test single value
        assert_eq!(lcm_slice(&[3]), 3);
        assert_eq!(lcm_slice(&[5]), 5);

        // Test two values
        assert_eq!(lcm_slice(&[3, 5]), 15);
        assert_eq!(lcm_slice(&[4, 6]), 12);
        assert_eq!(lcm_slice(&[2, 4, 8]), 8);

        // Test multiple values
        assert_eq!(lcm_slice(&[2, 3, 4]), 12);
        assert_eq!(lcm_slice(&[3, 5, 7]), 105);
        assert_eq!(lcm_slice(&[6, 8, 10]), 120);

        // Test with 1
        assert_eq!(lcm_slice(&[1, 3]), 3);
        assert_eq!(lcm_slice(&[3, 1]), 3);
    }
}
