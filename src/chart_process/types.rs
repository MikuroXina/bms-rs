//! Type definition module

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::ops::{Bound, Range, RangeBounds};
use std::path::PathBuf;
use std::time::Duration;

use strict_num_extended::FinF64;

/// A trait for types that can represent zero.
pub trait Zero {
    /// Returns the zero value for this type.
    fn zero() -> Self;

    /// Checks if the value is zero.
    fn is_zero(&self) -> bool;
}

pub use super::TimeSpan;
use crate::bms::command::StringValue;
use crate::bms::prelude::Bms;
use crate::chart_process::ChartEvent;

/// Flow events that affect playback speed/scroll.
#[derive(Debug, Clone)]
pub enum FlowEvent {
    /// BPM change event.
    Bpm(f64),
    /// Speed factor change event (BMS only).
    Speed(f64),
    /// Scroll factor change event.
    Scroll(f64),
}
#[cfg(feature = "bmson")]
use crate::bmson::prelude::Bmson;

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
pub struct ManualBpmGenerator(pub FinF64);

impl AsRef<FinF64> for ManualBpmGenerator {
    fn as_ref(&self) -> &FinF64 {
        &self.0
    }
}

impl From<FinF64> for ManualBpmGenerator {
    fn from(value: FinF64) -> Self {
        Self(value)
    }
}

impl From<ManualBpmGenerator> for FinF64 {
    fn from(value: ManualBpmGenerator) -> Self {
        value.0
    }
}

impl ManualBpmGenerator {
    /// Returns a reference to the contained BPM value.
    #[must_use]
    pub const fn value(&self) -> &FinF64 {
        &self.0
    }

    /// Consumes self and returns the contained BPM value.
    #[must_use]
    pub const fn into_value(self) -> FinF64 {
        self.0
    }
}

/// Base BPM wrapper type, encapsulating a `FinF64` value.
#[derive(Debug, Clone, PartialEq)]
pub struct BaseBpm(pub FinF64);

impl AsRef<FinF64> for BaseBpm {
    fn as_ref(&self) -> &FinF64 {
        &self.0
    }
}

impl BaseBpm {
    /// Create a new `BaseBpm`
    #[must_use]
    pub const fn new(value: FinF64) -> Self {
        Self(value)
    }

    /// Returns a reference to the contained BPM value.
    #[must_use]
    pub const fn value(&self) -> &FinF64 {
        &self.0
    }

    /// Consumes self and returns the contained BPM value.
    #[must_use]
    pub const fn into_value(self) -> FinF64 {
        self.0
    }
}

impl From<FinF64> for BaseBpm {
    fn from(value: FinF64) -> Self {
        Self(value)
    }
}

impl From<BaseBpm> for FinF64 {
    fn from(value: BaseBpm) -> Self {
        value.0
    }
}

impl From<StringValue<FinF64>> for BaseBpm {
    fn from(value: StringValue<FinF64>) -> Self {
        Self::new(FinF64::new(value.as_f64().unwrap_or(120.0)).unwrap_or(FINF64_120))
    }
}

/// Visible range per BPM, representing the relationship between BPM and visible Y range.
/// Formula: `visible_y_range` = `current_bpm` * `visible_range_per_bpm`
#[derive(Debug, Clone, PartialEq)]
pub struct VisibleRangePerBpm(FinF64);

impl AsRef<FinF64> for VisibleRangePerBpm {
    fn as_ref(&self) -> &FinF64 {
        &self.0
    }
}

/// Zero constant for `FinF64`
const FINF64_ZERO: FinF64 = FinF64::new_const(0.0);

/// One constant for `FinF64`
const FINF64_ONE: FinF64 = FinF64::new_const(1.0);

/// 120 constant for `FinF64` (default BPM)
pub(crate) const FINF64_120: FinF64 = FinF64::new_const(120.0);

impl VisibleRangePerBpm {
    /// Create a new `VisibleRangePerBpm` from base BPM and reaction time
    /// Formula: `visible_range_per_bpm` = `reaction_time_seconds` * 240 / `base_bpm`
    #[must_use]
    pub fn new(base_bpm: &BaseBpm, reaction_time: TimeSpan) -> Self {
        let bpm_value = base_bpm.value().get();
        if bpm_value == 0.0 {
            Self(FINF64_ZERO)
        } else {
            let reaction_secs = reaction_time.as_secs_f64().max(0.0);
            let value = reaction_secs * 240.0 / bpm_value;
            Self(FinF64::new(value).unwrap_or(FINF64_ZERO))
        }
    }

