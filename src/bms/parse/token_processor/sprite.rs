//! This module handles the tokens:
//!
//! - `#BANNER image` - The banner image path. 300x80 resolution is expected.
//! - `#BACKBMP image` - The background image path shown on playing.
//! - `#STAGEFILE image` - The splashscreen image path shown on loading the score.
//! - `#EXTCHR sprite_no bmp_no start_x start_y end_x end_y [offset_x offset_y [x y]]` - Extended character definition. It modifies a BMS player's sprite. Almost unsupported.
//! - `#CHARFILE character` - The character CHP path shown at the side on playing.

use std::path::Path;

use super::{TokenProcessor, TokenProcessorResult, all_tokens};
use crate::bms::{error::Result, model::sprite::Sprites, prelude::*};

/// It processes sprite headers such as `#STAGEFILE`, `#BANNER` and so on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpriteProcessor;

impl TokenProcessor for SpriteProcessor {
    type Output = Sprites;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> TokenProcessorResult<Self::Output> {
        let mut sprites = Sprites::default();
        all_tokens(input, prompter, |token| {
            Ok(match token {
                Token::Header { name, args } => self
                    .on_header(name.as_ref(), args.as_ref(), &mut sprites)
                    .err(),
                Token::Message { .. } | Token::NotACommand(_) => None,
            })
        })?;
        Ok(sprites)
    }
}

impl SpriteProcessor {
    fn on_header(&self, name: &str, args: &str, sprites: &mut Sprites) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "BANNER" => {
                if args.is_empty() {
                    return Err(ParseWarning::SyntaxError("expected banner filename".into()));
                }
                sprites.banner = Some(Path::new(args).into());
            }
            "BACKBMP" => {
                if args.is_empty() {
                    return Err(ParseWarning::SyntaxError(
                        "expected backbmp filename".into(),
                    ));
                }
                sprites.back_bmp = Some(Path::new(args).into());
            }
            "STAGEFILE" => {
                if args.is_empty() {
                    return Err(ParseWarning::SyntaxError(
                        "expected splashscreen image filename".into(),
                    ));
                }
                sprites.stage_file = Some(Path::new(args).into());
            }

            "EXTCHR" => {
                // Allow multiple spaces between parameters
                let params: Vec<_> = args.split_whitespace().collect();
                if !(6..=10).contains(&params.len()) {
                    return Err(ParseWarning::SyntaxError(
                        "params length must be between 6 and 10".into(),
                    ));
                }
                let sprite_num = params[0]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected sprite_num i32".into()))?;
                let bmp_num = params[1];
                // BMPNum supports hexadecimal (e.g. 09/FF), also supports -1/-257, etc.
                let bmp_num = if let Some(stripped) = bmp_num.strip_prefix("-") {
                    -stripped
                        .parse::<i32>()
                        .map_err(|_| ParseWarning::SyntaxError("expected bmp_num is i32".into()))?
                } else if bmp_num.starts_with("0x")
                    || bmp_num.chars().all(|c| c.is_ascii_hexdigit())
                {
                    i32::from_str_radix(bmp_num, 16)
                        .unwrap_or_else(|_| bmp_num.parse().unwrap_or(0))
                } else {
                    bmp_num.parse().map_err(|_| {
                        ParseWarning::SyntaxError("expected bmp_num is i32 in hexadecimal".into())
                    })?
                };
                let start_x = params[2]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected start_x is i32".into()))?;
                let start_y = params[3]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected start_y is i32".into()))?;
                let end_x = params[4]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected end_x is i32".into()))?;
                let end_y = params[5]
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected end_y is i32".into()))?;
                // offsetX/offsetY are optional
                let offset_x = params.get(6).and_then(|v| v.parse().ok());
                let offset_y = params.get(7).and_then(|v| v.parse().ok());
                // x/y are optional, only present if offset exists
                let abs_x = params.get(8).and_then(|v| v.parse().ok());
                let abs_y = params.get(9).and_then(|v| v.parse().ok());
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

            charfile if charfile.starts_with("CHARFILE") => {
                if args.is_empty() {
                    return Err(ParseWarning::SyntaxError(
                        "expected character filename".into(),
                    ));
                }
                sprites.char_file = Some(Path::new(args).into());
            }
            _ => {}
        }
        Ok(())
    }
}
