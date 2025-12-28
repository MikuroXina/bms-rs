//! This module handles the tokens:
//!
//! - `#BANNER image` - The banner image path. 300x80 resolution is expected.
//! - `#BACKBMP image` - The background image path shown on playing.
//! - `#STAGEFILE image` - The splashscreen image path shown on loading the score.
//! - `#EXTCHR sprite_no bmp_no start_x start_y end_x end_y [offset_x offset_y [x y]]` - Extended character definition. It modifies a BMS player's sprite. Almost unsupported.
//! - `#CHARFILE character` - The character CHP path shown at the side on playing.

use std::path::Path;

use super::{ProcessContext, TokenProcessor};
use crate::bms::ParseErrorWithRange;
use crate::bms::{
    model::sprite::Sprites,
    parse::{ParseWarning, Result},
    prelude::*,
};

/// It processes sprite headers such as `#STAGEFILE`, `#BANNER` and so on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpriteProcessor;

impl TokenProcessor for SpriteProcessor {
    type Output = Sprites;

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> core::result::Result<Self::Output, ParseErrorWithRange> {
        let mut sprites = Sprites::default();
        ctx.all_tokens(|token, _prompter| match token.content() {
            Token::Header { name, args } => {
                Ok(Self::on_header(name.as_ref(), args.as_ref(), &mut sprites)
                    .err()
                    .map(|warn| warn.into_wrapper(token)))
            }
            Token::Message { .. } | Token::NotACommand(_) => Ok(None),
        })?;
        Ok(sprites)
    }
}

impl SpriteProcessor {
    fn on_header(name: &str, args: &str, sprites: &mut Sprites) -> Result<()> {
        if name.eq_ignore_ascii_case("BANNER") {
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError("expected banner filename".into()));
            }
            sprites.banner = Some(Path::new(args).into());
        }
        if name.eq_ignore_ascii_case("BACKBMP") {
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError(
                    "expected backbmp filename".into(),
                ));
            }
            sprites.back_bmp = Some(Path::new(args).into());
        }
        if name.eq_ignore_ascii_case("STAGEFILE") {
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError(
                    "expected splashscreen image filename".into(),
                ));
            }
            sprites.stage_file = Some(Path::new(args).into());
        }
        if name.eq_ignore_ascii_case("EXTCHR") {
            // Allow multiple spaces between parameters
            let params: Vec<_> = args.split_whitespace().collect();
            if !(6..=10).contains(&params.len()) {
                return Err(ParseWarning::SyntaxError(
                    "params length must be between 6 and 10".into(),
                ));
            }
            let [
                sprite_num,
                bmp_num,
                start_x,
                start_y,
                end_x,
                end_y,
                rest @ ..,
            ] = params.as_slice()
            else {
                return Err(ParseWarning::SyntaxError(
                    "params length must be between 6 and 10".into(),
                ));
            };
            let sprite_num = sprite_num
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected sprite_num i32".into()))?;
            // BMPNum supports hexadecimal (e.g. 09/FF), also supports -1/-257, etc.
            let bmp_num = if let Some(stripped) = bmp_num.strip_prefix('-') {
                -stripped
                    .parse::<i32>()
                    .map_err(|_| ParseWarning::SyntaxError("expected bmp_num is i32".into()))?
            } else if let Some(hex) = bmp_num.strip_prefix("0x") {
                i32::from_str_radix(hex, 16)
                    .or_else(|_| bmp_num.parse())
                    .map_err(|_| {
                        ParseWarning::SyntaxError("expected bmp_num is i32 in hexadecimal".into())
                    })?
            } else if bmp_num.chars().all(|c| c.is_ascii_hexdigit()) {
                i32::from_str_radix(bmp_num, 16)
                    .or_else(|_| bmp_num.parse())
                    .map_err(|_| {
                        ParseWarning::SyntaxError("expected bmp_num is i32 in hexadecimal".into())
                    })?
            } else {
                bmp_num.parse().map_err(|_| {
                    ParseWarning::SyntaxError("expected bmp_num is i32 in hexadecimal".into())
                })?
            };
            let start_x = start_x
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected start_x is i32".into()))?;
            let start_y = start_y
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected start_y is i32".into()))?;
            let end_x = end_x
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected end_x is i32".into()))?;
            let end_y = end_y
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected end_y is i32".into()))?;
            // offsetX/offsetY are optional
            let offset_x = rest.first().and_then(|v| v.parse().ok());
            let offset_y = rest.get(1).and_then(|v| v.parse().ok());
            // x/y are optional, only present if offset exists
            let abs_x = rest.get(2).and_then(|v| v.parse().ok());
            let abs_y = rest.get(3).and_then(|v| v.parse().ok());
            let ev = ExtChrEvent {
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
            };
            sprites.extchr_events.push(ev);
        }
        if name.eq_ignore_ascii_case("CHARFILE") {
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError(
                    "expected character filename".into(),
                ));
            }
            sprites.char_file = Some(Path::new(args).into());
        }
        Ok(())
    }
}
