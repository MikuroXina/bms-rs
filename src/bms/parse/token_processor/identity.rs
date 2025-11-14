//! This module provides an identity token processor which does nothing. It is convenient for us to compose token processors on compilation else branch.

use crate::bms::ParseErrorWithRange;
use crate::bms::prelude::*;

use super::{ProcessContext, TokenProcessor};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdentityTokenProcessor;

impl TokenProcessor for IdentityTokenProcessor {
    type Output = ();

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> Result<Self::Output, ParseErrorWithRange> {
        let _ = ctx.take_input();
        Ok(())
    }
}