    /// Returns a reference to the contained value.
    #[must_use]
    pub const fn value(&self) -> &FinF64 {
        &self.0
    }

    /// Consumes self and returns the contained value.
    #[must_use]
    pub const fn into_value(self) -> FinF64 {
        self.0
    }

    /// Calculate visible window length in y units based on current BPM, speed, and playback ratio.
    /// Formula: `visible_window_y = current_bpm * visible_range_per_bpm * current_speed * playback_ratio / 240`
    /// This ensures events stay in visible window for exactly `reaction_time` duration.
    #[must_use]
    pub fn window_y(
        &self,
        current_bpm: &FinF64,
        current_speed: &FinF64,
        playback_ratio: &FinF64,
    ) -> YCoordinate {
        let speed_factor = current_speed.get() * playback_ratio.get();
        let adjusted = current_bpm.get() * self.0.get() * speed_factor / 240.0;
        YCoordinate::new(FinF64::new(adjusted).unwrap_or(FINF64_ZERO))
    }

    /// Calculate reaction time from visible range per BPM
    /// Formula: `reaction_time` = `visible_range_per_bpm` / `playhead_speed`
    /// where `playhead_speed` = 1/240 (Y/sec per BPM)
    #[must_use]
    pub fn to_reaction_time(&self) -> TimeSpan {
        if self.0.get() == 0.0 {
            TimeSpan::ZERO
        } else {
            let seconds = self.0.get() * 240.0;
            TimeSpan::from_duration(Duration::from_secs_f64(seconds))
        }
    }

    /// Create from `FinF64` value (for internal use)
    #[must_use]
    pub(crate) const fn from_finf64(value: FinF64) -> Self {
        Self(value)
    }
}

impl From<FinF64> for VisibleRangePerBpm {
    fn from(value: FinF64) -> Self {
        Self::from_finf64(value)
    }
}

impl From<VisibleRangePerBpm> for FinF64 {
    fn from(value: VisibleRangePerBpm) -> Self {
        value.0
    }
}

// ---- Generators for BMS ----
impl BaseBpmGenerator<Bms> for StartBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm
            .bpm
            .as_ref()
            .and_then(StringValue::as_f64)
            .and_then(|v| FinF64::new(v).ok())
            .map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for MinBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm
            .bpm
            .as_ref()
            .and_then(StringValue::as_f64)
            .into_iter()
            .chain(
                bms.bpm
                    .bpm_changes
                    .values()
                    .filter_map(|change| change.bpm.as_f64()),
            )
            .filter_map(|v| FinF64::new(v).ok())
            .min()
            .map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for MaxBpmGenerator {
    fn generate(&self, bms: &Bms) -> Option<BaseBpm> {
        bms.bpm
            .bpm
            .as_ref()
            .and_then(StringValue::as_f64)
            .into_iter()
            .chain(
                bms.bpm
                    .bpm_changes
                    .values()
                    .filter_map(|change| change.bpm.as_f64()),
            )
            .filter_map(|v| FinF64::new(v).ok())
            .max()
            .map(BaseBpm::new)
    }
}

impl BaseBpmGenerator<Bms> for ManualBpmGenerator {
    fn generate(&self, _bms: &Bms) -> Option<BaseBpm> {
        Some(BaseBpm::new(self.0))
    }
}

// ---- Generators for BMSON ----
#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for StartBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        FinF64::new(bmson.info.init_bpm.as_f64())
            .ok()
            .map(BaseBpm::new)
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for MinBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        std::iter::once(bmson.info.init_bpm.as_f64())
            .chain(bmson.bpm_events.iter().map(|ev| ev.bpm.as_f64()))
            .filter_map(|v| FinF64::new(v).ok())
            .min()
            .map(BaseBpm::new)
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for MaxBpmGenerator {
    fn generate(&self, bmson: &Bmson<'a>) -> Option<BaseBpm> {
        std::iter::once(bmson.info.init_bpm.as_f64())
            .chain(bmson.bpm_events.iter().map(|ev| ev.bpm.as_f64()))
            .filter_map(|v| FinF64::new(v).ok())
            .max()
            .map(BaseBpm::new)
    }
}

