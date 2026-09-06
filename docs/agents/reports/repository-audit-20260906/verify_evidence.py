#!/usr/bin/env python3
"""Verify this audit's provenance and reconstruct its per-file coverage ledger.

No network, installation, source mutation or execution of project code is performed.
This is evidence verification, not a rerun of builds or semantic review.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys


def git(source: Path, *arguments: str) -> bytes:
    result = subprocess.run(
        ["git", "-C", str(source), *arguments], check=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    return result.stdout


def reconstruct_inventory(source: Path, register: dict) -> bytes:
    commit = register["audited_commit"]
    observed = git(source, "rev-parse", f"{commit}^{{tree}}").decode().strip()
    if observed != register["audited_tree"]:
        raise ValueError("audited tree identity mismatch")
    focused = set(register["focused_paths"])
    output = io.StringIO()
    writer = csv.writer(output, lineterminator="\n")
    writer.writerow(["path", "git_blob_sha", "bytes", "lines", "full_text_checks", "semantic_scope"])
    count = 0
    seen: set[str] = set()
    for record in git(source, "ls-tree", "-r", "-l", "-z", commit).split(b"\0"):
        if not record:
            continue
        info, raw_path = record.split(b"\t", 1)
        mode, kind, blob, declared_size = info.decode("ascii").split()
        path = raw_path.decode("utf-8")
        if kind != "blob" or mode not in {"100644", "100755"}:
            raise ValueError(f"unexpected non-regular audited entry: {path}")
        data = git(source, "cat-file", "blob", blob)
        expected = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
        if expected != blob or len(data) != int(declared_size):
            raise ValueError(f"audited blob mismatch: {path}")
        text = data.decode("utf-8")
        flags = ["IDENTITY"]
        if path.endswith(".md"):
            flags.append("COMMONMARK_REFERENCES_DUPLICATES")
        if path.endswith(".py"):
            flags.append("PYTHON_AST")
        if path.endswith(".rs"):
            flags.append("RUST_LEXICAL")
        if path.endswith(".json"):
            flags.append("JSON")
        if path.endswith((".yml", ".yaml")):
            flags.append("YAML")
        if path.endswith(".toml") or path == "Cargo.lock":
            flags.append("CARGO_ACCEPTED" if path == "Cargo.toml" else "TOML10")
        if path.startswith(".github/workflows/"):
            flags.append("WORKFLOW_TOPOLOGY_PERMISSIONS")
        scope = "FOCUSED_NOT_FULL_PROOF" if path in focused else "STATIC_ONLY_NOT_SEMANTIC_REVIEW"
        writer.writerow([path, blob, len(data), len(text.splitlines()), ";".join(flags), scope])
        count += 1
        seen.add(path)
    if count != register["files"] or not focused <= seen:
        raise ValueError("file count or focused-path membership mismatch")
    payload = output.getvalue().encode("utf-8")
    if hashlib.sha256(payload).hexdigest() != register["inventory_csv_sha256"]:
        raise ValueError("reconstructed coverage CSV digest mismatch")
    return payload


def verify_retained_evidence(directory: Path, register: dict) -> None:
    for name, expected in register["retained_evidence_sha256"].items():
        if name not in {"execution-evidence.json", "EVIDENCE.md"}:
            raise ValueError("unexpected retained evidence path")
        data = (directory / name).read_bytes()
        if hashlib.sha256(data).hexdigest() != expected:
            raise ValueError(f"retained evidence digest mismatch: {name}")
    evidence = json.loads((directory / "execution-evidence.json").read_text(encoding="utf-8"))
    if evidence["source_commit"] != register["audited_commit"]:
        raise ValueError("retained execution evidence source mismatch")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, help="Git repository containing the audited commit")
    parser.add_argument("--inventory-out", type=Path, help="new CSV output file (requires --source)")
    args = parser.parse_args()
    if args.inventory_out and not args.source:
        parser.error("--inventory-out requires --source")
    directory = Path(__file__).resolve().parent
    try:
        register = json.loads((directory / "coverage-register.json").read_text(encoding="utf-8"))
        verify_retained_evidence(directory, register)
        result = {"evidence_integrity": "PASS", "audit_completeness": "QUALIFIED_NOT_100_PERCENT_SEMANTIC", "source": register["audited_commit"]}
        if args.source:
            payload = reconstruct_inventory(args.source, register)
            if args.inventory_out:
                with args.inventory_out.open("xb") as stream:
                    stream.write(payload)
            result["reconstructed_file_rows"] = register["files"]
            result["inventory_sha256"] = hashlib.sha256(payload).hexdigest()
        print(json.dumps(result, indent=2))
        return 0
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"Audit evidence verification failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
