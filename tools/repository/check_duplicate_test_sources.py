#!/usr/bin/env python3
"""Fail when one test-bearing source file is compiled into more than one test harness.

A `#[path = "../src/..."]` copy or a shared `tests/support/*` file compiled into several test
targets makes every test it defines run once per target in one `cargo test` run (and makes Cargo
compile it once per target). This guard reads the Cargo JSON messages of a workspace build or
Clippy run (`--message-format=json`), follows each workspace test-harness unit to its rustc
dep-info file and fails when a source file that defines tests is compiled into more than one
test-harness unit that is not on the explicit allowlist below.

The allowlist names the test targets that may still path-include test-bearing sources of another
target, each with its reason. It may only shrink: removing an entry once the duplication is gone
keeps the guard strict, and an entry that no longer matches any duplicated source is reported as
stale so the list cannot outlive its reason. The check fails closed on missing or unreadable
evidence.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

# (package, test target) -> (source path prefixes it may duplicate, reason). Only shrinks.
GAME_SERVER_SRC = "apps/game-server/src/"
CARVED_GAMEPLAY_SRC = (
    "crates/gameplay/combat/src/",
    "crates/gameplay/content/src/",
    "crates/gameplay/domain/src/",
    "crates/gameplay/durability/src/",
    "crates/gameplay/encounters/src/",
    "crates/gameplay/foundation/src/",
    "crates/gameplay/monster-ai/src/",
    "crates/gameplay/social/src/",
    "crates/gameplay/spell/src/",
    "crates/gameplay/world/src/",
)
COPY_REASON = (
    "path-includes crate-private game-server modules (including the gameplay modules carved out "
    "of apps/game-server/src into crates/gameplay) whose unit tests also compile here until those "
    "modules gate their test modules to their library target"
)
ALLOWED_DUPLICATING_TARGETS: dict[tuple[str, str], tuple[tuple[str, ...], str]] = {
    **{
        ("oteryn-game-server", target): (prefixes, COPY_REASON)
        for target, prefixes in (
            ("ability_engine", (GAME_SERVER_SRC, *CARVED_GAMEPLAY_SRC)),
            ("ai_bootstrap", ("crates/gameplay/monster-ai/src/",)),
            ("character_authority_postgres", (GAME_SERVER_SRC, *CARVED_GAMEPLAY_SRC)),
            ("durability_postgres", CARVED_GAMEPLAY_SRC),
            ("native_admission_source_postgres", CARVED_GAMEPLAY_SRC),
            ("runtime_scope_assignment_postgres", CARVED_GAMEPLAY_SRC),
            ("wp5_s3b_composition", CARVED_GAMEPLAY_SRC),
        )
    },
    ("oteryn-game-server", "oteryn_game_server"): (
        ("apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs",),
        "the library's monk-save PostgreSQL tests reuse this case file as their fixture; "
        "durability_postgres runs its cases against PostgreSQL",
    ),
}

TEST_ATTRIBUTE = re.compile(r"#\[\s*(?:[A-Za-z_][A-Za-z0-9_]*::)*test\s*[\](]")


class EvidenceError(Exception):
    pass


def dep_info_path(filename: str) -> Path:
    path = Path(filename)
    name = path.name
    for suffix in (".rmeta", ".rlib"):
        if name.endswith(suffix):
            name = name[: -len(suffix)]
            if name.startswith("lib"):
                name = name[3:]
            break
    else:
        name = path.stem if path.suffix == ".exe" else name
    return path.with_name(name + ".d")


def dep_info_sources(path: Path) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        raise EvidenceError(f"cannot read dep-info {path}: {error}") from error
    first = text.split("\n\n", 1)[0].replace("\\\n", " ")
    _, separator, deps = first.partition(": ")
    if not separator:
        raise EvidenceError(f"malformed dep-info {path}")
    return [token.replace("\\ ", " ") for token in re.findall(r"(?:\\ |[^ \n])+", deps)]


def package_name(package_id: str, manifest: Path) -> str:
    fragment = package_id.rpartition("#")[2]
    if "@" in fragment:
        return fragment.split("@", 1)[0]
    return manifest.parent.name


def harness_units(messages: list[dict], root: Path) -> dict[tuple[str, str, str], list[str]]:
    units: dict[tuple[str, str, str], list[str]] = {}
    for message in messages:
        if message.get("reason") != "compiler-artifact" or not message.get("profile", {}).get("test"):
            continue
        manifest = Path(message.get("manifest_path", ""))
        try:
            manifest.resolve().relative_to(root)
        except ValueError:
            continue
        target = message.get("target", {})
        filenames = message.get("filenames") or []
        if not filenames or not target.get("name"):
            raise EvidenceError(f"incomplete artifact message for {manifest}")
        key = (package_name(message.get("package_id", ""), manifest), "/".join(target.get("kind", [])), target["name"])
        units[key] = dep_info_sources(dep_info_path(filenames[0]))
    return units


def find_duplicates(units: dict[tuple[str, str, str], list[str]], root: Path) -> tuple[list[str], list[tuple[str, str]]]:
    by_source: dict[Path, set[tuple[str, str, str]]] = {}
    for key, sources in units.items():
        for source in sources:
            path = Path(source)
            if not path.is_absolute():
                path = root / path
            path = path.resolve()
            if path.suffix != ".rs":
                continue
            try:
                path.relative_to(root)
            except ValueError:
                continue
            by_source.setdefault(path, set()).add(key)
    violations: list[str] = []
    allowed_used: set[tuple[str, str]] = set()
    for path in sorted(by_source):
        holders = by_source[path]
        if len(holders) < 2:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as error:
            raise EvidenceError(f"cannot read {path}: {error}") from error
        if TEST_ATTRIBUTE.search(text) is None:
            continue
        relative = path.relative_to(root).as_posix()
        exempt = {
            key
            for key in holders
            if (key[0], key[2]) in ALLOWED_DUPLICATING_TARGETS
            and relative.startswith(ALLOWED_DUPLICATING_TARGETS[(key[0], key[2])][0])
        }
        allowed_used.update((key[0], key[2]) for key in exempt)
        strict = sorted(holders - exempt)
        if len(strict) > 1:
            names = ", ".join(f"{package}:{kind}:{target}" for package, kind, target in strict)
            violations.append(
                f"{relative} defines tests compiled into {len(strict)} test targets: {names}; "
                "include it from one target (mark a thin wrapper `test = false`) or gate its tests"
            )
    stale = sorted(set(ALLOWED_DUPLICATING_TARGETS) - allowed_used)
    return violations, stale


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: check_duplicate_test_sources.py <cargo-json-messages>", file=sys.stderr)
        return 2
    root = Path.cwd().resolve()
    try:
        messages = []
        for line in Path(argv[1]).read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line.startswith("{"):
                messages.append(json.loads(line))
        units = harness_units(messages, root)
        if not units:
            raise EvidenceError("no workspace test-harness units in the Cargo messages")
        violations, stale = find_duplicates(units, root)
    except (EvidenceError, OSError, ValueError) as error:
        print(f"duplicate test source guard: FAIL (evidence): {error}", file=sys.stderr)
        return 1
    for entry in stale:
        violations.append(f"allowlist entry {entry[0]}:{entry[1]} no longer duplicates a test-bearing source; remove it")
    if violations:
        print("duplicate test source guard: FAIL", file=sys.stderr)
        for violation in violations:
            print(f"  {violation}", file=sys.stderr)
        return 1
    print(f"duplicate test source guard: PASS ({len(units)} test-harness units, {len(ALLOWED_DUPLICATING_TARGETS)} allowlisted targets)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
