//! This module handles the tokens:
//!
//! - `#MIDIFILE path` - MIDI file path for the BGM. Deprecated.
//! - `#CDDA track_no` - Track number of a CD-DA for the BGM, for Delight Delight Republication.
//! - `#MATERIALSWAV path` - Specifies the shared audio path. Obsolete.
//! - `#MATERIALSBMP path` - Specifies the shared image path. Obsolete.
//! - `#MATERIALS path` - Unknown. Obsolete.

use std::{path::Path, str::FromStr};

use num::BigUint;

use super::{TokenProcessor, TokenProcessorOutput, all_tokens};
use crate::bms::{error::Result, model::resources::Resources, prelude::*};

/// It processes external resources such as `#MIDIFILE`, `#CDDA` and so on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourcesProcessor;

impl TokenProcessor for ResourcesProcessor {
    type Output = Resources;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        _prompter: &P,
    ) -> TokenProcessorOutput<Self::Output> {
        let mut resources = Resources::default();
        let TokenProcessorOutput {
            output: res,
            warnings,
        } = all_tokens(input, |token| {
            Ok(match token {
                Token::Header { name, args } => self
                    .on_header(name.as_ref(), args.as_ref(), &mut resources)
                    .err(),
                Token::Message { .. } | Token::NotACommand(_) => None,
            })
        });
        match res {
            Ok(()) => TokenProcessorOutput {
                output: Ok(resources),
                warnings,
            },
            Err(e) => TokenProcessorOutput {
                output: Err(e),
                warnings,
            },
        }
    }
}

impl ResourcesProcessor {
    fn on_header(&self, name: &str, args: &str, resources: &mut Resources) -> Result<()> {
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
