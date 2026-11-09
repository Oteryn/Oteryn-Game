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
the Charm, Proficiency, RewardClaim and StarterKit registrations, then the validators. The script never
commits; review `git status` and commit the merge yourself.

Use `--resolve --no-stage` for a worker that must leave the Git index untouched.
This requires a conflict-free worktree; it refreshes the bundle pins without staging.

With `--resolve` it finally runs `world-bundle-compiler pin-refresh` on the staged merge, so the
World pins (`content/world/pins/<slug>.json` and `.identity.json`) and the preproduction
manifest's `[world_bundle]` table follow the merged inputs; a pin conflict is derived too, unless
the two sides moved the reviewed `entry_start` differently.

`content/spells.manifest.json` is one canonical JSON line whose entries pin the bytes of the
files they name, and no step below rewrites it, so taking one side would drop the other side's
pins. It is merged key by key against the merge base instead, and every `{path, sha256}` entry
is re-hashed from the merged file it names. Both sides changing one key differently (other than
its digest) needs a person.

Scope: the command regenerates only the content tree, `content/world` and the Rust package pins.
It does not regenerate other Item-derived artifacts (the weapon proficiency sample, the TibiaWiki
snapshot re-key and promotion packet, the pins in `item_stats_promotion.rs`, the Crystal binding
output); where a `--check` exists it runs it, and a failure there needs a manual regeneration.
The native catalogue source budgets and World/NPC cohort fixtures are refreshed from the
normal canonical package by refresh_source_cohort_extent. Other derived count pins require
their corresponding source authoring tool.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
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
EXTRA_DERIVED = (
    "content/interactions/index.json",
    "content/quests/grizzly-boss-rooms.json",
    "tools/content-schema/native-gameplay/prey-model-results.json",
)
# Rewritten by the steps below but holding hand-maintained rows: the TibiaWiki import-only
# sources and batches (read back by world_project_v2_to_tree), hand-captured import records, and
# the hand-written fields of the interactions index. A conflict here needs a person.
HAND_MAINTAINED_PREFIXES = ("imports/",)
HAND_MAINTAINED = ("content/interactions/index.json",)
# World pins and their identity files (`<slug>.ruleset.json` is hand-written), rewritten by
# `world-bundle-compiler pin-refresh` from the staged inputs; the manifest follows the pin.
WORLD_PINS = "content/world/pins/"
PREPROD_MANIFEST = "deploy/synology-game/preprod-topology.toml"
# Merged key by key with its digests re-hashed; see the module docstring.
SPELLS_MANIFEST = "content/spells.manifest.json"

AUTHORING_TOOLS = (
    "tools/content-migration/raid_catalogue.py",
    "tools/content-schema/craft-authoring/craft_authoring.py",
    "tools/content-schema/imbuement-authoring/imbuement_content.py",
    "tools/content-schema/charm-authoring/charm_authoring.py",
    "tools/content-schema/proficiency-authoring/proficiency_authoring.py",
    "tools/content-schema/reward-claim-authoring/reward_claim_authoring.py",
    "tools/content-schema/starter-kit-authoring/starter_kit_authoring.py",
)

