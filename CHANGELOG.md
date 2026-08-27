# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Fixed
- A comment inside a keyword section is no longer deleted, and a property's value never lands inside one. The shortcuts that put a single value inline with its keyword emitted nothing else, so `target_sources(t PRIVATE\n\t# impl\n\tb.cpp)`, `list(APPEND V\n\t# note\n\ta.cpp)`, a one-pair `PROPERTIES` run, and a keyword with no values at all — `find_package(Foo REQUIRED # note ...)`, `target_sources(t PRIVATE # note)`, `PROPERTIES # note`, `COMMAND # note` — all lost their comments. A `PROPERTIES` key's trailing comment is now emitted after the value rather than before it, where it commented the value out
- `sort_sources` and `source_grouping` now only reorder argument lists that a command grammar marks as unordered, instead of every list that happened to look like filenames. This stops both passes from rewriting `set(VAR val CACHE <type> "docstring")` ([#3](https://github.com/sandercox/cmake-fmt/issues/3)), `COMMAND` argv lists ([#6](https://github.com/sandercox/cmake-fmt/issues/6)), `PROPERTIES` key/value pairs, `install(DIRECTORY ... FILES_MATCHING PATTERN ...)` glob pairs, `file(RENAME src dst)`, `configure_file(in out)`, `target_link_libraries` link order, `add_compile_options` flag lists, and a dotted target name such as `add_library(zz.lib ...)`
- Reordering now applies to: `set(VAR ...)` values, `add_library` / `add_executable` sources, `target_sources` scope and `FILE_SET FILES` lists, `list(APPEND|PREPEND|REMOVE_ITEM)` elements, `install(FILES|PROGRAMS)`, `source_group(... FILES ...)`, and keywords named `SOURCES`/`SRCS`/`FILES` on your own commands
- A keyword-less run is reordered only when every value looks like a source file, so flag lists (`set(warning_flags -Wno-unused -Wall)`), argument lists (`set(RUN_ARGS --output z.txt --input a.txt)`) and library lists (`list(APPEND LIBS libz.a liba.a)`) keep their order even when held in a variable. A keyword vouches for its own values, so `install(FILES README LICENSE ...)` still sorts
- Lists whose variable names a search path, flag list or argument list are left alone, matched case-insensitively so lowercase project-local names like `warning_flags` are covered too
- A quoted variable reference (`"${GENERATED}"`) now holds its position like the bare spelling, instead of sorting to the front of the list
- `# cmake-fmt: no-sort` now also suppresses `source_grouping`, which reorders too
- `source_grouping` now honours the same barriers as `sort_sources`: it no longer hoists a header across a `${...}` or past the name the list is held in, so `set(SRCS b.cpp ${GENERATED} b.h)` and `add_library(foo.cpp bar.cpp foo.h)` hold. It reorders independently of `sort_sources`, so this applied even with sorting off
- A target name is no longer read as the name of the list it holds, so ordinary targets whose names end in `_LIBS`, `_DIRS` or `_OPTIONS` — or contain `FLAGS` — still sort their sources. That blocklist is new in this release and the exemption keeps it from over-reaching; nothing regressed on the previous one. Conversely `set(${VAR} ...)` now holds, as does `list(APPEND ${VAR} ...)`: a list variable nobody can read is one nobody can vet
- Declaring a `command_grammars` entry no longer silently switches off sorting for keywords named `SOURCES`, `SRCS` or `FILES`. A config entry replaces the grammar auto-detected from `cmake_parse_arguments`, so a user who added one to fix wrapping lost the sorting they already had. Naming any keyword in `sortable_keywords` means that list is the whole list, and writing it as an empty list means nothing in that command is sortable — so there is always a way to say "not this one"
- `source_grouping` now also reaches a source list that follows a flag, so `add_library(lib STATIC a.cpp a.h)` groups its pair like `add_library(lib a.cpp a.h)` already did. `sort_sources` always sorted that run, so the two passes disagreed about a list the allowlist owns
- Parenthesized groups in conditions are no longer dropped — `if((TRUE))` became `if()` and `if((TRUE) AND (TRUE))` became `if(AND)`. A `( ... )` group is now kept as a single argument, in conditions and in keyword-aware commands alike. A `( ... )` group also holds its position when `sort_sources` or `source_grouping` is on, splitting a list into runs that sort independently, since a group is several real arguments and moving one across it changes the list. Known limitations: a group is an unbreakable atom, so a hand-wrapped one is collapsed onto a single line and can exceed `max_line_length`; a group containing a comment is emitted verbatim, so it keeps the indentation it had in the source and `comment_style` is not applied inside it; and an unterminated group still runs to the end of the file rather than to the end of its line, so the lines after it are folded into that command and a closing paren is invented for it. Most such inputs settle on the second pass; the ones whose text ends inside an unterminated bracket comment, quote or bracket argument grow by one group per run instead. An unterminated `${` grows the same way with no group involved (`f(${` does it on its own), so that class is older and wider than this change — invalid input either way, and a separate change adds a check that refuses to write any of it ([#5](https://github.com/sandercox/cmake-fmt/issues/5))

### Added
- The formatter now checks its own output before writing: if formatting would change what a file *says* rather than how it looks, the file is left exactly as written, a warning names what changed, and the run exits non-zero — in every mode, including plain output to stdout and `--line-ranges`. Re-indenting, re-wrapping, re-casing, `comment_style`, `closing_style` and the reordering passes are all recognised as permitted, including when a `# cmake-fmt:` directive in the file turns one of them on; inventing or dropping an argument, a comment or a parenthesis is not. Each exemption is cut to the setting that needs it: a block closer may say either what its author wrote or what `closing_style` would emit for it, `elseif` is never exempt because it carries a condition of its own, and a list is compared as a multiset only over the runs a grammar marks as unordered. Note that `--line-ranges` splices lines by index, so a range whose formatting changes the line count is now refused rather than written — the range is left as the author wrote it and the run exits non-zero. Ranges whose formatting keeps the line count are spliced and written as before, which is most of them
- `sortable_keywords` and `sortable_positional` in `command_grammars` and grammar files, to mark a list in your own command as unordered
- `target_sources` now models the `FILE_SET` form's `TYPE`, `BASE_DIRS` and `FILES` keywords, and `source_group` has a grammar entry

## [0.10.2]
- Fix when `.cmake-fmt` is a root file but there is no CMakeLists.txt use highest ancestor directory as root for function detections

## [0.10.1]
- Regression not picking up proper directory structures for function detections

## [0.10.0]

- Release all 0.9 and 0.8 beta features as stable release.

## [0.9.0-beta.4]
## [0.9.0-beta.3]
- Adopt odd/even minor version convention: odd minor versions (e.g. 0.9.x) are pre-releases, even minor versions (e.g. 0.10.x) are stable releases. This is required because the VS Code marketplace does not allow a version number previously used as pre-release to be republished as a stable release.
- Code signing for release binaries (Apple Developer ID for macOS, Azure Trusted Signing for Windows) and VSIX packages

## [0.8.0-beta.2]
- `root: true` config option to prevent inheriting parent directory `.cmake-fmt` files
- Fix config file path traversal resolving from absolute path

## [0.8.0-beta.1]
- Update README with `collapse_empty_flags` example
- `final_newline` now is `force`, `remove` or `preserve` (default)
- `inline_single_keyword` to keep simple source lists without excessive indentation
- `control_flow_space_before_paren` if true `if ()` or `if()`
- Change `leave` to `preserve` wherever it was used in styles (`leave` will still be accepted for backward compatibility)
- `space_between_command_parens` to control `set(SOURCES a.cpp)` vs `set( SOURCES a.cpp )`
- `indent_closing_paren` to control if closing parens should be indented or not when multiline
- Directory walking with recursive flag (`-r`) and `.cmake-fmt-ignore` file support
- Docker images published to Docker Hub and GitHub Container Registry on release

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
