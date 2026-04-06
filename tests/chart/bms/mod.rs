//! Integration tests for `bms_rs::chart::BmsProcessor`.

mod chart;
mod playback_state;
mod section;
mod visible_events;

use bms_rs::bms::prelude::*;

use super::{assert_time_close, MICROSECOND_EPSILON};

/// Parse BMS source and return the BMS struct, asserting no warnings.
///
/// # Panics
///
/// Panics if there are any lex or parse warnings.
pub fn parse_bms_no_warnings<T, P, R, M>(source: &str, config: ParseConfig<T, P, R, M>) -> Bms
where
    T: KeyLayoutMapper,
    P: Prompter,
    R: Rng,
    M: TokenModifier,
{
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);

    let ParseOutput {
        bms: bms_res,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, config);
    assert_eq!(parse_warnings, vec![]);
    bms_res.expect("Failed to parse BMS in test setup")
}
