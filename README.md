# cmake-fmt

[![crates.io](https://img.shields.io/crates/v/cmake-fmt)](https://crates.io/crates/cmake-fmt)
[![downloads](https://img.shields.io/crates/d/cmake-fmt)](https://crates.io/crates/cmake-fmt)

Fast CMake formatter with grammar-aware line wrapping and intelligent indentation.

## Examples

### Line Wrapping & Indentation

**Before:**
```cmake
TARGET_LINK_LIBRARIES(myapp PUBLIC Qt6::Core Qt6::Widgets Qt6::Network Boost::system Boost::filesystem PRIVATE spdlog::spdlog fmt::fmt nlohmann_json::nlohmann_json OpenSSL::SSL OpenSSL::Crypto CURL::libcurl)
```

**After:**
```cmake
target_link_libraries(myapp
	PUBLIC
		Qt6::Core
		Qt6::Widgets
		Qt6::Network
		Boost::system
		Boost::filesystem
	PRIVATE
		spdlog::spdlog
		fmt::fmt
		nlohmann_json::nlohmann_json
		OpenSSL::SSL
		OpenSSL::Crypto
		CURL::libcurl
)
```

### Command Casing & Grammar Awareness

**Before:**
```cmake
FIND_PACKAGE(Boost REQUIRED COMPONENTS system filesystem thread)
INSTALL(TARGETS myapp mylib RUNTIME DESTINATION bin LIBRARY DESTINATION lib ARCHIVE DESTINATION lib/static)
```

**After:**
```cmake
find_package(Boost REQUIRED COMPONENTS system filesystem thread)
install(TARGETS myapp mylib RUNTIME DESTINATION bin LIBRARY DESTINATION lib ARCHIVE DESTINATION lib/static)
```

## Installation

### VS Code Extension

Install from the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=cmake-fmt.cmake-fmt) (coming soon).

Enable format-on-save by adding this to your `settings.json`:

```json
{
  "[cmake]": {
    "editor.defaultFormatter": "cmake-fmt.cmake-fmt",
    "editor.formatOnSave": true
  }
}
```

### Cargo

```bash
cargo install cmake-fmt
```

### Pre-built Binaries

Download pre-built binaries from [GitHub Releases](https://github.com/sandercox/cmake-fmt/releases) (coming soon).

Supported platforms:
- Linux x64
- Linux ARM64
- macOS x64 (Intel)
- macOS ARM64 (Apple Silicon)
- Windows x64
- Windows ARM64

## Usage

Format a file to stdout:
```bash
cmake-fmt CMakeLists.txt
```

Format in-place:
```bash
cmake-fmt -i CMakeLists.txt
```

Check if files are formatted (exits with code 1 if changes needed):
```bash
cmake-fmt --check CMakeLists.txt
```

Show formatting diff:
```bash
cmake-fmt --diff CMakeLists.txt
```

Use in CI to enforce formatting:
```bash
cmake-fmt --check $(find . -name 'CMakeLists.txt' -o -name '*.cmake')
```

When going through your files for the first time, you can use `--interactive` to review changes hunk-by-hunk and choose which ones to apply or skip/disable.

## Configuration

cmake-fmt searches for config files upward from the formatted file's directory. Config file names (in priority order):
1. `.cmake-fmt` (YAML format)
2. `.cmake-fmt.yml`
3. `.cmake-fmt.yaml`
4. `.cmake-fmt.tml`
5. `.cmake-fmt.toml`

Multiple config files can coexist — root-level defaults are merged with directory-level overrides.

### Example Config

```toml
indent_width = 2
use_tabs = false
command_case = "lowercase"
max_line_length = 120
```

## Configuration Reference

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `disable_format` | boolean | `false` | Skip formatting entirely |
| `indent_width` | integer | `4` | Number of spaces per indent level |
| `max_line_length` | integer | `80` | Max line length (0 = unlimited) |
| `use_tabs` | boolean | `true` | Use tabs for indentation |
| `command_case` | enum | `lowercase` | Built-in command casing: `lowercase`, `uppercase`, `leave` |
| `user_command_case` | enum | `infer` | User-defined command casing: `lowercase`, `uppercase`, `leave`, `infer` |
| `max_blank_lines` | integer | `1` | Maximum consecutive blank lines allowed |
| `line_ending` | enum | `auto` | Line ending style: `auto`, `lf`, `crlf` |
| `closing_style` | enum | `remove` | Closing statement style: `leave`, `remove`, `force` |
| `force_break_keywords` | boolean | `false` | Always break keywords onto separate lines |
| `final_newline` | boolean | `true` | Ensure file ends with newline |
| `comment_style` | enum | `hash_space` | Comment formatting: `leave`, `hash_space`, `hash_no_space` |
| `source_grouping` | enum | `none` | Group source files: `none`, `headers_first`, `sources_first` |
| `sort_sources` | enum | `none` | Sort source file lists: `none`, `alphabetical` |

### Per-Setting Examples

#### `indent_width`

Controls spaces per indentation level (when `use_tabs=false`).

**`indent_width = 2`:**
```cmake
if(BUILD_TESTING)
  add_subdirectory(tests)
endif()
```

**`indent_width = 4` (default):**
```cmake
if(BUILD_TESTING)
    add_subdirectory(tests)
endif()
```

#### `max_line_length`

Controls when lines are wrapped. Setting `0` disables line length limits.

**`max_line_length = 40`:**
```cmake
target_link_libraries(app
	PUBLIC
		Boost::system
		Boost::filesystem
		Qt6::Core
)
```

**`max_line_length = 80` (default):**
```cmake
target_link_libraries(app PUBLIC Boost::system Boost::filesystem Qt6::Core)
```

#### `command_case`

Controls casing for built-in CMake commands.

**`command_case = lowercase` (default):**
```cmake
add_executable(myapp main.cpp)
target_link_libraries(myapp PUBLIC Boost::system)
```

**`command_case = uppercase`:**
```cmake
ADD_EXECUTABLE(myapp main.cpp)
TARGET_LINK_LIBRARIES(myapp PUBLIC Boost::system)
```

**`command_case = leave`:**
Preserves original casing from source file.

#### `use_tabs`

**`use_tabs = true` (default):**
Uses tab characters for indentation.

**`use_tabs = false`:**
Uses spaces for indentation (count controlled by `indent_width`).

#### `closing_style`

Controls whether closing statements like `endif()` include the condition.

**`closing_style = remove` (default):**
```cmake
if(BUILD_TESTING)
	enable_testing()
endif()
```

**`closing_style = force`:**
```cmake
if(BUILD_TESTING)
	enable_testing()
endif(BUILD_TESTING)
```

**`closing_style = leave`:**
Preserves original closing style from source file.

#### `force_break_keywords`

When `true`, always breaks keywords onto their own lines, even if the command would fit on one line.

**`force_break_keywords = false` (default):**
```cmake
target_link_libraries(app PUBLIC Boost::system)
```

**`force_break_keywords = true`:**
```cmake
target_link_libraries(app
	PUBLIC
		Boost::system
)
```

#### `comment_style`

Controls comment formatting.

**`comment_style = hash_space` (default):**
Ensures one space after `#`:
```cmake
# This is a comment
```

**`comment_style = hash_no_space`:**
Removes space after `#`:
```cmake
#This is a comment
```

**`comment_style = leave`:**
Preserves original comment formatting.

#### `max_blank_lines`

Limits consecutive blank lines. For example, `max_blank_lines = 1` (default) allows at most one blank line between statements.

#### `final_newline`

When `true` (default), ensures file ends with a newline character.

#### `disable_format`

When `true`, skips formatting entirely. Useful for temporarily disabling formatting in specific directories via directory-level config.

#### `sort_sources`

**`sort_sources = alphabetical`:**
Sorts file lists in commands like `add_executable()` alphabetically.

**`sort_sources = none` (default):**
Preserves original file order.

#### `source_grouping`

**`source_grouping = headers_first`:**
Groups header files (`.h`, `.hpp`) before source files (`.cpp`, `.c`) in commands like `add_executable()` but also `set()` when file lists are detected.

```cmake
set(SOURCES
    main.h main.cpp
    application.h application.cpp
)
```

**`source_grouping = sources_first`:**
Groups source files before header files.

**`source_grouping = none` (default):**
No grouping applied.

### Custom Command Grammars

The `command_grammars` setting is only available in config files (not via `--style`). It allows you to define custom command grammars for project-specific CMake functions.

**Example `.cmake-fmt`:**
```yaml
custom_grammer:
  my_custom_command:
    options:
      - VERBOSE
    one_value_keywords:
      - NAME
      - VERSION
    multi_value_keywords:
      - SOURCES
      - HEADERS
```

For detailed grammar syntax, run:
```bash
cmake-fmt --help-grammar
```

## CLI Reference

| Flag | Description |
|------|-------------|
| `-i`, `--in-place` | Format files in-place |
| `--check` | Check if files are formatted (exit 1 if changes needed) |
| `--dry-run` | Dry run mode (same as --check) |
| `--diff` | Show diff of formatting changes |
| `--interactive` | Interactive mode: review formatting changes hunk-by-hunk |
| `--style <STYLE>` | Override config inline (e.g., "indent_width=4,max_line_length=100") |
| `--export-grammar <FILE>` | Export detected custom grammars to a file (scanned from input files) |
| `--export-all-grammar <FILE>` | Export all grammars including builtins to a file |
| `--grammar-file <FILE>` | Import additional grammar file(s) (can be specified multiple times) |
| `--verbose` | Show verbose output during file scanning and analysis |
| `--assume-filename <PATH>` | Treat stdin as if formatting this file (resolves config/grammar from its path) |
| `--line-ranges <RANGES>` | Format only specific line ranges (e.g., "1:5,10:15") |
| `--help-style` | Show all available style settings |
| `--help-grammar` | Show grammar file format and keyword types |
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

## License

MIT — see [LICENSE](LICENSE) for details.
