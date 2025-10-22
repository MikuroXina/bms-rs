//! This module provides an identity token processor which does nothing. It is convenient for us to compose token processors on compilation else branch.

use crate::bms::prelude::*;

use super::{TokenProcessor, TokenProcessorResult};

#[allow(dead_code)]
pub struct IdentityTokenProcessor;

impl TokenProcessor for IdentityTokenProcessor {
    fn process(&self, input: &mut &[&TokenWithRange<'_>]) -> TokenProcessorResult {
        *input = &[];
        Ok(vec![])
    }
}
