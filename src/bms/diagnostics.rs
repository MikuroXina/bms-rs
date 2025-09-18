//! Fancy diagnostics support using `ariadne`.
//!
//! This module provides convenient methods to convert errors carrying `SourcePosMixin`
//! (such as `LexWarningWithRange`, `ParseWarningWithRange`, `AstBuildWarningWithRange`,
//! `AstParseWarningWithRange`, and the aggregated `BmsWarning`) to `ariadne::Report`
//! without modifying existing error type definitions.
//!
//! Since `SourcePosMixin` contains index span information (start/end byte offsets), this module
//! lets ariadne automatically handle row/column calculations for display purposes.
//!
//! # Usage Example
//!
//! ```rust
//! use bms_rs::bms::{diagnostics::emit_bms_warnings, BmsWarning, parse_bms, command::channel::mapper::KeyLayoutBeat};
//!
//! // Parse BMS file
//! let bms_source = "#TITLE Test\n#ARTIST Composer\n#INVALID command\n";
//! let output = parse_bms::<KeyLayoutBeat>(bms_source);
//!
//! // Output all warnings
//! emit_bms_warnings("test.bms", bms_source, &output.warnings);
//! ```

use crate::bms::BmsWarning;

use ariadne::{Report, Source};

/// Simple source container that holds the filename and source text.
/// Ariadne will automatically handle row/column calculations from byte offsets.
///
/// # Usage Example
///
/// ```rust
/// use bms_rs::bms::diagnostics::SimpleSource;
///
/// // Create source container
/// let source_text = "#TITLE test\n#ARTIST composer\n";
/// let source = SimpleSource::new("test.bms", source_text);
///
/// // Get source text
/// assert_eq!(source.text(), source_text);
/// ```
pub struct SimpleSource<'a> {
    /// Name of the source file.
    name: &'a str,
    /// Source text content.
    text: &'a str,
}

impl<'a> SimpleSource<'a> {
    /// Create a new source container instance.
    ///
    /// # Parameters
    /// * `name` - Name of the source file
    /// * `text` - Complete text content of the source file
    #[must_use]
    pub const fn new(name: &'a str, text: &'a str) -> Self {
        Self { name, text }
    }

    /// Get source text content.
    ///
    /// # Returns
    /// Returns the complete text content of the source file
    #[must_use]
    pub const fn text(&self) -> &'a str {
        self.text
    }

    /// Get source file name.
    ///
    /// # Returns
    /// Returns the name of the source file
    #[must_use]
    pub const fn name(&self) -> &'a str {
        self.name
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
///     // Use ariadne to render the report - ariadne will automatically handle row/column calculation
///     let _ = report.print(("test.bms".to_string(), ariadne_source.clone()));
/// }
/// ```
pub trait ToAriadne {
    /// Convert error to ariadne Report.
    ///
    /// # Parameters
    /// * `src` - Source file container (used for filename, ariadne handles row/column calculation)
    ///
    /// # Returns
    /// Returns the constructed ariadne Report
    fn to_report<'a>(&self, src: &SimpleSource<'a>)
    -> Report<'a, (String, std::ops::Range<usize>)>;
}

/// Convenience method: batch render `BmsWarning` list.
///
/// This function automatically creates `SimpleSource` and generates beautiful diagnostic output for each warning.
/// Ariadne will automatically handle row/column calculations from the provided byte ranges.
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
/// // Batch output all warnings - ariadne will automatically calculate row/column positions
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
