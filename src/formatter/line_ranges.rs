use crate::formatter::{
    FormatConfig, FormatWarning,
    config::{FinalNewline, LineEnding},
    detect_line_ending, format_text_with_diagnostics_and_path,
};
use std::path::Path;

/// Represents a line range (1-based, inclusive)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

impl LineRange {
    /// Check if a line number (1-based) is contained within this range
    pub fn contains(&self, line: usize) -> bool {
        line >= self.start && line <= self.end
    }
}

/// Parse line ranges from a string like "1:5,10:15,20:20"
///
/// Returns a Vec of LineRange structs or a descriptive error message
pub fn parse_line_ranges(input: &str) -> Result<Vec<LineRange>, String> {
    if input.trim().is_empty() {
        return Err("Line ranges cannot be empty".to_string());
    }

    let mut ranges = Vec::new();

    for part in input.split(',') {
        let part = part.trim();

        if part.is_empty() {
            continue;
        }

        let components: Vec<&str> = part.split(':').collect();

        if components.len() != 2 {
            return Err(format!(
                "Invalid range format '{}'. Expected START:END",
                part
            ));
        }

        let start_str = components[0].trim();
        let end_str = components[1].trim();

        let start = start_str
            .parse::<usize>()
            .map_err(|_| format!("Invalid start line '{}' in range '{}'", start_str, part))?;

        let end = end_str
            .parse::<usize>()
            .map_err(|_| format!("Invalid end line '{}' in range '{}'", end_str, part))?;

        if start < 1 {
            return Err(format!("Invalid range '{}' (start must be >= 1)", part));
        }

        if start > end {
            return Err(format!("Invalid range {} (start > end)", part));
        }

        ranges.push(LineRange { start, end });
    }

    if ranges.is_empty() {
        return Err("No valid ranges found".to_string());
    }

    Ok(ranges)
}

