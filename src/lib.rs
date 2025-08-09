//! The BMS format parser.
//!
//! Be-Music Source, called BMS for short, is a file format devised by Urao Yane in 1998 for a simulator of the game Beatmania by KONAMI. This describes what and when notes are arranged and its music metadata. It is a plain text file with some "command" lines starting with `#` character.
//!
//! # Usage
//!
//! - **NOTE**: BMS files now is almost with Shift_JIS encoding. It's recommended to use [`encoding_rs`](https://crates.io/crates/encoding_rs) crate to parse raw file to `Cow<str>`, which is a compatible type of `&str`, using `AsRef::as_ref`.
//!
//! ## Simple Usage
//!
//! For most use cases, you can use the [`bms::parse_bms`] function to parse a BMS file in one step:
//!
//! ```
//! use bms_rs::bms::{parse_bms, BmsOutput};
//!
//! let source = std::fs::read_to_string("tests/files/lilith_mx.bms").unwrap();
//! let BmsOutput { bms, warnings } = parse_bms(&source);
//! assert_eq!(warnings, vec![]);
//! println!("Title: {}", bms.header.title.as_deref().unwrap_or("Unknown"));
//! println!("BPM: {}", bms.arrangers.bpm.unwrap_or(120.into()));
//! println!("Warnings: {:?}", warnings);
//! ```
//!
//! ## Advanced Usage
//!
//! You can call each step explicitly:
//!
//! ```
//! use bms_rs::bms::prelude::*;
//!
//! let source = "#TITLE Test\n#00111:0101";
//! // Step 1: lexing
//! let BmsLexOutput { tokens, lex_warnings } = parse_bms_step_lex(source);
//! assert!(lex_warnings.is_empty());
//!
//! // Step 2: build AST
//! let BmsAstBuildOutput { root, ast_build_warnings } = parse_bms_step_build_ast(&tokens);
//! assert!(ast_build_warnings.is_empty());
//!
//! // Step 3: parse AST with rng
//! use rand::{rngs::StdRng, SeedableRng};
//! let _rng = RandRng(StdRng::from_os_rng());
//! let rng = RandRng(StdRng::seed_from_u64(1));
//! let BmsAstParseOutput { tokens, ast_parse_warnings } = parse_bms_step_parse_ast(root, rng);
//! assert!(ast_parse_warnings.is_empty());
//!
//! // Step 4: build model with a prompt handler
//! let BmsParseOutput { bms, parse_warnings } = parse_bms_step_model(&tokens, AlwaysWarnAndUseOlder);
//! assert!(parse_warnings.is_empty());
//! ```
//!
//! You can also map key channels between modes:
//!
//! ```
//! use bms_rs::bms::prelude::*;
//!
//! // Convert from Beat mapping to PMS mapping
//! let (side, key) = convert_key_channel_between(
//!     ModeKeyChannel::Beat,
//!     ModeKeyChannel::Pms,
//!     PlayerSide::Player2,
//!     Key::Key3,
//! );
//! assert_eq!((side, key), (PlayerSide::Player1, Key::Key7));
//! ```
//!
//! # Features
//!
//! - For supported commands, see [docs.rs Token](https://docs.rs/bms-rs/latest/bms_rs/bms/lex/token/enum.Token.html).
//!
//! - For supported note channels, see [docs.rs#Channel](https://docs.rs/bms-rs/latest/bms_rs/bms/command/channel/enum.Channel.html).
//!
//! ## Default Features
//!
//! - `bmson` feature enables the BMSON format support.
//! - `serde` feature enables the `serde` support. It supports [`serde::Serialize`] for all the definications in this crate, and [`serde::Deserialize`] for all the result types.
//! - `rand` feature enables the random number generator support. It supports [`bms::parse::random::rng::RandRng`].
//!
//! ## Optional Features
//!
//! - `minor-command` feature enables the commands that are almost never used in modern BMS Players.
//!
//! # About the format
//!
//! ## Command
//!
//! Each command starts with `#` character, and other lines will be ignored. Some commands require arguments separated by whitespace character such as spaces or tabs.
//!
//! ```text
//! #PLAYER 1
//! #GENRE FUGA
//! #TITLE BAR(^^)
//! #ARTIST MikuroXina
//! #BPM 120
//! #PLAYLEVEL 6
//! #RANK 2
//!
//! #WAV01 hoge.WAV
//! #WAV02 foo.WAV
//! #WAV03 bar.WAV
//!
//! #00211:0303030303
//! ```
//!
//! ### Header command
//!
//! Header commands are used to express the metadata of the music or the definition for note arrangement.
//!
//! ### Message command
//!
//! Message command starts with `#XXXYY:ZZ...`. `XXX` is the number of the measure, `YY` is the channel of the message, and `ZZ...` is the object id sequence.
//!
//! The measure must start from 1, but some player may allow the 0 measure (i.e. Lunatic Rave 2).
//!
//! The channel commonly expresses what the lane be arranged the note to.
//!
//! The object id is formed by 2-digit of 36-radix (`[0-9a-zA-Z]`) integer. So the sequence length must be an even number. The `00` object id is the special id, expresses the rest (no object lies). The object lies on the position divided equally by how many the object is in the measure. For example:
//!
//! ```text
//! #00211:0303000303
//! ```
//!
//! This will be placed as:
//!
//! ```text
//! 003|--|--------------|
//!    |  |03            |
//!    |  |03            |
//!    |  |              |
//!    |  |03            |
//! 002|--|03------------|
//!    |  |  []  []  []  |
//!    |()|[]  []  []  []|
//!    |-----------------|
//! ```

#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod bms;
pub mod bmson;

pub use bms::{BmsOutput, BmsTokenIter, BmsWarning, command, lex, parse};
