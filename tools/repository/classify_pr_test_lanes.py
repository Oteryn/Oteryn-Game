#!/usr/bin/env python3
"""Select conservative PR lanes using only a verified protected-base checkout.

Cargo edges are not a complete file-input graph. The audited non-server snapshot
binds server-only isolation. Neutral-documentation routing separately compares
protected consumer drift against an audited protected-main baseline and fails
closed only when later changes can affect build/document input discovery.
"""
from __future__ import annotations

import difflib
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]

AUDITED_INPUT_SHA256 = "220e5dbe065665815eefb8219ac9d4609f5d06afbe15315d7a6d2b200839f3a6"
AUDITED_DOC_INPUT_SHA256 = "4b37d0e2e6c70161a29f3def3891a17a9c3e48f4048b883fa457b66b20d654b3"
AUDITED_DOC_CONSUMER_BASE_SHA = "256aa3b152c944cb8451906effe1f0090c5b798d"
SERVER = "oteryn-game-server"
WINDOWS = {"oteryn-client", "oteryn-synthetic-client-harness", "oteryn-simulation-determinism"}
REQUIRED = {
    SERVER: "apps/game-server",
    "oteryn-client": "apps/client",
    "oteryn-synthetic-client-harness": "tools/synthetic-client-harness",
    "oteryn-simulation-determinism": "crates/simulation-determinism",
}
BUILD_INPUTS = {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rustfmt.toml", "deny.toml", "workspace-boundaries.toml", ".gitattributes", ".gitmodules"}
ATLAS_FULLWORLD_PATHS = {
    "tools/game-atlas-fullworld-source/producer.py",
    "tools/game-atlas-fullworld-source/self_test.py",
}
ATLAS_INTENTIONALLY_FULL_PATHS = {
    "tools/game-atlas-fullworld-source/README.md",
    "tools/game-atlas-fullworld-source/animated.py",
    "tools/game-atlas-fullworld-source/animated_self_test.py",
}
ATLAS_INTENTIONALLY_FULL_PREFIXES = (
    "tools/game-atlas-appearances/",
    "tools/game-atlas-outfit-spatial/",
    "tools/game-atlas-creature-gameplay/",
    "tools/game-atlas-creatures/",
    "tools/game-atlas-profile-spike/",
    "tools/game-atlas-semantic-search/",
    "tools/game-atlas-thais-fixture/",
)
DEGRADED_ROUTING_REASONS = {
    "classifier-input-failure",
    "classifier-or-metadata-failure",
    "incomplete-enumeration",
    "invalid-file-record",
    "missing-rename-source",
    "invalid-rename-source",
    "unreviewed-consumer-input-snapshot",
    "unreviewed-document-consumer-inputs",
    "unverified-or-special-candidate-modes",
}
UNMODELLED_ROUTING_REASONS = {
    "mixed-or-unowned-surface",
    "unmodelled-input",
}


def full(reason: str, surface: str = "unknown") -> dict:
    return dict(rust=True, windows=True, surface=surface, reason=reason)


def routing_health(result: dict) -> str:
    reason = result.get("reason")
    if reason in DEGRADED_ROUTING_REASONS:
        return "degraded"
    if reason in UNMODELLED_ROUTING_REASONS:
        return "unmodelled"
    return "modelled"


def valid_path(value) -> bool:
    return isinstance(value, str) and bool(value) and not any(c in value for c in "\x00\n\r\\") and not value.startswith("/") and ".." not in value.split("/") and str(PurePosixPath(value)) == value


def external_local_dependency_roots() -> tuple[str, ...]:
    """Return root-manifest [patch.*] path trees consumed outside workspace membership.

    Cargo accepts manifest constructs that Python's TOML 1.0 parser does not, so keep this
    deliberately narrow: inspect only patch sections and extract literal path assignments.
    The root Cargo.toml is already an audited BUILD_INPUT, so any patch-table shape change
    invalidates the snapshot before a new path tree can be trusted.
    """
    roots: set[str] = set()
    in_patch = False
    header = re.compile(r"^\s*\[([^\]]+)\]\s*(?:#.*)?$")
    path_assignment = re.compile(r'\bpath\s*=\s*"([^"]+)"')

    for line in (ROOT / "Cargo.toml").read_text(encoding="utf-8").splitlines():
        match = header.match(line)
        if match is not None:
            in_patch = match.group(1).startswith("patch.")
            continue
        if not in_patch:
            continue
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        path_match = path_assignment.search(line)
        if path_match is None:
            continue
        raw = path_match.group(1)
        candidate = PurePosixPath(raw)
        normalized = str(candidate)
        if candidate.is_absolute() or normalized == "." or not valid_path(normalized):
            raise ValueError("unsafe patched dependency path")
        roots.add(normalized)

    return tuple(sorted(roots))

def neutral(path: str) -> bool:
    if PurePosixPath(path).name in {"AGENTS.md", "AGENTS.override.md"} or path.startswith("docs/migration/"):
        return False
    return path in {"README.md", "CHANGELOG.md", "CONTRIBUTING.md"} or (path.startswith("docs/") and path.endswith(".md"))


def agent_governance(path: str) -> bool:
    """Return paths whose semantics are validated by governance/policy gates, not product builds."""
    if PurePosixPath(path).name in {"AGENTS.md", "AGENTS.override.md"}:
        return True
    if path.startswith("tools/agents/"):
        return True
    return path.startswith("docs/agents/") and not path.startswith("docs/agents/evidence/")


def agent_task_record(path: str) -> bool:
    """Return task-lifecycle records owned exclusively by agent governance."""
    return path.startswith("docs/agents/tasks/")


def non_runtime(path: str) -> bool:
    return neutral(path) or agent_governance(path)


def atlas_fullworld_path(path: str) -> bool:
    return path in ATLAS_FULLWORLD_PATHS


def atlas_path_disposition(path: str) -> str | None:
    if path in ATLAS_FULLWORLD_PATHS:
        return "atlas-fullworld"
    if path in ATLAS_INTENTIONALLY_FULL_PATHS or path.startswith(ATLAS_INTENTIONALLY_FULL_PREFIXES):
        return "full"
    return None


def atlas_fullworld_required(files, changed_count, complete=True) -> bool:
    """Select the bounded Atlas producer lane, failing closed on malformed evidence."""
    try:
        if complete is not True or type(changed_count) is not int or not isinstance(files, list) or len(files) != changed_count or not files:
            return True
        for item in files:
            if not isinstance(item, dict):
                return True
            path = item.get("filename")
            previous = item.get("previous_filename")
            if not valid_path(path):
                return True
            if atlas_fullworld_path(path):
                return True
            if previous is not None:
                if not valid_path(previous):
                    return True
                if atlas_fullworld_path(previous):
                    return True
        return False
    except (TypeError, ValueError, AttributeError):
        return True


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


def audited_input_path(metadata: dict, path: str, include_server: bool = False) -> bool:
    roots, _ = graph(metadata)
    prefixes = tuple(root + "/" for name, root in roots.items() if include_server or name != SERVER)
    prefixes += tuple(root + "/" for root in external_local_dependency_roots())
    return path in BUILD_INPUTS or path.startswith(".cargo/") or path.startswith(prefixes)


def input_digest(metadata: dict, include_server: bool = False) -> str:
    roots, _ = graph(metadata)
    prefixes = tuple(path + "/" for name, path in roots.items() if include_server or name != SERVER)
    prefixes += tuple(root + "/" for root in external_local_dependency_roots())
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


def document_consumer_content_safe(path: str, baseline: bytes, current: bytes | None = None) -> bool:
    """Prove that baseline-to-current drift cannot add or modify a consumer.

    This intentionally is not an absence-of-known-markers test.  Rust is open
    ended (aliases, grouped imports and macros can all hide filesystem access),
    so only blank lines and ordinary non-doc line comments are admitted. Rust
    doc comments are attributes (and macro-visible), while every executable or
    uncertain line can change behavior, so all of those fail closed.
    """
    if path.startswith(".cargo/") or PurePosixPath(path).name in BUILD_INPUTS | {"build.rs"}:
        return False
    if current is None:
        current, baseline = baseline, b""
    if PurePosixPath(path).suffix != ".rs":
        return baseline == current
    try:
        before_lines, current_lines = baseline.splitlines(), current.splitlines()
        before_comments = rust_ordinary_comment_lines(baseline)
        current_comments = rust_ordinary_comment_lines(current)
        if before_comments is None or current_comments is None:
            return False
        matcher = difflib.SequenceMatcher(None, before_lines, current_lines, autojunk=False)
        for tag, before_start, before_end, current_start, current_end in matcher.get_opcodes():
            if tag == "equal":
                continue
            changed = ((before_lines, before_comments, before_start, before_end),
                       (current_lines, current_comments, current_start, current_end))
            for lines, comments, start, end in changed:
                for index in range(start, end):
                    line = lines[index]
                    if line.strip() and not comments[index]:
                        return False
        return True
    except (TypeError, UnicodeError):
        return False


def rust_ordinary_comment_lines(content: bytes) -> list[bool] | None:
    """Identify standalone ordinary line comments in proven Rust code context.

    This deliberately small lexer tracks every Rust construct that can span a
    line and make a leading ``//`` mere content. Unknown or unterminated state
    is rejected rather than guessed safe.
    """
    lines = content.splitlines()
    ordinary = [False] * len(lines)
    state = "code"
    block_depth = 0
    raw_hashes = 0
    escaped = False
    for line_index, line in enumerate(lines):
        first = len(line) - len(line.lstrip())
        if state == "code" and line[first:].startswith(b"//"):
            ordinary[line_index] = not line[first:].startswith((b"///", b"//!"))

        index = 0
        while index < len(line):
            if state == "line":
                break
            if state == "block":
                if line.startswith(b"/*", index):
                    block_depth += 1
                    index += 2
                elif line.startswith(b"*/", index):
                    block_depth -= 1
                    index += 2
                    if block_depth == 0:
                        state = "code"
                else:
                    index += 1
                continue
            if state == "string":
                byte = line[index]
                index += 1
                if escaped:
                    escaped = False
                elif byte == 0x5C:
                    escaped = True
                elif byte == 0x22:
                    state = "code"
                continue
            if state == "raw":
                terminator = b'"' + (b"#" * raw_hashes)
                if line.startswith(terminator, index):
                    index += len(terminator)
                    state = "code"
                else:
                    index += 1
                continue

            if line.startswith(b"//", index):
                state = "line"
                break
            if line.startswith(b"/*", index):
                state, block_depth = "block", 1
                index += 2
                continue
            raw = re.match(br"(?:br|cr|r)(\#*)\"", line[index:])
            if raw is not None:
                state, raw_hashes = "raw", len(raw.group(1))
                index += len(raw.group(0))
                continue
            if line.startswith((b'b"', b'c"'), index):
                state, escaped = "string", False
                index += 2
                continue
            if line.startswith(b"b'", index):
                end = rust_character_literal_end(line, index + 1, byte=True)
                if end is None:
                    return None
                index = end
                continue
            if line[index] == 0x27:
                end = rust_character_literal_end(line, index, byte=False)
                if end is None:
                    # Lifetimes and labels are not character literals and have
                    # no lexical state to track. Anything else is uncertain.
                    lifetime = re.match(br"'[A-Za-z_][A-Za-z0-9_]*(?!')", line[index:])
                    if lifetime is None:
                        return None
                    index += len(lifetime.group(0))
                    continue
                index = end
                continue
            if line[index] == 0x22:
                state, escaped = "string", False
            index += 1
        if state == "line":
            state = "code"
        elif state == "string" and escaped:
            escaped = False
    return ordinary if state == "code" else None


def rust_character_literal_end(line: bytes, quote: int, *, byte: bool) -> int | None:
    """Return the byte after a valid Rust character literal, else ``None``.

    Character literals cannot span physical lines. Recognizing their complete
    token here prevents embedded double quotes from corrupting string/raw-string
    state; rejecting malformed or uncertain forms keeps the proof fail closed.
    """
    index = quote + 1
    if index >= len(line):
        return None
    if line[index] == 0x5C:
        index += 1
        if index >= len(line):
            return None
        escape = line[index]
        if escape in b"nrt\\0'\"":
            index += 1
        elif escape == ord("x"):
            digits = line[index + 1:index + 3]
            if len(digits) != 2 or re.fullmatch(br"[0-9A-Fa-f]{2}", digits) is None:
                return None
            if not byte and int(digits, 16) > 0x7F:
                return None
            index += 3
        elif escape == ord("u") and not byte:
            # Rust requires the first code-point digit after ``{`` to be
            # hexadecimal; separators may only follow that first digit.
            match = re.match(br"u\{([0-9A-Fa-f][0-9A-Fa-f_]*)\}", line[index:])
            if match is None:
                return None
            try:
                digits = match.group(1).replace(b"_", b"")
                if not 1 <= len(digits) <= 6:
                    return None
                value = int(digits, 16)
                if value > 0x10FFFF or 0xD800 <= value <= 0xDFFF:
                    return None
            except ValueError:
                return None
            index += len(match.group(0))
        else:
            return None
    else:
        width = 1
        if line[index] >= 0x80:
            if byte:
                return None
            for candidate in range(2, 5):
                try:
                    decoded = line[index:index + candidate].decode("utf-8")
                except UnicodeDecodeError:
                    continue
                if len(decoded) == 1:
                    width = candidate
                    break
            else:
                return None
        elif line[index] in b"'\\\t\r\n":
            return None
        index += width
    return index + 1 if index < len(line) and line[index] == 0x27 else None


def document_consumers_safe(metadata: dict) -> bool:
    try:
        roots, _ = graph(metadata)
        if re.fullmatch(r"[0-9a-f]{40}", AUDITED_DOC_CONSUMER_BASE_SHA) is None:
            return False
        probe = subprocess.run(
            ["git", "cat-file", "-e", f"{AUDITED_DOC_CONSUMER_BASE_SHA}^{{commit}}"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False,
        )
        if probe.returncode != 0:
            fetched = subprocess.run(
                ["git", "fetch", "--no-tags", "--depth=1", "origin", AUDITED_DOC_CONSUMER_BASE_SHA],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False,
            )
            if fetched.returncode != 0:
                return False
            subprocess.check_call(
                ["git", "cat-file", "-e", f"{AUDITED_DOC_CONSUMER_BASE_SHA}^{{commit}}"],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            )
        current = subprocess.check_output(["git", "rev-parse", "HEAD"]).decode().strip()
        if re.fullmatch(r"[0-9a-f]{40}", current) is None:
            return False
        pathspecs = sorted(BUILD_INPUTS) + [".cargo"] + sorted(roots.values())
        added_deleted_or_typed = subprocess.check_output(
            ["git", "diff", "--no-ext-diff", "--no-textconv", "--no-renames",
             "--diff-filter=ADT", "--name-only", "-z",
             AUDITED_DOC_CONSUMER_BASE_SHA, current, "--", *pathspecs]
        )
        if added_deleted_or_typed:
            return False
        changed = subprocess.check_output(
            ["git", "diff", "--no-ext-diff", "--no-textconv", "--no-renames",
             "--diff-filter=M", "--name-only", "-z",
             AUDITED_DOC_CONSUMER_BASE_SHA, current, "--", *pathspecs]
        )
        for raw_path in (item for item in changed.split(b"\0") if item):
            path = raw_path.decode("utf-8")
            if not valid_path(path):
                return False
            baseline = subprocess.check_output(["git", "show", f"{AUDITED_DOC_CONSUMER_BASE_SHA}:{path}"])
            content = subprocess.check_output(["git", "show", f"{current}:{path}"])
            if not document_consumer_content_safe(path, baseline, content):
                return False
        return True
    except (OSError, UnicodeError, ValueError, KeyError, TypeError, AttributeError, subprocess.SubprocessError):
        return False


def candidate_modes_safe(sha: str) -> bool:
    if re.fullmatch(r"[0-9a-f]{40}", sha) is None:
        return False
    rows = subprocess.check_output(["git", "ls-tree", "-r", "-z", sha]).split(b"\0")
    entries = [row for row in rows if row]
    return bool(entries) and all(row.split(b"\t", 1)[0].split()[0] in {b"100644", b"100755"} for row in entries)


def git_diff_records(before: str, after: str) -> list[dict]:
    """Enumerate the complete immutable tree delta for two exact commits."""
    if any(re.fullmatch(r"[0-9a-f]{40}", sha or "") is None for sha in (before, after)) or before == after:
        raise ValueError("invalid-or-empty-git-range")
    raw = subprocess.check_output(
        ["git", "diff", "--no-ext-diff", "--no-textconv", "--find-renames",
         "--name-status", "-z", before, after, "--"]
    )
    if not raw or not raw.endswith(b"\0"):
        raise ValueError("empty-or-incomplete-git-diff")
    fields = raw[:-1].split(b"\0")
    files = []
    index = 0
    while index < len(fields):
        code = fields[index]
        index += 1
        if code in {b"A", b"M", b"D"}:
            if index >= len(fields):
                raise ValueError("malformed-git-diff")
            status = {b"A": "added", b"M": "modified", b"D": "removed"}[code]
            files.append({"filename": fields[index].decode("utf-8"), "status": status})
            index += 1
            continue
        if code[:1] in {b"R", b"C"} and code[1:].isdigit():
            if index + 1 >= len(fields):
                raise ValueError("malformed-git-diff")
            previous = fields[index].decode("utf-8")
            current = fields[index + 1].decode("utf-8")
            files.append({
                "filename": current,
                "status": "renamed" if code.startswith(b"R") else "copied",
                "previous_filename": previous,
            })
            index += 2
            continue
        raise ValueError("unsupported-git-diff-status")
    return files


def pr_file_records() -> tuple[list[dict], int, bool]:
    """Use transported records when complete, otherwise recover from exact Git trees."""
    completeness = os.environ["ENUMERATION_COMPLETE"]
    if completeness == "true":
        files = json.loads(os.environ["CHANGED_FILE_RECORDS"])
        return files, int(os.environ["CHANGED_FILE_COUNT"]), True
    if completeness != "false":
        raise ValueError("invalid-enumeration-state")
    count_text = os.environ["CHANGED_FILE_COUNT"]
    if re.fullmatch(r"[1-9][0-9]*", count_text) is None:
        raise ValueError("invalid-transported-file-count")
    expected_head = os.environ["EXPECTED_HEAD"].strip().lower()
    base = subprocess.check_output(["git", "rev-parse", "HEAD"]).decode().strip().lower()
    files = git_diff_records(base, expected_head)
    return files, len(files), True


def classify(files, changed_count, metadata, digest, complete=True, docs_digest=None,
             candidate_modes_verified=False, docs_consumers_verified=None) -> dict:
    try:
        if candidate_modes_verified is not True:
            return full("unverified-or-special-candidate-modes")
        if complete is not True or type(changed_count) is not int or not isinstance(files, list) or len(files) != changed_count or not files:
            return full("incomplete-enumeration")
        paths, filenames = [], set()
        added_surfaces, removed_surfaces = set(), set()
        for item in files:
            path = item["filename"]
            status = item.get("status")
            previous = item.get("previous_filename")
            if not valid_path(path) or path in filenames or status not in {"added", "modified", "removed", "renamed", "copied", "changed", "unchanged"}:
                return full("invalid-file-record")
            filenames.add(path)
            paths.append(path)
            if status == "added":
                added_surfaces.add(non_runtime(path))
            elif status == "removed":
                removed_surfaces.add(non_runtime(path))
            if status == "renamed" and not previous:
                return full("missing-rename-source")
            if previous is not None:
                if not valid_path(previous):
                    return full("invalid-rename-source")
                if non_runtime(path) != non_runtime(previous):
                    return full("cross-surface-rename")
                paths.append(previous)
        if added_surfaces and removed_surfaces and any(
            added != removed for added in added_surfaces for removed in removed_surfaces
        ):
            return full("possible-cross-surface-rename")
        if all(agent_task_record(path) for path in paths):
            return dict(
                rust=False,
                windows=False,
                surface="agent-governance",
                reason="agent-task-record-only",
            )
        non_runtime_present = any(non_runtime(path) for path in paths)
        governance_present = any(agent_governance(path) for path in paths)
        if non_runtime_present:
            graph(metadata)
            proof_surface = "agent-governance" if governance_present else "docs"
            if docs_consumers_verified is False:
                return full("unreviewed-document-consumer-inputs", proof_surface)
            if docs_consumers_verified is not True and (digest != AUDITED_INPUT_SHA256 or docs_digest != AUDITED_DOC_INPUT_SHA256):
                return full("unreviewed-document-consumer-inputs", proof_surface)
        if all(non_runtime(path) for path in paths):
            docs_present = any(neutral(path) and not agent_governance(path) for path in paths)
            if governance_present:
                reason = "agent-governance-plus-neutral-documentation" if docs_present else "agent-governance-only"
                return dict(rust=False, windows=False, surface="agent-governance", reason=reason)
            return dict(rust=False, windows=False, surface="docs", reason="neutral-documentation")
        material_paths = [path for path in paths if not agent_governance(path)]
        if any(path.startswith(".cargo/") or PurePosixPath(path).name in BUILD_INPUTS | {"build.rs"} for path in material_paths):
            return full("explicit-build-or-dependency-input", "dependencies-build")
        if any(path.startswith((".github/", "tools/repository/", "docs/migration/")) for path in material_paths):
            return full("explicit-build-or-control-input", "control-plane")
        roots, reverse = graph(metadata)
        affected = set()
        atlas_fullworld = False
        for path in material_paths:
            if neutral(path):
                continue
            disposition = atlas_path_disposition(path)
            if disposition == "atlas-fullworld":
                atlas_fullworld = True
                continue
            if disposition == "full":
                return full("explicit-atlas-non-cargo-full", "atlas")
            owners = [name for name, root in roots.items() if path.startswith(root + "/")]
            if len(owners) != 1 or PurePosixPath(path).suffix not in {".rs", ".sql"}:
                return full("unmodelled-input")
            affected.add(owners[0])
        pending = list(affected)
        while pending:
            for consumer in reverse[pending.pop()] - affected:
                affected.add(consumer)
                pending.append(consumer)
        if not affected:
            if atlas_fullworld:
                return dict(rust=False, windows=False, surface="atlas-fullworld", reason="audited-atlas-fullworld-source")
            return full("mixed-or-unowned-surface")
        if affected & WINDOWS:
            surface = "simulation" if "oteryn-simulation-determinism" in affected else "shared" if SERVER in affected else "client"
            return full("windows-consumer-affected", surface)
        if SERVER not in affected or affected != {SERVER}:
            return full("mixed-or-unowned-surface")
        if digest != AUDITED_INPUT_SHA256:
            return full("unreviewed-consumer-input-snapshot")
        surface = "durability" if any(any(token in path for token in ("/durability/", "/migrations/", "postgres", "reconnect")) for path in material_paths) else "server"
        reason = "server-only-reverse-closure-and-audited-inputs"
        if atlas_fullworld:
            reason += "-plus-atlas-fullworld"
        return dict(rust=True, windows=False, surface=surface, reason=reason)
    except (KeyError, TypeError, ValueError, AttributeError):
        return full("classifier-input-failure")


def classify_post_merge(event, metadata) -> dict:
    """Reuse PR risk semantics only for a verified, complete protected-main push."""
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
        files = git_diff_records(before, after)
        result = classify(files, len(files), metadata, input_digest(metadata),
                          docs_digest=input_digest(metadata, include_server=True),
                          candidate_modes_verified=candidate_modes_safe(after),
                          docs_consumers_verified=document_consumers_safe(metadata))
        if result["rust"] is False and result["windows"] is False and result["surface"] in {"docs", "agent-governance"}:
            return result
        if result["rust"] is True and result["windows"] is False and result["surface"] in {"server", "durability"}:
            return result
        return full(result["reason"], result["surface"])
    except (OSError, ValueError, KeyError, IndexError, TypeError, AttributeError, subprocess.SubprocessError):
        return full("post-merge-input-or-git-failure")


def main() -> int:
    post_merge = False
    atlas_fullworld = True
    try:
        post_merge = sys.argv[1] == "--post-merge"
        metadata = json.loads(Path(sys.argv[2] if post_merge else sys.argv[1]).read_text(encoding="utf-8"))
        if post_merge:
            result = classify_post_merge(json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8")), metadata)
        else:
            files, changed_count, complete = pr_file_records()
            atlas_fullworld = atlas_fullworld_required(files, changed_count, complete)
            digest = input_digest(metadata)
            non_runtime_candidate = (
                isinstance(files, list) and bool(files)
                and any(
                    isinstance(item, dict)
                    and (
                        (isinstance(item.get("filename"), str) and non_runtime(item["filename"]))
                        or
                        (isinstance(item.get("previous_filename"), str) and non_runtime(item["previous_filename"]))
                    )
                    for item in files
                )
            )
            result = classify(files, changed_count, metadata, digest,
                              complete=complete,
                              docs_digest=input_digest(metadata, include_server=True),
                              candidate_modes_verified=candidate_modes_safe(os.environ["EXPECTED_HEAD"]),
                              docs_consumers_verified=document_consumers_safe(metadata) if non_runtime_candidate else None)
    except (OSError, ValueError, KeyError, IndexError, TypeError, AttributeError, subprocess.SubprocessError):
        result = full("classifier-or-metadata-failure")
    health = routing_health(result)
    print(json.dumps(result | {"routing_health": health}, sort_keys=True))
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"rust={str(result['rust']).lower()}\nwindows={str(result['windows']).lower()}\n")
        if not post_merge:
            output.write(f"atlas_fullworld={str(atlas_fullworld).lower()}\n")
            output.write(f"surface={result['surface']}\n")
            output.write(f"reason={result['reason']}\n")
            output.write(f"routing_health={health}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
