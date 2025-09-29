use std::{cell::RefCell, path::Path, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::{
    ParseWarning, Result, filter_message, hex_values_from_message, ids_from_message,
    prompt::{DefDuplication, Prompter},
};
use crate::bms::{model::Bms, prelude::*};

/// A processor of tokens in the BMS. An implementation takes control only one feature about definitions and placements such as `WAVxx` definition and its sound object.
///
/// There are some invariants on calling:
///
/// - Once `on_message` is called, `one_header` must not be invoked after that.
/// - The effects of called `on_message` must be same regardless order of calls.
pub trait TokenProcessor {
    /// Processes a header command consists of `#{name} {args}`.
    fn on_header(&self, name: &str, args: &str) -> Result<()>;
    /// Processes a message command consists of `#{track}{channel}:{message}`.
    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()>;
}

/// It processes `#WAVxx` and `#LNOBJ` definitions and objects on `Bgm` and `Note` channels.
pub struct WavProcessor<'a, P, T>(Rc<RefCell<Bms<T>>>, &'a P);

impl<P: Prompter, T: KeyLayoutMapper> TokenProcessor for WavProcessor<'_, P, T> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name.to_uppercase().starts_with("WAV") {
            let id = name.trim_start_matches("WAV");
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError(
                    "expected key audio filename".into(),
                ));
            }
            let path = Path::new(args);
            let wav_obj_id = ObjId::try_from(id).map_err(|id| {
                ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
            })?;
            let mut bms = self.0.borrow_mut();
            if let Some(older) = bms.notes.wav_files.get_mut(&wav_obj_id) {
                self.1
                    .handle_def_duplication(DefDuplication::Wav {
                        id: wav_obj_id,
                        older,
                        newer: path,
                    })
                    .apply_def(older, path.into(), wav_obj_id)?;
            } else {
                bms.notes.wav_files.insert(wav_obj_id, path.into());
            }
        }
        if name == "LNOBJ" {
            let end_id = ObjId::try_from(args).map_err(|id| {
                ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
            })?;
            let mut end_note = self
                .0
                .borrow_mut()
                .notes
                .pop_latest_of(end_id)
                .ok_or(ParseWarning::UndefinedObject(end_id))?;
            let WavObj {
                offset, channel_id, ..
            } = &end_note;
            let begin_idx = self
                .0
                .borrow()
                .notes
                .notes_in(..offset)
                .rev()
                .find(|(_, obj)| obj.channel_id == *channel_id)
                .ok_or_else(|| {
                    ParseWarning::SyntaxError(format!(
                        "expected preceding object for #LNOBJ {end_id:?}",
                    ))
                })
                .map(|(index, _)| index)?;
            let mut begin_note =
                self.0
                    .borrow_mut()
                    .notes
                    .pop_by_idx(begin_idx)
                    .ok_or_else(|| {
                        ParseWarning::SyntaxError(format!(
                            "Cannot find begin note for LNOBJ {end_id:?}"
                        ))
                    })?;

            let mut begin_note_tuple = begin_note
                .channel_id
                .try_into_map::<T>()
                .ok_or_else(|| {
                    ParseWarning::SyntaxError(format!(
                        "channel of specified note for LNOBJ cannot become LN {end_id:?}"
                    ))
                })?
                .as_tuple();
            begin_note_tuple.1 = NoteKind::Long;
            begin_note.channel_id = T::from_tuple(begin_note_tuple).to_channel_id();
            self.0.borrow_mut().notes.push_note(begin_note);

            let mut end_note_tuple = end_note
                .channel_id
                .try_into_map::<T>()
                .ok_or_else(|| {
                    ParseWarning::SyntaxError(format!(
                        "channel of specified note for LNOBJ cannot become LN {end_id:?}"
                    ))
                })?
                .as_tuple();
            end_note_tuple.1 = NoteKind::Long;
            end_note.channel_id = T::from_tuple(end_note_tuple).to_channel_id();
            self.0.borrow_mut().notes.push_note(end_note);
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if let Channel::Bgm = channel {
            for (time, obj) in ids_from_message(track, message, |w| self.1.warn(w)) {
                self.0.borrow_mut().notes.push_bgm(time, obj);
            }
        }
        if let Channel::Note { channel_id } = channel {
            for (offset, obj) in ids_from_message(track, message, |w| self.1.warn(w)) {
                self.0.borrow_mut().notes.push_note(WavObj {
                    offset,
                    channel_id,
                    wav_id: obj,
                });
            }
        }
        Ok(())
    }
}

