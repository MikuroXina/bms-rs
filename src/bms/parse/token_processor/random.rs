//! This module handle tokens:
//!
//! - `#RANDOM` - Starts a random scope which can contain only `#IF`-`#ENDIF` scopes. The random scope must close with `#ENDRANDOM`. A random integer from 1 to the integer will be generated when parsing the score. Then if the integer of `#IF` equals to the random integer, the commands in an if scope will be parsed, otherwise all command in it will be ignored. Any command except `#IF` and `#ENDIF` must not be included in the scope, but some players allow it.
//! - `#SETRANDOM` - Starts a random scope but the integer will be used as the generated random number. It should be used only for tests.
//! - `#IF` - Starts an if scope when the integer equals to the generated random number. This must be placed in a random scope. This is handled via [`Token::Header`].
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

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use crate::bms::{
    command::mixin::SourceRangeMixin,
    lex::token::{Token, TokenWithRange},
    parse::{ParseError, ParseErrorWithRange, ParseWarning, ParseWarningWithRange},
    prelude::*,
};

use super::{ProcessContext, TokenProcessor};

/// It processes `#RANDOM` and `#SWITCH` control commands.
#[derive(Debug)]
pub struct RandomTokenProcessor<R, N> {
    rng: Rc<RefCell<R>>,
    /// It must not be empty.
    state_stack: RefCell<Vec<ProcessState>>,
    next: N,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
enum ProcessState {
    #[default]
    Root,
    Random {
        generated: u64,
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
        generated: u64,
    },
    SwitchActive {
        generated: u64,
    },
    SwitchAfterActive {
        generated: u64,
    },
    SwitchSkipping,
}

struct BranchBuffer<'t> {
    conditions: Vec<u64>,
    tokens: Vec<TokenWithRange<'t>>,
    nested_objects: Vec<RandomizedObjects>,
}

struct RandomScope<'t> {
    line_number: usize,
    generating: Option<ControlFlowValue>,
    max_value: Option<u64>,
    branches: BTreeMap<u64, RandomizedBranch>,
    covered_values: BTreeSet<u64>,
    current_branch: Option<BranchBuffer<'t>>,
}

struct Collector<'t> {
    stack: Vec<RandomScope<'t>>,
    finished_objects: Vec<RandomizedObjects>,
}

impl<'t> Collector<'t> {
    const fn new() -> Self {
        Self {
            stack: Vec::new(),
            finished_objects: Vec::new(),
        }
    }

    fn push_random(&mut self, generating: ControlFlowValue, line_number: usize) {
        let max_value = match &generating {
            ControlFlowValue::GenMax(m) => Some(*m),
            ControlFlowValue::Set(_) => None, // Explicit value, cannot infer range for ELSE
        };
        self.stack.push(RandomScope {
            line_number,
            generating: Some(generating),
            max_value,
            branches: BTreeMap::new(),
            covered_values: BTreeSet::new(),
            current_branch: None,
        });
    }

    fn current_scope_mut(&mut self) -> Option<&mut RandomScope<'t>> {
        self.stack.last_mut()
    }

    fn start_branch(&mut self, condition: u64) {
        if let Some(scope) = self.current_scope_mut() {
            scope.covered_values.insert(condition);
            scope.current_branch = Some(BranchBuffer {
                conditions: vec![condition],
                tokens: Vec::new(),
                nested_objects: Vec::new(),
            });
        }
    }

    fn add_token(&mut self, token: TokenWithRange<'t>) {
        // Add to the top-most branch buffer
        // Also, if we are nested, we only add to the immediate parent's buffer.
        if let Some(scope) = self.current_scope_mut()
            && let Some(branch) = &mut scope.current_branch
        {
            branch.tokens.push(token);
        }
    }

    fn add_nested_object(&mut self, obj: RandomizedObjects) {
        if let Some(scope) = self.current_scope_mut() {
            if let Some(branch) = &mut scope.current_branch {
                branch.nested_objects.push(obj);
            }
        } else {
            // Root level
            self.finished_objects.push(obj);
        }
    }
}

