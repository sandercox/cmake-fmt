use std::fmt;

/// Suppression directive types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    /// Disable formatting for subsequent commands
    Off,
    /// Re-enable formatting
    On,
    /// Skip formatting for the next command only
    Skip,
    /// Disable sorting for the next command only
    NoSort,
    /// Style override (key=value)
    Style { key: String, value: String },
}

/// Warnings produced during suppression tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuppressionWarning {
    /// A suppression region was opened but never closed
    UnclosedRegion { start_line: usize },
    /// An "on" directive appeared without a matching "off"
    UnmatchedOn { line: usize },
    /// An "off" directive appeared while already in a suppressed region
    NestedOff { line: usize },
}

impl fmt::Display for SuppressionWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SuppressionWarning::UnclosedRegion { start_line } => {
                write!(
                    f,
                    "line {}: warning: suppression region started here but never closed",
                    start_line
                )
            }
            SuppressionWarning::UnmatchedOn { line } => {
                write!(
                    f,
                    "line {}: warning: 'cmake-fmt: on' without matching 'cmake-fmt: off'",
                    line
                )
            }
            SuppressionWarning::NestedOff { line } => {
                write!(
                    f,
                    "line {}: warning: nested 'cmake-fmt: off' (already in suppressed region)",
                    line
                )
            }
        }
    }
}

/// Parse a comment string to detect suppression directives
///
/// Returns Some(Directive) if the comment contains a valid directive,
/// None otherwise.
///
/// Supports both spaced and compact forms:
/// - "# cmake-fmt: off" or "# cmake-fmt:off"
/// - "# cmake-fmt: on" or "# cmake-fmt:on"
/// - "# cmake-fmt: skip" or "# cmake-fmt:skip"
pub fn parse_directive(comment: &str) -> Option<Directive> {
    // Strip leading '#' and trim
    let content = comment.strip_prefix('#')?.trim();

    // Check for directive prefix, in both the compact and the spaced spelling
    let after_prefix = content
        .strip_prefix("cmake-fmt:")
        .or_else(|| content.strip_prefix("cmake-fmt :"))?
        .trim();

    // Match directive type
    match after_prefix {
        "off" => Some(Directive::Off),
        "on" => Some(Directive::On),
        "skip" => Some(Directive::Skip),
        "no-sort" => Some(Directive::NoSort),
        _ => {
            // Check for style override (key=value)
            if let Some(eq_pos) = after_prefix.find('=') {
                let key = after_prefix[..eq_pos].trim();
                let value = after_prefix[eq_pos + 1..].trim();
                if !key.is_empty() && !value.is_empty() {
                    return Some(Directive::Style {
                        key: key.to_string(),
                        value: value.to_string(),
                    });
                }
            }
            None
        }
    }
}

/// Tracks suppression state during formatting
pub struct SuppressionTracker {
    /// Whether we are currently in a suppressed region
    active: bool,
    /// Line number where the current suppression region started (1-indexed)
    start_line: Option<usize>,
    /// Whether the next command should be skipped
    skip_next: bool,
    /// Whether sorting should be skipped for the next command
    skip_sort_next: bool,
    /// Accumulated warnings
    warnings: Vec<SuppressionWarning>,
}

impl SuppressionTracker {
    /// Create a new tracker with suppression disabled
    pub fn new() -> Self {
        Self {
            active: false,
            start_line: None,
            skip_next: false,
            skip_sort_next: false,
            warnings: Vec::new(),
        }
    }

    /// Process a directive and update state
    /// Note: Style directives should not be passed to this method
    pub fn process_directive(&mut self, directive: Directive, line: usize) {
        match directive {
            Directive::Off => {
                if self.active {
                    // Nested off - warn but don't change state
                    self.warnings.push(SuppressionWarning::NestedOff { line });
                } else {
                    // Start suppression
                    self.active = true;
                    self.start_line = Some(line);
                }
            }
            Directive::On => {
                if self.active {
                    // End suppression
                    self.active = false;
                    self.start_line = None;
                } else {
                    // Unmatched on - warn
                    self.warnings.push(SuppressionWarning::UnmatchedOn { line });
                }
            }
            Directive::Skip => {
                // Set skip flag for next command
                self.skip_next = true;
            }
            Directive::NoSort => {
                // Set no-sort flag for next command
                self.skip_sort_next = true;
            }
            Directive::Style { .. } => {
                // Style directives are not suppression directives
                // They should be handled separately in format_file
            }
        }
    }