/// It processes `#BPM` and `#BPMxx` definitions and objects on `BpmChange` and `BpmChangeU8` channels.
pub struct BpmProcessor<'a, P>(Rc<RefCell<Bms>>, &'a P);

impl<P: Prompter> TokenProcessor for BpmProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name == "BPM" {
            let bpm = Decimal::from_fraction(
                GenericFraction::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected decimal BPM".into()))?,
            );
            self.0.borrow_mut().arrangers.bpm = Some(bpm);
        } else if name.starts_with("BPM") || name.starts_with("EXBPM") {
            let id = if name.starts_with("BPM") {
                name.trim_start_matches("BPM")
            } else {
                name.trim_start_matches("EXBPM")
            };
            let bpm_obj_id = ObjId::try_from(id).map_err(|id| {
                ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
            })?;
            let bpm = Decimal::from_fraction(
                GenericFraction::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected decimal BPM".into()))?,
            );
            let scope_defines = &mut self.0.borrow_mut().scope_defines;
            if let Some(older) = scope_defines.bpm_defs.get_mut(&bpm_obj_id) {
                self.1
                    .handle_def_duplication(DefDuplication::BpmChange {
                        id: bpm_obj_id,
                        older: older.clone(),
                        newer: bpm.clone(),
                    })
                    .apply_def(older, bpm.clone(), bpm_obj_id)?;
            } else {
                scope_defines.bpm_defs.insert(bpm_obj_id, bpm.clone());
            }
        }
        #[cfg(feature = "minor-command")]
        if name == "BASEBPM" {
            let bpm = Decimal::from_fraction(
                GenericFraction::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected decimal BPM".into()))?,
            );
            self.0.borrow_mut().arrangers.base_bpm = Some(bpm);
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if let Channel::BpmChange = channel {
            for (time, obj) in ids_from_message(track, message, |w| self.1.warn(w)) {
                // Record used BPM change id for validity checks
                self.0
                    .borrow_mut()
                    .arrangers
                    .bpm_change_ids_used
                    .insert(obj);
                let bpm = self
                    .0
                    .borrow()
                    .scope_defines
                    .bpm_defs
                    .get(&obj)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(obj))?;
                self.0
                    .borrow_mut()
                    .arrangers
                    .push_bpm_change(BpmChangeObj { time, bpm }, self.1)?;
            }
        }
        if let Channel::BpmChangeU8 = channel {
            for (time, value) in hex_values_from_message(track, message, |w| self.1.warn(w)) {
                self.0.borrow_mut().arrangers.push_bpm_change(
                    BpmChangeObj {
                        time,
                        bpm: Decimal::from(value),
                    },
                    self.1,
                )?;
            }
        }
        Ok(())
    }
}

/// It processes `#SCROLLxx` definitions and objects on `Scroll` channel.
pub struct ScrollProcessor<'a, P>(Rc<RefCell<Bms>>, &'a P);

impl<P: Prompter> TokenProcessor for ScrollProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name.starts_with("SCROLL") {
            let id = name.trim_start_matches("SCROLL");
            let factor =
                Decimal::from_fraction(GenericFraction::from_str(args).map_err(|_| {
                    ParseWarning::SyntaxError("expected decimal scroll factor".into())
                })?);
            let scroll_obj_id = ObjId::try_from(id).map_err(|id| {
                ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
            })?;
            if let Some(older) = self
                .0
                .borrow_mut()
                .scope_defines
                .scroll_defs
                .get_mut(&scroll_obj_id)
            {
                self.1
                    .handle_def_duplication(DefDuplication::ScrollingFactorChange {
                        id: scroll_obj_id,
                        older: older.clone(),
                        newer: factor.clone(),
                    })
                    .apply_def(older, factor, scroll_obj_id)?;
            } else {
                self.0
                    .borrow_mut()
                    .scope_defines
                    .scroll_defs
                    .insert(scroll_obj_id, factor);
            }
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if let Channel::Scroll = channel {
            for (time, obj) in ids_from_message(track, message, |w| self.1.warn(w)) {
                let factor = self
                    .0
                    .borrow()
                    .scope_defines
                    .scroll_defs
                    .get(&obj)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(obj))?;
                self.0
                    .borrow_mut()
                    .arrangers
                    .push_scrolling_factor_change(ScrollingFactorObj { time, factor }, self.1)?;
            }
        }
        Ok(())
    }
}

