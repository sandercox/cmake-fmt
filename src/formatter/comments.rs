use crate::SyntaxNode;
use crate::syntax_kind::SyntaxKind;
use rowan::NodeOrToken;

/// A leading comment with its text and whether a blank line preceded it
pub struct LeadingComment {
    pub text: String,
    /// Where the comment token starts in the source.
    ///
    /// What identifies a comment, which its text does not: a file may hold the
    /// same `# note` twice, and the set of comments already emitted used to be
    /// keyed on the text, so the second copy was read as already handled and
    /// dropped. The content guard caught it and refused the file, which made a
    /// valid file unformattable rather than silently wrong.
    pub offset: usize,
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
                        temp_comments.push((
                            token.text().to_string(),
                            has_blank,
                            token.text_range().start().into(),
                        ));
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
    for (i, (text, has_blank_after, offset)) in temp_comments.iter().enumerate() {
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
            offset: *offset,
            blank_line_before,
        });
    }

    result
}

/// A trailing comment with its text and where it starts in the source.
pub struct TrailingComment {
    pub text: String,
    /// See [`LeadingComment::offset`].
    pub offset: usize,
}

/// Extract a trailing comment that appears on the same line after a node
/// Returns Some(comment) if a comment appears before the next newline
pub fn extract_trailing_comment(node: &SyntaxNode) -> Option<TrailingComment> {
    let mut current = node.next_sibling_or_token();

    while let Some(next) = current {
        match &next {
            NodeOrToken::Token(token) => {
                match token.kind() {
                    SyntaxKind::COMMENT | SyntaxKind::BRACKET_COMMENT => {
                        // Found a trailing comment (before any newline)
                        return Some(TrailingComment {
                            text: token.text().to_string(),
                            offset: token.text_range().start().into(),
                        });
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