/// Format specific line ranges while preserving unselected lines byte-for-byte
///
/// Algorithm:
/// 1. Format the entire file
/// 2. Split both original and formatted into lines
/// 3. For each line, use formatted version if in range, otherwise use original
/// 4. Join with detected line ending and preserve final newline
pub fn format_with_line_ranges(
    input: &str,
    config: &FormatConfig,
    ranges: &[LineRange],
    file_path: Option<&Path>,
    verbose: bool,
) -> (String, Vec<FormatWarning>) {
    // Step 1: Format the full file
    let (formatted, warnings) =
        format_text_with_diagnostics_and_path(input, config, file_path, verbose);

    // Step 2: Split into lines
    let original_lines: Vec<&str> = input.lines().collect();
    let formatted_lines: Vec<&str> = formatted.lines().collect();

    // Step 3: Build result by selecting from ranges
    let mut result_lines = Vec::new();

    for (line_num, original_line) in original_lines.iter().enumerate() {
        let line_1based = line_num + 1;

        // Check if this line is in any range
        let in_range = ranges.iter().any(|r| r.contains(line_1based));

        if in_range {
            // Use formatted line (with bounds check)
            let line_to_use = formatted_lines.get(line_num).unwrap_or(original_line);
            result_lines.push(*line_to_use);
        } else {
            // Use original line
            result_lines.push(*original_line);
        }
    }

    // Step 4: Detect line ending from original input
    let line_ending = detect_line_ending(input);
    let separator = match line_ending {
        LineEnding::CrLf => "\r\n",
        LineEnding::Lf | LineEnding::Auto => "\n",
    };

    // Step 5: Join lines with detected separator
    let mut result = result_lines.join(separator);

    // Step 6: Preserve final newline behavior
    let input_had_newline = input.ends_with("\r\n") || input.ends_with('\n');
    match config.final_newline {
        FinalNewline::Force => {
            // Always ensure result ends with newline
            if !result.ends_with(separator) && !result.is_empty() {
                result.push_str(separator);
            }
        }
        FinalNewline::Remove => {
            // Strip trailing newline if present
            if result.ends_with(separator) {
                let trim_len = result.len() - separator.len();
                result.truncate(trim_len);
            }
        }
        FinalNewline::Preserve => {
            // Preserve input's trailing newline state
            if input_had_newline {
                if !result.ends_with(separator) && !result.is_empty() {
                    result.push_str(separator);
                }
            } else {
                // Input had no trailing newline - ensure result also doesn't
                while result.ends_with(separator) {
                    let trim_len = result.len() - separator.len();
                    result.truncate(trim_len);
                }
            }
        }
    }

    (result, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_range() {
        let result = parse_line_ranges("1:5").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], LineRange { start: 1, end: 5 });
    }

    #[test]
    fn test_parse_multiple_ranges() {
        let result = parse_line_ranges("1:2,5:10,15:15").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], LineRange { start: 1, end: 2 });
        assert_eq!(result[1], LineRange { start: 5, end: 10 });
        assert_eq!(result[2], LineRange { start: 15, end: 15 });
    }

    #[test]
    fn test_parse_invalid_format() {
        let err = parse_line_ranges("abc").unwrap_err();
        assert!(err.contains("Invalid range format"));
    }

    #[test]
    fn test_parse_inverted_range() {
        let err = parse_line_ranges("10:5").unwrap_err();
        assert!(err.contains("start > end"));
    }

    #[test]
    fn test_parse_zero_start() {
        let err = parse_line_ranges("0:5").unwrap_err();
        assert!(err.contains("start must be >= 1"));
    }

    #[test]
    fn test_line_range_contains() {
        let range = LineRange { start: 5, end: 10 };
        assert!(!range.contains(4));
        assert!(range.contains(5));
        assert!(range.contains(7));
        assert!(range.contains(10));
        assert!(!range.contains(11));
    }

    #[test]
    fn test_format_with_line_ranges_simple() {
        let input = "set(  FOO   bar)\nmessage(hello)\nset(  BAZ   qux)\n";
        let config = FormatConfig::default();
        let ranges = vec![LineRange { start: 1, end: 1 }];

        let (result, _warnings) = format_with_line_ranges(input, &config, &ranges, None, false);

        // First line should be formatted, others unchanged
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "set(FOO bar)");
        assert_eq!(lines[1], "message(hello)");
        assert_eq!(lines[2], "set(  BAZ   qux)");
    }

    #[test]
    fn test_format_with_line_ranges_preserves_final_newline() {
        let input = "set(  FOO   bar)\nmessage(hello)\n";
        let config = FormatConfig::default();
        let ranges = vec![LineRange { start: 1, end: 1 }];

        let (result, _warnings) = format_with_line_ranges(input, &config, &ranges, None, false);

        assert!(result.ends_with('\n'), "Should preserve final newline");
    }

    #[test]
    fn test_format_with_line_ranges_no_final_newline() {
        let input = "set(  FOO   bar)\nmessage(hello)";
        // Use Preserve mode to preserve input's trailing newline state
        let config = FormatConfig {
            final_newline: FinalNewline::Preserve,
            ..Default::default()
        };
        let ranges = vec![LineRange { start: 1, end: 1 }];

        let (result, _warnings) = format_with_line_ranges(input, &config, &ranges, None, false);

        assert!(
            !result.ends_with('\n'),
            "Preserve mode should not add final newline if original didn't have one"
        );
    }

    #[test]
    fn test_format_with_line_ranges_force_adds_newline() {
        let input = "set(  FOO   bar)\nmessage(hello)";
        let config = FormatConfig {
            final_newline: FinalNewline::Force,
            ..Default::default()
        };
        let ranges = vec![LineRange { start: 1, end: 1 }];

        let (result, _warnings) = format_with_line_ranges(input, &config, &ranges, None, false);

        assert!(
            result.ends_with('\n'),
            "Force mode should add trailing newline even if original didn't have one"
        );
    }

    #[test]
    fn test_format_with_line_ranges_remove_strips_newline() {
        let input = "set(  FOO   bar)\nmessage(hello)\n";
        let config = FormatConfig {
            final_newline: FinalNewline::Remove,
            ..Default::default()
        };
        let ranges = vec![LineRange { start: 1, end: 1 }];

        let (result, _warnings) = format_with_line_ranges(input, &config, &ranges, None, false);

        assert!(
            !result.ends_with('\n'),
            "Remove mode should strip trailing newline"
        );
    }
}
