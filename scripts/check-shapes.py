#!/usr/bin/env python3
"""Formatter invariant checks over generated keyword-section shapes.

Two properties, across a matrix of style settings:

  content     Formatting never loses a code token and never loses a comment.
              This is the one that matters: the losses this guards against
              were silent, at exit 0, and at a stable fixed point that
              --check then called formatted.

  fixed point Formatting an already-formatted file changes nothing. A tool
              whose output is not its own input makes --check reject what -i
              has just written.

A content loss always fails. A non-fixed-point is compared against
scripts/known-drifts.txt, so a shape that is already known to be unstable
does not fail the build but a new one does. The baseline is keyed on the
shape's text rather than its filename, so editing the generator cannot
silently empty it.

Usage: check-shapes.py <cmake-fmt binary> [--shapes DIR] [--baseline FILE]
                       [--update-baseline]
"""

import argparse
import os
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _fmtlib import STYLES, fmt, shapes, tokens  # noqa: E402

DEFAULT_BASELINE = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                "known-drifts.txt")


def load_baseline(path):
    if not os.path.exists(path):
        return set()
    with open(path) as fh:
        return {line.strip() for line in fh
                if line.strip() and not line.startswith("#")}


def key(style, text):
    """Baseline key: the style, and the shape's text with newlines escaped."""
    return "%s\t%s" % (style or "<default>", text.replace("\n", "\\n"))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("binary")
    ap.add_argument("--shapes", help="directory of shapes (default: generated)")
    ap.add_argument("--baseline", default=DEFAULT_BASELINE)
    ap.add_argument("--update-baseline", action="store_true",
                    help="rewrite the baseline from this run")
    args = ap.parse_args()

    work = tempfile.mkdtemp(prefix="cmake-fmt-shapes-")
    if args.shapes:
        files = sorted(os.path.join(args.shapes, f)
                       for f in os.listdir(args.shapes) if f.endswith(".cmake"))
    else:
        files = []
        for name, text in shapes():
            path = os.path.join(work, name)
            with open(path, "w") as fh:
                fh.write(text)
            files.append(path)
    if not files:
        sys.exit("no shapes to check")

    known = load_baseline(args.baseline)
    seen_drifts, new_drifts, losses, crashes = set(), [], [], []

    for style in STYLES:
        lost_code = lost_comments = drifted = 0
        for path in files:
            source = open(path).read()
            first = fmt(args.binary, style, path)
            if first.returncode != 0 or "panicked" in first.stderr:
                head = (first.stderr.strip().splitlines() or [""])[0]
                crashes.append((style, path, head))
                continue

            src_code, src_comments = tokens(source)
            out_code, out_comments = tokens(first.stdout)
            if sorted(src_code) != sorted(out_code):
                lost_code += 1
                losses.append((style, path, "code",
                               sorted(set(src_code) - set(out_code))[:4]))
            elif src_comments != out_comments:
                lost_comments += 1
                losses.append((style, path, "comment",
                               [c for c in src_comments if c not in out_comments][:4]))

            once = os.path.join(work, "_once.cmake")
            with open(once, "w") as fh:
                fh.write(first.stdout)
            if fmt(args.binary, style, once).stdout != first.stdout:
                drifted += 1
                k = key(style, source)
                seen_drifts.add(k)
                if k not in known:
                    new_drifts.append((style, path, source))

        print("  %-56s code:%-3d comments:%-3d drift:%d"
              % (style or "<default>", lost_code, lost_comments, drifted))

    if args.update_baseline:
        with open(args.baseline, "w") as fh:
            fh.write("# Shapes that are known not to be a fixed point: "
                     "formatting the output again\n"
                     "# changes it. Each line is a style setting and the "
                     "shape's text, newlines\n"
                     "# escaped. check-shapes.py fails on a drift that is "
                     "not listed here.\n"
                     "#\n"
                     "# Regenerate: scripts/check-shapes.py <binary> "
                     "--update-baseline\n")
            for k in sorted(seen_drifts):
                fh.write(k + "\n")
        print("\nbaseline updated: %d known drift(s)" % len(seen_drifts))
        return 0

    print()
    for style, path, kind, what in losses[:10]:
        print("  LOSS [%s] %s lost %s %s"
              % (style or "<default>", os.path.basename(path), kind, what))
    if len(losses) > 10:
        print("  ... and %d more losses" % (len(losses) - 10))
    for style, path, err in crashes[:10]:
        print("  CRASH [%s] %s  %s"
              % (style or "<default>", os.path.basename(path), err))
    for style, path, source in new_drifts[:10]:
        print("  NEW DRIFT [%s] %s"
              % (style or "<default>", os.path.basename(path)))
        print("      %s" % source.replace("\n", "\\n"))

    stale = known - seen_drifts
    if stale:
        print("  %d baseline entr(y/ies) no longer drift -- rerun with "
              "--update-baseline to tighten" % len(stale))

    failed = bool(losses or crashes or new_drifts)
    print("\n%s: %d content losses, %d crashes, %d new drifts (%d known)"
          % ("FAIL" if failed else "PASS", len(losses), len(crashes),
             len(new_drifts), len(seen_drifts)))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
