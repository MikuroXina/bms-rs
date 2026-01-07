//! Bmson Processor Module.
#![cfg(feature = "bmson")]

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use num::{One, ToPrimitive, Zero};

use crate::bms::prelude::{BgaLayer, Key, NoteKind, PlayerSide};
use crate::bmson::prelude::*;
use crate::chart_process::ChartEvent;
use crate::chart_process::types::{
    AllEventsIndex, BmpId, ChartEventIdGenerator, ChartResources, FlowEvent, ParsedChart,
    PlayheadEvent, TimeSpan, WavId, YCoordinate,
};
use crate::{bms::Decimal, util::StrExtension};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// BMSON format parser.
///
/// This struct serves as a namespace for BMSON parsing functions.
/// It parses BMSON files and returns a `ParsedChart` containing all precomputed data.
pub struct BmsonProcessor;

impl BmsonProcessor {
    /// Parse BMSON file and return a `ParsedChart` containing all precomputed data.
    #[must_use]
    pub fn parse(bmson: &Bmson<'_>) -> ParsedChart {
        let init_bpm: Decimal = bmson.info.init_bpm.as_f64().into();
        let pulses_denom = Decimal::from(4 * bmson.info.resolution.get());
        let pulses_to_y =
            |pulses: i64| YCoordinate::new(Decimal::from(pulses) / pulses_denom.clone());

        // Preprocessing: assign IDs to all audio and image resources
        let mut audio_name_to_id = HashMap::new();
        let mut bmp_name_to_id = HashMap::new();
        let mut next_audio_id = 0usize;
        let mut next_bmp_id = 0usize;

        // Process audio files
        for sound_channel in &bmson.sound_channels {
            let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(sound_channel.name.to_string())
            else {
                continue;
            };
            e.insert(WavId::new(next_audio_id));
            next_audio_id += 1;
        }

        // Process mine audio files
        for mine_channel in &bmson.mine_channels {
            let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(mine_channel.name.to_string())
            else {
                continue;
            };
            e.insert(WavId::new(next_audio_id));
            next_audio_id += 1;
        }

        // Process hidden key audio files
        for key_channel in &bmson.key_channels {
            let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(key_channel.name.to_string())
            else {
                continue;
            };
            e.insert(WavId::new(next_audio_id));
            next_audio_id += 1;
        }

        // Process image files
        for BgaHeader { name, .. } in &bmson.bga.bga_header {
            let std::collections::hash_map::Entry::Vacant(e) =
                bmp_name_to_id.entry(name.to_string())
            else {
                continue;
            };
            e.insert(BmpId::new(next_bmp_id));
            next_bmp_id += 1;
        }

        // Pre-index flow events by y for fast next_flow_event_after
        let mut flow_events_by_y: BTreeMap<YCoordinate, Vec<FlowEvent>> = BTreeMap::new();
        for ev in &bmson.bpm_events {
            let y = pulses_to_y(ev.y.0 as i64);
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Bpm(ev.bpm.as_f64().into()));
        }
        for ScrollEvent { y, rate } in &bmson.scroll_events {
            let y = pulses_to_y(y.0 as i64);
            flow_events_by_y
                .entry(y)
                .or_default()
                .push(FlowEvent::Scroll(rate.as_f64().into()));
        }

        let all_events =
            AllEventsIndex::precompute_events(bmson, &audio_name_to_id, &bmp_name_to_id);

        // Build resource maps
        let wav_files: HashMap<WavId, PathBuf> = audio_name_to_id
            .into_iter()
            .map(|(name, id)| (id, PathBuf::from(name)))
            .collect();
        let bmp_files: HashMap<BmpId, PathBuf> = bmp_name_to_id
            .into_iter()
            .map(|(name, id)| (id, PathBuf::from(name)))
            .collect();

        ParsedChart::new(
            ChartResources::new(wav_files, bmp_files),
            all_events,
            flow_events_by_y,
            init_bpm,
            Decimal::one(), // BMSON doesn't have Speed concept, default to 1.0
        )
    }
}

