use bms_rs::bms::{
    lex::parse,
    parse::{Bms, prompt::AlwaysUseNewer, rng::RngMock},
};

#[test]
fn test_not_base_62() {
    let ts = parse(
        r"
        #WAVaa hoge.wav
        #WAVAA fuga.wav
    ",
    )
    .expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]), AlwaysUseNewer).expect("must be parsed");
    eprintln!("{:?}", bms);
    assert_eq!(bms.header.wav_files.len(), 1);
    assert_eq!(
        bms.header.wav_files.iter().next().unwrap().1,
        &std::path::Path::new("fuga.wav").to_path_buf()
    );
}

#[test]
fn test_base_62() {
    let ts = parse(
        r"
        #WAVaa hoge.wav
        #WAVAA fuga.wav

        #BASE 62
    ",
    )
    .expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]), AlwaysUseNewer).expect("must be parsed");
    eprintln!("{:?}", bms);
    assert_eq!(bms.header.wav_files.len(), 2);
}