#[cfg(feature = "bmson")]
impl<'a> BaseBpmGenerator<Bmson<'a>> for ManualBpmGenerator {
    fn generate(&self, _bmson: &Bmson<'a>) -> Option<BaseBpm> {
        Some(BaseBpm::new(self.0))
    }
}

/// Y coordinate wrapper type, using finite f64 numbers.
///
/// Unified y unit description: In default 4/4 time, one measure equals 1; BMS uses `#SECLEN` for linear conversion, BMSON normalizes via `pulses / (4*resolution)` to measure units.
#[derive(Debug, Clone, PartialEq)]
pub struct YCoordinate(pub FinF64);

impl AsRef<FinF64> for YCoordinate {
    fn as_ref(&self) -> &FinF64 {
        &self.0
    }
}

impl YCoordinate {
    /// Create a new `YCoordinate`
    #[must_use]
    pub const fn new(value: FinF64) -> Self {
        Self(value)
    }

    /// Returns a reference to the contained value.
    #[must_use]
    pub const fn value(&self) -> &FinF64 {
        &self.0
    }

    /// Consumes self and returns the contained value.
    #[must_use]
    pub const fn into_value(self) -> FinF64 {
        self.0
    }

    /// Returns the value as f64.
    #[must_use]
    pub const fn as_f64(&self) -> f64 {
        self.0.as_f64()
    }

    /// Creates a zero of Y coordinate.
    #[must_use]
    pub const fn zero() -> Self {
        Self(FINF64_ZERO)
    }
}

impl From<FinF64> for YCoordinate {
    fn from(value: FinF64) -> Self {
        Self(value)
    }
}

impl From<YCoordinate> for FinF64 {
    fn from(value: YCoordinate) -> Self {
        value.0
    }
}

impl From<f64> for YCoordinate {
    fn from(value: f64) -> Self {
        Self(FinF64::new(value).unwrap_or(FINF64_ZERO))
    }
}

impl std::ops::Add for YCoordinate {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self((self.0 + rhs.0).unwrap_or(FINF64_ZERO))
    }
}

impl std::ops::Add for &YCoordinate {
    type Output = YCoordinate;

    fn add(self, rhs: Self) -> Self::Output {
        YCoordinate((self.0 + rhs.0).unwrap_or(FINF64_ZERO))
    }
}

impl std::ops::Sub for YCoordinate {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self((self.0 - rhs.0).unwrap_or(FINF64_ZERO))
    }
}

impl std::ops::Sub for &YCoordinate {
    type Output = YCoordinate;

    fn sub(self, rhs: Self) -> Self::Output {
        YCoordinate((self.0 - rhs.0).unwrap_or(FINF64_ZERO))
    }
}

impl std::ops::Mul for YCoordinate {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self((self.0 * rhs.0).unwrap_or(FINF64_ZERO))
    }
}

impl std::ops::Div for YCoordinate {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self((self.0 / rhs.0).unwrap_or(FINF64_ZERO))
    }
}

impl std::ops::Div for &YCoordinate {
    type Output = YCoordinate;

    fn div(self, rhs: Self) -> Self::Output {
        YCoordinate((self.0 / rhs.0).unwrap_or(FINF64_ZERO))
    }
}

impl Zero for YCoordinate {
    fn zero() -> Self {
        Self(FINF64_ZERO)
    }

    fn is_zero(&self) -> bool {
        self.0.get() == 0.0
    }
}

impl Eq for YCoordinate {}

impl PartialOrd for YCoordinate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for YCoordinate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(Ordering::Equal)
    }
}

/// Display ratio wrapper type, representing the actual position of a note in the display area.
///
/// 0 is the judgment line, 1 is the position where the note generally starts to appear.
/// The value of this type is only affected by: current Y, Y visible range, and current Speed, Scroll values.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct DisplayRatio(pub FinF64);

