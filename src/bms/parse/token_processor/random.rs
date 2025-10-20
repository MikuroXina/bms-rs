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
//! | `RANDOM`, `SETRANDOM` | push `Random` | pop -> push `Random` | push `Random` | push `Random` | error | error | error | error |
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
    ast::rng::Rng,
    bms::prelude::*,
    parse::{ParseWarning, Result},
};

use super::TokenProcessor;

/// It processes `#RANDOM` and `#SWITCH` control commands.
#[derive(Debug)]
pub struct RandomTokenProcessor<'a, P, R, N> {
    prompter: &'a P,
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

impl<'a, P, R, N> RandomTokenProcessor<'a, P, R, N> {
    pub fn new(prompter: &'a P, rng: Rc<RefCell<R>>, next: N, relaxed: bool) -> Self {
        Self {
            prompter,
            rng,
            state_stack: RefCell::new(vec![ProcessState::Root]),
            next,
            relaxed,
        }
    }
}

impl<P: Prompter, R: Rng, N: TokenProcessor> RandomTokenProcessor<'_, P, R, N> {
    fn visit_random(&self, args: &str) -> Result<()> {
        let push_new_one = || {
            let max: BigUint = args.trim().parse().map_err(|_| {
                ParseWarning::SyntaxError(format!("expected integer but got {args:?}"))
            })?;
            let range = BigUint::from(1u64)..=max;
            let generated = self.rng.borrow_mut().generate(range.clone());
            if !range.contains(&generated) {
                return Err(ParseWarning::RandomGeneratedValueOutOfRange {
                    expected: range,
                    actual: generated,
                });
            }
            self.state_stack
                .borrow_mut()
                .push(ProcessState::Random { generated });
            Ok(())
        };
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::Root | ProcessState::IfBlock { .. } | ProcessState::ElseBlock { .. } => {
                push_new_one()
            }
            ProcessState::Random { .. } => {
                // close this scope and start new one
                self.state_stack.borrow_mut().pop();
                push_new_one()
            }
            _ => Err(ParseWarning::UnexpectedControlFlow),
        }
    }

    fn visit_set_random(&self, args: &str) -> Result<()> {
        let push_new_one = || {
            let generated = args.trim().parse().map_err(|_| {
                ParseWarning::SyntaxError(format!("expected integer but got {args:?}"))
            })?;
            self.state_stack
                .borrow_mut()
                .push(ProcessState::Random { generated });
            Ok(())
        };
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::Root | ProcessState::IfBlock { .. } | ProcessState::ElseBlock { .. } => {
                push_new_one()
            }
            ProcessState::Random { .. } => {
                // close this scope and start new one
                self.state_stack.borrow_mut().pop();
                push_new_one()
            }
            _ => Err(ParseWarning::UnexpectedControlFlow),
        }
    }

    fn visit_if(&self, args: &str) -> Result<()> {
        let push_new_one = |generated: BigUint| {
            let cond = args.trim().parse().map_err(|_| {
                ParseWarning::SyntaxError(format!("expected integer but got {args:?}"))
            })?;
            let activated = generated == cond;
            self.state_stack.borrow_mut().push(ProcessState::IfBlock {
                if_chain_has_been_activated: activated,
                activated,
            });
            Ok(())
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
            _ => Err(ParseWarning::UnexpectedControlFlow),
        }
    }

    fn visit_else_if(&self, args: &str) -> Result<()> {
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
                    let cond = args.trim().parse().map_err(|_| {
                        ParseWarning::SyntaxError(format!("expected integer but got {args:?}"))
                    })?;
                    let activated = generated == cond;
                    self.state_stack.borrow_mut().push(ProcessState::IfBlock {
                        if_chain_has_been_activated: activated,
                        activated,
                    });
                }
                Ok(())
            }
            _ => Err(ParseWarning::UnexpectedControlFlow),
        }
    }

    fn visit_else(&self) -> Result<()> {
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
            _ => Err(ParseWarning::UnexpectedControlFlow),
        }
    }

    fn visit_end_if(&self) -> Result<()> {
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::IfBlock { .. } | ProcessState::ElseBlock { .. } => {
                self.state_stack.borrow_mut().pop();
                Ok(())
            }
            _ => Err(ParseWarning::UnexpectedControlFlow),
        }
    }

    fn visit_end_random(&self) -> Result<()> {
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::Random { .. }
            | ProcessState::IfBlock { .. }
            | ProcessState::ElseBlock { .. } => {
                self.state_stack.borrow_mut().pop();
                Ok(())
            }
            _ => Err(ParseWarning::UnexpectedControlFlow),
        }
    }

    fn visit_switch(&self, args: &str) -> Result<()> {
        let push_new_one = || {
            let max: BigUint = args.trim().parse().map_err(|_| {
                ParseWarning::SyntaxError(format!("expected integer but got {args:?}"))
            })?;
            let range = BigUint::from(1u64)..=max;
            let generated = self.rng.borrow_mut().generate(range.clone());
            if !range.contains(&generated) {
                return Err(ParseWarning::SwitchGeneratedValueOutOfRange {
                    expected: range,
                    actual: generated,
                });
            }
            self.state_stack
                .borrow_mut()
                .push(ProcessState::SwitchBeforeActive { generated });
            Ok(())
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

    fn visit_set_switch(&self, args: &str) -> Result<()> {
        let push_new_one = || {
            let generated = args.trim().parse().map_err(|_| {
                ParseWarning::SyntaxError(format!("expected integer but got {args:?}"))
            })?;
            self.state_stack
                .borrow_mut()
                .push(ProcessState::SwitchBeforeActive { generated });
            Ok(())
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

    fn visit_case(&self, args: &str) -> Result<()> {
        let cond = args
            .trim()
            .parse()
            .map_err(|_| ParseWarning::SyntaxError(format!("expected integer but got {args:?}")))?;
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::SwitchBeforeActive { generated } => {
                if generated == cond {
                    self.state_stack.borrow_mut().pop();
                    self.state_stack
                        .borrow_mut()
                        .push(ProcessState::SwitchActive { generated });
                }
                Ok(())
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
                Ok(())
            }
            ProcessState::SwitchSkipping => Ok(()),
            _ => Err(ParseWarning::UnexpectedControlFlow),
        }
    }

    fn visit_skip(&self) -> Result<()> {
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
            _ => Err(ParseWarning::UnexpectedControlFlow),
        }
    }

    fn visit_default(&self) -> Result<()> {
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
            _ => Err(ParseWarning::UnexpectedControlFlow),
        }
    }

    fn visit_end_switch(&self) -> Result<()> {
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::SwitchBeforeActive { .. }
            | ProcessState::SwitchActive { .. }
            | ProcessState::SwitchAfterActive { .. }
            | ProcessState::SwitchSkipping => {
                self.state_stack.borrow_mut().pop();
                Ok(())
            }
            _ => Err(ParseWarning::UnexpectedControlFlow),
        }
    }

    fn visit_others(&self, name: &str, args: &str) -> Result<()> {
        let top = self.state_stack.borrow().last().cloned().unwrap();
        match top {
            ProcessState::Root
            | ProcessState::IfBlock {
                activated: true, ..
            }
            | ProcessState::ElseBlock { activated: true }
            | ProcessState::SwitchActive { .. } => self.next.on_header(name, args),
            ProcessState::Random { .. } => Err(ParseWarning::UnexpectedControlFlow),
            _ => Ok(()),
        }
    }
}

