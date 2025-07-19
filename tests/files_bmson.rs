use bms_rs::bmson::Bmson;
use std::fs;

#[test]
fn test_bmson_parse() {
    let path = "tests/lostokens.bmson";
    let data = fs::read_to_string(path).expect("failed to read lostokens.bmson");
    let bmson: Bmson = serde_json::from_str(&data).expect("failed to parse bmson json");
    // 基本字段断言
    assert_eq!(bmson.info.title, "lostokens");
    assert_eq!(bmson.info.level, 5);
    assert!(!bmson.sound_channels.is_empty());
}
