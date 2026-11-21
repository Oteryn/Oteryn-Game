#!/usr/bin/env python3
"""Compute the CI pin-rotation proposal from its actual source bytes.

This generates content, not authority: the protected-base audit still rejects
its own modification, and integration still requires owner authorization and
independent review. Existing legacy-head pins deliberately remain unchanged.
"""

import argparse
import hashlib
from pathlib import Path
import re


ROOT = Path(__file__).resolve().parents[2]
CORE = "tools/repository/validate_repository_policy_core.py"
AUDIT = ".github/workflows/merge-authority-audit.yml"
CONTRACT_TEST = "tools/repository/test_validate_merge_group_pg_sim.py"
RUST_WORKFLOW = ".github/workflows/rust.yml"
REFERENCES = (
    CORE,
    ".github/workflows/merge-group-gate.yml",
    ".github/workflows/pr-postgres-harness.yml",
    ".github/workflows/postgres-test-partitions.yml",
    ".github/workflows/node-boot-qualification.yml",
)
SOURCES = {
    "tools/ci/plan_pg_partitions.py": ("plan_pg_partitions.py", '"$planner"'),
    ".github/workflows/node-boot-build.yml": (".github/workflows/node-boot-build.yml", "$build_workflow"),
    "tools/qualification/node_boot/prebuilt.py": ("tools/qualification/node_boot/prebuilt.py", "$helper"),
}


def git_blob(data):
    return hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest()


def replace_constant(text, name, digest):
    if len(digest) not in (40, 64) or re.fullmatch(r"[0-9a-f]+", digest) is None:
        raise ValueError("invalid derived pin digest")
    pattern = rf'(?m)^(\s*{name}\s*[:=]\s*")[0-9a-f]{{{len(digest)}}}("\s*)$'
    result, count = re.subn(pattern, lambda match: match[1] + digest + match[2], text)
    if count != 1:
        raise ValueError(f"expected one exact pin: {name}")
    return result


def proposal(root):
    paths = set(REFERENCES) | {AUDIT, CONTRACT_TEST, ".github/workflows/merge-gate.yml", RUST_WORKFLOW}
    original = {path: (root / path).read_text(encoding="utf-8") for path in paths}
    changed = dict(original)
    digests = {path: hashlib.sha256((root / path).read_bytes()).hexdigest() for path in SOURCES}
    seen = {path: 0 for path in SOURCES}
    for path in REFERENCES:
        lines = []
        for line in changed[path].splitlines(keepends=True):
            if "sha256sum -c -" in line:
                for source, markers in SOURCES.items():
                    if any(marker in line for marker in markers):
                        line, count = re.subn(r"\b[0-9a-f]{64}\b", digests[source], line)
                        if count != 1:
                            raise ValueError(f"expected one SHA-256 reference to {source} in {path}")
                        seen[source] += 1
            lines.append(line)
        changed[path] = "".join(lines)
    if not all(seen.values()):
        raise ValueError("a source has no protected digest reference")

    group = git_blob(changed[".github/workflows/merge-group-gate.yml"].encode())
    pr = git_blob(changed[".github/workflows/merge-gate.yml"].encode())
    changed[CORE] = replace_constant(changed[CORE], "EXPECTED_MERGE_GROUP_GATE_BLOB", group)
    changed[CORE] = replace_constant(changed[CORE], "EXPECTED_POST_MERGE_RUST_SHA256",
                                     hashlib.sha256(original[RUST_WORKFLOW].encode()).hexdigest())
    changed[AUDIT] = replace_constant(changed[AUDIT], "EXPECTED_MERGE_GROUP_GATE_BLOB", group)
    changed[AUDIT] = replace_constant(changed[AUDIT], "EXPECTED_MERGE_GATE_BLOB", pr)
    changed[CONTRACT_TEST] = replace_constant(changed[CONTRACT_TEST], "APPROVED", group)
    return {path: text for path, text in changed.items() if text != original[path]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="report stale derived pins without writing")
    args = parser.parse_args()
    changes = proposal(ROOT)
    if args.check:
        for path in sorted(changes):
            print(f"stale CI pins: {path}")
        return int(bool(changes))
    for path, content in sorted(changes.items()):
        (ROOT / path).write_text(content, encoding="utf-8")
        print(f"refreshed CI pins: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
