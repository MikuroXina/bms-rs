//! Builders for control flow structures from token reference stream.

use crate::bms::command::mixin::{MaybeWithRange, SourceRangeMixinExt};

use crate::bms::lex::{
    TokenRefStream,
    token::{Token, TokenWithRange},
};
use crate::bms::parse::{ControlFlowError, ControlFlowErrorWithRange};
use num::BigUint;

use super::{ControlFlowValue, IfBlock, NonControlToken, Random, Switch, TokenUnit};

/// Builders that construct control-flow models from token reference slices.
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
        tokens: &[&'a TokenWithRange<'a>],
        start: usize,
    ) -> Result<(Option<Self>, usize), ControlFlowErrorWithRange>;
}

/// Internal parser helper for processing a sequence of tokens into `TokenUnit`s.
///
/// This struct holds references to the token slice and a mutable cursor, enabling
/// stateful parsing that can consume tokens and advance the position. It handles
/// the common logic of identifying control flow structures (`#RANDOM`, `#SWITCH`)
/// and collecting regular tokens into `TokenUnit::Tokens`.
struct ControlFlowParser<'a, 'b> {
    /// The full slice of tokens being parsed.
    tokens: &'b [&'a TokenWithRange<'a>],
    /// Mutable reference to the current parsing position (index in `tokens`).
    cursor: &'b mut usize,
}

impl<'a, 'b> ControlFlowParser<'a, 'b> {
    /// Creates a new `ControlFlowParser`.
    ///
    /// # Arguments
    ///
    /// * `tokens` - The slice of tokens to parse.
    /// * `cursor` - A mutable reference to the current index in `tokens`.
    fn new(tokens: &'b [&'a TokenWithRange<'a>], cursor: &'b mut usize) -> Self {
        Self { tokens, cursor }
    }

