#!/usr/bin/env python3
"""Compare the formatter's output between two git refs.

Answers the question a test suite cannot: this commit changes the output --
does it change anything but layout? A rendering fix legitimately moves blank
lines around, so differences are classified rather than counted:

  content  a code token or a comment appeared, vanished or changed order.
           Always worth investigating.
  layout   the same tokens and comments, written differently -- indentation,
           line breaks, blank-line placement.

Usage: compare-builds.py <ref-a> <ref-b> [extra files...]
       scripts/compare-builds.py main HEAD
"""

import os
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _fmtlib import STYLES, fmt, shapes, tokens  # noqa: E402


def sh(*cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def build(root, ref, dest):
    """Check out ref into a worktree and build it. Returns the binary path."""
    print("building %s ..." % ref, flush=True)
    r = sh("git", "-C", root, "worktree", "add", "--detach", dest, ref)
    if r.returncode != 0:
        sys.exit("could not check out %s:\n%s" % (ref, r.stderr))
    r = sh("cargo", "build", "--release", cwd=dest)
    if r.returncode != 0:
        sys.exit("build failed for %s:\n%s" % (ref, r.stderr[-2000:]))
    return os.path.join(dest, "target", "release", "cmake-fmt")


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    ref_a, ref_b = sys.argv[1], sys.argv[2]
    extra = sys.argv[3:]

    root = sh("git", "rev-parse", "--show-toplevel").stdout.strip()
    work = tempfile.mkdtemp(prefix="cmake-fmt-compare-")
    worktrees = [os.path.join(work, "a"), os.path.join(work, "b")]
    try:
        bin_a = build(root, ref_a, worktrees[0])
        bin_b = build(root, ref_b, worktrees[1])

        files = []
        for name, text in shapes():
            path = os.path.join(work, name)
            with open(path, "w") as fh:
                fh.write(text)
            files.append(path)
        for dirpath, _, names in os.walk(os.path.join(root, "tests", "corpus")):
            for n in names:
                if n.endswith(".cmake") or n == "CMakeLists.txt":
                    files.append(os.path.join(dirpath, n))
        files += extra

        print("\n%-56s %8s %8s" % ("style", "content", "layout"))
        total_content = 0
        for style in STYLES:
            content = layout = shown = 0
            for path in files:
                a = fmt(bin_a, style, path).stdout
                b = fmt(bin_b, style, path).stdout
                if a == b:
                    continue
                if tokens(a) == tokens(b):
                    layout += 1
                    continue
                content += 1
                if shown < 2:
                    shown += 1
                    print("    CONTENT [%s] %s"
                          % (style or "<default>",
                             os.path.relpath(path, root) if path.startswith(root)
                             else os.path.basename(path)))
                    for line in list(_diff(a, b))[:8]:
                        print("      " + line)
            total_content += content
            print("%-56s %8d %8d" % (style or "<default>", content, layout))

        print("\ncontent = a token or comment moved or vanished "
              "(%d total -- investigate every one)" % total_content)
        print("layout  = same tokens and comments, written differently")
        return 1 if total_content else 0
    finally:
        for wt in worktrees:
            sh("git", "-C", root, "worktree", "remove", "--force", wt)
        shutil.rmtree(work, ignore_errors=True)


def _diff(a, b):
    import difflib
    return difflib.unified_diff(a.splitlines(), b.splitlines(),
                                "a", "b", lineterm="", n=1)


if __name__ == "__main__":
    sys.exit(main())
