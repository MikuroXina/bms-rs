//! Bmson Processor Module.

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::bms::prelude::*;
use crate::bmson::{Bmson, Note, ScrollEvent, SoundChannel};
use crate::chart_process::{
    BmpId, ChartEvent, ChartProcessor, ControlEvent, NoteView, WavId, YCoordinate,
};
use num::ToPrimitive;

/// ChartProcessor of Bmson files.
pub struct BmsonProcessor<'a> {
    bmson: Bmson<'a>,

    // Playback state
    started_at: Option<SystemTime>,
    last_poll_at: Option<SystemTime>,
    progressed_y: f64,

    // Flow parameters
    default_reaction_time: Duration,
    default_bpm_bound: f64,
    current_bpm: f64,
    current_speed: f64,
    current_scroll: f64,

    /// 待消费的外部事件队列
    inbox: Vec<ControlEvent>,
}

impl<'a> BmsonProcessor<'a> {
    /// 创建 BMSON 处理器并初始化播放状态与默认参数。
    #[must_use]
    pub fn new(bmson: Bmson<'a>) -> Self {
        let init_bpm = bmson.info.init_bpm.as_f64();
        Self {
            bmson,
            started_at: None,
            last_poll_at: None,
            progressed_y: 0.0,
            inbox: Vec::new(),
            default_reaction_time: Duration::from_millis(500),
            default_bpm_bound: init_bpm,
            current_bpm: init_bpm,
            current_speed: 1.0,
            current_scroll: 1.0,
        }
    }

    /// 将脉冲数转换为统一的 y 坐标（单位：小节）。一小节 = 4*resolution 脉冲。
    fn pulses_to_y(&self, pulses: u64) -> f64 {
        let denom = (4 * self.bmson.info.resolution.get()) as f64;
        if denom > 0.0 {
            (pulses as f64) / denom
        } else {
            0.0
        }
    }

    /// 当前瞬时位移速度（y 单位每秒）。
    /// y 为归一化后的小节单位：`y = pulses / (4*resolution)`，默认 4/4 下一小节为 1。
    /// 模型：v = (current_bpm / default_bpm_bound)
    /// 注：Speed/Scroll 仅影响显示位置（y 缩放），不改变时间轴推进。
    fn current_velocity(&self) -> f64 {
        if self.default_bpm_bound <= 0.0 {
            return 0.0;
        }
        self.current_bpm / self.default_bpm_bound
    }

    /// 取下一条会影响速度的事件（按 y 升序）：BPM/SCROLL。
    fn next_flow_event_after(&self, y_from_exclusive: f64) -> Option<(f64, FlowEvent)> {
        let mut best: Option<(f64, FlowEvent)> = None;

        for ev in &self.bmson.bpm_events {
            let y = self.pulses_to_y(ev.y.0);
            if y > y_from_exclusive {
                best = min_by_y(best, (y, FlowEvent::Bpm(ev.bpm.as_f64())));
            }
        }
        for ScrollEvent { y, rate } in &self.bmson.scroll_events {
            let y = self.pulses_to_y(y.0);
            if y > y_from_exclusive {
                best = min_by_y(best, (y, FlowEvent::Scroll(rate.as_f64())));
            }
        }
        best
    }

    fn step_to(&mut self, now: SystemTime) {
        let Some(started) = self.started_at else {
            return;
        };
        let last = self.last_poll_at.unwrap_or(started);
        if now <= last {
            return;
        }

        let mut remaining = now.duration_since(last).unwrap_or_default();
        let mut cur_vel = self.current_velocity();
        let mut cur_y = self.progressed_y;
        loop {
            let next_event = self.next_flow_event_after(cur_y);
            if next_event.is_none() || cur_vel <= 0.0 || remaining.is_zero() {
                cur_y += cur_vel * remaining.as_secs_f64();
                break;
            }
            let (event_y, evt) = next_event.unwrap();
            if event_y <= cur_y {
                self.apply_flow_event(evt);
                cur_vel = self.current_velocity();
                continue;
            }
            let distance = event_y - cur_y;
            if cur_vel > 0.0 {
                let time_to_event = Duration::from_secs_f64(distance / cur_vel);
                if time_to_event <= remaining {
                    cur_y = event_y;
                    remaining -= time_to_event;
                    self.apply_flow_event(evt);
                    cur_vel = self.current_velocity();
                    continue;
                }
            }
            cur_y += cur_vel * remaining.as_secs_f64();
            break;
        }

        self.progressed_y = cur_y;
        self.last_poll_at = Some(now);
    }

