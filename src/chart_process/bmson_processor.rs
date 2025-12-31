//! Bmson Processor Module.
#![cfg(feature = "bmson")]

use std::collections::{BTreeMap, HashMap};

use gametime::TimeSpan;
use num::{One, ToPrimitive, Zero};

use crate::bms::prelude::*;
use crate::bmson::prelude::*;
use crate::chart_process::base_bpm::VisibleRangePerBpm;
use crate::chart_process::player::UniversalChartPlayer;
use crate::chart_process::resource::{BmpId, NameBasedResourceMapping, WavId};
use crate::chart_process::{AllEventsIndex, ChartEventIdGenerator, PlayheadEvent, YCoordinate};
use crate::util::StrExtension;

use super::{EventParseOutput, FlowEvent};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// `ChartProcessor` of Bmson files.
///
/// This processor parses BMSON charts and produces an `EventParseOutput`.
/// Use the `to_player()` method to convert the parse output into a playable chart.
pub struct BmsonProcessor<'a> {
    /// Phantom data for lifetime
    _phantom: std::marker::PhantomData<&'a ()>,

    /// Parsed chart output
    output: EventParseOutput<NameBasedResourceMapping>,
}

impl<'a> BmsonProcessor<'a> {
    /// Create BMSON processor by parsing BMSON chart.
    #[must_use]
    pub fn new(bmson: &Bmson<'a>) -> Self {
        let init_bpm: Decimal = bmson.info.init_bpm.as_f64().into();

        // Preprocess: assign IDs to all audio and image resources
        let mut audio_name_to_id = HashMap::new();
        let mut bmp_name_to_id = HashMap::new();
        let mut next_audio_id = 0usize;
        let mut next_bmp_id = 0usize;

        // Process audio files
        for sound_channel in &bmson.sound_channels {
            if let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(sound_channel.name.to_string())
            {
                e.insert(WavId::new(next_audio_id));
                next_audio_id += 1;
            }
        }

        // Process mine audio files
        for mine_channel in &bmson.mine_channels {
            if let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(mine_channel.name.to_string())
            {
                e.insert(WavId::new(next_audio_id));
                next_audio_id += 1;
            }
        }

        // Process hidden key audio files
        for key_channel in &bmson.key_channels {
            if let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(key_channel.name.to_string())
            {
                e.insert(WavId::new(next_audio_id));
                next_audio_id += 1;
            }
        }

        // Process image files
        for BgaHeader { name, .. } in &bmson.bga.bga_header {
            if let std::collections::hash_map::Entry::Vacant(e) =
                bmp_name_to_id.entry(name.to_string())
            {
                e.insert(BmpId::new(next_bmp_id));
                next_bmp_id += 1;
            }
        }

        // Precompute all events
        let all_events = Self::precompute_events(bmson, &audio_name_to_id, &bmp_name_to_id);

        // Build flow events mapping
        let flow_events_by_y = Self::build_flow_events(bmson);

        // Build resource mapping
        let resources = NameBasedResourceMapping::new(audio_name_to_id, bmp_name_to_id);

        let output = EventParseOutput {
            all_events,
            flow_events_by_y,
            init_bpm,
            resources,
        };

        Self {
            _phantom: std::marker::PhantomData,
            output,
        }
    }

    /// Convert the parse output into a playable chart.
    ///
    /// # Arguments
    /// * `visible_range_per_bpm` - Visible range configuration for playback
    #[must_use]
    pub fn to_player(
        self,
        visible_range_per_bpm: VisibleRangePerBpm,
    ) -> UniversalChartPlayer<NameBasedResourceMapping> {
        UniversalChartPlayer::from_parse_output(self.output, visible_range_per_bpm)
    }

    /// Get access to all parsed events.
    #[must_use]
    pub const fn all_events(&self) -> &AllEventsIndex {
        &self.output.all_events
    }

    /// Get the initial BPM.
    #[must_use]
    pub const fn init_bpm(&self) -> &Decimal {
        &self.output.init_bpm
    }

    /// Get access to the resource mapping.
    #[must_use]
    pub const fn resources(&self) -> &NameBasedResourceMapping {
        &self.output.resources
    }

