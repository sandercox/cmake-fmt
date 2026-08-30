//! `--help-style` and `--help-grammar` describe the tool to its users, and every
//! claim in them is checkable against the code. Four were wrong at once in an
//! earlier round — a default, a round-trip that does not work, and two
//! descriptions of what a setting reaches — and nothing would have noticed any
//! of them going wrong again.

use cmake_fmt::formatter::FormatConfig;
use std::process::Command;

fn cmake_fmt_bin() -> String {
    env!("CARGO_BIN_EXE_cmake-fmt").to_string()
}

fn help(flag: &str) -> String {
    let output = Command::new(cmake_fmt_bin())
        .arg(flag)
        .output()
        .unwrap_or_else(|e| panic!("running {} failed: {}", flag, e));
    assert!(output.status.success(), "{} exited non-zero", flag);
    String::from_utf8_lossy(&output.stdout).to_string() + &String::from_utf8_lossy(&output.stderr)
}

/// Whether `--style <key>=…` is refused as an unknown config key.
///
/// Needs a real file to format: `--help-style` and the other informational modes
/// return before `--style` is parsed, so probing with one of those made this
/// check unfailable.
fn style_rejects(key: &str) -> bool {
    let tempdir = tempfile::TempDir::new().expect("tempdir");
    let path = tempdir.path().join("CMakeLists.txt");
    std::fs::write(&path, "set(A b)\n").expect("write");
    let output = Command::new(cmake_fmt_bin())
        .args([
            "--style",
            &format!("{}=2", key),
            "--check",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("run");
    String::from_utf8_lossy(&output.stderr).contains("Unknown config key")
}

/// The default column of one `--help-style` row.
fn stated_default(help: &str, setting: &str) -> String {
    // Only the `--style` table: the "Config file only" block below it uses the
    // same shape, and matching that would call a config-file-only key a
    // `--style` setting.
    let table = help.split("CLI usage:").next().unwrap_or(help);
    let Some(line) = table
        .lines()
        .find(|line| line.trim_start().starts_with(&format!("{} ", setting)))
    else {
        return String::new();
    };
    let fields: Vec<&str> = line.split_whitespace().collect();
    fields
        .get(2)
        .unwrap_or_else(|| panic!("row for {} has no default column: {}", setting, line))
        .to_string()
}

#[test]
fn test_help_style_states_the_real_defaults() {
    // `final_newline` was documented as `force` while the code default is
    // `preserve` — observable on a file with no trailing newline, and wrong
    // since before this branch. These are checked against `FormatConfig` itself
    // rather than against a second list, so there is nothing to keep in step.
    let help = help("--help-style");
    let defaults = FormatConfig::default();

    for setting in ["final_newline", "indent_width", "max_line_length"] {
        assert!(
            !stated_default(&help, setting).is_empty(),
            "--help-style lost its row for {}",
            setting
        );
    }
    assert_eq!(
        stated_default(&help, "final_newline"),
        format!("{:?}", defaults.final_newline).to_lowercase()
    );
    assert_eq!(
        stated_default(&help, "indent_width"),
        defaults.indent_width.to_string()
    );
    assert_eq!(
        stated_default(&help, "max_line_length"),
        defaults.max_line_length.to_string()
    );
    assert_eq!(
        stated_default(&help, "use_tabs"),
        defaults.use_tabs.to_string()
    );
    assert_eq!(
        stated_default(&help, "collapse_empty_flags"),
        defaults.collapse_empty_flags.to_string()
    );
    assert_eq!(
        stated_default(&help, "inline_single_keyword"),
        defaults.inline_single_keyword.to_string()
    );
    assert_eq!(
        stated_default(&help, "space_between_command_parens"),
        defaults.space_between_command_parens.to_string()
    );
    assert_eq!(
        stated_default(&help, "control_flow_space_before_paren"),
        defaults.control_flow_space_before_paren.to_string()
    );
    assert_eq!(
        stated_default(&help, "indent_closing_paren"),
        defaults.indent_closing_paren.to_string()
    );
}

#[test]
fn test_help_style_lists_every_setting_style_accepts() {
    // The table says it is the list of settings; three that parse fine were
    // missing from it, including two this branch's own work reads.
    let help = help("--help-style");
    for setting in [
        "disable_format",
        "indent_width",
        "max_line_length",
        "use_tabs",
        "command_case",
        "user_command_case",
        "max_blank_lines",
        "line_ending",
        "final_newline",
        "comment_style",
        "source_grouping",
        "sort_sources",
        "space_between_command_parens",
        "control_flow_space_before_paren",
        "indent_closing_paren",
        "collapse_empty_flags",
        "inline_single_keyword",
    ] {
        // In the table, not merely somewhere in the dump — the example footer
        // mentions several settings by name and would satisfy a substring test
        assert!(
            !stated_default(&help, setting).is_empty(),
            "--help-style has no table row for {}, which `--style` accepts:\n{}",
            setting,
            help
        );
        // ...and it really is accepted. The message is `Unknown config key`;
        // asserting on "Unknown style setting" — a string that appears nowhere
        // in the crate — made this half of the check unfailable, and adding
        // `command_grammars`, `root` or `grammar_files` to the list above passed
        // even though `--style` rejects all three.
        assert!(
            !style_rejects(setting),
            "{} is listed in the --style table but --style rejects it",
            setting
        );
    }
}

#[test]
fn test_help_style_lists_the_config_file_only_keys_and_lines_them_up() {
    // The "Config file only" block and the table's column alignment are both
    // things this branch edited and neither had a test: dropping either new row,
    // or reverting the key column from 31 back to 25 — which is what made the
    // two longest names overhang their column — all passed.
    let help = help("--help-style");
    let footer = help
        .split("Config file only")
        .nth(1)
        .unwrap_or_else(|| panic!("--help-style lost its config-file-only block:\n{}", help));

    for key in ["command_grammars", "grammar_files", "root"] {
        assert!(
            footer.contains(key),
            "the config-file-only block omits {}:\n{}",
            key,
            footer
        );
        // and it really is config-file-only. The probe needs a real file:
        // `--help-style` returns before `--style` is ever parsed.
        assert!(
            style_rejects(key),
            "{} is listed as config-file-only but `--style` accepts it",
            key
        );
    }

    // Every row's Type column starts at the same offset. The longest key is 31
    // characters, so a narrower field pushes those rows' remaining columns right
    // and nothing else's.
    let table = help.split("CLI usage:").next().unwrap_or(&help);
    let offsets: Vec<usize> = table
        .lines()
        .filter(|line| line.starts_with("  ") && line.contains("boolean"))
        .map(|line| line.find("boolean").unwrap())
        .collect();
    assert!(
        offsets.len() > 4,
        "expected several boolean rows to compare"
    );
    assert!(
        offsets.windows(2).all(|w| w[0] == w[1]),
        "the Type column does not line up across rows: offsets {:?}\n{}",
        offsets,
        table
    );
}

#[test]
fn test_help_grammar_says_the_multi_mode_export_does_not_round_trip() {
    // `--export-all-grammar` writes `mode:` entries and `--grammar-file` skips
    // every one of them with a warning, so the export does not round-trip. The
    // help documents the shape because that is what the export writes, and says
    // so — deleting that paragraph left the whole suite green.
    let help = help("--help-grammar");
    assert!(
        help.contains("NOT YET SUPPORTED ON IMPORT"),
        "--help-grammar no longer says that multi-mode entries do not import:\n{}",
        help
    );

    // and the claim is true
    let tempdir = tempfile::TempDir::new().expect("tempdir");
    let exported = tempdir.path().join("g.yaml");
    let output = Command::new(cmake_fmt_bin())
        .args(["--export-all-grammar", exported.to_str().unwrap()])
        .output()
        .expect("run");
    assert!(output.status.success());
    let text = std::fs::read_to_string(&exported).expect("read back");
    let modes = text.matches("\n  mode:").count();
    assert!(modes > 0, "the export wrote no mode entries at all");

    let source = tempdir.path().join("CMakeLists.txt");
    std::fs::write(&source, "set(A b)\n").expect("write");
    let output = Command::new(cmake_fmt_bin())
        .args([
            "--grammar-file",
            exported.to_str().unwrap(),
            source.to_str().unwrap(),
        ])
        .output()
        .expect("run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let skipped = stderr.matches("Skipping multi-mode grammar entry").count();
    assert_eq!(
        skipped, modes,
        "the export wrote {} mode entries and the import skipped {} — if these \
         now agree because import works, the help paragraph should go",
        modes, skipped
    );
}
