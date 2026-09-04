# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Fixed
- A comment written *before* a command's first argument is no longer deleted when that argument is a keyword — `install(\n\t# ship the headers\n\tFILES a.h\n\tDESTINATION inc)` lost it, and so did every command whose first argument its grammar names. Over every (command, keyword) pair cmake-fmt knows, the previous release loses this comment on 227 of 375
- A comment inside a keyword section is no longer deleted, and a property's value never lands inside one. The shortcuts that put a single value inline with its keyword emitted nothing else, so `target_sources(t PRIVATE\n\t# impl\n\tb.cpp)`, `list(APPEND V\n\t# note\n\ta.cpp)`, a one-pair `PROPERTIES` run, and a keyword with no values at all — `find_package(Foo REQUIRED # note\n)`, `target_sources(t PRIVATE # note\n)`, `PROPERTIES # note\n`, `COMMAND # note\n` — all lost their comments.
- `sort_sources` and `source_grouping` now only reorder argument lists that a command grammar marks as unordered, instead of every list that happened to look like filenames. This stops both passes from rewriting `set(VAR val CACHE PATH "docs (etc/xdg)")` — a docstring that looks like a path ([#3](https://github.com/sandercox/cmake-fmt/issues/3)), `COMMAND` argv lists ([#6](https://github.com/sandercox/cmake-fmt/issues/6)), `PROPERTIES` key/value pairs, `install(DIRECTORY ... FILES_MATCHING PATTERN ...)` glob pairs, `file(RENAME src dst)`, `configure_file(in out)`, `target_link_libraries` link order, an `add_compile_options` list whose entries look like files (`z.opt a.opt` — an actual flag spelling such as `-Wall` was already safe), and a dotted target name such as `add_library(zz.lib ...)`
- A keyword-less run is reordered only when every value looks like a source file, so a library list held in a variable — `list(APPEND LIBS libz.a liba.a)` — keeps its order. A keyword vouches for its own values, so `install(FILES README LICENSE ...)` sorts — extension-less names included, which the previous release left alone
- A list held in a variable nobody can read is left alone: `set(${VAR} z.cpp a.cpp)` and `list(APPEND ${VAR} z.cpp a.cpp)` both hold, where the previous release sorted them. A name that cannot be read is a name that cannot be vetted
- Lists whose variable names a search path, flag list or argument list are left alone, matched case-insensitively so lowercase project-local names like `warning_flags` are covered too
- A quoted variable reference (`"${GENERATED}"`) now holds its position like the bare spelling, instead of sorting to the front of the list — `target_sources(t PRIVATE z.cpp a.cpp "${GENERATED}")` used to hoist it ahead of both sources
- Blank lines around a comment are written back as they were read. A blank line before a comment was written twice; a blank recorded after a section's last argument was dropped by the grouping pass; and a blank between two comment groups named a comment by index — which sorting could move, and which an intervening argument could leave naming a later comment entirely. The last three cost the output its first-pass fixed point, so `cmake-fmt -i` followed by `cmake-fmt --check` disagreed; the first only added a line nobody wrote
- `# cmake-fmt: no-sort` now also suppresses `source_grouping`, which reorders too
- `source_grouping` now honours the same barriers as `sort_sources`: it no longer hoists a header across a `${...}` or past the name the list is held in, so `set(SRCS b.cpp ${GENERATED} b.h)` and `add_library(foo.cpp bar.cpp foo.h)` hold. It reorders independently of `sort_sources`, so this applied even with sorting off
- `source_grouping` now also reaches a source list that follows a flag, so `add_library(lib STATIC a.cpp a.h)` groups its pair like `add_library(lib a.cpp a.h)` already did. `sort_sources` always sorted that run, so the two passes disagreed about a list the allowlist owns

### Added
- `sortable_keywords` and `sortable_positional` in `command_grammars` and grammar files, to mark a list in your own command as unordered. Naming any keyword makes that list the whole list, and writing `sortable_keywords: []` says nothing in that command is sortable — so there is always a way to say "not this one". Omitting the key in a `command_grammars` entry falls back to keywords named `SOURCES`, `SRCS` or `FILES`, which is what an auto-detected grammar would have given you
- `target_sources` now models the `FILE_SET` form's `TYPE`, `BASE_DIRS` and `FILES` keywords, and `source_group` has a grammar entry — so `FILE_SET ... FILES` and `source_group(... FILES ...)` are reordered where previously they were not

### Changed
- `if`/`elseif`/`while` conditions laid out on more than one line now break before `AND`/`OR` and keep each clause on its own line, instead of putting every word on a separate line. This covers both a condition too long to fit and one the author wrapped by hand. A clause too long for one line is filled across continuation lines. A blank line written inside a condition is not preserved once this layout applies; a condition carrying a comment keeps the previous layout and its blank lines ([#2](https://github.com/sandercox/cmake-fmt/issues/2))
- A trailing comment now counts toward its condition's line, so `if(A AND B) # a long comment` wraps at the operator rather than one argument per line — the comment is part of the line the condition has to fit on. The comment is measured as it will be written: one space before the `#`, the whitespace after it set by `comment_style`, nothing after the last non-space character, and only the first comment kept on the line
- The space `space_between_command_parens` puts inside the parens now follows the arguments a block closer actually emits, rather than the ones its author wrote — which `closing_style` may already have replaced. So `endforeach()` under `closing_style = remove` no longer comes out as `endforeach( )` with a space around nothing, and `endfunction( my_fn ARG )` under `force` now gets the space its arguments earn. The two disagreeing meant a second pass changed the output again. `elseif` is excluded from all of this because it carries a condition of its own, which `closing_style` never touches

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
