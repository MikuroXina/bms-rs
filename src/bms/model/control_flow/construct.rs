//! Builders for control flow structures from `TokenStream`.

use crate::bms::command::mixin::SourceRangeMixinExt;
use crate::bms::lex::{
    TokenStream,
    token::{Token, TokenWithRange},
};
use crate::bms::parse::{ControlFlowError, ControlFlowErrorWithRange};

use super::header;
use super::{ControlFlowValue, IfBlock, NonControlToken, Random, Switch, TokenUnit};

/// Builders that construct control-flow models from token slices.
///
/// Parses control-flow blocks (`#RANDOM`/`#SWITCH`) and produces typed models.
/// Returns the constructed value and the next cursor position, or `None` when
/// construction cannot proceed (e.g., cursor out of bounds). Any syntax or
/// structural errors return `Err(ControlFlowErrorWithRange)`.
pub trait BuildFromStream<'a>: Sized {
    /// Parse from `tokens[start]` and return `(Some(value), next_index)`;
    /// return `(None, next_index)` when construction cannot proceed;
    /// on any syntax or structural error, return `Err(ControlFlowErrorWithRange)`.
    fn build_from_stream(
        tokens: &[TokenWithRange<'a>],
        start: usize,
    ) -> Result<(Option<Self>, usize), ControlFlowErrorWithRange>;
}

