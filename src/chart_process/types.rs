//! Type definition module

use crate::bms::prelude::Bms;
#[cfg(feature = "bmson")]
use crate::bmson::prelude::Bmson;
use crate::{bms::Decimal, chart_process::ChartEvent};
use num::{One, ToPrimitive, Zero};
use std::collections::BTreeMap;
use std::time::Duration;

/// Trait for generating the base BPM used to derive default visible window length.
pub trait BaseBpmGenerator<S> {
    /// Generate a `BaseBpm` from the given source.
    /// Returns `None` when the source lacks sufficient information to determine a base BPM.
    fn generate(&self, source: &S) -> Option<BaseBpm>;
}

/// Generator that uses the chart's start/initial BPM.
#[derive(Debug, Clone, Copy, Default)]
pub struct StartBpmGenerator;

/// Generator that uses the minimum BPM across initial BPM and all BPM change events.
#[derive(Debug, Clone, Copy, Default)]
pub struct MinBpmGenerator;

/// Generator that uses the maximum BPM across initial BPM and all BPM change events.
#[derive(Debug, Clone, Copy, Default)]
pub struct MaxBpmGenerator;

/// Generator that uses a manually specified BPM value.
#[derive(Debug, Clone)]
pub struct ManualBpmGenerator(pub Decimal);

/// Base BPM wrapper type, encapsulating a `Decimal` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseBpm(pub Decimal);

impl BaseBpm {
    /// Create a new BaseBpm
    #[must_use]
    pub const fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Get the internal Decimal value
    #[must_use]
    pub const fn value(&self) -> &Decimal {
        &self.0
    }
}

impl From<Decimal> for BaseBpm {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

/// Visible range per BPM, representing the relationship between BPM and visible Y range.
/// Formula: visible_y_range = current_bpm * visible_range_per_bpm
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleRangePerBpm(Decimal);

impl VisibleRangePerBpm {
    /// Create a new VisibleRangePerBpm from base BPM and reaction time
    /// Formula: visible_range_per_bpm = reaction_time_seconds / base_bpm
    #[must_use]
    pub fn new(base_bpm: &BaseBpm, reaction_time: Duration) -> Self {
        if base_bpm.value().is_zero() {
            Self(Decimal::zero())
        } else {
            let seconds = Decimal::from(reaction_time.as_secs_f64());
            Self(seconds / base_bpm.value().clone())
        }
    }

    /// Calculate visible window length in y units based on current BPM.
    /// Formula: `visible_window_y = current_bpm * visible_range_per_bpm`.
    #[must_use]
    pub fn window_y(&self, current_bpm: &Decimal) -> Decimal {
        current_bpm.clone() * self.value().clone()
    }

    /// Get the internal Decimal value
    #[must_use]
    pub const fn value(&self) -> &Decimal {
        &self.0
    }

    /// Calculate reaction time from visible range per BPM
    /// Formula: reaction_time = visible_range_per_bpm / playhead_speed
    /// where playhead_speed = 1/240 (Y/sec per BPM)
    #[must_use]
    pub fn to_reaction_time(&self) -> Duration {
        if self.0.is_zero() {
            Duration::from_secs(0)
        } else {
            let seconds = self.0.clone() * Decimal::from(240);
            Duration::from_secs_f64(seconds.to_f64().unwrap_or(0.0))
        }
    }

    /// Create from Decimal value (for internal use)
    #[must_use]
    pub(crate) const fn from_decimal(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<Decimal> for VisibleRangePerBpm {
    fn from(value: Decimal) -> Self {
        Self::from_decimal(value)
    }
}

// ---- Generators for BMS ----
impl BaseBpmGenerator<Bms> for StartBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm.bpm.as_ref().cloned().map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for MinBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm
            .bpm
            .iter()
            .cloned()
            .chain(
                bms.bpm
                    .bpm_changes
                    .values()
                    .map(|change| change.bpm.clone()),
            )
            .min()
            .map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for MaxBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm
            .bpm
            .iter()
            .cloned()
            .chain(
                bms.bpm
                    .bpm_changes
                    .values()
                    .map(|change| change.bpm.clone()),
            )
            .max()
            .map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for ManualBpmGenerator {
    fn generate(&self, _bms: &Bms) -> Option<BaseBpm> {
        Some(BaseBpm::new(self.0.clone()))
    }
}

// ---- Generators for BMSON ----
#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for StartBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        Some(BaseBpm::new(Decimal::from(bmson.info.init_bpm.as_f64())))
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for MinBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        std::iter::once(Decimal::from(bmson.info.init_bpm.as_f64()))
            .chain(
                bmson
                    .bpm_events
                    .iter()
                    .map(|ev| Decimal::from(ev.bpm.as_f64())),
            )
            .min()
            .map(BaseBpm::new)
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for MaxBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        std::iter::once(Decimal::from(bmson.info.init_bpm.as_f64()))
            .chain(
                bmson
                    .bpm_events
                    .iter()
                    .map(|ev| Decimal::from(ev.bpm.as_f64())),
            )
            .max()
            .map(BaseBpm::new)
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for ManualBpmGenerator {
    fn generate(&self, _bmson: &Bmson<'a>) -> Option<BaseBpm> {
        Some(BaseBpm::new(self.0.clone()))
    }
}

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
        Self(Decimal::from(value))
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
        Self(Decimal::zero())
    }

