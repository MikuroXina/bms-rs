//! Some impl with [Cursor] usage for Structures in [command] module, for lex part.

use crate::{
    bms::command::{JudgeLevel, ObjId, PlayerMode, PoorMode},
    parse::ParseWarning,
};

use super::{Result, cursor::Cursor};

impl PlayerMode {
    pub(crate) fn from(c: &mut Cursor) -> Result<Self> {
        Ok(match c.next_token() {
            Some("1") => Self::Single,
            Some("2") => Self::Two,
            Some("3") => Self::Double,
            _ => return Err(c.make_err_expected_token("one of 1, 2 or 3")),
        })
    }
}

impl std::fmt::Display for PlayerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerMode::Single => write!(f, "1"),
            PlayerMode::Two => write!(f, "2"),
            PlayerMode::Double => write!(f, "3"),
        }
    }
}

impl std::str::FromStr for PlayerMode {
    type Err = ParseWarning;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "1" => Self::Single,
            "2" => Self::Two,
            "3" => Self::Double,
            _ => {
                return Err(ParseWarning::SyntaxError(
                    "expected one of 0, 1 or 2".into(),
                ));
            }
        })
    }
}

impl JudgeLevel {
    pub(crate) fn try_read(c: &mut Cursor) -> Result<Self> {
        c.next_token()
            .ok_or_else(|| c.make_err_expected_token("one of [0,4]"))?
            .try_into()
            .map_err(|_| c.make_err_expected_token("one of [0,4]"))
    }
}

impl ObjId {
    pub(crate) fn try_load(value: &str, c: &mut Cursor) -> Result<Self> {
        Self::try_from(value).map_err(|_| c.make_err_object_id(value.to_string()))
    }
}

impl PoorMode {
    pub(crate) fn from(c: &mut Cursor) -> Result<Self> {
        Ok(match c.next_token() {
            Some("0") => Self::Interrupt,
            Some("1") => Self::Overlay,
            Some("2") => Self::Hidden,
            _ => return Err(c.make_err_expected_token("one of 0, 1 or 2")),
        })
    }
}

impl std::str::FromStr for PoorMode {
    type Err = ParseWarning;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "0" => Self::Interrupt,
            "1" => Self::Overlay,
            "2" => Self::Hidden,
            _ => {
                return Err(ParseWarning::SyntaxError(
                    "expected one of 0, 1 or 2".into(),
                ));
            }
        })
    }
}
