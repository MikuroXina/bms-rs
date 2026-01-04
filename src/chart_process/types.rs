//! Type definition module

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::ops::{Bound, Range, RangeBounds};
use std::path::PathBuf;
use std::time::Duration;

use num::{One, ToPrimitive, Zero};

pub use super::TimeSpan;
pub use super::core::FlowEvent;
use crate::bms::prelude::Bms;
#[cfg(feature = "bmson")]
use crate::bmson::prelude::Bmson;
use crate::{bms::Decimal, chart_process::ChartEvent};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

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

impl AsRef<Decimal> for ManualBpmGenerator {
    fn as_ref(&self) -> &Decimal {
        &self.0
    }
}

impl From<Decimal> for ManualBpmGenerator {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<ManualBpmGenerator> for Decimal {
    fn from(value: ManualBpmGenerator) -> Self {
        value.0
    }
}

impl ManualBpmGenerator {
    /// Returns a reference to the contained BPM value.
    #[must_use]
    pub const fn value(&self) -> &Decimal {
        &self.0
    }

    /// Consumes self and returns the contained BPM value.
    #[must_use]
    pub fn into_value(self) -> Decimal {
        self.0
    }
}

/// Base BPM wrapper type, encapsulating a `Decimal` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseBpm(pub Decimal);

impl AsRef<Decimal> for BaseBpm {
    fn as_ref(&self) -> &Decimal {
        &self.0
    }
}

impl BaseBpm {
    /// Create a new `BaseBpm`
    #[must_use]
    pub const fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Returns a reference to the contained BPM value.
    #[must_use]
    pub const fn value(&self) -> &Decimal {
        &self.0
    }

    /// Consumes self and returns the contained BPM value.
    #[must_use]
    pub fn into_value(self) -> Decimal {
        self.0
    }
}

impl From<Decimal> for BaseBpm {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<BaseBpm> for Decimal {
    fn from(value: BaseBpm) -> Self {
        value.0
    }
}

/// Visible range per BPM, representing the relationship between BPM and visible Y range.
/// Formula: `visible_y_range` = `current_bpm` * `visible_range_per_bpm`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleRangePerBpm(Decimal);

impl AsRef<Decimal> for VisibleRangePerBpm {
    fn as_ref(&self) -> &Decimal {
        &self.0
    }
}

impl VisibleRangePerBpm {
    /// Create a new `VisibleRangePerBpm` from base BPM and reaction time
    /// Formula: `visible_range_per_bpm` = `reaction_time_seconds` * 240 / `base_bpm`
    #[must_use]
    pub fn new(base_bpm: &BaseBpm, reaction_time: TimeSpan) -> Self {
        if base_bpm.value().is_zero() {
            Self(Decimal::zero())
        } else {
            Self(
                Decimal::from(reaction_time.as_nanos().max(0)) / NANOS_PER_SECOND
                    * Decimal::from(240u64)
                    / base_bpm.value().clone(),
            )
        }
    }

    /// Returns a reference to the contained value.
    #[must_use]
    pub const fn value(&self) -> &Decimal {
        &self.0
    }

    /// Consumes self and returns the contained value.
    #[must_use]
    pub fn into_value(self) -> Decimal {
        self.0
    }

    /// Calculate visible window length in y units based on current BPM, speed, and playback ratio.
    /// Formula: `visible_window_y = current_bpm * visible_range_per_bpm * current_speed * playback_ratio / 240`
    /// This ensures events stay in visible window for exactly `reaction_time` duration.
    #[must_use]
    pub fn window_y(
        &self,
        current_bpm: &Decimal,
        current_speed: &Decimal,
        playback_ratio: &Decimal,
    ) -> YCoordinate {
        let speed_factor = current_speed * playback_ratio;
        let adjusted = current_bpm * self.value() * speed_factor / Decimal::from(240u64);
        YCoordinate::new(adjusted)
    }