    /// Create a DisplayRatio representing the position where note starts to appear (value 1)
    #[must_use]
    pub fn at_appearance() -> Self {
        Self(Decimal::one())
    }
}

impl From<Decimal> for DisplayRatio {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<f64> for DisplayRatio {
    fn from(value: f64) -> Self {
        Self(Decimal::from(value))
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

/// Identifier type which is unique over all chart events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChartEventId(pub usize);

impl ChartEventId {
    /// Create a new ChartEventId
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

impl From<usize> for ChartEventId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<ChartEventId> for usize {
    fn from(id: ChartEventId) -> Self {
        id.0
    }
}

/// Generator for sequential `ChartEventId`s
#[derive(Debug, Clone, Default)]
pub struct ChartEventIdGenerator {
    next: usize,
}

impl ChartEventIdGenerator {
    /// Create a new generator starting from `start`
    #[must_use]
    pub const fn new(start: usize) -> Self {
        Self { next: start }
    }

    /// Allocate and return the next `ChartEventId`
    #[must_use]
    pub const fn next_id(&mut self) -> ChartEventId {
        let id = ChartEventId(self.next);
        self.next += 1;
        id
    }

    /// Return the next `ChartEventId` that will be used
    #[must_use]
    pub const fn peek_next(&self) -> ChartEventId {
        ChartEventId::new(self.next)
    }
}

/// Timeline event and position wrapper type.
///
/// Represents an event in chart playback and its position on the timeline.
#[derive(Debug, Clone)]
pub struct PlayheadEvent {
    /// Event identifier
    pub id: ChartEventId,
    /// Event position on timeline (y coordinate)
    pub position: YCoordinate,
    /// Chart event
    pub event: ChartEvent,
    /// Activate time since chart playback started
    pub activate_time: Duration,
}

impl PlayheadEvent {
    /// Create a new ChartEventWithPosition
    #[must_use]
    pub const fn new(
        id: ChartEventId,
        position: YCoordinate,
        event: ChartEvent,
        activate_time: Duration,
    ) -> Self {
        Self {
            position,
            event,
            id,
            activate_time,
        }
    }

    /// Get event identifier
    #[must_use]
    pub const fn id(&self) -> ChartEventId {
        self.id
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

    /// Get activate time
    #[must_use]
    pub const fn activate_time(&self) -> &Duration {
        &self.activate_time
    }
}

impl PartialEq for PlayheadEvent {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for PlayheadEvent {}

impl std::hash::Hash for PlayheadEvent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Visible area event and position and display ratio wrapper type.
///
/// Represents an event in the visible area, including its position, event content, and display ratio.
#[derive(Debug, Clone)]
pub struct VisibleChartEvent {
    /// Event identifier
    pub id: ChartEventId,
    /// Event position on timeline (y coordinate)
    pub position: YCoordinate,
    /// Chart event
    pub event: ChartEvent,
    /// Display ratio
    pub display_ratio: DisplayRatio,
    /// Activate time since chart playback started
    pub activate_time: Duration,
}

impl VisibleChartEvent {
    /// Create a new VisibleEvent
    #[must_use]
    pub const fn new(
        id: ChartEventId,
        position: YCoordinate,
        event: ChartEvent,
        display_ratio: DisplayRatio,
        activate_time: Duration,
    ) -> Self {
        Self {
            position,
            event,
            display_ratio,
            id,
            activate_time,
        }
    }

    /// Get event identifier
    #[must_use]
    pub const fn id(&self) -> ChartEventId {
        self.id
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

    /// Get activate time
    #[must_use]
    pub const fn activate_time(&self) -> &Duration {
        &self.activate_time
    }
}

impl PartialEq for VisibleChartEvent {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for VisibleChartEvent {}

impl std::hash::Hash for VisibleChartEvent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AllEventsIndex {
    events: Vec<PlayheadEvent>,
    by_y: BTreeMap<YCoordinate, Vec<usize>>,
    by_time: Vec<usize>,
}

impl AllEventsIndex {
    #[must_use]
    pub fn new(map: BTreeMap<YCoordinate, Vec<PlayheadEvent>>) -> Self {
        let mut events: Vec<PlayheadEvent> = Vec::new();
        let mut by_y: BTreeMap<YCoordinate, Vec<usize>> = BTreeMap::new();

        for (y_coord, y_events) in map {
            let mut indices: Vec<usize> = Vec::with_capacity(y_events.len());
            for ev in y_events {
                let idx = events.len();
                events.push(ev);
                indices.push(idx);
            }
            by_y.insert(y_coord, indices);
        }

        let mut by_time: Vec<usize> = (0..events.len()).collect();
        by_time.sort_by(|&a, &b| {
            events[a]
                .activate_time
                .cmp(&events[b].activate_time)
                .then_with(|| events[a].position.cmp(&events[b].position))
                .then_with(|| events[a].id.cmp(&events[b].id))
        });

        Self {
            events,
            by_y,
            by_time,
        }
    }

    #[must_use]
    pub const fn as_events(&self) -> &Vec<PlayheadEvent> {
        &self.events
    }

    #[must_use]
    pub const fn as_by_y(&self) -> &BTreeMap<YCoordinate, Vec<usize>> {
        &self.by_y
    }

    #[must_use]
    pub fn events_in_y_range(
        &self,
        start_exclusive: YCoordinate,
        end_inclusive: YCoordinate,
    ) -> Vec<PlayheadEvent> {
        use std::ops::Bound::{Excluded, Included};

        self.by_y
            .range((Excluded(start_exclusive), Included(end_inclusive)))
            .flat_map(|(_, indices)| indices.iter().copied())
            .map(|idx| self.events[idx].clone())
            .collect()
    }

    pub fn events_in_time_range(&self, start: Duration, end: Duration) -> Vec<PlayheadEvent> {
        let start_idx = self
            .by_time
            .partition_point(|&idx| self.events[idx].activate_time < start);
        let end_idx = self
            .by_time
            .partition_point(|&idx| self.events[idx].activate_time <= end);

        self.by_time[start_idx..end_idx]
            .iter()
            .copied()
            .map(|idx| self.events[idx].clone())
            .collect()
    }
}