    fn apply_flow_event(&mut self, evt: FlowEvent) {
        match evt {
            FlowEvent::Bpm(bpm) => self.current_bpm = bpm,
            FlowEvent::Scroll(s) => self.current_scroll = s,
        }
    }

    fn visible_window_y(&self) -> f64 {
        let v = self.current_velocity();
        v * self.default_reaction_time.as_secs_f64()
    }

    fn lane_from_x(x: Option<std::num::NonZeroU8>) -> Option<(PlayerSide, Key)> {
        let lane_value = x.map_or(0, |l| l.get());
        let (adjusted_lane, side) = if lane_value > 8 {
            (lane_value - 8, PlayerSide::Player2)
        } else {
            (lane_value, PlayerSide::Player1)
        };
        let key = match adjusted_lane {
            1 => Key::Key(1),
            2 => Key::Key(2),
            3 => Key::Key(3),
            4 => Key::Key(4),
            5 => Key::Key(5),
            6 => Key::Key(6),
            7 => Key::Key(7),
            8 => Key::Scratch(1),
            _ => return None,
        };
        Some((side, key))
    }
}

impl<'a> ChartProcessor for BmsonProcessor<'a> {
    fn audio_files(&self) -> HashMap<WavId, &Path> {
        // bmson 里资源在 channel.name 中，无法映射为索引表；这里返回空表。
        HashMap::new()
    }

    fn bmp_files(&self) -> HashMap<BmpId, &Path> {
        HashMap::new()
    }

    fn default_reaction_time(&self) -> Duration {
        self.default_reaction_time
    }
    fn default_bpm_bound(&self) -> f64 {
        self.default_bpm_bound
    }

    fn current_bpm(&self) -> f64 {
        self.current_bpm
    }
    fn current_speed(&self) -> f64 {
        self.current_speed
    }
    fn current_scroll(&self) -> f64 {
        self.current_scroll
    }

    fn start_play(&mut self, now: SystemTime) {
        self.started_at = Some(now);
        self.last_poll_at = Some(now);
        self.progressed_y = 0.0;
        self.current_bpm = self.bmson.info.init_bpm.as_f64();
    }

