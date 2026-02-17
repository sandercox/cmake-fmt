# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0] - 2026-02-17
### Changed
- Upgraded to Rust edition 2024

## [0.5.0] - 2026-02-13
### Added
- VS Code extension with format-on-save and format-selection
- `--assume-filename` flag for stdin config/grammar resolution
- Parallel file processing with rayon for multi-file formatting
- `--line-ranges=START:END` for partial formatting
- Extensionless `.cmake-fmt` config file support (TOML)
- JSON Schema validation for config files in VS Code
- GitHub Actions CI for multi-platform VSIX builds
- `sort_sources` option for auto-sorting filenames
- `final_newline` configuration option
- `comment_style` configuration for comment whitespace normalization
- `disable_format` option for per-directory opt-out
- Recursive `.cmake-fmt` config resolution with root-first merging
- BinPack keyword rendering with `sub_keywords` for complex grammars

### Fixed
- Trailing comment preservation in keyword-aware formatting
- `cmake-fmt:off` now suppresses comment normalization

## [0.4.1] - 2026-02-12
### Added
- cmake_parse_arguments extraction for user-defined command grammars
- Manual grammar definitions in `.cmake-fmt` config for custom commands
- Keyword-aware try-flat-first (don't break commands that fit on one line)
- set_source_files_properties PROPERTIES pair grammar
- Full multiline keyword sections
- Source file grouping (.h/.cpp pairs on same line)

### Fixed
- Stack overflow on large real-world files

## [0.4.0] - 2026-02-10
### Added
- Updated builtin command registry to CMake 4.2.3 (~120 commands)
- Command grammar system with flag, single-value, and multi-value keywords
- Per-mode formatting for complex commands (install, file, string, list)
- Project-wide user command scanning from function/macro definitions
- `force_break_keywords` configuration option
- Custom command all-or-nothing line breaking

## [0.3.0] - 2026-02-10
### Added
- `# cmake-fmt: off` / `# cmake-fmt: on` / `# cmake-fmt: skip` suppression comments
- Interactive mode (`--interactive`) for per-section diff review
- Split `command_case` into builtin vs user-defined with infer mode

### Changed
- Renamed binary and config files from `cmake-format` to `cmake-fmt`

## [0.2.0] - 2026-02-10
### Added
- Line ending configuration (`auto`/`lf`/`crlf`) with auto-detection
- Block closer modes (`leave`/`remove`/`force`) for endif(), endforeach(), etc.
- Comments inside argument lists preserved
- Blank lines inside argument lists preserved (manual grouping honored)
- Multiline commands stay multiline (respect author's layout choice)
- First argument on same line as command name for builtins

### Fixed
- Comment indentation now respects tabs/spaces config inside blocks
- Edge cases: generator expressions, nested variables, bracket args, comment placement

## [0.1.0] - 2026-02-09
### Added
- `--diff` flag showing unified diff format
- Colored diff output with TTY detection
- Patch-compatible diff headers
- Exit codes for CI integration (0 = no changes, 1 = changes found)

## [0.0.0] - 2026-02-09
### Added
- CMake parser with lossless Concrete Syntax Tree (rowan)
- Configurable max line length with intelligent line breaking
- Keyword-aware indentation (PUBLIC, PRIVATE on own line, values double-indented)
- Short lines stay on one line automatically
- Indentation normalization (spaces/tabs, configurable depth)
- Command casing enforcement (lowercase/uppercase/leave)
- Blank line rules (spacing between commands/sections)
- Config file support (`.cmake-format.toml` and `.cmake-format.yaml`)
- clang-format-style CLI (`-i`, `--check`/`--dry-run`, file globs, stdin/stdout)
- Semantic preservation (formatted output is semantically identical)
- Idempotency (format twice = format once)
- Fixture-based test suite
