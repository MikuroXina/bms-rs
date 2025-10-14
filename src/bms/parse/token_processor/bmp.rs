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
use std::ops::ControlFlow;

/// It processes `#BMPxx`, `#BGAxx` and `#@BGAxx` definitions and objects on `BgaBase`, `BgaLayer`, `BgaPoor`, `BgaLayer2` and so on channels.
pub struct BmpProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for BmpProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> ControlFlow<Result<()>> {
        match name.to_ascii_uppercase().as_str() {
            bmp if bmp.starts_with("BMP") => {
                let id = &name["BMP".len()..];
                if args.is_empty() {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected image filename".into(),
                    )));
                }
                let path = Path::new(args);
                if id == "00" {
                    self.0.borrow_mut().graphics.poor_bmp = Some(path.into());
                    return ControlFlow::Break(Ok(()));
                }

                let bmp_obj_id =
                    match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                        Ok(v) => v,
                        Err(e) => return ControlFlow::Break(Err(e)),
                    };
                let to_insert = Bmp {
                    file: path.into(),
                    transparent_color: Argb::default(),
                };
                if let Some(older) = self.0.borrow_mut().graphics.bmp_files.get_mut(&bmp_obj_id) {
                    if let Err(e) = self
                        .1
                        .handle_def_duplication(DefDuplication::Bmp {
                            id: bmp_obj_id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, bmp_obj_id)
                    {
                        return ControlFlow::Break(Err(e));
                    }
                } else {
                    self.0
                        .borrow_mut()
                        .graphics
                        .bmp_files
                        .insert(bmp_obj_id, to_insert);
                    return ControlFlow::Break(Ok(()));
                }
            }
            exbmp if exbmp.starts_with("EXBMP") => {
                let id = &name["EXBMP".len()..];

                let args: Vec<_> = args.split_whitespace().collect();
                if args.len() != 2 {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(format!(
                        "expected 2 arguments but got {args:?}",
                    ))));
                }

                let parts: Vec<&str> = args[0].split(',').collect();
                if parts.len() != 4 {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected 4 comma-separated values".into(),
                    )));
                }
                let alpha = match parts[0].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "invalid alpha value".into(),
                        )));
                    }
                };
                let red = match parts[1].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "invalid red value".into(),
                        )));
                    }
                };
                let green = match parts[2].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "invalid green value".into(),
                        )));
                    }
                };
                let blue = match parts[3].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "invalid blue value".into(),
                        )));
                    }
                };
                let transparent_color = Argb {
                    alpha,
                    red,
                    green,
                    blue,
                };

                let path = args[1];
                let bmp_obj_id =
                    match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                        Ok(v) => v,
                        Err(e) => return ControlFlow::Break(Err(e)),
                    };
                let to_insert = Bmp {
                    file: path.into(),
                    transparent_color,
                };
                if let Some(older) = self.0.borrow_mut().graphics.bmp_files.get_mut(&bmp_obj_id) {
                    if let Err(e) = self
                        .1
                        .handle_def_duplication(DefDuplication::Bmp {
                            id: bmp_obj_id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, bmp_obj_id)
                    {
                        return ControlFlow::Break(Err(e));
                    }
                } else {
                    self.0
                        .borrow_mut()
                        .graphics
                        .bmp_files
                        .insert(bmp_obj_id, to_insert);
                    return ControlFlow::Break(Ok(()));
                }
            }
            #[cfg(feature = "minor-command")]
            argb if argb.starts_with("ARGB") => {
                let id = &name["ARGB".len()..];
                let parts: Vec<_> = args.split(',').collect();
                if parts.len() != 4 {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected 4 comma-separated values".into(),
                    )));
                }
                let alpha = match parts[0].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected u8 alpha value".into(),
                        )));
                    }
                };
                let red = match parts[1].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected u8 red value".into(),
                        )));
                    }
                };
                let green = match parts[2].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected u8 green value".into(),
                        )));
                    }
                };
                let blue = match parts[3].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected u8 blue value".into(),
                        )));
                    }
                };
                let id = match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                    Ok(v) => v,
                    Err(e) => return ControlFlow::Break(Err(e)),
                };
                let argb = Argb {
                    alpha,
                    red,
                    green,
                    blue,
                };

                if let Some(older) = self.0.borrow_mut().scope_defines.argb_defs.get_mut(&id) {
                    if let Err(e) = self
                        .1
                        .handle_def_duplication(DefDuplication::BgaArgb {
                            id,
                            older,
                            newer: &argb,
                        })
                        .apply_def(older, argb, id)
                    {
                        return ControlFlow::Break(Err(e));
                    }
                } else {
                    self.0.borrow_mut().scope_defines.argb_defs.insert(id, argb);
                    return ControlFlow::Break(Ok(()));
                }
            }
            "POORBGA" => match PoorMode::from_str(args) {
                Ok(mode) => {
                    self.0.borrow_mut().graphics.poor_bga_mode = mode;
                    return ControlFlow::Break(Ok(()));
                }
                Err(e) => return ControlFlow::Break(Err(e)),
            },
            #[cfg(feature = "minor-command")]
            atbga if atbga.starts_with("@BGA") => {
                let id = &name["@BGA".len()..];
                let args: Vec<_> = args.split_whitespace().collect();
                if args.len() != 7 {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(format!(
                        "expected 7 arguments but found: {args:?}"
                    ))));
                }

                let sx = match args[1].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
                let sy = match args[2].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
                let w = match args[3].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
                let h = match args[4].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
                let dx = match args[5].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
                let dy = match args[6].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
                let id = match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                    Ok(v) => v,
                    Err(e) => return ControlFlow::Break(Err(e)),
                };
                let source_bmp =
                    match ObjId::try_from(args[0], self.0.borrow().header.case_sensitive_obj_id) {
                        Ok(v) => v,
                        Err(e) => return ControlFlow::Break(Err(e)),
                    };
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
                    if let Err(e) = self
                        .1
                        .handle_def_duplication(DefDuplication::AtBga {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, id)
                    {
                        return ControlFlow::Break(Err(e));
                    }
                } else {
                    self.0
                        .borrow_mut()
                        .scope_defines
                        .atbga_defs
                        .insert(id, to_insert);
                    return ControlFlow::Break(Ok(()));
                }
            }
            #[cfg(feature = "minor-command")]
            bga if bga.starts_with("BGA") && !bga.starts_with("BGAPOOR") => {
                let id = &name["BGA".len()..];
                let args: Vec<_> = args.split_whitespace().collect();
                if args.len() != 7 {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(format!(
                        "expected 7 arguments but found: {args:?}"
                    ))));
                }
                let x1 = match args[1].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
                let y1 = match args[2].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
                let x2 = match args[3].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
                let y2 = match args[4].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
                let dx = match args[5].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
                let dy = match args[6].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
                let id = match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                    Ok(v) => v,
                    Err(e) => return ControlFlow::Break(Err(e)),
                };
                let source_bmp =
                    match ObjId::try_from(args[0], self.0.borrow().header.case_sensitive_obj_id) {
                        Ok(v) => v,
                        Err(e) => return ControlFlow::Break(Err(e)),
                    };
                let to_insert = BgaDef {
                    id,
                    source_bmp,
                    trim_top_left: PixelPoint::new(x1, y1),
                    trim_bottom_right: PixelPoint::new(x2, y2),
                    draw_point: PixelPoint::new(dx, dy),
                };
                if let Some(older) = self.0.borrow_mut().scope_defines.bga_defs.get_mut(&id) {
                    if let Err(e) = self
                        .1
                        .handle_def_duplication(DefDuplication::Bga {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, id)
                    {
                        return ControlFlow::Break(Err(e));
                    }
                } else {
                    self.0
                        .borrow_mut()
                        .scope_defines
                        .bga_defs
                        .insert(id, to_insert);
                    return ControlFlow::Break(Ok(()));
                }
            }

            #[cfg(feature = "minor-command")]
            swbga if swbga.starts_with("SWBGA") => {
                let id = &name[5..];
                let args: Vec<_> = args.split_whitespace().collect();
                if args.len() != 2 {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(format!(
                        "expected 2 arguments but found: {args:?}"
                    ))));
                }

                // Parse fr:time:line:loop:a,r,g,b pattern
                let mut parts = args[0].split(':');
                let frame_rate = match parts.next() {
                    Some(v) => match v.parse() {
                        Ok(v) => v,
                        Err(_) => {
                            return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                                "swbga frame_rate u32".into(),
                            )));
                        }
                    },
                    None => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "swbga frame_rate".into(),
                        )));
                    }
                };
                let total_time = match parts.next() {
                    Some(v) => match v.parse() {
                        Ok(v) => v,
                        Err(_) => {
                            return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                                "swbga total_time u32".into(),
                            )));
                        }
                    },
                    None => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "swbga total_time".into(),
                        )));
                    }
                };
                let line = match parts.next() {
                    Some(v) => match v.parse() {
                        Ok(v) => v,
                        Err(_) => {
                            return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                                "swbga line u8".into(),
                            )));
                        }
                    },
                    None => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "swbga line".into(),
                        )));
                    }
                };
                let loop_mode_raw = match parts.next() {
                    Some(v) => match v.parse::<u8>() {
                        Ok(v) => v,
                        Err(_) => {
                            return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                                "swbga loop 0/1".into(),
                            )));
                        }
                    },
                    None => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "swbga loop".into(),
                        )));
                    }
                };
                let loop_mode = match loop_mode_raw {
                    0 => false,
                    1 => true,
                    _ => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "swbga loop 0/1".into(),
                        )));
                    }
                };
                let argb_str = match parts.next() {
                    Some(v) => v,
                    None => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "swbga argb".into(),
                        )));
                    }
                };
                let argb_parts: Vec<_> = argb_str.split(',').collect();
                if argb_parts.len() != 4 {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "swbga argb 4 values".into(),
                    )));
                }
                let alpha = match argb_parts[0].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "swbga argb alpha".into(),
                        )));
                    }
                };
                let red = match argb_parts[1].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "swbga argb red".into(),
                        )));
                    }
                };
                let green = match argb_parts[2].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "swbga argb green".into(),
                        )));
                    }
                };
                let blue = match argb_parts[3].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "swbga argb blue".into(),
                        )));
                    }
                };

                let pattern = args[1].to_owned();
                let sw_obj_id =
                    match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                        Ok(v) => v,
                        Err(e) => return ControlFlow::Break(Err(e)),
                    };
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
                    if let Err(e) = self
                        .1
                        .handle_def_duplication(DefDuplication::SwBgaEvent {
                            id: sw_obj_id,
                            older,
                            newer: &ev,
                        })
                        .apply_def(older, ev, sw_obj_id)
                    {
                        return ControlFlow::Break(Err(e));
                    }
                } else {
                    self.0
                        .borrow_mut()
                        .scope_defines
                        .swbga_events
                        .insert(sw_obj_id, ev);
                    return ControlFlow::Break(Ok(()));
                }
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> ControlFlow<Result<()>> {
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
                        return ControlFlow::Break(Err(ParseWarning::UndefinedObject(obj)));
                    }
                    let layer = BgaLayer::from_channel(channel)
                        .unwrap_or_else(|| panic!("Invalid channel for BgaLayer: {channel:?}"));
                    if let Err(e) = self.0.borrow_mut().graphics.push_bga_change(
                        BgaObj {
                            time,
                            id: obj,
                            layer,
                        },
                        channel,
                        self.1,
                    ) {
                        return ControlFlow::Break(Err(e));
                    }
                }
                return ControlFlow::Break(Ok(()));
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
                    if let Err(e) = self.0.borrow_mut().graphics.push_bga_opacity_change(
                        BgaOpacityObj {
                            time,
                            layer,
                            opacity: opacity_value,
                        },
                        channel,
                        self.1,
                    ) {
                        return ControlFlow::Break(Err(e));
                    }
                }
                return ControlFlow::Break(Ok(()));
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
                    let argb = match self
                        .0
                        .borrow()
                        .scope_defines
                        .argb_defs
                        .get(&argb_id)
                        .cloned()
                    {
                        Some(v) => v,
                        None => {
                            return ControlFlow::Break(Err(ParseWarning::UndefinedObject(argb_id)));
                        }
                    };
                    if let Err(e) = self.0.borrow_mut().graphics.push_bga_argb_change(
                        BgaArgbObj { time, layer, argb },
                        channel,
                        self.1,
                    ) {
                        return ControlFlow::Break(Err(e));
                    }
                }
                return ControlFlow::Break(Ok(()));
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
                        .ok_or(ParseWarning::UndefinedObject(keybound_id));
                    let event = match event {
                        Ok(v) => v,
                        Err(e) => return ControlFlow::Break(Err(e)),
                    };
                    if let Err(e) = self
                        .0
                        .borrow_mut()
                        .notes
                        .push_bga_keybound_event(BgaKeyboundObj { time, event }, self.1)
                    {
                        return ControlFlow::Break(Err(e));
                    }
                }
                return ControlFlow::Break(Ok(()));
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }
}
