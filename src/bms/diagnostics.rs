//! Fancy diagnostics support using `ariadne`.
//!
//! This module provides convenient methods to convert errors carrying `SourcePosMixin`
//! (such as `LexWarningWithRange`, `ParseWarningWithRange`, `AstBuildWarningWithRange`,
//! `AstParseWarningWithRange`, and the aggregated `BmsWarning`) to `ariadne::Report`
//! without modifying existing error type definitions.
//!
//! Since `SourcePosMixin` contains index span information (start/end byte offsets), this module
//! provides tools to convert byte offsets to row/column positions for display purposes,
//! with line-based whole-line positioning.
//!
//! # Usage Example
//!
//! ```rust
//! use bms_rs::bms::{parse_bms, diagnostics::emit_bms_warnings, command::channel::mapper::KeyLayoutBeat};
//!
//! // Parse BMS file
//! let bms_source = "#TITLE Test\n#ARTIST Composer\n#INVALID command\n";
//! let output = parse_bms::<KeyLayoutBeat>(bms_source);
//!
//! // Output all warnings
//! emit_bms_warnings("test.bms", bms_source, &output.warnings);
//! ```

use crate::bms::{
    BmsWarning,
    ast::{AstBuildWarningWithRange, AstParseWarning, AstParseWarningWithRange},
    lex::LexWarningWithRange,
    parse::ParseWarningWithRange,
};

use ariadne::{Color, Label, Report, ReportKind, Source};

/// Simple source mapping that supports converting `(row, column)` to byte offsets by line.
/// Implemented independently to avoid conflicts with existing structures.
///
/// # Usage Example
///
/// ```rust
/// use bms_rs::bms::diagnostics::SimpleSource;
///
/// // Create source mapping
/// let source_text = "#TITLE test\n#ARTIST composer\n";
/// let source = SimpleSource::new("test.bms", source_text);
///
/// // Get source text
/// assert_eq!(source.text(), source_text);
///
/// // Get line span
/// let line_span = source.line_span(1); // Get first line
/// ```
pub struct SimpleSource<'a> {
    name: &'a str,
    /// Source text content.
    text: &'a str,
    /// Starting byte offset of each line (including virtual line 0 at offset 0),
    /// length is `lines + 1`, with the last element being `text.len()`.
    line_starts: Vec<usize>,
}

/// Implementation of SimpleSource.
impl<'a> SimpleSource<'a> {
    /// Create a new source mapping instance.
    ///
    /// # Parameters
    /// * `name` - Name of the source file
    /// * `text` - Complete text content of the source file
    pub fn new(name: &'a str, text: &'a str) -> Self {
        let mut line_starts = Vec::with_capacity(text.lines().count() + 2);
        line_starts.push(0);
        let mut acc = 0usize;
        for line in text.split_inclusive(['\n']) {
            acc += line.len();
            line_starts.push(acc);
        }
        if *line_starts.last().unwrap_or(&0) != text.len() {
            line_starts.push(text.len());
        }
        Self {
            name,
            text,
            line_starts,
        }
    }

    fn line_start(&self, row1: usize) -> usize {
        // row1 starts from 1; clamp when out of bounds
        let idx = row1
            .saturating_sub(1)
            .min(self.line_starts.len().saturating_sub(2));
        self.line_starts[idx]
    }

    fn line_end(&self, row1: usize) -> usize {
        let idx = row1.min(self.line_starts.len().saturating_sub(1));
        self.line_starts[idx]
    }

    #[allow(dead_code)]
    /// Convert 1-based (row, col) to byte offset, count columns by characters and clamp within line range.
    /// Internal use method.
    fn offset_of(&self, row1: usize, col1: usize) -> usize {
        let start = self.line_start(row1);
        let end = self.line_end(row1);
        let line = &self.text[start..end];
        let mut char_count = 0usize;
        let mut byte_off = 0usize;
        for (i, ch) in line.char_indices() {
            char_count += 1;
            if char_count >= col1 {
                byte_off = i;
                break;
            }
            byte_off = i + ch.len_utf8();
        }
        start + if col1 <= 1 { 0 } else { byte_off }
    }

