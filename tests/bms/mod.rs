//! Tests for `bms_rs::bms`.

mod base_62;
mod comment;
mod control_flow_model;
mod cursor_with_edges;
mod diagnostics_test;
mod extra_channel;
mod files;
mod nested_random;
mod nested_switch;
mod parse_extended_tokens;
mod playing_conditions;
mod prelude_test;
mod prompt_handlers;
mod unparse_merge;
mod unparse_roundtrip;

use bms_rs::bms::prelude::*;
use pretty_assertions::assert_eq;

/// Parses the BMS source with the given RNG and asserts that the resulting objects match expectations.
pub fn test_bms_assert_objs(src: &str, rng: impl Rng, expect_objs: Vec<WavObj>) {
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(src);
    assert_eq!(lex_warnings, vec![]);

    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _, _, _>(
        &tokens,
        default_config_with_rng(rng).prompter(AlwaysUseNewer),
    );
    assert_eq!(parse_warnings, vec![]);
    let bms = match bms {
        Ok(b) => b,
        Err(e) => panic!("parse failed: {:?}", e),
    };
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        expect_objs
    );
}
