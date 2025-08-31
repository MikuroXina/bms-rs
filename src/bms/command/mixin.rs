//! Mixin types for structures.
//!
//! - `SourcePosMixin` is a generic wrapper that attaches position information (index span) to a value.
//! - `SourcePosMixinExt` is a trait that provides extension methods for `SourcePosMixin`, providing more convenient methods to create `SourcePosMixin` instances.

use std::ops::Range;

/// A generic wrapper that attaches position information (index span) to a value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SourceRangeMixin<T> {
    /// Wrapped content value
    content: T,
    /// Range of indices in the source string (0-based, inclusive start, exclusive end)
    range: Range<usize>,
}

impl<T> SourceRangeMixin<T> {
    /// Creates a new `SourceRangeMixin` with a range
    pub const fn new(content: T, range: Range<usize>) -> Self {
        Self { content, range }
    }

    /// Creates a new `SourceRangeMixin` with start and end indices
    pub const fn new_with_start_end(content: T, start: usize, end: usize) -> Self {
        Self::new(content, start..end)
    }

    /// Returns the wrapped content.
    pub const fn content(&self) -> &T {
        &self.content
    }

    /// Returns the wrapped content as a mutable reference.
    pub const fn content_mut(&mut self) -> &mut T {
        &mut self.content
    }

    /// Leans the content out of the wrapper.
    pub fn into_content(self) -> T {
        self.content
    }

    /// Returns the start index of the source span.
    pub const fn start(&self) -> usize {
        self.range.start
    }

    /// Returns the end index of the source span.
    pub const fn end(&self) -> usize {
        self.range.end
    }

    /// Returns the source range.
    pub const fn range(&self) -> &Range<usize> {
        &self.range
    }

    /// Returns the source span as a tuple of (start, end).
    pub const fn as_span(&self) -> (usize, usize) {
        (self.range.start, self.range.end)
    }

    /// Returns the length of the source span.
    pub const fn len(&self) -> usize {
        self.range.end.saturating_sub(self.range.start)
    }

    /// Returns true if the source span's length is 0.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a, T> SourceRangeMixin<T> {
    /// Returns the inner reference version of the wrapper.
    pub fn inner_ref(&'a self) -> SourceRangeMixin<&'a T> {
        let content = &self.content;
        SourceRangeMixin::new(content, self.range.clone())
    }
}

impl<T> SourceRangeMixin<T> {
    /// Maps the content of the wrapper.
    pub fn map<U, F>(self, f: F) -> SourceRangeMixin<U>
    where
        F: FnOnce(T) -> U,
    {
        SourceRangeMixin::new(f(self.content), self.range)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for SourceRangeMixin<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at indices [{}, {})",
            self.content, self.range.start, self.range.end
        )
    }
}

impl<T> From<(T, Range<usize>)> for SourceRangeMixin<T> {
    fn from(value: (T, Range<usize>)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl<T> From<(T, usize, usize)> for SourceRangeMixin<T> {
    fn from(value: (T, usize, usize)) -> Self {
        Self::new_with_start_end(value.0, value.1, value.2)
    }
}

impl<T> From<SourceRangeMixin<T>> for (T, Range<usize>) {
    fn from(value: SourceRangeMixin<T>) -> Self {
        (value.content, value.range)
    }
}

impl<T> From<SourceRangeMixin<T>> for (T, usize, usize) {
    fn from(value: SourceRangeMixin<T>) -> Self {
        (value.content, value.range.start, value.range.end)
    }
}

// Convenience implementation for creating empty SourcePosMixin with just a span
impl From<Range<usize>> for SourceRangeMixin<()> {
    fn from(value: Range<usize>) -> Self {
        Self::new((), value)
    }
}

impl From<(usize, usize)> for SourceRangeMixin<()> {
    fn from(value: (usize, usize)) -> Self {
        Self::new_with_start_end((), value.0, value.1)
    }
}

impl<T: std::error::Error + 'static> std::error::Error for SourceRangeMixin<T> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.content)
    }
}

/// Extension methods for [`SourceRangeMixin`].
pub trait SourceRangeMixinExt {
    /// Creates a new `SourceRangeMixin` with the same span as a wrapper.
    fn into_wrapper<W>(self, wrapper: &SourceRangeMixin<W>) -> SourceRangeMixin<Self>
    where
        Self: Sized,
    {
        SourceRangeMixin::new(self, wrapper.range.clone())
    }

    /// Creates a new `SourceRangeMixin` with a given range.
    fn into_wrapper_range(self, range: Range<usize>) -> SourceRangeMixin<Self>
    where
        Self: Sized,
    {
        SourceRangeMixin::new(self, range)
    }

    /// Creates a new `SourceRangeMixin` with a given (start, end) span.
    fn into_wrapper_span(self, span: (usize, usize)) -> SourceRangeMixin<Self>
    where
        Self: Sized,
    {
        SourceRangeMixin::new_with_start_end(self, span.0, span.1)
    }
}

impl<T> SourceRangeMixinExt for T {}
