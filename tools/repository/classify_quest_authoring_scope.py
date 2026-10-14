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
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

TOOL_DIR = "tools/content-schema/quest-authoring"
DOC_STATUSES = {"added", "modified", "changed"}
# The GitHub pull-request files endpoint lists at most 3000 files.
MAX_ENUMERATED = 3000


def referenced_docs(
    root: Path, docs: list[str], base_blobs: dict[str, bytes] | None = None
) -> dict[str, str]:
    """Return {doc: reason} for each doc that another file in the Quest tool directory refers to."""
    # A pin may match the head content or the base content (a file changed without its pin updated).
    wanted: dict[str, list[bytes]] = {}
    for doc in docs:
        blobs = [(root / doc).read_bytes()] if (root / doc).is_file() else []
        if base_blobs and doc in base_blobs:
            blobs.append(base_blobs[doc])
        wanted[doc] = [hashlib.sha256(blob).hexdigest().encode() for blob in blobs]
    found: dict[str, str] = {}
    base = root / TOOL_DIR
    for current, _dirs, names in os.walk(base):
        current_rel = Path(current).relative_to(root).as_posix()
        pending = [doc for doc in docs if doc not in found]
        if not pending:
            return found
        for name in names:
            if name.endswith(".md") or name.endswith((".gz", ".png", ".jpg", ".zip", ".otbm")):
                continue
            data = (Path(current) / name).read_bytes()
            for doc in [doc for doc in pending if doc not in found]:
                if _mentions(data, doc, current_rel):
                    found[doc] = f"referenced by {current_rel}/{name}"
                elif any(digest in data for digest in wanted[doc]):
                    found[doc] = f"content hash pinned by {current_rel}/{name}"
    return found


_START_BREAKS = frozenset(b" \t\r\n\"'`()[]{}<>=,;:|")
_WORD = frozenset(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-")


def _mentions(data: bytes, doc: str, current_rel: str) -> bool:
    """True if `data` names `doc` by any path that resolves to it, with no limit on the alphabet.

    For each occurrence of the doc's file name, every text span that ends there and starts after a
    delimiter (or space) is normalized (./, ../, backslashes, percent escapes) against the referring
    file's directory, the repository root and the tool directory. A tool-relative bare name is only
    a pin for its own directory.
    """
    needle = posixpath.basename(doc).encode()
    tool_form = posixpath.relpath(doc, TOOL_DIR)
    tool_ok = posixpath.dirname(doc) != TOOL_DIR
    start = data.find(needle)
    while start != -1:
        end = start + len(needle)
        if end >= len(data) or data[end] not in _WORD:
            line_start = max(data.rfind(b"\n", 0, start) + 1, start - 1000)
            for pos in range(start, line_start - 1, -1):
                if pos == line_start or data[pos - 1] in _START_BREAKS:
                    raw = data[pos:end].decode("utf-8", "replace")
                    for text in {raw, raw.replace("\\", "/"), urllib.parse.unquote(raw)}:
                        if text.startswith("/"):
                            continue
                        if (
                            posixpath.normpath(text) == doc
                            or posixpath.normpath(posixpath.join(current_rel, text)) == doc
                            or (tool_ok and posixpath.normpath(text) == tool_form)
                        ):
                            return True
        start = data.find(needle, start + 1)
    return False


def classify(
    records: object, root: Path, *, complete: bool = True, base_blobs: dict[str, bytes] | None = None
) -> tuple[str, str]:
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
        if not re.fullmatch(r"[A-Za-z0-9._ +@/-]+", path):
            return "full", "doc path outside the plain alphabet"
        if ".." in path.split("/") or (root / path).is_symlink() or not (root / path).is_file():
            return "full", f"unreadable doc: {path}"
        docs.append(path)
    pinned = referenced_docs(root, docs, base_blobs)
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


def live_pull(repository: str, number: str, token: str) -> tuple[str, str]:
    """Current head and base SHA of the pull request."""
    request = urllib.request.Request(
        f"https://api.github.com/repos/{repository}/pulls/{number}",
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {token}",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "Oteryn-quest-scope",
        },
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        pull = json.load(response)
    return str(pull["head"]["sha"]), str(pull["base"]["sha"])


def fetch_base_blobs(repository: str, base_sha: str, token: str, records: object) -> dict[str, bytes]:
    """Base-tree content of each changed doc; a file absent at base (added) has none."""
    blobs: dict[str, bytes] = {}
    for record in records if isinstance(records, list) else []:
        path = record.get("filename") if isinstance(record, dict) else None
        if not isinstance(path, str) or not path.endswith(".md") or record.get("status") == "added":
            continue
        request = urllib.request.Request(
            f"https://api.github.com/repos/{repository}/contents/{urllib.parse.quote(path)}?ref={base_sha}",
            headers={
                "Accept": "application/vnd.github.raw+json",
                "Authorization": f"Bearer {token}",
                "X-GitHub-Api-Version": "2022-11-28",
                "User-Agent": "Oteryn-quest-scope",
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                blobs[path] = response.read()
        except urllib.error.HTTPError as error:
            if error.code != 404:
                raise
    return blobs


def safe_reason(reason: str) -> str:
    """Untrusted text (file names) never reaches GITHUB_OUTPUT: keep a fixed safe alphabet."""
    return re.sub(r"[^A-Za-z0-9 ._/:(),-]", "?", reason)[:200]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--records", type=Path)
    parser.add_argument("--repository")
    parser.add_argument("--pull-request")
    parser.add_argument("--base-sha")
    parser.add_argument("--head-sha")
    args = parser.parse_args(argv)
    root = args.root.resolve()
    try:
        if args.records:
            records, complete = json.loads(args.records.read_text(encoding="utf-8")), True
        elif args.repository and args.pull_request:
            token = os.environ.get("GH_TOKEN", "")
            if not (args.head_sha and args.base_sha):
                raise ValueError("expected head and base SHA missing")
            # The file list is mutable; it counts only if head and base were the checked-out
            # revision before and after enumeration (merge-gate.yml does the same).
            if live_pull(args.repository, args.pull_request, token) != (args.head_sha, args.base_sha):
                raise ValueError("pull request moved before enumeration")
            records, complete = fetch_records(args.repository, args.pull_request, token)
            if live_pull(args.repository, args.pull_request, token) != (args.head_sha, args.base_sha):
                raise ValueError("pull request moved during enumeration")
        else:
            records, complete = [], False
        base_blobs = (
            fetch_base_blobs(args.repository, args.base_sha, os.environ.get("GH_TOKEN", ""), records)
            if args.base_sha and args.repository and not args.records
            else None
        )
        verdict, reason = classify(records, root, complete=complete, base_blobs=base_blobs)
    except (OSError, ValueError, KeyError, TypeError) as error:
        verdict, reason = "full", f"classification failed closed: {type(error).__name__}"
    reason = safe_reason(reason)
    print(f"Quest scope: {verdict} ({reason})")
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as handle:
            handle.write(f"full={'false' if verdict == 'docs_only' else 'true'}\n")
            handle.write(f"reason={reason}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