    /// Parses tokens starting from the current cursor position until a stop token is encountered
    /// or the end of the stream is reached.
    ///
    /// # Arguments
    ///
    /// - `stop`: A list of header names (e.g., `["ENDIF", "ELSE"]`) that should stop the parsing loop
    ///   when encountered; these tokens are not consumed and the cursor will point to them.
    /// - `check_header`: A callback function invoked for headers that are not standard control flow
    ///   beginnings (`#RANDOM`, `#SWITCH`) or in the `stop` list. This allows custom validation
    ///   (e.g., checking for misplaced `#ELSE` or `#CASE`).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<TokenUnit<'a>>)` - The collected units (nested blocks or token groups).
    /// * `Err(ControlFlowErrorWithRange)` - If a parsing error occurs (propagated from sub-parsers or `check_header`).
    fn parse<F>(
        &mut self,
        stop: &[&str],
        mut check_header: F,
    ) -> Result<Vec<TokenUnit<'a>>, ControlFlowErrorWithRange>
    where
        F: FnMut(&TokenWithRange<'a>, &str) -> Result<(), ControlFlowErrorWithRange>,
    {
        // Parse stream into `TokenUnit`s, flattening nested control blocks
        // and avoiding deep indentation via early-continue and guard clauses.
        let mut out = Vec::new();
        let mut acc: Vec<NonControlToken<'a>> = Vec::new();

        while let Some(tok) = self.tokens.get(*self.cursor) {
            let content = tok.content();
            if let Token::Header { name, .. } = content {
                // Stop when encountering a sentinel header; do not consume it.
                if stop.iter().any(|s| name.eq_ignore_ascii_case(s)) {
                    break;
                }

                // Handle nested RANDOM/SETRANDOM.
                if name.eq_ignore_ascii_case("RANDOM") || name.eq_ignore_ascii_case("SETRANDOM") {
                    if !acc.is_empty() {
                        out.push(TokenUnit::from(std::mem::take(&mut acc)));
                    }
                    let (r_opt, next) = Random::build_from_stream(self.tokens, *self.cursor)?;
                    if let Some(r) = r_opt {
                        out.push(TokenUnit::from(r));
                    }
                    *self.cursor = next;
                    continue;
                }

                // Handle nested SWITCH/SETSWITCH.
                if name.eq_ignore_ascii_case("SWITCH") || name.eq_ignore_ascii_case("SETSWITCH") {
                    if !acc.is_empty() {
                        out.push(TokenUnit::from(std::mem::take(&mut acc)));
                    }
                    let (s_opt, next) = Switch::build_from_stream(self.tokens, *self.cursor)?;
                    if let Some(s) = s_opt {
                        out.push(TokenUnit::from(s));
                    }
                    *self.cursor = next;
                    continue;
                }

                // Custom header validation (e.g., forbid orphan ELSE/CASE).
                check_header(tok, name)?;

                // Treat as non-control token.
                acc.extend(NonControlToken::try_from_token_with_range(tok).ok());
                *self.cursor += 1;
                continue;
            }

            // Non-header token path.
            acc.extend(NonControlToken::try_from_token_with_range(tok).ok());
            *self.cursor += 1;
        }

        if !acc.is_empty() {
            out.push(TokenUnit::from(acc));
        }
        Ok(out)
    }
}

impl<'a> BuildFromStream<'a> for Random<'a> {
    fn build_from_stream(
        tokens: &[&'a TokenWithRange<'a>],
        start: usize,
    ) -> Result<(Option<Self>, usize), ControlFlowErrorWithRange> {
        // Guard: out-of-bounds start yields None with unchanged cursor.
        let Some(t0) = tokens.get(start) else {
            return Ok((None, start));
        };
        let (value_inner, mut i) = match t0.content() {
            Token::Header { name, args } if name.eq_ignore_ascii_case("RANDOM") => args
                .parse()
                .map(|max| (ControlFlowValue::GenMax(max), start + 1))
                .map_err(|_| {
                    ControlFlowError::InvalidIntegerArgument {
                        header: "RANDOM".to_string(),
                        args: args.to_string(),
                    }
                    .into_wrapper(t0)
                })?,
            Token::Header { name, args } if name.eq_ignore_ascii_case("SETRANDOM") => args
                .parse()
                .map(|val| (ControlFlowValue::Set(val), start + 1))
                .map_err(|_| {
                    ControlFlowError::InvalidIntegerArgument {
                        header: "SETRANDOM".to_string(),
                        args: args.to_string(),
                    }
                    .into_wrapper(t0)
                })?,
            _ => unreachable!(),
        };
        let value = MaybeWithRange::wrapped(value_inner.into_wrapper(t0));

        // Parse IF/ELSEIF/ELSE chains inside a RANDOM block using guard clauses
        // to avoid deep nesting. Errors are raised for orphaned headers.
        let mut branches: Vec<IfBlock<'a>> = Vec::new();
        while let Some(cur) = tokens.get(i) {
            let Token::Header { name, args } = cur.content() else {
                i += 1;
                continue;
            };

            if name.eq_ignore_ascii_case("IF") {
                let head_cond: BigUint = args.parse().map_err(|_| {
                    ControlFlowError::InvalidIntegerArgument {
                        header: "IF".to_string(),
                        args: args.to_string(),
                    }
                    .into_wrapper(cur)
                })?;
                i += 1;
                let head_units = ControlFlowParser::new(tokens, &mut i)
                    .parse(&["ELSEIF", "ELSE", "ENDIF"], |_, _| Ok(()))?;
                let mut branch = IfBlock::new_if(head_cond.into_wrapper(cur), head_units);

                // Consume chained ELSEIF/ELSE until ENDIF.
                loop {
                    if i >= tokens.len() {
                        break;
                    }
                    let cur = &tokens[i];
                    let Token::Header { name, args } = cur.content() else {
                        break;
                    };

                    if name.eq_ignore_ascii_case("ELSEIF") {
                        let cond: BigUint = args.parse().map_err(|_| {
                            ControlFlowError::InvalidIntegerArgument {
                                header: "ELSEIF".to_string(),
                                args: args.to_string(),
                            }
                            .into_wrapper(cur)
                        })?;
                        i += 1;
                        let units = ControlFlowParser::new(tokens, &mut i)
                            .parse(&["ELSEIF", "ELSE", "ENDIF"], |_, _| Ok(()))?;
                        branch = branch.or_else_if(cond.into_wrapper(cur), units);
                        continue;
                    }

                    if name.eq_ignore_ascii_case("ELSE") {
                        i += 1;
                        let units = ControlFlowParser::new(tokens, &mut i)
                            .parse(&["ENDIF"], |_, _| Ok(()))?;
                        branch = branch.or_else(units);
                        continue;
                    }

                    if name.eq_ignore_ascii_case("ENDIF") {
                        i += 1;
                        break;
                    }

                    break;
                }

                branches.push(branch);
                continue;
            }

            if name.eq_ignore_ascii_case("ELSEIF") {
                return Err(ControlFlowError::ElseIfWithoutIf.into_wrapper(cur));
            }
            if name.eq_ignore_ascii_case("ELSE") {
                return Err(ControlFlowError::ElseWithoutIfOrElseIf.into_wrapper(cur));
            }
            if name.eq_ignore_ascii_case("ENDIF") {
                return Err(ControlFlowError::EndIfWithoutIfElseIfOrElse.into_wrapper(cur));
            }
            if name.eq_ignore_ascii_case("ENDRANDOM") {
                i += 1;
                break;
            }

            i += 1;
        }

        Ok((Some(Random { value, branches }), i))
    }
}