/// It processes `#STOPxx` definitions and objects on `Stop` channel.
pub struct StopProcessor<'a, P>(Rc<RefCell<Bms>>, &'a P);

impl<P: Prompter> TokenProcessor for StopProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name.starts_with("STOP") {
            let id = name.trim_start_matches("STOP");
            let len =
                Decimal::from_fraction(GenericFraction::from_str(args).map_err(|_| {
                    ParseWarning::SyntaxError("expected decimal stop length".into())
                })?);

            let stop_obj_id = ObjId::try_from(id).map_err(|id| {
                ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
            })?;

            if let Some(older) = self
                .0
                .borrow_mut()
                .scope_defines
                .stop_defs
                .get_mut(&stop_obj_id)
            {
                self.1
                    .handle_def_duplication(DefDuplication::Stop {
                        id: stop_obj_id,
                        older: older.clone(),
                        newer: len.clone(),
                    })
                    .apply_def(older, len, stop_obj_id)?;
            } else {
                self.0
                    .borrow_mut()
                    .scope_defines
                    .stop_defs
                    .insert(stop_obj_id, len);
            }
        }
        #[cfg(feature = "minor-command")]
        if name.starts_with("STP") {
            // Parse xxx.yyy zzzz
            use std::{num::NonZeroU64, time::Duration};
            let args: Vec<_> = args.split_whitespace().collect();
            if args.len() != 3 {
                return Err(ParseWarning::SyntaxError(
                    "stp measure/pos must be 3 digits".into(),
                ));
            }

            let (measure, pos) = args[0].split_once('.').unwrap_or((args[0], "000"));
            let measure: u16 = measure
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected measure u16".into()))?;
            let pos: u16 = pos
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected pos u16".into()))?;
            let ms: u64 = args[2]
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected pos u64".into()))?;
            let time = ObjTime::new(
                measure as u64,
                pos as u64,
                NonZeroU64::new(1000).expect("1000 should be a valid NonZeroU64"),
            );
            let duration = Duration::from_millis(ms);

            // Store by ObjTime as key, handle duplication with prompt handler
            let ev = StpEvent { time, duration };
            if let Some(older) = self.0.borrow_mut().arrangers.stp_events.get_mut(&time) {
                use crate::parse::prompt::ChannelDuplication;

                self.1
                    .handle_channel_duplication(ChannelDuplication::StpEvent {
                        time,
                        older,
                        newer: &ev,
                    })
                    .apply_channel(older, ev, time, Channel::Stop)?;
            } else {
                self.0.borrow_mut().arrangers.stp_events.insert(time, ev);
            }
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if let Channel::Stop = channel {
            for (time, obj) in ids_from_message(track, message, |w| self.1.warn(w)) {
                // Record used STOP id for validity checks
                self.0.borrow_mut().arrangers.stop_ids_used.insert(obj);
                let duration = self
                    .0
                    .borrow()
                    .scope_defines
                    .stop_defs
                    .get(&obj)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(obj))?;
                self.0.borrow_mut().arrangers.push_stop(StopObj {
                    time,
                    duration: duration.clone(),
                });
            }
        }
        Ok(())
    }
}

/// It processes `#SPEEDxx` definitions and objects on `Speed` channel.
pub struct SpeedProcessor<'a, P>(Rc<RefCell<Bms>>, &'a P);

impl<P: Prompter> TokenProcessor for SpeedProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name.starts_with("SPEED") {
            let id = name.trim_start_matches("SPEED");
            let factor = Decimal::from_fraction(GenericFraction::from_str(args).map_err(|_| {
                ParseWarning::SyntaxError(format!("expected decimal but found: {args}"))
            })?);
            let speed_obj_id = ObjId::try_from(id).map_err(|id| {
                ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
            })?;

            if let Some(older) = self
                .0
                .borrow_mut()
                .scope_defines
                .speed_defs
                .get_mut(&speed_obj_id)
            {
                self.1
                    .handle_def_duplication(DefDuplication::SpeedFactorChange {
                        id: speed_obj_id,
                        older: older.clone(),
                        newer: factor.clone(),
                    })
                    .apply_def(older, factor.clone(), speed_obj_id)?;
            } else {
                self.0
                    .borrow_mut()
                    .scope_defines
                    .speed_defs
                    .insert(speed_obj_id, factor.clone());
            }
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if let Channel::Speed = channel {
            for (time, obj) in ids_from_message(track, message, |w| self.1.warn(w)) {
                let factor = self
                    .0
                    .borrow()
                    .scope_defines
                    .speed_defs
                    .get(&obj)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(obj))?;
                self.0.borrow_mut().arrangers.push_speed_factor_change(
                    SpeedObj {
                        time,
                        factor: factor.clone(),
                    },
                    self.1,
                )?;
            }
        }
        Ok(())
    }
}

