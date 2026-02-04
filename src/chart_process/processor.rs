//! Module for chart processors

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::ops::{Bound, Range, RangeBounds};
use std::path::PathBuf;

use crate::bms::command::channel::NoteKind;
use crate::chart_process::{ChartEvent, FlowEvent, PlayheadEvent, TimeSpan, YCoordinate};
use strict_num_extended::FinF64;

pub mod bms;
pub mod bmson;

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

/// Index for all chart events, organized by Y coordinate and time.
///
/// This structure provides efficient lookups for events by their Y coordinate
/// and activation time. For long notes, we maintain precomputed indices
/// for both start and end positions to support efficient visibility queries.
///
/// # Long Note Visibility
///
/// Long notes should remain visible even when their start position has
/// passed the judgment line, as long as any part of them is still
/// within the visible window.
///
/// The `visible_ln_by_start` and `visible_ln_by_end` indices enable
/// O(log n) queries for long notes that intersect with the view range,
/// making this implementation suitable for time rewind functionality.
#[derive(Debug, Clone)]
pub struct AllEventsIndex {
    events: Vec<PlayheadEvent>,
    by_y: BTreeMap<YCoordinate, Range<usize>>,
    by_time: BTreeMap<TimeSpan, Vec<usize>>,
    /// Maps long note start position to event index.
    visible_ln_by_start: BTreeMap<YCoordinate, usize>,
    /// Maps long note end position to event index.
    visible_ln_by_end: BTreeMap<YCoordinate, usize>,
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

        // Build precomputed indices for long notes.
        let mut visible_ln_by_start: BTreeMap<YCoordinate, usize> = BTreeMap::new();
        let mut visible_ln_by_end: BTreeMap<YCoordinate, usize> = BTreeMap::new();

        for (idx, ev) in events.iter().enumerate() {
            if let ChartEvent::Note {
                kind: NoteKind::Long,
                length: Some(length),
                ..
            } = ev.event()
            {
                let start_y = ev.position().clone();
                let end_y = start_y.clone() + length.clone();

                visible_ln_by_start.insert(start_y, idx);
                visible_ln_by_end.insert(end_y, idx);
            }
        }

