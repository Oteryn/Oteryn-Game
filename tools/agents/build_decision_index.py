#!/usr/bin/env python3
"""Build docs/agents/DECISION_INDEX.md: D-numbered decisions -> PR -> documents.

The index is derived from two sources on protected main:
- squash-merge subjects ("... (D84-D87) (#1141)");
- the allocation line in the header (before the first "## ") of a decision
  document the merge adds ("- Allocation: D286 (...)" or
  "- control-plane allocation D300 (#1622)").
Two different owner-sequence D numbers for one document or one merge fail the
build instead of picking one. Rows already in the index are kept, so a shallow
clone only adds rows; it never drops history it cannot see.

Usage: python tools/agents/build_decision_index.py [--ref origin/main] [--gaps]
"""
from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
INDEX = ROOT / "docs/agents/DECISION_INDEX.md"

PR_RE = re.compile(r"\(#(\d+)\)\s*$")
# D84, D84-D87, D84–D87, D84-87; not "Combat D1", not slice subjects like "D3-4:".
D_RE = re.compile(r"(?<![A-Za-z0-9-])(?:([A-Z]{2,}[A-Z0-9]*)-)?D(\d{1,3})(?:\s*[–-]\s*D?(\d{1,3}))?(?![\d:])")
SLICE_RE = re.compile(r"^D\d+-\d+:")
SKIP_RE = re.compile(r"\b(?:Combat|combat) D\d|archive the ", re.IGNORECASE)
DOC_PREFIXES = ("docs/architecture/", "docs/agents/programs/", "docs/product/", "docs/content/", "docs/contracts/")
ALLOC_RE = re.compile(r"^\s*-\s+(?:Allocation:\s*|control-plane allocation\s+)D(\d{1,3})\b", re.IGNORECASE)
DECISION_DOC_RE = re.compile(r"^docs/architecture/reviews/.*DECISION.*\.md$")
ROW_RE = re.compile(r"^\| (?P<ids>[^|]+) \| #(?P<pr>\d+) \| (?P<date>[^|]+) \| (?P<subject>.*) \| (?P<docs>[^|]*) \|$")


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, check=True, capture_output=True, text=True).stdout


def decision_ids(subject: str) -> list[str]:
    ids: list[str] = []
    for prefix, first, last in D_RE.findall(subject):
        start = int(first)
        end = int(last) if last else start
        if end < start or end - start > 20:
            end = start
        label = f"{prefix}-" if prefix else ""
        ids.extend(f"{label}D{n}" for n in range(start, end + 1))
    return ids


class DecisionConflict(Exception):
    pass


def header_allocation(text: str, where: str) -> str | None:
    """The owner-sequence D number in a document header's allocation line, if any."""
    found: set[str] = set()
    for line in text.splitlines():
        if line.startswith("## "):
            break
        m = ALLOC_RE.match(line)
        if m:
            found.add(f"D{int(m.group(1))}")
    if len(found) > 1:
        raise DecisionConflict(f"{where}: header allocates {', '.join(sorted(found, key=lambda d: int(d[1:])))}")
    return found.pop() if found else None


def merge_ids(subject_ids: list[str], header_ids: dict[str, str], where: str) -> list[str]:
    """Subject ids plus header ids; fail when they name different owner-sequence numbers."""
    owner = {i for i in subject_ids if i.startswith("D")}
    headers = set(header_ids.values())
    if owner and headers - owner:
        raise DecisionConflict(f"{where}: subject {', '.join(sorted(owner))} vs header {', '.join(sorted(headers))}")
    if not subject_ids and len(headers) > 1:
        detail = "; ".join(f"{doc} {d}" for doc, d in sorted(header_ids.items()))
        raise DecisionConflict(f"{where}: documents allocate different numbers ({detail})")
    ids = list(subject_ids)
    ids.extend(d for d in sorted(headers, key=lambda d: int(d[1:])) if d not in ids)
    return ids


