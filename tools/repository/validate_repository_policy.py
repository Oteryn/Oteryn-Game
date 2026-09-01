#!/usr/bin/env python3
"""Fail-closed wrapper for the Game repository-policy validator."""
from __future__ import annotations

import hashlib
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CORE_PATH = Path(__file__).with_name("validate_repository_policy_core.py")
MERGE_GROUP_GATE = ROOT / ".github/workflows/merge-group-gate.yml"
EXPECTED_MERGE_GROUP_GATE_BLOB = "1e0e7b70a806fe744d394ca8abf43ee434ead3f2"


def git_blob_sha(data: bytes) -> str:
    return hashlib.sha1(f"blob {len(data)}\0".encode("ascii") + data).hexdigest()


def load_core():
    spec = importlib.util.spec_from_file_location("validate_repository_policy_core", CORE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load repository policy core: {CORE_PATH}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main() -> int:
    if not MERGE_GROUP_GATE.is_file():
        print("Repository policy validation failed:\n- missing exact Merge Queue gate")
        return 1
    actual = git_blob_sha(MERGE_GROUP_GATE.read_bytes())
    if actual != EXPECTED_MERGE_GROUP_GATE_BLOB:
        print(
            "Repository policy validation failed:\n"
            f"- merge-group gate blob drift: expected {EXPECTED_MERGE_GROUP_GATE_BLOB}, got {actual}"
        )
        return 1
    return load_core().main()


if __name__ == "__main__":
    raise SystemExit(main())
