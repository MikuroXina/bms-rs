use super::{LexWarning, LexWarningWithRange};
use crate::bms::command::mixin::SourceRangeMixinExt;

pub(crate) struct Cursor<'a> {
    /// The line position, starts with 1.
    line: usize,
    /// The column position of char count, starts with 1. It is NOT byte count.
    col: usize,
    /// The index position.
    index: usize,
    /// The source str.
    source: &'a str,
}

impl<'a> Cursor<'a> {
    pub(crate) const fn new(source: &'a str) -> Self {
        Self {
            line: 1,
            col: 1,
            index: 0,
            source,
        }
    }

    pub(crate) fn is_end(&self) -> bool {
        self.peek_next_token().is_none()
    }

    fn peek_next_token_range(&self) -> std::ops::Range<usize> {
        const fn is_separator(c: char) -> bool {
            c.is_whitespace() || c == '\n'
        }
        let next_token_start = self.source[self.index..]
            .find(|c: char| !is_separator(c))
            .map_or(self.source.len(), |i| i + self.index);
        let next_token_end = self.source[next_token_start..]
            .trim_start()
            .find(is_separator)
            .map_or(self.source.len(), |i| i + next_token_start);
        next_token_start..next_token_end
    }

    pub(crate) fn peek_next_token(&self) -> Option<&'a str> {
        let ret = self.peek_next_token_range();
        if ret.is_empty() {
            return None;
        }
        Some(&self.source[ret])
    }

    /// Move cursor, through and return the next token with range.
    pub(crate) fn next_token_with_range(&mut self) -> Option<(std::ops::Range<usize>, &'a str)> {
        let ret = self.peek_next_token_range();
        if ret.is_empty() {
            return None;
        }
        let token_str = &self.source[ret.clone()];
        let advanced_lines = self.source[self.index..ret.end]
            .chars()
            .filter(|&c| c == '\n')
            .count();
        self.line += advanced_lines;
        if advanced_lines != 0 {
            self.col = 1;
        }
        self.col += self.source[self.index..ret.end]
            .lines()
            .last()
            .unwrap_or("")
            .chars()
            .count();
        self.index = ret.end;
        Some((ret, token_str))
    }

    /// Move cursor, through and return the next token.
    pub(crate) fn next_token(&mut self) -> Option<&'a str> {
        self.next_token_with_range().map(|(_, token)| token)
    }

    /// Determine the end of the current line and handle CRLF (\r\n) correctly.
    ///
    /// Returns a tuple `(remaining_end, line_end_index)` where:
    /// - `remaining_end` is the byte offset from current `index` to the first `\n` if any,
    ///   otherwise the remaining source length from `index` to the end.
    /// - `line_end_index` is the absolute byte index where the line content ends (exclusive).
    ///   If CRLF is detected right before the `\n`, the `\r` will be excluded from the line
    ///   content so that callers get a clean line without the trailing `\r`.
    fn current_line_bounds(&self) -> (usize, usize) {
        // Find the end of the remaining part until the first line feed (\n),
        // or the end of the source if no line feed exists.
        let remaining_end = self.source[self.index..]
            .find('\n')
            .unwrap_or_else(|| self.source[self.index..].len());

        // If the slice right before the line feed is a CRLF sequence ("\r\n"),
        // exclude the carriage return (\r) from the returned line content.
        let line_end_index = if self
            .source
            .get(self.index + remaining_end - 1..=self.index + remaining_end)
            == Some("\r\n")
        {
            self.index + remaining_end - 1
        } else {
            self.index + remaining_end
        };

        (remaining_end, line_end_index)
    }

    /// Move cursor, through and return the entire line.
    pub(crate) fn next_line_entire(&mut self) -> &'a str {
        // Compute the current line bounds without consuming the trailing newline.
        let (remaining_end, ret_line_end_index) = self.current_line_bounds();
        let ret_remaining = &self.source[self.index..ret_line_end_index];
        // Find the start of the line to return the full line content.
        let line_start_index = self.source[..self.index].rfind('\n').unwrap_or(0);
        // Update cursor column and index based on the consumed content.
        self.col += ret_remaining.chars().count();
        self.index += remaining_end;
        // Return the entire line content (from line start to line end), trimmed.
        self.source[line_start_index..ret_line_end_index].trim()
    }

    /// Returns the current byte index in the source string.
    pub(crate) const fn index(&self) -> usize {
        self.index
    }

    pub(crate) fn make_err_expected_token(
        &self,
        message: impl Into<String>,
    ) -> LexWarningWithRange {
        LexWarning::ExpectedToken {
            message: message.into(),
        }
        .into_wrapper_range(self.index()..self.index())
    }

    pub(crate) fn make_err_unknown_channel(
        &self,
        channel: impl Into<String>,
    ) -> LexWarningWithRange {
        LexWarning::UnknownChannel {
            channel: channel.into(),
        }
        .into_wrapper_range(self.index()..self.index())
    }
}

