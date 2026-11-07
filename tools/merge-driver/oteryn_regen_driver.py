#!/usr/bin/env python3
"""Git merge driver for derived content files (`merge=oteryn-regen` in .gitattributes).

Git calls it as `oteryn_regen_driver.py %O %A %B %P` (base, ours, theirs, path). A clean three-way
merge is kept. On a conflict the file is a derived registry, so neither side is a decision: take
theirs (the incoming `origin/main` side) and exit 0. That content may be stale; run
`tools/merge-driver/regen.sh` after the merge, which regenerates it. Fail-closed by design: nothing
here proves freshness, so a merge that skips the regeneration is still stopped by the freshness
checks in CI. GitHub's server-side merge ignores custom drivers; this is a local-merge aid only.
"""

from __future__ import annotations

import shutil
import subprocess
import sys


def main(argv: list[str]) -> int:
    if len(argv) != 5:
        print("usage: oteryn_regen_driver.py %O %A %B %P", file=sys.stderr)
        return 2
    base, ours, theirs, path = argv[1:]
    merged = subprocess.run(["git", "merge-file", ours, base, theirs]).returncode
    if merged == 0:
        return 0
    if merged < 0 or merged > 127:
        return 2  # git merge-file itself failed: leave the conflict to a person
    shutil.copyfile(theirs, ours)
    print(
        f"oteryn-regen: {path} conflicted; took the incoming side. "
        "Run tools/merge-driver/regen.sh before committing.",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
