//! This module provides an identity token processor which does nothing. It is convenient for us to compose token processors on compilation else branch.

use crate::{bms::prelude::*, parse::Result};

use super::TokenProcessor;

pub struct IdentityTokenProcessor;

impl TokenProcessor for IdentityTokenProcessor {
    fn process(&self, _: &mut &[Token<'_>]) -> Result<()> {
        Ok(())
    }
}