    /// Precompute all events from BMSON chart.
    fn precompute_events(
        bmson: &Bmson<'a>,
        audio_name_to_id: &HashMap<String, WavId>,
        bmp_name_to_id: &HashMap<String, BmpId>,
    ) -> AllEventsIndex {
        use std::collections::BTreeSet;
        use std::time::Duration;

        let denom = Decimal::from(4 * bmson.info.resolution.get());
        let denom_inv = if denom == Decimal::zero() {
            Decimal::zero()
        } else {
            Decimal::one() / denom
        };
        let pulses_to_y = |pulses: u64| {
            let pulses = Decimal::from(pulses);
            YCoordinate::new(&pulses * &denom_inv)
        };

        // Collect all Y points
        let mut points: BTreeSet<YCoordinate> = BTreeSet::new();
        points.insert(YCoordinate::zero());

        for SoundChannel { notes, .. } in &bmson.sound_channels {
            for Note { y, .. } in notes {
                points.insert(pulses_to_y(y.0));
            }
        }

        for MineChannel { notes, .. } in &bmson.mine_channels {
            for MineEvent { y, .. } in notes {
                points.insert(pulses_to_y(y.0));
            }
        }

        for KeyChannel { notes, .. } in &bmson.key_channels {
            for KeyEvent { y, .. } in notes {
                points.insert(pulses_to_y(y.0));
            }
        }

        for ev in &bmson.bpm_events {
            points.insert(pulses_to_y(ev.y.0));
        }

        for ScrollEvent { y, .. } in &bmson.scroll_events {
            points.insert(pulses_to_y(y.0));
        }

        for stop in &bmson.stop_events {
            points.insert(pulses_to_y(stop.y.0));
        }

        for BgaEvent { y, .. } in &bmson.bga.bga_events {
            points.insert(pulses_to_y(y.0));
        }

        for BgaEvent { y, .. } in &bmson.bga.layer_events {
            points.insert(pulses_to_y(y.0));
        }

        for BgaEvent { y, .. } in &bmson.bga.poor_events {
            points.insert(pulses_to_y(y.0));
        }

        if let Some(lines) = &bmson.lines {
            for bar_line in lines {
                points.insert(pulses_to_y(bar_line.y.0));
            }
        } else {
            let max_y = points
                .iter()
                .cloned()
                .max()
                .unwrap_or_else(YCoordinate::zero);
            let floor = max_y.value().to_i64().unwrap_or(0);
            for i in 0..=floor {
                points.insert(YCoordinate::new(Decimal::from(i)));
            }
        }

        // Build BPM map and calculate cumulative times
        let init_bpm: Decimal = bmson.info.init_bpm.as_f64().into();
        let mut bpm_map: BTreeMap<YCoordinate, Decimal> = BTreeMap::new();
        bpm_map.insert(YCoordinate::zero(), init_bpm.clone());

        for ev in &bmson.bpm_events {
            bpm_map.insert(pulses_to_y(ev.y.0), ev.bpm.as_f64().into());
        }

        let mut stop_list: Vec<(YCoordinate, u64)> = bmson
            .stop_events
            .iter()
            .map(|st| (pulses_to_y(st.y.0), st.duration))
            .collect();
        stop_list.sort_by(|a, b| a.0.cmp(&b.0));

        let mut cum_map: BTreeMap<YCoordinate, u64> = BTreeMap::new();
        let mut total_nanos: u64 = 0;
        let mut prev = YCoordinate::zero();
        cum_map.insert(prev.clone(), 0);

        let mut cur_bpm = bpm_map
            .range((std::ops::Bound::Unbounded, std::ops::Bound::Included(&prev)))
            .next_back()
            .map(|(_, b)| b.clone())
            .unwrap_or_else(|| init_bpm.clone());

        let nanos_for_stop = |stop_y: &YCoordinate, stop_pulses: u64| -> u64 {
            let bpm_at_stop = bpm_map
                .range((
                    std::ops::Bound::Unbounded,
                    std::ops::Bound::Included(stop_y),
                ))
                .next_back()
                .map(|(_, b)| b.clone())
                .unwrap_or_else(|| init_bpm.clone());

            if bpm_at_stop > Decimal::zero() {
                let stop_y_len = pulses_to_y(stop_pulses);
                let numerator =
                    stop_y_len.value() * Decimal::from(240u64) * Decimal::from(NANOS_PER_SECOND);
                (numerator / bpm_at_stop).round().to_u64().unwrap_or(0)
            } else {
                // BPM <= 0 is invalid; stop duration contributes no time
                0
            }
        };

        let mut stop_idx = 0usize;

        for curr in points {
            if curr <= prev {
                continue;
            }

            let delta_y = Decimal::from(&curr - &prev);
            let delta_nanos = if cur_bpm > Decimal::zero() {
                let numerator = delta_y * Decimal::from(240u64) * Decimal::from(NANOS_PER_SECOND);
                (numerator / cur_bpm).round().to_u64().unwrap_or(0)
            } else {
                // BPM <= 0 is invalid; treat as no time progression to avoid division issues
                0
            };

            total_nanos = total_nanos.saturating_add(delta_nanos);

            while let Some((sy, stop_pulses)) = stop_list.get(stop_idx) {
                if sy > &curr {
                    break;
                }
                if sy > &prev {
                    total_nanos = total_nanos.saturating_add(nanos_for_stop(sy, *stop_pulses));
                }
                stop_idx += 1;
            }

            cur_bpm = bpm_map
                .range((std::ops::Bound::Unbounded, std::ops::Bound::Included(&curr)))
                .next_back()
                .map(|(_, b)| b.clone())
                .unwrap_or_else(|| init_bpm.clone());

            cum_map.insert(curr.clone(), total_nanos);
            prev = curr;
        }

        // Build events map with activate times
        let mut events_map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        let to_time_span = |nanos: u64| TimeSpan::from_duration(Duration::from_nanos(nanos));
        let mut id_gen: ChartEventIdGenerator = ChartEventIdGenerator::default();

        // Sound channel notes
        for SoundChannel { name, notes } in &bmson.sound_channels {
            let mut last_restart_y = YCoordinate::zero();
            for Note { y, x, l, c, .. } in notes {
                let y_coord = pulses_to_y(y.0);
                let wav_id = audio_name_to_id.get(name.as_ref()).copied();

                if let Some((side, key)) =
                    Self::lane_from_x(Some(bmson.info.mode_hint.as_ref()), *x)
                {
                    let length = (*l > 0).then(|| {
                        let end_y = pulses_to_y(y.0 + l);
                        &end_y - &y_coord
                    });
                    let kind = if *l > 0 {
                        NoteKind::Long
                    } else {
                        NoteKind::Visible
                    };
                    let continue_play = c.then(|| {
                        let to = cum_map.get(&y_coord).copied().unwrap_or(0);
                        let from = cum_map.get(&last_restart_y).copied().unwrap_or(0);
                        to_time_span(to.saturating_sub(from))
                    });

                    let event = crate::chart_process::ChartEvent::Note {
                        side,
                        key,
                        kind,
                        wav_id,
                        length,
                        continue_play,
                    };

                    let at = to_time_span(cum_map.get(&y_coord).copied().unwrap_or(0));
                    let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);

                    if !*c {
                        last_restart_y = y_coord.clone();
                    }

                    events_map.entry(y_coord).or_default().push(evp);
                } else {
                    let event = crate::chart_process::ChartEvent::Bgm { wav_id };
                    let at = to_time_span(cum_map.get(&y_coord).copied().unwrap_or(0));
                    let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                    events_map.entry(y_coord).or_default().push(evp);
                }
            }
        }

