//! This module provides an identity token processor which does nothing. It is convenient for us to compose token processors on compilation else branch.

use crate::bms::prelude::*;

use super::{TokenProcessor, TokenProcessorOutput};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdentityTokenProcessor;

impl TokenProcessor for IdentityTokenProcessor {
    type Output = ();

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        _: &P,
    ) -> TokenProcessorOutput<Self::Output> {
        *input = &[];
        TokenProcessorOutput {
            output: Ok(()),
            warnings: Vec::new(),
        }
    }
}
