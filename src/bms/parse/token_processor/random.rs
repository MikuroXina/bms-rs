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
//! - `#DEF` - Starts a case scope if any `#CASE` had not matched to the generated random number. It must be placed in the end of the switch scope.
//! - `#ENDSW` - Closes the random scope.
//!
//! And with a relaxed flag:
//!
//! - `#RONDAM` - Type of `#RANDOM`.
//! - `＃ENDIF` - Full width `#` typo of `#ENDIF`.
//! - `#RANDOM[n]` - `#RANDOM` and args without spaces.
//! - `#IF[n]` - `#IF` and args without spaces.
//!
//! ## Development note
//!
//! The state transition table about transiting from stack top state and token to the operation here:
//!
//! | token \ state | `Root` | `Random` | `IfBlock` | `ElseBlock` | `SwitchBeforeActive` | `SwitchActive` | `SwitchAfterActive` | `SwitchSkipping` |
//! | --: | -- | -- | -- | -- | -- | -- | -- | -- |
//! | `RANDOM`, `SETRANDOM` | push `Random` | pop -> push `Random` | push `Random` | push `Random` | ignore | push `Random` | ignore | ignore |
//! | `IF` | error | push `IfBlock` | pop -> push `IfBlock` | pop -> push `IfBlock` | error | error | error | error |
//! | `ELSEIF` | error | error | pop -> push `IfBlock` | error | error | error | error | error |
//! | `ELSE` | error | error | pop -> push `ElseBlock` | error | error | error | error | error |
//! | `ENDIF` | error | error | pop | pop | error | error | error | error |
//! | `ENDRANDOM` | error | pop | pop | pop | error | error | error | error |
//! | `SWITCH`, `SETSWITCH` | push `SwitchBeforeActive` | pop -> push `SwitchBeforeActive` | push `SwitchBeforeActive` | push `SwitchBeforeActive` | push `SwitchBeforeActive` | push `SwitchBeforeActive` | push `SwitchBeforeActive` | push `SwitchBeforeActive` |
//! | `CASE` | error | error | error | error | pop -> push `SwitchActive` if matches generated else `SwitchBeforeActive` | pop -> push `SwitchActive` if matches generated else `SwitchAfterActive` | pop -> push `SwitchActive` if matches generated else `SwitchAfterActive` | ignore |
//! | `SKIP` | error | error | error | error | ignore | pop -> push `SwitchSkipping` | ignore | ignore |
//! | `DEF` | error | error | error | error | pop -> push `SwitchActive` | pop -> push `SwitchAfterActive` | ignore | ignore |
//! | `ENDSW` | error | error | error | error | pop | pop | pop | pop |
//! | others | call next | error | call next if activated | call next if activated | ignore | call next | ignore | ignore |

use std::{cell::RefCell, rc::Rc};

use num::BigUint;

use crate::{
    bms::prelude::*,
    parse::{ParseError, ParseWarning},
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
            if !range.contains(&generated) {
                return Err(ParseError::RandomGeneratedValueOutOfRange {
                    expected: range,
                    actual: generated,
                });
            }
            self.state_stack
                .borrow_mut()
                .push(ProcessState::Random { generated });
            Ok(None)
        };
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::Root
            | ProcessState::IfBlock { .. }
            | ProcessState::ElseBlock { .. }
            | ProcessState::SwitchActive { .. } => push_new_one(),
            ProcessState::Random { .. } => {
                // close this scope and start new one
                self.state_stack.borrow_mut().pop();
                push_new_one()
            }
            _ => Ok(None),
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
            self.state_stack
                .borrow_mut()
                .push(ProcessState::Random { generated });
            Ok(None)
        };
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::Root
            | ProcessState::IfBlock { .. }
            | ProcessState::ElseBlock { .. }
            | ProcessState::SwitchActive { .. } => push_new_one(),
            ProcessState::Random { .. } => {
                // close this scope and start new one
                self.state_stack.borrow_mut().pop();
                push_new_one()
            }
            _ => Ok(None),
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
            ProcessState::Random { generated } => push_new_one(generated),
            ProcessState::IfBlock { .. } | ProcessState::ElseBlock { .. } => {
                // close this scope and start new one
                self.state_stack.borrow_mut().pop();
                let ProcessState::Random { generated } =
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
                let ProcessState::Random { generated } =
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
            if !range.contains(&generated) {
                return Err(ParseError::SwitchGeneratedValueOutOfRange {
                    expected: range,
                    actual: generated,
                });
            }
            self.state_stack
                .borrow_mut()
                .push(ProcessState::SwitchBeforeActive { generated });
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
            self.state_stack
                .borrow_mut()
                .push(ProcessState::SwitchBeforeActive { generated });
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
        dbg!(self.state_stack.borrow());
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

    fn visit_others(&self, token: &TokenWithRange<'_>) -> TokenProcessorResult {
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::Root
            | ProcessState::IfBlock {
                activated: true, ..
            }
            | ProcessState::ElseBlock { activated: true }
            | ProcessState::SwitchActive { .. } => {
                let mut slice = std::slice::from_ref(token);
                self.next.process(&mut slice)
            }
            ProcessState::Random { .. } => Err(ParseError::UnexpectedControlFlow(
                "non-control flow tokens must not be on a random scope",
            )
            .into_wrapper(token)),
            _ => Ok(vec![]),
        }
    }
}

