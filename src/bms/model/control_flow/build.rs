//! Builders for control flow structures from `TokenStream`.
use num::BigUint;

use crate::bms::lex::{
    TokenStream,
    token::{Token, TokenWithRange},
};

use super::{ControlFlowValue, IfBlock, NonControlToken, Random, Switch, TokenUnit};

/// Build a control-flow model from a slice of tokens starting at `start`.
///
/// Implementors parse tokens beginning at `start` and return the constructed
/// value alongside the next index after the parsed block.
pub trait BuildFromStream<'a>: Sized {
    /// Build the value from `tokens`, starting at `start`, returning `(value, next_index)`.
    fn build_from_stream(tokens: &[TokenWithRange<'a>], start: usize) -> (Self, usize);
}

fn collect_units<'a>(
    tokens: &[TokenWithRange<'a>],
    i: &mut usize,
    stop: &[&str],
) -> Vec<TokenUnit<'a>> {
    let mut out: Vec<TokenUnit<'a>> = Vec::new();
    let mut acc: Vec<NonControlToken<'a>> = Vec::new();
    loop {
        if *i >= tokens.len() {
            break;
        }
        match tokens[*i].content() {
            Token::Header { name, .. } => {
                if stop.iter().any(|s| name.eq_ignore_ascii_case(s)) {
                    break;
                }
                if name.eq_ignore_ascii_case("RANDOM") || name.eq_ignore_ascii_case("SETRANDOM") {
                    if !acc.is_empty() {
                        out.push(TokenUnit::from(acc));
                        acc = Vec::new();
                    }
                    let (r, next) = Random::build_from_stream(tokens, *i);
                    out.push(TokenUnit::from(r));
                    *i = next;
                    continue;
                }
                if name.eq_ignore_ascii_case("SWITCH") || name.eq_ignore_ascii_case("SETSWITCH") {
                    if !acc.is_empty() {
                        out.push(TokenUnit::from(acc));
                        acc = Vec::new();
                    }
                    let (s, next) = Switch::build_from_stream(tokens, *i);
                    out.push(TokenUnit::from(s));
                    *i = next;
                    continue;
                }
                let t = tokens[*i].content().clone();
                if let Ok(nc) = NonControlToken::try_from_token(t) {
                    acc.push(nc);
                }
                *i += 1;
            }
            _ => {
                let t = tokens[*i].content().clone();
                if let Ok(nc) = NonControlToken::try_from_token(t) {
                    acc.push(nc);
                }
                *i += 1;
            }
        }
    }
    if !acc.is_empty() {
        out.push(TokenUnit::from(acc));
    }
    out
}

impl<'a> BuildFromStream<'a> for Random<'a> {
    fn build_from_stream(tokens: &[TokenWithRange<'a>], start: usize) -> (Self, usize) {
        let (value, mut i) = match tokens[start].content() {
            Token::Header { name, args } if name.eq_ignore_ascii_case("RANDOM") => {
                let max: BigUint = args.parse().unwrap_or_else(|_| BigUint::from(1u64));
                (ControlFlowValue::GenMax(max), start + 1)
            }
            Token::Header { name, args } if name.eq_ignore_ascii_case("SETRANDOM") => {
                let val: BigUint = args.parse().unwrap_or_else(|_| BigUint::from(1u64));
                (ControlFlowValue::Set(val), start + 1)
            }
            _ => unreachable!(),
        };

        let mut branches: Vec<IfBlock<'a>> = Vec::new();
        while i < tokens.len() {
            match tokens[i].content() {
                Token::Header { name, args } if name.eq_ignore_ascii_case("IF") => {
                    let head_cond: BigUint = args.parse().unwrap_or_else(|_| BigUint::from(0u64));
                    i += 1;
                    let mut units_i = i;
                    let head_units =
                        collect_units(tokens, &mut units_i, &["ELSEIF", "ELSE", "ENDIF"]);
                    i = units_i;
                    let mut branch = IfBlock::new_if(head_cond, head_units);
                    loop {
                        if i >= tokens.len() {
                            break;
                        }
                        match tokens[i].content() {
                            Token::Header { name, args } if name.eq_ignore_ascii_case("ELSEIF") => {
                                let cond: BigUint =
                                    args.parse().unwrap_or_else(|_| BigUint::from(0u64));
                                i += 1;
                                let mut u_i = i;
                                let units =
                                    collect_units(tokens, &mut u_i, &["ELSEIF", "ELSE", "ENDIF"]);
                                i = u_i;
                                branch = branch.or_else_if(cond, units);
                            }
                            Token::Header { name, .. } if name.eq_ignore_ascii_case("ELSE") => {
                                i += 1;
                                let mut u_i = i;
                                let units = collect_units(tokens, &mut u_i, &["ENDIF"]);
                                i = u_i;
                                branch = branch.or_else(units);
                            }
                            Token::Header { name, .. } if name.eq_ignore_ascii_case("ENDIF") => {
                                i += 1;
                                break;
                            }
                            _ => break,
                        }
                    }
                    branches.push(branch);
                }
                Token::Header { name, .. } if name.eq_ignore_ascii_case("ENDRANDOM") => {
                    i += 1;
                    break;
                }
                _ => {
                    i += 1;
                }
            }
        }

        (Random { value, branches }, i)
    }
}

