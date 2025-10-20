//! This module provides an identity token processor which does nothing. It is convenient for us to compose token processors on compilation else branch.

use crate::{bms::prelude::*, parse::Result};

use super::TokenProcessor;

pub struct IdentityTokenProcessor;

impl TokenProcessor for IdentityTokenProcessor {
    fn on_header(&self, _: &str, _: &str) -> Result<()> {
        Ok(())
    }

    fn on_message(&self, _: Track, _: Channel, _: &str) -> Result<()> {
        Ok(())
    }
}
