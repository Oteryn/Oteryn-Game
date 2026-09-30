#!/usr/bin/env python3
"""Regenerate every derived content file and resolve merge conflicts in them.

After `git merge origin/main` stops on conflicts, run:

    python3 tools/content-migration/regenerate_content.py --resolve

Every file below is derived from committed inputs, so a conflict in it is never a decision: take
either side, regenerate, and the result is exact. A conflict in any other path stops the script,
because that one needs a person. So does a conflict in a file the steps rewrite but that also
holds hand-maintained rows (`imports/**`, `content/interactions/index.json`): taking one side
would drop the other side's rows.

Steps: the legacy WorldProject package (content/world, from the materializer), the Rust
inventory pins in content_world_project_repository.rs, the content tree (world_project_v2_to_tree),
the Charm, Proficiency and RewardClaim registrations, then the validators. The script never
commits; review `git status` and commit the merge yourself.

Scope: the command regenerates only the content tree, `content/world` and the Rust package pins.
It does not regenerate other Item-derived artifacts (the weapon proficiency sample, the TibiaWiki
snapshot re-key and promotion packet, the pins in `item_stats_promotion.rs`, the Crystal binding
output); where a `--check` exists it runs it, and a failure there needs a manual regeneration.
Hand-written count pins (for example `PROFICIENCY_BINDING_COUNT`) can go stale after a clean merge;
the checks catch that, and the pin is then edited by hand.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORLD = ROOT / "content/world"
PIN_PATH = "apps/game-server/tests/content_world_project_repository.rs"
PIN_TEST = ROOT / PIN_PATH

REGISTRY = (
    "content/project.json",
    "content/manifest.json",
    "content/content.lock.json",
)
# Written by the authoring tools but not listed in the content manifest.
EXTRA_DERIVED = ("content/interactions/index.json",)
# Rewritten by the steps below but holding hand-maintained rows: the TibiaWiki import-only
# sources and batches (read back by world_project_v2_to_tree), hand-captured import records, and
# the hand-written fields of the interactions index. A conflict here needs a person.
HAND_MAINTAINED_PREFIXES = ("imports/",)
HAND_MAINTAINED = ("content/interactions/index.json",)

AUTHORING_TOOLS = (
    "tools/content-schema/charm-authoring/charm_authoring.py",
    "tools/content-schema/proficiency-authoring/proficiency_authoring.py",
    "tools/content-schema/reward-claim-authoring/reward_claim_authoring.py",
)

CHECKS = (
    ["python3", "tools/content-migration/validate_world_project_v2_to_tree.py"],
    ["python3", "tools/content-migration/test_world_project_v2_to_tree.py"],
    ["python3", "tools/content-schema/validate_materialized_game_tree.py"],
    ["python3", "tools/content-census/item_key_references.py"],
    *(["python3", tool, "content", "--check"] for tool in AUTHORING_TOOLS),
    ["python3", "tools/content-schema/item-authoring/test_engine_items.py"],
    ["python3", "tools/content-schema/item-authoring/item_weapon_proficiency.py", "--check"],
    ["python3", "tools/content-census/g4_item_crystal_binding_generator.py", "--check"],
    [
        "cargo",
        "test",
        "--locked",
        "-q",
        "-p",
        "oteryn-game-server",
        "--test",
        "content_world_project_repository",
    ],
)


def run(command: list[str]) -> None:
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=ROOT, check=True)


def git(*args: str) -> str:
    result = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True)
    return result.stdout if result.returncode == 0 else ""


def derived_paths() -> set[str]:
    """Every path the steps below write: the managed files of both merge sides, and the rest."""
    managed = set()
    for stage in (":2:", ":3:", "HEAD:", ""):
        spec = f"{stage}content/manifest.json"
        text = (
            git("show", spec)
            if stage
            else (ROOT / "content/manifest.json").read_text(encoding="utf-8")
        )
        try:
            managed.update(row["path"] for row in json.loads(text)["managed_files"])
        except ValueError:
            continue  # the working-tree copy holds conflict markers
    world = {
        f"content/world/{locator}"
        for locator in document_locators(git("show", f"HEAD:{PIN_PATH}"))
    }
    return managed | world | set(REGISTRY) | set(EXTRA_DERIVED)


def needs_person(path: str, derived: set[str]) -> bool:
    if path.startswith(HAND_MAINTAINED_PREFIXES) or path in HAND_MAINTAINED:
        return True
    return path not in derived and path != PIN_PATH


def resolve_conflicts() -> list[str]:
    conflicted = git("diff", "--name-only", "--diff-filter=U").split()
    derived = derived_paths()
    manual = [path for path in conflicted if needs_person(path, derived)]
    # Check everything before writing anything, so a stop leaves the merge untouched.
    if not manual and PIN_PATH in conflicted and not merge_pin_test(write=False):
        manual = [PIN_PATH]
    if manual:
        print(
            "These conflicts need a person; resolve them first:",
            file=sys.stderr,
        )
        for path in manual:
            print(f"  {path}", file=sys.stderr)
        sys.exit(2)
    for path in conflicted:
        if path == PIN_PATH:
            merge_pin_test(write=True)
        elif git("show", f":3:{path}"):
            # Fully derived: either side parses, and the steps below overwrite it exactly.
            run(["git", "checkout", "--theirs", "--", path])
        elif git("show", f":2:{path}"):
            run(["git", "checkout", "--ours", "--", path])
        else:
            (ROOT / path).unlink(missing_ok=True)
    return conflicted


PIN_VALUE = re.compile(
    r'(\(\s*"[^"]+",\s*)[\d_]+(,\s*")[0-9a-f]{64}(")|(const TREE_SHA256: &str = ")[0-9a-f]{64}(";)'
)


def merge_pin_test(*, write: bool) -> bool:
    """Merge the Rust test with its generated pins masked; True when only the pins conflicted.

    The rest of the file is hand-written, so it is merged, never replaced by one side.
    """

    def masked(stage: str) -> str:
        return PIN_VALUE.sub(
            lambda m: (
                f"{m.group(1)}0{m.group(2)}{'0' * 64}{m.group(3)}"
                if m.group(1)
                else f"{m.group(4)}{'0' * 64}{m.group(5)}"
            ),
            git("show", f"{stage}{PIN_PATH}"),
        )

    with tempfile.TemporaryDirectory() as temp:
        paths = []
        for name, stage in (("ours", ":2:"), ("base", ":1:"), ("theirs", ":3:")):
            path = Path(temp) / name
            path.write_text(masked(stage), encoding="utf-8")
            paths.append(str(path))
        merged = subprocess.run(
            ["git", "merge-file", "-p", *paths], capture_output=True, text=True
        )
    if merged.returncode != 0:
        return False
    if write:
        PIN_TEST.write_text(merged.stdout, encoding="utf-8", newline="\n")
    return True


def document_locators(text: str | None = None) -> list[str]:
    text = PIN_TEST.read_text(encoding="utf-8") if text is None else text
    block = re.search(
        r"const DOCUMENTS: \[\(&str, usize, &str\); \d+\] = \[(.*?)\n\];", text, re.S
    )
    if not block:
        raise SystemExit(f"DOCUMENTS table not found in {PIN_TEST}")
    return re.findall(r'\(\s*"([^"]+)",', block.group(1))


def materialize_world() -> None:
    with tempfile.TemporaryDirectory() as temp:
        output = Path(temp) / "world"
        run(
            [
                "cargo",
                "run",
                "--locked",
                "-q",
                "-p",
                "oteryn-game-server",
                "--example",
                "materialize_content_world_project_v2",
                "--",
                "--output-root",
                str(output),
            ]
        )
        produced = sorted(
            p.relative_to(output).as_posix() for p in output.rglob("*") if p.is_file()
        )
        if produced != sorted(document_locators()):
            raise SystemExit(
                f"materializer produced {produced}, pins list {sorted(document_locators())}"
            )
        for locator in produced:
            shutil.copyfile(output / locator, WORLD / locator)


def literal(length: int, previous: str) -> str:
    """Keep the pinned literal when its value holds; otherwise follow its digit grouping."""
    if previous.replace("_", "") == str(length):
        return previous
    return f"{length:_}" if "_" in previous or length >= 10_000 else str(length)


def repin_rust_inventory() -> None:
    """Rewrite byte lengths and SHA-256 pins so the test pins exactly the regenerated package."""
    text = PIN_TEST.read_text(encoding="utf-8")
    tree = hashlib.sha256()
    for locator in document_locators():
        data = (WORLD / locator).read_bytes()
        tree.update(
            len(locator).to_bytes(8, "big")
            + locator.encode()
            + len(data).to_bytes(8, "big")
            + data
        )
        entry = re.compile(
            r'(\(\s*"' + re.escape(locator) + r'",\s*)([\d_]+)(,\s*")[0-9a-f]{64}(")'
        )
        text, count = entry.subn(
            lambda m: (
                f"{m.group(1)}{literal(len(data), m.group(2))}{m.group(3)}"
                f"{hashlib.sha256(data).hexdigest()}{m.group(4)}"
            ),
            text,
        )
        if count != 1:
            raise SystemExit(
                f"{PIN_TEST.name}: expected one DOCUMENTS entry for {locator}, found {count}"
            )
    text, count = re.subn(
        r'(const TREE_SHA256: &str = ")[0-9a-f]{64}(";)',
        lambda m: m.group(1) + tree.hexdigest() + m.group(2),
        text,
    )
    if count != 1:
        raise SystemExit(f"{PIN_TEST.name}: TREE_SHA256 not found")
    PIN_TEST.write_text(text, encoding="utf-8", newline="\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--resolve",
        action="store_true",
        help="after a stopped git merge: resolve derived conflicts, regenerate and stage",
    )
    parser.add_argument("--skip-checks", action="store_true", help="regenerate only")
    args = parser.parse_args()
    conflicted = resolve_conflicts() if args.resolve else []
    materialize_world()
    repin_rust_inventory()
    run(["python3", "tools/content-migration/world_project_v2_to_tree.py"])
    for tool in AUTHORING_TOOLS:
        if (ROOT / tool).is_file():
            run(["python3", tool, "content"])
    if not args.skip_checks:
        for command in CHECKS:
            if command[0] != "python3" or (ROOT / command[1]).is_file():
                run(command)
    if args.resolve:
        derived = derived_paths() | {PIN_PATH}
        changed = [
            path
            for path in git(
                "ls-files", "--modified", "--others", "--exclude-standard"
            ).split()
            if path in derived
        ]
        run(["git", "add", "-A", "--", *sorted(set(conflicted) | set(changed))])
    print("Derived content regenerated. Review `git status` and commit.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
