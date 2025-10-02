//! Type definition module

use crate::bms::Decimal;
use crate::chart_process::ChartEvent;
use fraction::{BigUint, GenericDecimal};
use std::str::FromStr;

/// Y coordinate wrapper type, using arbitrary precision decimal numbers.
///
/// Unified y unit description: In default 4/4 time, one measure equals 1; BMS uses `#SECLEN` for linear conversion, BMSON normalizes via `pulses / (4*resolution)` to measure units.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct YCoordinate(pub Decimal);

impl YCoordinate {
    /// Create a new YCoordinate
    #[must_use]
    pub const fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Get the internal Decimal value
    #[must_use]
    pub const fn value(&self) -> &Decimal {
        &self.0
    }

    /// Convert to f64 (for compatibility)
    #[must_use]
    pub fn as_f64(&self) -> f64 {
        self.0.to_string().parse::<f64>().unwrap_or(0.0)
    }
}

impl From<Decimal> for YCoordinate {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<f64> for YCoordinate {
    fn from(value: f64) -> Self {
        // Convert f64 to string then parse as Decimal
        let decimal_str = value.to_string();
        let decimal = GenericDecimal::from_str(&decimal_str).unwrap_or_else(|_| {
            // If parsing fails, use 0
            GenericDecimal::from(BigUint::from(0u32))
        });
        Self(decimal)
    }
}

impl std::ops::Add for YCoordinate {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for YCoordinate {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Mul for YCoordinate {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl std::ops::Div for YCoordinate {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}

/// Display ratio wrapper type, representing the actual position of a note in the display area.
///
/// 0 is the judgment line, 1 is the position where the note generally starts to appear.
/// The value of this type is only affected by: current Y, Y visible range, and current Speed, Scroll values.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct DisplayRatio(pub Decimal);

impl DisplayRatio {
    /// Create a new DisplayRatio
    #[must_use]
    pub const fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Get the internal Decimal value
    #[must_use]
    pub const fn value(&self) -> &Decimal {
        &self.0
    }

    /// Convert to f64 (for compatibility)
    #[must_use]
    pub fn as_f64(&self) -> f64 {
        self.0.to_string().parse::<f64>().unwrap_or(0.0)
    }

    /// Create a DisplayRatio representing the judgment line (value 0)
    #[must_use]
    pub fn at_judgment_line() -> Self {
        Self(Decimal::from(0))
    }

    /// Create a DisplayRatio representing the position where note starts to appear (value 1)
    #[must_use]
    pub fn at_appearance() -> Self {
        Self(Decimal::from(1))
    }
}

impl From<Decimal> for DisplayRatio {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<f64> for DisplayRatio {
    fn from(value: f64) -> Self {
        // Convert f64 to string then parse as Decimal
        let decimal_str = value.to_string();
        let decimal = GenericDecimal::from_str(&decimal_str).unwrap_or_else(|_| {
            // If parsing fails, use 0
            GenericDecimal::from(BigUint::from(0u32))
        });
        Self(decimal)
    }
}

/// WAV audio file ID wrapper type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WavId(pub usize);

impl WavId {
    /// Create a new WavId
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    /// Get the internal usize value
    #[must_use]
    pub const fn value(self) -> usize {
        self.0
    }
}

impl From<usize> for WavId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<WavId> for usize {
    fn from(id: WavId) -> Self {
        id.0
    }
}

/// BMP/BGA image file ID wrapper type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BmpId(pub usize);

impl BmpId {
    /// Create a new BmpId
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    /// Get the internal usize value
    #[must_use]
    pub const fn value(self) -> usize {
        self.0
    }
}

impl From<usize> for BmpId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<BmpId> for usize {
    fn from(id: BmpId) -> Self {
        id.0
    }
}

/// Timeline event and position wrapper type.
///
/// Represents an event in chart playback and its position on the timeline.
#[derive(Debug, Clone)]
pub struct ChartEventWithPosition {
    /// Event position on timeline (y coordinate)
    pub position: YCoordinate,
    /// Chart event
    pub event: ChartEvent,
}

impl ChartEventWithPosition {
    /// Create a new ChartEventWithPosition
    #[must_use]
    pub const fn new(position: YCoordinate, event: ChartEvent) -> Self {
        Self { position, event }
    }

    /// Get event position
    #[must_use]
    pub const fn position(&self) -> &YCoordinate {
        &self.position
    }

    /// Get chart event
    #[must_use]
    pub const fn event(&self) -> &ChartEvent {
        &self.event
    }

    /// Destructure into tuple
    #[must_use]
    pub fn into_tuple(self) -> (YCoordinate, ChartEvent) {
        (self.position, self.event)
    }
}

impl From<(YCoordinate, ChartEvent)> for ChartEventWithPosition {
    fn from((position, event): (YCoordinate, ChartEvent)) -> Self {
        Self::new(position, event)
    }
}

impl From<ChartEventWithPosition> for (YCoordinate, ChartEvent) {
    fn from(wrapper: ChartEventWithPosition) -> Self {
        wrapper.into_tuple()
    }
}

/// Visible area event and position and display ratio wrapper type.
///
/// Represents an event in the visible area, including its position, event content, and display ratio.
#[derive(Debug, Clone)]
pub struct VisibleEvent {
    /// Event position on timeline (y coordinate)
    pub position: YCoordinate,
    /// Chart event
    pub event: ChartEvent,
    /// Display ratio
    pub display_ratio: DisplayRatio,
}

impl VisibleEvent {
    /// Create a new VisibleEvent
    #[must_use]
    pub const fn new(
        position: YCoordinate,
        event: ChartEvent,
        display_ratio: DisplayRatio,
    ) -> Self {
        Self {
            position,
            event,
            display_ratio,
        }
    }

    /// Get event position
    #[must_use]
    pub const fn position(&self) -> &YCoordinate {
        &self.position
    }

    /// Get chart event
    #[must_use]
    pub const fn event(&self) -> &ChartEvent {
        &self.event
    }

    /// Get display ratio
    #[must_use]
    pub const fn display_ratio(&self) -> &DisplayRatio {
        &self.display_ratio
    }

    /// Destructure into tuple
    #[must_use]
    pub fn into_tuple(self) -> (YCoordinate, ChartEvent, DisplayRatio) {
        (self.position, self.event, self.display_ratio)
    }
}

impl From<(YCoordinate, ChartEvent, DisplayRatio)> for VisibleEvent {
    fn from((position, event, display_ratio): (YCoordinate, ChartEvent, DisplayRatio)) -> Self {
        Self::new(position, event, display_ratio)
    }
}

impl From<VisibleEvent> for (YCoordinate, ChartEvent, DisplayRatio) {
    fn from(wrapper: VisibleEvent) -> Self {
        wrapper.into_tuple()
    }
}
