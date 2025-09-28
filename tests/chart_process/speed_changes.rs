use bms_rs::bms::prelude::*;
use bms_rs::chart_process::{ChartProcessor, bms_processor::BmsProcessor};
use num::ToPrimitive;
use std::time::{Duration, SystemTime};

#[test]
fn test_speed_changes() {
    // 使用现有的 bemuse_ext.bms 文件来测试 Speed 变化
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

    // 验证初始 BPM
    assert_eq!(bms.arrangers.bpm, None); // bemuse_ext.bms 没有设置初始 BPM

    // 验证 Speed 变化
    assert_eq!(bms.arrangers.speed_factor_changes.len(), 4);
    assert_eq!(bms.arrangers.scrolling_factor_changes.len(), 4);
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
        .filter(|(_, e)| matches!(e, bms_rs::chart_process::ChartEvent::SpeedChange { .. }))
        .cloned()
        .collect();

    let scroll_events: Vec<_> = all_events
        .iter()
        .filter(|(_, e)| matches!(e, bms_rs::chart_process::ChartEvent::ScrollChange { .. }))
        .cloned()
        .collect();

    assert!(!speed_events.is_empty(), "应该有 Speed 变化事件");
    assert!(!scroll_events.is_empty(), "应该有 Scroll 变化事件");

    // 检查Speed变化事件的具体值
    if let Some((y, bms_rs::chart_process::ChartEvent::SpeedChange { factor })) =
        speed_events.first()
    {
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
    if let Some((y, bms_rs::chart_process::ChartEvent::ScrollChange { factor })) =
        scroll_events.first()
    {
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