    /// Calculate reaction time from visible range per BPM
    /// Formula: `reaction_time` = `visible_range_per_bpm` / `playhead_speed`
    /// where `playhead_speed` = 1/240 (Y/sec per BPM)
    #[must_use]
    pub fn to_reaction_time(&self) -> TimeSpan {
        if self.0.is_zero() {
            TimeSpan::ZERO
        } else {
            let base = &self.0 * &Decimal::from(240);
            let nanos = (&base * &Decimal::from(NANOS_PER_SECOND))
                .to_u64()
                .unwrap_or(0);
            TimeSpan::from_duration(Duration::from_nanos(nanos))
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

impl From<VisibleRangePerBpm> for Decimal {
    fn from(value: VisibleRangePerBpm) -> Self {
        value.0
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

impl AsRef<Decimal> for YCoordinate {
    fn as_ref(&self) -> &Decimal {
        &self.0
    }
}

impl YCoordinate {
    /// Create a new `YCoordinate`
    #[must_use]
    pub const fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Returns a reference to the contained value.
    #[must_use]
    pub const fn value(&self) -> &Decimal {
        &self.0
    }

    /// Consumes self and returns the contained value.
    #[must_use]
    pub fn into_value(self) -> Decimal {
        self.0
    }

    /// Creates a zero of Y coordinate.
    #[must_use]
    pub fn zero() -> Self {
        Self(Decimal::zero())
    }
}

impl From<Decimal> for YCoordinate {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<YCoordinate> for Decimal {
    fn from(value: YCoordinate) -> Self {
        value.0
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

impl std::ops::Add for &YCoordinate {
    type Output = YCoordinate;

    fn add(self, rhs: Self) -> Self::Output {
        YCoordinate(&self.0 + &rhs.0)
    }
}

impl std::ops::Sub for YCoordinate {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Sub for &YCoordinate {
    type Output = YCoordinate;

    fn sub(self, rhs: Self) -> Self::Output {
        YCoordinate(&self.0 - &rhs.0)
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

impl std::ops::Div for &YCoordinate {
    type Output = YCoordinate;

    fn div(self, rhs: Self) -> Self::Output {
        YCoordinate(&self.0 / &rhs.0)
    }
}

impl Zero for YCoordinate {
    fn zero() -> Self {
        Self(Decimal::zero())
    }

    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

/// Display ratio wrapper type, representing the actual position of a note in the display area.
///
/// 0 is the judgment line, 1 is the position where the note generally starts to appear.
/// The value of this type is only affected by: current Y, Y visible range, and current Speed, Scroll values.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct DisplayRatio(pub Decimal);

impl AsRef<Decimal> for DisplayRatio {
    fn as_ref(&self) -> &Decimal {
        &self.0
    }
}

impl DisplayRatio {
    /// Create a new `DisplayRatio`
    #[must_use]
    pub const fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Returns a reference to the contained value.
    #[must_use]
    pub const fn value(&self) -> &Decimal {
        &self.0
    }

    /// Consumes self and returns the contained value.
    #[must_use]
    pub fn into_value(self) -> Decimal {
        self.0
    }

    /// Create a `DisplayRatio` representing the judgment line (value 0)
    #[must_use]
    pub fn at_judgment_line() -> Self {
        Self(Decimal::zero())
    }

    /// Create a `DisplayRatio` representing the position where note starts to appear (value 1)
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

impl From<DisplayRatio> for Decimal {
    fn from(value: DisplayRatio) -> Self {
        value.0
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

    #[must_use]
    pub const fn as_events(&self) -> &Vec<PlayheadEvent> {
        &self.events
    }

    #[must_use]
    pub const fn as_by_y(&self) -> &BTreeMap<YCoordinate, Range<usize>> {
        &self.by_y
    }

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
    pub wav_files: HashMap<WavId, PathBuf>,
    /// BMP ID -> file path mapping.
    pub bmp_files: HashMap<BmpId, PathBuf>,
}

/// Parsed chart data containing all precomputed information.
///
/// This structure is immutable and can be used to create multiple player instances.
#[derive(Debug, Clone)]
pub struct ParsedChart {
    /// Resource file mapping.
    pub resources: ChartResources,
    /// Event index (by Y coordinate and time).
    pub events: AllEventsIndex,
    /// Flow event mapping (affects playback speed).
    pub flow_events: BTreeMap<YCoordinate, Vec<FlowEvent>>,
    /// Initial BPM.
    pub init_bpm: Decimal,
    /// Initial Speed (BMS-specific, BMSON defaults to 1.0).
    pub init_speed: Decimal,
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