fn lane_from_x(mode_hint: &str, x: Option<std::num::NonZeroU8>) -> Option<(PlayerSide, Key)> {
    let lane_value = x?.get();

    if !mode_hint.starts_with_ignore_case("beat") {
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

impl AllEventsIndex {
    fn precompute_events<'a>(
        bmson: &Bmson<'a>,
        audio_name_to_id: &HashMap<String, WavId>,
        bmp_name_to_id: &HashMap<String, BmpId>,
    ) -> Self {
        use std::collections::BTreeSet;
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
        let nanos_for_stop = |stop_y: &YCoordinate, stop_pulses: u64| {
            let bpm_at_stop = bpm_map
                .range((
                    std::ops::Bound::Unbounded,
                    std::ops::Bound::Included(stop_y),
                ))
                .next_back()
                .map(|(_, b)| b.clone())
                .unwrap_or_else(|| init_bpm.clone());
            {
                let stop_y_len = pulses_to_y(stop_pulses);
                (stop_y_len.value() * Decimal::from(240u64) * Decimal::from(NANOS_PER_SECOND)
                    / bpm_at_stop)
                    .round()
                    .to_u64()
                    .unwrap_or(0)
            }
        };
        let mut stop_idx = 0usize;
        for curr in points.into_iter() {
            if curr <= prev {
                continue;
            }
            let delta_y = Decimal::from(&curr - &prev);
            let delta_nanos = (&delta_y * Decimal::from(240u64) * Decimal::from(NANOS_PER_SECOND)
                / cur_bpm)
                .round()
                .to_u64()
                .unwrap_or(0);
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
        let mut events_map: BTreeMap<YCoordinate, Vec<PlayheadEvent>> = BTreeMap::new();
        let to_time_span =
            |nanos: u64| TimeSpan::from_duration(std::time::Duration::from_nanos(nanos));
        let mut id_gen: ChartEventIdGenerator = ChartEventIdGenerator::default();
        for SoundChannel { name, notes } in &bmson.sound_channels {
            let mut last_restart_y = YCoordinate::zero();
            for Note { y, x, l, c, .. } in notes {
                let y_coord = pulses_to_y(y.0);
                let wav_id = audio_name_to_id.get(name.as_ref()).copied();
                if let Some((side, key)) = lane_from_x(bmson.info.mode_hint.as_ref(), *x) {
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
                    let event = ChartEvent::Note {
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
                    let event = ChartEvent::Bgm { wav_id };
                    let at = to_time_span(cum_map.get(&y_coord).copied().unwrap_or(0));
                    let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                    events_map.entry(y_coord).or_default().push(evp);
                }
            }
        }
        for ev in &bmson.bpm_events {
            let y = pulses_to_y(ev.y.0);
            let event = ChartEvent::BpmChange {
                bpm: ev.bpm.as_f64().into(),
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }
        for ScrollEvent { y, rate } in &bmson.scroll_events {
            let y = pulses_to_y(y.0);
            let event = ChartEvent::ScrollChange {
                factor: rate.as_f64().into(),
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }
        let mut id_to_bmp: HashMap<u32, Option<BmpId>> = HashMap::new();
        for BgaHeader { id, name } in &bmson.bga.bga_header {
            id_to_bmp.insert(id.0, bmp_name_to_id.get(name.as_ref()).copied());
        }
        for BgaEvent { y, id } in &bmson.bga.bga_events {
            let y = pulses_to_y(y.0);
            let bmp_id = id_to_bmp.get(&id.0).copied().flatten();
            let event = ChartEvent::BgaChange {
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
            let event = ChartEvent::BgaChange {
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
            let event = ChartEvent::BgaChange {
                layer: BgaLayer::Poor,
                bmp_id,
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }
        if let Some(lines) = &bmson.lines {
            for bar_line in lines {
                let y = pulses_to_y(bar_line.y.0);
                let event = ChartEvent::BarLine;
                let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
                let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
                events_map.entry(y).or_default().push(evp);
            }
        } else {
            let max_y = events_map
                .keys()
                .map(super::types::YCoordinate::value)
                .max()
                .cloned()
                .unwrap_or_else(Decimal::zero);
            if max_y > Decimal::zero() {
                let mut current_y = Decimal::zero();
                while current_y <= max_y {
                    let y_coord = YCoordinate::from(current_y.clone());
                    let event = ChartEvent::BarLine;
                    let at = to_time_span(cum_map.get(&y_coord).copied().unwrap_or(0));
                    let evp = PlayheadEvent::new(id_gen.next_id(), y_coord.clone(), event, at);
                    events_map.entry(y_coord).or_default().push(evp);
                    current_y += Decimal::one();
                }
            }
        }
        for stop in &bmson.stop_events {
            let y = pulses_to_y(stop.y.0);
            let event = ChartEvent::Stop {
                duration: (stop.duration as f64).into(),
            };
            let at = to_time_span(cum_map.get(&y).copied().unwrap_or(0));
            let evp = PlayheadEvent::new(id_gen.next_id(), y.clone(), event, at);
            events_map.entry(y).or_default().push(evp);
        }
        for MineChannel { name, notes } in &bmson.mine_channels {
            for MineEvent { x, y, .. } in notes {
                let y_coord = pulses_to_y(y.0);
                let Some((side, key)) = lane_from_x(bmson.info.mode_hint.as_ref(), *x) else {
                    continue;
                };
                let wav_id = audio_name_to_id.get(name.as_ref()).copied();
                let event = ChartEvent::Note {
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
        for KeyChannel { name, notes } in &bmson.key_channels {
            for KeyEvent { x, y } in notes {
                let y_coord = pulses_to_y(y.0);
                let Some((side, key)) = lane_from_x(bmson.info.mode_hint.as_ref(), *x) else {
                    continue;
                };
                let wav_id = audio_name_to_id.get(name.as_ref()).copied();
                let event = ChartEvent::Note {
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
        Self::new(events_map)
    }
}