        Self {
            events,
            by_y,
            by_time,
            visible_ln_by_start,
            visible_ln_by_end,
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
    /// An event is considered visible in `(start, end]` if and only if:
    /// - Normal events: position is within `(start, end]`
    /// - Long notes: `end_y` > `start` AND `start_y` <= `end`
    ///
    /// This method uses precomputed indices for long notes to efficiently locate
    /// events in the range. Finding the range is O(log N), but cloning events
    /// results in O(N) overall complexity. This makes it suitable for time rewind
    /// functionality.
    ///
    /// # Parameters
    /// - `range`: The Y coordinate range to query (start, end]
    ///
    /// # Returns
    /// A vector of events within the specified range
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Get events visible in a window from 10.0 to 20.0
    /// let events = index.events_in_y_range(10.0..=20.0);
    ///
    /// // Long notes intersecting the view are included
    /// // - LN starting at 5.0 and ending at 15.0: included (intersects)
    /// // - LN starting at 10.0 and ending at 20.0: included (within view)
    /// // - LN starting at 15.0 and ending at 25.0: included (intersects)
    /// // - LN starting at 5.0 and ending at 8.0: excluded (completely before)
    /// // - LN starting at 22.0 and ending at 30.0: excluded (completely after)
    /// ```
    #[must_use]
    pub fn events_in_y_range<R>(&self, range: R) -> Vec<PlayheadEvent>
    where
        R: RangeBounds<YCoordinate> + Clone,
    {
        let view_start = match range.start_bound() {
            Bound::Included(start) | Bound::Excluded(start) => start.clone(),
            Bound::Unbounded => YCoordinate::zero(),
        };

        let view_end = match range.end_bound() {
            Bound::Included(end) | Bound::Excluded(end) => end.clone(),
            Bound::Unbounded => YCoordinate::from(f64::MAX),
        };

        let start_inclusive = matches!(range.start_bound(), Bound::Included(_));

        // Optimization: Pre-allocate capacity based on estimated event count
        let estimated_capacity: usize = self
            .by_y
            .range(range.clone())
            .map(|(_, idx_range)| idx_range.len())
            .sum();
        let mut visible = Vec::with_capacity(estimated_capacity);

        // Step 1: Collect normal events (exclude long notes)
        for (_, idx_range) in self.by_y.range(range) {
            for idx in idx_range.clone() {
                let Some(ev) = self.events.get(idx) else {
                    continue;
                };

                // Skip long notes (will be processed in step 2)
                if matches!(
                    ev.event(),
                    ChartEvent::Note {
                        kind: NoteKind::Long,
                        ..
                    }
                ) {
                    continue;
                }

                // Normal events: process immediately
                let start_y = ev.position();
                let passes_start = if start_inclusive {
                    *start_y >= view_start
                } else {
                    *start_y > view_start
                };

                if passes_start && *start_y <= view_end {
                    visible.push(ev.clone());
                }
            }
        }

        // Step 2: Collect long notes using non-overlapping BTreeMap range queries
        // LN visible condition: end_y > view_start AND start_y <= view_end
        // To avoid deduplication, we use three mutually exclusive queries:
        let lower_bound = if start_inclusive {
            Bound::Included(view_start.clone())
        } else {
            Bound::Excluded(view_start.clone())
        };

        // 2.1: LNs starting in [view_start, view_end] (automatically satisfy end_y > view_start)
        for (_, &idx) in self
            .visible_ln_by_start
            .range((lower_bound.clone(), Bound::Included(view_end.clone())))
        {
            if let Some(ev) = self.events.get(idx) {
                visible.push(ev.clone());
            }
        }

        // 2.2: LNs with end_y in [view_start, view_end] but start_y < view_start
        for (_, &idx) in self
            .visible_ln_by_end
            .range((lower_bound, Bound::Included(view_end.clone())))
        {
            let Some(ev) = self.events.get(idx) else {
                continue;
            };
            let ln_start = ev.position();
            if *ln_start < view_start {
                visible.push(ev.clone());
            }
        }

        // 2.3: LNs with end_y > view_end but start_y < view_start (fully cover the view)
        for (_, &idx) in self
            .visible_ln_by_end
            .range((Bound::Excluded(view_end), Bound::Unbounded))
        {
            let Some(ev) = self.events.get(idx) else {
                continue;
            };
            let ln_start = ev.position();
            if *ln_start < view_start {
                visible.push(ev.clone());
            }
        }

        visible
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

/// Playable chart data containing all precomputed information.
///
/// This structure is immutable and ready for playback. It can be used to create
/// multiple player instances. Note that this structure does NOT contain playback
/// state - playback state is managed by `ChartPlayer`.
#[derive(Debug, Clone)]
pub struct PlayableChart {
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

impl PlayableChart {
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

    /// Create a new `PlayableChart` from its constituent parts.
    ///
    /// This is an internal constructor used by chart processors to assemble
    /// a parsed chart from its components.
    #[must_use]
    pub(crate) const fn from_parts(
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

    use super::AllEventsIndex;
    use super::ChartEventId;
    use crate::bms::command::channel::{Key, NoteKind, PlayerSide};
    use crate::chart_process::{ChartEvent, PlayheadEvent, TimeSpan, YCoordinate};

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

    #[test]
    fn events_in_y_range_includes_long_notes_intersecting_view() {
        // Test case 1: LN start and end within view
        let mut map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        map.insert(
            YCoordinate::from(5.0),
            vec![PlayheadEvent::new(
                ChartEventId::new(1),
                YCoordinate::from(5.0),
                ChartEvent::Note {
                    side: PlayerSide::Player1,
                    key: Key::Key(1),
                    kind: NoteKind::Long,
                    wav_id: None,
                    length: Some(YCoordinate::from(3.0)), // ends at 8.0
                    continue_play: None,
                },
                TimeSpan::ZERO,
            )],
        );
        map.insert(YCoordinate::from(15.0), vec![mk_event(2, 15.0, 0)]);

        let idx = AllEventsIndex::new(map);

        // LN should be included (start=5.0, end=8.0, both within range)
        assert!(
            idx.events_in_y_range((
                std::ops::Bound::Included(YCoordinate::from(0.0)),
                std::ops::Bound::Included(YCoordinate::from(10.0)),
            ))
            .into_iter()
            .map(|ev| ev.id.value())
            .any(|x| x == 1)
        );
    }

    #[test]
    fn events_in_y_range_includes_ln_starting_before_view() {
        // Test case 2: LN starts before view, ends within view
        let mut map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        map.insert(
            YCoordinate::from(5.0),
            vec![PlayheadEvent::new(
                ChartEventId::new(1),
                YCoordinate::from(5.0),
                ChartEvent::Note {
                    side: PlayerSide::Player1,
                    key: Key::Key(1),
                    kind: NoteKind::Long,
                    wav_id: None,
                    length: Some(YCoordinate::from(3.0)), // ends at 8.0
                    continue_play: None,
                },
                TimeSpan::ZERO,
            )],
        );

        let idx = AllEventsIndex::new(map);

        // LN should be included (intersects with view)
        assert!(
            idx.events_in_y_range((
                std::ops::Bound::Included(YCoordinate::from(7.0)),
                std::ops::Bound::Included(YCoordinate::from(10.0)),
            ))
            .into_iter()
            .map(|ev| ev.id.value())
            .any(|x| x == 1)
        );
    }

    #[test]
    fn events_in_y_range_includes_ln_ending_after_view() {
        // Test case 3: LN starts within view, ends after view
        let mut map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        map.insert(
            YCoordinate::from(5.0),
            vec![PlayheadEvent::new(
                ChartEventId::new(1),
                YCoordinate::from(5.0),
                ChartEvent::Note {
                    side: PlayerSide::Player1,
                    key: crate::bms::prelude::Key::Key(1),
                    kind: NoteKind::Long,
                    wav_id: None,
                    length: Some(YCoordinate::from(10.0)), // ends at 15.0
                    continue_play: None,
                },
                TimeSpan::ZERO,
            )],
        );

        let idx = AllEventsIndex::new(map);

        // LN should be included (intersects with view)
        assert!(
            idx.events_in_y_range((
                std::ops::Bound::Included(YCoordinate::from(0.0)),
                std::ops::Bound::Included(YCoordinate::from(10.0)),
            ))
            .into_iter()
            .map(|ev| ev.id.value())
            .any(|x| x == 1)
        );
    }

    #[test]
    fn events_in_y_range_includes_ln_fully_covering_view() {
        // Test case 4: LN starts before and ends after view (fully covers)
        let mut map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        map.insert(
            YCoordinate::from(0.0),
            vec![PlayheadEvent::new(
                ChartEventId::new(1),
                YCoordinate::from(0.0),
                ChartEvent::Note {
                    side: PlayerSide::Player1,
                    key: crate::bms::prelude::Key::Key(1),
                    kind: NoteKind::Long,
                    wav_id: None,
                    length: Some(YCoordinate::from(20.0)), // ends at 20.0
                    continue_play: None,
                },
                TimeSpan::ZERO,
            )],
        );

        let idx = AllEventsIndex::new(map);

        // LN should be included (fully covers the view)
        assert!(
            idx.events_in_y_range((
                std::ops::Bound::Included(YCoordinate::from(5.0)),
                std::ops::Bound::Included(YCoordinate::from(15.0)),
            ))
            .into_iter()
            .map(|ev| ev.id.value())
            .any(|x| x == 1)
        );
    }

    #[test]
    fn events_in_y_range_excludes_ln_before_view() {
        // Test case 5: LN completely before view
        let mut map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        map.insert(
            YCoordinate::from(0.0),
            vec![PlayheadEvent::new(
                ChartEventId::new(1),
                YCoordinate::from(0.0),
                ChartEvent::Note {
                    side: PlayerSide::Player1,
                    key: crate::bms::prelude::Key::Key(1),
                    kind: NoteKind::Long,
                    wav_id: None,
                    length: Some(YCoordinate::from(3.0)), // ends at 3.0
                    continue_play: None,
                },
                TimeSpan::ZERO,
            )],
        );
        map.insert(YCoordinate::from(10.0), vec![mk_event(2, 10.0, 0)]);

        let idx = AllEventsIndex::new(map);

        // Query range [5.0, 15.0]
        let got_ids: Vec<usize> = idx
            .events_in_y_range((
                std::ops::Bound::Included(YCoordinate::from(5.0)),
                std::ops::Bound::Included(YCoordinate::from(15.0)),
            ))
            .into_iter()
            .map(|ev| ev.id.value())
            .collect();

        // LN should be excluded (completely before view)
        assert!(!got_ids.contains(&1));
        // Normal event should be included
        assert!(got_ids.contains(&2));
    }

    #[test]
    fn events_in_y_range_excludes_ln_after_view() {
        // Test case 6: LN completely after view
        let mut map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        map.insert(YCoordinate::from(0.0), vec![mk_event(1, 0.0, 0)]);
        map.insert(
            YCoordinate::from(20.0),
            vec![PlayheadEvent::new(
                ChartEventId::new(2),
                YCoordinate::from(20.0),
                ChartEvent::Note {
                    side: PlayerSide::Player1,
                    key: crate::bms::prelude::Key::Key(1),
                    kind: NoteKind::Long,
                    wav_id: None,
                    length: Some(YCoordinate::from(5.0)), // ends at 25.0
                    continue_play: None,
                },
                TimeSpan::ZERO,
            )],
        );

        let idx = AllEventsIndex::new(map);

        // Query range [0.0, 10.0]
        let got_ids: Vec<usize> = idx
            .events_in_y_range((
                std::ops::Bound::Included(YCoordinate::from(0.0)),
                std::ops::Bound::Included(YCoordinate::from(10.0)),
            ))
            .into_iter()
            .map(|ev| ev.id.value())
            .collect();

        // LN should be excluded (completely after view)
        assert!(!got_ids.contains(&2));
        // Normal event should be included
        assert!(got_ids.contains(&1));
    }

    #[test]
    fn events_in_y_range_prevents_duplicate_long_notes() {
        // Test that LN is not added twice when both start and end in range
        let mut map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        map.insert(
            YCoordinate::from(5.0),
            vec![PlayheadEvent::new(
                ChartEventId::new(1),
                YCoordinate::from(5.0),
                ChartEvent::Note {
                    side: PlayerSide::Player1,
                    key: crate::bms::prelude::Key::Key(1),
                    kind: NoteKind::Long,
                    wav_id: None,
                    length: Some(YCoordinate::from(3.0)), // ends at 8.0
                    continue_play: None,
                },
                TimeSpan::ZERO,
            )],
        );

        let idx = AllEventsIndex::new(map);

        // Query range [0.0, 10.0] (both start and end within range)
        let got_ids: Vec<usize> = idx
            .events_in_y_range((
                std::ops::Bound::Included(YCoordinate::from(0.0)),
                std::ops::Bound::Included(YCoordinate::from(10.0)),
            ))
            .into_iter()
            .map(|ev| ev.id.value())
            .collect();

        // LN should appear exactly once
        let count = got_ids.iter().filter(|&&id| id == 1).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn events_in_y_range_respects_excluded_start_bound() {
        // Test that (start, end] correctly excludes start for normal notes
        // For long notes, if they intersect with view (even if start is excluded), they are included
        let mut map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        map.insert(
            YCoordinate::from(5.0),
            vec![mk_event(1, 5.0, 0)], // Normal note at 5.0
        );
        map.insert(YCoordinate::from(6.0), vec![mk_event(2, 6.0, 0)]);

        let idx = AllEventsIndex::new(map);

        // Query range (5.0, 10.0] - should exclude event at 5.0
        let got_ids: Vec<usize> = idx
            .events_in_y_range((
                std::ops::Bound::Excluded(YCoordinate::from(5.0)),
                std::ops::Bound::Included(YCoordinate::from(10.0)),
            ))
            .into_iter()
            .map(|ev| ev.id.value())
            .collect();

        // Normal note at excluded bound should NOT be included
        assert!(!got_ids.contains(&1));
        // Normal event after bound should be included
        assert!(got_ids.contains(&2));
    }
}
