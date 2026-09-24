#!/usr/bin/env python3
"""Select product CI lanes from the exact candidate tree.

Cargo metadata owns package dependency closure. Files outside Cargo packages are
auxiliary unless the exact candidate's product/build sources reference them.
Canonical routing/build controls remain conservative FULL inputs. This avoids
historical snapshot repins while keeping real non-Cargo product inputs attached
to their consuming packages.
"""
from __future__ import annotations

import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]

SERVER = "oteryn-game-server"
WINDOWS = {"oteryn-client", "oteryn-synthetic-client-harness", "oteryn-simulation-determinism"}
CONTROL_CONSUMER = "__canonical_routing_control__"
REQUIRED = {
    SERVER: "apps/game-server",
    "oteryn-client": "apps/client",
    "oteryn-synthetic-client-harness": "tools/synthetic-client-harness",
    "oteryn-simulation-determinism": "crates/simulation-determinism",
}
BUILD_INPUTS = {
    "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rustfmt.toml",
    "deny.toml", "workspace-boundaries.toml", ".gitattributes", ".gitmodules",
}
CANONICAL_CONTROL_PATHS = {
    ".github/workflows/merge-gate.yml",
    ".github/workflows/merge-group-gate.yml",
    ".github/workflows/rust.yml",
}
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
    "unverified-or-special-candidate-modes",
    "unverified-reference-consumers",
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
    return (
        isinstance(value, str)
        and bool(value)
        and not any(c in value for c in "\x00\n\r\\")
        and not value.startswith("/")
        and ".." not in value.split("/")
        and str(PurePosixPath(value)) == value
    )


def neutral(path: str) -> bool:
    if PurePosixPath(path).name in {"AGENTS.md", "AGENTS.override.md"} or path.startswith("docs/migration/"):
        return False
    return path in {"README.md", "CHANGELOG.md", "CONTRIBUTING.md"} or (
        path.startswith("docs/") and path.endswith(".md")
    )


def agent_governance(path: str) -> bool:
    if PurePosixPath(path).name in {"AGENTS.md", "AGENTS.override.md"}:
        return True
    if path.startswith("tools/agents/"):
        return True
    return path.startswith("docs/agents/") and not path.startswith("docs/agents/evidence/")


def canonical_control(path: str) -> bool:
    return (
        path in CANONICAL_CONTROL_PATHS
        or path.startswith(".github/actions/")
        or path.startswith("tools/repository/")
        or path.startswith("docs/migration/")
    )


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



def package_owner(roots: dict[str, str], path: str) -> str | None:
    owners = [name for name, root in roots.items() if path == root or path.startswith(root + "/")]
    if len(owners) > 1:
        raise ValueError("overlapping package ownership")
    return owners[0] if owners else None


def auxiliary_path(roots: dict[str, str], path: str) -> bool:
    if package_owner(roots, path) is not None:
        return False
    if canonical_control(path):
        return False
    if path.startswith("tools/game-atlas-"):
        return False
    if (
        path.startswith(("docs/", ".github/", "tools/"))
        or PurePosixPath(path).name in {
            "AGENTS.md", "AGENTS.override.md", "README.md", "CHANGELOG.md",
            "CONTRIBUTING.md", "SECURITY.md", "LICENSE", "LICENSE-ASSETS.md",
            "TRADEMARKS.md", ".editorconfig", ".gitignore",
        }
    ):
        return True
    return False


def routing_surface(roots: dict[str, str], path: str) -> str:
    if canonical_control(path) or path.startswith(".cargo/") or PurePosixPath(path).name in BUILD_INPUTS | {"build.rs"}:
        return "control"
    disposition = atlas_path_disposition(path)
    if disposition is not None:
        return "atlas"
    if package_owner(roots, path) is not None:
        return "product"
    if auxiliary_path(roots, path):
        return "auxiliary"
    return "unknown"


def file_reference_patterns(path: str) -> tuple[str, ...]:
    """Return conservative literals for an exact repository-file consumer."""
    parts = PurePosixPath(path).parts
    patterns = {path, parts[-1]}
    if len(parts) >= 2:
        patterns.add("/".join(parts[-2:]))
    return tuple(sorted(patterns, key=lambda value: (-len(value), value)))


def directory_reference_patterns(path: str) -> tuple[str, ...]:
    """Return non-top-level ancestor paths that may be consumed as directories."""
    parts = PurePosixPath(path).parts[:-1]
    patterns = {
        "/".join(parts[:length])
        for length in range(2, len(parts) + 1)
    }
    return tuple(sorted(patterns, key=lambda value: (-len(value), value)))


