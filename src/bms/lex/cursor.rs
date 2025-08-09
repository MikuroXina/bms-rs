use crate::bms::command::PositionWrapper;

use super::LexWarning;

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
    pub(crate) fn new(source: &'a str) -> Self {
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
        fn is_separator(c: char) -> bool {
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

    /// Move cursor, through and return the next token.
    pub(crate) fn next_token(&mut self) -> Option<&'a str> {
        let ret = self.peek_next_token_range();
        if ret.is_empty() {
            return None;
        }
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
        Some(&self.source[ret])
    }

    /// Move cursor, through and return the remaining part of this line.
    pub(crate) fn next_line_remaining(&mut self) -> &'a str {
        // Get remaining
        let remaining_end = self.source[self.index..]
            .find('\n')
            .unwrap_or(self.source[self.index..].len());
        let ret_line_end_index = if self
            .source
            .get(self.index + remaining_end - 1..=self.index + remaining_end)
            == Some("\r\n")
        {
            self.index + remaining_end - 1
        } else {
            self.index + remaining_end
        };
        let ret_remaining = &self.source[self.index..ret_line_end_index];
        // Record from remaining
        self.col += ret_remaining.chars().count();
        self.index += remaining_end;
        // Return remaining
        ret_remaining.trim()
    }

    /// Move cursor, through and return the entire line.
    pub(crate) fn next_line_entire(&mut self) -> &'a str {
        // Get remaining
        let remaining_end = self.source[self.index..]
            .find('\n')
            .unwrap_or(self.source[self.index..].len());
        let ret_line_end_index = if self
            .source
            .get(self.index + remaining_end - 1..=self.index + remaining_end)
            == Some("\r\n")
        {
            self.index + remaining_end - 1
        } else {
            self.index + remaining_end
        };
        let ret_remaining = &self.source[self.index..ret_line_end_index];
        // Get line start index
        let line_start_index = self.source[..self.index].rfind('\n').unwrap_or(0);
        // Record from remaining
        self.col += ret_remaining.chars().count();
        self.index += remaining_end;
        // Return entire line
        self.source[line_start_index..ret_line_end_index].trim()
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn col(&self) -> usize {
        self.col
    }

    pub(crate) fn make_err_expected_token(&self, message: impl Into<String>) -> LexWarning {
        LexWarning::ExpectedToken(PositionWrapper::new(
            message.into(),
            self.line(),
            self.col(),
        ))
    }

    pub(crate) fn make_err_object_id(&self, object: impl Into<String>) -> LexWarning {
        LexWarning::UnknownObject(PositionWrapper::new(object.into(), self.line(), self.col()))
    }

    pub(crate) fn make_err_unknown_channel(&self, channel: impl Into<String>) -> LexWarning {
        LexWarning::UnknownChannel(PositionWrapper::new(
            channel.into(),
            self.line(),
            self.col(),
        ))
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

    assert_eq!(cursor.line(), 1);
    assert_eq!(cursor.col(), 1);
    assert_eq!(cursor.next_token(), Some("hoge"));
    assert_eq!(cursor.line(), 2);
    assert_eq!(cursor.col(), 17);
    assert_eq!(cursor.next_token(), Some("foo"));
    assert_eq!(cursor.line(), 3);
    assert_eq!(cursor.col(), 16);
    assert_eq!(cursor.next_token(), Some("bar"));
    assert_eq!(cursor.line(), 4);
    assert_eq!(cursor.col(), 16);
    assert_eq!(cursor.next_token(), Some("bar"));
    assert_eq!(cursor.line(), 4);
    assert_eq!(cursor.col(), 20);
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
    assert_eq!(cursor.next_line_remaining(), "花たちに希望を [SP ANOTHER]");
    assert_eq!(cursor.next_token(), Some("#ARTIST"));
    assert_eq!(cursor.next_line_remaining(), "Sound piercer feat.DAZBEE");
    assert_eq!(cursor.next_token(), Some("#BPM"));
    assert_eq!(cursor.next_line_remaining(), "187");
}
