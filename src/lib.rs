//! The BMS format parser.
//!
//! Be-Music Source, called BMS for short, is a file format devised by Urao Yane in 1998 for a simulator of the game Beatmania by KONAMI. This describes what and when notes are arranged and its music metadata. It is a plain text file with some "command" lines starting with `#` character.
//! 
//! For detailed information about the BMS format and this crate, see `README.md` in [github:MikuroXina/bms-rs](https://github.com/MikuroXina/bms-rs).
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
//! For more control over the parsing process, you can use the individual steps:
//!
//! At first, you can get the tokens stream with [`bms::lex::TokenStream::parse_lex`]. Then pass it and the random generator to [`bms::parse::model::Bms::from_token_stream`] to get the notes data. Because BMS format has some randomized syntax.
//!
//! ```
//! use rand::{rngs::StdRng, SeedableRng};
//! use bms_rs::bms::prelude::*;
//!
//! let source = std::fs::read_to_string("tests/files/lilith_mx.bms").unwrap();
//! let BmsLexOutput { tokens, lex_warnings } = TokenStream::parse_lex(&source);
//! assert_eq!(lex_warnings, vec![]);
//! // You can modify the tokens before parsing, for some commands that this library does not warpped.
//! let rng = RandRng(StdRng::seed_from_u64(42));
//! let BmsParseOutput { bms, parse_warnings, playing_warnings, playing_errors } = Bms::from_token_stream(
//!     tokens.tokens(), rng, AlwaysWarnAndUseNewer
//!     );
//! // According to [BMS command memo#BEHAVIOR IN GENERAL IMPLEMENTATION](https://hitkey.bms.ms/cmds.htm#BEHAVIOR-IN-GENERAL-IMPLEMENTATION), the newer values are used for the duplicated objects.
//! assert_eq!(parse_warnings, vec![]);
//! assert_eq!(playing_warnings, vec![]);
//! assert_eq!(playing_errors, vec![]);
//! ```
//!
//! # Features
//!
//! - For supported commands, see [docs.rs#Token](https://docs.rs/bms-rs/latest/bms_rs/bms/lex/token/enum.Token.html).
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

#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod bms;
pub mod bmson;

pub use bms::{command, lex, parse};