impl<P: Prompter, R: Rng, N: TokenProcessor> TokenProcessor for RandomTokenProcessor<'_, P, R, N> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        let upper_name = name.to_ascii_uppercase();
        if self.relaxed {
            match upper_name.as_str() {
                "RONDAM" => {
                    self.visit_random(args)?;
                    return Ok(());
                }
                upper_name
                    if upper_name.starts_with("RANDOM") && upper_name.len() > "RANDOM".len() =>
                {
                    self.visit_random(upper_name.trim_start_matches("RANDOM"))?;
                    return Ok(());
                }
                upper_name if upper_name.starts_with("IF") && upper_name.len() > "IF".len() => {
                    self.visit_if(upper_name.trim_start_matches("IF"))?;
                    return Ok(());
                }
                _ => {}
            }
        }
        match upper_name.as_str() {
            "RANDOM" => self.visit_random(args),
            "SETRANDOM" => self.visit_set_random(args),
            "IF" => self.visit_if(args),
            "ELSEIF" => self.visit_else_if(args),
            "ELSE" => self.visit_else(),
            "ENDIF" => self.visit_else_if(args),
            "ENDRANDOM" => self.visit_end_random(),
            "SWITCH" => self.visit_switch(args),
            "SETSWITCH" => self.visit_set_switch(args),
            "CASE" => self.visit_case(args),
            "SKIP" => self.visit_skip(),
            "DEF" => self.visit_default(),
            "ENDSW" => self.visit_end_switch(),
            _ => self.visit_others(name, args),
        }
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        self.next.on_message(track, channel, message)
    }

    fn on_comment(&self, line: &str) -> Result<()> {
        if self.relaxed && line.trim().to_ascii_uppercase() == "＃ENDIF" {
            return self.visit_end_if();
        }

        self.next.on_comment(line)
    }
}