impl<R: Rng, N: TokenProcessor> TokenProcessor for RandomTokenProcessor<R, N> {
    fn process(&self, input: &mut &[TokenWithRange<'_>]) -> TokenProcessorResult {
        let mut warnings = vec![];
        for token in &**input {
            match token.content() {
                Token::Header { name, args } => {
                    warnings.extend(self.on_header(name.as_ref(), args.as_ref(), token)?);
                }
                Token::Message { .. } => {
                    let mut slice = std::slice::from_ref(token);
                    warnings.extend(self.next.process(&mut slice)?);
                }
                Token::NotACommand(line) => {
                    warnings.extend(
                        self.on_comment(line)
                            .map_err(|err| err.into_wrapper(token))?
                            .map(|warning| warning.into_wrapper(token)),
                    );
                }
            }
        }
        *input = &[];
        Ok(warnings)
    }
}

impl<R: Rng, N: TokenProcessor> RandomTokenProcessor<R, N> {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        token: &TokenWithRange<'_>,
    ) -> TokenProcessorResult {
        let upper_name = name.to_ascii_uppercase();
        let mut warnings = vec![];
        if self.relaxed {
            match upper_name.as_str() {
                "RONDAM" => {
                    warnings.extend(
                        self.visit_random(args)
                            .map_err(|err| err.into_wrapper(token))?,
                    );
                }
                upper_name
                    if upper_name.starts_with("RANDOM") && upper_name.len() > "RANDOM".len() =>
                {
                    warnings.extend(
                        self.visit_random(upper_name.trim_start_matches("RANDOM"))
                            .map_err(|err| err.into_wrapper(token))?,
                    );
                }
                upper_name if upper_name.starts_with("IF") && upper_name.len() > "IF".len() => {
                    warnings.extend(
                        self.visit_if(upper_name.trim_start_matches("IF"))
                            .map_err(|err| err.into_wrapper(token))?,
                    );
                }
                _ => {}
            }
        }
        Ok(warnings
            .into_iter()
            .chain(
                match upper_name.as_str() {
                    "RANDOM" => self.visit_random(args),
                    "SETRANDOM" => self.visit_set_random(args),
                    "IF" => self.visit_if(args),
                    "ELSEIF" => self.visit_else_if(args),
                    "ELSE" => self.visit_else().map(|_| None),
                    "ENDIF" => self.visit_else_if(args),
                    "ENDRANDOM" => self.visit_end_random().map(|_| None),
                    "SWITCH" => self.visit_switch(args),
                    "SETSWITCH" => self.visit_set_switch(args),
                    "CASE" => self.visit_case(args),
                    "SKIP" => self.visit_skip().map(|_| None),
                    "DEF" => self.visit_default().map(|_| None),
                    "ENDSW" => self.visit_end_switch().map(|_| None),
                    _ => return self.visit_others(token),
                }
                .map_err(|err| err.into_wrapper(token))?,
            )
            .map(|warning| warning.into_wrapper(token))
            .collect::<Vec<_>>())
    }

    fn on_comment(&self, line: &str) -> Result<Option<ParseWarning>, ParseError> {
        if self.relaxed && line.trim().eq_ignore_ascii_case("＃ENDIF") {
            self.visit_end_if()?;
        }
        Ok(None)
    }
}
