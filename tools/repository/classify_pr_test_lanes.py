#!/usr/bin/env python3
"""Select conservative PR lanes using only a verified protected-base checkout.

Cargo edges are not a complete file-input graph. The audited snapshot additionally
binds all non-server workspace package trees and root build/dependency inputs.
Changed consumer code (including new cross-package includes) disables the server
optimization until a reviewed update adopts that input contract.
"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys

AUDITED_INPUT_SHA256 = "9f7aff4dc25c9c6561b77ea73342b675eeccb1d008ab9d1fbdbd504618ec5ab8"
AUDITED_DOC_INPUT_SHA256 = "b983e36bc734fafd55dcad16fb0a335d73a490338b30e10a999f4899bde5da43"
SERVER = "oteryn-game-server"
WINDOWS = {"oteryn-client", "oteryn-synthetic-client-harness", "oteryn-simulation-determinism"}
REQUIRED = {
    SERVER: "apps/game-server",
    "oteryn-client": "apps/client",
    "oteryn-synthetic-client-harness": "tools/synthetic-client-harness",
    "oteryn-simulation-determinism": "crates/simulation-determinism",
}
BUILD_INPUTS = {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rustfmt.toml", "deny.toml", "workspace-boundaries.toml", ".gitattributes", ".gitmodules"}


def full(reason: str, surface: str = "unknown") -> dict:
    return dict(rust=True, windows=True, surface=surface, reason=reason)


def valid_path(value) -> bool:
    return isinstance(value, str) and bool(value) and not any(c in value for c in "\x00\n\r\\") and not value.startswith("/") and ".." not in value.split("/") and str(PurePosixPath(value)) == value


def neutral(path: str) -> bool:
    if PurePosixPath(path).name in {"AGENTS.md", "AGENTS.override.md"} or path.startswith("docs/migration/"):
        return False
    return path in {"README.md", "CHANGELOG.md", "CONTRIBUTING.md"} or (path.startswith("docs/") and path.endswith(".md"))


def graph(metadata: dict):
    workspace = PurePosixPath(metadata["workspace_root"])
    if not workspace.is_absolute():
        raise ValueError("relative workspace")
    packages = metadata["packages"]
    members = metadata["workspace_members"]
    if not isinstance(packages, list) or not isinstance(members, list) or not packages or len(set(members)) != len(members):
        raise ValueError("invalid workspace")
    roots, ids = {}, set()
    for package in packages:
        name, ident = package["name"], package["id"]
        manifest = PurePosixPath(package["manifest_path"])
        root = str(manifest.parent.relative_to(workspace))
        if not isinstance(name, str) or not name or name in roots or ident in ids or manifest.name != "Cargo.toml" or not valid_path(root) or root == ".":
            raise ValueError("ambiguous package")
        roots[name] = root
        ids.add(ident)
    if ids != set(members) or any(roots.get(name) != root for name, root in REQUIRED.items()):
        raise ValueError("unresolved workspace roots")
    if len(set(roots.values())) != len(roots) or any(a != b and a.startswith(b + "/") for a in roots.values() for b in roots.values()):
        raise ValueError("overlapping package roots")
    by_path = {str(workspace / path): name for name, path in roots.items()}
    reverse = {name: set() for name in roots}
    for package in packages:
        dependencies = package["dependencies"]
        if not isinstance(dependencies, list):
            raise ValueError("invalid dependencies")
        for dependency in dependencies:
            if not isinstance(dependency.get("name"), str) or dependency.get("kind") not in {None, "dev", "build"}:
                raise ValueError("unknown dependency")
            path = dependency.get("path")
            if path is not None:
                if path not in by_path:
                    raise ValueError("unresolved local dependency")
                # Deliberately union optional, dev/build and every target condition.
                reverse[by_path[path]].add(package["name"])
    return roots, reverse


def input_digest(metadata: dict, include_server: bool = False) -> str:
    roots, _ = graph(metadata)
    prefixes = tuple(path + "/" for name, path in roots.items() if include_server or name != SERVER)
    records = subprocess.check_output(["git", "ls-tree", "-r", "-z", "HEAD"]).split(b"\0")
    selected = []
    for record in records:
        if not record:
            continue
        info, raw_path = record.split(b"\t", 1)
        if info.split()[0] not in {b"100644", b"100755"}:
            raise ValueError("unmodelled symlink/submodule input")
        path = raw_path.decode("utf-8")
        if path in BUILD_INPUTS or path.startswith(".cargo/") or path.startswith(prefixes):
            selected.append(record)
    return hashlib.sha256(b"\0".join(sorted(selected)) + b"\0").hexdigest()


def candidate_modes_safe(sha: str) -> bool:
    if re.fullmatch(r"[0-9a-f]{40}", sha) is None:
        return False
    rows = subprocess.check_output(["git", "ls-tree", "-r", "-z", sha]).split(b"\0")
    entries = [row for row in rows if row]
    return bool(entries) and all(row.split(b"\t", 1)[0].split()[0] in {b"100644", b"100755"} for row in entries)


def classify(files, changed_count, metadata, digest, complete=True, docs_digest=None, candidate_modes_verified=False) -> dict:
    try:
        if candidate_modes_verified is not True:
            return full("unverified-or-special-candidate-modes")
        if complete is not True or type(changed_count) is not int or not isinstance(files, list) or len(files) != changed_count or not files:
            return full("incomplete-enumeration")
        paths, filenames = [], set()
        for item in files:
            path = item["filename"]
            status = item.get("status")
            previous = item.get("previous_filename")
            if not valid_path(path) or path in filenames or status not in {"added", "modified", "removed", "renamed", "copied", "changed", "unchanged"}:
                return full("invalid-file-record")
            filenames.add(path)
            paths.append(path)
            if status == "renamed" and not previous:
                return full("missing-rename-source")
            if previous is not None:
                if not valid_path(previous):
                    return full("invalid-rename-source")
                if neutral(path) != neutral(previous):
                    return full("cross-surface-rename")
                paths.append(previous)
        if all(neutral(path) for path in paths):
            graph(metadata)
            if digest != AUDITED_INPUT_SHA256 or docs_digest != AUDITED_DOC_INPUT_SHA256:
                return full("unreviewed-document-consumer-inputs", "docs")
            return dict(rust=False, windows=False, surface="docs", reason="neutral-documentation")
        if any(path.startswith(".cargo/") or PurePosixPath(path).name in BUILD_INPUTS | {"build.rs"} for path in paths):
            return full("explicit-build-or-dependency-input", "dependencies-build")
        if any(path.startswith((".github/", "tools/repository/", "tools/agents/", "docs/migration/")) or PurePosixPath(path).name in {"AGENTS.md", "AGENTS.override.md"} for path in paths):
            return full("explicit-build-or-control-input", "control-plane")
        roots, reverse = graph(metadata)
        affected = set()
        for path in paths:
            if neutral(path):
                continue
            owners = [name for name, root in roots.items() if path.startswith(root + "/")]
            if len(owners) != 1 or PurePosixPath(path).suffix not in {".rs", ".sql"}:
                return full("unmodelled-input")
            affected.add(owners[0])
        pending = list(affected)
        while pending:
            for consumer in reverse[pending.pop()] - affected:
                affected.add(consumer)
                pending.append(consumer)
        if affected & WINDOWS:
            surface = "simulation" if "oteryn-simulation-determinism" in affected else "shared" if SERVER in affected else "client"
            return full("windows-consumer-affected", surface)
        if SERVER not in affected or affected != {SERVER}:
            return full("mixed-or-unowned-surface")
        if digest != AUDITED_INPUT_SHA256:
            return full("unreviewed-consumer-input-snapshot")
        surface = "durability" if any(any(token in path for token in ("/durability/", "/migrations/", "postgres", "reconnect")) for path in paths) else "server"
        return dict(rust=True, windows=False, surface=surface, reason="server-only-reverse-closure-and-audited-inputs")
    except (KeyError, TypeError, ValueError, AttributeError):
        return full("classifier-input-failure")


def classify_post_merge(event, metadata) -> dict:
    """Reuse PR risk semantics only for a verified, complete protected-main push.

    The workflow executes this code and Cargo metadata at the already-protected
    event SHA. Neither the event's capped commit list nor PR metadata is used.
    Git's complete before/after tree diff also covers batched queue merges.
    """
    try:
        if (os.environ.get("GITHUB_EVENT_NAME") != "push"
                or os.environ.get("GITHUB_REF") != "refs/heads/main"
                or os.environ.get("GITHUB_REF_PROTECTED") != "true"
                or os.environ.get("GITHUB_REPOSITORY") != "Oteryn/Oteryn-Game"
                or event["repository"]["full_name"] != "Oteryn/Oteryn-Game"
                or event["ref"] != "refs/heads/main"
                or any(event[key] is not False for key in ("forced", "created", "deleted"))):
            return full("not-a-normal-protected-main-push")
        before, after = event["before"], event["after"]
        if any(not isinstance(sha, str) or re.fullmatch(r"[0-9a-f]{40}", sha) is None or sha == "0" * 40 for sha in (before, after)):
            return full("invalid-push-range")
        if before == after or after != os.environ.get("GITHUB_SHA"):
            return full("push-identity-mismatch")
        actual = subprocess.check_output(["git", "rev-parse", "HEAD"]).decode().strip()
        if actual != after or subprocess.check_output(["git", "rev-parse", "--is-shallow-repository"]).strip() != b"false":
            return full("unverified-or-incomplete-protected-checkout")
        subprocess.check_output(["git", "merge-base", "--is-ancestor", before, after], stderr=subprocess.PIPE)
        # --no-renames retains both sides of renames/copies as ordinary paths.
        raw = subprocess.check_output(["git", "diff", "--no-ext-diff", "--no-textconv", "--no-renames", "--name-status", "-z", before, after, "--"])
        if not raw or not raw.endswith(b"\0"):
            return full("empty-or-incomplete-git-diff")
        fields = raw[:-1].split(b"\0")
        if len(fields) % 2:
            return full("malformed-git-diff")
        statuses = {b"A": "added", b"M": "modified", b"D": "removed"}
        files = [{"filename": fields[index + 1].decode("utf-8"), "status": statuses[fields[index]]}
                 for index in range(0, len(fields), 2)]
        result = classify(files, len(files), metadata, input_digest(metadata),
                          candidate_modes_verified=candidate_modes_safe(after))
        # Post-merge policy never omits Linux/PG/policy/supply chain, even docs.
        if result["rust"] is True and result["windows"] is False and result["surface"] in {"server", "durability"}:
            return result
        return full(result["reason"], result["surface"])
    except (OSError, ValueError, KeyError, IndexError, TypeError, AttributeError, subprocess.SubprocessError):
        return full("post-merge-input-or-git-failure")


def main() -> int:
    try:
        post_merge = sys.argv[1] == "--post-merge"
        metadata = json.loads(Path(sys.argv[2] if post_merge else sys.argv[1]).read_text(encoding="utf-8"))
        if post_merge:
            result = classify_post_merge(json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8")), metadata)
        else:
            digest = input_digest(metadata)
            result = classify(json.loads(os.environ["CHANGED_FILE_RECORDS"]), int(os.environ["CHANGED_FILE_COUNT"]), metadata, digest,
                              complete=os.environ["ENUMERATION_COMPLETE"] == "true", docs_digest=input_digest(metadata, include_server=True),
                              candidate_modes_verified=candidate_modes_safe(os.environ["EXPECTED_HEAD"]))
    except (OSError, ValueError, KeyError, IndexError, TypeError, AttributeError, subprocess.SubprocessError):
        result = full("classifier-or-metadata-failure")
    print(json.dumps(result, sort_keys=True))
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"rust={str(result['rust']).lower()}\nwindows={str(result['windows']).lower()}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
