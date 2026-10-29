#!/usr/bin/env python3
"""Print the next free migration, capability, command, mechanic ordinal and
offer order, counting current main and the files of every open PR.

Usage: next-free-numbers.py [--ref origin/main] [--no-prs]
Needs git (run inside a clone) and, for open PRs, gh with REST access.
Numbers taken only by an open PR are listed so you can avoid them; re-run
right before pushing, since main and the PRs keep moving. Reading every open
PR's file list is deliberate here: it is the sanctioned exception to the
live-state read budget in docs/agents/CONTEXT_ROUTING.md, because a number
clash costs a CI cycle and a renumbering commit.
"""
import argparse
import json
import re
import subprocess
import sys

REPO = "Oteryn/Oteryn-Game"
KINDS = {
    "migration": re.compile(r"^apps/game-server/migrations/(\d{4})_[^/]+\.sql$"),
    "capability": re.compile(
        r"^docs/contracts/protocol-oteryn/v1/registry/capabilities/(\d{4})-[^/]+\.json$"),
    "command": re.compile(
        r"^docs/contracts/protocol-oteryn/v1/registry/command_types/(\d{4})-[^/]+\.json$"),
    "mechanic ordinal": re.compile(
        r"^crates/gameplay/dispatch/mechanics/(\d{4})-[^/]+\.mechanic$"),
}
MECH_DIR = "crates/gameplay/dispatch/mechanics/"
OFFER = re.compile(r"^offer=(\d+),", re.M)


def run(*cmd):
    return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout


def numbers_in(paths):
    found = {k: set() for k in KINDS}
    for p in paths:
        for kind, rx in KINDS.items():
            m = rx.match(p)
            if m:
                found[kind].add(int(m.group(1)))
    return found


def offers_on_ref(ref):
    out = set()
    for p in run("git", "ls-tree", "--name-only", ref, MECH_DIR).split():
        if p.endswith(".mechanic"):
            out.update(int(x) for x in OFFER.findall(run("git", "show", f"{ref}:{p}")))
    return out


def open_pr_files():
    prs = []
    for page in range(1, 11):
        batch = json.loads(run("gh", "api",
                               f"repos/{REPO}/pulls?state=open&per_page=100&page={page}",
                               "--jq", "[.[]|{n:.number,t:.title,sha:.head.sha}]"))
        prs += batch
        if len(batch) < 100:
            break
    result = []
    for pr in prs:
        # Explicit pages: GitHub's own "next" links use numeric repository
        # ids, which some proxies refuse. The files API stops at 3000 files.
        files = []
        for page in range(1, 31):
            batch = json.loads(run("gh", "api",
                                   f"repos/{REPO}/pulls/{pr['n']}/files?per_page=100&page={page}",
                                   "--jq", "[.[]|{f:.filename,s:.status,p:(.patch // \"\")}]"))
            files += batch
            if len(batch) < 100:
                break
        result.append((pr, [f for f in files if f["s"] != "removed"]))
    return result


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", default="origin/main")
    ap.add_argument("--no-prs", action="store_true")
    args = ap.parse_args()

    main_nums = numbers_in(run("git", "ls-tree", "-r", "--name-only", args.ref).split())
    main_nums["offer order"] = offers_on_ref(args.ref)
    pr_nums = {k: {} for k in main_nums}
    if not args.no_prs:
        for pr, files in open_pr_files():
            got = numbers_in(f["f"] for f in files)
            for f in files:
                if f["f"].startswith(MECH_DIR) and f["f"].endswith(".mechanic"):
                    text = f["p"]
                    if not text:  # GitHub omits the patch for large diffs
                        text = "\n".join("+" + line for line in run(
                            "gh", "api", "-H", "Accept: application/vnd.github.raw",
                            f"repos/{REPO}/contents/{f['f']}?ref={pr['sha']}").splitlines())
                    got.setdefault("offer order", set()).update(
                        int(x) for x in re.findall(r"^\+offer=(\d+),", text, re.M))
            for kind, nums in got.items():
                for n in nums - main_nums[kind]:
                    pr_nums[kind].setdefault(n, []).append(pr["n"])

    print(f"base {args.ref} = {run('git', 'rev-parse', '--short=10', args.ref).strip()}")
    for kind, nums in main_nums.items():
        taken = nums | set(pr_nums[kind])
        top = max(taken) if taken else 0
        held = ", ".join(f"{n} (#{'/#'.join(map(str, p))})" for n, p in sorted(pr_nums[kind].items()))
        print(f"{kind:17} main max {max(nums) if nums else '-':>5}   next free {top + 1:>5}"
              + (f"   held by open PRs: {held}" if held else ""))
    dups = [k for k, v in pr_nums.items() for n, p in v.items() if len(p) > 1]
    if dups:
        print("CLASH: the same number is claimed by more than one open PR (see above)")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