def scan(ref: str, gaps: list[str] | None = None) -> dict[int, dict]:
    rows: dict[int, dict] = {}
    log = git("log", ref, "--format=%H%x1f%ad%x1f%s", "--date=short")
    for line in log.splitlines():
        sha, date, subject = line.split("\x1f", 2)
        pr = PR_RE.search(subject)
        if not pr or SLICE_RE.match(subject) or SKIP_RE.search(subject):
            continue
        status = git("show", "--name-status", "--format=", sha).splitlines()
        added = [parts[-1] for parts in (l.split("\t") for l in status) if parts[0] == "A" and DECISION_DOC_RE.match(parts[-1])]
        header_ids: dict[str, str] = {}
        for doc in added:
            alloc = header_allocation(git("show", f"{sha}:{doc}"), doc)
            if alloc:
                header_ids[doc] = alloc
        ids = merge_ids(decision_ids(PR_RE.sub("", subject)), header_ids, f"#{pr.group(1)}")
        if not ids:
            if gaps is not None:
                gaps.extend(f"#{pr.group(1)} {doc}" for doc in added)
            continue
        files = git("show", "--name-only", "--format=", sha).split()
        docs = sorted(f for f in files if f.startswith(DOC_PREFIXES))
        rows[int(pr.group(1))] = {
            "ids": ids,
            "date": date,
            "subject": PR_RE.sub("", subject).strip().replace("|", "/"),
            "docs": docs,
        }
    return rows


def existing() -> dict[int, dict]:
    rows: dict[int, dict] = {}
    if not INDEX.exists():
        return rows
    for line in INDEX.read_text(encoding="utf-8").splitlines():
        m = ROW_RE.match(line)
        if m and m.group("ids").strip() != "Decision":
            rows[int(m.group("pr"))] = {
                "ids": [i.strip() for i in m.group("ids").split(",")],
                "date": m.group("date").strip(),
                "subject": m.group("subject").strip(),
                "docs": [d.strip(" `") for d in m.group("docs").split("<br>") if d.strip(" `")],
            }
    return rows


def sort_key(item: tuple[int, dict]) -> tuple:
    pr, row = item
    first = row["ids"][0]
    prefix, _, num = first.rpartition("D")
    return (prefix, int(num), pr)


def render(rows: dict[int, dict]) -> str:
    out = [
        "# Decision index",
        "",
        "Generated by `python tools/agents/build_decision_index.py` from squash-merge subjects and decision-document allocation lines on protected `main`.",
        "Regenerate it with the daily archive batch; rows are only added, never removed.",
        "",
        "D numbers are **not unique**: the owner decision sequence and several lane-local sequences",
        "(NPC, quest, encounter, spell `SPELL-D*`, `OPS-NODE-BOOT-01` and others) reuse the same numbers.",
        "Read the subject and the linked document before relying on a row.",
        "Lane slices such as `D3-4:` and Combat slices (`Combat D1`) are not decisions and are left out.",
        "A decision whose number never appeared in a merge subject is missing here; search `docs/architecture/reviews/` for it.",
        "",
        "| Decision | PR | Date | Subject | Documents |",
        "|---|---|---|---|---|",
    ]
    for pr, row in sorted(rows.items(), key=sort_key):
        docs = "<br>".join(f"`{d}`" for d in row["docs"])
        out.append(f"| {', '.join(row['ids'])} | #{pr} | {row['date']} | {row['subject']} | {docs} |")
    return "\n".join(out) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--ref", default="origin/main")
    parser.add_argument("--gaps", action="store_true", help="list added decision documents with no D number")
    args = parser.parse_args()
    gaps: list[str] = []
    rows = existing()
    try:
        rows.update(scan(args.ref, gaps))
    except DecisionConflict as exc:
        raise SystemExit(f"decision number conflict: {exc}")
    INDEX.write_text(render(rows), encoding="utf-8")
    print(f"{INDEX.relative_to(ROOT)}: {len(rows)} rows")
    if args.gaps:
        for gap in gaps:
            print(f"gap: {gap}")


if __name__ == "__main__":
    main()
