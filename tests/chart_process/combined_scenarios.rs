use bms_rs::bms::prelude::*;
use bms_rs::chart_process::{ChartProcessor, bms_processor::BmsProcessor};
use num::ToPrimitive;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

#[test]
fn test_combined_changes_parsing() {
    // 使用现有的 lilith_mx.bms 文件来测试组合变化的解析
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

    // 验证初始 BPM
    assert_eq!(bms.arrangers.bpm, Some(Decimal::from(151)));

    // 验证各种变化的数量
    assert_eq!(bms.arrangers.bpm_changes.len(), 2);
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
        .filter(|(_, e)| matches!(e, bms_rs::chart_process::ChartEvent::BpmChange { .. }))
        .collect();

    assert!(!bpm_events.is_empty(), "应该有 BPM 变化事件");

    // 检查BPM变化事件的具体值
    if let Some((y, bms_rs::chart_process::ChartEvent::BpmChange { bpm })) = bpm_events.first() {
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
        .filter(|(_, e)| matches!(e, bms_rs::chart_process::ChartEvent::BpmChange { .. }))
        .collect();

    assert!(!bpm_events.is_empty(), "应该有 BPM 变化事件");

    // 检查第二个BPM变化事件的具体值
    if let Some((y, bms_rs::chart_process::ChartEvent::BpmChange { bpm })) = bpm_events.first() {
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
        .filter(|(_, e)| matches!(e, bms_rs::chart_process::ChartEvent::BpmChange { .. }))
        .collect();

    assert!(!bpm_events.is_empty(), "1秒时应该有 BPM 变化事件");

    // 检查BPM变化事件的具体值
    if let Some((y, bms_rs::chart_process::ChartEvent::BpmChange { bpm })) = bpm_events.first() {
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
        .filter(|(_, e)| matches!(e, bms_rs::chart_process::ChartEvent::BpmChange { .. }))
        .collect();

    assert!(!bpm_events.is_empty(), "9秒时应该有 BPM 变化事件");

    // 检查第二个BPM变化事件的具体值
    if let Some((y, bms_rs::chart_process::ChartEvent::BpmChange { bpm })) = bpm_events.first() {
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