impl<R, N> RandomTokenProcessor<R, N> {
    pub fn new(rng: Rc<RefCell<R>>, next: N) -> Self {
        Self {
            rng,
            state_stack: RefCell::new(vec![ProcessState::Root]),
            next,
        }
    }
}

impl<R: Rng, N: TokenProcessor<Output = Bms> + Clone> RandomTokenProcessor<R, N> {
    // Helper to process a branch buffer into a Bms
    fn process_branch_buffer<'t>(
        &self,
        buffer: &BranchBuffer<'t>,
        prompter: &impl crate::bms::parse::Prompter,
    ) -> Result<Bms, crate::bms::parse::ParseErrorWithRange> {
        // Create a new processor for recursion
        let sub_processor = RandomTokenProcessor::new(self.rng.clone(), self.next.clone());

        let tokens_vec = buffer.tokens.iter().collect::<Vec<_>>();
        let mut tokens_slice = tokens_vec.as_slice();
        let mut ctx = ProcessContext::new(&mut tokens_slice, prompter);

        let (bms, nested) = sub_processor.process(&mut ctx)?;

        let mut final_bms = bms;
        final_bms.randomized.extend(nested);
        final_bms.randomized.extend(buffer.nested_objects.clone());

        Ok(final_bms)
    }

    fn top_state<'t>(
        &self,
        token: &TokenWithRange<'t>,
    ) -> Result<ProcessState, ParseErrorWithRange> {
        self.state_stack.borrow().last().cloned().ok_or_else(|| {
            SourceRangeMixin::new(
                ParseError::UnexpectedControlFlow("internal control flow state is empty"),
                token.range().clone(),
            )
        })
    }

    fn finish_current_branch<'t>(
        &self,
        collector: &mut Collector<'t>,
        prompter: &impl crate::bms::parse::Prompter,
    ) -> Result<(), crate::bms::parse::ParseErrorWithRange> {
        if let Some(scope) = collector.current_scope_mut()
            && let Some(buffer) = scope.current_branch.take()
        {
            let bms = self.process_branch_buffer(&buffer, prompter)?;
            for &cond in buffer.conditions.iter() {
                scope
                    .branches
                    .insert(cond, RandomizedBranch::new(cond, bms.clone()));
            }
        }
        Ok(())
    }

    fn visit_random<'t>(
        &self,
        args: &str,
        collector: &mut Collector<'t>,
        prompter: &impl crate::bms::parse::Prompter,
        token: &TokenWithRange<'t>,
    ) -> core::result::Result<Option<ParseWarningWithRange>, ParseErrorWithRange> {
        let push_new_one = |collector_mut: &mut Collector<'t>| {
            let max: u64 = match args.parse().map_err(|_| {
                SourceRangeMixin::new(
                    ParseWarning::SyntaxError(format!("expected integer but got {args:?}")),
                    token.range().clone(),
                )
            }) {
                Ok(max) => max,
                Err(warning) => return Ok(Some(warning)),
            };
            let range = 1u64..=max;
            let activated = self.is_activated();
            let generated = self.rng.borrow_mut().generate(range.clone());
            if activated && !range.contains(&generated) {
                return Err(SourceRangeMixin::new(
                    ParseError::RandomGeneratedValueOutOfRange {
                        expected: range,
                        actual: generated,
                    },
                    token.range().clone(),
                ));
            }
            self.state_stack.borrow_mut().push(ProcessState::Random {
                generated,
                activated,
            });

            collector_mut.push_random(ControlFlowValue::GenMax(max), token.range().start);

            Ok(None)
        };

        let top = self.top_state(token)?;
        match top {
            ProcessState::Root
            | ProcessState::IfBlock { .. }
            | ProcessState::ElseBlock { .. }
            | ProcessState::SwitchBeforeActive { .. }
            | ProcessState::SwitchActive { .. }
            | ProcessState::SwitchAfterActive { .. }
            | ProcessState::SwitchSkipping => push_new_one(collector),
            ProcessState::Random { .. } => {
                self.visit_end_random(collector, prompter, token)?;
                push_new_one(collector)
            }
        }
    }

    fn visit_set_random<'t>(
        &self,
        args: &str,
        collector: &mut Collector<'t>,
        prompter: &impl crate::bms::parse::Prompter,
        token: &TokenWithRange<'t>,
    ) -> core::result::Result<Option<ParseWarningWithRange>, ParseErrorWithRange> {
        let push_new_one = |collector_mut: &mut Collector<'t>| {
            let generated: u64 = match args.parse().map_err(|_| {
                SourceRangeMixin::new(
                    ParseWarning::SyntaxError(format!("expected integer but got {args:?}")),
                    token.range().clone(),
                )
            }) {
                Ok(max) => max,
                Err(warning) => return Ok(Some(warning)),
            };
            let activated = self.is_activated();
            self.state_stack.borrow_mut().push(ProcessState::Random {
                generated,
                activated,
            });
            collector_mut.push_random(ControlFlowValue::Set(generated), token.range().start);
            Ok(None)
        };
        let top = self.top_state(token)?;
        match top {
            ProcessState::Root
            | ProcessState::IfBlock { .. }
            | ProcessState::ElseBlock { .. }
            | ProcessState::SwitchBeforeActive { .. }
            | ProcessState::SwitchActive { .. }
            | ProcessState::SwitchAfterActive { .. }
            | ProcessState::SwitchSkipping => push_new_one(collector),
            ProcessState::Random { .. } => {
                self.visit_end_random(collector, prompter, token)?;
                push_new_one(collector)
            }
        }
    }

    fn visit_if<'t>(
        &self,
        args: &str,
        collector: &mut Collector<'t>,
        prompter: &impl crate::bms::parse::Prompter,
        token: &TokenWithRange<'t>,
    ) -> core::result::Result<Option<ParseWarningWithRange>, ParseErrorWithRange> {
        let push_new_one = |collector_mut: &mut Collector<'t>, generated: u64| {
            let cond = match args.parse().map_err(|_| {
                SourceRangeMixin::new(
                    ParseWarning::SyntaxError(format!("expected integer but got {args:?}")),
                    token.range().clone(),
                )
            }) {
                Ok(max) => max,
                Err(warning) => return Ok(Some(warning)),
            };
            let activated = generated == cond;
            self.state_stack.borrow_mut().push(ProcessState::IfBlock {
                if_chain_has_been_activated: activated,
                activated,
            });

            self.finish_current_branch(collector_mut, prompter)?;
            collector_mut.start_branch(cond);

            Ok(None)
        };
        let top = self.top_state(token)?;
        match top {
            ProcessState::Random { generated, .. } => push_new_one(collector, generated),
            ProcessState::IfBlock { .. } | ProcessState::ElseBlock { .. } => {
                self.visit_end_if(collector, prompter, token)?;
                let generated = match self.state_stack.borrow().last().cloned() {
                    Some(ProcessState::Random { generated, .. }) => generated,
                    _ => {
                        return Err(SourceRangeMixin::new(
                            ParseError::UnexpectedControlFlow("#IF must be on a random scope"),
                            token.range().clone(),
                        ));
                    }
                };
                push_new_one(collector, generated)
            }
            _ => Err(SourceRangeMixin::new(
                ParseError::UnexpectedControlFlow("#IF must be on a random scope"),
                token.range().clone(),
            )),
        }
    }

    fn visit_else_if<'t>(
        &self,
        args: &str,
        collector: &mut Collector<'t>,
        prompter: &impl crate::bms::parse::Prompter,
        token: &TokenWithRange<'t>,
    ) -> core::result::Result<Option<ParseWarningWithRange>, ParseErrorWithRange> {
        let top = self.top_state(token)?;
        match top {
            ProcessState::IfBlock {
                if_chain_has_been_activated,
                ..
            } => {
                self.state_stack.borrow_mut().pop();
                let generated = match self.state_stack.borrow().last().cloned() {
                    Some(ProcessState::Random { generated, .. }) => generated,
                    _ => {
                        return Err(SourceRangeMixin::new(
                            ParseError::UnexpectedControlFlow("#ELSEIF must be on a random scope"),
                            token.range().clone(),
                        ));
                    }
                };

                self.finish_current_branch(collector, prompter)?;

                let cond = match args.parse().map_err(|_| {
                    SourceRangeMixin::new(
                        ParseWarning::SyntaxError(format!("expected integer but got {args:?}")),
                        token.range().clone(),
                    )
                }) {
                    Ok(max) => max,
                    Err(warning) => return Ok(Some(warning)),
                };

                if if_chain_has_been_activated {
                    self.state_stack.borrow_mut().push(ProcessState::IfBlock {
                        if_chain_has_been_activated,
                        activated: false,
                    });
                } else {
                    let activated = generated == cond;
                    self.state_stack.borrow_mut().push(ProcessState::IfBlock {
                        if_chain_has_been_activated: activated,
                        activated,
                    });
                }

                collector.start_branch(cond);

                Ok(None)
            }
            _ => Err(SourceRangeMixin::new(
                ParseError::UnexpectedControlFlow("#ELSEIF must come after of a #IF"),
                token.range().clone(),
            )),
        }
    }

    fn visit_else<'t>(
        &self,
        collector: &mut Collector<'t>,
        prompter: &impl crate::bms::parse::Prompter,
        token: &TokenWithRange<'t>,
    ) -> core::result::Result<(), ParseErrorWithRange> {
        let top = self.top_state(token)?;
        match top {
            ProcessState::IfBlock {
                if_chain_has_been_activated,
                ..
            } => {
                self.state_stack.borrow_mut().pop();
                self.state_stack.borrow_mut().push(ProcessState::ElseBlock {
                    activated: !if_chain_has_been_activated,
                });

                self.finish_current_branch(collector, prompter)?;

                if let Some(scope) = collector.current_scope_mut()
                    && let Some(max) = &scope.max_value
                {
                    let mut conditions = Vec::new();
                    let mut current = 1u64;
                    while current <= *max {
                        if !scope.covered_values.contains(&current) {
                            conditions.push(current);
                            scope.covered_values.insert(current);
                        }
                        current = current.saturating_add(1u64);
                    }

                    scope.current_branch = Some(BranchBuffer {
                        conditions,
                        tokens: Vec::new(),
                        nested_objects: Vec::new(),
                    });
                }

                Ok(())
            }
            _ => Err(SourceRangeMixin::new(
                ParseError::UnexpectedControlFlow("#ELSE must come after #IF or #ELSEIF"),
                token.range().clone(),
            )),
        }
    }

    fn visit_end_if<'t>(
        &self,
        collector: &mut Collector<'t>,
        prompter: &impl crate::bms::parse::Prompter,
        token: &TokenWithRange<'t>,
    ) -> core::result::Result<(), ParseErrorWithRange> {
        let top = self.top_state(token)?;
        match top {
            ProcessState::IfBlock { .. } | ProcessState::ElseBlock { .. } => {
                self.state_stack.borrow_mut().pop();
                self.finish_current_branch(collector, prompter)?;
                Ok(())
            }
            _ => Err(SourceRangeMixin::new(
                ParseError::UnexpectedControlFlow("#ENDIF must come after #IF, #ELSEIF or #ELSE"),
                token.range().clone(),
            )),
        }
    }

    fn visit_end_random<'t>(
        &self,
        collector: &mut Collector<'t>,
        prompter: &impl crate::bms::parse::Prompter,
        token: &TokenWithRange<'t>,
    ) -> core::result::Result<(), ParseErrorWithRange> {
        let top = self.top_state(token)?;
        match top {
            ProcessState::Random { .. }
            | ProcessState::IfBlock { .. }
            | ProcessState::ElseBlock { .. } => {
                self.state_stack.borrow_mut().pop();

                self.finish_current_branch(collector, prompter)?;

                if let Some(scope) = collector.stack.pop() {
                    let obj = RandomizedObjects {
                        line_number: scope.line_number,
                        generating: scope.generating,
                        branches: scope.branches,
                    };
                    collector.add_nested_object(obj);
                }

                Ok(())
            }
            _ => Err(SourceRangeMixin::new(
                ParseError::UnexpectedControlFlow("#ENDRANDOM must come after #RANDOM"),
                token.range().clone(),
            )),
        }
    }

    fn visit_switch<'t>(
        &self,
        args: &str,
        collector: &mut Collector<'t>,
        prompter: &impl crate::bms::parse::Prompter,
        token: &TokenWithRange<'t>,
    ) -> core::result::Result<Option<ParseWarningWithRange>, ParseErrorWithRange> {
        let push_new_one = |collector_mut: &mut Collector<'t>| {
            let max: u64 = match args.parse().map_err(|_| {
                SourceRangeMixin::new(
                    ParseWarning::SyntaxError(format!("expected integer but got {args:?}")),
                    token.range().clone(),
                )
            }) {
                Ok(max) => max,
                Err(warning) => return Ok(Some(warning)),
            };
            let range = 1u64..=max;
            let activated = self.is_activated();
            let generated = self.rng.borrow_mut().generate(range.clone());
            if activated {
                if !range.contains(&generated) {
                    return Err(SourceRangeMixin::new(
                        ParseError::SwitchGeneratedValueOutOfRange {
                            expected: range,
                            actual: generated,
                        },
                        token.range().clone(),
                    ));
                }
                self.state_stack
                    .borrow_mut()
                    .push(ProcessState::SwitchBeforeActive { generated });
            } else {
                self.state_stack
                    .borrow_mut()
                    .push(ProcessState::SwitchAfterActive { generated });
            }

            collector_mut.push_random(ControlFlowValue::GenMax(max), token.range().start);

            Ok(None)
        };
        let top = self.top_state(token)?;
        match top {
            ProcessState::Random { .. } => {
                self.state_stack.borrow_mut().pop();
                self.visit_end_random(collector, prompter, token)?;
                push_new_one(collector)
            }
            _ => push_new_one(collector),
        }
    }

    fn visit_set_switch<'t>(
        &self,
        args: &str,
        collector: &mut Collector<'t>,
        prompter: &impl crate::bms::parse::Prompter,
        token: &TokenWithRange<'t>,
    ) -> core::result::Result<Option<ParseWarningWithRange>, ParseErrorWithRange> {
        let push_new_one = |collector_mut: &mut Collector<'t>| {
            let generated: u64 = match args.parse().map_err(|_| {
                SourceRangeMixin::new(
                    ParseWarning::SyntaxError(format!("expected integer but got {args:?}")),
                    token.range().clone(),
                )
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
            collector_mut.push_random(ControlFlowValue::Set(generated), token.range().start);
            Ok(None)
        };
        let top = self.top_state(token)?;
        match top {
            ProcessState::Random { .. } => {
                self.state_stack.borrow_mut().pop();
                self.visit_end_random(collector, prompter, token)?;
                push_new_one(collector)
            }
            _ => push_new_one(collector),
        }
    }

    fn visit_case<'t>(
        &self,
        args: &str,
        collector: &mut Collector<'t>,
        prompter: &impl crate::bms::parse::Prompter,
        token: &TokenWithRange<'t>,
    ) -> core::result::Result<Option<ParseWarningWithRange>, ParseErrorWithRange> {
        let cond = match args.parse().map_err(|_| {
            SourceRangeMixin::new(
                ParseWarning::SyntaxError(format!("expected integer but got {args:?}")),
                token.range().clone(),
            )
        }) {
            Ok(max) => max,
            Err(warning) => return Ok(Some(warning)),
        };

        loop {
            let top = self.top_state(token)?;
            if let ProcessState::Random { .. }
            | ProcessState::IfBlock { .. }
            | ProcessState::ElseBlock { .. } = top
            {
                self.state_stack.borrow_mut().pop();
                // Close scopes implicitly
                self.finish_current_branch(collector, prompter)?;
                if let ProcessState::Random { .. } = top
                    && let Some(scope) = collector.stack.pop()
                {
                    let obj = RandomizedObjects {
                        line_number: scope.line_number,
                        generating: scope.generating,
                        branches: scope.branches,
                    };
                    collector.add_nested_object(obj);
                }
            } else {
                break;
            }
        }

        self.finish_current_branch(collector, prompter)?;

        let top = self.top_state(token)?;
        match top {
            ProcessState::SwitchBeforeActive { generated } => {
                if generated == cond {
                    self.state_stack.borrow_mut().pop();
                    self.state_stack
                        .borrow_mut()
                        .push(ProcessState::SwitchActive { generated });
                }
                collector.start_branch(cond);
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
                collector.start_branch(cond);
                Ok(None)
            }
            ProcessState::SwitchSkipping => {
                collector.start_branch(cond);
                Ok(None)
            }
            _ => Err(SourceRangeMixin::new(
                ParseError::UnexpectedControlFlow("#CASE must be on a switch block"),
                token.range().clone(),
            )),
        }
    }

    fn visit_skip<'t>(
        &self,
        _collector: &mut Collector<'t>,
        _prompter: &impl crate::bms::parse::Prompter,
        token: &TokenWithRange<'t>,
    ) -> core::result::Result<(), ParseErrorWithRange> {
        let top = self.top_state(token)?;
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
            _ => Err(SourceRangeMixin::new(
                ParseError::UnexpectedControlFlow("#SKIP must be on a switch block"),
                token.range().clone(),
            )),
        }
    }

    fn visit_default<'t>(
        &self,
        collector: &mut Collector<'t>,
        prompter: &impl crate::bms::parse::Prompter,
        token: &TokenWithRange<'t>,
    ) -> core::result::Result<(), ParseErrorWithRange> {
        self.finish_current_branch(collector, prompter)?;

        let top = self.top_state(token)?;
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
            _ => Err(SourceRangeMixin::new(
                ParseError::UnexpectedControlFlow("#DEF must be on a switch block"),
                token.range().clone(),
            )),
        }
        .map(|_| {
            if let Some(scope) = collector.current_scope_mut()
                && let Some(max) = &scope.max_value
            {
                let mut conditions = Vec::new();
                let mut current = 1u64;
                while current <= *max {
                    if !scope.covered_values.contains(&current) {
                        conditions.push(current);
                        scope.covered_values.insert(current);
                    }
                    current = current.saturating_add(1u64);
                }

                scope.current_branch = Some(BranchBuffer {
                    conditions,
                    tokens: Vec::new(),
                    nested_objects: Vec::new(),
                });
            }
        })
    }

    fn visit_end_switch<'t>(
        &self,
        collector: &mut Collector<'t>,
        prompter: &impl crate::bms::parse::Prompter,
        token: &TokenWithRange<'t>,
    ) -> Result<(), ParseErrorWithRange> {
        let top = self.top_state(token)?;
        match top {
            ProcessState::SwitchBeforeActive { .. }
            | ProcessState::SwitchActive { .. }
            | ProcessState::SwitchAfterActive { .. }
            | ProcessState::SwitchSkipping => {
                self.state_stack.borrow_mut().pop();
                self.finish_current_branch(collector, prompter)?;

                if let Some(scope) = collector.stack.pop() {
                    let obj = RandomizedObjects {
                        line_number: scope.line_number,
                        generating: scope.generating,
                        branches: scope.branches,
                    };
                    collector.add_nested_object(obj);
                }

                Ok(())
            }
            _ => Err(SourceRangeMixin::new(
                ParseError::UnexpectedControlFlow("#ENDSWITCH must come after #SWITCH"),
                token.range().clone(),
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

    fn on_header<'t>(
        &self,
        name: &str,
        args: &str,
        collector: &mut Collector<'t>,
        prompter: &impl crate::bms::parse::Prompter,
        token: &TokenWithRange<'t>,
    ) -> Result<Option<ParseWarningWithRange>, ParseErrorWithRange> {
        if name.eq_ignore_ascii_case("RANDOM") {
            return self.visit_random(args, collector, prompter, token);
        }
        if name.eq_ignore_ascii_case("SETRANDOM") {
            return self.visit_set_random(args, collector, prompter, token);
        }
        if name.eq_ignore_ascii_case("IF") {
            return self.visit_if(args, collector, prompter, token);
        }
        if name.eq_ignore_ascii_case("ELSEIF") {
            return self.visit_else_if(args, collector, prompter, token);
        }
        if name.eq_ignore_ascii_case("ELSE") {
            return self.visit_else(collector, prompter, token).map(|_| None);
        }
        if name.eq_ignore_ascii_case("ENDIF") {
            return self.visit_end_if(collector, prompter, token).map(|_| None);
        }
        if name.eq_ignore_ascii_case("ENDRANDOM") {
            return self
                .visit_end_random(collector, prompter, token)
                .map(|_| None);
        }
        if name.eq_ignore_ascii_case("SWITCH") {
            return self.visit_switch(args, collector, prompter, token);
        }
        if name.eq_ignore_ascii_case("SETSWITCH") {
            return self.visit_set_switch(args, collector, prompter, token);
        }
        if name.eq_ignore_ascii_case("CASE") {
            return self.visit_case(args, collector, prompter, token);
        }
        if name.eq_ignore_ascii_case("SKIP") {
            return self.visit_skip(collector, prompter, token).map(|_| None);
        }
        if name.eq_ignore_ascii_case("DEF") {
            return self.visit_default(collector, prompter, token).map(|_| None);
        }
        if name.eq_ignore_ascii_case("ENDSW") {
            return self
                .visit_end_switch(collector, prompter, token)
                .map(|_| None);
        }
        Ok(None)
    }
}

impl<R: Rng, N: TokenProcessor<Output = Bms> + Clone> TokenProcessor
    for RandomTokenProcessor<R, N>
{
    type Output = (N::Output, Vec<RandomizedObjects>);

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> Result<Self::Output, crate::bms::parse::ParseErrorWithRange> {
        let checkpoint = ctx.save();
        let mut activated: Vec<&'a TokenWithRange<'t>> = Vec::new();
        let mut collector = Collector::new();

        let view = ctx.take_input();
        let prompter = ctx.prompter();

        for token in view.iter().copied() {
            let warn = match token.content() {
                Token::Header { name, args } => self.on_header(
                    name.as_ref(),
                    args.as_ref(),
                    &mut collector,
                    prompter,
                    token,
                )?,
                Token::Message { .. } => None,
                Token::NotACommand(_line) => None,
            };

            if let Some(w) = warn {
                ctx.warn(w);
            }

            let is_control = matches!(token.content(), Token::Header { name, .. } if
                ["RANDOM", "SETRANDOM", "IF", "ELSEIF", "ELSE", "ENDIF", "ENDRANDOM",
                 "SWITCH", "SETSWITCH", "CASE", "SKIP", "DEF", "ENDSW"].iter().any(|&k| name.eq_ignore_ascii_case(k))
            );

            if !is_control {
                collector.add_token(token.clone());
            }

            if self.is_activated() {
                activated.push(token);
            }
        }

        let mut tmp = &activated[..];
        let mut view_ctx = ProcessContext {
            input: &mut tmp,
            prompter: ctx.prompter(),
            reported: Vec::new(),
        };
        let out = self.next.process(&mut view_ctx)?;
        ctx.reported.extend(view_ctx.into_warnings());
        ctx.restore(checkpoint);

        Ok((out, collector.finished_objects))
    }
}
