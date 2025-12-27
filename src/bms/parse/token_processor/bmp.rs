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

use super::{ProcessContext, TokenProcessor, parse_obj_ids};
use crate::{
    bms::{
        ParseErrorWithRange,
        model::bmp::BmpObjects,
        parse::{
            Result,
            prompt::{DefDuplication, Prompter},
        },
        prelude::*,
    },
    util::StrExtension,
};

/// It processes `#BMPxx`, `#BGAxx` and `#@BGAxx` definitions and objects on `BgaBase`, `BgaLayer`, `BgaPoor`, `BgaLayer2` and so on channels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmpProcessor {
    case_sensitive_obj_id: Rc<RefCell<bool>>,
}

impl BmpProcessor {
    pub fn new(case_sensitive_obj_id: &Rc<RefCell<bool>>) -> Self {
        Self {
            case_sensitive_obj_id: Rc::clone(case_sensitive_obj_id),
        }
    }
}

impl TokenProcessor for BmpProcessor {
    type Output = BmpObjects;

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> core::result::Result<Self::Output, ParseErrorWithRange> {
        let mut objects = BmpObjects::default();
        ctx.all_tokens(|token, prompter| match token.content() {
            Token::Header { name, args } => Ok(self
                .on_header(name.as_ref(), args.as_ref(), prompter, &mut objects)
                .map(|()| Vec::new())
                .unwrap_or_else(|warn| vec![warn.into_wrapper(token)])),
            Token::Message {
                track,
                channel,
                message,
            } => Ok(self
                .on_message(
                    *track,
                    *channel,
                    message.as_ref().into_wrapper(token),
                    prompter,
                    &mut objects,
                )
                .unwrap_or_else(|warn| vec![warn.into_wrapper(token)])),
            Token::NotACommand(_) => Ok(Vec::new()),
        })?;
        Ok(objects)
    }
}