impl<'a> BuildFromStream<'a> for Switch<'a> {
    fn build_from_stream(tokens: &[TokenWithRange<'a>], start: usize) -> (Self, usize) {
        let (value, mut i) = match tokens[start].content() {
            Token::Header { name, args } if name.eq_ignore_ascii_case("SWITCH") => {
                let max: BigUint = args.parse().unwrap_or_else(|_| BigUint::from(1u64));
                (ControlFlowValue::GenMax(max), start + 1)
            }
            Token::Header { name, args } if name.eq_ignore_ascii_case("SETSWITCH") => {
                let val: BigUint = args.parse().unwrap_or_else(|_| BigUint::from(1u64));
                (ControlFlowValue::Set(val), start + 1)
            }
            _ => unreachable!(),
        };

        let mut sw = Switch::new(value);
        while i < tokens.len() {
            match tokens[i].content() {
                Token::Header { name, args } if name.eq_ignore_ascii_case("CASE") => {
                    let cond: BigUint = args.parse().unwrap_or_else(|_| BigUint::from(0u64));
                    i += 1;
                    let mut u_i = i;
                    let units = collect_units(
                        tokens,
                        &mut u_i,
                        &["CASE", "DEF", "ENDSW", "ENDSWITCH", "SKIP"],
                    );
                    i = u_i;
                    let mut skip = false;
                    if i < tokens.len()
                        && let Token::Header { name, .. } = tokens[i].content()
                        && name.eq_ignore_ascii_case("SKIP")
                    {
                        skip = true;
                        i += 1;
                    }
                    sw = if skip {
                        sw.case_with_skip(cond, units)
                    } else {
                        sw.case_no_skip(cond, units)
                    };
                }
                Token::Header { name, .. } if name.eq_ignore_ascii_case("DEF") => {
                    i += 1;
                    let mut u_i = i;
                    let units = collect_units(
                        tokens,
                        &mut u_i,
                        &["CASE", "DEF", "ENDSW", "ENDSWITCH", "SKIP"],
                    );
                    i = u_i;
                    let mut skip = false;
                    if i < tokens.len()
                        && let Token::Header { name, .. } = tokens[i].content()
                        && name.eq_ignore_ascii_case("SKIP")
                    {
                        skip = true;
                        i += 1;
                    }
                    sw = sw.def_with_skip(units, skip);
                }
                Token::Header { name, .. }
                    if name.eq_ignore_ascii_case("ENDSW")
                        || name.eq_ignore_ascii_case("ENDSWITCH") =>
                {
                    i += 1;
                    break;
                }
                _ => {
                    i += 1;
                }
            }
        }

        (sw.build(), i)
    }
}

#[must_use]
/// Scans `TokenStream` and constructs all top-level control-flow blocks.
///
/// This function walks the token list, building `Random` and `Switch` blocks
/// into `TokenUnit`s. Non-control tokens outside these blocks are ignored.
pub fn build_blocks<'a>(tokens: &TokenStream<'a>) -> Vec<TokenUnit<'a>> {
    let mut i = 0usize;
    let mut out: Vec<TokenUnit<'a>> = Vec::new();
    while i < tokens.tokens.len() {
        match tokens.tokens[i].content() {
            Token::Header { name, .. }
                if name.eq_ignore_ascii_case("RANDOM")
                    || name.eq_ignore_ascii_case("SETRANDOM") =>
            {
                let (r, next) = Random::build_from_stream(&tokens.tokens, i);
                out.push(TokenUnit::from(r));
                i = next;
            }
            Token::Header { name, .. }
                if name.eq_ignore_ascii_case("SWITCH")
                    || name.eq_ignore_ascii_case("SETSWITCH") =>
            {
                let (s, next) = Switch::build_from_stream(&tokens.tokens, i);
                out.push(TokenUnit::from(s));
                i = next;
            }
            _ => {
                i += 1;
            }
        }
    }
    out
}
