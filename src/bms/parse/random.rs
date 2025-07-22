mod ast_build;
mod ast_parse;

use thiserror::Error;

use super::{ParseWarning, rng::Rng};
use crate::bms::{lex::token::Token, parse::BmsParseTokenIter};

use self::ast_build::*;
use self::ast_parse::*;

/// Parses the control flow of the token.
/// Returns the tokens that will be executed, and not contains control flow tokens.
pub(super) fn parse_control_flow<'a>(
    token_stream: &mut BmsParseTokenIter<'a>,
    mut rng: impl Rng,
) -> (Vec<&'a Token<'a>>, Vec<ParseWarning>) {
    let (ast, errors) = build_control_flow_ast(token_stream);
    let mut ast_iter = ast.into_iter().peekable();
    let tokens: Vec<&'a Token<'a>> = parse_control_flow_ast(&mut ast_iter, &mut rng);
    (
        tokens,
        errors
            .into_iter()
            .map(ParseWarning::ViolateControlFlowRule)
            .collect(),
    )
}

/// Control flow rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ControlFlowRule {
    // Random related
    #[error("unmatched end if")]
    UnmatchedEndIf,
    #[error("unmatched end random")]
    UnmatchedEndRandom,
    #[error("unmatched end switch")]
    UnmatchedEndSwitch,
    #[error("unmatched else if")]
    UnmatchedElseIf,
    #[error("unmatched else")]
    UnmatchedElse,
    #[error("duplicate if branch value in random block")]
    RandomDuplicateIfBranchValue,
    #[error("if branch value out of range in random block")]
    RandomIfBranchValueOutOfRange,
    #[error("unmatched token in random block, e.g. Tokens between Random and If.")]
    UnmatchedTokenInRandomBlock,
    // Switch related
    #[error("duplicate case value in switch block")]
    SwitchDuplicateCaseValue,
    #[error("case value out of range in switch block")]
    SwitchCaseValueOutOfRange,
    #[error("duplicate def branch in switch block")]
    SwitchDuplicateDef,
    #[error("unmatched skip")]
    UnmatchedSkip,
    #[error("unmatched case")]
    UnmatchedCase,
    #[error("unmatched def")]
    UnmatchedDef,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bms::lex::token::Token, parse::BmsParseTokenIter};
    use num::{BigUint, One};

    struct DummyRng;
    impl Rng for DummyRng {
        fn generate(&mut self, min: BigUint, _max: BigUint) -> BigUint {
            min // always return min for deterministic test
        }
    }

    #[test]
    fn test_switch_insane_tokenized() {
        use Token::*;
        let tokens = vec![
            Switch(BigUint::from(5u64)),
            Def,
            Title("0055"),
            Skip,
            Case(BigUint::one()),
            Title("0100000000000000"),
            Random(BigUint::from(2u64)),
            If(BigUint::one()),
            Title("04"),
            Else,
            Title("05"),
            EndIf,
            // Missing EndRandom!!!
            Case(BigUint::from(2u64)),
            Title("0200000000000000"),
            Skip,
            Case(BigUint::from(3u64)),
            Title("0300000000000000"),
            Switch(BigUint::from(2u64)),
            Case(BigUint::one()),
            Title("1111"),
            Skip,
            Case(BigUint::from(2u64)),
            Title("2222"),
            Skip,
            EndSwitch,
            Skip,
            EndSwitch,
        ];
        let (ast, errors) = build_control_flow_ast(&mut BmsParseTokenIter::from_tokens(&tokens));
        println!("AST structure: {ast:#?}");
        let Some(Unit::SwitchBlock { cases, .. }) =
            ast.iter().find(|u| matches!(u, Unit::SwitchBlock { .. }))
        else {
            panic!("AST structure error");
        };
        let Some(case1) = cases.iter().find(|c| match c.value {
            CaseBranchValue::Case(ref v) if v == &BigUint::one() => true,
            _ => false,
        }) else {
            panic!("Case(1) not found");
        };
        println!("Case(1) tokens: {:#?}", case1.tokens);
        let mut rng = DummyRng;
        let mut ast_iter = ast.clone().into_iter().peekable();
        let _tokens = parse_control_flow_ast(&mut ast_iter, &mut rng);
        let mut rng = DummyRng;
        let mut ast_iter = ast.into_iter().peekable();
        let _tokens = parse_control_flow_ast(&mut ast_iter, &mut rng);
        assert_eq!(errors, vec![]);
    }
}