def directory_reference_occurrences(content: bytes, pattern: str) -> list[int]:
    """Return bounded directory-literal occurrences, excluding sibling-name prefixes."""
    needle = pattern.encode("utf-8")
    start = 0
    matches: list[int] = []
    while True:
        index = content.find(needle, start)
        if index < 0:
            return matches
        end = index + len(needle)
        tail = content[end:]
        bounded = (
            not tail
            or tail[:1] in {b'"', b"'", b" ", b"\t", b"\r", b"\n"}
            or (
                tail[:1] == b"/"
                and (
                    len(tail) == 1
                    or tail[1:2] in {b'"', b"'", b" ", b"\t", b"\r", b"\n"}
                )
            )
        )
        if bounded:
            matches.append(index)
        start = index + 1


def standalone_directory_reference(content: bytes, pattern: str) -> bool:
    """Require a bounded directory literal, not a prefix of a sibling path/name."""
    return bool(directory_reference_occurrences(content, pattern))


def workflow_directory_reference_is_routing_only(content: bytes, pattern: str) -> bool:
    """Return true only when every bounded directory occurrence is a routing predicate."""
    occurrences = directory_reference_occurrences(content, pattern)
    if not occurrences:
        return False
    for index in occurrences:
        prefix = content[max(0, index - 128):index]
        python_startswith = re.search(rb"\.startswith\(\s*['\"]$", prefix) is not None
        github_startswith = (
            re.search(rb"\bstartsWith\([^,\n]+,\s*['\"]$", prefix) is not None
        )
        if not (python_startswith or github_startswith):
            return False
    return True


def consumer_pathspecs(roots: dict[str, str]) -> list[str]:
    """Scan Cargo packages plus canonical workflows that select product CI."""
    return sorted(set(roots.values())) + sorted(CANONICAL_CONTROL_PATHS)


def candidate_reference_consumers(
    metadata: dict,
    sha: str,
    paths: list[str],
) -> dict[str, set[str]]:
    """Map changed non-Cargo files to exact-candidate product/control consumers.

    Candidate content is read as data only. Matching covers exact files plus
    bounded, standalone parent-directory literals across Cargo packages and canonical product
    CI workflows. False positives only allocate broader lanes; malformed or
    unavailable evidence fails closed.
    """
    if re.fullmatch(r"[0-9a-f]{40}", sha or "") is None:
        raise ValueError("invalid candidate SHA")
    roots, _ = graph(metadata)
    unique = sorted(set(paths))
    if not unique or any(not valid_path(path) for path in unique):
        raise ValueError("invalid candidate reference path")
    reverse_files: dict[str, set[str]] = {}
    reverse_directories: dict[str, set[str]] = {}
    for path in unique:
        for value in file_reference_patterns(path):
            reverse_files.setdefault(value, set()).add(path)
        for value in directory_reference_patterns(path):
            reverse_directories.setdefault(value, set()).add(path)

    command = ["git", "grep", "-l", "-z", "-F"]
    for pattern in sorted(set(reverse_files) | set(reverse_directories)):
        command.extend(["-e", pattern])
    command.extend([sha, "--", *consumer_pathspecs(roots)])
    result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if result.returncode not in {0, 1}:
        raise ValueError("candidate reference scan failed")
    raw_matches = [item for item in result.stdout.split(b"\0") if item]
    consumers: dict[str, set[str]] = {path: set() for path in unique}
    prefix = f"{sha}:"
    for raw in raw_matches:
        rendered = raw.decode("utf-8")
        if not rendered.startswith(prefix):
            raise ValueError("candidate reference scan returned an unbound path")
        consumer_path = rendered[len(prefix):]
        if not valid_path(consumer_path):
            raise ValueError("invalid consumer path")
        content = subprocess.check_output(["git", "show", f"{sha}:{consumer_path}"])
        owner = package_owner(roots, consumer_path)
        control = consumer_path in CANONICAL_CONTROL_PATHS
        if owner is None and not control:
            continue
        selected: set[str] = set()
        for pattern, targets in reverse_files.items():
            if pattern.encode("utf-8") in content:
                selected.update(targets)
        # Directory references stay conservative for both package and workflow
        # consumers. The only exception is a syntactically bounded path-membership
        # predicate (for example path.startswith('docs/architecture/')), which
        # selects CI routing but does not consume every file in that directory.
        for pattern, targets in reverse_directories.items():
            if not standalone_directory_reference(content, pattern):
                continue
            if (
                control
                and workflow_directory_reference_is_routing_only(content, pattern)
            ):
                continue
            selected.update(targets)
        for target in selected:
            consumers[target].add(owner if owner is not None else CONTROL_CONSUMER)
    return consumers


