//! The parser module of BMS(.bms/.bme/.bml/.pms) file.
//!
//! This module consists of two phases: lexical analyzing and token parsing.
//!
//! `lex` module provides definitions of BMS tokens and a translator from string into them. It supports major commands as possible, because the BMS specification is not standardized yet. If you found a lack of definition,  please tell me by opening an issue (only if not open yet).
//!
//! `parse` module provides definitions of BMS semantic objects and managers of BMS score data. The notes are serializable, but parsed result can't bring back into the BMS format text because of there are randomized syntax in BMS.
//!
//! `time` module provides definition of timing for notes as [`time::Track`] and [`time::ObjTime`].
//!
//! In detail, our policies are:
//!
//! - Support only UTF-8 (as required `String` to input).
//! - Do not support editing BMS source text.
//! - Do not support commands having ambiguous semantics.
//! - Do not support syntax came from typo (such as `#RONDOM` or `#END IF`).

pub mod lex;
pub mod parse;
pub mod time;

use thiserror::Error;

use self::{lex::LexWarning, parse::ParseWarning};

/// An error occurred when parsing the BMS format file.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum BmsWarning {
    /// An error comes from lexical analyzer.
    #[error("Warn: lex: {0}")]
    LexWarning(LexWarning),
    /// An error comes from syntax parser.
    #[error("Warn: parse: {0}")]
    ParseWarning(ParseWarning),
}

impl From<LexWarning> for BmsWarning {
    fn from(e: LexWarning) -> Self {
        Self::LexWarning(e)
    }
}
impl From<ParseWarning> for BmsWarning {
    fn from(e: ParseWarning) -> Self {
        Self::ParseWarning(e)
    }
}

/// A custom result type for bms-rs.
pub type Result<T> = std::result::Result<T, BmsWarning>;
