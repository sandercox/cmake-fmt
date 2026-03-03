use crate::SyntaxNode;
use crate::syntax_kind::SyntaxKind;
use rowan::NodeOrToken;

/// A leading comment with its text and whether a blank line preceded it
pub struct LeadingComment {
    pub text: String,
    /// True if there was a blank line between this comment and the previous
    /// comment (or between this comment and the preceding command, for the
    /// first comment in the group).
    pub blank_line_before: bool,
}

/// Extract leading comments that appear before a node
/// Returns comments in source order with blank-line metadata
pub fn extract_leading_comments(node: &SyntaxNode) -> Vec<LeadingComment> {
    let mut current = node.prev_sibling_or_token();

    // Walk backwards collecting comments and tracking newlines between them
    let mut temp_comments = Vec::new();
    let mut newline_count: usize = 0;
    while let Some(prev) = current {
        match &prev {
            NodeOrToken::Token(token) => {
                match token.kind() {
                    SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
                        // newline_count = newlines between this comment and the
                        // previously-collected item (walking toward the command)
                        let has_blank = newline_count >= 2;
                        temp_comments.push((token.text().to_string(), has_blank));
                        newline_count = 0;
                    }
                    SyntaxKind::NEWLINE => {
                        newline_count += 1;
                    }
                    SyntaxKind::WHITESPACE => {
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
    // After reversal: item[i].has_blank = blank line between item[i] and item[i+1]
    // (the gap toward the command). We want blank_line_before for each comment,
    // which is the gap between item[i-1] and item[i] = item[i-1].has_blank.
    temp_comments.reverse();

    let mut result = Vec::with_capacity(temp_comments.len());
    for (i, (text, has_blank_after)) in temp_comments.iter().enumerate() {
        let blank_line_before = if i == 0 {
            // First comment: blank line before it comes from the gap between the
            // previous command/content and this comment. That was tracked as the
            // LAST newline_count before we stopped (not stored in temp_comments).
            // We use the `newline_count` left over after the loop.
            newline_count >= 2
        } else {
            // Subsequent comments: blank line before = previous comment's has_blank_after
            temp_comments[i - 1].1
        };
        let _ = has_blank_after; // used implicitly by next iteration
        result.push(LeadingComment {
            text: text.clone(),
            blank_line_before,
        });
    }

    result
}

/// Extract a trailing comment that appears on the same line after a node
/// Returns Some(comment) if a comment appears before the next newline
pub fn extract_trailing_comment(node: &SyntaxNode) -> Option<String> {
    let mut current = node.next_sibling_or_token();

    while let Some(next) = current {
        match &next {
            NodeOrToken::Token(token) => {
                match token.kind() {
                    SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
                        // Found a trailing comment (before any newline)
                        return Some(token.text().to_string());
                    }
                    SyntaxKind::WHITESPACE => {
                        // Continue through whitespace (spaces/tabs only, not newlines)
                    }
                    SyntaxKind::NEWLINE => {
                        // Hit a newline before finding a comment - no trailing comment
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
