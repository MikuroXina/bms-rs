//! Mixin types for structures.
//!
//! - `SourcePosMixin` is a generic wrapper that attaches position information (index span) to a value.
//! - `SourcePosMixinExt` is a trait that provides extension methods for `SourcePosMixin`, providing more convenient methods to create `SourcePosMixin` instances.

/// A generic wrapper that attaches position information (index span) to a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SourceRangeMixin<T> {
    /// Wrapped content value
    content: T,
    /// Start index in the source string (0-based, inclusive)
    start: usize,
    /// End index in the source string (0-based, exclusive)
    end: usize,
}

impl<T> SourceRangeMixin<T> {
    /// Instances a new `SourcePosMixin`
    pub const fn new(content: T, start: usize, end: usize) -> Self {
        Self {
            content,
            start,
            end,
        }
    }

    /// Returns the wrapped content.
    pub fn content(&self) -> &T {
        &self.content
    }

    /// Returns the wrapped content as a mutable reference.
    pub fn content_mut(&mut self) -> &mut T {
        &mut self.content
    }

    /// Leans the content out of the wrapper.
    pub fn into_content(self) -> T {
        self.content
    }

    /// Returns the start index of the source span.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the end index of the source span.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns the source span as a tuple of (start, end).
    pub fn as_span(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    /// Returns the length of the source span.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns true if the source span's length is 0.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a, T> SourceRangeMixin<T> {
    /// Returns the inner reference version of the wrapper.
    pub fn inner_ref(&'a self) -> SourceRangeMixin<&'a T> {
        let content = &self.content;
        SourceRangeMixin::new(content, self.start, self.end)
    }
}

impl<T> SourceRangeMixin<T> {
    /// Maps the content of the wrapper.
    pub fn map<U, F>(self, f: F) -> SourceRangeMixin<U>
    where
        F: FnOnce(T) -> U,
    {
        SourceRangeMixin::new(f(self.content), self.start, self.end)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for SourceRangeMixin<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at indices [{}, {})",
            self.content, self.start, self.end
        )
    }
}

impl<T> From<(T, usize, usize)> for SourceRangeMixin<T> {
    fn from(value: (T, usize, usize)) -> Self {
        Self::new(value.0, value.1, value.2)
    }
}

impl<T> From<SourceRangeMixin<T>> for (T, usize, usize) {
    fn from(value: SourceRangeMixin<T>) -> Self {
        (value.content, value.start, value.end)
    }
}

// Convenience implementation for creating empty SourcePosMixin with just a span
impl From<(usize, usize)> for SourceRangeMixin<()> {
    fn from(value: (usize, usize)) -> Self {
        Self::new((), value.0, value.1)
    }
}

impl<T: std::error::Error + 'static> std::error::Error for SourceRangeMixin<T> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.content)
    }
}

/// Extension methods for `SourcePosMixin`.
pub trait SourcePosMixinExt {
    /// Instances a new `SourcePosMixin` with the same span as a wrapper.
    fn into_wrapper<W>(self, wrapper: &SourceRangeMixin<W>) -> SourceRangeMixin<Self>
    where
        Self: Sized,
    {
        SourceRangeMixin::new(self, wrapper.start, wrapper.end)
    }

    /// Instances a new `SourcePosMixin` with a given start and end indices.
    fn into_wrapper_manual(self, start: usize, end: usize) -> SourceRangeMixin<Self>
    where
        Self: Sized,
    {
        SourceRangeMixin::new(self, start, end)
    }

    /// Instances a new `SourcePosMixin` with a given (start, end) span.
    fn into_wrapper_span(self, span: (usize, usize)) -> SourceRangeMixin<Self>
    where
        Self: Sized,
    {
        SourceRangeMixin::new(self, span.0, span.1)
    }
}

impl<T> SourcePosMixinExt for T {}
