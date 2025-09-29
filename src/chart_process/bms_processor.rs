//! Bms Processor Module.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::bms::prelude::*;
use crate::chart_process::{
    BmpId, ChartEvent, ChartProcessor, ControlEvent, DisplayRatio, WavId, YCoordinate,
};
use num::ToPrimitive;
use std::str::FromStr;

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

    /// 已生成小节线的Track集合
    generated_bar_lines: HashSet<Track>,

    /// 所有事件的映射（按 Y 坐标排序）
    all_events: BTreeMap<YCoordinate, Vec<ChartEvent>>,

    /// 预加载的事件列表（当前可见区域内的所有事件）
    preloaded_events: Vec<(YCoordinate, ChartEvent)>,

    // Flow parameters
    default_visible_y_length: YCoordinate,
    current_bpm: Decimal,
    current_speed: Decimal,
    current_scroll: Decimal,
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
            .cloned()
            .unwrap_or(Decimal::from(120));

        // 基于开始BPM和600ms反应时间计算可见Y长度
        // 公式：可见Y长度 = (BPM / 120.0) * 0.6秒
        // 其中 0.6秒 = 600ms，120.0是基准BPM
        let reaction_time_seconds = Decimal::from_str("0.6").unwrap(); // 600ms
        let base_bpm = Decimal::from(120);
        let visible_y_length = (init_bpm.clone() / base_bpm) * reaction_time_seconds;

        let all_events = Self::precompute_all_events(&bms);

        Self {
            bms,
            started_at: None,
            last_poll_at: None,
            progressed_y: 0.0,
            inbox: Vec::new(),
            generated_bar_lines: HashSet::new(),
            all_events,
            preloaded_events: Vec::new(),
            default_visible_y_length: YCoordinate::from(visible_y_length),
            current_bpm: init_bpm,
            current_speed: Decimal::from(1),
            current_scroll: Decimal::from(1),
        }
    }

    /// 预先计算所有事件，按 Y 坐标分组存储
    fn precompute_all_events(bms: &Bms<T>) -> BTreeMap<YCoordinate, Vec<ChartEvent>> {
        let mut events_map: BTreeMap<YCoordinate, Vec<ChartEvent>> = BTreeMap::new();

        // Note / Wav 到达事件
        for obj in bms.notes().all_notes() {
            let y = YCoordinate::from(Self::y_of_time_static(bms, obj.offset));
            let event = Self::event_for_note_static(bms, obj, y.value().to_f64().unwrap_or(0.0));

            events_map.entry(y).or_default().push(event);
        }

        // BPM 变更事件
        for change in bms.arrangers.bpm_changes.values() {
            let y = YCoordinate::from(Self::y_of_time_static(bms, change.time));
            let event = ChartEvent::BpmChange {
                bpm: change.bpm.clone(),
            };

            events_map.entry(y).or_default().push(event);
        }

        // Scroll 变更事件
        for change in bms.arrangers.scrolling_factor_changes.values() {
            let y = YCoordinate::from(Self::y_of_time_static(bms, change.time));
            let event = ChartEvent::ScrollChange {
                factor: change.factor.clone(),
            };

            events_map.entry(y).or_default().push(event);
        }

        // Speed 变更事件
        for change in bms.arrangers.speed_factor_changes.values() {
            let y = YCoordinate::from(Self::y_of_time_static(bms, change.time));
            let event = ChartEvent::SpeedChange {
                factor: change.factor.clone(),
            };

            events_map.entry(y).or_default().push(event);
        }

        // Stop 事件
        for stop in bms.arrangers.stops.values() {
            let y = YCoordinate::from(Self::y_of_time_static(bms, stop.time));
            let event = ChartEvent::Stop {
                duration: stop.duration.clone(),
            };

            events_map.entry(y).or_default().push(event);
        }

        // BGA 变化事件
        for bga_obj in bms.graphics.bga_changes.values() {
            let y = YCoordinate::from(Self::y_of_time_static(bms, bga_obj.time));
            let bmp_index = bga_obj.id.as_u16() as usize;
            let event = ChartEvent::BgaChange {
                layer: bga_obj.layer,
                bmp_id: Some(BmpId::from(bmp_index)),
            };

            events_map.entry(y).or_default().push(event);
        }

        // BGA 不透明度变化事件（需要启用 minor-command 特性）
        #[cfg(feature = "minor-command")]
        for (layer, opacity_changes) in &bms.graphics.bga_opacity_changes {
            for opacity_obj in opacity_changes.values() {
                let y = YCoordinate::from(Self::y_of_time_static(bms, opacity_obj.time));
                let event = ChartEvent::BgaOpacityChange {
                    layer: *layer,
                    opacity: opacity_obj.opacity,
                };

                events_map.entry(y).or_default().push(event);
            }
        }

        // BGA ARGB 颜色变化事件（需要启用 minor-command 特性）
        #[cfg(feature = "minor-command")]
        for (layer, argb_changes) in &bms.graphics.bga_argb_changes {
            for argb_obj in argb_changes.values() {
                let y = YCoordinate::from(Self::y_of_time_static(bms, argb_obj.time));
                let argb = ((argb_obj.argb.alpha as u32) << 24)
                    | ((argb_obj.argb.red as u32) << 16)
                    | ((argb_obj.argb.green as u32) << 8)
                    | (argb_obj.argb.blue as u32);
                let event = ChartEvent::BgaArgbChange {
                    layer: *layer,
                    argb,
                };

                events_map.entry(y).or_default().push(event);
            }
        }

        // BGM 音量变化事件
        for bgm_volume_obj in bms.notes.bgm_volume_changes.values() {
            let y = YCoordinate::from(Self::y_of_time_static(bms, bgm_volume_obj.time));
            let event = ChartEvent::BgmVolumeChange {
                volume: bgm_volume_obj.volume,
            };

            events_map.entry(y).or_default().push(event);
        }

        // KEY 音量变化事件
        for key_volume_obj in bms.notes.key_volume_changes.values() {
            let y = YCoordinate::from(Self::y_of_time_static(bms, key_volume_obj.time));
            let event = ChartEvent::KeyVolumeChange {
                volume: key_volume_obj.volume,
            };

            events_map.entry(y).or_default().push(event);
        }

        // 文本显示事件
        for text_obj in bms.notes.text_events.values() {
            let y = YCoordinate::from(Self::y_of_time_static(bms, text_obj.time));
            let event = ChartEvent::TextDisplay {
                text: text_obj.text.clone(),
            };

            events_map.entry(y).or_default().push(event);
        }

        // 判定等级变化事件
        for judge_obj in bms.notes.judge_events.values() {
            let y = YCoordinate::from(Self::y_of_time_static(bms, judge_obj.time));
            let event = ChartEvent::JudgeLevelChange {
                level: judge_obj.judge_level,
            };

            events_map.entry(y).or_default().push(event);
        }

        // Minor-command 特性事件
        #[cfg(feature = "minor-command")]
        {
            // 视频跳转事件
            for seek_obj in bms.notes.seek_events.values() {
                let y = YCoordinate::from(Self::y_of_time_static(bms, seek_obj.time));
                let event = ChartEvent::VideoSeek {
                    seek_time: seek_obj.position.to_string().parse::<f64>().unwrap_or(0.0),
                };

                events_map.entry(y).or_default().push(event);
            }

            // BGA 键绑定事件
            for bga_keybound_obj in bms.notes.bga_keybound_events.values() {
                let y = YCoordinate::from(Self::y_of_time_static(bms, bga_keybound_obj.time));
                let event = ChartEvent::BgaKeybound {
                    event: bga_keybound_obj.event.clone(),
                };

                events_map.entry(y).or_default().push(event);
            }

            // 选项变化事件
            for option_obj in bms.notes.option_events.values() {
                let y = YCoordinate::from(Self::y_of_time_static(bms, option_obj.time));
                let event = ChartEvent::OptionChange {
                    option: option_obj.option.clone(),
                };

                events_map.entry(y).or_default().push(event);
            }
        }

        events_map
    }

    /// 静态版本的 y_of_time，用于预计算
    fn y_of_time_static(bms: &Bms<T>, time: ObjTime) -> f64 {
        let mut y = 0.0f64;
        // 累加完整小节
        for t in 0..time.track().0 {
            let section_len = bms
                .arrangers
                .section_len_changes
                .get(&Track(t))
                .and_then(|s| dec_to_f64(&s.length))
                .unwrap_or(1.0);
            y += section_len;
        }
        // 当前小节内按比例累加
        let current_len = bms
            .arrangers
            .section_len_changes
            .get(&time.track())
            .and_then(|s| dec_to_f64(&s.length))
            .unwrap_or(1.0);
        if time.denominator().get() > 0 {
            y += current_len * (time.numerator() as f64) / (time.denominator().get() as f64);
        }
        y
    }

    /// 静态版本的 event_for_note，用于预计算
    fn event_for_note_static(bms: &Bms<T>, obj: &WavObj, y: f64) -> ChartEvent {
        if let Some((side, key, kind)) = Self::lane_of_channel_id(obj.channel_id) {
            let wav_id = Some(WavId::from(obj.wav_id.as_u16() as usize));
            let length = if kind == NoteKind::Long {
                // 长条音符：查找下一个同通道的音符来计算长度
                if let Some(next_obj) = bms.notes().next_obj_by_key(obj.channel_id, obj.offset) {
                    let next_y = Self::y_of_time_static(bms, next_obj.offset);
                    Some(YCoordinate::from(next_y - y))
                } else {
                    None
                }
            } else {
                None
            };
            ChartEvent::Note {
                side,
                key,
                kind,
                wav_id,
                length,
                continue_play: false, // BMS固定为false
            }
        } else {
            let wav_id = Some(WavId::from(obj.wav_id.as_u16() as usize));
            ChartEvent::Bgm { wav_id }
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
    /// 模型：v = (current_bpm / 120.0) * speed_factor（使用固定基准BPM 120）
    /// 注：Speed 影响y前进速度，但不改变实际时间推进；Scroll 仅影响显示位置。
    fn current_velocity(&self) -> f64 {
        let base_bpm = Decimal::from(120);
        if self.current_bpm <= Decimal::from(0) {
            return 0.0;
        }
        let velocity = self.current_bpm.clone() / base_bpm;
        let speed_factor = self.current_speed.to_f64().unwrap_or(1.0);
        (velocity.to_f64().unwrap_or(0.0) * speed_factor).max(std::f64::EPSILON) // speed必须为正值
    }

    /// 取下一条会影响速度的事件（按 y 升序）：BPM/SCROLL/SPEED 变更。
    fn next_flow_event_after(&self, y_from_exclusive: f64) -> Option<(f64, FlowEvent)> {
        // 收集三个事件源，找 y 大于阈值的最小项
        let mut best: Option<(f64, FlowEvent)> = None;

        for change in self.bms.arrangers.bpm_changes.values() {
            let y = self.y_of_time(change.time);
            if y > y_from_exclusive {
                let bpm =
                    dec_to_f64(&change.bpm).unwrap_or(self.current_bpm.to_f64().unwrap_or(120.0));
                best = min_by_y(best, (y, FlowEvent::Bpm(bpm)));
            }
        }
        for change in self.bms.arrangers.scrolling_factor_changes.values() {
            let y = self.y_of_time(change.time);
            if y > y_from_exclusive {
                let factor = dec_to_f64(&change.factor)
                    .unwrap_or(self.current_scroll.to_f64().unwrap_or(1.0));
                best = min_by_y(best, (y, FlowEvent::Scroll(factor)));
            }
        }
        for change in self.bms.arrangers.speed_factor_changes.values() {
            let y = self.y_of_time(change.time);
            if y > y_from_exclusive {
                let factor = dec_to_f64(&change.factor)
                    .unwrap_or(self.current_speed.to_f64().unwrap_or(1.0));
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
            FlowEvent::Bpm(bpm) => self.current_bpm = Decimal::from(bpm),
            FlowEvent::Speed(s) => self.current_speed = Decimal::from(s),
            FlowEvent::Scroll(s) => self.current_scroll = Decimal::from(s),
        }
    }

    /// 计算可见窗口长度（y 单位）：基于当前BPM和600ms反应时间
    fn visible_window_y(&self) -> f64 {
        // 基于当前BPM和600ms反应时间动态计算可见窗口长度
        // 公式：可见Y长度 = (当前BPM / 120.0) * 0.6秒
        let reaction_time_seconds = Decimal::from_str("0.6").unwrap(); // 600ms
        let base_bpm = Decimal::from(120);
        let visible_y = (self.current_bpm.clone() / base_bpm) * reaction_time_seconds;
        visible_y.to_f64().unwrap_or(0.0)
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
}

impl<T> ChartProcessor for BmsProcessor<T>
where
    T: KeyLayoutMapper,
{
    fn audio_files(&self) -> HashMap<WavId, &Path> {
        self.bms
            .notes
            .wav_files
            .iter()
            .map(|(obj_id, path)| (WavId::from(obj_id.as_u16() as usize), path.as_path()))
            .collect()
    }

    fn bmp_files(&self) -> HashMap<BmpId, &Path> {
        self.bms
            .graphics
            .bmp_files
            .iter()
            .map(|(obj_id, bmp)| (BmpId::from(obj_id.as_u16() as usize), bmp.file.as_path()))
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
        self.generated_bar_lines.clear();
        self.preloaded_events.clear();
        // 初始化 current_bpm 为 header 或默认
        self.current_bpm = self
            .bms
            .arrangers
            .bpm
            .as_ref()
            .cloned()
            .unwrap_or(Decimal::from(120));
    }

    fn update(&mut self, now: SystemTime) -> impl Iterator<Item = (YCoordinate, ChartEvent)> {
        // 处理通过 post_events 投递的外部事件
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

        // 从预先计算好的事件 Map 中获取事件
        for (y_coord, events) in &self.all_events {
            let y = y_coord.value().to_f64().unwrap_or(0.0);

            // 如果事件在当前时刻触发
            if y > prev_y && y <= cur_y {
                for event in events {
                    triggered_events.push((y_coord.clone(), event.clone()));
                }
            }

            // 如果事件在预加载范围内
            if y > cur_y && y <= preload_end_y {
                for event in events {
                    new_preloaded_events.push((y_coord.clone(), event.clone()));
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
    ) -> impl Iterator<Item = (YCoordinate, ChartEvent, DisplayRatio)> {
        self.step_to(now);
        let current_y = self.progressed_y;
        let visible_window_y = self.visible_window_y();
        let scroll_factor = self.current_scroll.to_f64().unwrap_or(1.0);

        self.preloaded_events.iter().map(move |(y_coord, event)| {
            let event_y = y_coord.value().to_f64().unwrap_or(std::f64::EPSILON);
            // 计算显示比例：(event_y - current_y) / visible_window_y * scroll_factor
            // 注意：scroll可以为非零的正负值
            let display_ratio_value = if visible_window_y > 0.0 {
                ((event_y - current_y) / visible_window_y) * scroll_factor
            } else {
                0.0
            };
            let display_ratio = DisplayRatio::from(display_ratio_value);
            (y_coord.clone(), event.clone(), display_ratio)
        })
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
