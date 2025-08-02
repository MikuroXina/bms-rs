//! Defines for graphics.

/// A 2D point in pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PixelPoint {
    /// X coordinate in pixels.
    pub x: i16,
    /// Y coordinate in pixels.
    pub y: i16,
}

impl PixelPoint {
    /// Creates a new pixel point.
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

impl From<(i16, i16)> for PixelPoint {
    fn from((x, y): (i16, i16)) -> Self {
        Self { x, y }
    }
}

impl From<PixelPoint> for (i16, i16) {
    fn from(point: PixelPoint) -> Self {
        (point.x, point.y)
    }
}

/// A 2D size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PixelSize {
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
}

impl PixelSize {
    /// Creates a new pixel size.
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

impl From<(u16, u16)> for PixelSize {
    fn from((width, height): (u16, u16)) -> Self {
        Self { width, height }
    }
}

impl From<PixelSize> for (u16, u16) {
    fn from(size: PixelSize) -> Self {
        (size.width, size.height)
    }
}