#[test]
fn test1() {
    let mut cursor = Cursor::new(
        r"
            hoge
            foo
            bar bar
        ",
    );

    // Test basic cursor functionality with index tracking
    assert_eq!(cursor.index(), 0);

    // Test token parsing
    assert_eq!(cursor.next_token(), Some("hoge"));
    assert!(cursor.index() > 0); // Index should advance

    assert_eq!(cursor.next_token(), Some("foo"));
    assert!(cursor.index() > 0); // Index should advance further

    assert_eq!(cursor.next_token(), Some("bar"));
    assert!(cursor.index() > 0); // Index should advance further

    assert_eq!(cursor.next_token(), Some("bar"));
    assert!(cursor.index() > 0); // Index should advance further

    // Test end of input
    assert_eq!(cursor.next_token(), None);
}

#[test]
fn test2() {
    const SOURCE: &str = r"
        #TITLE 花たちに希望を [SP ANOTHER]
        #ARTIST Sound piercer feat.DAZBEE
        #BPM 187
    ";

    let mut cursor = Cursor::new(SOURCE);

    assert_eq!(cursor.next_token(), Some("#TITLE"));
    assert_eq!(cursor.next_line_entire(), "花たちに希望を [SP ANOTHER]");
    assert_eq!(cursor.next_token(), Some("#ARTIST"));
    assert_eq!(cursor.next_line_entire(), "Sound piercer feat.DAZBEE");
    assert_eq!(cursor.next_token(), Some("#BPM"));
    assert_eq!(cursor.next_line_entire(), "187");
}

#[test]
fn test_next_line_crlf() {
    const SOURCE: &str = "#TITLE Hello\r\n#ARTIST Foo\r\nLAST\r\n";

    let mut cursor = Cursor::new(SOURCE);

    // remaining variant
    assert_eq!(cursor.next_token(), Some("#TITLE"));
    assert_eq!(cursor.next_line_entire(), "Hello");

    assert_eq!(cursor.next_token(), Some("#ARTIST"));
    assert_eq!(cursor.next_line_entire(), "Foo");

    assert_eq!(cursor.next_token(), Some("LAST"));
    assert_eq!(cursor.next_line_entire(), "");

    // reset for entire variant
    let mut cursor = Cursor::new(SOURCE);
    assert_eq!(cursor.next_token(), Some("#TITLE"));
    assert_eq!(cursor.next_line_entire(), "#TITLE Hello");

    assert_eq!(cursor.next_token(), Some("#ARTIST"));
    assert_eq!(cursor.next_line_entire(), "#ARTIST Foo");

    assert_eq!(cursor.next_token(), Some("LAST"));
    assert_eq!(cursor.next_line_entire(), "LAST");
}

#[test]
fn test_next_line_no_trailing_newline() {
    const SOURCE: &str = "#A Alpha\n#B Beta\nEND";

    let mut cursor = Cursor::new(SOURCE);

    // remaining variant
    assert_eq!(cursor.next_token(), Some("#A"));
    assert_eq!(cursor.next_line_entire(), "Alpha");

    assert_eq!(cursor.next_token(), Some("#B"));
    assert_eq!(cursor.next_line_entire(), "Beta");

    assert_eq!(cursor.next_token(), Some("END"));
    assert_eq!(cursor.next_line_entire(), "");

    // reset for entire variant
    let mut cursor = Cursor::new(SOURCE);
    assert_eq!(cursor.next_token(), Some("#A"));
    assert_eq!(cursor.next_line_entire(), "#A Alpha");

    assert_eq!(cursor.next_token(), Some("#B"));
    assert_eq!(cursor.next_line_entire(), "#B Beta");

    assert_eq!(cursor.next_token(), Some("END"));
    assert_eq!(cursor.next_line_entire(), "END");
}
