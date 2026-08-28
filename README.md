<p align="center">
  <img src="editors/vscode/icon.png" alt="cmake-fmt logo" width="128" height="128">
</p>

<h1 align="center">cmake-fmt</h1>

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

### Condition Wrapping

**Before:**
```cmake
if(NOT JUMBO_BUILD_MODE STREQUAL "BATCH" AND NOT JUMBO_BUILD_MODE STREQUAL "GROUP")
endif()
```

**After:**
```cmake
if(NOT JUMBO_BUILD_MODE STREQUAL "BATCH"
	AND NOT JUMBO_BUILD_MODE STREQUAL "GROUP"
)
endif()
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

Install from the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=paralleldimension.cmake-fmt).

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

### Docker

Pull from Docker Hub:
```bash
docker pull paralleldimension/cmake-fmt
```

Or from GitHub Container Registry:
```bash
docker pull ghcr.io/sandercox/cmake-fmt
```

Format files using Docker:
```bash
docker run --rm -v "$(pwd):/work" -w /work paralleldimension/cmake-fmt cmake-fmt -i CMakeLists.txt
```

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

Format all CMake files in the current directory:
```bash
cmake-fmt -i .
```

Recursively format an entire project:
```bash
cmake-fmt -ri src/
```

Use in CI to enforce formatting:
```bash
cmake-fmt --check -r .
```

Create a `.cmake-fmt-ignore` file (gitignore syntax) to exclude paths:
```
build/
third_party/
generated/*.cmake
```

When going through your files for the first time, you can use `--interactive` to review changes hunk-by-hunk and choose which ones to apply or skip/disable.

## Configuration

cmake-fmt searches for config files upward from the formatted file's directory. Config file names (in priority order):
1. `.cmake-fmt` (YAML format)
2. `.cmake-fmt.yml`
3. `.cmake-fmt.yaml`
4. `.cmake-fmt.tml`
5. `.cmake-fmt.toml`

Multiple config files can coexist — root-level defaults are merged with directory-level overrides. Add `root: true` to a config file to stop inheriting from parent directories.

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
| `root` | boolean | `false` | Stop searching parent directories for config files |
| `disable_format` | boolean | `false` | Skip formatting entirely |
| `indent_width` | integer | `4` | Number of spaces per indent level |
| `max_line_length` | integer | `80` | Max line length (0 = unlimited) |
| `use_tabs` | boolean | `true` | Use tabs for indentation |
| `command_case` | enum | `lowercase` | Built-in command casing: `lowercase`, `uppercase`, `preserve` |
| `user_command_case` | enum | `infer` | User-defined command casing: `lowercase`, `uppercase`, `preserve`, `infer` |
| `max_blank_lines` | integer | `1` | Maximum consecutive blank lines allowed |
| `line_ending` | enum | `auto` | Line ending style: `auto`, `lf`, `crlf` |
| `closing_style` | enum | `remove` | Closing statement style: `preserve`, `remove`, `force` |
| `force_break_keywords` | boolean | `false` | Always break keywords onto separate lines |
| `final_newline` | enum | `preserve` | Final newline: `preserve`, `remove`, `force` (also accepts `true`/`false`; `leave` accepted as alias for `preserve`) |
| `comment_style` | enum | `hash_space` | Comment formatting: `preserve`, `hash_space`, `hash_no_space` |
| `source_grouping` | enum | `none` | Group source files: `none`, `headers_first`, `sources_first` |
| `sort_sources` | enum | `none` | Sort source file lists: `none`, `alphabetical` |
| `collapse_empty_flags` | boolean | `true` | Collapse no-argument flags inline with preceding positional args |
| `inline_single_keyword` | boolean | `false` | Keep single keyword inline with args, single-indent values |
| `control_flow_space_before_paren` | boolean | `false` | Insert space before ( in control flow statements |
| `space_between_command_parens` | boolean | `false` | Insert space inside command parentheses |
| `indent_closing_paren` | boolean | `false` | Indent closing `)` one level in multiline commands |

### Per-Setting Examples

#### `root`

Prevents inheriting settings from `.cmake-fmt` files in parent directories. When `root: true` is set, cmake-fmt starts with default values and only applies settings from this file and any closer (child) config files.

This is useful for monorepos or subdirectories that need completely independent formatting rules.

**Without `root: true`** (default behavior):
```
/project/.cmake-fmt          # indent_width: 2, max_line_length: 100
/project/lib/.cmake-fmt      # use_tabs: false
# Files in /project/lib/ get: indent_width=2 + max_line_length=100 + use_tabs=false (merged)
```

**With `root: true`:**
```
/project/.cmake-fmt          # indent_width: 2, max_line_length: 100
/project/lib/.cmake-fmt      # root: true, use_tabs: false
# Files in /project/lib/ get: use_tabs=false + all other settings at defaults (parent ignored)
```

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

**`command_case = preserve`:**
Preserves original casing from source file. (`"leave"` is accepted as a backward-compatible alias.)

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

**`closing_style = preserve`:**
Preserves original closing style from source file. (`"leave"` is accepted as a backward-compatible alias.)

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

**`comment_style = preserve`:**
Preserves original comment formatting. (`"leave"` is accepted as a backward-compatible alias.)

#### `max_blank_lines`

Limits consecutive blank lines. For example, `max_blank_lines = 1` (default) allows at most one blank line between statements.

#### `final_newline`

**`final_newline = preserve` (default):**
Preserves the original file's trailing newline state. If the input file ended with a newline, the output will too. If it didn't, the output won't add one. (`"leave"` is accepted as a backward-compatible alias.)

**`final_newline = force`:**
Ensures the file ends with a newline character. Equivalent to the previous `true` setting.

**`final_newline = remove`:**
Strips any trailing newline from the output. Equivalent to the previous `false` setting.

For backward compatibility, `true` and `false` are still accepted (mapped to `force` and `remove` respectively).

#### `disable_format`

When `true`, skips formatting entirely. Useful for temporarily disabling formatting in specific directories via directory-level config.

#### `sort_sources`

**`sort_sources = alphabetical`:**
Sorts file lists alphabetically, case-insensitively.

**`sort_sources = none` (default):**
Preserves original file order.

#### Which lists get reordered

`sort_sources` and `source_grouping` both reorder arguments, so both apply only
where a list is known to be unordered:

| Command | What is sorted |
| --- | --- |
| `set(VAR a.cpp b.cpp)` | the values (not the `CACHE <type> "<docstring>"` form) |
| `add_library`, `add_executable` | the sources, after the target name |
| `target_sources` | `PUBLIC` / `PRIVATE` / `INTERFACE`, and `FILE_SET`'s `FILES` |
| `list(APPEND\|PREPEND\|REMOVE_ITEM var …)` | the elements |
| `install(FILES\|PROGRAMS …)` | the file list |
| `source_group(… FILES …)` | the file list |
| your own commands | keywords named `SOURCES`, `SRCS` or `FILES` — see the note below |

That last row is a *fallback*, and only for a `command_grammars` entry in `.cmake-fmt` or a grammar auto-detected from `cmake_parse_arguments` — it is what such a grammar gets when it says nothing about sortability. A grammar file loaded with `--grammar-file` has no fallback: a keyword there is sortable only if `sortable_keywords` names it.

Everything else is left alone, because argument order usually carries meaning:
`COMMAND` holds an argv, `PROPERTIES` holds key/value pairs, `file(RENAME a b)`
holds source then destination, `target_link_libraries` holds link order.

Four things are never reordered even inside a list that is:

- A variable reference or generator expression (`${GENERATED}`, `"${GENERATED}"`,
  `$<TARGET_OBJECTS:x>`) holds its position, and files do not move across it —
  what it expands to is unknown.
- A keyword-less run whose values don't all look like source files. A keyword
  names what its values are; `set(VAR ...)` does not, and the same shape holds
  sources in one project and compiler flags in the next. So flag shapes
  (`-Wall`, `/O2`, `--input`, `A=1`), library extensions (`.a`, `.so`, `.lib`)
  and extension-less names keep their order there. Under a keyword that vouches
  for them — `install(FILES README LICENSE ...)` — they still sort.
- A `list(APPEND ${DYNAMIC} …)` whose list variable is itself a reference, since
  the name cannot be read and therefore cannot be vetted. A dynamic *target*
  name does not block anything — `add_library(${PROJECT_NAME} a.cpp b.cpp)`
  still sorts.
- A list whose variable names a search path, flag list or argument list:
  `CMAKE_MODULE_PATH`, `CMAKE_PREFIX_PATH`, anything containing `FLAGS`, or a
  name ending in `_PATH`, `_PATHS`, `_DIRS`, `_DIRECTORIES`, `_OPTIONS`,
  `_ARGS`, `_ARGUMENTS`, `_LIBS`, `_LIBRARIES`, `_PATTERNS`. Matched
  case-insensitively, since project-local lists are often lowercase.

A positional list in a command cmake-fmt does not recognize is not reordered at
all — there is no grammar to say whether its order matters. Add a
[`command_grammars`](#custom-command-grammars) entry to opt one in.

To mark a list in your own command as unordered, name its keyword in
[`command_grammars`](#custom-command-grammars):

```yaml
command_grammars:
  my_add_library:
    one_value_keywords: [NAME]
    multi_value_keywords: [SOURCES, COMMAND]
    sortable_keywords: [SOURCES]   # COMMAND keeps its order
```

Use `sortable_positional: true` for a command whose keyword-less arguments are a
file list. It reaches two runs, and only two:

- The **leading** run, the arguments before the first keyword. Its first
  argument is always pinned, because in every command that has one it names the
  list or the target rather than belonging to it — so `my_cmd(x.cpp z.cpp a.cpp)`
  sorts to `x.cpp a.cpp z.cpp`, and a two-element list never reorders.
- The run that **overflows a leading single-value keyword**, with nothing
  pinned, because that keyword already consumed the name. This is what sorts
  `list(APPEND SRCS ...)`, and it applies to your grammar too: with
  `one_value_keywords: [FROM]` and `sortable_positional: true`,
  `my_copy(FROM base.cpp z.cpp a.cpp)` sorts `z.cpp a.cpp`. **If that tail is
  order-significant** — a destination after a source, say — do not set
  `sortable_positional` on it.

A run after a leading *flag* is not reached, and neither is anything a
multi-value keyword owns (that is what `sortable_keywords` is for). Builtins
reach a run after a flag as well, because their grammars name those flags; a
`sortable_positional` grammar cannot yet say the same, and it cannot say that
its first positional is a *target* rather than the list — so a wrapper around
`add_library` holds where `add_library` itself would sort, which is the safe
direction to be wrong in.

To skip one command, put `# cmake-fmt: no-sort` on the line before it; that
suppresses both reordering passes.

#### `source_grouping`

**`source_grouping = headers_first`:**
Groups header files (`.h`, `.hpp`) before source files (`.cpp`, `.c`) in the same
lists `sort_sources` applies to.

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

#### `collapse_empty_flags`

Controls whether no-argument flags (like `REQUIRED`, `CONFIG`) are kept inline or placed on their own line. Type-selector flags (like `STATIC`, `SHARED`) always stay inline with the target name regardless of this setting.

**`collapse_empty_flags = true` && `max_line_length=60` (default):**
```cmake
find_package(Boost REQUIRED CONFIG
	COMPONENTS
		system
		filesystem
)
```

**`collapse_empty_flags = false` && `max_line_length=60`:**
```cmake
find_package(Boost
	REQUIRED
	CONFIG
	COMPONENTS
		system
		filesystem
)
```

#### `inline_single_keyword`

When `true` and a keyword command has exactly one keyword section, the keyword stays on the same line as the command's positional arguments, and values are single-indented instead of double-indented. When multiple keyword sections exist (e.g., `PUBLIC` + `PRIVATE`), the standard layout is always used regardless of this setting.

**`inline_single_keyword = false` (default):**
```cmake
target_sources(mylib
	PUBLIC
		src/a.cpp
		src/b.cpp
)
```

**`inline_single_keyword = true`:**
```cmake
target_sources(mylib PUBLIC
	src/a.cpp
	src/b.cpp
)
```

Note: When multiple keyword sections exist (e.g., both `PUBLIC` and `PRIVATE`), the standard double-indented layout is always used regardless of this setting.

#### `control_flow_space_before_paren`

Inserts a space between the command name and `(` for control flow and block statements only.
Regular commands like `set()`, `message()`, and `add_library()` are never affected.

**`control_flow_space_before_paren = false` (default):**
```cmake
if(BUILD_TESTING)
    add_subdirectory(tests)
endif()
```

**`control_flow_space_before_paren = true`:**
```cmake
if (BUILD_TESTING)
    add_subdirectory(tests)
endif ()
```

#### `space_between_command_parens`

Inserts a space inside the parentheses of commands. In single-line commands, both opening and
closing parens get an inner space. In multiline commands, the opening `(` gets a trailing space
(when followed by content on the same line), but the closing `)` stays at its normal indentation.

**`space_between_command_parens = false` (default):**
```cmake
set(MY_VAR "value")
target_sources(mylib
    PUBLIC
        src/a.cpp
)
```

**`space_between_command_parens = true`:**
```cmake
set( MY_VAR "value" )
target_sources( mylib
    PUBLIC
        src/a.cpp
)
```

#### `indent_closing_paren`

Indents the closing `)` by one indent level in multiline commands. Has no effect on single-line
commands. Can be combined with `space_between_command_parens`.

**`indent_closing_paren = false` (default):**
```cmake
target_sources(mylib
    PUBLIC
        src/a.cpp
        src/b.cpp
)
```

**`indent_closing_paren = true`:**
```cmake
target_sources(mylib
    PUBLIC
        src/a.cpp
        src/b.cpp
    )
```

### Custom Command Grammars

The `command_grammars` setting is only available in config files (not via `--style`). It allows you to define custom command grammars for project-specific CMake functions.

**Example `.cmake-fmt`:**
```yaml
command_grammars:
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
| `-r`, `--recursive` | Recursively format files in directories |
| `--ignore-file <FILE>` | Path to additional ignore file (gitignore syntax) |
| `--line-ranges <RANGES>` | Format only specific line ranges (e.g., "1:5,10:15") |
| `--help-style` | Show all available style settings |
| `--help-grammar` | Show grammar file format and keyword types |
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

## License

MIT — see [LICENSE](LICENSE) for details.