impl AsRef<FinF64> for DisplayRatio {
    fn as_ref(&self) -> &FinF64 {
        &self.0
    }
}

impl DisplayRatio {
    /// Create a new `DisplayRatio`
    #[must_use]
    pub const fn new(value: FinF64) -> Self {
        Self(value)
    }

    /// Returns a reference to the contained value.
    #[must_use]
    pub const fn value(&self) -> &FinF64 {
        &self.0
    }

    /// Consumes self and returns the contained value.
    #[must_use]
    pub const fn into_value(self) -> FinF64 {
        self.0
    }

    /// Returns the value as f64.
    #[must_use]
    pub const fn as_f64(&self) -> f64 {
        self.0.as_f64()
    }

    /// Create a `DisplayRatio` representing the judgment line (value 0)
    #[must_use]
    pub const fn at_judgment_line() -> Self {
        Self(FINF64_ZERO)
    }

    /// Create a `DisplayRatio` representing the position where note starts to appear (value 1)
    #[must_use]
    pub const fn at_appearance() -> Self {
        Self(FINF64_ONE)
    }
}

impl From<FinF64> for DisplayRatio {
    fn from(value: FinF64) -> Self {
        Self(value)
    }
}

impl From<DisplayRatio> for FinF64 {
    fn from(value: DisplayRatio) -> Self {
        value.0
    }
}

impl From<f64> for DisplayRatio {
    fn from(value: f64) -> Self {
        Self(FinF64::new(value).unwrap_or(FINF64_ZERO))
    }
}

/// WAV audio file ID wrapper type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WavId(pub usize);

impl AsRef<usize> for WavId {
    fn as_ref(&self) -> &usize {
        &self.0
    }
}

