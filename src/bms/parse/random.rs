mod ast_build;
mod ast_parse;

use thiserror::Error;

use super::{ParseWarning, rng::Rng};
use crate::bms::lex::token::Token;

use self::ast_build::*;
use self::ast_parse::*;

/// Parses the control flow of the token.
/// Returns the tokens that will be executed, and not contains control flow tokens.
pub(super) fn parse_control_flow<'a>(
    token_stream: &mut std::iter::Peekable<impl Iterator<Item = &'a Token<'a>>>,
    mut rng: impl Rng,
) -> (Vec<&'a Token<'a>>, Vec<ParseWarning>) {
    let mut error_list = Vec::new();
    let ast: Vec<Unit<'a>> = build_control_flow_ast(token_stream, &mut error_list);
    let mut ast_iter = ast.into_iter().peekable();
    let tokens: Vec<&'a Token<'a>> = parse_control_flow_ast(&mut ast_iter, &mut rng);
    (
        tokens,
        error_list
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
    use crate::bms::lex::token::Token;

    struct DummyRng;
    impl Rng for DummyRng {
        fn generate(&mut self, _range: std::ops::RangeInclusive<u32>) -> u32 {
            // Always return the maximum value
            *_range.end()
        }
    }

    #[test]
    fn test_switch_nested_switch_case() {
        use Token::*;
        let tokens = vec![
            Title("11000000"),
            Switch(2),
            Case(1),
            Title("00220000"),
            Random(2),
            If(1),
            Title("00550000"),
            ElseIf(2),
            Title("00006600"),
            EndIf,
            EndRandom,
            Skip,
            Case(2),
            Title("00003300"),
            Skip,
            EndSwitch,
            Title("00000044"),
        ];
        let mut errors = Vec::new();
        let ast = build_control_flow_ast(&mut tokens.iter().peekable(), &mut errors);
        println!("AST structure: {ast:#?}");
        let Some(Unit::SwitchBlock { cases, .. }) =
            ast.iter().find(|u| matches!(u, Unit::SwitchBlock { .. }))
        else {
            panic!("AST structure error");
        };
        let Some(case1) = cases
            .iter()
            .find(|c| matches!(c.value, CaseBranchValue::Case(1)))
        else {
            panic!("Case(1) not found");
        };
        println!("Case(1) tokens: {:#?}", case1.tokens);
        assert_eq!(errors, vec![]);
        assert!(matches!(&ast[0], Unit::Token(_))); // 11000000
        assert!(matches!(&ast[1], Unit::SwitchBlock { .. }));
        assert!(matches!(&ast[2], Unit::Token(_))); // 00000044
        let Unit::SwitchBlock { cases, .. } = &ast[1] else {
            panic!("AST structure error");
        };
        let Some(CaseBranch { tokens, .. }) = cases
            .iter()
            .find(|c| matches!(c.value, CaseBranchValue::Case(1)))
        else {
            panic!("Case(1) not found");
        };
        assert!(matches!(&tokens[0], Unit::Token(_))); // 00220000
        assert!(matches!(&tokens[1], Unit::RandomBlock { .. }));
        let Unit::RandomBlock { if_blocks, .. } = &tokens[1] else {
            panic!("RandomBlock not found");
        };
        let if_block = &if_blocks[0];
        assert!(
            if_block
                .branches
                .get(&1)
                .unwrap()
                .tokens
                .iter()
                .any(|u| matches!(u, Unit::Token(Title("00550000"))))
        );
        assert!(
            if_block
                .branches
                .get(&2)
                .unwrap()
                .tokens
                .iter()
                .any(|u| matches!(u, Unit::Token(Title("00006600"))))
        );
        let Some(CaseBranch { tokens, .. }) = cases
            .iter()
            .find(|c| matches!(c.value, CaseBranchValue::Case(2)))
        else {
            panic!("Case(2) not found");
        };
        assert!(matches!(&tokens[0], Unit::Token(Title("00003300"))));
        let mut rng = DummyRng;
        let mut ast_iter = ast.into_iter().peekable();
        let tokens = parse_control_flow_ast(&mut ast_iter, &mut rng);
        let expected = ["11000000", "00003300", "00000044"];
        assert_eq!(tokens.len(), 3);
        for (i, t) in tokens.iter().enumerate() {
            match t {
                Title(s) => {
                    assert_eq!(s, &expected[i], "Title content mismatch");
                }
                _ => panic!("Token type mismatch"),
            }
        }
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn test_switch_insane_tokenized() {
        use Token::*;
        let tokens = vec![
            Switch(5),
            Def,
            Title("0055"),
            Skip,
            Case(1),
            Title("0100000000000000"),
            Random(2),
            If(1),
            Title("04"),
            Else,
            Title("05"),
            EndIf,
            // Missing EndRandom!!!
            Case(2),
            Title("0200000000000000"),
            Skip,
            Case(3),
            Title("0300000000000000"),
            Switch(2),
            Case(1),
            Title("1111"),
            Skip,
            Case(2),
            Title("2222"),
            Skip,
            EndSwitch,
            Skip,
            EndSwitch,
        ];
        let mut errors = Vec::new();
        let ast = build_control_flow_ast(&mut tokens.iter().peekable(), &mut errors);
        println!("AST structure: {ast:#?}");
        let Some(Unit::SwitchBlock { cases, .. }) =
            ast.iter().find(|u| matches!(u, Unit::SwitchBlock { .. }))
        else {
            panic!("AST structure error");
        };
        let Some(case1) = cases
            .iter()
            .find(|c| matches!(c.value, CaseBranchValue::Case(1)))
        else {
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
