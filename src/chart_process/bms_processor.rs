//! Bms Processor Module.

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::bms::prelude::*;
use crate::chart_process::{ChartEvent, ChartProcessor, ControlEvent, NoteView, YCoordinate};
use num::ToPrimitive;

#[inline]
fn dec_to_f64(d: &Decimal) -> Option<f64> {
    <Decimal as ToPrimitive>::to_f64(d)
}

/// ChartProcessor of Bms files.
pub struct BmsProcessor<T = KeyLayoutBeat>
where
    T: KeyLayoutMapper,
{
    bms: Bms<T>,

    // Playback state
    started_at: Option<SystemTime>,
    last_poll_at: Option<SystemTime>,
    /// 已经推进的累计位移（y，实际移动距离单位）
    progressed_y: f64,

    /// 待消费的外部事件队列
    inbox: Vec<ControlEvent>,

    // Flow parameters
    default_reaction_time: Duration,
    default_bpm_bound: f64,
    current_bpm: f64,
    current_speed: f64,
    current_scroll: f64,
}

impl<T> BmsProcessor<T>
where
    T: KeyLayoutMapper,
{
    /// 创建处理器，初始化默认参数
    #[must_use]
    pub fn new(bms: Bms<T>) -> Self {
        // 初始化 BPM：优先使用谱面初始 BPM，否则 120
        let init_bpm = bms
            .arrangers
            .bpm
            .as_ref()
            .and_then(dec_to_f64)
            .unwrap_or(120.0);
        Self {
            bms,
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

    /// 获取指定 Track 的长度（SectionLength），默认 1.0
    fn section_length_of(&self, track: Track) -> f64 {
        self.bms
            .arrangers
            .section_len_changes
            .get(&track)
            .and_then(|s| dec_to_f64(&s.length))
            .unwrap_or(1.0)
    }

    /// 将 `ObjTime` 转换为累计位移 y（单位：小节，默认 4/4 下一小节为 1；按 `#SECLEN` 线性换算）。
    fn y_of_time(&self, time: ObjTime) -> f64 {
        let mut y = 0.0f64;
        // 累加完整小节
        for t in 0..time.track().0 {
            y += self.section_length_of(Track(t));
        }
        // 当前小节内按比例累加
        let current_len = self.section_length_of(time.track());
        if time.denominator().get() > 0 {
            y += current_len * (time.numerator() as f64) / (time.denominator().get() as f64);
        }
        y
    }

    /// 当前瞬时位移速度（y 单位每秒）。
    /// 模型：v = (current_bpm / default_bpm_bound)
    /// 注：Speed 仅影响显示位置（y 缩放），不改变时间轴推进；Scroll 同理仅影响显示。
    fn current_velocity(&self) -> f64 {
        if self.default_bpm_bound <= 0.0 {
            return 0.0;
        }
        self.current_bpm / self.default_bpm_bound
    }

    /// 取下一条会影响速度的事件（按 y 升序）：BPM/SCROLL/SPEED 变更。
    fn next_flow_event_after(&self, y_from_exclusive: f64) -> Option<(f64, FlowEvent)> {
        // 收集三个事件源，找 y 大于阈值的最小项
        let mut best: Option<(f64, FlowEvent)> = None;

        for change in self.bms.arrangers.bpm_changes.values() {
            let y = self.y_of_time(change.time);
            if y > y_from_exclusive {
                let bpm = dec_to_f64(&change.bpm).unwrap_or(self.current_bpm);
                best = min_by_y(best, (y, FlowEvent::Bpm(bpm)));
            }
        }
        for change in self.bms.arrangers.scrolling_factor_changes.values() {
            let y = self.y_of_time(change.time);
            if y > y_from_exclusive {
                let factor = dec_to_f64(&change.factor).unwrap_or(self.current_scroll);
                best = min_by_y(best, (y, FlowEvent::Scroll(factor)));
            }
        }
        for change in self.bms.arrangers.speed_factor_changes.values() {
            let y = self.y_of_time(change.time);
            if y > y_from_exclusive {
                let factor = dec_to_f64(&change.factor).unwrap_or(self.current_speed);
                best = min_by_y(best, (y, FlowEvent::Speed(factor)));
            }
        }

        best
    }

    /// 将时间推进到 `now`，对进度与速度进行按事件分段积分。
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
        // 分段推进，直到用完时间片
        loop {
            // 下一个会影响速度的事件
            let next_event = self.next_flow_event_after(cur_y);
            if next_event.is_none() || cur_vel <= 0.0 || remaining.is_zero() {
                // 直接前进到结尾
                cur_y += cur_vel * (remaining.as_secs_f64());
                break;
            }
            let (event_y, evt) = next_event.unwrap();
            if event_y <= cur_y {
                // 防御：若事件位置不前进则避免死循环
                self.apply_flow_event(evt);
                cur_vel = self.current_velocity();
                continue;
            }
            // 到达事件所需时间
            let distance = event_y - cur_y;
            if cur_vel > 0.0 {
                let time_to_event = Duration::from_secs_f64(distance / cur_vel);
                if time_to_event <= remaining {
                    // 先推进到事件点
                    cur_y = event_y;
                    remaining -= time_to_event;
                    self.apply_flow_event(evt);
                    cur_vel = self.current_velocity();
                    continue;
                }
            }
            // 时间不够到事件，推进后结束
            cur_y += cur_vel * remaining.as_secs_f64();
            break;
        }

        self.progressed_y = cur_y;
        self.last_poll_at = Some(now);
    }

    fn apply_flow_event(&mut self, evt: FlowEvent) {
        match evt {
            FlowEvent::Bpm(bpm) => self.current_bpm = bpm,
            FlowEvent::Speed(s) => self.current_speed = s,
            FlowEvent::Scroll(s) => self.current_scroll = s,
        }
    }

    /// 计算可见窗口长度（y 单位）：基于默认反应时间与当前速度
    fn visible_window_y(&self) -> f64 {
        // 以当前瞬时速度计算窗口对应位移
        let v = self.current_velocity();
        v * self.default_reaction_time.as_secs_f64()
    }

    fn lane_of_channel_id(channel_id: NoteChannelId) -> Option<(PlayerSide, Key, NoteKind)> {
        if let Some(map) = channel_id.try_into_map::<T>() {
            let side = map.side();
            let key = map.key();
            let kind = map.kind();
            Some((side, key, kind))
        } else {
            None
        }
    }

    fn build_note_view(&self, obj: &WavObj) -> Option<(f64, NoteView)> {
        let (side, key, _kind) = Self::lane_of_channel_id(obj.channel_id)?;
        let y = self.y_of_time(obj.offset);
        let distance = y - self.progressed_y;
        let wav_index = Some(obj.wav_id.as_u16() as usize);
        Some((
            y,
            NoteView {
                side,
                key,
                distance_to_hit: distance.into(),
                wav_index,
            },
        ))
    }

    fn event_for_note(&self, obj: &WavObj, y: f64) -> (YCoordinate, ChartEvent) {
        if let Some((side, key, kind)) = Self::lane_of_channel_id(obj.channel_id) {
            let wav_index = Some(obj.wav_id.as_u16() as usize);
            (
                y.into(),
                ChartEvent::Note {
                    side,
                    key,
                    kind,
                    wav_index,
                },
            )
        } else {
            let wav_index = Some(obj.wav_id.as_u16() as usize);
            (y.into(), ChartEvent::Bgm { wav_index })
        }
    }
}

impl<T> ChartProcessor for BmsProcessor<T>
where
    T: KeyLayoutMapper,
{
    fn audio_files(&self) -> HashMap<usize, &Path> {
        self.bms
            .notes
            .wav_files
            .iter()
            .map(|(obj_id, path)| (obj_id.as_u16() as usize, path.as_path()))
            .collect()
    }

    fn bmp_files(&self) -> HashMap<usize, &Path> {
        self.bms
            .graphics
            .bmp_files
            .iter()
            .map(|(obj_id, bmp)| (obj_id.as_u16() as usize, bmp.file.as_path()))
            .collect()
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
        // 初始化 current_bpm 为 header 或默认
        self.current_bpm = self
            .bms
            .arrangers
            .bpm
            .as_ref()
            .and_then(dec_to_f64)
            .unwrap_or(self.default_bpm_bound.max(120.0));
    }

    fn update(&mut self, now: SystemTime) -> Vec<(YCoordinate, ChartEvent)> {
        // 处理通过 post_events 投递的外部事件
        let incoming = std::mem::take(&mut self.inbox);
        for evt in &incoming {
            match *evt {
                ControlEvent::SetDefaultReactionTime { seconds } => {
                    if seconds.is_finite() && seconds > 0.0 {
                        self.default_reaction_time = Duration::from_secs_f64(seconds);
                    }
                }
                ControlEvent::SetDefaultBpmBound { bpm } => {
                    if bpm.is_finite() && bpm > 0.0 {
                        self.default_bpm_bound = bpm;
                    }
                }
            }
        }

        let prev_y = self.progressed_y;
        self.step_to(now);
        let cur_y = self.progressed_y;

        let mut events: Vec<(YCoordinate, ChartEvent)> = Vec::new();

        // Note / Wav 到达事件
        for obj in self.bms.notes().all_notes() {
            let y = self.y_of_time(obj.offset);
            if y > prev_y && y <= cur_y {
                let (y_coord, evt) = self.event_for_note(obj, y);
                events.push((y_coord, evt));
            }
        }

        // BPM 变更
        for change in self.bms.arrangers.bpm_changes.values() {
            let y = self.y_of_time(change.time);
            if y > prev_y
                && y <= cur_y
                && let Some(bpm) = dec_to_f64(&change.bpm)
            {
                events.push((y.into(), ChartEvent::BpmChange { bpm }));
            }
        }

        // Scroll 变更
        for change in self.bms.arrangers.scrolling_factor_changes.values() {
            let y = self.y_of_time(change.time);
            if y > prev_y
                && y <= cur_y
                && let Some(factor) = dec_to_f64(&change.factor)
            {
                events.push((y.into(), ChartEvent::ScrollChange { factor }));
            }
        }

        // Speed 变更
        for change in self.bms.arrangers.speed_factor_changes.values() {
            let y = self.y_of_time(change.time);
            if y > prev_y
                && y <= cur_y
                && let Some(factor) = dec_to_f64(&change.factor)
            {
                events.push((y.into(), ChartEvent::SpeedChange { factor }));
            }
        }

        // Stop 事件
        for stop in self.bms.arrangers.stops.values() {
            let y = self.y_of_time(stop.time);
            if y > prev_y
                && y <= cur_y
                && let Some(d) = dec_to_f64(&stop.duration)
            {
                events.push((y.into(), ChartEvent::Stop { duration: d }));
            }
        }

        // BGA 变化事件
        for bga_obj in self.bms.graphics.bga_changes.values() {
            let y = self.y_of_time(bga_obj.time);
            if y > prev_y && y <= cur_y {
                let bmp_index = bga_obj.id.as_u16() as usize;
                events.push((
                    y.into(),
                    ChartEvent::BgaChange {
                        layer: bga_obj.layer,
                        bmp_index,
                    },
                ));
            }
        }

        // BGA 不透明度变化事件（需要启用 minor-command 特性）
        #[cfg(feature = "minor-command")]
        for (layer, opacity_changes) in &self.bms.graphics.bga_opacity_changes {
            for opacity_obj in opacity_changes.values() {
                let y = self.y_of_time(opacity_obj.time);
                if y > prev_y && y <= cur_y {
                    events.push((
                        y.into(),
                        ChartEvent::BgaOpacityChange {
                            layer: *layer,
                            opacity: opacity_obj.opacity,
                        },
                    ));
                }
            }
        }

        // BGA ARGB 颜色变化事件（需要启用 minor-command 特性）
        #[cfg(feature = "minor-command")]
        for (layer, argb_changes) in &self.bms.graphics.bga_argb_changes {
            for argb_obj in argb_changes.values() {
                let y = self.y_of_time(argb_obj.time);
                if y > prev_y && y <= cur_y {
                    events.push((
                        y.into(),
                        ChartEvent::BgaArgbChange {
                            layer: *layer,
                            argb: ((argb_obj.argb.alpha as u32) << 24)
                                | ((argb_obj.argb.red as u32) << 16)
                                | ((argb_obj.argb.green as u32) << 8)
                                | (argb_obj.argb.blue as u32),
                        },
                    ));
                }
            }
        }

        events.sort_by(|a, b| {
            a.0.value()
                .partial_cmp(&b.0.value())
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

        let mut views: Vec<(f64, NoteView)> = Vec::new();
        for obj in self.bms.notes().all_notes() {
            if let Some((y, mut view)) = self.build_note_view(obj) {
                let raw_distance = y - cur_y;
                let scaled_distance = self.current_scroll * self.current_speed * raw_distance;
                if scaled_distance >= min_scaled && scaled_distance <= max_scaled {
                    view.distance_to_hit = scaled_distance.into();
                    views.push((y, view));
                }
            }
        }
        views.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        views.into_iter().map(|(_, v)| v).collect()
    }
}

#[derive(Debug, Clone, Copy)]
enum FlowEvent {
    Bpm(f64),
    Speed(f64),
    Scroll(f64),
}

fn min_by_y(
    best: Option<(f64, FlowEvent)>,
    candidate: (f64, FlowEvent),
) -> Option<(f64, FlowEvent)> {
    match best {
        None => Some(candidate),
        Some((y, _)) if candidate.0 < y => Some(candidate),
        Some(other) => Some(other),
    }
}
