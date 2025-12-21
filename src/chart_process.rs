//! Chart Processor
//!
//! Unified Y coordinate definition:
//! - In the default 4/4 time signature, the length of "one measure" is 1.
//! - BMS: When the section length is the default value, each `Track` has a length of 1. The section length comes from the `#XXX02:V` message per measure, where `V` represents the multiple of the default length (e.g., `#00302:0.5` means the 3rd measure has half the default length). Cumulative y is linearly converted with this multiple.
//! - BMSON: `info.resolution` is the number of pulses corresponding to a quarter note (1/4), so one measure length is `4 * resolution` pulses; all position y is normalized to measure units through `pulses / (4 * resolution)`.
//! - Speed (default 1.0): Only affects display coordinates (e.g., `visible_notes` `distance_to_hit`), that is, scales the y difference proportionally; does not change time progression and BPM values, nor the actual duration of that measure.

use std::{collections::HashMap, ops::RangeBounds, path::Path};

use gametime::{TimeSpan, TimeStamp};

use crate::bms::prelude::SwBgaEvent;
use crate::bms::{
    Decimal,
    prelude::{Argb, BgaLayer, Key, NoteKind, PlayerSide},
};
use crate::chart_process::types::{
    BmpId, PlayheadEvent, VisibleChartEvent, VisibleRangePerBpm, WavId, YCoordinate,
};

pub mod bms_processor;
pub mod bmson_processor;

// Type definition module
pub mod types;

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
        /// Judge level (VeryHard, Hard, Normal, Easy, OtherInt)
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
    /// Formula: visible_y_range = current_bpm * visible_range_per_bpm
    /// This replaces the old SetDefaultVisibleYLength event.
    SetVisibleRangePerBpm {
        /// Visible range per BPM (y coordinate units per BPM, >0)
        visible_range_per_bpm: VisibleRangePerBpm,
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

/// Unified y unit description: In default 4/4 time, one measure equals 1; BMS uses `#SECLEN` for linear conversion, BMSON normalizes via `pulses / (4*resolution)`.
pub trait ChartProcessor {
    /// Read: audio file resources (id to path mapping).
    fn audio_files(&self) -> HashMap<WavId, &Path>;
    /// Read: BGA/BMP image resources (id to path mapping).
    fn bmp_files(&self) -> HashMap<BmpId, &Path>;

    /// Read: visible range per BPM (controls the relationship between BPM and visible Y range).
    fn visible_range_per_bpm(&self) -> &VisibleRangePerBpm;

    /// Read: current BPM (changes with events).
    fn current_bpm(&self) -> &Decimal;
    /// Read: current Speed factor (changes with events).
    fn current_speed(&self) -> &Decimal;
    /// Read: current Scroll factor (changes with events).
    fn current_scroll(&self) -> &Decimal;
    /// Read: current playback ratio (default 1).
    fn playback_ratio(&self) -> &Decimal;

    /// Notify: start playback, record starting absolute time.
    fn start_play(&mut self, now: TimeStamp);

    /// Get: playback start time.
    ///
    /// Returns `Some(Instant)` if `start_play` has been called and playback is active,
    /// otherwise returns `None`.
    fn started_at(&self) -> Option<TimeStamp>;

    /// Update: advance internal timeline, return timeline events generated since last call (Elm style).
    fn update(&mut self, now: TimeStamp) -> impl Iterator<Item = PlayheadEvent>;

    /// Query: events in a time window centered at current moment.
    ///
    /// The window is `range + now`, where `now` is the current playhead time since [`start_play`]
    /// (scaled by [`playback_ratio`]) and `range` is a `TimeSpan` offset range.
    fn events_in_time_range(
        &mut self,
        range: impl RangeBounds<TimeSpan>,
    ) -> impl Iterator<Item = PlayheadEvent>;

    /// Post external control events (such as setting default reaction time/default BPM), will be consumed before next `update`.
    ///
    /// These events are used to dynamically adjust player configuration parameters. Chart playback related events (such as notes, BGM, etc.)
    /// are returned by the [`update`] method, not posted through this method.
    fn post_events(&mut self, events: impl Iterator<Item = ControlEvent>);

    /// Query: all events in current visible area (preload logic).
    fn visible_events(&mut self, now: TimeStamp) -> impl Iterator<Item = VisibleChartEvent>;
}
