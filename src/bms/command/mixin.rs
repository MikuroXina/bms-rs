//! Mixin types for structures.
//!
//! - `SourcePosMixin` is a generic wrapper that attaches position information (row/column) to a value.
//! - `SourcePosMixinExt` is a trait that provides extension methods for `SourcePosMixin`, providing more convenient methods to create `SourcePosMixin` instances.

use num::BigUint;

/// A generic wrapper that attaches position information (row/column) to a value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SourcePosMixin<T> {
    /// Wrapped content value
    pub content: T,
    /// Source line number (1-based)
    row: usize,
    /// Source column number (1-based)
    column: usize,
}

impl<T> SourcePosMixin<T> {
    /// Instances a new `SourcePosMixin`
    pub const fn new(content: T, row: usize, column: usize) -> Self {
        Self {
            content,
            row,
            column,
        }
    }

    /// Returns the row number of the source position.
    pub fn row(&self) -> usize {
        self.row
    }

    /// Returns the column number of the source position.
    pub fn column(&self) -> usize {
        self.column
    }

    /// Returns the source position as a tuple of (row, column).
    pub fn as_pos(&self) -> (usize, usize) {
        (self.row, self.column)
    }
}

impl<T> std::ops::Deref for SourcePosMixin<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl<T> std::ops::DerefMut for SourcePosMixin<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}

impl<T: std::fmt::Display> std::fmt::Display for SourcePosMixin<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at line {}, column {}",
            self.content, self.row, self.column
        )
    }
}

impl<T> From<(T, usize, usize)> for SourcePosMixin<T> {
    fn from(value: (T, usize, usize)) -> Self {
        Self::new(value.0, value.1, value.2)
    }
}

impl<T> From<SourcePosMixin<T>> for (T, usize, usize) {
    fn from(value: SourcePosMixin<T>) -> Self {
        (value.content, value.row, value.column)
    }
}

impl<T: std::error::Error + 'static> std::error::Error for SourcePosMixin<T> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.content)
    }
}

/// Extension methods for `SourcePosMixin`.
pub trait SourcePosMixinExt {
    /// Instances a new `SourcePosMixin` with the same row and column as a wrapper.
    fn into_wrapper<W>(self, wrapper: &SourcePosMixin<W>) -> SourcePosMixin<Self>
    where
        Self: Sized,
    {
        SourcePosMixin::new(self, wrapper.row, wrapper.column)
    }

    /// Instances a new `SourcePosMixin` with a given row and column.
    fn into_wrapper_manual(self, row: usize, column: usize) -> SourcePosMixin<Self>
    where
        Self: Sized,
    {
        SourcePosMixin::new(self, row, column)
    }

    /// Instances a new `SourcePosMixin` with a given (row, column).
    fn into_wrapper_tuple(self, pos: (usize, usize)) -> SourcePosMixin<Self>
    where
        Self: Sized,
    {
        SourcePosMixin::new(self, pos.0, pos.1)
    }
}

impl<T> SourcePosMixinExt for SourcePosMixin<T> {}

// Provide extension methods for commonly wrapped inner types.
// Note: We intentionally implement per-type instead of a blanket impl to avoid coherence
// issues with other specific impls in modules like `ast::structure`.
impl SourcePosMixinExt for String {}
impl SourcePosMixinExt for BigUint {}
impl SourcePosMixinExt for u8 {}
impl SourcePosMixinExt for u16 {}
impl SourcePosMixinExt for u32 {}
impl SourcePosMixinExt for u64 {}
impl SourcePosMixinExt for u128 {}
impl SourcePosMixinExt for usize {}
impl SourcePosMixinExt for i8 {}
impl SourcePosMixinExt for i16 {}
impl SourcePosMixinExt for i32 {}
impl SourcePosMixinExt for i64 {}
impl SourcePosMixinExt for i128 {}
impl SourcePosMixinExt for isize {}
impl SourcePosMixinExt for f32 {}
impl SourcePosMixinExt for f64 {}
impl SourcePosMixinExt for bool {}
impl SourcePosMixinExt for char {}
impl SourcePosMixinExt for &str {}
impl SourcePosMixinExt for &[u8] {}