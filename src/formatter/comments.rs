use crate::cst::ArgumentList;
use crate::syntax_kind::SyntaxKind;
use crate::SyntaxNode;
use rowan::NodeOrToken;

/// Extract leading comments that appear before a node
/// Returns comments in source order
pub fn extract_leading_comments(node: &SyntaxNode) -> Vec<String> {
    let mut comments = Vec::new();
    let mut current = node.prev_sibling_or_token();

    // Walk backwards collecting comments
    let mut temp_comments = Vec::new();
    while let Some(prev) = current {
        match &prev {
            NodeOrToken::Token(token) => {
                match token.kind() {
                    SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
                        temp_comments.push(token.text().to_string());
                    }
                    SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE => {
                        // Continue walking through whitespace
                    }
                    _ => {
                        // Stop at any other token
                        break;
                    }
                }
            }
            NodeOrToken::Node(_) => {
                // Stop at any node
                break;
            }
        }
        current = prev.prev_sibling_or_token();
    }

    // Reverse to get source order
    temp_comments.reverse();
    comments.extend(temp_comments);

    comments
}

/// Extract a trailing comment that appears on the same line after a node
/// Returns Some(comment) if a comment appears before the next newline
pub fn extract_trailing_comment(node: &SyntaxNode) -> Option<String> {
    let mut current = node.next_sibling_or_token();

    while let Some(next) = current {
        match &next {
            NodeOrToken::Token(token) => {
                match token.kind() {
                    SyntaxKind::COMMENT => {
                        // Found a trailing comment
                        return Some(token.text().to_string());
                    }
                    SyntaxKind::WHITESPACE => {
                        // Continue through whitespace
                    }
                    SyntaxKind::NEWLINE => {
                        // Hit a newline before finding a comment
                        return None;
                    }
                    _ => {
                        // Hit another token type
                        return None;
                    }
                }
            }
            NodeOrToken::Node(_) => {
                // Hit a node
                return None;
            }
        }
        current = next.next_sibling_or_token();
    }

    None
}

/// Check if an argument list contains inline comments
pub fn has_inline_comments(arg_list: &ArgumentList) -> bool {
    for child in arg_list.syntax().children_with_tokens() {
        if let NodeOrToken::Token(token) = child {
            if matches!(token.kind(), SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT) {
                return true;
            }
        }
    }
    false
}
