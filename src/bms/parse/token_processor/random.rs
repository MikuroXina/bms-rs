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

use num::BigUint;

use crate::{
    bms::{
        error::{ParseError, ParseWarning},
        prelude::*,
    },
    parse::token_processor::all_tokens_with_range,
    util::StrExtension,
};

use super::{TokenProcessor, TokenProcessorResult};

/// It processes `#RANDOM` and `#SWITCH` control commands.
#[derive(Debug)]
pub struct RandomTokenProcessor<R, N> {
    rng: Rc<RefCell<R>>,
    /// It must not be empty.
    state_stack: RefCell<Vec<ProcessState>>,
    next: N,
    relaxed: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
enum ProcessState {
    #[default]
    Root,
    Random {
        generated: BigUint,
        activated: bool,
    },
    IfBlock {
        if_chain_has_been_activated: bool,
        activated: bool,
    },
    ElseBlock {
        activated: bool,
    },
    SwitchBeforeActive {
        generated: BigUint,
    },
    SwitchActive {
        generated: BigUint,
    },
    SwitchAfterActive {
        generated: BigUint,
    },
    SwitchSkipping,
}

impl<R, N> RandomTokenProcessor<R, N> {
    pub fn new(rng: Rc<RefCell<R>>, next: N, relaxed: bool) -> Self {
        Self {
            rng,
            state_stack: RefCell::new(vec![ProcessState::Root]),
            next,
            relaxed,
        }
    }
}

impl<R: Rng, N: TokenProcessor> RandomTokenProcessor<R, N> {
    fn visit_random(&self, args: &str) -> Result<Option<ParseWarning>, ParseError> {
        let push_new_one = || {
            let max: BigUint = match args.parse().map_err(|_| {
                ParseWarning::SyntaxError(format!("expected integer but got {args:?}"))
            }) {
                Ok(max) => max,
                Err(warning) => return Ok(Some(warning)),
            };
            let range = BigUint::from(1u64)..=max;
            let generated = self.rng.borrow_mut().generate(range.clone());
            let activated = self.is_activated();
            if activated && !range.contains(&generated) {
                return Err(ParseError::RandomGeneratedValueOutOfRange {
                    expected: range,
                    actual: generated,
                });
            }
            self.state_stack.borrow_mut().push(ProcessState::Random {
                generated,
                activated,
            });
            Ok(None)
        };
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::Root
            | ProcessState::IfBlock { .. }
            | ProcessState::ElseBlock { .. }
            | ProcessState::SwitchBeforeActive { .. }
            | ProcessState::SwitchActive { .. }
            | ProcessState::SwitchAfterActive { .. }
            | ProcessState::SwitchSkipping => push_new_one(),
            ProcessState::Random { .. } => {
                // close this scope and start new one
                self.state_stack.borrow_mut().pop();
                push_new_one()
            }
        }
    }