        // BPM events
        for ev in &bmson.bpm_events {
            let y = pulses_to_y(ev.y.0);
            let event = crate::chart_process::ChartEvent::BpmChange {
                bpm: ev.bpm.as_f64().into(),
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }

        // Scroll events
        for ScrollEvent { y, rate } in &bmson.scroll_events {
            let y = pulses_to_y(y.0);
            let event = crate::chart_process::ChartEvent::ScrollChange {
                factor: rate.as_f64().into(),
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }

        // BGA events
        let mut id_to_bmp: HashMap<u32, Option<BmpId>> = HashMap::new();
        for BgaHeader { id, name } in &bmson.bga.bga_header {
            id_to_bmp.insert(id.0, bmp_name_to_id.get(name.as_ref()).copied());
        }

        for BgaEvent { y, id } in &bmson.bga.bga_events {
            let y = pulses_to_y(y.0);
            let bmp_id = id_to_bmp.get(&id.0).copied().flatten();
            let event = crate::chart_process::ChartEvent::BgaChange {
                layer: BgaLayer::Base,
                bmp_id,
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }

        for BgaEvent { y, id } in &bmson.bga.layer_events {
            let y = pulses_to_y(y.0);
            let bmp_id = id_to_bmp.get(&id.0).copied().flatten();
            let event = crate::chart_process::ChartEvent::BgaChange {
                layer: BgaLayer::Overlay,
                bmp_id,
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }

        for BgaEvent { y, id } in &bmson.bga.poor_events {
            let y = pulses_to_y(y.0);
            let bmp_id = id_to_bmp.get(&id.0).copied().flatten();
            let event = crate::chart_process::ChartEvent::BgaChange {
                layer: BgaLayer::Poor,
                bmp_id,
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }

        // Bar lines
        if let Some(lines) = &bmson.lines {
            for bar_line in lines {
                let y = pulses_to_y(bar_line.y.0);
                let event = crate::chart_process::ChartEvent::BarLine;
                let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
                let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
                events_map.entry(y).or_default().push(evp);
            }
        } else {
            let max_y = events_map
                .keys()
                .map(super::YCoordinate::value)
                .max()
                .cloned()
                .unwrap_or_else(Decimal::zero);

            if max_y > Decimal::zero() {
                let mut current_y = Decimal::zero();
                while current_y <= max_y {
                    let y_coord = YCoordinate::from(current_y.clone());
                    let event = crate::chart_process::ChartEvent::BarLine;
                    let at = to_time_span(cum_map.get(&y_coord).copied().unwrap_or(0));
                    let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                    events_map.entry(y_coord).or_default().push(evp);
                    current_y += Decimal::one();
                }
            }
        }

        // Stop events
        for stop in &bmson.stop_events {
            let y = pulses_to_y(stop.y.0);
            let event = crate::chart_process::ChartEvent::Stop {
                duration: Decimal::from(stop.duration),
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }

        // Mine channel notes
        for MineChannel { name, notes } in &bmson.mine_channels {
            for MineEvent { x, y, .. } in notes {
                let y_coord = pulses_to_y(y.0);
                let Some((side, key)) = Self::lane_from_x(Some(bmson.info.mode_hint.as_ref()), *x)
                else {
                    continue;
                };
                let wav_id = audio_name_to_id.get(name.as_ref()).copied();
                let event = crate::chart_process::ChartEvent::Note {
                    side,
                    key,
                    kind: NoteKind::Landmine,
                    wav_id,
                    length: None,
                    continue_play: None,
                };
                let at = to_time_span(cum_map.get(&y_coord).copied().unwrap_or(0));
                let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                events_map.entry(y_coord).or_default().push(evp);
            }
        }

        // Key channel notes
        for KeyChannel { name, notes } in &bmson.key_channels {
            for KeyEvent { x, y } in notes {
                let y_coord = pulses_to_y(y.0);
                let Some((side, key)) = Self::lane_from_x(Some(bmson.info.mode_hint.as_ref()), *x)
                else {
                    continue;
                };
                let wav_id = audio_name_to_id.get(name.as_ref()).copied();
                let event = crate::chart_process::ChartEvent::Note {
                    side,
                    key,
                    kind: NoteKind::Invisible,
                    wav_id,
                    length: None,
                    continue_play: None,
                };
                let at = to_time_span(cum_map.get(&y_coord).copied().unwrap_or(0));
                let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                events_map.entry(y_coord).or_default().push(evp);
            }
        }

        AllEventsIndex::new(events_map)
    }

    /// Build flow events mapping by Y coordinate.
    fn build_flow_events(bmson: &Bmson<'a>) -> BTreeMap<YCoordinate, Vec<FlowEvent>> {
        let mut flow_events_by_y: BTreeMap<YCoordinate, Vec<FlowEvent>> = BTreeMap::new();

        let pulses_denom = Decimal::from(4 * bmson.info.resolution.get());
        let pulses_to_y =
            |pulses: i64| YCoordinate::new(Decimal::from(pulses) / pulses_denom.clone());

        for ev in &bmson.bpm_events {
            let y = pulses_to_y(ev.y.0 as i64);
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Bpm(ev.bpm.as_f64().into()));
        }

        for ScrollEvent { y, rate } in &bmson.scroll_events {
            let y_coord = pulses_to_y(y.0 as i64);
            flow_events_by_y
                .entry(y_coord)
                .or_default()
                .push(FlowEvent::Scroll(rate.as_f64().into()));
        }

        flow_events_by_y
    }

    /// Helper function to determine lane from x coordinate in BMSON.
    fn lane_from_x(
        mode_hint: Option<&str>,
        x: Option<std::num::NonZeroU8>,
    ) -> Option<(PlayerSide, Key)> {
        let lane_value = x?.get();

        if !mode_hint
            .map(|hint| hint.starts_with_ignore_case("beat"))
            .unwrap_or(false)
        {
            return Some((PlayerSide::Player1, Key::Key(lane_value)));
        }

        let (adjusted_lane, side) = if lane_value > 8 {
            (lane_value - 8, PlayerSide::Player2)
        } else {
            (lane_value, PlayerSide::Player1)
        };

        let key = match adjusted_lane {
            1..=7 => Key::Key(adjusted_lane),
            8 => Key::Scratch(1),
            _ => return None,
        };

        Some((side, key))
    }
}
