#!/usr/bin/env python3
"""Write the generated keyword-section shapes to a directory, to look at or
to feed to another tool.

Usage: gen-shapes.py <outdir>
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _fmtlib import shapes  # noqa: E402


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    for stale in os.listdir(out):
        if stale.endswith(".cmake"):
            os.remove(os.path.join(out, stale))
    n = 0
    for name, text in shapes():
        with open(os.path.join(out, name), "w") as fh:
            fh.write(text)
        n += 1
    print("generated %d shapes in %s" % (n, out))


if __name__ == "__main__":
    main()
