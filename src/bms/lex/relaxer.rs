use crate::bms::command::mixin::SourceRangeMixin;
use crate::bms::lex::{
    cursor::Cursor,
    token::{Token, TokenWithRange},
};
use num::BigUint;

/// Trait providing extensible relaxed lexing (Relaxer) rules.
pub trait Relaxer {
    /// Normalize raw command token (case, fullwidth `#`, common typos).
    fn normalize_command(&self, command: &str) -> String;

    /// Handle special patterns that require reading extra tokens or constructing a specific Token;
    /// return `Some(TokenWithRange)` if handled.
    fn try_handle_special<'a>(
        &self,
        command_upper: &str,
        cursor: &mut Cursor<'a>,
        start_index: usize,
    ) -> Option<TokenWithRange<'a>>;
}

/// Convert input to uppercase for subsequent rule matching.
pub struct UppercaseRelaxer;

impl Relaxer for UppercaseRelaxer {
    fn normalize_command(&self, command: &str) -> String {
        command.to_uppercase()
    }
    fn try_handle_special<'a>(
        &self,
        _command_upper: &str,
        _cursor: &mut Cursor<'a>,
        _start_index: usize,
    ) -> Option<TokenWithRange<'a>> {
        None
    }
}

/// Replace a leading fullwidth `＃` with ASCII `#`.
pub struct FullwidthHashRelaxer;

impl Relaxer for FullwidthHashRelaxer {
    fn normalize_command(&self, command: &str) -> String {
        if command.starts_with('＃') {
            let mut s = String::from("#");
            s.push_str(&command.chars().skip(1).collect::<String>());
            s
        } else {
            command.to_string()
        }
    }
    fn try_handle_special<'a>(
        &self,
        _command_upper: &str,
        _cursor: &mut Cursor<'a>,
        _start_index: usize,
    ) -> Option<TokenWithRange<'a>> {
        None
    }
}

/// Handle common typos/aliases: `#RONDAM` → `#RANDOM`, `#IFEND` → `#ENDIF`.
pub struct TypoRelaxer;

impl Relaxer for TypoRelaxer {
    fn normalize_command(&self, command: &str) -> String {
        match command {
            "#RONDAM" => "#RANDOM".into(),
            "#IFEND" => "#ENDIF".into(),
            _ => command.to_string(),
        }
    }
    fn try_handle_special<'a>(
        &self,
        _command_upper: &str,
        _cursor: &mut Cursor<'a>,
        _start_index: usize,
    ) -> Option<TokenWithRange<'a>> {
        None
    }
}

/// Handle number suffix without space: `#RANDOMn`, `#IFn`.
pub struct NumberSuffixRelaxer;

impl Relaxer for NumberSuffixRelaxer {
    fn normalize_command(&self, command: &str) -> String {
        command.to_string()
    }
    fn try_handle_special<'a>(
        &self,
        command_upper: &str,
        cursor: &mut Cursor<'a>,
        start_index: usize,
    ) -> Option<TokenWithRange<'a>> {
        if let Some(suffix) = command_upper.strip_prefix("#RANDOM")
            && !suffix.is_empty()
            && suffix.chars().all(|ch| ch.is_ascii_digit())
            && let Ok(max) = suffix.parse::<BigUint>()
        {
            let range = start_index..cursor.index();
            return Some(SourceRangeMixin::new(Token::Random(max), range));
        }
        if let Some(suffix) = command_upper.strip_prefix("#IF")
            && !suffix.is_empty()
            && suffix.chars().all(|ch| ch.is_ascii_digit())
            && let Ok(target) = suffix.parse::<BigUint>()
        {
            let range = start_index..cursor.index();
            return Some(SourceRangeMixin::new(Token::If(target), range));
        }
        None
    }
}

/// Handle tokenized form `#END IF` (consumes the following `IF`).
pub struct SpacedEndIfRelaxer;

impl Relaxer for SpacedEndIfRelaxer {
    fn normalize_command(&self, command: &str) -> String {
        command.to_string()
    }
    fn try_handle_special<'a>(
        &self,
        command_upper: &str,
        cursor: &mut Cursor<'a>,
        start_index: usize,
    ) -> Option<TokenWithRange<'a>> {
        if command_upper == "#END"
            && let Some(next) = cursor.peek_next_token()
            && next.to_uppercase() == "IF"
        {
            let _ = cursor.next_token();
            let range = start_index..cursor.index();
            return Some(SourceRangeMixin::new(Token::EndIf, range));
        }
        None
    }
}

/// Return the default Relaxer chain: Uppercase → fullwidth `#` → typo → number suffix → `#END IF`.
#[must_use]
pub fn default_relaxers() -> Vec<Box<dyn Relaxer>> {
    vec![
        Box::new(UppercaseRelaxer),
        Box::new(FullwidthHashRelaxer),
        Box::new(TypoRelaxer),
        Box::new(NumberSuffixRelaxer),
        Box::new(SpacedEndIfRelaxer),
    ]
}
