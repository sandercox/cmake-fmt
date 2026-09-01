# scripts

## Formatter invariant checks

`tests/corpus` is real CMake, and real CMake almost never writes comments or
blank lines into the gaps *between* a keyword's arguments. The whole corpus is
a fixed point under every style setting even on releases that delete those
comments outright, so the corpus cannot see that class of defect. These
scripts generate the shapes that can.

| script | what it does |
| --- | --- |
| `gen-shapes.py <dir>` | writes the generated shapes out, to look at |
| `check-shapes.py <binary>` | the gate: no token or comment may vanish, and the output must be its own input |
| `compare-builds.py <ref-a> <ref-b>` | builds both refs and classifies every output difference as content or layout |
| `_fmtlib.py` | shared shape list, tokenizer and runner — not a CLI |

`check-shapes.py` runs in CI on every pull request (`.github/workflows/ci.yml`).

### Two things to know before you test by hand

**`sort_sources` and `source_grouping` both default to `none`.** A run at
default settings exercises neither pass, so nothing about reordering is
reachable without `--style sort_sources=alphabetical` and/or
`--style source_grouping=headers_first`. A corpus guard that ran at default
settings and therefore tested neither is exactly how the reordering defects in
0.10.2 shipped.

**`debug_assert!` compiles out of `--release`.** Run `cargo test` as well as
`cargo test --release`, or an assertion that only fires in a debug build never
runs.

### Usage

```sh
cargo build --release
python3 scripts/check-shapes.py target/release/cmake-fmt

# what did this branch change, and was any of it content?
python3 scripts/compare-builds.py main HEAD
```

`check-shapes.py` fails on any content loss. It does not fail on a
non-fixed-point that is already listed in `known-drifts.txt`, so a known-open
shape does not block the build while a new one does. After fixing one:

```sh
python3 scripts/check-shapes.py target/release/cmake-fmt --update-baseline
```

The baseline is keyed on each shape's text rather than its filename, so
editing the generator cannot silently empty it.

## Release

`bump-version.sh` — sync the version across `Cargo.toml` and the VS Code
extension. See `RELEASING.md`.
