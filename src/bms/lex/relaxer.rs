use crate::bms::command::mixin::SourceRangeMixin;
use crate::bms::lex::{
    cursor::Cursor,
    token::{Token, TokenWithRange},
};
use num::BigUint;

/// Trait以提供可扩展的宽松词法（Relaxer）规则。
pub trait Relaxer {
    /// 规范化原始命令token（如大小写、全角`#`、常见拼写）。
    fn normalize_command(&self, command: &str) -> String;

    /// 处理需要读取额外token或构造特定Token的特殊模式；若已处理，返回`Some(TokenWithRange)`。
    fn try_handle_special<'a>(
        &self,
        command_upper: &str,
        cursor: &mut Cursor<'a>,
        start_index: usize,
    ) -> Option<TokenWithRange<'a>>;
}

/// 将输入统一转换为大写以便后续规则匹配。
pub struct UppercaseRelaxer;

impl Relaxer for UppercaseRelaxer {
    fn normalize_command(&self, command: &str) -> String {
        command.to_uppercase()
    }
    fn try_handle_special<'a>(
        &self,
        _command_upper: &str,
        _cursor: &mut Cursor<'a>,
        _start_index: usize,
    ) -> Option<TokenWithRange<'a>> {
        None
    }
}

/// 处理以全角`＃`开头的命令，将其替换为半角`#`。
pub struct FullwidthHashRelaxer;

impl Relaxer for FullwidthHashRelaxer {
    fn normalize_command(&self, command: &str) -> String {
        if command.starts_with('＃') {
            let mut s = String::from("#");
            s.push_str(&command.chars().skip(1).collect::<String>());
            s
        } else {
            command.to_string()
        }
    }
    fn try_handle_special<'a>(
        &self,
        _command_upper: &str,
        _cursor: &mut Cursor<'a>,
        _start_index: usize,
    ) -> Option<TokenWithRange<'a>> {
        None
    }
}

/// 处理常见拼写/别名：`#RONDAM`→`#RANDOM`，`#IFEND`→`#ENDIF`。
pub struct TypoRelaxer;

impl Relaxer for TypoRelaxer {
    fn normalize_command(&self, command: &str) -> String {
        match command {
            "#RONDAM" => "#RANDOM".into(),
            "#IFEND" => "#ENDIF".into(),
            _ => command.to_string(),
        }
    }
    fn try_handle_special<'a>(
        &self,
        _command_upper: &str,
        _cursor: &mut Cursor<'a>,
        _start_index: usize,
    ) -> Option<TokenWithRange<'a>> {
        None
    }
}

/// 处理无空格数字后缀：`#RANDOMn`、`#IFn`。
pub struct NumberSuffixRelaxer;

impl Relaxer for NumberSuffixRelaxer {
    fn normalize_command(&self, command: &str) -> String {
        command.to_string()
    }
    fn try_handle_special<'a>(
        &self,
        command_upper: &str,
        cursor: &mut Cursor<'a>,
        start_index: usize,
    ) -> Option<TokenWithRange<'a>> {
        if let Some(suffix) = command_upper.strip_prefix("#RANDOM")
            && !suffix.is_empty()
            && suffix.chars().all(|ch| ch.is_ascii_digit())
            && let Ok(max) = suffix.parse::<BigUint>()
        {
            let range = start_index..cursor.index();
            return Some(SourceRangeMixin::new(Token::Random(max), range));
        }
        if let Some(suffix) = command_upper.strip_prefix("#IF")
            && !suffix.is_empty()
            && suffix.chars().all(|ch| ch.is_ascii_digit())
            && let Ok(target) = suffix.parse::<BigUint>()
        {
            let range = start_index..cursor.index();
            return Some(SourceRangeMixin::new(Token::If(target), range));
        }
        None
    }
}

/// 处理分词形式的 `#END IF`（需消费后续`IF`）。
pub struct SpacedEndIfRelaxer;

impl Relaxer for SpacedEndIfRelaxer {
    fn normalize_command(&self, command: &str) -> String {
        command.to_string()
    }
    fn try_handle_special<'a>(
        &self,
        command_upper: &str,
        cursor: &mut Cursor<'a>,
        start_index: usize,
    ) -> Option<TokenWithRange<'a>> {
        if command_upper == "#END"
            && let Some(next) = cursor.peek_next_token()
            && next.to_uppercase() == "IF"
        {
            let _ = cursor.next_token();
            let range = start_index..cursor.index();
            return Some(SourceRangeMixin::new(Token::EndIf, range));
        }
        None
    }
}

/// 返回默认的 Relaxer 列表：大写→全角`#`→拼写→数字后缀→`#END IF`。
#[must_use]
pub fn default_relaxers() -> Vec<Box<dyn Relaxer>> {
    vec![
        Box::new(UppercaseRelaxer),
        Box::new(FullwidthHashRelaxer),
        Box::new(TypoRelaxer),
        Box::new(NumberSuffixRelaxer),
        Box::new(SpacedEndIfRelaxer),
    ]
}