def expand_reverse_closure(affected: set[str], reverse: dict[str, set[str]]) -> set[str]:
    pending = list(affected)
    while pending:
        for consumer in reverse[pending.pop()] - affected:
            affected.add(consumer)
            pending.append(consumer)
    return affected


def auxiliary_result(paths: list[str]) -> dict:
    governance = any(agent_governance(path) for path in paths)
    docs = any(neutral(path) for path in paths)
    workflows = any(path.startswith(".github/") for path in paths)
    if governance and not docs and not workflows:
        surface = "agent-governance"
    elif docs and not governance and not workflows:
        surface = "docs"
    else:
        surface = "auxiliary"
    return dict(rust=False, windows=False, surface=surface, reason="unconsumed-auxiliary-inputs")


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



def classify(
    files,
    changed_count,
    metadata,
    complete=True,
    *,
    candidate_modes_verified=False,
    candidate_sha=None,
    reference_consumers=None,
) -> dict:
    try:
        if candidate_modes_verified is not True:
            return full("unverified-or-special-candidate-modes")
        if (
            complete is not True
            or type(changed_count) is not int
            or not isinstance(files, list)
            or len(files) != changed_count
            or not files
        ):
            return full("incomplete-enumeration")

        roots, reverse = graph(metadata)
        records: list[tuple[str, str, str | None]] = []
        paths: list[str] = []
        filenames: set[str] = set()
        added_surfaces: set[str] = set()
        removed_surfaces: set[str] = set()

        for item in files:
            if not isinstance(item, dict):
                return full("invalid-file-record")
            path = item.get("filename")
            status = item.get("status")
            previous = item.get("previous_filename")
            if (
                not valid_path(path)
                or path in filenames
                or status not in {
                    "added", "modified", "removed", "renamed",
                    "copied", "changed", "unchanged",
                }
            ):
                return full("invalid-file-record")
            filenames.add(path)
            if status == "renamed" and not previous:
                return full("missing-rename-source")
            if previous is not None and not valid_path(previous):
                return full("invalid-rename-source")

            current_surface = routing_surface(roots, path)
            if status == "added":
                added_surfaces.add(current_surface)
            elif status == "removed":
                removed_surfaces.add(current_surface)
            if previous is not None:
                previous_surface = routing_surface(roots, previous)
                if current_surface != previous_surface:
                    return full("cross-surface-rename")
                paths.append(previous)
            paths.append(path)
            records.append((path, status, previous))

        if added_surfaces and removed_surfaces and any(
            added != removed for added in added_surfaces for removed in removed_surfaces
        ):
            return full("possible-cross-surface-rename")

        for path in paths:
            if path.startswith(".cargo/") or PurePosixPath(path).name in BUILD_INPUTS | {"build.rs"}:
                return full("explicit-build-or-dependency-input", "dependencies-build")
            if canonical_control(path):
                return full("explicit-build-or-control-input", "control-plane")
            if atlas_path_disposition(path) == "full":
                return full("explicit-atlas-non-cargo-full", "atlas")

        scan_paths = sorted(set(paths))
        if reference_consumers is None:
            reference_consumers = {path: set() for path in scan_paths}
            auxiliary_scan_paths = [
                path
                for path in scan_paths
                if package_owner(roots, path) is None
                and atlas_path_disposition(path) is None
            ]
            if auxiliary_scan_paths:
                if re.fullmatch(r"[0-9a-f]{40}", candidate_sha or "") is None:
                    return full("unverified-reference-consumers")
                reference_consumers.update(
                    candidate_reference_consumers(metadata, candidate_sha, auxiliary_scan_paths)
                )
        if (
            not isinstance(reference_consumers, dict)
            or any(path not in reference_consumers for path in scan_paths)
        ):
            return full("unverified-reference-consumers")

        affected: set[str] = set()
        auxiliary: list[str] = []
        unknown: list[str] = []
        atlas_fullworld = False

        for path in scan_paths:
            disposition = atlas_path_disposition(path)
            if disposition == "atlas-fullworld":
                atlas_fullworld = True
                continue

            owner = package_owner(roots, path)
            if owner is not None:
                affected.add(owner)
            elif auxiliary_path(roots, path):
                auxiliary.append(path)
            else:
                unknown.append(path)

            consumers = reference_consumers.get(path)
            if not isinstance(consumers, (set, list, tuple)):
                return full("unverified-reference-consumers")
            for consumer in consumers:
                if consumer == CONTROL_CONSUMER:
                    return full("canonical-control-consumer-affected", "control-plane")
                if consumer not in roots:
                    return full("unverified-reference-consumers")
                affected.add(consumer)

        unresolved = [
            path for path in unknown
            if not reference_consumers.get(path)
        ]
        if unresolved:
            return full("unmodelled-input")

        expand_reverse_closure(affected, reverse)

        if affected & WINDOWS:
            surface = (
                "simulation" if "oteryn-simulation-determinism" in affected
                else "shared" if SERVER in affected
                else "client"
            )
            return full("windows-consumer-affected", surface)

        if affected:
            if affected != {SERVER}:
                return full("mixed-or-unowned-surface")
            material = scan_paths
            surface = "durability" if any(
                any(token in path for token in ("/durability/", "/migrations/", "postgres", "reconnect"))
                for path in material
            ) else "server"
            reason = "server-only-exact-consumer-closure"
            if atlas_fullworld:
                reason += "-plus-atlas-fullworld"
            return dict(rust=True, windows=False, surface=surface, reason=reason)

        if atlas_fullworld:
            return dict(
                rust=False,
                windows=False,
                surface="atlas-fullworld",
                reason="audited-atlas-fullworld-source",
            )

        if auxiliary and len(auxiliary) == len(scan_paths):
            return auxiliary_result(auxiliary)

        return full("mixed-or-unowned-surface")
    except (
        OSError,
        KeyError,
        TypeError,
        ValueError,
        AttributeError,
        subprocess.SubprocessError,
        UnicodeError,
    ):
        return full("classifier-input-failure")


