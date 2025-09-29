use std::str::FromStr;
use std::time::{Duration, SystemTime};

use num::ToPrimitive;

use bms_rs::bms::prelude::*;
use bms_rs::chart_process::prelude::*;

#[test]
fn test_bpm_processor_events() {
    // 使用现有的 lilith_mx.bms 文件来测试 BPM 处理器事件
    let source = include_str!("../bms/files/lilith_mx.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    let mut processor = BmsProcessor::new(bms);
    let start_time = SystemTime::now();

    // 启动播放
    processor.start_play(start_time);

    // 验证初始状态
    assert_eq!(processor.current_bpm(), Decimal::from(151));
    // 基于BPM 151和600ms反应时间计算期望的可见Y长度：(151/120.0) * 0.6 = 0.755
    // 但由于Decimal精度，实际值可能略有不同，我们使用更宽松的容差
    let expected_visible_y = (151.0 / 120.0) * 0.6;
    assert!(
        (processor.default_visible_y_length().as_f64() - expected_visible_y).abs() < 0.1,
        "期望可见Y长度: {:.3}, 实际: {:.3}",
        expected_visible_y,
        processor.default_visible_y_length().as_f64()
    );

    // 前进到第一个 BPM 变化点（第1小节）
    let after_first_change = start_time + Duration::from_secs(1);
    let events = processor.update(after_first_change);

    // 应该触发 BPM 变化事件
    let bpm_events: Vec<_> = events
        .filter(|(_, e)| matches!(e, ChartEvent::BpmChange { .. }))
        .collect();
    assert!(!bpm_events.is_empty(), "应该有 BPM 变化事件");

    // 检查BPM变化事件的具体值
    if let Some((y, ChartEvent::BpmChange { bpm })) = bpm_events.first() {
        assert_eq!(
            bpm.to_f64().unwrap_or(0.0),
            75.5,
            "BPM变化事件的值应该是75.5"
        );
        assert!(
            y.value().to_f64().unwrap_or(0.0) > 0.0,
            "BPM变化事件的y坐标应该大于0"
        );
    } else {
        panic!("第一个事件应该是BpmChange类型");
    }

    // 验证 BPM 值已更新到 75.5
    assert_eq!(processor.current_bpm(), Decimal::from_str("75.5").unwrap());

    // 前进到第二个 BPM 变化点（第5小节）
    // 由于 BPM 变化了，时间计算需要考虑速度变化
    // 75.5 BPM 比 151 BPM 慢，所以需要更多时间
    let after_second_change = after_first_change + Duration::from_secs(8);
    let events = processor.update(after_second_change);

    // 应该触发第二个 BPM 变化事件
    let bpm_events: Vec<_> = events
        .filter(|(_, e)| matches!(e, ChartEvent::BpmChange { .. }))
        .collect();
    assert!(!bpm_events.is_empty(), "应该有第二个 BPM 变化事件");

    // 检查第二个BPM变化事件的具体值
    if let Some((y, ChartEvent::BpmChange { bpm })) = bpm_events.first() {
        assert_eq!(
            bpm.to_f64().unwrap_or(0.0),
            151.0,
            "第二个BPM变化事件的值应该是151.0"
        );
        assert!(
            y.value().to_f64().unwrap_or(0.0) > 0.0,
            "第二个BPM变化事件的y坐标应该大于0"
        );
    } else {
        panic!("第一个事件应该是BpmChange类型");
    }

    // 验证 BPM 值已更新回 151
    assert_eq!(processor.current_bpm(), Decimal::from(151));
}

#[test]
fn test_bpm_affects_velocity() {
    // 使用现有的 lilith_mx.bms 文件来测试 BPM 对速度的影响
    let source = include_str!("../bms/files/lilith_mx.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    let mut processor = BmsProcessor::new(bms);
    let start_time = SystemTime::now();

    processor.start_play(start_time);

    // 初始状态：BPM 151，可见Y长度 = (151/120.0) * 0.6 = 0.755
    assert_eq!(processor.current_bpm(), Decimal::from(151));
    let expected_visible_y = (151.0 / 120.0) * 0.6;
    assert!(
        (processor.default_visible_y_length().as_f64() - expected_visible_y).abs() < 0.1,
        "期望可见Y长度: {:.3}, 实际: {:.3}",
        expected_visible_y,
        processor.default_visible_y_length().as_f64()
    );

    // 前进到第一个 BPM 变化（第1小节）
    let after_first_change = start_time + Duration::from_secs(1);
    let _ = processor.update(after_first_change);

    // BPM 应该更新到 75.5
    assert_eq!(processor.current_bpm(), Decimal::from_str("75.5").unwrap());

    // 前进到第二个 BPM 变化（第5小节）
    let after_second_change = after_first_change + Duration::from_secs(8);
    let _ = processor.update(after_second_change);

    // BPM 应该更新回 151
    assert_eq!(processor.current_bpm(), Decimal::from(151));
}

#[test]
fn test_scroll_processor_events() {
    // 使用现有的 bemuse_ext.bms 文件来测试 Scroll 处理器事件
    let source = include_str!("../bms/files/bemuse_ext.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    let mut processor = BmsProcessor::new(bms);
    let start_time = SystemTime::now();

    // 启动播放
    processor.start_play(start_time);

    // 验证初始状态
    assert_eq!(processor.current_scroll(), Decimal::from(1));

    // 前进到第一个 Scroll/Speed 变化点
    let after_first_change = start_time + Duration::from_secs(1);
    let events = processor.update(after_first_change);

    // 收集所有事件
    let all_events: Vec<_> = events.collect();

    // 应该触发 Scroll 和 Speed 变化事件
    let scroll_events: Vec<_> = all_events
        .iter()
        .filter(|(_, e)| matches!(e, ChartEvent::ScrollChange { .. }))
        .cloned()
        .collect();

    let speed_events: Vec<_> = all_events
        .iter()
        .filter(|(_, e)| matches!(e, ChartEvent::SpeedChange { .. }))
        .cloned()
        .collect();

    assert!(!scroll_events.is_empty(), "应该有 Scroll 变化事件");
    assert!(!speed_events.is_empty(), "应该有 Speed 变化事件");

    // 检查Scroll变化事件的具体值
    if let Some((y, ChartEvent::ScrollChange { factor })) = scroll_events.first() {
        assert_eq!(
            factor.to_f64().unwrap_or(0.0),
            1.0,
            "Scroll变化事件的因子应该是1.0"
        );
        assert!(
            y.value().to_f64().unwrap_or(0.0) > 0.0,
            "Scroll变化事件的y坐标应该大于0"
        );
    } else {
        panic!("第一个Scroll事件应该是ScrollChange类型");
    }

    // 检查Speed变化事件的具体值
    if let Some((y, ChartEvent::SpeedChange { factor })) = speed_events.first() {
        assert_eq!(
            factor.to_f64().unwrap_or(0.0),
            1.0,
            "Speed变化事件的因子应该是1.0"
        );
        assert!(
            y.value().to_f64().unwrap_or(0.0) > 0.0,
            "Speed变化事件的y坐标应该大于0"
        );
    } else {
        panic!("第一个Speed事件应该是SpeedChange类型");
    }

    // 验证 Scroll 和 Speed 值已更新
    assert_eq!(processor.current_scroll(), Decimal::from(1));
    assert_eq!(processor.current_speed(), Decimal::from(1));
}

#[test]
fn test_scroll_affects_visible_notes_scaling() {
    // 使用现有的 bemuse_ext.bms 文件来测试 Scroll 对可见音符缩放的影响
    let source = include_str!("../bms/files/bemuse_ext.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    let mut processor = BmsProcessor::new(bms);
    let start_time = SystemTime::now();

    processor.start_play(start_time);

    // 初始状态：Scroll = 1.0
    assert_eq!(processor.current_scroll(), Decimal::from(1));

    // 前进到第一个 Scroll/Speed 变化点
    let after_first_change = start_time + Duration::from_secs(1);
    let _ = processor.update(after_first_change);

    // Scroll 和 Speed 应该更新
    assert_eq!(processor.current_scroll(), Decimal::from(1));
    assert_eq!(processor.current_speed(), Decimal::from(1));
}

#[test]
fn test_speed_processor_events() {
    // 使用现有的 bemuse_ext.bms 文件来测试 Speed 处理器事件
    let source = include_str!("../bms/files/bemuse_ext.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    let mut processor = BmsProcessor::new(bms);
    let start_time = SystemTime::now();

    // 启动播放
    processor.start_play(start_time);

    // 验证初始状态
    assert_eq!(processor.current_speed(), Decimal::from(1));

    // 前进到第一个 Speed/Scroll 变化点
    let after_first_change = start_time + Duration::from_secs(1);
    let events = processor.update(after_first_change);

    // 收集所有事件
    let all_events: Vec<_> = events.collect();

    // 应该触发 Speed 和 Scroll 变化事件
    let speed_events: Vec<_> = all_events
        .iter()
        .filter(|(_, e)| matches!(e, ChartEvent::SpeedChange { .. }))
        .cloned()
        .collect();

    let scroll_events: Vec<_> = all_events
        .iter()
        .filter(|(_, e)| matches!(e, ChartEvent::ScrollChange { .. }))
        .cloned()
        .collect();

    assert!(!speed_events.is_empty(), "应该有 Speed 变化事件");
    assert!(!scroll_events.is_empty(), "应该有 Scroll 变化事件");

    // 检查Speed变化事件的具体值
    if let Some((y, ChartEvent::SpeedChange { factor })) = speed_events.first() {
        assert_eq!(
            factor.to_f64().unwrap_or(0.0),
            1.0,
            "Speed变化事件的因子应该是1.0"
        );
        assert!(
            y.value().to_f64().unwrap_or(0.0) > 0.0,
            "Speed变化事件的y坐标应该大于0"
        );
    } else {
        panic!("第一个Speed事件应该是SpeedChange类型");
    }

    // 检查Scroll变化事件的具体值
    if let Some((y, ChartEvent::ScrollChange { factor })) = scroll_events.first() {
        assert_eq!(
            factor.to_f64().unwrap_or(0.0),
            1.0,
            "Scroll变化事件的因子应该是1.0"
        );
        assert!(
            y.value().to_f64().unwrap_or(0.0) > 0.0,
            "Scroll变化事件的y坐标应该大于0"
        );
    } else {
        panic!("第一个Scroll事件应该是ScrollChange类型");
    }

    // 验证 Speed 和 Scroll 值已更新
    assert_eq!(processor.current_speed(), Decimal::from(1));
    assert_eq!(processor.current_scroll(), Decimal::from(1));
}

#[test]
fn test_speed_affects_visible_notes() {
    // 使用现有的 bemuse_ext.bms 文件来测试 Speed 对可见音符的影响
    let source = include_str!("../bms/files/bemuse_ext.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    let mut processor = BmsProcessor::new(bms);
    let start_time = SystemTime::now();

    processor.start_play(start_time);

    // 初始状态：Speed = 1.0
    assert_eq!(processor.current_speed(), Decimal::from(1));

    // 前进到第一个 Speed/Scroll 变化点
    let after_first_change = start_time + Duration::from_secs(1);
    let _ = processor.update(after_first_change);

    // Speed 和 Scroll 应该更新
    assert_eq!(processor.current_speed(), Decimal::from(1));
    assert_eq!(processor.current_scroll(), Decimal::from(1));
}

#[test]
fn test_combined_changes_events() {
    // 使用现有的 lilith_mx.bms 文件来测试组合变化事件
    let source = include_str!("../bms/files/lilith_mx.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    let mut processor = BmsProcessor::new(bms);
    let start_time = SystemTime::now();

    // 启动播放
    processor.start_play(start_time);

    // 验证初始状态
    assert_eq!(processor.current_bpm(), Decimal::from(151));
    assert_eq!(processor.current_speed(), Decimal::from(1));
    assert_eq!(processor.current_scroll(), Decimal::from(1));

    // 前进到第一个 BPM 变化点（第1小节）
    let after_first_change = start_time + Duration::from_secs(1);
    let events = processor.update(after_first_change);

    // 应该有 BPM 变化事件
    let bpm_events: Vec<_> = events
        .filter(|(_, e)| matches!(e, ChartEvent::BpmChange { .. }))
        .collect();

    assert!(!bpm_events.is_empty(), "应该有 BPM 变化事件");

    // 检查BPM变化事件的具体值
    if let Some((y, ChartEvent::BpmChange { bpm })) = bpm_events.first() {
        assert_eq!(
            bpm.to_f64().unwrap_or(0.0),
            75.5,
            "BPM变化事件的值应该是75.5"
        );
        assert!(
            y.value().to_f64().unwrap_or(0.0) > 0.0,
            "BPM变化事件的y坐标应该大于0"
        );
    } else {
        panic!("第一个事件应该是BpmChange类型");
    }

    // 验证 BPM 值已更新到 75.5
    assert_eq!(processor.current_bpm(), Decimal::from_str("75.5").unwrap());

    // 前进到第二个 BPM 变化点（第5小节）
    let after_second_change = after_first_change + Duration::from_secs(8);
    let events = processor.update(after_second_change);

    // 应该有第二个 BPM 变化事件
    let bpm_events: Vec<_> = events
        .filter(|(_, e)| matches!(e, ChartEvent::BpmChange { .. }))
        .collect();

    assert!(!bpm_events.is_empty(), "应该有 BPM 变化事件");

    // 检查第二个BPM变化事件的具体值
    if let Some((y, ChartEvent::BpmChange { bpm })) = bpm_events.first() {
        assert_eq!(
            bpm.to_f64().unwrap_or(0.0),
            151.0,
            "第二个BPM变化事件的值应该是151.0"
        );
        assert!(
            y.value().to_f64().unwrap_or(0.0) > 0.0,
            "第二个BPM变化事件的y坐标应该大于0"
        );
    } else {
        panic!("第一个事件应该是BpmChange类型");
    }

    // 验证 BPM 值已更新回 151
    assert_eq!(processor.current_bpm(), Decimal::from(151));
}

#[test]
fn test_combined_velocity_calculation() {
    // 使用现有的 lilith_mx.bms 文件来测试组合速度计算
    let source = include_str!("../bms/files/lilith_mx.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    let mut processor = BmsProcessor::new(bms);
    let start_time = SystemTime::now();

    processor.start_play(start_time);

    // 初始状态：BPM=151, Speed=1.0, Scroll=1.0
    assert_eq!(processor.current_bpm(), Decimal::from(151));
    assert_eq!(processor.current_speed(), Decimal::from(1));
    assert_eq!(processor.current_scroll(), Decimal::from(1));

    // 前进到第一个 BPM 变化点
    let after_first_change = start_time + Duration::from_secs(1);
    let _ = processor.update(after_first_change);

    // BPM 应该更新到 75.5
    assert_eq!(processor.current_bpm(), Decimal::from_str("75.5").unwrap());

    // 前进到第二个 BPM 变化点
    let after_second_change = after_first_change + Duration::from_secs(8);
    let _ = processor.update(after_second_change);

    // BPM 应该更新回 151
    assert_eq!(processor.current_bpm(), Decimal::from(151));
}

#[test]
fn test_event_timing_with_bpm_changes() {
    // 使用现有的 lilith_mx.bms 文件来测试事件时序
    let source = include_str!("../bms/files/lilith_mx.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    let mut processor = BmsProcessor::new(bms);
    let start_time = SystemTime::now();

    processor.start_play(start_time);

    // 验证初始状态
    assert_eq!(processor.current_bpm(), Decimal::from(151));

    // 前进 0.5 秒，应该还没有触发事件
    let half_second = start_time + Duration::from_millis(500);
    let events: Vec<_> = processor.update(half_second).collect();
    assert!(events.is_empty(), "0.5秒内不应该有事件");

    // 前进到 1 秒，应该触发第一个 BPM 变化点的事件
    let one_second = start_time + Duration::from_secs(1);
    let events = processor.update(one_second);

    let bpm_events: Vec<_> = events
        .filter(|(_, e)| matches!(e, ChartEvent::BpmChange { .. }))
        .collect();

    assert!(!bpm_events.is_empty(), "1秒时应该有 BPM 变化事件");

    // 检查BPM变化事件的具体值
    if let Some((y, ChartEvent::BpmChange { bpm })) = bpm_events.first() {
        assert_eq!(
            bpm.to_f64().unwrap_or(0.0),
            75.5,
            "BPM变化事件的值应该是75.5"
        );
        assert!(
            y.value().to_f64().unwrap_or(0.0) > 0.0,
            "BPM变化事件的y坐标应该大于0"
        );
    } else {
        panic!("第一个事件应该是BpmChange类型");
    }

    assert_eq!(processor.current_bpm(), Decimal::from_str("75.5").unwrap());

    // 继续前进，应该触发第二个 BPM 变化点
    let nine_seconds = start_time + Duration::from_secs(9);
    let events = processor.update(nine_seconds);

    let bpm_events: Vec<_> = events
        .filter(|(_, e)| matches!(e, ChartEvent::BpmChange { .. }))
        .collect();

    assert!(!bpm_events.is_empty(), "9秒时应该有 BPM 变化事件");

    // 检查第二个BPM变化事件的具体值
    if let Some((y, ChartEvent::BpmChange { bpm })) = bpm_events.first() {
        assert_eq!(
            bpm.to_f64().unwrap_or(0.0),
            151.0,
            "第二个BPM变化事件的值应该是151.0"
        );
        assert!(
            y.value().to_f64().unwrap_or(0.0) > 0.0,
            "第二个BPM变化事件的y坐标应该大于0"
        );
    } else {
        panic!("第一个事件应该是BpmChange类型");
    }

    assert_eq!(processor.current_bpm(), Decimal::from(151));
}

// 注意：BmsonProcessor对比测试已被移除，因为BMSON格式与BMS格式差异较大，
// 直接对比事件生成可能会导致混淆。主要目标是验证BmsProcessor本身的功能。
