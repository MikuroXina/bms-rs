//! This module handles the tokens:
//!
//! - `#BMP[00-ZZ] filename` - Image file definition. The black will be transparent.
//! - `#BGA[00-ZZ] bmp_index crop_top_left_x crop_top_left_y crop_bottom_right_x crop_bottom_right_y draw_top_left_x draw_top_left_y` - Cropped image definition.
//! - `#@BGA[00-ZZ] bmp_index crop_top_left_x crop_top_;eft_y crop_width crop_height draw_top_left_x draw_top_left_y` - Cropped image definition.
//! - `#EXBMP[00-ZZ] a,r,g,b filename` - Image file definition with the color to be transparent.
//! - `#POORBGA mode` / `#BGAPOOR mode` - Display option for POOR (MISS) image.
//! - `#xxx04:` - Base layer channel of BGA.
//! - `#xxx06:` - Poor layer channel of BGA.
//! - `#xxx07:` - Overlay layer channel of BGA.
//! - `#xxx0A:` - Secondary overlay layer channel of BGA.
//! - `#xxx0B:` - Opacity [01-FF] of base layer channel of BGA.
//! - `#xxx0C:` - Opacity [01-FF] of overlay layer channel of BGA.
//! - `#xxx0D:` - Opacity [01-FF] of secondary overlay layer channel of BGA.
//! - `#xxx0E:` - Opacity [01-FF] of poor channel of BGA.
//! - `#ARGB[01-ZZ] a,r,g,b` - Transparent color definition.
//! - `#xxxA1:` - Transparent color object channel for base layer of BGA.
//! - `#xxxA2:` - Transparent color object channel for overlay layer of BGA.
//! - `#xxxA3:` - Transparent color object channel for secondary overlay layer of BGA.
//! - `#xxxA4:` - Transparent color object channel for poor layer of BGA.
//! - `#SWBGA[01-ZZ] fr:time:line:loop:a,r,g,b pattern` - Key bound animated images.
//! - `#xxxA5:` - Key bound BGA animation trigger channel.

use std::{cell::RefCell, path::Path, rc::Rc, str::FromStr};

#[cfg(feature = "minor-command")]
use super::hex_values_from_message;
use super::{
    super::prompt::{DefDuplication, Prompter},
    ParseWarning, Result, TokenProcessor, ids_from_message,
};
use crate::bms::{model::Bms, prelude::*};

/// It processes `#BMPxx`, `#BGAxx` and `#@BGAxx` definitions and objects on `BgaBase`, `BgaLayer`, `BgaPoor`, `BgaLayer2` and so on channels.
pub struct BmpProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for BmpProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            bmp if bmp.starts_with("BMP") => {
                let id = &name["BMP".len()..];
                if args.is_empty() {
                    return Err(ParseWarning::SyntaxError("expected image filename".into()));
                }
                let path = Path::new(args);
                if id == "00" {
                    self.0.borrow_mut().graphics.poor_bmp = Some(path.into());
                    return Ok(());
                }

                let bmp_obj_id = ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id)?;
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
                let id = &name["EXBMP".len()..];

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
                let bmp_obj_id = ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id)?;
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
            #[cfg(feature = "minor-command")]
            argb if argb.starts_with("ARGB") => {
                let id = &name["ARGB".len()..];
                let parts: Vec<_> = args.split(',').collect();
                if parts.len() != 4 {
                    return Err(ParseWarning::SyntaxError(
                        "expected 4 comma-separated values".into(),
                    ));
                }
                let alpha = parts[0]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected u8 alpha value".into()))?;
                let red = parts[1]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected u8 red value".into()))?;
                let green = parts[2]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected u8 green value".into()))?;
                let blue = parts[3]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected u8 blue value".into()))?;
                let id = ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id)?;
                let argb = Argb {
                    alpha,
                    red,
                    green,
                    blue,
                };

                if let Some(older) = self.0.borrow_mut().scope_defines.argb_defs.get_mut(&id) {
                    self.1
                        .handle_def_duplication(DefDuplication::BgaArgb {
                            id,
                            older,
                            newer: &argb,
                        })
                        .apply_def(older, argb, id)?;
                } else {
                    self.0.borrow_mut().scope_defines.argb_defs.insert(id, argb);
                }
            }
            "POORBGA" => {
                self.0.borrow_mut().graphics.poor_bga_mode = PoorMode::from_str(args)?;
            }
            #[cfg(feature = "minor-command")]
            atbga if atbga.starts_with("@BGA") => {
                let id = &name["@BGA".len()..];
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
                let id = ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id)?;
                let source_bmp =
                    ObjId::try_from(args[0], self.0.borrow().header.case_sensitive_obj_id)?;
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
                let id = &name["BGA".len()..];
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
                let id = ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id)?;
                let source_bmp =
                    ObjId::try_from(args[0], self.0.borrow().header.case_sensitive_obj_id)?;
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
                let id = &name[5..];
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
                let sw_obj_id = ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id)?;
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
        let is_sensitive = self.0.borrow().header.case_sensitive_obj_id;
        match channel {
            channel @ (Channel::BgaBase
            | Channel::BgaPoor
            | Channel::BgaLayer
            | Channel::BgaLayer2) => {
                for (time, obj) in
                    ids_from_message(track, message, is_sensitive, |w| self.1.warn(w))
                {
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
                for (time, argb_id) in
                    ids_from_message(track, message, is_sensitive, |w| self.1.warn(w))
                {
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
                for (time, keybound_id) in
                    ids_from_message(track, message, is_sensitive, |w| self.1.warn(w))
                {
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