impl BmpProcessor {
    fn bga_layer(channel: Channel) -> Result<BgaLayer> {
        BgaLayer::from_channel(channel).ok_or(ParseWarning::SyntaxError(format!(
            "invalid channel for BgaLayer: {channel:?}",
        )))
    }

    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        objects: &mut BmpObjects,
    ) -> Result<()> {
        if let Some(id) = name.strip_prefix_ignore_case("BMP") {
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError("expected image filename".into()));
            }
            let path = Path::new(args);
            if id == "00" {
                objects.poor_bmp = Some(path.into());
                return Ok(());
            }

            let bmp_obj_id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;
            let to_insert = Bmp {
                file: path.into(),
                transparent_color: Argb::default(),
            };
            if let Some(older) = objects.bmp_files.get_mut(&bmp_obj_id) {
                prompter
                    .handle_def_duplication(DefDuplication::Bmp {
                        id: bmp_obj_id,
                        older,
                        newer: &to_insert,
                    })
                    .apply_def(older, to_insert, bmp_obj_id)?;
            } else {
                objects.bmp_files.insert(bmp_obj_id, to_insert);
            }
        }
        if let Some(id) = name.strip_prefix_ignore_case("EXBMP") {
            let args: Vec<_> = args.split_whitespace().collect();
            let [argb_spec, path] = args.as_slice() else {
                return Err(ParseWarning::SyntaxError(format!(
                    "expected 2 arguments but got {args:?}"
                )));
            };

            let parts: Vec<&str> = argb_spec.split(',').collect();
            let [alpha_s, red_s, green_s, blue_s] = parts.as_slice() else {
                return Err(ParseWarning::SyntaxError(
                    "expected 4 comma-separated values".into(),
                ));
            };
            let alpha = alpha_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("invalid alpha value".into()))?;
            let red = red_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("invalid red value".into()))?;
            let green = green_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("invalid green value".into()))?;
            let blue = blue_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("invalid blue value".into()))?;
            let transparent_color = Argb {
                alpha,
                red,
                green,
                blue,
            };

            let bmp_obj_id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;
            let to_insert = Bmp {
                file: path.into(),
                transparent_color,
            };
            if let Some(older) = objects.bmp_files.get_mut(&bmp_obj_id) {
                prompter
                    .handle_def_duplication(DefDuplication::Bmp {
                        id: bmp_obj_id,
                        older,
                        newer: &to_insert,
                    })
                    .apply_def(older, to_insert, bmp_obj_id)?;
            } else {
                objects.bmp_files.insert(bmp_obj_id, to_insert);
            }
        }
        if let Some(id) = name.strip_prefix_ignore_case("ARGB") {
            let parts: Vec<_> = args.split(',').collect();
            let [alpha_s, red_s, green_s, blue_s] = parts.as_slice() else {
                return Err(ParseWarning::SyntaxError(
                    "expected 4 comma-separated values".into(),
                ));
            };
            let alpha = alpha_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected u8 alpha value".into()))?;
            let red = red_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected u8 red value".into()))?;
            let green = green_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected u8 green value".into()))?;
            let blue = blue_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected u8 blue value".into()))?;
            let id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;
            let argb = Argb {
                alpha,
                red,
                green,
                blue,
            };

            if let Some(older) = objects.argb_defs.get_mut(&id) {
                prompter
                    .handle_def_duplication(DefDuplication::BgaArgb {
                        id,
                        older,
                        newer: &argb,
                    })
                    .apply_def(older, argb, id)?;
            } else {
                objects.argb_defs.insert(id, argb);
            }
        }
        if name.eq_ignore_ascii_case("POORBGA") {
            objects.poor_bga_mode = PoorMode::from_str(args)?;
        }
        if let Some(id) = name.strip_prefix_ignore_case("@BGA") {
            let args: Vec<_> = args.split_whitespace().collect();
            let [bmp_index, sx_s, sy_s, w_s, h_s, dx_s, dy_s] = args.as_slice() else {
                return Err(ParseWarning::SyntaxError(format!(
                    "expected 7 arguments but found: {args:?}"
                )));
            };

            let sx = sx_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let sy = sy_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let w = w_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let h = h_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let dx = dx_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let dy = dy_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;
            let source_bmp = ObjId::try_from(bmp_index, *self.case_sensitive_obj_id.borrow())?;
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
            if let Some(older) = objects.atbga_defs.get_mut(&id) {
                prompter
                    .handle_def_duplication(DefDuplication::AtBga {
                        id,
                        older,
                        newer: &to_insert,
                    })
                    .apply_def(older, to_insert, id)?;
            } else {
                objects.atbga_defs.insert(id, to_insert);
            }
        }
        if !name.starts_with_ignore_case("BGAPOOR")
            && let Some(id) = name.strip_prefix_ignore_case("BGA")
        {
            let args: Vec<_> = args.split_whitespace().collect();
            let [bmp_index, x1_s, y1_s, x2_s, y2_s, dx_s, dy_s] = args.as_slice() else {
                return Err(ParseWarning::SyntaxError(format!(
                    "expected 7 arguments but found: {args:?}"
                )));
            };

            let x1 = x1_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let y1 = y1_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let x2 = x2_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let y2 = y2_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let dx = dx_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let dy = dy_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;
            let source_bmp = ObjId::try_from(bmp_index, *self.case_sensitive_obj_id.borrow())?;
            let to_insert = BgaDef {
                id,
                source_bmp,
                trim_top_left: PixelPoint::new(x1, y1),
                trim_bottom_right: PixelPoint::new(x2, y2),
                draw_point: PixelPoint::new(dx, dy),
            };
            if let Some(older) = objects.bga_defs.get_mut(&id) {
                prompter
                    .handle_def_duplication(DefDuplication::Bga {
                        id,
                        older,
                        newer: &to_insert,
                    })
                    .apply_def(older, to_insert, id)?;
            } else {
                objects.bga_defs.insert(id, to_insert);
            }
        }
        if let Some(id) = name.strip_prefix_ignore_case("SWBGA") {
            let args: Vec<_> = args.split_whitespace().collect();
            let [spec, pattern] = args.as_slice() else {
                return Err(ParseWarning::SyntaxError(format!(
                    "expected 2 arguments but found: {args:?}"
                )));
            };

            // Parse fr:time:line:loop:a,r,g,b pattern
            let mut parts = spec.split(':');
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
            let [alpha_s, red_s, green_s, blue_s] = argb_parts.as_slice() else {
                return Err(ParseWarning::SyntaxError("swbga argb 4 values".into()));
            };
            let alpha = alpha_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("swbga argb alpha".into()))?;
            let red = red_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("swbga argb red".into()))?;
            let green = green_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("swbga argb green".into()))?;
            let blue = blue_s
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("swbga argb blue".into()))?;

            let pattern = pattern.to_string();
            let sw_obj_id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;
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

            if let Some(older) = objects.swbga_events.get_mut(&sw_obj_id) {
                prompter
                    .handle_def_duplication(DefDuplication::SwBgaEvent {
                        id: sw_obj_id,
                        older,
                        newer: &ev,
                    })
                    .apply_def(older, ev, sw_obj_id)?;
            } else {
                objects.swbga_events.insert(sw_obj_id, ev);
            }
        }
        Ok(())
    }

    fn on_message(
        &self,
        track: Track,
        channel: Channel,
        message: SourceRangeMixin<&str>,
        prompter: &impl Prompter,
        objects: &mut BmpObjects,
    ) -> Result<Vec<ParseWarningWithRange>> {
        let mut warnings: Vec<ParseWarningWithRange> = Vec::new();
        match channel {
            channel @ (Channel::BgaBase
            | Channel::BgaPoor
            | Channel::BgaLayer
            | Channel::BgaLayer2) => {
                let (pairs, w) = parse_obj_ids(track, &message, &self.case_sensitive_obj_id);
                warnings.extend(w);
                for (time, obj) in pairs {
                    if !objects.bmp_files.contains_key(&obj) {
                        return Err(ParseWarning::UndefinedObject(obj));
                    }
                    let layer = Self::bga_layer(channel)?;
                    objects.push_bga_change(
                        BgaObj {
                            time,
                            id: obj,
                            layer,
                        },
                        channel,
                        prompter,
                    )?;
                }
            }
            channel @ (Channel::BgaBaseOpacity
            | Channel::BgaLayerOpacity
            | Channel::BgaLayer2Opacity
            | Channel::BgaPoorOpacity) => {
                use super::parse_hex_values;
                let (pairs, w) = parse_hex_values(track, &message);
                warnings.extend(w);
                for (time, opacity_value) in pairs {
                    let layer = Self::bga_layer(channel)?;
                    objects.push_bga_opacity_change(
                        BgaOpacityObj {
                            time,
                            layer,
                            opacity: opacity_value,
                        },
                        channel,
                        prompter,
                    )?;
                }
            }
            channel @ (Channel::BgaBaseArgb
            | Channel::BgaLayerArgb
            | Channel::BgaLayer2Argb
            | Channel::BgaPoorArgb) => {
                use super::parse_obj_ids;
                let (pairs, w) = parse_obj_ids(track, &message, &self.case_sensitive_obj_id);
                warnings.extend(w);
                for (time, argb_id) in pairs {
                    let layer = Self::bga_layer(channel)?;
                    let argb = objects
                        .argb_defs
                        .get(&argb_id)
                        .cloned()
                        .ok_or(ParseWarning::UndefinedObject(argb_id))?;
                    objects.push_bga_argb_change(
                        BgaArgbObj { time, layer, argb },
                        channel,
                        prompter,
                    )?;
                }
            }
            Channel::BgaKeybound => {
                use super::parse_obj_ids;
                let (pairs, w) = parse_obj_ids(track, &message, &self.case_sensitive_obj_id);
                warnings.extend(w);
                for (time, keybound_id) in pairs {
                    let event = objects
                        .swbga_events
                        .get(&keybound_id)
                        .cloned()
                        .ok_or(ParseWarning::UndefinedObject(keybound_id))?;
                    objects.push_bga_keybound_event(BgaKeyboundObj { time, event }, prompter)?;
                }
            }
            _ => {}
        }
        Ok(warnings)
    }
}
