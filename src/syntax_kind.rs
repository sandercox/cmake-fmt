use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum SyntaxKind {
    // Token kinds (leaves)
    WHITESPACE,
    NEWLINE,
    COMMENT,
    BRACKET_COMMENT,
    COMMAND_NAME,
    LPAREN,
    RPAREN,
    UNQUOTED_ARGUMENT,
    QUOTED_ARGUMENT,
    BRACKET_ARGUMENT,
    VARIABLE_REF,
    ENV_VAR_REF,
    CACHE_VAR_REF,
    GENERATOR_EXPR,
    ESCAPE_SEQUENCE,
    EOF,

    // Composite node kinds (for parser in Plan 02)
    FILE,
    COMMAND_INVOCATION,
    ARGUMENT_LIST,
    QUOTED_ELEMENT,
    ERROR,

    // Sentinel for bounds checking
    __LAST,
}

impl fmt::Display for SyntaxKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CMakeLang;

impl rowan::Language for CMakeLang {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        assert!((raw.0 as usize) < (SyntaxKind::__LAST as usize));
        unsafe { std::mem::transmute(raw.0) }
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

pub type SyntaxNode = rowan::SyntaxNode<CMakeLang>;
pub type SyntaxToken = rowan::SyntaxToken<CMakeLang>;
