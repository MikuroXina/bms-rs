//! This module provides an identity token processor which does nothing. It is convenient for us to compose token processors on compilation else branch.

use crate::bms::{error::ParseErrorWithRange, prelude::*};

use super::TokenProcessor;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdentityTokenProcessor;

impl TokenProcessor for IdentityTokenProcessor {
    type Output = ();

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        _: &P,
    ) -> (
        Self::Output,
        Vec<ParseWarningWithRange>,
        Vec<ParseErrorWithRange>,
    ) {
        *input = &[];
        ((), Vec::new(), Vec::new())
    }
}