CHECKS = (
    ["python3", "tools/content-migration/refresh_season_creature_inventory.py", "--check"],
    ["python3", "tools/content-migration/refresh_equipment_profile_pin.py", "--check"],
    ["python3", "tools/content-schema/quest-authoring/arena_completion.py", "--check"],
    ["python3", "tools/content-schema/quest-authoring/grizzly_rooms.py", "--check"],
    ["python3", "tools/content-migration/reward_bag_catalogue.py", "--check"],
    ["python3", "tools/content-schema/quest-authoring/quest_batch4.py", "--available", "--check"],
    ["python3", "tools/content-migration/prey_difficulty.py", "--check"],
    ["python3", "tools/content-migration/test_prey_difficulty.py"],
    ["python3", "tools/content-schema/store-authoring/build_runtime_catalogue.py", "--check"],
    ["python3", "tools/content-migration/imbuement_catalogue.py", "--check"],
    ["python3", "tools/content-migration/register_spell_families.py", "--check"],
    ["python3", "tools/content-migration/test_import_spell_families.py"],
    ["python3", "tools/content-migration/validate_world_project_v2_to_tree.py"],
    ["python3", "tools/content-migration/test_world_project_v2_to_tree.py"],
    ["python3", "tools/content-schema/validate_materialized_game_tree.py"],
    ["python3", "tools/content-census/item_key_references.py"],
    *(["python3", tool, "content", "--check"] for tool in AUTHORING_TOOLS),
    ["python3", "tools/content-schema/item-authoring/test_engine_items.py"],
    [
        "python3",
        "tools/content-schema/item-authoring/item_weapon_proficiency.py",
        "--check",
    ],
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
    """Run git and fail closed: any error stops the script instead of reading as "nothing"."""
    result = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True)
    if result.returncode != 0:
        raise SystemExit(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout


def show(stage: str, path: str) -> str | None:
    """The file at an index stage (":1:", ":2:", ":3:") or "HEAD:"; None when it has none."""
    if stage == "HEAD:":
        present = bool(git("ls-tree", "HEAD", "--", path))
    else:
        entries = (
            entry.partition("\t")
            for entry in git("ls-files", "-z", "--stage", "--", path).split("\0")
        )
        present = any(
            name == path and info.split()[2] == stage.strip(":")
            for info, _, name in entries
        )
    return git("show", f"{stage}{path}") if present else None


def prey_report_paths() -> set[str]:
    """Only the active authoring date's reports are rewritten by regeneration."""
    try:
        date = json.loads((ROOT / "rulesets/prey/difficulty.json").read_text(encoding="utf-8"))["model"]["date"]
        if re.fullmatch(r"\d{4}-\d{2}-\d{2}", date) is None:
            return set()
    except (OSError, ValueError, KeyError, TypeError):
        # A conflicting or invalid authoring file remains a human-owned input.
        return set()
    return {f"docs/balance/PREY-TIME-SIM-{date}.md", f"docs/balance/PREY-SLOT-VALUE-{date}.md"}


def derived_paths() -> set[str]:
    """Every path the steps below write: the managed files of both merge sides, and the rest."""
    managed = set()
    for stage in (":2:", ":3:", "HEAD:", ""):
        text = (
            show(stage, "content/manifest.json")
            if stage
            else (ROOT / "content/manifest.json").read_text(encoding="utf-8")
        )
        if text is None:
            continue  # no merge in progress, so that stage does not exist
        try:
            managed.update(row["path"] for row in json.loads(text)["managed_files"])
        except ValueError:
            continue  # the working-tree copy holds conflict markers
    world = {
        f"content/world/{locator}"
        for locator in document_locators(git("show", f"HEAD:{PIN_PATH}"))
    }
    return managed | world | set(REGISTRY) | set(EXTRA_DERIVED) | prey_report_paths()


def is_world_pin(path: str) -> bool:
    name = path.removeprefix(WORLD_PINS)
    return (
        path.startswith(WORLD_PINS)
        and "/" not in name
        and name.endswith(".json")
        and not name.endswith(".ruleset.json")
    )


def entry_start_agrees(path: str) -> bool:
    """Both sides of a conflicted pin keep one reviewed `entry_start` (identity files have none)."""
    if path.endswith(".identity.json"):
        return True
    sides = [show(stage, path) for stage in (":2:", ":3:")]
    if None in sides:
        return False
    try:
        return len({json.dumps(json.loads(side)["entry_start"], sort_keys=True) for side in sides}) == 1
    except (ValueError, KeyError):
        return False


# Stands in for a file pin's digest until the merged file is hashed.
REHASH = "<rehash>"


def is_file_pin(value: object) -> bool:
    return isinstance(value, dict) and "path" in value and "sha256" in value


def spells_manifest_keys() -> dict | None:
    """Three-way merge of the conflicted spells manifest by top-level key, digests left out.

    None when a side does not parse or both sides changed one key differently."""
    sides = [show(stage, SPELLS_MANIFEST) for stage in (":1:", ":2:", ":3:")]
    if sides[1] is None or sides[2] is None:
        return None
    try:
        base, ours, theirs = (json.loads(side) if side is not None else {} for side in sides)
    except ValueError:
        return None
    if not all(isinstance(side, dict) for side in (base, ours, theirs)):
        return None
    absent = object()

    def comparable(side: dict, key: str) -> object:
        if key not in side:
            return absent
        value = side[key]
        if is_file_pin(value):
            return {**value, "sha256": REHASH}
        return value

    merged = {}
    for key in sorted(set(base) | set(ours) | set(theirs)):
        was, mine, other = (comparable(side, key) for side in (base, ours, theirs))
        if mine == other or other == was:
            chosen = mine
        elif mine == was:
            chosen = other
        else:
            return None
        if chosen is not absent:
            merged[key] = chosen
    return merged


def pinned_file(entry: dict) -> Path:
    return (ROOT / "content" / entry["path"]).resolve()


def merge_spells_manifest(keys: dict) -> None:
    """Write the merged manifest, each file pin re-hashed from the merged file it names."""
    manifest = {}
    for key, value in keys.items():
        if is_file_pin(value) and value["sha256"] == REHASH:
            value = {**value, "sha256": hashlib.sha256(pinned_file(value).read_bytes()).hexdigest()}
        manifest[key] = value
    text = json.dumps(manifest, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    (ROOT / SPELLS_MANIFEST).write_text(text + "\n", encoding="utf-8")


def needs_person(path: str, derived: set[str]) -> bool:
    if path.startswith(HAND_MAINTAINED_PREFIXES) or path in HAND_MAINTAINED:
        return True
    if is_world_pin(path):
        return not entry_start_agrees(path)
    return path not in derived and path != PIN_PATH


def resolve_conflicts() -> list[str]:
    conflicted = git("diff", "--name-only", "--diff-filter=U").splitlines()
    derived = derived_paths()
    manual = [path for path in conflicted if needs_person(path, derived)]
    spells_keys = None
    if SPELLS_MANIFEST in conflicted and SPELLS_MANIFEST not in manual:
        spells_keys = spells_manifest_keys()
        missing = [
            value["path"]
            for value in (spells_keys or {}).values()
            if is_file_pin(value) and not pinned_file(value).is_file()
        ]
        if spells_keys is None or missing:
            manual.append(SPELLS_MANIFEST)
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
        if path == SPELLS_MANIFEST:
            continue  # hashed below, once the files it names are resolved
        if path == PIN_PATH:
            merge_pin_test(write=True)
        elif show(":3:", path) is not None:
            # Fully derived: either side parses, and the steps below overwrite it exactly.
            run(["git", "checkout", "--theirs", "--", path])
        elif show(":2:", path) is not None:
            run(["git", "checkout", "--ours", "--", path])
        else:
            (ROOT / path).unlink(missing_ok=True)
    if spells_keys is not None:
        merge_spells_manifest(spells_keys)
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
            show(stage, PIN_PATH) or "",
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


# The from-scratch materializer chain cannot produce the #1807 monster/spell delta or the D3-7
# corpse admission, so the package is reproduced from this pinned predecessor (the parent of #1830;
# the materializer digest-verifies it). Keep in step with g4-canonical-worldproject-package-seed.yml.
PREDECESSOR_PIN = "d78f0c80e6efe7b5d2fea6dae010eaf236ccd7a1"
PREDECESSOR_DOCUMENTS = (
    "project.json",
    "manifest.json",
    "content.lock.json",
    "assets/catalog.json",
    "definitions/declarations.json",
    "definitions/reference.json",
    "editor/author.json",
    "presentations/bindings.json",
    "provenance/imports.json",
    "provenance/sources.json",
    "worlds/world.json",
)


def export_predecessor(destination: Path) -> None:
    """Write exactly the pinned predecessor's package documents; tracked content/world is never an input."""
    if subprocess.run(
        ["git", "cat-file", "-e", f"{PREDECESSOR_PIN}^{{commit}}"], cwd=ROOT, capture_output=True
    ).returncode:
        run(["git", "fetch", "--no-tags", "--depth=1", "origin", PREDECESSOR_PIN])
    for locator in PREDECESSOR_DOCUMENTS:
        target = destination / locator
        target.parent.mkdir(parents=True, exist_ok=True)
        blob = subprocess.run(
            ["git", "show", f"{PREDECESSOR_PIN}:content/world/{locator}"],
            cwd=ROOT,
            capture_output=True,
            check=True,
        ).stdout
        target.write_bytes(blob)


def materialize_world() -> None:
    with tempfile.TemporaryDirectory() as temp:
        output = Path(temp) / "world"
        predecessor = Path(temp) / "predecessor"
        export_predecessor(predecessor)
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
                "--predecessor-root",
                str(predecessor),
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
    # The NPC catalogue proves that it loaded this same complete package.
    npc_test = ROOT / "apps/game-server/tests/content_npc_catalogue.rs"
    npc_text, count = re.subn(
        r'(catalogue\.source_tree_digest\(\),\s*")[0-9a-f]{64}(")',
        lambda match: match.group(1) + tree.hexdigest() + match.group(2),
        npc_test.read_text(encoding="utf-8"),
    )
    if count != 1:
        raise SystemExit(f"{npc_test.name}: source_tree_digest pin not found")
    npc_test.write_text(npc_text, encoding="utf-8", newline="\n")
    # The inert raw intake must retain every normal non-raw batch, including
    # separately qualified source successors. Its historical cohort assertions
    # remain authored tests; only this exact measured extent is generated.
    raw_test = ROOT / "apps/game-server/tests/integration_rest/content_item_fx_audio_raw.rs"
    if raw_test.is_file():
        imports = json.loads((WORLD / "provenance/imports.json").read_bytes())["batches"]
        raw_id = "g4-item-fx-audio295-raw-evidence-r1"
        if sum(batch["batch_id"] == raw_id for batch in imports) != 1:
            raise SystemExit("raw import fixture requires exactly one unchanged raw cohort")
        raw_text, count = re.subn(
            r"assert_eq!\(batches\.len\(\), [\d_]+\);",
            f"assert_eq!(batches.len(), {len(imports)-1});",
            raw_test.read_text(encoding="utf-8"),
        )
        if count != 1:
            raise SystemExit(f"{raw_test.name}: expected one generated source extent")
        raw_test.write_text(raw_text, encoding="utf-8", newline="\n")


def source_cohort_extent(world: Path) -> dict[str, int | str]:
    """Measure only the normal, already admitted WorldProject source documents."""
    reference = json.loads((world / "definitions/reference.json").read_bytes())
    declarations = json.loads((world / "definitions/declarations.json").read_bytes())
    provenance = json.loads((world / "provenance/sources.json").read_bytes())
    imports = json.loads((world / "provenance/imports.json").read_bytes())
    project = json.loads((world / "project.json").read_bytes())
    records = reference["records"]
    profiles = declarations["authoring_profiles"]
    npcs = [row for row in declarations["records"] if row["kind"] == "NPC"]
    npc_profiles = {
        (row[field]["family"], row[field]["key"], row[field]["revision"])
        for row in npcs for field in ("presentation", "behavior")
        if row.get(field) is not None
    }
    profile_id = lambda row: (row["target"]["family"], row["target"]["key"], row["target"]["revision"])
    reference_id = lambda row: (row["identity"]["family"], row["identity"]["key"], row["identity"]["revision"])
    known_profiles = {profile_id(row) for row in profiles}
    known_references = {reference_id(row) for row in records}
    if len(known_references) != len(records) or not npc_profiles <= known_profiles or not npc_profiles <= known_references:
        raise SystemExit("source cohort contains duplicate references or missing NPC profiles")
    return {
        "references": len(records),
        "sources": len(provenance["sources"]),
        "imports": len(imports["batches"]),
        "creatures": sum(row["kind"] == "Creature" for row in records),
        "creature_records": sum(row["kind"] != "Item" and reference_id(row) not in npc_profiles for row in records),
        "creature_profiles": sum(profile_id(row) not in npc_profiles for row in profiles),
        "npc_records": len(npc_profiles),
        "project_revision": project["project_revision"],
    }


def refresh_source_cohort_extent() -> None:
    """Keep strict native source bounds and repository fixtures at the exact cohort."""
    measured = source_cohort_extent(WORLD)
    def rewrite(path: Path, pattern: str, replacement: str) -> None:
        text, count = re.subn(pattern, replacement, path.read_text(encoding="utf-8"))
        if count != 1:
            raise SystemExit(f"{path.name}: expected one source-cohort field, found {count}")
        path.write_text(text, encoding="utf-8", newline="\n")
    catalogue = ROOT / "crates/gameplay/content/src/content/npc_catalogue.rs"
    catalogue_text = catalogue.read_text(encoding="utf-8")
    catalogue_text = re.sub(r"\bCW2_B1_FULL_ITEM_FAMILY_COUNT,\s*", "", catalogue_text, count=1)
    catalogue_text = re.sub(r"            // #1807 monster source mechanics.*?            max_import_records: [^\n]+", "            // Generated source-cohort bounds: regenerate_content.py measures the\n            // already admitted normal World package; limit+1 remains rejected.\n            max_reference_records: " + f'{measured["references"]:_},' + "\n            max_import_records: " + f'{measured["sources"]:_},', catalogue_text, count=1, flags=re.S)
    catalogue.write_text(catalogue_text, encoding="utf-8", newline="\n")
    rewrite(catalogue, r"max_reference_records: [^,\n]+,", f'max_reference_records: {measured["references"]:_},')
    rewrite(catalogue, r"max_import_records: [^,\n]+,", f'max_import_records: {measured["sources"]:_},')
    for name, key in (("CREATURES", "creatures"), ("CREATURE_RECORDS", "creature_records"), ("CREATURE_PROFILES", "creature_profiles"), ("NPC_RECORDS", "npc_records")):
        rewrite(PIN_TEST, rf"const {name}: usize = [\d_]+;", f'const {name}: usize = {measured[key]:_};')
    rewrite(PIN_TEST, r"max_import_records: [^,\n]+,", f'max_import_records: {measured["sources"]:_},')
    rewrite(PIN_TEST, r"assert_eq!\(project\.imports\(\)\.len\(\), [\d_]+\);", f'assert_eq!(project.imports().len(), {measured["imports"]});')
    rewrite(PIN_TEST, r"assert_eq!\(v2\.sources\.len\(\), [\d_]+\);", f'assert_eq!(v2.sources.len(), {measured["sources"]});')
    for path, receiver in ((PIN_TEST, "project"), (ROOT / "apps/game-server/tests/content_npc_catalogue.rs", "catalogue")):
        rewrite(path, rf'({receiver}\.project_revision\(\),\s*")[^"]+("\s*\))', rf'\g<1>{measured["project_revision"]}\g<2>')

def unresolved_worktree_paths() -> list[str]:
    """Workers resolve files without staging the coordinator's pending merge."""
    return [
        path
        for path in git("diff", "--name-only", "--diff-filter=U").splitlines()
        if not (ROOT / path).is_file()
        or re.search(rb"^(?:<<<<<<<|\|\|\|\|\|\|\||=======|>>>>>>>)", (ROOT / path).read_bytes(), re.M)
    ]


def refresh_quest_callback_proof() -> None:
    """Re-pin the callback inventory witness without changing reviewed core snapshots."""
    directory = ROOT / "tools/content-schema/quest-authoring/samples/completion242"
    approval_path = directory / "source-fix-approval.json"
    receipt_path = directory / "source-fix-receipt.json"
    if not approval_path.is_file() or not receipt_path.is_file():
        return
    approval = json.loads(approval_path.read_bytes())
    receipt = json.loads(receipt_path.read_bytes())
    callback_path = "content/quests/world-callbacks.json"
    digest = hashlib.sha256((ROOT / callback_path).read_bytes()).hexdigest()
    for document in (approval, receipt):
        for descriptor in document["proof_inputs"]:
            if descriptor["path"] == callback_path:
                descriptor["sha256"] = digest
    approval_path.write_text(json.dumps(approval, ensure_ascii=False, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    receipt["approval"]["sha256"] = hashlib.sha256(approval_path.read_bytes()).hexdigest()
    receipt_path.write_text(json.dumps(receipt, ensure_ascii=False, sort_keys=True, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--resolve",
        action="store_true",
        help="after a stopped git merge: resolve derived conflicts, regenerate and stage",
    )
    parser.add_argument("--no-stage", action="store_true", help="leave the git index untouched, including with --resolve")
    parser.add_argument("--skip-checks", action="store_true", help="regenerate only")
    parser.add_argument("--check", action="store_true", help="run the registered content checks without writing")
    parser.add_argument("--capability-golden", help="regenerate only the named runtime capability golden")
    parser.add_argument("--native-only", action="store_true", help="regenerate native providers and their registry pins without rebuilding the unchanged World package")
    args = parser.parse_args()
    if args.capability_golden:
        if args.check or args.resolve or args.skip_checks or args.no_stage:
            parser.error("--capability-golden cannot be combined with content options")
        run(["python3", "tools/contracts/update_capability_golden.py", args.capability_golden, "--client-registrations"])
        return 0
    if args.native_only:
        run(["python3", "tools/content-schema/quest-authoring/arena_completion.py"])
        if args.resolve or args.no_stage or args.check:
            parser.error("--native-only cannot be combined with --resolve, --no-stage or --check")
        run(["python3", "tools/content-migration/register_spell_families.py"])
        run(["python3", "tools/content-migration/refresh_equipment_profile_pin.py"])
        run(["python3", "tools/content-schema/quest-authoring/grizzly_rooms.py"])
        run(["python3", "tools/content-migration/raid_catalogue.py", "content"])
        run(["python3", "tools/content-migration/refresh_season_creature_inventory.py"])
        if not args.skip_checks:
            run(["python3", "tools/content-migration/register_spell_families.py", "--check"])
        return 0
    if args.check:
        if args.resolve or args.skip_checks or args.no_stage:
            parser.error("--check cannot be combined with mutation options")
        for command in CHECKS:
            if command[0] != "python3" or (ROOT / command[1]).is_file():
                run(command)
        return 0
    if args.no_stage and unresolved_worktree_paths():
        raise SystemExit("--no-stage requires a conflict-free worktree")
    conflicted = resolve_conflicts() if args.resolve and not args.no_stage else []
    run(["python3", "tools/content-schema/quest-authoring/arena_completion.py"])
    run(["python3", "tools/content-schema/quest-authoring/quest_batch4.py", "--available"])
    refresh_quest_callback_proof()
    # Source-backed field regeneration uses the same explicit source root as its tests.
    # Without local donor checkouts the committed, pinned source catalogue is retained.
    if sources := os.environ.get("OTERYN_SPELL_SOURCE_ROOT"):
        run(["python3", "tools/content-schema/spell-authoring/build_field_profiles.py", "--sources", sources])
    materialize_world()
    refresh_source_cohort_extent()
    repin_rust_inventory()
    run(["python3", "tools/content-schema/store-authoring/build_runtime_catalogue.py", "--refresh-owner-offers"])
    run(["python3", "tools/content-migration/world_project_v2_to_tree.py"])
    for tool in AUTHORING_TOOLS:
        if (ROOT / tool).is_file():
            run(["python3", tool, "content"])
    # Room provenance includes the native Item provider, including NPC inputs.
    # Refresh after all native/map producers, before freshness checks and staging.
    run(["python3", "tools/content-schema/quest-authoring/grizzly_rooms.py"])
    run(["python3", "tools/content-migration/refresh_equipment_profile_pin.py"])
    run(["python3", "tools/content-migration/refresh_season_creature_inventory.py"])
    if not args.skip_checks:
        for command in CHECKS:
            if command[0] != "python3" or (ROOT / command[1]).is_file():
                run(command)
    if args.resolve:
        derived = derived_paths() | {PIN_PATH, "apps/game-server/tests/content_npc_catalogue.rs"}
        changed = [
            path
            for path in git(
                "ls-files", "--modified", "--others", "--exclude-standard"
            ).splitlines()
            if path in derived
        ]
        if not args.no_stage:
            run(["git", "add", "-A", "--", *sorted(set(conflicted) | set(changed))])
        # Refresh the bundle pin last; the input digest is a staged-tree diagnostic only.
        run(["cargo", "+1.99.1", "run", "--locked", "--release", "-p", "oteryn-world-bundle-compiler", "--", "pin-refresh", "."])
        if not args.no_stage:
            run(["git", "add", "--", WORLD_PINS, PREPROD_MANIFEST])
    print("Derived content regenerated. Review `git status` and commit.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