    /// Check if formatting is currently suppressed
    pub fn is_suppressed(&self) -> bool {
        self.active
    }

    /// Check if the next command should be skipped
    pub fn should_skip_next(&self) -> bool {
        self.skip_next
    }

    /// Clear the skip flag (after consuming it)
    pub fn clear_skip(&mut self) {
        self.skip_next = false;
    }

    /// Check if sorting should be skipped for the next command
    pub fn should_skip_sort_next(&self) -> bool {
        self.skip_sort_next
    }

    /// Clear the skip sort flag (after consuming it)
    pub fn clear_skip_sort(&mut self) {
        self.skip_sort_next = false;
    }

    /// Finalize tracking and check for unclosed regions
    pub fn finalize(&mut self) {
        // Unclosed regions at end of file are intentional - no warning needed.
        // Users can use "cmake-fmt: off" to suppress formatting for the rest of the file.
    }

    /// Consume the tracker and return accumulated warnings
    pub fn into_warnings(self) -> Vec<SuppressionWarning> {
        self.warnings
    }
}

/// Convert byte offset from rowan to 1-indexed line number
///
/// Counts newlines in source[..byte_offset] and adds 1
pub fn line_number_at_offset(source: &str, byte_offset: usize) -> usize {
    let prefix = &source[..byte_offset.min(source.len())];
    prefix.matches('\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_directive_off() {
        assert_eq!(parse_directive("# cmake-fmt: off"), Some(Directive::Off));
        assert_eq!(parse_directive("# cmake-fmt:off"), Some(Directive::Off));
        assert_eq!(parse_directive("#cmake-fmt: off"), Some(Directive::Off));
        assert_eq!(parse_directive("#cmake-fmt:off"), Some(Directive::Off));
    }

    #[test]
    fn test_parse_directive_on() {
        assert_eq!(parse_directive("# cmake-fmt: on"), Some(Directive::On));
        assert_eq!(parse_directive("# cmake-fmt:on"), Some(Directive::On));
    }

    #[test]
    fn test_parse_directive_skip() {
        assert_eq!(parse_directive("# cmake-fmt: skip"), Some(Directive::Skip));
        assert_eq!(parse_directive("# cmake-fmt:skip"), Some(Directive::Skip));
    }

    #[test]
    fn test_parse_directive_no_sort() {
        assert_eq!(
            parse_directive("# cmake-fmt: no-sort"),
            Some(Directive::NoSort)
        );
        assert_eq!(
            parse_directive("# cmake-fmt:no-sort"),
            Some(Directive::NoSort)
        );
    }

    #[test]
    fn test_parse_directive_space_before_colon() {
        // The `cmake-fmt :` spelling is a separate arm from `cmake-fmt:`, and
        // every other test here uses the latter. Without this, dropping the
        // arm leaves the suite green while every spaced directive silently
        // stops being a directive and formats as an ordinary comment.
        assert_eq!(parse_directive("# cmake-fmt : off"), Some(Directive::Off));
        assert_eq!(parse_directive("# cmake-fmt : on"), Some(Directive::On));
        assert_eq!(parse_directive("# cmake-fmt : skip"), Some(Directive::Skip));
        assert_eq!(
            parse_directive("# cmake-fmt : no-sort"),
            Some(Directive::NoSort)
        );
        assert_eq!(
            parse_directive("# cmake-fmt :indent_width=4"),
            Some(Directive::Style {
                key: "indent_width".to_string(),
                value: "4".to_string(),
            })
        );
        // Only one space is accepted, and only before the colon.
        assert_eq!(parse_directive("# cmake-fmt  : off"), None);
        assert_eq!(parse_directive("# cmake -fmt: off"), None);
    }

    #[test]
    fn test_parse_directive_non_directive() {
        assert_eq!(parse_directive("# regular comment"), None);
        assert_eq!(parse_directive("# cmake-fmt: invalid"), None);
        assert_eq!(parse_directive("# cmake-format: off"), None);
        assert_eq!(parse_directive("not a comment"), None);
    }

    #[test]
    fn test_suppression_tracker_basic_region() {
        let mut tracker = SuppressionTracker::new();
        assert!(!tracker.is_suppressed());

        // Start suppression
        tracker.process_directive(Directive::Off, 10);
        assert!(tracker.is_suppressed());

        // End suppression
        tracker.process_directive(Directive::On, 20);
        assert!(!tracker.is_suppressed());

        tracker.finalize();
        let warnings = tracker.into_warnings();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_suppression_tracker_unclosed_region() {
        let mut tracker = SuppressionTracker::new();

        tracker.process_directive(Directive::Off, 10);
        assert!(tracker.is_suppressed());

        tracker.finalize();
        let warnings = tracker.into_warnings();
        // Unclosed regions no longer produce warnings - it's valid to leave cmake-fmt: off open
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn test_suppression_tracker_unmatched_on() {
        let mut tracker = SuppressionTracker::new();

        tracker.process_directive(Directive::On, 5);
        assert!(!tracker.is_suppressed());

        tracker.finalize();
        let warnings = tracker.into_warnings();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0], SuppressionWarning::UnmatchedOn { line: 5 });
    }

    #[test]
    fn test_suppression_tracker_nested_off() {
        let mut tracker = SuppressionTracker::new();

        tracker.process_directive(Directive::Off, 10);
        assert!(tracker.is_suppressed());

        tracker.process_directive(Directive::Off, 15);
        assert!(tracker.is_suppressed()); // Still suppressed

        tracker.finalize();
        let warnings = tracker.into_warnings();
        // Should have: nested off warning only (unclosed regions no longer warn)
        assert_eq!(warnings.len(), 1);
        assert!(warnings.contains(&SuppressionWarning::NestedOff { line: 15 }));
    }

    #[test]
    fn test_suppression_tracker_skip() {
        let mut tracker = SuppressionTracker::new();

        assert!(!tracker.should_skip_next());

        tracker.process_directive(Directive::Skip, 10);
        assert!(tracker.should_skip_next());

        tracker.clear_skip();
        assert!(!tracker.should_skip_next());
    }

    #[test]
    fn test_line_number_at_offset() {
        let source = "line 1\nline 2\nline 3\n";
        assert_eq!(line_number_at_offset(source, 0), 1); // Start of file
        assert_eq!(line_number_at_offset(source, 6), 1); // Before first \n
        assert_eq!(line_number_at_offset(source, 7), 2); // After first \n
        assert_eq!(line_number_at_offset(source, 14), 3); // After second \n
        assert_eq!(line_number_at_offset(source, 100), 4); // Beyond end (clamped)
    }

    #[test]
    fn test_warning_display() {
        let w1 = SuppressionWarning::UnclosedRegion { start_line: 42 };
        assert_eq!(
            w1.to_string(),
            "line 42: warning: suppression region started here but never closed"
        );

        let w2 = SuppressionWarning::UnmatchedOn { line: 10 };
        assert_eq!(
            w2.to_string(),
            "line 10: warning: 'cmake-fmt: on' without matching 'cmake-fmt: off'"
        );

        let w3 = SuppressionWarning::NestedOff { line: 20 };
        assert_eq!(
            w3.to_string(),
            "line 20: warning: nested 'cmake-fmt: off' (already in suppressed region)"
        );
    }

    #[test]
    fn test_parse_directive_style() {
        // Basic style override
        assert_eq!(
            parse_directive("# cmake-fmt: indent_width=2"),
            Some(Directive::Style {
                key: "indent_width".to_string(),
                value: "2".to_string(),
            })
        );

        // Compact form
        assert_eq!(
            parse_directive("# cmake-fmt:indent_width=2"),
            Some(Directive::Style {
                key: "indent_width".to_string(),
                value: "2".to_string(),
            })
        );

        // Spaces around equals
        assert_eq!(
            parse_directive("# cmake-fmt: indent_width = 2"),
            Some(Directive::Style {
                key: "indent_width".to_string(),
                value: "2".to_string(),
            })
        );

        // Empty value returns None
        assert_eq!(parse_directive("# cmake-fmt: indent_width="), None);

        // No equals returns None (not a style override)
        assert_eq!(parse_directive("# cmake-fmt: indent_width"), None);
    }
}
