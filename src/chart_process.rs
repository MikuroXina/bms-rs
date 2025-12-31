//! Chart Processor
//!
//! Unified Y coordinate definition:
//! - In the default 4/4 time signature, the length of "one measure" is 1.
//! - BMS: When the section length is the default value, each `Track` has a length of 1. The section length comes from the `#XXX02:V` message per measure, where `V` represents the multiple of the default length (e.g., `#00302:0.5` means the 3rd measure has half the default length). Cumulative y is linearly converted with this multiple.
//! - BMSON: `info.resolution` is the number of pulses corresponding to a quarter note (1/4), so one measure length is `4 * resolution` pulses; all position y is normalized to measure units through `pulses / (4 * resolution)`.
//! - Speed (default 1.0): Only affects display coordinates (e.g., `visible_notes` `distance_to_hit`), that is, scales the y difference proportionally; does not change time progression and BPM values, nor the actual duration of that measure.

use std::collections::BTreeMap;

use gametime::TimeSpan;

use crate::bms::prelude::SwBgaEvent;
use crate::bms::{
    Decimal,
    prelude::{Argb, BgaLayer, Key, NoteKind, PlayerSide},
};
use crate::chart_process::core::FlowEvent;
use crate::chart_process::resource::{BmpId, ResourceMapping, WavId};

// BaseBpm logic
pub mod base_bpm;

// Core processor logic
pub mod core;

// Resource mapping module
pub mod resource;

// Y coordinate calculator module
pub mod y_calculator;

/// Output of chart processing.
///
/// Contains all the information needed for chart playback.
pub struct EventParseOutput<R: ResourceMapping> {
    /// All events with their positions and activation times
    pub all_events: AllEventsIndex,

    /// Flow events (BPM/Speed/Scroll changes) indexed by Y coordinate
    pub flow_events_by_y: BTreeMap<YCoordinate, Vec<FlowEvent>>,

    /// Initial BPM
    pub init_bpm: Decimal,

    /// Resource mapping
    pub resources: R,
}

/// Chart processor trait.
///
/// Defines the interface for processing different chart formats
/// into a unified `EventParseOutput`.
pub trait ChartProcessor<R: ResourceMapping> {
    /// Process the chart and generate event list.
    ///
    /// Returns an `EventParseOutput` containing all events and metadata.
    fn process(&self) -> EventParseOutput<R>;
}

// Chart player module
pub mod player;

// BMS processor implementation
pub mod bms_processor;

// BMSON processor implementation
pub mod bmson_processor;

// Prelude module
pub mod prelude;

/// Events generated during playback (Elm style).
///
/// These events represent actual events during chart playback, such as note triggers, BGM playback,
/// BPM changes, etc. Setting and control related events have been separated into [`ControlEvent`].
///
/// The effects of [`ChartEvent`] members on [`YCoordinate`] and [`DisplayRatio`] are calculated by the corresponding
/// [`ChartProcessor`] implementation, so there's no need to recalculate them.
#[derive(Debug, Clone)]
pub enum ChartEvent {
    /// Key note reaches judgment line (includes visible, long, mine, invisible notes, distinguished by `kind`)
    Note {
        /// Player side
        side: PlayerSide,
        /// Key position
        key: Key,
        /// Note type (`NoteKind`)
        kind: NoteKind,
        /// Corresponding sound resource ID (if any)
        wav_id: Option<WavId>,
        /// Note length (end position for long notes, None for regular notes)
        length: Option<YCoordinate>,
        /// Note continue play span. None for BMS; in BMSON, Some(span) when Note.c is true.
        continue_play: Option<TimeSpan>,
    },
    /// BGM and other non-key triggers (no valid side/key)
    Bgm {
        /// Corresponding sound resource ID (if any)
        wav_id: Option<WavId>,
    },
    /// BPM change
    BpmChange {
        /// New BPM value (beats per minute)
        bpm: Decimal,
    },
    /// Scroll factor change
    ScrollChange {
        /// Scroll factor (relative value)
        factor: Decimal,
    },
    /// Speed factor change
    SpeedChange {
        /// Spacing factor (relative value)
        factor: Decimal,
    },
    /// Stop scroll event
    Stop {
        /// Stop duration (BMS: converted from chart-defined time units; BMSON: pulse count)
        duration: Decimal,
    },
    /// BGA (background animation) change event
    ///
    /// Triggered when playback position reaches BGA change time point, indicating the need to switch to the specified background image.
    /// Supports multiple BGA layers: Base (base layer), Overlay (overlay layer), Overlay2 (second overlay layer), and Poor (displayed on failure).
    BgaChange {
        /// BGA layer
        layer: BgaLayer,
        /// BGA/BMP resource ID, get the corresponding file path through the `bmp_files()` method (if any)
        bmp_id: Option<BmpId>,
    },
    /// BGA opacity change event (requires minor-command feature)
    ///
    /// Dynamically adjust the opacity of the specified BGA layer to achieve fade-in/fade-out effects.
    BgaOpacityChange {
        /// BGA layer
        layer: BgaLayer,
        /// Opacity value (0x01-0xFF, 0x01 means almost transparent, 0xFF means completely opaque)
        opacity: u8,
    },
    /// BGA ARGB color change event (requires minor-command feature)
    ///
    /// Dynamically adjust the color of the specified BGA layer through ARGB values to achieve color filter effects.
    BgaArgbChange {
        /// BGA layer
        layer: BgaLayer,
        /// ARGB color value (format: 0xAARRGGBB)
        argb: Argb,
    },
    /// BGM volume change event
    ///
    /// Triggered when playback position reaches BGM volume change time point, used to adjust background music volume.
    BgmVolumeChange {
        /// Volume value (0x01-0xFF, 0x01 means minimum volume, 0xFF means maximum volume)
        volume: u8,
    },
    /// KEY volume change event
    ///
    /// Triggered when playback position reaches KEY volume change time point, used to adjust key sound effect volume.
    KeyVolumeChange {
        /// Volume value (0x01-0xFF, 0x01 means minimum volume, 0xFF means maximum volume)
        volume: u8,
    },
    /// Text display event
    ///
    /// Triggered when playback position reaches text display time point, used to display text information in the chart.
    TextDisplay {
        /// Text content to display
        text: String,
    },
    /// Judge level change event
    ///
    /// Triggered when playback position reaches judge level change time point, used to adjust the strictness of the judgment window.
    JudgeLevelChange {
        /// Judge level (`VeryHard`, Hard, Normal, Easy, `OtherInt`)
        level: crate::bms::command::JudgeLevel,
    },
    /// Video seek event (requires minor-command feature)
    ///
    /// Triggered when playback position reaches video seek time point, used for video playback control.
    VideoSeek {
        /// Seek time point (seconds)
        seek_time: f64,
    },
    /// BGA key binding event (requires minor-command feature)
    ///
    /// Triggered when playback position reaches BGA key binding time point, used for BGA and key binding control.
    BgaKeybound {
        /// BGA key binding event type
        event: SwBgaEvent,
    },
    /// Option change event (requires minor-command feature)
    ///
    /// Triggered when playback position reaches option change time point, used for dynamic game option adjustment.
    OptionChange {
        /// Option content
        option: String,
    },
    /// Measure line event
    ///
    /// Triggered when playback position reaches measure line position, used for chart structure display.
    BarLine,
}

