use super::LexError;

pub(crate) struct Cursor<'a> {
    line: usize,
    col: usize,
    index: usize,
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
        self.peek_token().is_none()
    }

    fn get_token(&self) -> std::ops::Range<usize> {
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

    pub(crate) fn peek_token(&self) -> Option<&'a str> {
        let ret = self.get_token();
        if ret.is_empty() {
            return None;
        }
        Some(&self.source[ret])
    }

    pub(crate) fn next_token(&mut self) -> Option<&'a str> {
        let ret = self.get_token();
        if ret.is_empty() {
            return None;
        }
        self.line += self.source[self.index..ret.end]
            .chars()
            .filter(|&c| c == '\n')
            .count();
        self.col = ret.start - self.source[..ret.end].rfind('\n').unwrap_or(0);
        self.index = ret.end;
        Some(&self.source[ret])
    }

    pub(crate) fn next_line_remaining(&mut self) -> &'a str {
        let remaining_end = self.source[self.index..]
            .find('\n')
            .unwrap_or(self.source.len());
        let ret = if self
            .source
            .get(self.index + remaining_end - 1..=self.index + remaining_end)
            == Some("\r\n")
        {
            &self.source[self.index..self.index + remaining_end - 1]
        } else {
            &self.source[self.index..self.index + remaining_end]
        };
        self.col += remaining_end;
        self.index += remaining_end;
        ret
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn col(&self) -> usize {
        self.col
    }

    pub(crate) fn err_expected_token(&self, message: &'static str) -> LexError {
        LexError::ExpectedToken {
            line: self.line(),
            col: self.col(),
            message,
        }
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

    assert_eq!(cursor.line(), 0);
    assert_eq!(cursor.col(), 0);
    assert_eq!(cursor.next_token(), Some("hoge"));
    assert_eq!(cursor.line(), 1);
    assert_eq!(cursor.col(), 13);
    assert_eq!(cursor.next_token(), Some("foo"));
    assert_eq!(cursor.line(), 2);
    assert_eq!(cursor.col(), 13);
    assert_eq!(cursor.next_token(), Some("bar"));
    assert_eq!(cursor.line(), 3);
    assert_eq!(cursor.col(), 13);
    assert_eq!(cursor.next_token(), Some("bar"));
    assert_eq!(cursor.line(), 3);
    assert_eq!(cursor.col(), 17);
}
