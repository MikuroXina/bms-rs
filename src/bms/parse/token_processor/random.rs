//! This module handle tokens:
//!
//! - `#RANDOM` - Starts a random scope which can contain only `#IF`-`#ENDIF` scopes. The random scope must close with `#ENDRANDOM`. A random integer from 1 to the integer will be generated when parsing the score. Then if the integer of `#IF` equals to the random integer, the commands in an if scope will be parsed, otherwise all command in it will be ignored. Any command except `#IF` and `#ENDIF` must not be included in the scope, but some players allow it.
//! - `#SETRANDOM` - Starts a random scope but the integer will be used as the generated random number. It should be used only for tests.
//! - `#IF` - Starts an if scope when the integer equals to the generated random number. This must be placed in a random scope. See also [`Token::Random`].
//! - `#ELSEIF` - Starts an if scope when the integer equals to the generated random number. It must be in an if scope. If preceding `#IF` had matched to the generated, this scope don't start.
//! - `#ELSE` - Starts an if scope when the preceding `#IF` had not matched to the generated random number. It must be in an if scope.
//! - `#ENDIF` - Closes the if scope.
//! - `#ENDRANDOM` - Closes the random scope.
//! - `#SWITCH` - Starts a switch scope which can contain only `#CASE` or `#DEF` scopes. The switch scope must close with `#ENDSW`. A random integer from 1 to the integer will be generated when parsing the score. Then if the integer of `#CASE` equals to the random integer, the commands in a case scope will be parsed, otherwise all command in it will be ignored. Any command except `#CASE` and `#DEF` must not be included in the scope, but some players allow it.
//! - `#SETSWITCH` - Starts a switch scope but the integer will be used as the generated random number. It should be used only for tests.
//! - `#CASE` - Starts a case scope if the integer equals to the generated random number. If there's no `#SKIP` command in the scope, the command control flow will **fallthrough** to the next `#CASE` or `#DEF`.
//! - `#SKIP` - Escapes the current switch scope. It is often used in the end of every case scope.
//! - `#DEF` - Starts a case scope if any `#CASE` had not matched to the generated random number. It must be placed in the end of the switch scope, otherwise the following cases are ignored.
//! - `#ENDSW` - Closes the random scope.
//!
//! And with a relaxed flag:
//!
//! - `#RONDAM` - Type of `#RANDOM`.
//! - `＃ENDIF` - Full width `#` typo of `#ENDIF`.
//! - `#END IF` - Type of `#ENDIF`.
//! - `#RANDOM[n]` - `#RANDOM` and args without spaces.
//! - `#IF[n]` - `#IF` and args without spaces.
//!
//! ## Development note
//!
//! The state transition table about transiting from stack top state and token to the operation here:
//!
//! | token \ state | `Root` | `Random` | `IfBlock` | `ElseBlock` | `SwitchBeforeActive` | `SwitchActive` | `SwitchAfterActive` | `SwitchSkipping` |
//! | --: | -- | -- | -- | -- | -- | -- | -- | -- |
//! | `RANDOM`, `SETRANDOM` | push `Random` | pop -> push `Random` | push `Random` | push `Random` | push `Random` | push `Random` | push `Random` | push `Random` |
//! | `IF` | error | push `IfBlock` | pop -> push `IfBlock` | pop -> push `IfBlock` | error | error | error | error |
//! | `ELSEIF` | error | error | pop -> push `IfBlock` | error | error | error | error | error |
//! | `ELSE` | error | error | pop -> push `ElseBlock` | error | error | error | error | error |
//! | `ENDIF` | error | error | pop | pop | error | error | error | error |
//! | `ENDRANDOM` | error | pop | pop | pop | error | error | error | error |
//! | `SWITCH`, `SETSWITCH` | push `SwitchBeforeActive` | pop -> push `SwitchBeforeActive` | push `SwitchBeforeActive` | push `SwitchBeforeActive` | push `SwitchBeforeActive` | push `SwitchBeforeActive` | push `SwitchBeforeActive` | push `SwitchBeforeActive` |
//! | `CASE` | error | pop until `RandomBlock`, `IfBlock`, or `ElseBlock` -> parse again | same to left | same to left | same to left | pop -> push `SwitchActive` if matches generated else `SwitchAfterActive` | pop -> push `SwitchActive` if matches generated else `SwitchAfterActive` | ignore |
//! | `SKIP` | error | error | error | error | ignore | pop -> push `SwitchSkipping` | ignore | ignore |
//! | `DEF` | error | error | error | error | pop -> push `SwitchActive` | pop -> push `SwitchAfterActive` | ignore | ignore |
//! | `ENDSW` | error | error | error | error | pop | pop | pop | pop |
//! | others | call next | error | call next if activated | call next if activated | ignore | call next | ignore | ignore |

use std::{cell::RefCell, rc::Rc};

use crate::bms::prelude::*;

use super::{ProcessContext, TokenProcessor};

/// It processes `#RANDOM` and `#SWITCH` control commands.
#[derive(Debug)]
pub struct RandomTokenProcessor<R, N> {
    rng: Rc<RefCell<R>>,
    next: N,
}

impl<R, N> RandomTokenProcessor<R, N> {
    pub fn new(rng: Rc<RefCell<R>>, next: N) -> Self {
        Self { rng, next }
    }
}

impl<R: Rng, N: TokenProcessor> TokenProcessor for RandomTokenProcessor<R, N> {
    type Output = <N as TokenProcessor>::Output;

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> Result<Self::Output, crate::bms::parse::ParseErrorWithRange> {
        let checkpoint = ctx.save();
        let mut owned: Vec<TokenWithRange<'t>> = Vec::new();
        ctx.all_tokens(|token, _| {
            owned.push(token.clone());
            Ok(None)
        })?;

        let stream = TokenStream { tokens: owned };
        let (units, mut warns) = crate::bms::model::control_flow::build::build_blocks(&stream)?;
        let mut activated_owned: Vec<TokenWithRange<'t>> = Vec::new();
        {
            let mut rng = self.rng.borrow_mut();
            for u in units {
                let (tokens, w) =
                    crate::bms::model::control_flow::activate::Activate::activate(u, &mut *rng)?;
                warns.extend(w);
                activated_owned.extend(tokens);
            }
        }
        ctx.reported.extend(warns);

        let activated_refs: Vec<&TokenWithRange<'t>> = activated_owned.iter().collect();
        let mut tmp = &activated_refs[..];
        let mut view_ctx = ProcessContext {
            input: &mut tmp,
            prompter: ctx.prompter(),
            reported: Vec::new(),
        };
        let out = self.next.process(&mut view_ctx)?;
        ctx.reported.extend(view_ctx.into_warnings());
        ctx.restore(checkpoint);
        Ok(out)
    }
}