    fn visit_set_random(&self, args: &str) -> Result<Option<ParseWarning>, ParseError> {
        let push_new_one = || {
            let generated = match args.parse().map_err(|_| {
                ParseWarning::SyntaxError(format!("expected integer but got {args:?}"))
            }) {
                Ok(max) => max,
                Err(warning) => return Ok(Some(warning)),
            };
            let activated = self.is_activated();
            self.state_stack.borrow_mut().push(ProcessState::Random {
                generated,
                activated,
            });
            Ok(None)
        };
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::Root
            | ProcessState::IfBlock { .. }
            | ProcessState::ElseBlock { .. }
            | ProcessState::SwitchBeforeActive { .. }
            | ProcessState::SwitchActive { .. }
            | ProcessState::SwitchAfterActive { .. }
            | ProcessState::SwitchSkipping => push_new_one(),
            ProcessState::Random { .. } => {
                // close this scope and start new one
                self.state_stack.borrow_mut().pop();
                push_new_one()
            }
        }
    }

    fn visit_if(&self, args: &str) -> Result<Option<ParseWarning>, ParseError> {
        let push_new_one = |generated: BigUint| {
            let cond = match args.parse().map_err(|_| {
                ParseWarning::SyntaxError(format!("expected integer but got {args:?}"))
            }) {
                Ok(max) => max,
                Err(warning) => return Ok(Some(warning)),
            };
            let activated = generated == cond;
            self.state_stack.borrow_mut().push(ProcessState::IfBlock {
                if_chain_has_been_activated: activated,
                activated,
            });
            Ok(None)
        };
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::Random { generated, .. } => push_new_one(generated),
            ProcessState::IfBlock { .. } | ProcessState::ElseBlock { .. } => {
                // close this scope and start new one
                self.state_stack.borrow_mut().pop();
                let ProcessState::Random { generated, .. } =
                    self.state_stack.borrow().last().cloned().unwrap()
                else {
                    panic!("ElseBlock is not on Random");
                };
                push_new_one(generated)
            }
            _ => Err(ParseError::UnexpectedControlFlow(
                "#IF must be on a random scope",
            )),
        }
    }

    fn visit_else_if(&self, args: &str) -> Result<Option<ParseWarning>, ParseError> {
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::IfBlock {
                if_chain_has_been_activated,
                ..
            } => {
                self.state_stack.borrow_mut().pop();
                let ProcessState::Random { generated, .. } =
                    self.state_stack.borrow().last().cloned().unwrap()
                else {
                    panic!("IfBlock is not on Random");
                };
                if if_chain_has_been_activated {
                    self.state_stack.borrow_mut().push(ProcessState::IfBlock {
                        if_chain_has_been_activated,
                        activated: false,
                    });
                } else {
                    let cond = match args.parse().map_err(|_| {
                        ParseWarning::SyntaxError(format!("expected integer but got {args:?}"))
                    }) {
                        Ok(max) => max,
                        Err(warning) => return Ok(Some(warning)),
                    };
                    let activated = generated == cond;
                    self.state_stack.borrow_mut().push(ProcessState::IfBlock {
                        if_chain_has_been_activated: activated,
                        activated,
                    });
                }
                Ok(None)
            }
            _ => Err(ParseError::UnexpectedControlFlow(
                "#ELSEIF must come after of a #IF",
            )),
        }
    }

    fn visit_else(&self) -> Result<(), ParseError> {
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::IfBlock {
                if_chain_has_been_activated,
                ..
            } => {
                self.state_stack.borrow_mut().pop();
                self.state_stack.borrow_mut().push(ProcessState::ElseBlock {
                    activated: !if_chain_has_been_activated,
                });
                Ok(())
            }
            _ => Err(ParseError::UnexpectedControlFlow(
                "#ELSE must come after #IF or #ELSEIF",
            )),
        }
    }

    fn visit_end_if(&self) -> Result<(), ParseError> {
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::IfBlock { .. } | ProcessState::ElseBlock { .. } => {
                self.state_stack.borrow_mut().pop();
                Ok(())
            }
            _ => Err(ParseError::UnexpectedControlFlow(
                "#ENDIF must come after #IF, #ELSEIF or #ELSE",
            )),
        }
    }

    fn visit_end_random(&self) -> Result<(), ParseError> {
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::Random { .. }
            | ProcessState::IfBlock { .. }
            | ProcessState::ElseBlock { .. } => {
                self.state_stack.borrow_mut().pop();
                Ok(())
            }
            _ => Err(ParseError::UnexpectedControlFlow(
                "#ENDRANDOM must come after #RANDOM",
            )),
        }
    }

    fn visit_switch(&self, args: &str) -> Result<Option<ParseWarning>, ParseError> {
        let push_new_one = || {
            let max: BigUint = match args.parse().map_err(|_| {
                ParseWarning::SyntaxError(format!("expected integer but got {args:?}"))
            }) {
                Ok(max) => max,
                Err(warning) => return Ok(Some(warning)),
            };
            let range = BigUint::from(1u64)..=max;
            let generated = self.rng.borrow_mut().generate(range.clone());
            let activated = self.is_activated();
            if activated {
                if !range.contains(&generated) {
                    return Err(ParseError::SwitchGeneratedValueOutOfRange {
                        expected: range,
                        actual: generated,
                    });
                }
                self.state_stack
                    .borrow_mut()
                    .push(ProcessState::SwitchBeforeActive { generated });
            } else {
                self.state_stack
                    .borrow_mut()
                    .push(ProcessState::SwitchAfterActive { generated });
            }
            Ok(None)
        };
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::Random { .. } => {
                self.state_stack.borrow_mut().pop();
                push_new_one()
            }
            _ => push_new_one(),
        }
    }

    fn visit_set_switch(&self, args: &str) -> Result<Option<ParseWarning>, ParseError> {
        let push_new_one = || {
            let generated = match args.parse().map_err(|_| {
                ParseWarning::SyntaxError(format!("expected integer but got {args:?}"))
            }) {
                Ok(max) => max,
                Err(warning) => return Ok(Some(warning)),
            };
            let activated = self.is_activated();
            if activated {
                self.state_stack
                    .borrow_mut()
                    .push(ProcessState::SwitchBeforeActive { generated });
            } else {
                self.state_stack
                    .borrow_mut()
                    .push(ProcessState::SwitchAfterActive { generated });
            }
            Ok(None)
        };
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::Random { .. } => {
                self.state_stack.borrow_mut().pop();
                push_new_one()
            }
            _ => push_new_one(),
        }
    }

    fn visit_case(&self, args: &str) -> Result<Option<ParseWarning>, ParseError> {
        let cond = match args
            .parse()
            .map_err(|_| ParseWarning::SyntaxError(format!("expected integer but got {args:?}")))
        {
            Ok(max) => max,
            Err(warning) => return Ok(Some(warning)),
        };
        loop {
            let top = self.state_stack.borrow().last().cloned().unwrap();
            if let ProcessState::Random { .. }
            | ProcessState::IfBlock { .. }
            | ProcessState::ElseBlock { .. } = top
            {
                self.state_stack.borrow_mut().pop();
            } else {
                break;
            }
        }
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::SwitchBeforeActive { generated } => {
                if generated == cond {
                    self.state_stack.borrow_mut().pop();
                    self.state_stack
                        .borrow_mut()
                        .push(ProcessState::SwitchActive { generated });
                }
                Ok(None)
            }
            ProcessState::SwitchActive { generated }
            | ProcessState::SwitchAfterActive { generated } => {
                self.state_stack.borrow_mut().pop();
                if generated == cond {
                    self.state_stack
                        .borrow_mut()
                        .push(ProcessState::SwitchActive { generated });
                } else {
                    self.state_stack
                        .borrow_mut()
                        .push(ProcessState::SwitchAfterActive { generated });
                }
                Ok(None)
            }
            ProcessState::SwitchSkipping => Ok(None),
            _ => Err(ParseError::UnexpectedControlFlow(
                "#CASE must be on a switch block",
            )),
        }
    }

    fn visit_skip(&self) -> Result<(), ParseError> {
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::SwitchActive { .. } => {
                self.state_stack.borrow_mut().pop();
                self.state_stack
                    .borrow_mut()
                    .push(ProcessState::SwitchSkipping);
                Ok(())
            }
            ProcessState::SwitchBeforeActive { .. }
            | ProcessState::SwitchAfterActive { .. }
            | ProcessState::SwitchSkipping => Ok(()),
            _ => Err(ParseError::UnexpectedControlFlow(
                "#SKIP must be on a switch block",
            )),
        }
    }

    fn visit_default(&self) -> Result<(), ParseError> {
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::SwitchBeforeActive { generated } => {
                self.state_stack.borrow_mut().pop();
                self.state_stack
                    .borrow_mut()
                    .push(ProcessState::SwitchActive { generated });
                Ok(())
            }
            ProcessState::SwitchActive { generated } => {
                self.state_stack.borrow_mut().pop();
                self.state_stack
                    .borrow_mut()
                    .push(ProcessState::SwitchAfterActive { generated });
                Ok(())
            }
            ProcessState::SwitchAfterActive { .. } | ProcessState::SwitchSkipping => Ok(()),
            _ => Err(ParseError::UnexpectedControlFlow(
                "#DEF must be on a switch block",
            )),
        }
    }

    fn visit_end_switch(&self) -> Result<(), ParseError> {
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::SwitchBeforeActive { .. }
            | ProcessState::SwitchActive { .. }
            | ProcessState::SwitchAfterActive { .. }
            | ProcessState::SwitchSkipping => {
                self.state_stack.borrow_mut().pop();
                Ok(())
            }
            _ => Err(ParseError::UnexpectedControlFlow(
                "#ENDSWITCH must come after #SWITCH",
            )),
        }
    }

    fn is_activated(&self) -> bool {
        self.state_stack.borrow().iter().all(|state| {
            matches!(
                state,
                ProcessState::Root
                    | ProcessState::Random {
                        activated: true,
                        ..
                    }
                    | ProcessState::IfBlock {
                        activated: true,
                        ..
                    }
                    | ProcessState::ElseBlock { activated: true }
                    | ProcessState::SwitchActive { .. }
            )
        })
    }
}

