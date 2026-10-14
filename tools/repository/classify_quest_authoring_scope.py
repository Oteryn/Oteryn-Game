#!/usr/bin/env python3
"""Decide whether a Quest-authoring pull request needs the full offline Quest check.

`.github/workflows/quest-authoring-schema.yml` runs `run_checks.py` (about 18 minutes) for every
change under `tools/content-schema/quest-authoring/`. A pull request that changes only prose there
does not need it, but a blanket `*.md` ignore would be wrong: some README files are hash-pinned
inputs (for example `donor_sources/maps/README.md` in `qualification.json`).

The result is `docs_only` only when every changed file of the pull request is an added or modified
`.md` file under the Quest tool directory that no other file there references, by relative path,
repository path or SHA-256 of its current content. Everything else is `full`: a mixed diff, a
rename or delete, an unknown status or path, a workflow, build or dependency change, an incomplete
file list, a referenced README. Any doubt is `full`; the check never skips on a guess.

Usage:
  classify_quest_authoring_scope.py --root DIR --pull-request N --repository OWNER/REPO
  classify_quest_authoring_scope.py --root DIR --records FILE.json

With `GITHUB_OUTPUT` set it writes `full=true|false` and `reason=...`.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import posixpath
import re
import sys
import urllib.request
from pathlib import Path

TOOL_DIR = "tools/content-schema/quest-authoring"
DOC_STATUSES = {"added", "modified", "changed"}
# The GitHub pull-request files endpoint lists at most 3000 files.
MAX_ENUMERATED = 3000


def referenced_docs(root: Path, docs: list[str]) -> dict[str, str]:
    """Return {doc: reason} for each doc that another file in the Quest tool directory refers to."""
    wanted = {doc: hashlib.sha256((root / doc).read_bytes()).hexdigest().encode() for doc in docs if (root / doc).is_file()}
    found: dict[str, str] = {}
    base = root / TOOL_DIR
    for current, _dirs, names in os.walk(base):
        current_rel = Path(current).relative_to(root).as_posix()
        candidates = {
            doc: re.compile(
                rb"(?<![\w/\-])(?:"
                + b"|".join(
                    re.escape(path.encode())
                    for path in {
                        posixpath.relpath(doc, current_rel),
                        doc,
                        # Tool-relative form; a bare name is only a pin for its own directory.
                        *([posixpath.relpath(doc, TOOL_DIR)] if posixpath.dirname(doc) != TOOL_DIR else []),
                    }
                )
                + rb")(?![\w\-])"
            )
            for doc in docs
            if doc not in found
        }
        if not candidates:
            return found
        for name in names:
            if name.endswith(".md") or name.endswith((".gz", ".png", ".jpg", ".zip", ".otbm")):
                continue
            data = (Path(current) / name).read_bytes()
            for doc, pattern in candidates.items():
                if doc in found:
                    continue
                if posixpath.basename(doc).encode() in data and pattern.search(data):
                    found[doc] = f"referenced by {current_rel}/{name}"
                elif doc in wanted and wanted[doc] in data:
                    found[doc] = f"content hash pinned by {current_rel}/{name}"
    return found


def classify(records: object, root: Path, *, complete: bool = True) -> tuple[str, str]:
    """Return ("docs_only" | "full", reason) for the pull request file records."""
    if not complete:
        return "full", "changed-file list is incomplete"
    if not isinstance(records, list) or not records:
        return "full", "no changed-file records"
    if len(records) >= MAX_ENUMERATED:
        return "full", "changed-file list reached the API limit"
    docs: list[str] = []
    for record in records:
        if not isinstance(record, dict):
            return "full", "malformed changed-file record"
        path = record.get("filename")
        status = record.get("status")
        if not isinstance(path, str) or not isinstance(status, str):
            return "full", "malformed changed-file record"
        if status not in DOC_STATUSES or record.get("previous_filename"):
            return "full", f"{status} {path}"
        if not path.startswith(TOOL_DIR + "/") or not path.endswith(".md"):
            return "full", f"not a Quest tool doc: {path}"
        if ".." in path.split("/") or not (root / path).is_file():
            return "full", f"unreadable doc: {path}"
        docs.append(path)
    pinned = referenced_docs(root, docs)
    if pinned:
        doc, why = sorted(pinned.items())[0]
        return "full", f"{doc} {why}"
    return "docs_only", f"{len(docs)} unreferenced Quest tool doc(s)"


def fetch_records(repository: str, number: str, token: str) -> tuple[list[dict], bool]:
    records: list[dict] = []
    for page in range(1, 31):
        request = urllib.request.Request(
            f"https://api.github.com/repos/{repository}/pulls/{number}/files?per_page=100&page={page}",
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {token}",
                "X-GitHub-Api-Version": "2022-11-28",
                "User-Agent": "Oteryn-quest-scope",
            },
        )
        with urllib.request.urlopen(request, timeout=30) as response:
            batch = json.load(response)
        if not isinstance(batch, list):
            return [], False
        records += [
            {key: item[key] for key in ("filename", "status", "previous_filename") if key in item}
            for item in batch
            if isinstance(item, dict)
        ]
        if len(batch) < 100:
            return records, True
    return records, False


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--records", type=Path)
    parser.add_argument("--repository")
    parser.add_argument("--pull-request")
    args = parser.parse_args(argv)
    root = args.root.resolve()
    try:
        if args.records:
            records, complete = json.loads(args.records.read_text(encoding="utf-8")), True
        elif args.repository and args.pull_request:
            records, complete = fetch_records(args.repository, args.pull_request, os.environ.get("GH_TOKEN", ""))
        else:
            records, complete = [], False
        verdict, reason = classify(records, root, complete=complete)
    except (OSError, ValueError) as error:
        verdict, reason = "full", f"classification failed closed: {type(error).__name__}"
    print(f"Quest scope: {verdict} ({reason})")
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as handle:
            handle.write(f"full={'false' if verdict == 'docs_only' else 'true'}\n")
            handle.write(f"reason={reason}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
