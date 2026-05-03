//! Chart event types

use crate::chart::process::{BmpId, ChartEventId, WavId};
use crate::chart::types::{Argb, BgaLayer, Key, NoteKind, PlayerSide};
use gametime::TimeSpan;
use strict_num_extended::FinF64;
use strict_num_extended::NonNegativeF64;
use strict_num_extended::PositiveF64;

use crate::chart::MAX_NON_NEGATIVE_F64;

/// Y coordinate wrapper type.
///
/// Represents a non-negative position on the timeline (measure units).
/// Unified y unit description: In default 4/4 time, one measure equals 1; BMS uses `#SECLEN` for linear conversion, BMSON normalizes via `pulses / (4*resolution)` to measure units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct YCoordinate(pub NonNegativeF64);

impl YCoordinate {
    /// Create a new `YCoordinate` from `NonNegativeF64`.
    #[must_use]
    pub const fn new(value: NonNegativeF64) -> Self {
        Self(value)
    }

    /// Get the internal `NonNegativeF64` value.
    #[must_use]
    pub const fn value(&self) -> &NonNegativeF64 {
        &self.0
    }

    /// Convert to f64.
    #[must_use]
    pub const fn as_f64(&self) -> f64 {
        self.0.as_f64()
    }

    /// Zero value.
    pub const ZERO: Self = Self(NonNegativeF64::ZERO);
    /// One value.
    pub const ONE: Self = Self(NonNegativeF64::ONE);
}

impl From<NonNegativeF64> for YCoordinate {
    fn from(value: NonNegativeF64) -> Self {
        Self(value)
    }
}

impl From<YCoordinate> for NonNegativeF64 {
    fn from(value: YCoordinate) -> Self {
        value.0
    }
}

impl AsRef<NonNegativeF64> for YCoordinate {
    fn as_ref(&self) -> &NonNegativeF64 {
        &self.0
    }
}

impl std::ops::Add for YCoordinate {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.add(rhs.0).unwrap_or(MAX_NON_NEGATIVE_F64))
    }
}

impl std::ops::Add<NonNegativeF64> for YCoordinate {
    type Output = Self;

    fn add(self, rhs: NonNegativeF64) -> Self::Output {
        Self(self.0.add(rhs).unwrap_or(MAX_NON_NEGATIVE_F64))
    }
}

impl std::ops::Sub for YCoordinate {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(NonNegativeF64::new(self.0.as_f64() - rhs.0.as_f64()).unwrap_or(NonNegativeF64::ZERO))
    }
}

impl std::ops::Sub<NonNegativeF64> for YCoordinate {
    type Output = Self;

    fn sub(self, rhs: NonNegativeF64) -> Self::Output {
        Self(NonNegativeF64::new(self.0.as_f64() - rhs.as_f64()).unwrap_or(NonNegativeF64::ZERO))
    }
}

impl std::ops::Mul<FinF64> for YCoordinate {
    type Output = Self;

    fn mul(self, rhs: FinF64) -> Self::Output {
        Self(NonNegativeF64::new(self.0.as_f64() * rhs.as_f64()).unwrap_or(self.0))
    }
}

impl std::ops::Div<FinF64> for YCoordinate {
    type Output = Self;

    fn div(self, rhs: FinF64) -> Self::Output {
        Self(NonNegativeF64::new(self.0.as_f64() / rhs.as_f64()).unwrap_or(self.0))
    }
}

/// BMS-specific events enum.
/// Contains all BMS-specific events that don't apply to BMSON.
#[derive(Debug, Clone)]
pub enum BmsEvent {
    /// BGA opacity change event.
    BgaOpacityChange {
        /// Target BGA layer.
        layer: BgaLayer,
        /// Opacity value (0-255).
        opacity: u8,
    },
    /// BGA ARGB color change event.
    BgaArgbChange {
        /// Target BGA layer.
        layer: BgaLayer,
        /// ARGB color value.
        argb: Argb,
    },
    /// BGM volume change event.
    BgmVolumeChange {
        /// Volume value (0-255).
        volume: u8,
    },
    /// Key volume change event.
    KeyVolumeChange {
        /// Volume value (0-255).
        volume: u8,
    },
    /// Text display event.
    TextDisplay {
        /// Display text content.
        text: String,
    },
    /// Judge level change event.
    JudgeLevelChange {
        /// New judge level.
        level: crate::bms::command::JudgeLevel,
    },
    /// Video seek event.
    VideoSeek {
        /// Seek time point (seconds)
        seek_time: f64,
    },
    /// BGA key binding event.
    BgaKeybound {
        /// BGA event to bind.
        event: crate::bms::command::minor_command::SwBgaEvent,
    },
    /// Option change event.
    OptionChange {
        /// Option name.
        option: String,
    },
}

/// Events generated during playback (Elm style).
///
/// These events represent actual events during chart playback, such as note triggers, BGM playback,
/// BPM changes, etc.
///
/// The effects of [`ChartEvent`] members on Y coordinates and [`crate::chart::player::DisplayRatio`] are calculated by the corresponding
/// process implementation, so there's no need to recalculate them.
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
        length: Option<NonNegativeF64>,
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
        bpm: PositiveF64,
    },
    /// Scroll factor change
    ScrollChange {
        /// Scroll factor (relative value)
        factor: FinF64,
    },
    /// Speed factor change
    SpeedChange {
        /// Spacing factor (relative value)
        factor: PositiveF64,
    },
    /// Stop scroll event
    Stop {
        /// Stop duration (BMS: converted from chart-defined time units; BMSON: pulse count)
        duration: NonNegativeF64,
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
    /// BMS-specific event (BMS only, not applicable to BMSON)
    Bms(BmsEvent),
    /// Measure line event
    ///
    /// Triggered when playback position reaches measure line position, used for chart structure display.
    BarLine,
}

/// Timeline event and position wrapper type.
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
            id,
            position,
            event,
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
    Bpm(PositiveF64),
    /// Speed factor change event (BMS only).
    Speed(PositiveF64),
    /// Scroll factor change event.
    Scroll(FinF64),
}
