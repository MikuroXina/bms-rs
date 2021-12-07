use std::ffi::OsStr;

use bms_rs::{command::*, parse};

#[test]
fn simple() {
    const SRC: &str = r"
#PLAYER 1
#GENRE FUGA
#TITLE BAR(^^)
#ARTIST MikuroXina
#BPM 120
#PLAYLEVEL 6
#RANK 2

#WAV01 hoge.WAV
#WAV02 foo.WAV
#WAV03 bar.WAV

#00211:0303030303

#00211:0303000303

#00211:010101
#00211:00020202
";

    let ts = parse(SRC).expect("SRC must be parsed");

    let tokens: Vec<_> = ts.into_iter().collect();
    use bms_rs::token::Token::*;
    assert_eq!(
        tokens,
        vec![
            Player(PlayerMode::Single),
            Genre("FUGA"),
            Title("BAR(^^)"),
            Artist("MikuroXina"),
            Bpm(120),
            PlayLevel(6),
            Rank(JudgeLevel::Normal),
            Wav(WavId(1.try_into().unwrap()), OsStr::new("hoge.WAV")),
            Wav(WavId(2.try_into().unwrap()), OsStr::new("foo.WAV")),
            Wav(WavId(3.try_into().unwrap()), OsStr::new("bar.WAV")),
            Message {
                track: Track(2),
                channel: Channel::Note {
                    kind: NoteKind::Visible,
                    is_player1: true,
                    key: Key::Key1,
                },
                message: "0303030303",
            },
            Message {
                track: Track(2),
                channel: Channel::Note {
                    kind: NoteKind::Visible,
                    is_player1: true,
                    key: Key::Key1,
                },
                message: "0303000303",
            },
            Message {
                track: Track(2),
                channel: Channel::Note {
                    kind: NoteKind::Visible,
                    is_player1: true,
                    key: Key::Key1,
                },
                message: "010101",
            },
            Message {
                track: Track(2),
                channel: Channel::Note {
                    kind: NoteKind::Visible,
                    is_player1: true,
                    key: Key::Key1,
                },
                message: "00020202",
            },
        ]
    );
}
