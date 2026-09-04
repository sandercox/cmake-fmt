"""Shared helpers for the formatter's invariant scripts.

Not a CLI. Imported by gen-shapes.py, check-shapes.py and compare-builds.py.
"""

import re
import subprocess

# (opening line, arguments) -- one per render-arm class. A fix that lands in
# one arm has repeatedly not landed in its twin, so each class is represented.
COMMANDS = [
    ("target_sources(t PRIVATE", ["a.cpp", "b.cpp"]),                # MultiValue
    ("install(FILES", ["a.h", "b.h"]),                               # mode keyword
    ("list(APPEND V", ["a.cpp", "b.cpp"]),                           # SingleValue overflow
    ("add_library(l STATIC", ["a.cpp", "a.h"]),                      # Flag, and a grouping pair
    ("set_target_properties(t PROPERTIES", ["CXX_STANDARD", "17"]),  # PairValue
    ("find_package(Foo REQUIRED", ["COMPONENTS", "X"]),              # valueless Flag
    ("define_property(TEST PROPERTY", ["foo"]),                      # multi-mode
    ("source_group(g FILES", ["a.cpp", "b.cpp"]),                    # grammar entry added in 0.11
]

# B = blank line, C = comment. Every arrangement of up to two of each. The
# four that a position-keyed encoding cannot represent are BCB, CBC, BCCB
# and CBCB -- one placement is always lost.
ARRANGEMENTS = [
    "", "C", "B", "BC", "CB", "CC", "BCB", "CBC",
    "BCC", "CCB", "BCCB", "CBCB", "BB", "CBB",
]

# Both reordering passes default to none, so a run at default settings
# exercises neither of them.
STYLES = [
    "",
    "sort_sources=alphabetical",
    "source_grouping=headers_first",
    "source_grouping=sources_first",
    "sort_sources=alphabetical,source_grouping=headers_first",
    "sort_sources=alphabetical,source_grouping=sources_first",
    "inline_single_keyword=true",
    "inline_single_keyword=true,sort_sources=alphabetical",
    "collapse_empty_flags=false",
    "max_blank_lines=2",
]


def shapes():
    """Yield (name, text) for every generated shape.

    One command, with a chosen arrangement of comments and blank lines placed
    in one gap between its arguments. Real CMake almost never writes those
    gaps, which is why the corpus is a fixed point even on releases that lose
    the comments outright -- the shapes have to be generated.
    """
    n = 0
    for arrangement in ARRANGEMENTS:
        for head, args in COMMANDS:
            for pos in range(len(args) + 1):
                gap, seen = [], 0
                for ch in arrangement:
                    if ch == "B":
                        gap.append("")
                    else:
                        seen += 1
                        gap.append("\t\t# note%d" % seen)
                lines = [head]
                for i, arg in enumerate(args):
                    if i == pos:
                        lines += gap
                    lines.append("\t\t" + arg)
                if pos == len(args):
                    lines += gap
                lines.append(")")
                n += 1
                yield "s%05d.cmake" % n, "\n".join(lines) + "\n"


def tokens(text):
    """Split into (code tokens, sorted comment bodies).

    Parens are separators: a first argument is glued to its command name by
    '(', so a whitespace split cannot see it -- and that argument is exactly
    the one a mis-emitted comment swallows. Indentation and line breaks are
    deliberately invisible here, so that a layout change is not mistaken for
    a content change.
    """
    code, comments = [], []
    for line in text.splitlines():
        if "#" in line:
            head, comment = line.split("#", 1)
            comments.append(comment.strip())
        else:
            head = line
        code += [t for t in re.split(r"[\s()]+", head) if t]
    return code, sorted(comments)


def fmt(binary, style, path):
    """Run the formatter over one file. Returns the CompletedProcess."""
    cmd = [binary] + (["--style", style] if style else []) + [path]
    return subprocess.run(cmd, capture_output=True, text=True)