def classify_post_merge(event, metadata) -> dict:
    """Apply the same exact-candidate routing semantics to protected-main pushes."""
    try:
        if (
            os.environ.get("GITHUB_EVENT_NAME") != "push"
            or os.environ.get("GITHUB_REF") != "refs/heads/main"
            or os.environ.get("GITHUB_REF_PROTECTED") != "true"
            or os.environ.get("GITHUB_REPOSITORY") != "Oteryn/Oteryn-Game"
            or event["repository"]["full_name"] != "Oteryn/Oteryn-Game"
            or event["ref"] != "refs/heads/main"
            or any(event[key] is not False for key in ("forced", "created", "deleted"))
        ):
            return full("not-a-normal-protected-main-push")
        before, after = event["before"], event["after"]
        if any(
            not isinstance(sha, str)
            or re.fullmatch(r"[0-9a-f]{40}", sha) is None
            or sha == "0" * 40
            for sha in (before, after)
        ):
            return full("invalid-push-range")
        if before == after or after != os.environ.get("GITHUB_SHA"):
            return full("push-identity-mismatch")
        actual = subprocess.check_output(["git", "rev-parse", "HEAD"]).decode().strip()
        if (
            actual != after
            or subprocess.check_output(["git", "rev-parse", "--is-shallow-repository"]).strip() != b"false"
        ):
            return full("unverified-or-incomplete-protected-checkout")
        subprocess.check_output(["git", "merge-base", "--is-ancestor", before, after], stderr=subprocess.PIPE)
        files = git_diff_records(before, after)
        return classify(
            files,
            len(files),
            metadata,
            candidate_modes_verified=candidate_modes_safe(after),
            candidate_sha=after,
        )
    except (
        OSError,
        ValueError,
        KeyError,
        IndexError,
        TypeError,
        AttributeError,
        subprocess.SubprocessError,
    ):
        return full("post-merge-input-or-git-failure")


def main() -> int:
    post_merge = False
    atlas_fullworld = True
    try:
        post_merge = sys.argv[1] == "--post-merge"
        metadata = json.loads(
            Path(sys.argv[2] if post_merge else sys.argv[1]).read_text(encoding="utf-8")
        )
        if post_merge:
            result = classify_post_merge(
                json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8")),
                metadata,
            )
        else:
            files, changed_count, complete = pr_file_records()
            atlas_fullworld = atlas_fullworld_required(files, changed_count, complete)
            expected_head = os.environ["EXPECTED_HEAD"].strip().lower()
            result = classify(
                files,
                changed_count,
                metadata,
                complete=complete,
                candidate_modes_verified=candidate_modes_safe(expected_head),
                candidate_sha=expected_head,
            )
    except (
        OSError,
        ValueError,
        KeyError,
        IndexError,
        TypeError,
        AttributeError,
        subprocess.SubprocessError,
    ):
        result = full("classifier-or-metadata-failure")

    health = routing_health(result)
    print(json.dumps(result | {"routing_health": health}, sort_keys=True))
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"rust={str(result['rust']).lower()}\n")
        output.write(f"windows={str(result['windows']).lower()}\n")
        if not post_merge:
            output.write(f"atlas_fullworld={str(atlas_fullworld).lower()}\n")
            output.write(f"surface={result['surface']}\n")
            output.write(f"reason={result['reason']}\n")
            output.write(f"routing_health={health}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