impl<R: Rng, N: TokenProcessor> TokenProcessor for RandomTokenProcessor<R, N> {
    type Output = <N as TokenProcessor>::Output;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> TokenProcessorResult<Self::Output> {
        let mut activated = vec![];
        all_tokens_with_range(input, prompter, |token| {
            let res = match token.content() {
                Token::Header { name, args } => self.on_header(name.as_ref(), args.as_ref())?,
                Token::Message { .. } => None,
                Token::NotACommand(line) => self.on_comment(line)?,
            };
            if self.is_activated() {
                activated.push(token);
            }
            Ok(res)
        })?;
        self.next.process(&mut &activated[..], prompter)
    }
}

impl<R: Rng, N: TokenProcessor> RandomTokenProcessor<R, N> {
    fn on_header(&self, name: &str, args: &str) -> Result<Option<ParseWarning>, ParseError> {
        if self.relaxed {
            if name.eq_ignore_ascii_case("RONDAM") {
                return self.visit_random(args);
            }
            if name.eq_ignore_ascii_case("END") && args.eq_ignore_ascii_case("IF") {
                return self.visit_end_if().map(|_| None);
            }
            if let Some(wrong_arg) = name.strip_prefix_ignore_case("RANDOM") {
                return self.visit_random(wrong_arg);
            }
            if let Some(wrong_arg) = name.strip_prefix_ignore_case("IF") {
                return self.visit_if(wrong_arg);
            }
        }

        if name.eq_ignore_ascii_case("RANDOM") {
            return self.visit_random(args);
        }
        if name.eq_ignore_ascii_case("SETRANDOM") {
            return self.visit_set_random(args);
        }
        if name.eq_ignore_ascii_case("IF") {
            return self.visit_if(args);
        }
        if name.eq_ignore_ascii_case("ELSEIF") {
            return self.visit_else_if(args);
        }
        if name.eq_ignore_ascii_case("ELSE") {
            return self.visit_else().map(|_| None);
        }
        if name.eq_ignore_ascii_case("ENDIF") {
            return self.visit_end_if().map(|_| None);
        }
        if name.eq_ignore_ascii_case("ENDRANDOM") {
            return self.visit_end_random().map(|_| None);
        }
        if name.eq_ignore_ascii_case("SWITCH") {
            return self.visit_switch(args);
        }
        if name.eq_ignore_ascii_case("SETSWITCH") {
            return self.visit_set_switch(args);
        }
        if name.eq_ignore_ascii_case("CASE") {
            return self.visit_case(args);
        }
        if name.eq_ignore_ascii_case("SKIP") {
            return self.visit_skip().map(|_| None);
        }
        if name.eq_ignore_ascii_case("DEF") {
            return self.visit_default().map(|_| None);
        }
        if name.eq_ignore_ascii_case("ENDSW") {
            return self.visit_end_switch().map(|_| None);
        }
        Ok(None)
    }

    fn on_comment(&self, line: &str) -> Result<Option<ParseWarning>, ParseError> {
        if self.relaxed && line.trim().eq_ignore_ascii_case("＃ENDIF") {
            self.visit_end_if()?;
        }
        Ok(None)
    }
}