    /// Byte range of a line.
    pub fn line_span(&self, row1: usize) -> std::ops::Range<usize> {
        self.line_start(row1)..self.line_end(row1)
    }

    /// Get source text content.
    ///
    /// # Returns
    /// Returns the complete text content of the source file
    pub fn text(&self) -> &'a str {
        self.text
    }

    /// Convert byte offset to 1-based (row, col) position.
    /// Returns (row, col) where both are 1-based.
    ///
    /// # Parameters
    /// * `offset` - Byte offset in the source text (0-based)
    ///
    /// # Returns
    /// Returns (row, col) as 1-based values
    pub fn offset_to_position(&self, offset: usize) -> (usize, usize) {
        // Clamp offset to valid range
        let offset = offset.min(self.text.len());

        // Find the line containing this offset
        let mut row = 1;
        let mut line_start = 0;

        for (i, &start) in self.line_starts.iter().enumerate() {
            if start > offset {
                // Found the line before this offset
                row = i;
                line_start = self.line_starts[i - 1];
                break;
            }
        }

        // If we didn't find it, it's on the last line
        if row == 1 && self.line_starts.len() > 1 {
            for (i, &start) in self.line_starts.iter().enumerate() {
                if start <= offset {
                    row = i + 1;
                    line_start = start;
                } else {
                    break;
                }
            }
        }

        // Calculate column within the line
        let line_text = &self.text[line_start
            ..self
                .line_starts
                .get(row)
                .copied()
                .unwrap_or(self.text.len())];
        let col_offset = offset - line_start;

        // Count characters up to the offset
        let mut col = 1;
        let mut byte_count = 0;

        for ch in line_text.chars() {
            if byte_count >= col_offset {
                break;
            }
            col += 1;
            byte_count += ch.len_utf8();
        }

        (row, col)
    }
}

/// Trait for converting positioned errors to `ariadne::Report`.
///
/// # Usage Example
///
/// ```rust
/// use bms_rs::bms::{diagnostics::{SimpleSource, ToAriadne, emit_bms_warnings}, BmsWarning};
/// use ariadne::Source;
///
/// // Assume there are warnings generated during BMS parsing
/// let warnings: Vec<BmsWarning> = vec![/* warnings obtained from parsing */];
/// let source_text = "#TITLE test\n#ARTIST composer\n";
///
/// // Simpler way: use convenience function
/// emit_bms_warnings("test.bms", source_text, &warnings);
///
/// // Or handle each warning manually:
/// let source = SimpleSource::new("test.bms", source_text);
/// let ariadne_source = Source::from(source_text);
///
/// for warning in &warnings {
///     let report = warning.to_report(&source);
///     // Use ariadne to render the report
///     let _ = report.print(("test.bms".to_string(), ariadne_source.clone()));
/// }
/// ```
pub trait ToAriadne {
    /// Convert error to ariadne Report.
    ///
    /// # Parameters
    /// * `src` - Source file mapping
    ///
    /// # Returns
    /// Returns the constructed ariadne Report
    fn to_report<'a>(&self, src: &SimpleSource<'a>)
    -> Report<'a, (String, std::ops::Range<usize>)>;
}

impl ToAriadne for LexWarningWithRange {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> Report<'a, (String, std::ops::Range<usize>)> {
        let (start, _end) = self.as_span();
        let (row, col) = src.offset_to_position(start);
        let span = src.line_span(row);
        Report::build(ReportKind::Warning, src.name.to_string(), span.start)
            .with_message("lex: ".to_string() + &self.content().to_string())
            .with_label(
                Label::new((src.name.to_string(), span.clone()))
                    .with_message(format!("position {}:{}", row, col))
                    .with_color(Color::Yellow),
            )
            .finish()
    }
}

impl ToAriadne for ParseWarningWithRange {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> Report<'a, (String, std::ops::Range<usize>)> {
        let (start, _end) = self.as_span();
        let (row, col) = src.offset_to_position(start);
        let span = src.line_span(row);
        Report::build(ReportKind::Warning, src.name.to_string(), span.start)
            .with_message("parse: ".to_string() + &self.content().to_string())
            .with_label(
                Label::new((src.name.to_string(), span.clone()))
                    .with_message(format!("position {}:{}", row, col))
                    .with_color(Color::Blue),
            )
            .finish()
    }
}

