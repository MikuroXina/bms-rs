//! This module handles the tokens:
//!
//! - `#MIDIFILE path` - MIDI file path for the BGM. Deprecated.
//! - `#CDDA track_no` - Track number of a CD-DA for the BGM, for Delight Delight Republication.
//! - `#MATERIALSWAV path` - Specifies the shared audio path. Obsolete.
//! - `#MATERIALSBMP path` - Specifies the shared image path. Obsolete.
//! - `#MATERIALS path` - Unknown. Obsolete.

use std::{path::Path, str::FromStr};

use num::BigUint;

use super::{ProcessContext, TokenProcessor, all_tokens};
use crate::bms::ParseErrorWithRange;
use crate::bms::{model::resources::Resources, prelude::*};

/// It processes external resources such as `#MIDIFILE`, `#CDDA` and so on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourcesProcessor;

impl TokenProcessor for ResourcesProcessor {
    type Output = Resources;

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> Result<Self::Output, ParseErrorWithRange> {
        let mut resources = Resources::default();
        all_tokens(ctx, |token| {
            Ok(match token {
                Token::Header { name, args } => self
                    .on_header(name.as_ref(), args.as_ref(), &mut resources)
                    .err(),
                Token::Message { .. } | Token::NotACommand(_) => None,
            })
        })?;
        Ok(resources)
    }
}

impl ResourcesProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        resources: &mut Resources,
    ) -> core::result::Result<(), ParseWarning> {
        if name.eq_ignore_ascii_case("MIDIFILE") {
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError("expected midi filename".into()));
            }
            resources.midi_file = Some(Path::new(args).into());
        }
        if name.eq_ignore_ascii_case("CDDA") {
            let big_uint = BigUint::from_str(args)
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            resources.cdda.push(big_uint)
        }
        if name.eq_ignore_ascii_case("MATERIALSWAV") {
            resources.materials_wav.push(Path::new(args).into());
        }
        if name.eq_ignore_ascii_case("MATERIALSBMP") {
            resources.materials_bmp.push(Path::new(args).into());
        }
        if name.eq_ignore_ascii_case("MATERIALS") {
            resources.materials_path = Some(Path::new(args).into());
        }
        Ok(())
    }
}