/// Player control and setting events.
///
/// These events are used to control the player's configuration parameters, such as visible Y range.
/// Separated from chart playback related events (such as notes, BGM, BPM changes, etc.) to provide a clearer API.
#[derive(Debug, Clone)]
pub enum ControlEvent {
    /// Set: visible range per BPM
    ///
    /// The visible range per BPM controls the relationship between BPM and visible Y range.
    /// Formula: `visible_y_range` = `current_bpm` * `visible_range_per_bpm`
    /// This replaces the old `SetDefaultVisibleYLength` event.
    SetVisibleRangePerBpm {
        /// Visible range per BPM (y coordinate units per BPM, >0)
        visible_range_per_bpm: base_bpm::VisibleRangePerBpm,
    },
    /// Set: playback ratio
    ///
    /// Controls how fast the playback advances relative to real time.
    /// Default is 1.
    SetPlaybackRatio {
        /// Playback ratio (>= 0)
        ratio: Decimal,
    },
}

use std::cmp::Ordering;
use std::ops::{Bound, Range, RangeBounds};

use num::{One, Zero};

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

/// Indexed collection of all chart events.
///
/// Provides efficient lookup of events by Y coordinate or time.
#[derive(Debug, Clone)]
pub struct AllEventsIndex {
    events: Vec<core::PlayheadEvent>,
    by_y: BTreeMap<YCoordinate, Range<usize>>,
    by_time: BTreeMap<TimeSpan, Vec<usize>>,
}

impl AllEventsIndex {
    /// Create a new event index from a map of Y coordinates to events.
    ///
    /// # Arguments
    /// * `map` - Events indexed by their Y coordinate
    #[must_use]
    pub fn new(map: BTreeMap<YCoordinate, Vec<core::PlayheadEvent>>) -> Self {
        let mut events: Vec<core::PlayheadEvent> = Vec::new();
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

    /// Get all events within a Y coordinate range.
    ///
    /// # Arguments
    /// * `range` - The Y coordinate range to query
    #[must_use]
    pub fn events_in_y_range<R>(&self, range: R) -> Vec<core::PlayheadEvent>
    where
        R: RangeBounds<YCoordinate>,
    {
        self.by_y
            .range(range)
            .flat_map(|(_, indices)| self.events.get(indices.clone()).into_iter().flatten())
            .cloned()
            .collect()
    }

    /// Get all events within a time range.
    ///
    /// # Arguments
    /// * `range` - The time range to query
    pub fn events_in_time_range<R>(&self, range: R) -> Vec<core::PlayheadEvent>
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

    /// Get all events within a time range offset from a center time.
    ///
    /// # Arguments
    /// * `center` - The center time point
    /// * `range` - The offset range from the center
    pub fn events_in_time_range_offset_from<R>(
        &self,
        center: TimeSpan,
        range: R,
    ) -> Vec<core::PlayheadEvent>
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        AllEventsIndex, ChartEvent, TimeSpan, YCoordinate,
        core::{ChartEventId, PlayheadEvent},
    };

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
