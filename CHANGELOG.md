# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- `space_between_command_parens` no longer leaves a space at the end of a line. The pad goes *inside* the parens, but it was written next to the `(` whether or not an argument followed on that line, and the whole-buffer trailing-whitespace strip — which this release removes, because it also reached inside values — used to take it back off. The pad is now decided where the line break is: by the builder when only the renderer knows, and left out when the arguments start on the next line
- A backslash in an unquoted argument escapes what follows it, so `message(-DVERSION=\"${V}\")` is one argument. The lexer broke on the escaped `\"` instead, opening a quoted argument that never closed and running the parse to end of file — which silently deleted commands from `libgit2`, `ESP-IDF` and several vcpkg ports. `\#`, `\(`, `\)` and `\ ` were wrong the same way. A newline still ends the argument: CMake calls a backslash before one a bad character, not a line continuation
- Trailing whitespace is no longer removed from *inside* a value. Every line of the finished output used to be stripped, and a pass over finished text cannot tell the emitter's own padding from a token's payload — so `set(A "q   \nz")` and `set(A [[q   \nz]])` came back with the spaces deleted from the string itself. `cmake -P` reads a different value; a bracket argument is worse still, since CMake defines `[[...]]` as wholly literal and nothing can spell the bytes back. Comments are unaffected: they carry no value, so their trailing whitespace is still removed — now where each comment is emitted rather than by a pass over the whole buffer, which also fixes comments that the strip used to reach and the per-emitter trims had missed (a comment *block*, a leading bracket comment, a comment among a custom command's arguments, and a bracket comment in a keyword section). A `# cmake-fmt: off` region keeps the whitespace it was written with, where the old strip removed it
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
- `.cmake-fmt-ignore` (and `--ignore-file`) now apply to stdin input with `--assume-filename`, so editor format-on-save skips ignored files instead of formatting them. Matching follows git's rule that an excluded directory is final, so `build/` is not undone by a later `!build/keep.cmake`. Note the `.gitignore` family still does not apply on the stdin path: if your `build/` is excluded only by `.gitignore`, the directory walk skips it but format-on-save does not — put it in `.cmake-fmt-ignore` to have both ([#4](https://github.com/sandercox/cmake-fmt/issues/4))
- A directory excluded by an ignore file is skipped however you reach it: named on the command line, walked from above, walked from inside, reached through a symlink, or piped through stdin. Previously only a walk from above skipped it
- `--ignore-file` pointing at something that cannot be read is now an error instead of being silently ignored. The walk dropped the error, so a typo there formatted every file you meant to exclude — exiting 0 when nothing else needed formatting, and otherwise exiting 1 for entirely unrelated reasons. `/dev/null` and a `<(...)` pattern list are still accepted
- A directory named as the walk root is resolved before it is walked, so `cmake-fmt -r sub`, `-r sub/../sub` and `-r link-to-sub` reach the same verdict as each other and as the stdin path, while the files it reports keep the spelling you gave (a complaint about an ignore file discovered *above* a root names it absolutely, since you never typed it). An `--ignore-file` pattern anchored to its root previously excluded one spelling and not another
- `--ignore-file` is read once even when it names a one-shot stream, so `<(...)` and `/dev/stdin` — a pipe or a terminal — work for a directory walk. Such a source is copied before anything reads it, and the size check then runs on the copy: a stream past the 1 MiB bound is refused rather than truncated, so a pattern can no longer be cut in half. A complaint about the file names it as you typed it, and is printed once however many roots read it
- An excluded file piped through stdin is copied back byte for byte, so a buffer that is not valid UTF-8 is no longer rejected on a file the ignore rules said to leave alone
- Parenthesized groups in conditions are no longer dropped — `if((TRUE))` became `if()` and `if((TRUE) AND (TRUE))` became `if(AND)`. A `( ... )` group is now kept as a single argument, in conditions and in keyword-aware commands alike. A `( ... )` group also holds its position when `sort_sources` or `source_grouping` is on, splitting a list into runs that sort independently, since a group is several real arguments and moving one across it changes the list. Known limitations: a group is an unbreakable atom, so a hand-wrapped one is collapsed onto a single line and can exceed `max_line_length`; a group containing a comment is emitted verbatim, so it keeps the indentation it had in the source and `comment_style` is not applied inside it; and an unterminated group still runs to the end of the file rather than to the end of its line, so the lines after it are folded into that command and a closing paren is invented for it. Most such inputs settle on the second pass; the ones whose text ends inside an unterminated bracket comment, quote or bracket argument grow by one group per run instead. An unterminated `${` grows the same way with no group involved (`f(${` does it on its own), so that class is older and wider than this change — invalid input either way, and a separate change adds a check that refuses to write any of it ([#5](https://github.com/sandercox/cmake-fmt/issues/5))

### Changed
- A path named on the command line that is not there, or is not a regular file, now fails the run in every mode; so does a glob pattern that matches nothing. Both used to exit 0, so `cmake-fmt --check path/that/moved.cmake` and `cmake-fmt --check 'src/**/*.cmake'` reported success after a rename — which is how an unformatted file reaches a release. A *directory* holding no CMake files still succeeds: there was nothing to do and the caller said so. `--check` also counts only the files that would actually be reformatted, rather than every file it looked at
- `SuppressionWarning` is renamed `FormatWarning`, since it no longer carries only suppression warnings. The old name remains as a deprecated alias, and the enum is now `#[non_exhaustive]` so the next variant does not break an exhaustive `match`
- `if`/`elseif`/`while` conditions laid out on more than one line now break before `AND`/`OR` and keep each clause on its own line, instead of putting every word on a separate line. This covers both a condition too long to fit and one the author wrapped by hand. A clause too long for one line is filled across continuation lines. A blank line written inside a condition is not preserved once this layout applies; a condition carrying a comment keeps the previous layout and its blank lines ([#2](https://github.com/sandercox/cmake-fmt/issues/2))
- A trailing comment now counts toward its condition's line, so `if(A AND B) # a long comment` wraps at the operator rather than one argument per line — the comment is part of the line the condition has to fit on. The comment is measured as it will be written: one space before the `#`, the whitespace after it set by `comment_style`, nothing after the last non-space character, and only the first comment kept on the line
- The space `space_between_command_parens` puts inside the parens now follows the arguments a block closer actually emits, rather than the ones its author wrote — which `closing_style` may already have replaced. So `endforeach()` under `closing_style = remove` no longer comes out as `endforeach( )` with a space around nothing, and `endfunction( my_fn ARG )` under `force` now gets the space its arguments earn. The two disagreeing meant a second pass changed the output again. `elseif` is excluded from all of this because it carries a condition of its own, which `closing_style` never touches
- The formatter now checks its own output before writing: if formatting would change what a file *says* rather than how it looks, the file is left exactly as written, a warning names what changed, and the run exits non-zero — in every mode, including plain output to stdout and `--line-ranges`. Re-indenting, re-wrapping, re-casing, `comment_style`, `closing_style` and the reordering passes are all recognised as permitted, including when a `# cmake-fmt:` directive in the file turns one of them on; inventing or dropping an argument, a comment or a parenthesis is not. Each exemption is cut to the setting that needs it: a block closer may say either what its author wrote or what `closing_style` would emit for it, `elseif` is never exempt because it carries a condition of its own, and a list is compared as a multiset only over the runs a grammar marks as unordered. Note that `--line-ranges` splices lines by index, so the spliced buffer is checked in its own right: if it says anything different from the input, the **whole file** is left byte-identical and the run exits non-zero — one unsound range costs every sound range in the same invocation. A range is refused when formatting has changed the line count anywhere above it, so that the line at that index no longer holds the same content; a later range can be written again if an expansion and a collapse cancel out, so this is not simply "everything after the first refusal". How much still gets written depends entirely on the file: on the small fixtures in this repo most single-line ranges are still written, on its two files over 500 lines most are refused. Expect the second on a large real CMakeLists

### Added
- `sortable_keywords` and `sortable_positional` in `command_grammars` and grammar files, to mark a list in your own command as unordered. Naming any keyword makes that list the whole list, and writing `sortable_keywords: []` says nothing in that command is sortable — so there is always a way to say "not this one". Omitting the key in a `command_grammars` entry falls back to keywords named `SOURCES`, `SRCS` or `FILES`, which is what an auto-detected grammar would have given you
- `target_sources` now models the `FILE_SET` form's `TYPE`, `BASE_DIRS` and `FILES` keywords, and `source_group` has a grammar entry — so `FILE_SET ... FILES` and `source_group(... FILES ...)` are reordered where previously they were not

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