    fn update(&mut self, now: SystemTime) -> Vec<(YCoordinate, ChartEvent)> {
        let incoming = std::mem::take(&mut self.inbox);
        for evt in &incoming {
            match evt {
                ControlEvent::SetDefaultReactionTime { seconds } => {
                    if let Some(seconds_f64) = seconds.to_f64()
                        && seconds_f64.is_finite()
                        && seconds_f64 > 0.0
                    {
                        self.default_reaction_time = Duration::from_secs_f64(seconds_f64);
                    }
                }
                ControlEvent::SetDefaultBpmBound { bpm } => {
                    if let Some(bpm_f64) = bpm.to_f64()
                        && bpm_f64.is_finite()
                        && bpm_f64 > 0.0
                    {
                        self.default_bpm_bound = bpm_f64;
                    }
                }
            }
        }

        let prev_y = self.progressed_y;
        self.step_to(now);
        let cur_y = self.progressed_y;

        let mut events: Vec<(YCoordinate, ChartEvent)> = Vec::new();
        for SoundChannel { name: _, notes } in &self.bmson.sound_channels {
            for Note { y, x, .. } in notes {
                let yy = self.pulses_to_y(y.0);
                if yy > prev_y && yy <= cur_y {
                    if let Some((side, key)) = Self::lane_from_x(x.as_ref().copied()) {
                        events.push((
                            yy.into(),
                            ChartEvent::Note {
                                side,
                                key,
                                kind: NoteKind::Visible,
                                wav_id: None,
                            },
                        ));
                    } else {
                        events.push((yy.into(), ChartEvent::Bgm { wav_id: None }));
                    }
                }
            }
        }

        for ev in &self.bmson.bpm_events {
            let y = self.pulses_to_y(ev.y.0);
            if y > prev_y && y <= cur_y {
                events.push((
                    y.into(),
                    ChartEvent::BpmChange {
                        bpm: ev.bpm.as_f64().into(),
                    },
                ));
            }
        }
        for ScrollEvent { y, rate } in &self.bmson.scroll_events {
            let y = self.pulses_to_y(y.0);
            if y > prev_y && y <= cur_y {
                events.push((
                    y.into(),
                    ChartEvent::ScrollChange {
                        factor: rate.as_f64().into(),
                    },
                ));
            }
        }
        for stop in &self.bmson.stop_events {
            let y = self.pulses_to_y(stop.y.0);
            if y > prev_y && y <= cur_y {
                events.push((
                    y.into(),
                    ChartEvent::Stop {
                        duration: (stop.duration as f64).into(),
                    },
                ));
            }
        }

        // BGA 基础层事件
        for bga_event in &self.bmson.bga.bga_events {
            let y = self.pulses_to_y(bga_event.y.0);
            if y > prev_y && y <= cur_y {
                events.push((
                    y.into(),
                    ChartEvent::BgaChange {
                        layer: BgaLayer::Base,
                        bmp_id: Some(BmpId::from(bga_event.id.0 as usize)),
                    },
                ));
            }
        }

        // BGA 覆盖层事件
        for layer_event in &self.bmson.bga.layer_events {
            let y = self.pulses_to_y(layer_event.y.0);
            if y > prev_y && y <= cur_y {
                events.push((
                    y.into(),
                    ChartEvent::BgaChange {
                        layer: BgaLayer::Overlay,
                        bmp_id: Some(BmpId::from(layer_event.id.0 as usize)),
                    },
                ));
            }
        }

        // BGA 失败层事件
        for poor_event in &self.bmson.bga.poor_events {
            let y = self.pulses_to_y(poor_event.y.0);
            if y > prev_y && y <= cur_y {
                events.push((
                    y.into(),
                    ChartEvent::BgaChange {
                        layer: BgaLayer::Poor,
                        bmp_id: Some(BmpId::from(poor_event.id.0 as usize)),
                    },
                ));
            }
        }

        events.sort_by(|a, b| {
            a.0.value()
                .partial_cmp(b.0.value())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        events
    }

    fn post_events(&mut self, events: &[ControlEvent]) {
        self.inbox.extend_from_slice(events);
    }

    fn visible_notes(&mut self, now: SystemTime) -> Vec<NoteView> {
        self.step_to(now);
        let win_y = self.visible_window_y();
        let cur_y = self.progressed_y;
        let scaled_upper = self.current_scroll * self.current_speed * win_y;
        let (min_scaled, max_scaled) = if scaled_upper >= 0.0 {
            (0.0, scaled_upper)
        } else {
            (scaled_upper, 0.0)
        };

        let mut out: Vec<(f64, NoteView)> = Vec::new();
        for SoundChannel { name: _, notes } in &self.bmson.sound_channels {
            for Note { y, x, .. } in notes {
                let yy = self.pulses_to_y(y.0);
                let raw_distance = yy - cur_y;
                let scaled_distance = self.current_scroll * self.current_speed * raw_distance;
                if scaled_distance >= min_scaled
                    && scaled_distance <= max_scaled
                    && let Some((side, key)) = Self::lane_from_x(x.as_ref().copied())
                {
                    out.push((
                        yy,
                        NoteView {
                            side,
                            key,
                            distance_to_hit: scaled_distance.into(),
                            wav_id: None,
                        },
                    ));
                }
            }
        }
        out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        out.into_iter().map(|(_, v)| v).collect()
    }
}

#[derive(Debug, Clone, Copy)]
enum FlowEvent {
    Bpm(f64),
    Scroll(f64),
}

fn min_by_y(
    best: Option<(f64, FlowEvent)>,
    candidate: (f64, FlowEvent),
) -> Option<(f64, FlowEvent)> {
    match best {
        None => Some(candidate),
        Some((y, _)) if candidate.0 < y => Some(candidate),
        Some(o) => Some(o),
    }
}
