//! Bmson Processor Module.

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::bms::prelude::*;
use crate::bmson::prelude::*;
use crate::chart_process::{BmpId, ChartEvent, ChartProcessor, ControlEvent, WavId, YCoordinate};
use num::ToPrimitive;
use std::str::FromStr;

/// ChartProcessor of Bmson files.
pub struct BmsonProcessor<'a> {
    bmson: Bmson<'a>,

    // Resource ID mappings
    /// 音频文件名到WavId的映射
    audio_name_to_id: HashMap<String, WavId>,
    /// 图像文件名到BmpId的映射
    bmp_name_to_id: HashMap<String, BmpId>,

    // Playback state
    started_at: Option<SystemTime>,
    last_poll_at: Option<SystemTime>,
    progressed_y: f64,

    // Flow parameters
    default_visible_y_length: YCoordinate,
    current_bpm: Decimal,
    current_speed: Decimal,
    current_scroll: Decimal,

    /// 待消费的外部事件队列
    inbox: Vec<ControlEvent>,

    /// 预加载的事件列表（当前可见区域内的所有事件）
    preloaded_events: Vec<(YCoordinate, ChartEvent)>,
}

impl<'a> BmsonProcessor<'a> {
    /// 创建 BMSON 处理器并初始化播放状态与默认参数。
    #[must_use]
    pub fn new(bmson: Bmson<'a>) -> Self {
        let init_bpm: Decimal = bmson.info.init_bpm.as_f64().into();

        // 预处理：为所有音频和图像资源分配ID
        let mut audio_name_to_id = HashMap::new();
        let mut bmp_name_to_id = HashMap::new();
        let mut next_audio_id = 0usize;
        let mut next_bmp_id = 0usize;

        // 处理音频文件
        for sound_channel in &bmson.sound_channels {
            if let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(sound_channel.name.to_string())
            {
                e.insert(WavId::new(next_audio_id));
                next_audio_id += 1;
            }
        }

        // 处理地雷音频文件
        for mine_channel in &bmson.mine_channels {
            if let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(mine_channel.name.to_string())
            {
                e.insert(WavId::new(next_audio_id));
                next_audio_id += 1;
            }
        }

        // 处理隐藏键音频文件
        for key_channel in &bmson.key_channels {
            if let std::collections::hash_map::Entry::Vacant(e) =
                audio_name_to_id.entry(key_channel.name.to_string())
            {
                e.insert(WavId::new(next_audio_id));
                next_audio_id += 1;
            }
        }

        // 处理图像文件
        for BgaHeader { name, .. } in &bmson.bga.bga_header {
            if let std::collections::hash_map::Entry::Vacant(e) =
                bmp_name_to_id.entry(name.to_string())
            {
                e.insert(BmpId::new(next_bmp_id));
                next_bmp_id += 1;
            }
        }

        // 基于开始BPM和600ms反应时间计算可见Y长度
        // 公式：可见Y长度 = (BPM / 120.0) * 0.6秒
        // 其中 0.6秒 = 600ms，120.0是基准BPM
        let reaction_time_seconds = Decimal::from_str("0.6").unwrap(); // 600ms
        let base_bpm = Decimal::from(120);
        let visible_y_length = (init_bpm.clone() / base_bpm) * reaction_time_seconds;

        Self {
            bmson,
            audio_name_to_id,
            bmp_name_to_id,
            started_at: None,
            last_poll_at: None,
            progressed_y: 0.0,
            inbox: Vec::new(),
            preloaded_events: Vec::new(),
            default_visible_y_length: YCoordinate::from(visible_y_length),
            current_bpm: init_bpm,
            current_speed: Decimal::from(1),
            current_scroll: Decimal::from(1),
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

    /// 获取音频文件名的WavId
    fn get_wav_id_for_name(&self, name: &str) -> Option<WavId> {
        self.audio_name_to_id.get(name).copied()
    }

    /// 获取图像文件名的BmpId
    fn get_bmp_id_for_name(&self, name: &str) -> Option<BmpId> {
        self.bmp_name_to_id.get(name).copied()
    }

    /// 当前瞬时位移速度（y 单位每秒）。
    /// y 为归一化后的小节单位：`y = pulses / (4*resolution)`，默认 4/4 下一小节为 1。
    /// 模型：v = current_bpm / 120.0（使用固定基准BPM 120）
    /// 注：Speed/Scroll 仅影响显示位置（y 缩放），不改变时间轴推进。
    fn current_velocity(&self) -> f64 {
        let base_bpm = Decimal::from(120);
        if self.current_bpm <= Decimal::from(0) {
            return 0.0;
        }
        let velocity = self.current_bpm.clone() / base_bpm;
        velocity.to_f64().unwrap_or(0.0)
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
            FlowEvent::Bpm(bpm) => self.current_bpm = Decimal::from(bpm),
            FlowEvent::Scroll(s) => self.current_scroll = Decimal::from(s),
        }
    }

    fn visible_window_y(&self) -> f64 {
        // 基于当前BPM和600ms反应时间动态计算可见窗口长度
        // 公式：可见Y长度 = (当前BPM / 120.0) * 0.6秒
        let reaction_time_seconds = Decimal::from_str("0.6").unwrap(); // 600ms
        let base_bpm = Decimal::from(120);
        let visible_y = (self.current_bpm.clone() / base_bpm) * reaction_time_seconds;
        visible_y.to_f64().unwrap_or(0.0)
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
        // 注意：BMSON中的音频文件路径是相对于谱面文件的，这里返回虚拟路径
        // 实际使用时需要根据谱面文件位置来解析这些路径
        self.audio_name_to_id
            .iter()
            .map(|(name, id)| (*id, Path::new(name)))
            .collect()
    }

    fn bmp_files(&self) -> HashMap<BmpId, &Path> {
        // 注意：BMSON中的图像文件路径是相对于谱面文件的，这里返回虚拟路径
        // 实际使用时需要根据谱面文件位置来解析这些路径
        self.bmp_name_to_id
            .iter()
            .map(|(name, id)| (*id, Path::new(name)))
            .collect()
    }

    fn default_visible_y_length(&self) -> YCoordinate {
        self.default_visible_y_length.clone()
    }

    fn current_bpm(&self) -> Decimal {
        self.current_bpm.clone()
    }
    fn current_speed(&self) -> Decimal {
        self.current_speed.clone()
    }
    fn current_scroll(&self) -> Decimal {
        self.current_scroll.clone()
    }

    fn start_play(&mut self, now: SystemTime) {
        self.started_at = Some(now);
        self.last_poll_at = Some(now);
        self.progressed_y = 0.0;
        self.preloaded_events.clear();
        self.current_bpm = self.bmson.info.init_bpm.as_f64().into();
    }

    fn update(&mut self, now: SystemTime) -> impl Iterator<Item = (YCoordinate, ChartEvent)> {
        let incoming = std::mem::take(&mut self.inbox);
        for evt in &incoming {
            match evt {
                ControlEvent::SetDefaultVisibleYLength { length } => {
                    self.default_visible_y_length = length.clone();
                }
            }
        }

        let prev_y = self.progressed_y;
        self.step_to(now);
        let cur_y = self.progressed_y;

        // 计算预加载范围：当前 y + 可视 y 范围
        let visible_y_length = self.visible_window_y();
        let preload_end_y = cur_y + visible_y_length;

        // 收集当前时刻触发的事件
        let mut triggered_events: Vec<(YCoordinate, ChartEvent)> = Vec::new();

        // 收集预加载范围内的事件
        let mut new_preloaded_events: Vec<(YCoordinate, ChartEvent)> = Vec::new();

        for SoundChannel { name, notes } in &self.bmson.sound_channels {
            for Note { y, x, l, c, .. } in notes {
                let yy = self.pulses_to_y(y.0);
                if yy > prev_y && yy <= cur_y {
                    // 当前时刻触发的事件
                    if let Some((side, key)) = Self::lane_from_x(x.as_ref().copied()) {
                        let wav_id = self.get_wav_id_for_name(name);
                        let length = if *l > 0 {
                            let end_y = self.pulses_to_y(y.0 + l);
                            Some(YCoordinate::from(end_y - yy))
                        } else {
                            None
                        };
                        let kind = if *l > 0 {
                            NoteKind::Long
                        } else {
                            NoteKind::Visible
                        };
                        triggered_events.push((
                            yy.into(),
                            ChartEvent::Note {
                                side,
                                key,
                                kind,
                                wav_id,
                                length,
                                continue_play: *c,
                            },
                        ));
                    } else {
                        let wav_id = self.get_wav_id_for_name(name);
                        triggered_events.push((yy.into(), ChartEvent::Bgm { wav_id }));
                    }
                }
                if yy > cur_y && yy <= preload_end_y {
                    // 预加载范围内的事件
                    if let Some((side, key)) = Self::lane_from_x(x.as_ref().copied()) {
                        let wav_id = self.get_wav_id_for_name(name);
                        let length = if *l > 0 {
                            let end_y = self.pulses_to_y(y.0 + l);
                            Some(YCoordinate::from(end_y - yy))
                        } else {
                            None
                        };
                        let kind = if *l > 0 {
                            NoteKind::Long
                        } else {
                            NoteKind::Visible
                        };
                        new_preloaded_events.push((
                            yy.into(),
                            ChartEvent::Note {
                                side,
                                key,
                                kind,
                                wav_id,
                                length,
                                continue_play: *c,
                            },
                        ));
                    } else {
                        let wav_id = self.get_wav_id_for_name(name);
                        new_preloaded_events.push((yy.into(), ChartEvent::Bgm { wav_id }));
                    }
                }
            }
        }

        for ev in &self.bmson.bpm_events {
            let y = self.pulses_to_y(ev.y.0);
            if y > prev_y && y <= cur_y {
                triggered_events.push((
                    y.into(),
                    ChartEvent::BpmChange {
                        bpm: ev.bpm.as_f64().into(),
                    },
                ));
            }
            if y > cur_y && y <= preload_end_y {
                new_preloaded_events.push((
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
                triggered_events.push((
                    y.into(),
                    ChartEvent::ScrollChange {
                        factor: rate.as_f64().into(),
                    },
                ));
            }
            if y > cur_y && y <= preload_end_y {
                new_preloaded_events.push((
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
                triggered_events.push((
                    y.into(),
                    ChartEvent::Stop {
                        duration: (stop.duration as f64).into(),
                    },
                ));
            }
            if y > cur_y && y <= preload_end_y {
                new_preloaded_events.push((
                    y.into(),
                    ChartEvent::Stop {
                        duration: (stop.duration as f64).into(),
                    },
                ));
            }
        }

        // BGA 基础层事件
        for BgaEvent { y, id, .. } in &self.bmson.bga.bga_events {
            let yy = self.pulses_to_y(y.0);
            if yy > prev_y && yy <= cur_y {
                let bmp_name = self
                    .bmson
                    .bga
                    .bga_header
                    .iter()
                    .find(|header| header.id.0 == id.0)
                    .map(|header| &*header.name);
                let bmp_id = bmp_name.and_then(|name| self.get_bmp_id_for_name(name));
                triggered_events.push((
                    yy.into(),
                    ChartEvent::BgaChange {
                        layer: BgaLayer::Base,
                        bmp_id,
                    },
                ));
            }
            if yy > cur_y && yy <= preload_end_y {
                let bmp_name = self
                    .bmson
                    .bga
                    .bga_header
                    .iter()
                    .find(|header| header.id.0 == id.0)
                    .map(|header| &*header.name);
                let bmp_id = bmp_name.and_then(|name| self.get_bmp_id_for_name(name));
                new_preloaded_events.push((
                    yy.into(),
                    ChartEvent::BgaChange {
                        layer: BgaLayer::Base,
                        bmp_id,
                    },
                ));
            }
        }

        // BGA 覆盖层事件
        for BgaEvent { y, id, .. } in &self.bmson.bga.layer_events {
            let yy = self.pulses_to_y(y.0);
            if yy > prev_y && yy <= cur_y {
                let bmp_name = self
                    .bmson
                    .bga
                    .bga_header
                    .iter()
                    .find(|header| header.id.0 == id.0)
                    .map(|header| &*header.name);
                let bmp_id = bmp_name.and_then(|name| self.get_bmp_id_for_name(name));
                triggered_events.push((
                    yy.into(),
                    ChartEvent::BgaChange {
                        layer: BgaLayer::Overlay,
                        bmp_id,
                    },
                ));
            }
            if yy > cur_y && yy <= preload_end_y {
                let bmp_name = self
                    .bmson
                    .bga
                    .bga_header
                    .iter()
                    .find(|header| header.id.0 == id.0)
                    .map(|header| &*header.name);
                let bmp_id = bmp_name.and_then(|name| self.get_bmp_id_for_name(name));
                new_preloaded_events.push((
                    yy.into(),
                    ChartEvent::BgaChange {
                        layer: BgaLayer::Overlay,
                        bmp_id,
                    },
                ));
            }
        }

        // BGA 失败层事件
        for BgaEvent { y, id, .. } in &self.bmson.bga.poor_events {
            let yy = self.pulses_to_y(y.0);
            if yy > prev_y && yy <= cur_y {
                let bmp_name = self
                    .bmson
                    .bga
                    .bga_header
                    .iter()
                    .find(|header| header.id.0 == id.0)
                    .map(|header| &*header.name);
                let bmp_id = bmp_name.and_then(|name| self.get_bmp_id_for_name(name));
                triggered_events.push((
                    yy.into(),
                    ChartEvent::BgaChange {
                        layer: BgaLayer::Poor,
                        bmp_id,
                    },
                ));
            }
            if yy > cur_y && yy <= preload_end_y {
                let bmp_name = self
                    .bmson
                    .bga
                    .bga_header
                    .iter()
                    .find(|header| header.id.0 == id.0)
                    .map(|header| &*header.name);
                let bmp_id = bmp_name.and_then(|name| self.get_bmp_id_for_name(name));
                new_preloaded_events.push((
                    yy.into(),
                    ChartEvent::BgaChange {
                        layer: BgaLayer::Poor,
                        bmp_id,
                    },
                ));
            }
        }

        // 小节线事件
        if let Some(lines) = &self.bmson.lines {
            for bar_line in lines {
                let y = self.pulses_to_y(bar_line.y.0);
                if y > prev_y && y <= cur_y {
                    triggered_events.push((y.into(), ChartEvent::BarLine));
                }
                if y > cur_y && y <= preload_end_y {
                    new_preloaded_events.push((y.into(), ChartEvent::BarLine));
                }
            }
        }

        // 地雷通道事件
        for MineChannel { name, notes } in &self.bmson.mine_channels {
            for MineEvent { x, y, .. } in notes {
                let yy = self.pulses_to_y(y.0);
                if yy > prev_y
                    && yy <= cur_y
                    && let Some((side, key)) = Self::lane_from_x(*x)
                {
                    let wav_id = self.get_wav_id_for_name(name);
                    triggered_events.push((
                        yy.into(),
                        ChartEvent::Note {
                            side,
                            key,
                            kind: NoteKind::Landmine,
                            wav_id,
                            length: None,
                            continue_play: false,
                        },
                    ));
                }
                if yy > cur_y
                    && yy <= preload_end_y
                    && let Some((side, key)) = Self::lane_from_x(*x)
                {
                    let wav_id = self.get_wav_id_for_name(name);
                    new_preloaded_events.push((
                        yy.into(),
                        ChartEvent::Note {
                            side,
                            key,
                            kind: NoteKind::Landmine,
                            wav_id,
                            length: None,
                            continue_play: false,
                        },
                    ));
                }
            }
        }

        // 隐藏键通道事件
        for KeyChannel { name, notes } in &self.bmson.key_channels {
            for KeyEvent { x, y, .. } in notes {
                let yy = self.pulses_to_y(y.0);
                if yy > prev_y
                    && yy <= cur_y
                    && let Some((side, key)) = Self::lane_from_x(*x)
                {
                    let wav_id = self.get_wav_id_for_name(name);
                    triggered_events.push((
                        yy.into(),
                        ChartEvent::Note {
                            side,
                            key,
                            kind: NoteKind::Invisible,
                            wav_id,
                            length: None,
                            continue_play: false,
                        },
                    ));
                }
                if yy > cur_y
                    && yy <= preload_end_y
                    && let Some((side, key)) = Self::lane_from_x(*x)
                {
                    let wav_id = self.get_wav_id_for_name(name);
                    new_preloaded_events.push((
                        yy.into(),
                        ChartEvent::Note {
                            side,
                            key,
                            kind: NoteKind::Invisible,
                            wav_id,
                            length: None,
                            continue_play: false,
                        },
                    ));
                }
            }
        }

        // 排序事件
        triggered_events.sort_by(|a, b| {
            a.0.value()
                .partial_cmp(b.0.value())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        new_preloaded_events.sort_by(|a, b| {
            a.0.value()
                .partial_cmp(b.0.value())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 更新预加载事件列表
        self.preloaded_events = new_preloaded_events;

        triggered_events.into_iter()
    }

    fn post_events(&mut self, events: &[ControlEvent]) {
        self.inbox.extend_from_slice(events);
    }

    fn visible_events(
        &mut self,
        now: SystemTime,
    ) -> impl Iterator<Item = (YCoordinate, ChartEvent)> {
        self.step_to(now);
        self.preloaded_events.iter().cloned()
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