fn collect_units<'a>(
    tokens: &[TokenWithRange<'a>],
    i: &mut usize,
    stop: &[&str],
) -> Result<Vec<TokenUnit<'a>>, ControlFlowErrorWithRange> {
    let mut out: Vec<TokenUnit<'a>> = Vec::new();
    let mut acc: Vec<NonControlToken<'a>> = Vec::new();
    while let Some(tok) = tokens.get(*i) {
        match tok.content() {
            Token::Header { name, .. } => {
                if stop.iter().any(|s| name.eq_ignore_ascii_case(s)) {
                    break;
                }
                if name.eq_ignore_ascii_case(header::RANDOM)
                    || name.eq_ignore_ascii_case(header::SET_RANDOM)
                {
                    if !acc.is_empty() {
                        out.push(TokenUnit::from(std::mem::take(&mut acc)));
                    }
                    let (r_opt, next) = Random::build_from_stream(tokens, *i)?;
                    if let Some(r) = r_opt {
                        out.push(TokenUnit::from(r));
                    }
                    *i = next;
                    continue;
                }
                if name.eq_ignore_ascii_case(header::SWITCH)
                    || name.eq_ignore_ascii_case(header::SET_SWITCH)
                {
                    if !acc.is_empty() {
                        out.push(TokenUnit::from(std::mem::take(&mut acc)));
                    }
                    let (s_opt, next) = Switch::build_from_stream(tokens, *i)?;
                    if let Some(s) = s_opt {
                        out.push(TokenUnit::from(s));
                    }
                    *i = next;
                    continue;
                }
                let t = tok.content().clone();
                if let Ok(nc) = NonControlToken::try_from_token(t) {
                    acc.push(nc);
                }
                *i += 1;
            }
            _ => {
                let t = tok.content().clone();
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
    Ok(out)
}

impl<'a> BuildFromStream<'a> for Random<'a> {
    fn build_from_stream(
        tokens: &[TokenWithRange<'a>],
        start: usize,
    ) -> Result<(Option<Self>, usize), ControlFlowErrorWithRange> {
        let t0 = match tokens.get(start) {
            Some(t) => t,
            None => return Ok((None, start)),
        };
        let (value, mut i) = match t0.content() {
            Token::Header { name, args } if name.eq_ignore_ascii_case(header::RANDOM) => {
                match args.parse() {
                    Ok(max) => (ControlFlowValue::GenMax(max), start + 1),
                    Err(_) => {
                        return Err(ControlFlowError::InvalidIntegerArgument {
                            header: "RANDOM".to_string(),
                            args: args.to_string(),
                        }
                        .into_wrapper(t0));
                    }
                }
            }
            Token::Header { name, args } if name.eq_ignore_ascii_case(header::SET_RANDOM) => {
                match args.parse() {
                    Ok(val) => (ControlFlowValue::Set(val), start + 1),
                    Err(_) => {
                        return Err(ControlFlowError::InvalidIntegerArgument {
                            header: "SETRANDOM".to_string(),
                            args: args.to_string(),
                        }
                        .into_wrapper(t0));
                    }
                }
            }
            _ => unreachable!(),
        };

        let mut branches: Vec<IfBlock<'a>> = Vec::new();
        while let Some(cur) = tokens.get(i) {
            match cur.content() {
                Token::Header { name, args } if name.eq_ignore_ascii_case(header::IF) => {
                    let head_cond = match args.parse() {
                        Ok(v) => v,
                        Err(_) => {
                            return Err(ControlFlowError::InvalidIntegerArgument {
                                header: "IF".to_string(),
                                args: args.to_string(),
                            }
                            .into_wrapper(cur));
                        }
                    };
                    i += 1;
                    let mut units_i = i;
                    let head_units = collect_units(
                        tokens,
                        &mut units_i,
                        &[header::ELSEIF, header::ELSE, header::ENDIF],
                    )?;
                    i = units_i;
                    let mut branch = IfBlock::new_if(head_cond, head_units);
                    loop {
                        if i >= tokens.len() {
                            break;
                        }
                        let cur = &tokens[i];
                        match cur.content() {
                            Token::Header { name, args }
                                if name.eq_ignore_ascii_case(header::ELSEIF) =>
                            {
                                let cond = match args.parse() {
                                    Ok(v) => v,
                                    Err(_) => {
                                        return Err(ControlFlowError::InvalidIntegerArgument {
                                            header: "ELSEIF".to_string(),
                                            args: args.to_string(),
                                        }
                                        .into_wrapper(cur));
                                    }
                                };
                                i += 1;
                                let mut u_i = i;
                                let units = collect_units(
                                    tokens,
                                    &mut u_i,
                                    &[header::ELSEIF, header::ELSE, header::ENDIF],
                                )?;
                                i = u_i;
                                branch = branch.or_else_if(cond, units);
                            }
                            Token::Header { name, .. }
                                if name.eq_ignore_ascii_case(header::ELSE) =>
                            {
                                i += 1;
                                let mut u_i = i;
                                let units = collect_units(tokens, &mut u_i, &[header::ENDIF])?;
                                i = u_i;
                                branch = branch.or_else(units);
                            }
                            Token::Header { name, .. }
                                if name.eq_ignore_ascii_case(header::ENDIF) =>
                            {
                                i += 1;
                                break;
                            }
                            _ => break,
                        }
                    }
                    branches.push(branch);
                }
                Token::Header { name, .. } if name.eq_ignore_ascii_case(header::ELSEIF) => {
                    return Err(ControlFlowError::ElseIfWithoutIf.into_wrapper(cur));
                }
                Token::Header { name, .. } if name.eq_ignore_ascii_case(header::ELSE) => {
                    return Err(ControlFlowError::ElseWithoutIfOrElseIf.into_wrapper(cur));
                }
                Token::Header { name, .. } if name.eq_ignore_ascii_case(header::ENDIF) => {
                    return Err(ControlFlowError::EndIfWithoutIfElseIfOrElse.into_wrapper(cur));
                }
                Token::Header { name, .. } if name.eq_ignore_ascii_case(header::ENDRANDOM) => {
                    i += 1;
                    break;
                }
                _ => {
                    i += 1;
                }
            }
        }

        Ok((Some(Random { value, branches }), i))
    }
}

impl<'a> BuildFromStream<'a> for Switch<'a> {
    fn build_from_stream(
        tokens: &[TokenWithRange<'a>],
        start: usize,
    ) -> Result<(Option<Self>, usize), ControlFlowErrorWithRange> {
        let t0 = match tokens.get(start) {
            Some(t) => t,
            None => return Ok((None, start)),
        };
        let (value, mut i) = match t0.content() {
            Token::Header { name, args } if name.eq_ignore_ascii_case(header::SWITCH) => {
                match args.parse() {
                    Ok(max) => (ControlFlowValue::GenMax(max), start + 1),
                    Err(_) => {
                        return Err(ControlFlowError::InvalidIntegerArgument {
                            header: "SWITCH".to_string(),
                            args: args.to_string(),
                        }
                        .into_wrapper(t0));
                    }
                }
            }
            Token::Header { name, args } if name.eq_ignore_ascii_case(header::SET_SWITCH) => {
                match args.parse() {
                    Ok(val) => (ControlFlowValue::Set(val), start + 1),
                    Err(_) => {
                        return Err(ControlFlowError::InvalidIntegerArgument {
                            header: "SETSWITCH".to_string(),
                            args: args.to_string(),
                        }
                        .into_wrapper(t0));
                    }
                }
            }
            _ => unreachable!(),
        };

        let mut sw = Switch::new(value);
        while let Some(cur) = tokens.get(i) {
            match cur.content() {
                Token::Header { name, args } if name.eq_ignore_ascii_case(header::CASE) => {
                    let cond = match args.parse() {
                        Ok(v) => v,
                        Err(_) => {
                            return Err(ControlFlowError::InvalidIntegerArgument {
                                header: "CASE".to_string(),
                                args: args.to_string(),
                            }
                            .into_wrapper(cur));
                        }
                    };
                    i += 1;
                    let mut u_i = i;
                    let units = collect_units(
                        tokens,
                        &mut u_i,
                        &[
                            header::CASE,
                            header::DEF,
                            header::ENDSW,
                            header::ENDSWITCH,
                            header::SKIP,
                        ],
                    )?;
                    i = u_i;
                    let mut skip = false;
                    if let Some(tok) = tokens.get(i)
                        && let Token::Header { name, .. } = tok.content()
                        && name.eq_ignore_ascii_case(header::SKIP)
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
                Token::Header { name, .. } if name.eq_ignore_ascii_case(header::DEF) => {
                    i += 1;
                    let mut u_i = i;
                    let units = collect_units(
                        tokens,
                        &mut u_i,
                        &[
                            header::CASE,
                            header::DEF,
                            header::ENDSW,
                            header::ENDSWITCH,
                            header::SKIP,
                        ],
                    )?;
                    i = u_i;
                    let mut skip = false;
                    if let Some(tok) = tokens.get(i)
                        && let Token::Header { name, .. } = tok.content()
                        && name.eq_ignore_ascii_case(header::SKIP)
                    {
                        skip = true;
                        i += 1;
                    }
                    sw = sw.def_with_skip(units, skip);
                }
                Token::Header { name, .. }
                    if name.eq_ignore_ascii_case(header::ENDSW)
                        || name.eq_ignore_ascii_case(header::ENDSWITCH) =>
                {
                    i += 1;
                    break;
                }
                _ => {
                    i += 1;
                }
            }
        }

        Ok((Some(sw.build()), i))
    }
}

/// Scans `TokenStream` and constructs all top-level control-flow blocks.
///
/// This function walks the token list, building `Random` and `Switch` blocks
/// into `TokenUnit`s. Non-control tokens outside these blocks are ignored.
pub fn build_blocks<'a>(
    tokens: &TokenStream<'a>,
) -> Result<Vec<TokenUnit<'a>>, ControlFlowErrorWithRange> {
    let mut i = 0usize;
    let mut out: Vec<TokenUnit<'a>> = Vec::new();
    let mut acc: Vec<NonControlToken<'a>> = Vec::new();
    while let Some(cur) = tokens.tokens.get(i) {
        match cur.content() {
            Token::Header { name, .. }
                if name.eq_ignore_ascii_case(header::RANDOM)
                    || name.eq_ignore_ascii_case(header::SET_RANDOM) =>
            {
                if !acc.is_empty() {
                    out.push(TokenUnit::from(std::mem::take(&mut acc)));
                }
                let (r_opt, next) = Random::build_from_stream(&tokens.tokens, i)?;
                if let Some(r) = r_opt {
                    out.push(TokenUnit::from(r));
                }
                i = next;
            }
            Token::Header { name, .. }
                if name.eq_ignore_ascii_case(header::SWITCH)
                    || name.eq_ignore_ascii_case(header::SET_SWITCH) =>
            {
                if !acc.is_empty() {
                    out.push(TokenUnit::from(std::mem::take(&mut acc)));
                }
                let (s_opt, next) = Switch::build_from_stream(&tokens.tokens, i)?;
                if let Some(s) = s_opt {
                    out.push(TokenUnit::from(s));
                }
                i = next;
            }
            Token::Header { name, .. } if name.eq_ignore_ascii_case(header::IF) => {
                return Err(ControlFlowError::IfWithoutRandom.into_wrapper(cur));
            }
            Token::Header { name, .. } if name.eq_ignore_ascii_case(header::ELSEIF) => {
                return Err(ControlFlowError::ElseIfWithoutIf.into_wrapper(cur));
            }
            Token::Header { name, .. } if name.eq_ignore_ascii_case(header::ELSE) => {
                return Err(ControlFlowError::ElseWithoutIfOrElseIf.into_wrapper(cur));
            }
            Token::Header { name, .. } if name.eq_ignore_ascii_case(header::ENDIF) => {
                return Err(ControlFlowError::EndIfWithoutIfElseIfOrElse.into_wrapper(cur));
            }
            Token::Header { name, .. } if name.eq_ignore_ascii_case(header::ENDRANDOM) => {
                return Err(ControlFlowError::EndRandomWithoutRandom.into_wrapper(cur));
            }
            Token::Header { name, .. } if name.eq_ignore_ascii_case(header::CASE) => {
                return Err(ControlFlowError::CaseWithoutSwitch.into_wrapper(cur));
            }
            Token::Header { name, .. } if name.eq_ignore_ascii_case(header::DEF) => {
                return Err(ControlFlowError::DefWithoutSwitch.into_wrapper(cur));
            }
            Token::Header { name, .. } if name.eq_ignore_ascii_case(header::SKIP) => {
                return Err(ControlFlowError::SkipOutsideCaseOrDef.into_wrapper(cur));
            }
            Token::Header { name, .. }
                if name.eq_ignore_ascii_case(header::ENDSW)
                    || name.eq_ignore_ascii_case(header::ENDSWITCH) =>
            {
                return Err(ControlFlowError::EndSwitchWithoutSwitch.into_wrapper(cur));
            }
            _ => {
                let t = cur.content().clone();
                if let Ok(nc) = NonControlToken::try_from_token(t) {
                    acc.push(nc);
                }
                i += 1;
            }
        }
    }
    if !acc.is_empty() {
        out.push(TokenUnit::from(acc));
    }
    Ok(out)
}