impl WavId {
    /// Create a new `WavId`
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the contained id value.
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

impl AsRef<usize> for BmpId {
    fn as_ref(&self) -> &usize {
        &self.0
    }
}

impl BmpId {
    /// Create a new `BmpId`
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the contained id value.
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

impl AsRef<usize> for ChartEventId {
    fn as_ref(&self) -> &usize {
        &self.0
    }
}

impl ChartEventId {
    /// Create a new `ChartEventId`
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the contained id value.
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
    pub activate_time: TimeSpan,
}

impl PlayheadEvent {
    /// Create a new `ChartEventWithPosition`
    #[must_use]
    pub const fn new(
        id: ChartEventId,
        position: YCoordinate,
        event: ChartEvent,
        activate_time: TimeSpan,
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
    pub const fn activate_time(&self) -> &TimeSpan {
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

/// Index for all chart events, organized by Y coordinate and time.
#[derive(Debug, Clone)]
pub struct AllEventsIndex {
    events: Vec<PlayheadEvent>,
    by_y: BTreeMap<YCoordinate, Range<usize>>,
    by_time: BTreeMap<TimeSpan, Vec<usize>>,
}

impl AllEventsIndex {
    /// Create a new event index from a map of events grouped by Y coordinate.
    ///
    /// This constructor flattens the input map into a single vector of events
    /// while maintaining indices for efficient Y-coordinate-based lookups.
    ///
    /// # Parameters
    /// - `map`: Events organized by their Y coordinates
    ///
    /// # Returns
    /// A new `AllEventsIndex` with optimized lookup structures
    #[must_use]
    pub fn new(map: BTreeMap<YCoordinate, Vec<PlayheadEvent>>) -> Self {
        let mut events: Vec<PlayheadEvent> = Vec::new();
        let mut by_y: BTreeMap<YCoordinate, Range<usize>> = BTreeMap::new();

        for (y_coord, y_events) in map {
            let start = events.len();
            events.extend(y_events);
            let end = events.len();
            by_y.insert(y_coord, start..end);
        }

        let mut by_time: BTreeMap<TimeSpan, Vec<usize>> = BTreeMap::new();
        for (idx, ev) in events.iter().enumerate() {
            by_time.entry(ev.activate_time).or_default().push(idx);
        }
        for indices in by_time.values_mut() {
            indices.sort_by(|&a, &b| {
                let Some(a_ev) = events.get(a) else {
                    return Ordering::Equal;
                };
                let Some(b_ev) = events.get(b) else {
                    return Ordering::Equal;
                };
                a_ev.position
                    .cmp(&b_ev.position)
                    .then_with(|| a_ev.id.cmp(&b_ev.id))
            });
        }

        Self {
            events,
            by_y,
            by_time,
        }
    }

    /// Get a reference to all events in chronological order.
    ///
    /// # Returns
    /// A slice of all events stored in this index
    #[must_use]
    pub const fn as_events(&self) -> &Vec<PlayheadEvent> {
        &self.events
    }

    /// Get a reference to the Y-coordinate-based index.
    ///
    /// # Returns
    /// A map from Y coordinates to ranges in the events vector
    #[must_use]
    pub const fn as_by_y(&self) -> &BTreeMap<YCoordinate, Range<usize>> {
        &self.by_y
    }

    /// Retrieve all events within a specified Y coordinate range.
    ///
    /// This method efficiently collects events whose Y coordinates fall within
    /// the given range bounds.
    ///
    /// # Parameters
    /// - `range`: The Y coordinate range to query
    ///
    /// # Returns
    /// A vector of events within the specified range
    #[must_use]
    pub fn events_in_y_range<R>(&self, range: R) -> Vec<PlayheadEvent>
    where
        R: RangeBounds<YCoordinate>,
    {
        self.by_y
            .range(range)
            .flat_map(|(_, indices)| self.events.get(indices.clone()).into_iter().flatten())
            .cloned()
            .collect()
    }

    /// Retrieve all events within a specified time range.
    ///
    /// This method queries events by their activation time, collecting all
    /// events that fall within the given time bounds.
    ///
    /// # Parameters
    /// - `range`: The time range to query
    ///
    /// # Returns
    /// A vector of events within the specified time range
    pub fn events_in_time_range<R>(&self, range: R) -> Vec<PlayheadEvent>
    where
        R: RangeBounds<TimeSpan>,
    {
        // To avoid panic when `start > end` or the range is empty.
        let mut start_bound = range.start_bound().cloned();
        let mut end_bound = range.end_bound().cloned();

        let start_value = match &start_bound {
            Bound::Unbounded => None,
            Bound::Included(v) | Bound::Excluded(v) => Some(v),
        };
        let end_value = match &end_bound {
            Bound::Unbounded => None,
            Bound::Included(v) | Bound::Excluded(v) => Some(v),
        };
        if let (Some(start), Some(end)) = (start_value, end_value)
            && start > end
        {
            std::mem::swap(&mut start_bound, &mut end_bound);
        }

        self.by_time
            .range((start_bound, end_bound))
            .flat_map(|(_, indices)| indices.iter().copied())
            .filter_map(|idx| self.events.get(idx).cloned())
            .collect()
    }

    /// Retrieve events within a time range relative to a center point.
    ///
    /// This method allows querying events relative to a specific time point,
    /// useful for looking ahead or behind a current playback position.
    ///
    /// # Parameters
    /// - `center`: The center time point for the range
    /// - `range`: The offset range from the center point (e.g., `-1.0s..=1.0s`)
    ///
    /// # Returns
    /// A vector of events within the offset-adjusted time range
    ///
    /// # Example
    /// ```ignore
    /// // Get events from 1 second before to 1 second after time t
    /// let events = index.events_in_time_range_offset_from(t, -1.0s..=1.0s);
    /// ```
    pub fn events_in_time_range_offset_from<R>(
        &self,
        center: TimeSpan,
        range: R,
    ) -> Vec<PlayheadEvent>
    where
        R: RangeBounds<TimeSpan>,
    {
        let start_bound = match range.start_bound() {
            Bound::Included(offset) => Bound::Included(center + *offset),
            Bound::Excluded(offset) => Bound::Excluded(center + *offset),
            Bound::Unbounded => Bound::Unbounded,
        };
        let end_bound = match range.end_bound() {
            Bound::Included(offset) => Bound::Included(center + *offset),
            Bound::Excluded(offset) => Bound::Excluded(center + *offset),
            Bound::Unbounded => Bound::Unbounded,
        };
        self.events_in_time_range((start_bound, end_bound))
    }
}

/// Resource file mapping for parsed charts.
#[derive(Debug, Clone)]
pub struct ChartResources {
    /// WAV ID -> file path mapping.
    pub(crate) wav_files: HashMap<WavId, PathBuf>,
    /// BMP ID -> file path mapping.
    pub(crate) bmp_files: HashMap<BmpId, PathBuf>,
}

impl ChartResources {
    /// Get WAV file mapping.
    #[must_use]
    pub const fn wav_files(&self) -> &HashMap<WavId, PathBuf> {
        &self.wav_files
    }

    /// Get BMP file mapping.
    #[must_use]
    pub const fn bmp_files(&self) -> &HashMap<BmpId, PathBuf> {
        &self.bmp_files
    }

    /// Create a new `ChartResources` (internal API).
    #[must_use]
    pub(crate) const fn new(
        wav_files: HashMap<WavId, PathBuf>,
        bmp_files: HashMap<BmpId, PathBuf>,
    ) -> Self {
        Self {
            wav_files,
            bmp_files,
        }
    }
}

/// Parsed chart data containing all precomputed information.
///
/// This structure is immutable and can be used to create multiple player instances.
#[derive(Debug, Clone)]
pub struct ParsedChart {
    /// Resource file mapping.
    pub(crate) resources: ChartResources,
    /// Event index (by Y coordinate and time).
    pub(crate) events: AllEventsIndex,
    /// Flow event mapping (affects playback speed).
    pub(crate) flow_events: BTreeMap<YCoordinate, Vec<FlowEvent>>,
    /// Initial BPM.
    pub(crate) init_bpm: FinF64,
    /// Initial Speed (BMS-specific, BMSON defaults to 1.0).
    pub(crate) init_speed: FinF64,
}

impl ParsedChart {
    /// Get resource file mapping.
    #[must_use]
    pub const fn resources(&self) -> &ChartResources {
        &self.resources
    }

    /// Get event index.
    #[must_use]
    pub const fn events(&self) -> &AllEventsIndex {
        &self.events
    }

    /// Get flow event mapping.
    #[must_use]
    pub const fn flow_events(&self) -> &BTreeMap<YCoordinate, Vec<FlowEvent>> {
        &self.flow_events
    }

    /// Get initial BPM.
    #[must_use]
    pub const fn init_bpm(&self) -> &FinF64 {
        &self.init_bpm
    }

    /// Get initial Speed.
    #[must_use]
    pub const fn init_speed(&self) -> &FinF64 {
        &self.init_speed
    }

    /// Get audio file resources (WAV ID to path mapping).
    ///
    /// This is a convenience method that directly accesses the audio files.
    /// Equivalent to `self.resources().wav_files()`.
    #[must_use]
    pub const fn audio_files(&self) -> &HashMap<WavId, PathBuf> {
        self.resources.wav_files()
    }

    /// Get BGA/BMP image resources (BMP ID to path mapping).
    ///
    /// This is a convenience method that directly accesses the image files.
    /// Equivalent to `self.resources().bmp_files()`.
    #[must_use]
    pub const fn bmp_files(&self) -> &HashMap<BmpId, PathBuf> {
        self.resources.bmp_files()
    }

    /// Create a new `ParsedChart` (internal API).
    #[must_use]
    pub(crate) const fn new(
        resources: ChartResources,
        events: AllEventsIndex,
        flow_events: BTreeMap<YCoordinate, Vec<FlowEvent>>,
        init_bpm: FinF64,
        init_speed: FinF64,
    ) -> Self {
        Self {
            resources,
            events,
            flow_events,
            init_bpm,
            init_speed,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{AllEventsIndex, ChartEvent, ChartEventId, PlayheadEvent, TimeSpan, YCoordinate};

    fn mk_event(id: usize, y: f64, time_secs: u64) -> PlayheadEvent {
        let y_coord = YCoordinate::from(y);
        PlayheadEvent::new(
            ChartEventId::new(id),
            y_coord,
            ChartEvent::BarLine,
            TimeSpan::SECOND * time_secs as i64,
        )
    }

    #[test]
    fn events_in_y_range_uses_btreemap_order_and_preserves_group_order() {
        let y0 = YCoordinate::from(0.0);
        let y1 = YCoordinate::from(1.0);

        let mut map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        map.insert(
            y0.clone(),
            vec![
                mk_event(2, 0.0, 1),
                mk_event(1, 0.0, 1),
                mk_event(3, 0.0, 2),
            ],
        );
        map.insert(y1.clone(), vec![mk_event(4, 1.0, 1)]);

        let idx = AllEventsIndex::new(map);

        let got_ids: Vec<usize> = idx
            .events_in_y_range((std::ops::Bound::Included(y0), std::ops::Bound::Included(y1)))
            .into_iter()
            .map(|ev| ev.id.value())
            .collect();
        assert_eq!(got_ids, vec![2, 1, 3, 4]);
    }

    #[test]
    fn events_in_time_range_respects_bounds_and_orders_within_same_time() {
        let mut map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        map.insert(
            YCoordinate::from(0.0),
            vec![mk_event(2, 0.0, 1), mk_event(1, 0.0, 1)],
        );
        map.insert(YCoordinate::from(1.0), vec![mk_event(3, 1.0, 2)]);

        let idx = AllEventsIndex::new(map);

        let got_ids: Vec<usize> = idx
            .events_in_time_range(TimeSpan::SECOND..TimeSpan::SECOND * 2)
            .into_iter()
            .map(|ev| ev.id.value())
            .collect();
        assert_eq!(got_ids, vec![1, 2]);
    }

    #[test]
    fn events_in_time_range_swaps_reversed_bounds() {
        use std::ops::Bound::{Included, Unbounded};

        let mut map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        map.insert(YCoordinate::from(0.0), vec![mk_event(1, 0.0, 1)]);
        map.insert(YCoordinate::from(1.0), vec![mk_event(2, 1.0, 2)]);

        let idx = AllEventsIndex::new(map);

        let got_ids: Vec<usize> = idx
            .events_in_time_range((Included(TimeSpan::SECOND * 2), Included(TimeSpan::SECOND)))
            .into_iter()
            .map(|ev| ev.id.value())
            .collect();
        assert_eq!(got_ids, vec![1, 2]);

        let got_ids_unbounded: Vec<usize> = idx
            .events_in_time_range((Unbounded, Included(TimeSpan::SECOND)))
            .into_iter()
            .map(|ev| ev.id.value())
            .collect();
        assert_eq!(got_ids_unbounded, vec![1]);
    }

    #[test]
    fn events_in_time_range_offset_from_returns_empty_when_end_is_negative() {
        let mut map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        map.insert(YCoordinate::from(0.0), vec![mk_event(1, 0.0, 0)]);
        map.insert(YCoordinate::from(1.0), vec![mk_event(2, 1.0, 1)]);

        let idx = AllEventsIndex::new(map);

        assert!(
            idx.events_in_time_range_offset_from(
                TimeSpan::MILLISECOND * 100,
                ..=(TimeSpan::ZERO - TimeSpan::MILLISECOND * 200),
            )
            .into_iter()
            .map(|ev| ev.id.value())
            .next()
            .is_none()
        );
    }

    #[test]
    fn events_in_time_range_offset_from_excludes_zero_when_end_is_excluded() {
        let mut map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        map.insert(YCoordinate::from(0.0), vec![mk_event(1, 0.0, 0)]);

        let idx = AllEventsIndex::new(map);

        assert!(
            idx.events_in_time_range_offset_from(TimeSpan::ZERO, ..TimeSpan::ZERO)
                .into_iter()
                .map(|ev| ev.id.value())
                .next()
                .is_none()
        );
    }

    #[test]
    fn events_in_time_range_offset_from_clamps_negative_start_to_zero() {
        let mut map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        map.insert(YCoordinate::from(0.0), vec![mk_event(1, 0.0, 0)]);
        map.insert(YCoordinate::from(1.0), vec![mk_event(2, 1.0, 1)]);

        let idx = AllEventsIndex::new(map);

        let got_ids: Vec<usize> = idx
            .events_in_time_range_offset_from(
                TimeSpan::MILLISECOND * 100,
                (TimeSpan::ZERO - TimeSpan::MILLISECOND * 200)..=TimeSpan::ZERO,
            )
            .into_iter()
            .map(|ev| ev.id.value())
            .collect();
        assert_eq!(got_ids, vec![1]);
    }
}
