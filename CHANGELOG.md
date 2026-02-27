# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
- Update README with `collapse_empty_flags` example
- `final_newline` now is `force`, `remove` or `preserve` (default)
- `inline_single_keyword` to keep simple source lists without excessive indentation
- `control_flow_space_before_paren` if true `if ()` or `if()`
- Change `leave` to `preserve` wherever it was used in styles (`leave` will still be accepted for backward compatibility)
- `space_between_command_parens` to control `set(SOURCES a.cpp)` vs `set( SOURCES a.cpp )`
- `indent_closing_paren` to control if closing parens should be indented or not when multiline
- Directory walking with recursive flag (`-r`) and `.cmake-fmt-ignore` file support

## [0.7.2] - 2026-02-20
### Fixed
- VS Code extension display name
- VS Code extension README
- `collapse_empty_flags` to remove empty flags from generated files

## [0.7.1] - 2026-02-20
### Fixed
- VS Code extension publisher name

## [0.7.0] - 2026-02-19
### Fixed
- Regression on multiline install TARGETS
- Regression disabling collapse_empty_flags moved library type to next line as well

## [0.7.0-beta.2] - 2026-02-17
### Added
- VS Code extension with format-on-save and format-selection
- cmake-fmt supports CMake 4.2.3
- initial released version