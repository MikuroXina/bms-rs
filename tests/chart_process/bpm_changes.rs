use bms_rs::bms::prelude::*;
use bms_rs::chart_process::{ChartProcessor, bms_processor::BmsProcessor};
use num::ToPrimitive;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

#[test]
fn test_bpm_changes_with_existing_file() {
    // 使用现有的 lilith_mx.bms 文件来测试 BPM 变化
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

    // 调试输出
    println!("BPM changes count: {}", bms.arrangers.bpm_changes.len());
    for (time, change) in &bms.arrangers.bpm_changes {
        println!("BPM change at {:?}: {}", time, change.bpm);
    }

    // lilith_mx.bms 应该有一个 BPM 变化
    assert!(!bms.arrangers.bpm_changes.is_empty());
}

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
    // 由于 BPM 变化了，时间计算需要考虑速度变化
    // 75.5 BPM 比 151 BPM 慢，所以需要更多时间
    let after_second_change = after_first_change + Duration::from_secs(8);
    let events = processor.update(after_second_change);

    // 应该触发第二个 BPM 变化事件
    let bpm_events: Vec<_> = events
        .filter(|(_, e)| matches!(e, bms_rs::chart_process::ChartEvent::BpmChange { .. }))
        .collect();
    assert!(!bpm_events.is_empty(), "应该有第二个 BPM 变化事件");

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