impl<'a> BuildFromStream<'a> for Switch<'a> {
    fn build_from_stream(
        tokens: &[&'a TokenWithRange<'a>],
        start: usize,
    ) -> Result<(Option<Self>, usize), ControlFlowErrorWithRange> {
        // Guard: out-of-bounds start yields None with unchanged cursor.
        let Some(t0) = tokens.get(start) else {
            return Ok((None, start));
        };
        let (value_inner, mut i) = match t0.content() {
            Token::Header { name, args } if name.eq_ignore_ascii_case("SWITCH") => args
                .parse()
                .map(|max| (ControlFlowValue::GenMax(max), start + 1))
                .map_err(|_| {
                    ControlFlowError::InvalidIntegerArgument {
                        header: "SWITCH".to_string(),
                        args: args.to_string(),
                    }
                    .into_wrapper(t0)
                })?,
            Token::Header { name, args } if name.eq_ignore_ascii_case("SETSWITCH") => args
                .parse()
                .map(|val| (ControlFlowValue::Set(val), start + 1))
                .map_err(|_| {
                    ControlFlowError::InvalidIntegerArgument {
                        header: "SETSWITCH".to_string(),
                        args: args.to_string(),
                    }
                    .into_wrapper(t0)
                })?,
            _ => unreachable!(),
        };
        let value = MaybeWithRange::wrapped(value_inner.into_wrapper(t0));

        // Parse CASE/DEF blocks inside a SWITCH, using early-continue to keep code flat.
        let mut sw = Switch::new(value);
        while let Some(cur) = tokens.get(i) {
            let Token::Header { name, args } = cur.content() else {
                i += 1;
                continue;
            };

            if name.eq_ignore_ascii_case("CASE") {
                let cond: BigUint = args.parse().map_err(|_| {
                    ControlFlowError::InvalidIntegerArgument {
                        header: "CASE".to_string(),
                        args: args.to_string(),
                    }
                    .into_wrapper(cur)
                })?;
                i += 1;
                let units = ControlFlowParser::new(tokens, &mut i).parse(
                    &["CASE", "DEF", "ENDSW", "ENDSWITCH", "SKIP"],
                    |_, _| Ok(()),
                )?;
                let mut skip = false;
                if let Some(tok) = tokens.get(i)
                    && let Token::Header { name, .. } = tok.content()
                    && name.eq_ignore_ascii_case("SKIP")
                {
                    skip = true;
                    i += 1;
                }
                sw = if skip {
                    sw.case_with_skip(cond.into_wrapper(cur), units)
                } else {
                    sw.case_no_skip(cond.into_wrapper(cur), units)
                };
                continue;
            }

            if name.eq_ignore_ascii_case("DEF") {
                i += 1;
                let units = ControlFlowParser::new(tokens, &mut i).parse(
                    &["CASE", "DEF", "ENDSW", "ENDSWITCH", "SKIP"],
                    |_, _| Ok(()),
                )?;
                let mut skip = false;
                if let Some(tok) = tokens.get(i)
                    && let Token::Header { name, .. } = tok.content()
                    && name.eq_ignore_ascii_case("SKIP")
                {
                    skip = true;
                    i += 1;
                }
                sw = sw.def_with_skip(units, skip);
                continue;
            }

            if name.eq_ignore_ascii_case("ENDSW") || name.eq_ignore_ascii_case("ENDSWITCH") {
                i += 1;
                break;
            }

            i += 1;
        }

        Ok((Some(sw.build()), i))
    }
}

/// Scans `TokenStream` and constructs all top-level control-flow blocks.
///
/// This function walks the token list, building `Random` and `Switch` blocks
/// into `TokenUnit`s. Non-control tokens outside these blocks are ignored.
pub fn build_blocks<'a>(
    tokens: &TokenRefStream<'a>,
) -> Result<Vec<TokenUnit<'a>>, ControlFlowErrorWithRange> {
    let mut i = 0usize;
    ControlFlowParser::new(&tokens.token_refs, &mut i).parse(&[], |cur, name| {
        if name.eq_ignore_ascii_case("IF") {
            return Err(ControlFlowError::IfWithoutRandom.into_wrapper(cur));
        }
        if name.eq_ignore_ascii_case("ELSEIF") {
            return Err(ControlFlowError::ElseIfWithoutIf.into_wrapper(cur));
        }
        if name.eq_ignore_ascii_case("ELSE") {
            return Err(ControlFlowError::ElseWithoutIfOrElseIf.into_wrapper(cur));
        }
        if name.eq_ignore_ascii_case("ENDIF") {
            return Err(ControlFlowError::EndIfWithoutIfElseIfOrElse.into_wrapper(cur));
        }
        if name.eq_ignore_ascii_case("ENDRANDOM") {
            return Err(ControlFlowError::EndRandomWithoutRandom.into_wrapper(cur));
        }
        if name.eq_ignore_ascii_case("CASE") {
            return Err(ControlFlowError::CaseWithoutSwitch.into_wrapper(cur));
        }
        if name.eq_ignore_ascii_case("DEF") {
            return Err(ControlFlowError::DefWithoutSwitch.into_wrapper(cur));
        }
        if name.eq_ignore_ascii_case("SKIP") {
            return Err(ControlFlowError::SkipOutsideCaseOrDef.into_wrapper(cur));
        }
        if name.eq_ignore_ascii_case("ENDSW") || name.eq_ignore_ascii_case("ENDSWITCH") {
            return Err(ControlFlowError::EndSwitchWithoutSwitch.into_wrapper(cur));
        }
        Ok(())
    })
}
