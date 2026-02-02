//! # Chart Processor
//!
//! ## Y Coordinate Definition
//!
//! - In the default 4/4 time signature, the length of "one measure" is 1.
//! - BMS: When the section length is the default value, each `Track` has a length of 1. The section length comes from the `#XXX02:V` message per measure, where `V` represents the multiple of the default length (e.g., `#00302:0.5` means the 3rd measure has half the default length). Cumulative y is linearly converted with this multiple.
//! - BMSON: `info.resolution` is the number of pulses corresponding to a quarter note (1/4), so one measure length is `4 * resolution` pulses; all position y is normalized to measure units through `pulses / (4 * resolution)`.
//! - Speed (default 1.0): Only affects display coordinates (e.g., `visible_notes` `distance_to_hit`), that is, scales the y difference proportionally; does not change time progression and BPM values, nor the actual duration of that measure.
//!
//! ## Values and Formulas
//!
//! ### Core Constants
//!
//! - `NANOS_PER_SECOND = 1_000_000_000`: Nanoseconds per second for time calculations
//! - `BASE_VELOCITY_FACTOR = 1/240`: Base velocity factor (Y/sec per BPM)
//!
//! ### Visible Range Configuration
//!
//! **`VisibleRangePerBpm` creation:**
//! ```text
//! visible_range_per_bpm = reaction_time_seconds * 240 / base_bpm
//! ```
//!
//! **Simplified visible range (for display):**
//! ```text
//! visible_y_range = current_bpm * visible_range_per_bpm
//! ```
//!
//! **Full visible window (includes speed and playback ratio):**
//! ```text
//! visible_window_y = (current_speed * playback_ratio / 240) * reaction_time * base_bpm
//! ```
//!
//! This ensures events stay in visible window for exactly `reaction_time * base_bpm / current_bpm` duration.
//!
//! ### Time Progression
//!
//! **Velocity (Y units per second):**
//! ```text
//! velocity = (current_bpm / 240) * current_speed * playback_ratio
//! ```
//!
//! **Time integration (`step_to` algorithm):**
//! ```text
//! delta_y = velocity * elapsed_time_nanos / NANOS_PER_SECOND
//! time_to_event = distance_y / velocity * NANOS_PER_SECOND
//! ```
//!
//! ### Display Coordinates
//!
//! **Display ratio (0 = judgment line, 1 = appearance position):**
//! ```text
//! display_ratio = (event_y - current_y) / visible_window_y * current_scroll
//! ```
//!
//! The value of this type is only affected by: current Y, Y visible range, and current Speed, Scroll values.
//!
//! ### Reaction Time
//!
//! **Calculate reaction time from visible range per BPM:**
//! ```text
//! reaction_time = visible_range_per_bpm / playhead_speed
//! where playhead_speed = 1/240
//! ```

pub use gametime::TimeSpan;

use crate::bms::prelude::SwBgaEvent;
use crate::bms::{
    Decimal,
    prelude::{Argb, BgaLayer, Key, NoteKind, PlayerSide},
};
use crate::chart_process::processor::{BmpId, ChartEventId, WavId};
use num::Zero;

pub mod base_bpm;
pub mod processor;

// Player module
pub mod player;

// Prelude module
pub mod prelude;

/// Events generated during playback (Elm style).
///
/// These events represent actual events during chart playback, such as note triggers, BGM playback,
/// BPM changes, etc. Setting and control related events have been separated into [`ControlEvent`].
///
/// The effects of [`ChartEvent`] members on [`YCoordinate`] and [`player::DisplayRatio`] are calculated by the corresponding
/// processor implementation, so there's no need to recalculate them.
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

/// Flow events that affect playback speed/scroll.
#[derive(Debug, Clone)]
pub enum FlowEvent {
    /// BPM change event.
    Bpm(Decimal),
    /// Speed factor change event (BMS only).
    Speed(Decimal),
    /// Scroll factor change event.
    Scroll(Decimal),
}

/// Y coordinate wrapper type, using arbitrary precision decimal numbers.
///
/// Unified y unit description: In default 4/4 time, one measure equals 1; BMS uses `#SECLEN` for linear conversion, BMSON normalizes via `pulses / (4*resolution)` to measure units.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
