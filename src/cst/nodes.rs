use crate::SyntaxNode;
use crate::syntax_kind::SyntaxKind;
use rowan::SyntaxToken as RowanSyntaxToken;

/// Helper macro to create typed AST node wrappers
macro_rules! ast_node {
    ($name:ident, $kind:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name(SyntaxNode);

        impl $name {
            pub fn cast(node: SyntaxNode) -> Option<Self> {
                if node.kind() == SyntaxKind::$kind {
                    Some(Self(node))
                } else {
                    None
                }
            }

            pub fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

ast_node!(File, FILE);
ast_node!(CommandInvocation, COMMAND_INVOCATION);
ast_node!(ArgumentList, ARGUMENT_LIST);

impl File {
    /// Iterate over all command invocations in the file
    pub fn commands(&self) -> impl Iterator<Item = CommandInvocation> + '_ {
        self.0.children().filter_map(CommandInvocation::cast)
    }
}

impl CommandInvocation {
    /// Get the command name token
    pub fn command_name(&self) -> Option<RowanSyntaxToken<crate::syntax_kind::CMakeLang>> {
        self.0
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|token| token.kind() == SyntaxKind::COMMAND_NAME)
    }

    /// Get the command name as a string
    pub fn name_text(&self) -> Option<String> {
        self.command_name().map(|token| token.text().to_string())
    }

    /// Get the argument list node
    pub fn argument_list(&self) -> Option<ArgumentList> {
        self.0.children().find_map(ArgumentList::cast)
    }
}

impl ArgumentList {
    /// Iterate over all argument tokens
    pub fn arguments(
        &self,
    ) -> impl Iterator<Item = RowanSyntaxToken<crate::syntax_kind::CMakeLang>> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .filter(|token| {
                matches!(
                    token.kind(),
                    SyntaxKind::UNQUOTED_ARGUMENT
                        | SyntaxKind::QUOTED_ARGUMENT
                        | SyntaxKind::BRACKET_ARGUMENT
                        | SyntaxKind::VARIABLE_REF
                        | SyntaxKind::ENV_VAR_REF
                        | SyntaxKind::CACHE_VAR_REF
                        | SyntaxKind::GENERATOR_EXPR
                )
            })
    }

    /// Iterate over nested argument lists
    pub fn nested_lists(&self) -> impl Iterator<Item = ArgumentList> + '_ {
        self.0.children().filter_map(ArgumentList::cast)
    }
}