impl ToAriadne for AstBuildWarningWithRange {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> Report<'a, (String, std::ops::Range<usize>)> {
        let (start, _end) = self.as_span();
        let (row, col) = src.offset_to_position(start);
        let span = src.line_span(row);
        Report::build(ReportKind::Warning, src.name.to_string(), span.start)
            .with_message("ast_build: ".to_string() + &self.content().to_string())
            .with_label(
                Label::new((src.name.to_string(), span.clone()))
                    .with_message(format!("position {}:{}", row, col))
                    .with_color(Color::Cyan),
            )
            .finish()
    }
}

impl ToAriadne for AstParseWarningWithRange {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> Report<'a, (String, std::ops::Range<usize>)> {
        let (start, _end) = self.as_span();
        let (row, col) = src.offset_to_position(start);
        let span = src.line_span(row);

        // AstParseWarning internally has nested SourcePosMixin<RangeInclusive<BigUint>>, but it also has a top-level position.
        // We use the top-level position to annotate the entire line and append expected/actual information to the message.
        let details = match self.content() {
            AstParseWarning::RandomGeneratedValueOutOfRange { expected, actual }
            | AstParseWarning::SwitchGeneratedValueOutOfRange { expected, actual } => {
                format!("expected {:?}, got {}", expected.content(), actual)
            }
        };

        Report::build(ReportKind::Warning, src.name.to_string(), span.start)
            .with_message(format!("ast_parse: {} ({})", self.content(), details))
            .with_label(
                Label::new((src.name.to_string(), span.clone()))
                    .with_message(format!("position {}:{}", row, col))
                    .with_color(Color::Magenta),
            )
            .finish()
    }
}

impl ToAriadne for BmsWarning {
    fn to_report<'a>(
        &self,
        src: &SimpleSource<'a>,
    ) -> Report<'a, (String, std::ops::Range<usize>)> {
        use BmsWarning::*;
        match self {
            Lex(e) => e.to_report(src),
            AstBuild(e) => e.to_report(src),
            AstParse(e) => e.to_report(src),
            Parse(e) => e.to_report(src),
            // PlayingWarning / PlayingError have no position, locate to file start 0..0
            PlayingWarning(w) => {
                let span = 0..0;
                Report::build(ReportKind::Warning, src.name.to_string(), 0)
                    .with_message(format!("playing warning: {}", w))
                    .with_label(Label::new((src.name.to_string(), span)))
                    .finish()
            }
            PlayingError(e) => {
                let span = 0..0;
                Report::build(ReportKind::Error, src.name.to_string(), 0)
                    .with_message(format!("playing error: {}", e))
                    .with_label(Label::new((src.name.to_string(), span)))
                    .finish()
            }
        }
    }
}

/// Convenience method: batch render `BmsWarning` list.
///
/// This function automatically creates `SimpleSource` and generates beautiful diagnostic output for each warning.
///
/// # Usage Example
///
/// ```rust
/// use bms_rs::bms::{diagnostics::emit_bms_warnings, BmsWarning};
///
/// // BMS source text
/// let bms_source = "#TITLE My Song\n#ARTIST Composer\n#BPM 120\n";
///
/// // Assume warning list obtained from parsing
/// let warnings: Vec<BmsWarning> = vec![/* parsing warnings */];
///
/// // Batch output all warnings
/// emit_bms_warnings("my_song.bms", bms_source, &warnings);
/// ```
///
/// # Parameters
/// * `name` - Name of the source file, used for display in diagnostic information
/// * `source` - Complete BMS source text
/// * `warnings` - List of warnings to display
pub fn emit_bms_warnings<'a>(
    name: &'a str,
    source: &'a str,
    warnings: impl IntoIterator<Item = &'a BmsWarning>,
) {
    let simple = SimpleSource::new(name, source);
    let ariadne_source = Source::from(source);
    for w in warnings {
        let report = w.to_report(&simple);
        let _ = report.print((name.to_string(), ariadne_source.clone()));
    }
}