/// It processes objects on `SectionLen` channel.
pub struct SectionLenProcessor<'a, P>(Rc<RefCell<Bms>>, &'a P);

impl<P: Prompter> TokenProcessor for SectionLenProcessor<'_, P> {
    fn on_header(&self, _: &str, _: &str) -> Result<()> {
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if let Channel::SectionLen = channel {
            let message = filter_message(message);
            let message = message.as_ref();
            let length = Decimal::from(Decimal::from_fraction(
                GenericFraction::from_str(message).map_err(|_| {
                    ParseWarning::SyntaxError(format!("Invalid section length: {message}"))
                })?,
            ));
            if length <= Decimal::from(0u64) {
                return Err(ParseWarning::SyntaxError(
                    "section length must be greater than zero".to_string(),
                ));
            }
            self.0
                .borrow_mut()
                .arrangers
                .push_section_len_change(SectionLenChangeObj { track, length }, self.1)?;
        }
        Ok(())
    }
}

/// It processes `#BMPxx`, `#BGAxx` and `#@BGAxx` definitions and objects on `BgaBase`, `BgaLayer`, `BgaPoor`, `BgaLayer2` and so on channels.
pub struct BmpProcessor<'a, P>(Rc<RefCell<Bms>>, &'a P);

impl<P: Prompter> TokenProcessor for BmpProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        match name {
            bmp if bmp.starts_with("BMP") => {
                let id = bmp.trim_start_matches("BMP");
                let path = Path::new(args);
                if id == "00" {
                    self.0.borrow_mut().graphics.poor_bmp = Some(path.into());
                    return Ok(());
                }

                let bmp_obj_id = ObjId::try_from(id).map_err(|id| {
                    ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
                })?;
                let to_insert = Bmp {
                    file: path.into(),
                    transparent_color: Argb::default(),
                };
                if let Some(older) = self.0.borrow_mut().graphics.bmp_files.get_mut(&bmp_obj_id) {
                    self.1
                        .handle_def_duplication(DefDuplication::Bmp {
                            id: bmp_obj_id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, bmp_obj_id)?;
                } else {
                    self.0
                        .borrow_mut()
                        .graphics
                        .bmp_files
                        .insert(bmp_obj_id, to_insert);
                }
            }
            exbmp if exbmp.starts_with("EXBMP") => {
                let id = exbmp.trim_start_matches("EXBMP");

                let args: Vec<_> = args.split_whitespace().collect();
                if args.len() != 2 {
                    return Err(ParseWarning::SyntaxError(format!(
                        "expected 2 arguments but got {args:?}",
                    )));
                }

                let parts: Vec<&str> = args[0].split(',').collect();
                if parts.len() != 4 {
                    return Err(ParseWarning::SyntaxError(
                        "expected 4 comma-separated values".into(),
                    ));
                }
                let alpha = parts[0]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("invalid alpha value".into()))?;
                let red = parts[1]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("invalid red value".into()))?;
                let green = parts[2]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("invalid green value".into()))?;
                let blue = parts[3]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("invalid blue value".into()))?;
                let transparent_color = Argb {
                    alpha,
                    red,
                    green,
                    blue,
                };

                let path = args[1];
                let bmp_obj_id = ObjId::try_from(id).map_err(|id| {
                    ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
                })?;
                let to_insert = Bmp {
                    file: path.into(),
                    transparent_color,
                };
                if let Some(older) = self.0.borrow_mut().graphics.bmp_files.get_mut(&bmp_obj_id) {
                    self.1
                        .handle_def_duplication(DefDuplication::Bmp {
                            id: bmp_obj_id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, bmp_obj_id)?;
                } else {
                    self.0
                        .borrow_mut()
                        .graphics
                        .bmp_files
                        .insert(bmp_obj_id, to_insert);
                }
            }
            "POORBGA" => {
                self.0.borrow_mut().graphics.poor_bga_mode = PoorMode::from_str(args)?;
            }
            #[cfg(feature = "minor-command")]
            atbga if atbga.starts_with("@BGA") => {
                let id = atbga.trim_start_matches("@BGA");
                let args: Vec<_> = args.split_whitespace().collect();
                if args.len() != 7 {
                    return Err(ParseWarning::SyntaxError(format!(
                        "expected 7 arguments but found: {args:?}"
                    )));
                }

                let sx = args[1]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                let sy = args[2]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                let w = args[3]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                let h = args[4]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                let dx = args[5]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                let dy = args[6]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                let id = ObjId::try_from(id).map_err(|id| {
                    ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
                })?;
                let source_bmp = ObjId::try_from(args[0]).map_err(|id| {
                    ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
                })?;
                let trim_top_left = (sx, sy);
                let trim_size = (w, h);
                let draw_point = (dx, dy);
                let to_insert = AtBgaDef {
                    id,
                    source_bmp,
                    trim_top_left: trim_top_left.to_owned().into(),
                    trim_size: trim_size.to_owned().into(),
                    draw_point: draw_point.to_owned().into(),
                };
                if let Some(older) = self.0.borrow_mut().scope_defines.atbga_defs.get_mut(&id) {
                    self.1
                        .handle_def_duplication(DefDuplication::AtBga {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, id)?;
                } else {
                    self.0
                        .borrow_mut()
                        .scope_defines
                        .atbga_defs
                        .insert(id, to_insert);
                }
            }
            #[cfg(feature = "minor-command")]
            bga if bga.starts_with("BGA") && !bga.starts_with("BGAPOOR") => {
                let id = bga.trim_start_matches("BGA");
                let args: Vec<_> = args.split_whitespace().collect();
                if args.len() != 7 {
                    return Err(ParseWarning::SyntaxError(format!(
                        "expected 7 arguments but found: {args:?}"
                    )));
                }

                let x1 = args[1]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                let y1 = args[2]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                let x2 = args[3]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                let y2 = args[4]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                let dx = args[5]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                let dy = args[6]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                let id = ObjId::try_from(id).map_err(|id| {
                    ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
                })?;
                let source_bmp = ObjId::try_from(args[0]).map_err(|id| {
                    ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
                })?;
                let to_insert = BgaDef {
                    id,
                    source_bmp,
                    trim_top_left: PixelPoint::new(x1, y1),
                    trim_bottom_right: PixelPoint::new(x2, y2),
                    draw_point: PixelPoint::new(dx, dy),
                };
                if let Some(older) = self.0.borrow_mut().scope_defines.bga_defs.get_mut(&id) {
                    self.1
                        .handle_def_duplication(DefDuplication::Bga {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, id)?;
                } else {
                    self.0
                        .borrow_mut()
                        .scope_defines
                        .bga_defs
                        .insert(id, to_insert);
                }
            }

            #[cfg(feature = "minor-command")]
            swbga if swbga.starts_with("SWBGA") => {
                let id = swbga.trim_start_matches("SWBGA");
                let args: Vec<_> = args.split_whitespace().collect();
                if args.len() != 2 {
                    return Err(ParseWarning::SyntaxError(format!(
                        "expected 2 arguments but found: {args:?}"
                    )));
                }

                // Parse fr:time:line:loop:a,r,g,b pattern
                let mut parts = args[0].split(':');
                let frame_rate = parts
                    .next()
                    .ok_or_else(|| ParseWarning::SyntaxError("swbga frame_rate".into()))?
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("swbga frame_rate u32".into()))?;
                let total_time = parts
                    .next()
                    .ok_or_else(|| ParseWarning::SyntaxError("swbga total_time".into()))?
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("swbga total_time u32".into()))?;
                let line = parts
                    .next()
                    .ok_or_else(|| ParseWarning::SyntaxError("swbga line".into()))?
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("swbga line u8".into()))?;
                let loop_mode = parts
                    .next()
                    .ok_or_else(|| ParseWarning::SyntaxError("swbga loop".into()))?
                    .parse::<u8>()
                    .map_err(|_| ParseWarning::SyntaxError("swbga loop 0/1".into()))?;
                let loop_mode = match loop_mode {
                    0 => false,
                    1 => true,
                    _ => return Err(ParseWarning::SyntaxError("swbga loop 0/1".into())),
                };
                let argb_str = parts
                    .next()
                    .ok_or_else(|| ParseWarning::SyntaxError("swbga argb".into()))?;
                let argb_parts: Vec<_> = argb_str.split(',').collect();
                if argb_parts.len() != 4 {
                    return Err(ParseWarning::SyntaxError("swbga argb 4 values".into()));
                }
                let alpha = argb_parts[0]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("swbga argb alpha".into()))?;
                let red = argb_parts[1]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("swbga argb red".into()))?;
                let green = argb_parts[2]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("swbga argb green".into()))?;
                let blue = argb_parts[3]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("swbga argb blue".into()))?;

                let pattern = args[1].to_owned();
                let sw_obj_id = ObjId::try_from(id).map_err(|id| {
                    ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
                })?;
                let ev = SwBgaEvent {
                    frame_rate,
                    total_time,
                    line,
                    loop_mode,
                    argb: Argb {
                        alpha,
                        red,
                        green,
                        blue,
                    },
                    pattern,
                };

                if let Some(older) = self
                    .0
                    .borrow_mut()
                    .scope_defines
                    .swbga_events
                    .get_mut(&sw_obj_id)
                {
                    self.1
                        .handle_def_duplication(DefDuplication::SwBgaEvent {
                            id: sw_obj_id,
                            older,
                            newer: &ev,
                        })
                        .apply_def(older, ev, sw_obj_id)?;
                } else {
                    self.0
                        .borrow_mut()
                        .scope_defines
                        .swbga_events
                        .insert(sw_obj_id, ev);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        match channel {
            channel @ (Channel::BgaBase
            | Channel::BgaPoor
            | Channel::BgaLayer
            | Channel::BgaLayer2) => {
                for (time, obj) in ids_from_message(track, message, |w| self.1.warn(w)) {
                    if !self.0.borrow().graphics.bmp_files.contains_key(&obj) {
                        return Err(ParseWarning::UndefinedObject(obj));
                    }
                    let layer = BgaLayer::from_channel(channel)
                        .unwrap_or_else(|| panic!("Invalid channel for BgaLayer: {channel:?}"));
                    self.0.borrow_mut().graphics.push_bga_change(
                        BgaObj {
                            time,
                            id: obj,
                            layer,
                        },
                        channel,
                        self.1,
                    )?;
                }
            }
            #[cfg(feature = "minor-command")]
            channel @ (Channel::BgaBaseOpacity
            | Channel::BgaLayerOpacity
            | Channel::BgaLayer2Opacity
            | Channel::BgaPoorOpacity) => {
                for (time, opacity_value) in
                    hex_values_from_message(track, message, |w| self.1.warn(w))
                {
                    let layer = BgaLayer::from_channel(channel)
                        .unwrap_or_else(|| panic!("Invalid channel for BgaLayer: {channel:?}"));
                    self.0.borrow_mut().graphics.push_bga_opacity_change(
                        BgaOpacityObj {
                            time,
                            layer,
                            opacity: opacity_value,
                        },
                        channel,
                        self.1,
                    )?;
                }
            }
            #[cfg(feature = "minor-command")]
            channel @ (Channel::BgaBaseArgb
            | Channel::BgaLayerArgb
            | Channel::BgaLayer2Argb
            | Channel::BgaPoorArgb) => {
                for (time, argb_id) in ids_from_message(track, message, |w| self.1.warn(w)) {
                    let layer = BgaLayer::from_channel(channel)
                        .unwrap_or_else(|| panic!("Invalid channel for BgaLayer: {channel:?}"));
                    let argb = self
                        .0
                        .borrow()
                        .scope_defines
                        .argb_defs
                        .get(&argb_id)
                        .cloned()
                        .ok_or(ParseWarning::UndefinedObject(argb_id))?;
                    self.0.borrow_mut().graphics.push_bga_argb_change(
                        BgaArgbObj { time, layer, argb },
                        channel,
                        self.1,
                    )?;
                }
            }
            #[cfg(feature = "minor-command")]
            Channel::BgaKeybound => {
                for (time, keybound_id) in ids_from_message(track, message, |w| self.1.warn(w)) {
                    let event = self
                        .0
                        .borrow()
                        .scope_defines
                        .swbga_events
                        .get(&keybound_id)
                        .cloned()
                        .ok_or(ParseWarning::UndefinedObject(keybound_id))?;
                    self.0
                        .borrow_mut()
                        .notes
                        .push_bga_keybound_event(BgaKeyboundObj { time, event }, self.1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
