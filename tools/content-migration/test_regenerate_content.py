#!/usr/bin/env python3
"""Fixture tests for regenerate_content.py --resolve: which merge conflicts it may resolve.

Builds a tiny git repository in a temp dir with a real stopped merge; no cargo, no network.
Run with `python3 tools/content-migration/test_regenerate_content.py`.
"""

from __future__ import annotations

import gzip
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import regenerate_content as rc

SOURCES = "imports/tibiawiki/sources.json"
DERIVED = "content/items/index.json"
INTERACTIONS = "content/interactions/index.json"
WORLD_PIN = "content/world/pins/oteryn.json"
PIN_TEST = f'const DOCUMENTS: [(&str, usize, &str); 1] = [\n    ("definitions/reference.json", 1, "{"0" * 64}"),\n];\n'


def git(root: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@t", *args],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout


def write(root: Path, path: str, value: object) -> None:
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    text = value if isinstance(value, str) else json.dumps(value, indent=2) + "\n"
    target.write_text(text, encoding="utf-8")


def sources(*batches: str) -> dict:
    return {
        "sources": [
            {"key": "oteryn:source.tibiawiki", "import_batch_id": batch}
            for batch in batches
        ]
    }


def world_pin(side: str, x: int = 1) -> dict:
    return {"digest": side, "entry_start": {"x": x, "y": 2, "floor": 7}}


def stopped_merge(root: Path, conflicting: list[str], *, moved_entry: bool = False) -> None:
    """Base, then `ours` and `theirs` each change every path in `conflicting`, then merge."""
    git(root, "init", "-q", "-b", "main")
    manifest = {"managed_files": [{"path": SOURCES}, {"path": DERIVED}]}
    write(root, "content/manifest.json", manifest)
    write(root, rc.PIN_PATH, PIN_TEST)
    write(root, SOURCES, sources("base"))
    write(root, DERIVED, {"items": ["base"]})
    write(root, INTERACTIONS, {"contract": "hand-written", "notes": "base"})
    write(root, WORLD_PIN, world_pin("base"))
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "base")
    git(root, "checkout", "-q", "-b", "theirs")
    for side in ("theirs", "ours"):
        if side == "ours":
            git(root, "checkout", "-q", "main")
        for path in conflicting:
            if path == SOURCES:
                write(root, path, sources("base", f"{side}-batch"))
            elif path == INTERACTIONS:
                write(root, path, {"contract": f"{side}-contract", "notes": side})
            elif path == WORLD_PIN:
                write(root, path, world_pin(side, 3 if moved_entry and side == "ours" else 1))
            else:
                write(root, path, {"items": ["base", side]})
        git(root, "commit", "-q", "-am", side)
    # The merge needs an identity too, or it stops before merging and leaves no conflict.
    merged = subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@t", "merge", "-q", "theirs"],
        cwd=root,
        capture_output=True,
    )
    assert merged.returncode != 0, "fixture merge must stop on conflicts"
    assert sorted(unmerged(root)) == sorted(conflicting), unmerged(root)


def resolve_in(root: Path) -> tuple[int, list[str]]:
    rc.ROOT, rc.PIN_TEST, rc.WORLD = root, root / rc.PIN_PATH, root / "content/world"
    try:
        return 0, rc.resolve_conflicts()
    except SystemExit as stop:
        return int(stop.code), []


def unmerged(root: Path) -> list[str]:
    return git(root, "diff", "--name-only", "--diff-filter=U").split()


def test_import_conflict_fails_closed_and_keeps_both_sides() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        stopped_merge(root, [SOURCES, DERIVED])
        code, _ = resolve_in(root)
        assert code == 2, code
        # Nothing was written: both paths stay unmerged, and each side's captured row survives.
        assert unmerged(root) == sorted([SOURCES, DERIVED]), unmerged(root)
        ours = json.loads(git(root, "show", f":2:{SOURCES}"))
        theirs = json.loads(git(root, "show", f":3:{SOURCES}"))
        assert [row["import_batch_id"] for row in ours["sources"]] == [
            "base",
            "ours-batch",
        ]
        assert [row["import_batch_id"] for row in theirs["sources"]] == [
            "base",
            "theirs-batch",
        ]
        text = (root / SOURCES).read_text(encoding="utf-8")
        assert "ours-batch" in text and "theirs-batch" in text


def test_interactions_index_conflict_fails_closed() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        stopped_merge(root, [INTERACTIONS])
        code, _ = resolve_in(root)
        assert code == 2, code
        assert unmerged(root) == [INTERACTIONS], unmerged(root)


def test_derived_only_conflict_is_resolved() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        stopped_merge(root, [DERIVED])
        code, resolved = resolve_in(root)
        assert code == 0 and resolved == [DERIVED], (code, resolved)
        assert json.loads((root / DERIVED).read_text(encoding="utf-8")) == {
            "items": ["base", "theirs"]
        }


def test_world_pin_conflict_is_derived() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        stopped_merge(root, [WORLD_PIN])
        code, resolved = resolve_in(root)
        # Either side parses; `pin-refresh` then rewrites its digests from the staged merge.
        assert code == 0 and resolved == [WORLD_PIN], (code, resolved)
        assert json.loads((root / WORLD_PIN).read_text(encoding="utf-8")) == world_pin("theirs")


def test_world_pin_conflict_on_a_moved_entry_start_fails_closed() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        stopped_merge(root, [WORLD_PIN], moved_entry=True)
        code, _ = resolve_in(root)
        assert code == 2, code
        assert unmerged(root) == [WORLD_PIN], unmerged(root)


def test_git_failure_while_enumerating_conflicts_fails_closed() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        stopped_merge(root, [SOURCES, DERIVED])
        before = {
            path: (root / path).read_bytes() for path in (SOURCES, DERIVED, rc.PIN_PATH)
        }
        corrupt = root / "corrupt-index"
        corrupt.write_bytes(b"not an index")
        commands: list[list[str]] = []
        saved = rc.run, sys.argv, os.environ.get("GIT_INDEX_FILE")
        rc.run, sys.argv = commands.append, ["regenerate_content.py", "--resolve"]
        os.environ["GIT_INDEX_FILE"] = str(corrupt)
        rc.ROOT, rc.PIN_TEST, rc.WORLD = (
            root,
            root / rc.PIN_PATH,
            root / "content/world",
        )
        try:
            rc.main()
            raise AssertionError(
                "--resolve must stop when git cannot list the conflicts"
            )
        except SystemExit as stop:
            assert stop.code not in (0, None), stop.code
        finally:
            rc.run, sys.argv = saved[0], saved[1]
            if saved[2] is None:
                os.environ.pop("GIT_INDEX_FILE", None)
            else:
                os.environ["GIT_INDEX_FILE"] = saved[2]
        # Nothing ran and nothing was written: the merge is exactly as git left it.
        assert commands == [], commands
        assert {path: (root / path).read_bytes() for path in before} == before
        assert unmerged(root) == sorted([SOURCES, DERIVED]), unmerged(root)


def canonical(value: dict) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n"


def sha(root: Path, path: str) -> str:
    return hashlib.sha256((root / path).read_bytes()).hexdigest()


def spells_merge(root: Path, ours: dict, theirs: dict) -> None:
    """Base, then each side rewrites the spells manifest and the profile files it names."""
    git(root, "init", "-q", "-b", "main")
    write(root, "content/manifest.json", {"managed_files": [{"path": rc.SPELLS_MANIFEST}]})
    write(root, rc.PIN_PATH, PIN_TEST)
    base = {"schema": "v5", "catalog": {"path": "abilities/catalog.json", "sha256": ""}}
    write(root, "content/abilities/catalog.json", {"spells": ["base"]})
    base["catalog"]["sha256"] = sha(root, "content/abilities/catalog.json")
    write(root, rc.SPELLS_MANIFEST, canonical(base))
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "base")
    git(root, "checkout", "-q", "-b", "theirs")
    for side, change in (("theirs", theirs), ("ours", ours)):
        if side == "ours":
            git(root, "checkout", "-q", "main")
        manifest = dict(base)
        for key, value in change.items():
            if isinstance(value, dict):
                write(root, f"content/{value['path']}", {"profiles": [side]})
                value = {**value, "sha256": sha(root, f"content/{value['path']}")}
            manifest[key] = value
        write(root, rc.SPELLS_MANIFEST, canonical(manifest))
        git(root, "add", "-A")
        git(root, "commit", "-q", "-m", side)
    merged = subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@t", "merge", "-q", "theirs"],
        cwd=root,
        capture_output=True,
        check=False,
    )
    assert merged.returncode != 0, "fixture merge must stop on conflicts"
    assert unmerged(root) == [rc.SPELLS_MANIFEST], unmerged(root)


def test_spells_manifest_conflict_merges_keys_and_rehashes_pins() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        spells_merge(
            root,
            {"items": {"path": "items/profiles.json"}},
            {"creatures": {"path": "creatures/profiles.json"}},
        )
        code, resolved = resolve_in(root)
        assert code == 0 and resolved == [rc.SPELLS_MANIFEST], (code, resolved)
        text = (root / rc.SPELLS_MANIFEST).read_text(encoding="utf-8")
        manifest = json.loads(text)
        assert text == canonical(manifest), text
        assert sorted(manifest) == ["catalog", "creatures", "items", "schema"], manifest
        for key in ("catalog", "creatures", "items"):
            assert manifest[key]["sha256"] == sha(root, f"content/{manifest[key]['path']}"), key


def test_spells_manifest_conflict_on_one_key_fails_closed() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        spells_merge(root, {"schema": "v6-ours"}, {"schema": "v6-theirs"})
        before = (root / rc.SPELLS_MANIFEST).read_bytes()
        code, _ = resolve_in(root)
        assert code == 2, code
        assert unmerged(root) == [rc.SPELLS_MANIFEST], unmerged(root)
        assert (root / rc.SPELLS_MANIFEST).read_bytes() == before


def test_no_stage_checks_binary_conflicts_without_decoding() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        compressed = root / "dialogue.json.gz"
        compressed.write_bytes(gzip.compress(b'{"records": []}', mtime=0))
        text = root / "quest.json"
        text.write_text('{"records": []}\n')
        paths = "dialogue.json.gz\nquest.json\n"
        with patch.object(rc, "ROOT", root), patch.object(rc, "git", return_value=paths):
            assert rc.unresolved_worktree_paths() == []
            text.write_text("<<<<<<< ours\n{}\n=======\n{}\n>>>>>>> incoming\n")
            assert rc.unresolved_worktree_paths() == ["quest.json"]
            compressed.unlink()
            assert rc.unresolved_worktree_paths() == ["dialogue.json.gz", "quest.json"]


def test_callback_proof_refresh_preserves_reviewed_snapshots() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        directory = "tools/content-schema/quest-authoring/samples/completion242/"
        callback = "content/quests/world-callbacks.json"
        proof = [{"path": callback, "sha256": "old"}, {"path": "other", "sha256": "kept"}]
        compiler = "tools/compiler.py"
        compiler_inputs = [{"path": compiler, "sha256": "old"}]
        approval = {"compiler_inputs": compiler_inputs, "proof_inputs": proof,
                    "approved_core_digests": ["core"], "approved_graphs": ["graph"]}
        receipt = {"compiler_inputs": compiler_inputs, "proof_inputs": proof, "changes": ["reviewed"],
                   "approval": {"path": directory + "source-fix-approval.json", "sha256": "old"}}
        write(root, compiler, "# merged compiler\n")
        write(root, callback, {"schema": 1, "callbacks": []})
        write(root, directory + "source-fix-approval.json", approval)
        write(root, directory + "source-fix-receipt.json", receipt)
        with patch.object(rc, "ROOT", root):
            rc.refresh_quest_callback_proof()
            paths = [root / (directory + name) for name in ("source-fix-approval.json", "source-fix-receipt.json")]
            first = [path.read_bytes() for path in paths]
            rc.refresh_quest_callback_proof()
            assert first == [path.read_bytes() for path in paths]
        approval["proof_inputs"][0]["sha256"] = hashlib.sha256((root / callback).read_bytes()).hexdigest()
        compiler_inputs[0]["sha256"] = sha(root, compiler)
        assert approval["proof_inputs"] == proof
        assert receipt["proof_inputs"] == proof
        receipt["approval"]["sha256"] = hashlib.sha256(first[0]).hexdigest()
        assert json.loads(first[0]) == approval
        assert json.loads(first[1]) == receipt


def test_unstaged_binary_conflict_does_not_require_text_decoding() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        (root / "packet.json.gz").write_bytes(b"\x1f\x8b\xff\x00")
        (root / "conflicted.json").write_bytes(b"<<<<<<< HEAD\n{}\n=======\n[]\n>>>>>>> main\n")
        with patch.object(rc, "ROOT", root), patch.object(rc, "git", return_value="packet.json.gz\nconflicted.json\n"):
            assert rc.unresolved_worktree_paths() == ["conflicted.json"]


def test_worker_conflict_scan_accepts_binary_and_rejects_unresolved_paths() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        (root / "resolved.json.gz").write_bytes(b"\x1f\x8b\xff\x00")
        (root / "conflict.json").write_bytes(b"<<<<<<< ours\n{}\n=======\n{}\n>>>>>>> theirs\n")
        with patch.object(rc, "ROOT", root), patch.object(
            rc, "git", return_value="resolved.json.gz\nconflict.json\nmissing.json\n"
        ):
            assert rc.unresolved_worktree_paths() == ["conflict.json", "missing.json"]



def test_source_cohort_extent_separates_npc_profiles_and_tracks_successors() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        identity = lambda family, key: {"family": family, "key": key, "revision": "r1"}
        head = identity("Presentation", "oteryn:presentation.npc.one")
        behavior = identity("Behavior", "oteryn:behavior.npc.one")
        creature = identity("Creature", "oteryn:creature.one")
        records = [{"kind": "Generic", "identity": head}, {"kind": "Generic", "identity": behavior}, {"kind": "Creature", "identity": creature}, {"kind": "Item", "identity": identity("Item", "oteryn:item.one")}]
        document = {"records": [{"kind": "NPC", "presentation": head, "behavior": behavior}], "authoring_profiles": [{"target": row} for row in [head, behavior, creature]]}
        write(root, "definitions/reference.json", {"records": records})
        write(root, "definitions/declarations.json", document)
        write(root, "provenance/sources.json", {"sources": [{"key": "one"}]})
        write(root, "provenance/imports.json", {"batches": [{"batch_id": "one"}]})
        write(root, "project.json", {"project_revision": "one"})
        assert rc.source_cohort_extent(root) == {"references": 4, "sources": 1, "imports": 1, "creatures": 1, "creature_records": 1, "creature_profiles": 1, "npc_records": 2, "project_revision": "one"}
        successor = identity("Creature", "oteryn:creature.two")
        records.append({"kind": "Creature", "identity": successor})
        document["authoring_profiles"].append({"target": successor})
        write(root, "definitions/reference.json", {"records": records})
        write(root, "definitions/declarations.json", document)
        extent = rc.source_cohort_extent(root)
        assert (extent["references"], extent["creatures"], extent["creature_records"], extent["creature_profiles"], extent["npc_records"]) == (5, 2, 2, 2, 2)
        # Missing source profiles and duplicate identities cannot silently become
        # a larger accepted count merely because the file remains valid JSON.
        document["authoring_profiles"] = document["authoring_profiles"][1:]
        write(root, "definitions/declarations.json", document)
        try:
            rc.source_cohort_extent(root)
        except SystemExit as error:
            assert "missing NPC profiles" in str(error)
        else:
            raise AssertionError("missing NPC profile admitted as a count")
        document["authoring_profiles"] = [{"target": row} for row in [head, behavior, creature, successor]]
        write(root, "definitions/declarations.json", document)
        records.append(records[0])
        write(root, "definitions/reference.json", {"records": records})
        try:
            rc.source_cohort_extent(root)
        except SystemExit as error:
            assert "duplicate references" in str(error)
        else:
            raise AssertionError("duplicate reference admitted as a count")


def test_normal_repin_updates_raw_extent_without_rewriting_history_checks() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        world = root / "content/world"
        write(root, rc.PIN_PATH, PIN_TEST + f'const TREE_SHA256: &str = "{"0" * 64}";\n')
        write(root, "content/world/definitions/reference.json", {})
        write(root, "content/world/placements/index.json", {"palette": [], "regions": []})
        client = root / "crates/client-assets/src/world.rs"
        write(root, str(client.relative_to(root)), f'pub const PLACEMENT_INDEX_SHA256: &str = "{"0" * 64}";\n')
        write(root, "content/world/provenance/imports.json", {"batches": [{"batch_id": key} for key in ["legacy-a", "legacy-b", "g4-item-fx-audio295-raw-evidence-r1", "seasonal-cake-creature-crystal-201189f-r1"]]})
        write(root, "apps/game-server/tests/content_npc_catalogue.rs", f'assert_eq!(catalogue.source_tree_digest(), "{"0" * 64}");\n')
        path = root / "apps/game-server/tests/integration_rest/content_item_fx_audio_raw.rs"
        write(root, str(path.relative_to(root)), 'assert_eq!(batches.len(), 2);\nassert_eq!(historical.len(), 2);\nassert_eq!(&batches[..prior.len()], prior.as_slice());\n')
        with patch.object(rc, "ROOT", root), patch.object(rc, "WORLD", world), patch.object(rc, "PIN_TEST", root / rc.PIN_PATH):
            rc.repin_rust_inventory()
        text = path.read_text()
        assert 'assert_eq!(batches.len(), 3);' in text
        assert 'assert_eq!(historical.len(), 2);' in text
        assert 'assert_eq!(&batches[..prior.len()], prior.as_slice());' in text
        placement_sha = hashlib.sha256((world / "placements/index.json").read_bytes()).hexdigest()
        assert f'pub const PLACEMENT_INDEX_SHA256: &str = "{placement_sha}";' in client.read_text()

def test_source_cohort_includes_installed_successors() -> None:
    installed = []

    class Measured(Exception):
        pass

    def measure():
        assert installed == [rc.ROOT], "measure only the complete installed World"
        raise Measured

    with patch.object(sys, "argv", ["regenerate_content.py"]), \
            patch.object(rc, "run"), \
            patch.object(rc, "refresh_quest_callback_proof"), \
            patch.object(rc, "materialize_world"), \
            patch("questpc7_thaian_authoring.install", side_effect=installed.append), \
            patch.object(rc, "refresh_source_cohort_extent", side_effect=measure):
        try:
            rc.main()
        except Measured:
            pass
        else:
            raise AssertionError("the complete World cohort must be measured")


def test_normal_generation_orders_native_callback_producers() -> None:
    commands: list[tuple[str, ...]] = []
    assert full_generation_trace(commands, lambda *_: None) == 0
    native = "tools/content-schema/native-gameplay/"
    tree = ("python3", "tools/content-migration/world_project_v2_to_tree.py")
    pairs = [
        (("python3", native + producer), ("python3", native + consumer, "--refresh-native-pin"))
        for producer, consumer in (
            ("build_native_callback_witness.py", "build_native_death_event_witness.py"),
            ("build_srx3_soul_war_witness.py", "build_srx3_soul_war_death_events.py"),
            ("build_srx3_rotten_blood_witness.py", "build_srx3_rotten_death_events.py"),
        )
    ]
    required = [tree, *(command for pair in pairs for command in pair)]
    nonunique = {command[1]: commands.count(command) for command in required if commands.count(command) != 1}
    assert not nonunique, nonunique
    for producer, consumer in pairs:
        assert commands.index(tree) < commands.index(producer) < commands.index(consumer), (producer, consumer)


def full_generation_trace(events, inspect_command, *, prepare=None, checks=False) -> int:
    """Exercise the actual dispatcher without running any producer or Git write."""
    def record(command):
        command = tuple(command)
        events.append(command)
        inspect_command(command, events)

    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        for tool in rc.AUTHORING_TOOLS:
            write(root, tool, "")
        feedback = "tools/content-schema/quest-authoring/samples/donor-source/refinements/conditions.json"
        write(root, feedback, {"source_fixture": True})
        write(root, "content/quests/definitions/index.json", {"authoring_sources": [{
            "path": feedback, "sha256": hashlib.sha256((root / feedback).read_bytes()).hexdigest(),
        }]})
        check = ("python3", "fixture/final_check.py", "--check")
        write(root, check[1], "")
        if prepare is not None:
            prepare(root)
        argv = ["regenerate_content.py", "--no-stage"]
        if not checks:
            argv.append("--skip-checks")
        with patch.object(rc, "ROOT", root), \
                patch.object(sys, "argv", argv), \
                patch.object(rc, "CHECKS", [check]), \
                patch.dict(os.environ, {"OTERYN_SPELL_SOURCE_ROOT": ""}), \
                patch.object(rc, "run", side_effect=record), \
                patch.object(rc, "unresolved_worktree_paths", return_value=[]) as unresolved, \
                patch.object(rc, "refresh_quest_callback_proof"), \
                patch.object(rc, "materialize_world", side_effect=lambda: events.append(("materialize-world",))), \
                patch("questpc7_thaian_authoring.install"), \
                patch.object(rc, "refresh_source_cohort_extent"), \
                patch.object(rc, "repin_rust_inventory"), \
                patch.object(rc, "refresh_world_metadata", side_effect=lambda: events.append(("world-metadata",))), \
                patch.object(subprocess, "run", side_effect=AssertionError("unexpected subprocess")):
            result = rc.main()
            unresolved.assert_called_once_with()
        assert not any(command[0] in {"git", "cargo"} for command in events)
        if not checks:
            assert not any("--check" in command for command in events)
        return result


def test_imbuement_candidate_precedes_content_consumers_and_refusal_stops_them() -> None:
    candidate = ("python3", "tools/content-schema/imbuement-authoring/imbuement_authoring.py", "build")
    tree = ("python3", "tools/content-migration/world_project_v2_to_tree.py")

    def require_candidate(command, events):
        if command == tree:
            assert candidate in events[:-1], "content consumer requires the fresh candidate"

    events = []
    assert full_generation_trace(events, require_candidate) == 0
    assert events.count(candidate) == 1

    def refuse_candidate(command, _events):
        if command == candidate:
            raise subprocess.CalledProcessError(1, list(command))

    refused = []
    try:
        full_generation_trace(refused, refuse_candidate)
    except subprocess.CalledProcessError as error:
        assert error.cmd == list(candidate)
    else:
        raise AssertionError("a failed candidate producer must stop regeneration")
    assert tree not in refused, "a refused prerequisite cannot reach its consumer"


def test_reward_variant_packet_follows_final_claim_item_and_metadata_inputs() -> None:
    variant = ("python3", "tools/content-schema/reward-claim-authoring/reward_claim_variant_migration.py")
    inputs = (
        ("python3", "tools/content-migration/world_project_v2_to_tree.py"),
        ("python3", "tools/content-schema/reward-claim-authoring/reward_claim_authoring.py", "content"),
        ("python3", "tools/content-migration/register_spell_families.py"),
        ("world-metadata",),
    )

    def require_final_inputs(command, events):
        if command == variant:
            assert all(source in events[:-1] for source in inputs), "variant packet requires final inputs"

    events = []
    assert full_generation_trace(events, require_final_inputs) == 0
    assert events.count(variant) == 1, "the variant packet must actually be regenerated"
    variant_index = events.index(variant)
    for source in inputs:
        assert max(index for index, command in enumerate(events) if command == source) < variant_index


def downstream_quest_commands():
    prefix = "tools/content-schema/quest-authoring/"
    return tuple(("python3", prefix + name + ".py") for name in (
        "qengine_access", "qmech_m24_kill_bindings", "quest_explore_area_candidates",
        "quest_talk_npc_candidates", "quest_reward_only_binding_candidates",
        "quest_donor_source_authoring",
    ))


def test_callback_and_w01_witnesses_follow_final_provider_in_dependency_order() -> None:
    native = ("python3", "tools/content-migration/register_spell_families.py")
    callback = ("python3", "tools/content-schema/native-gameplay/build_native_callback_witness.py")
    death = ("python3", "tools/content-schema/native-gameplay/build_native_death_event_witness.py", "--refresh-native-pin")
    rooms = ("python3", "tools/content-schema/quest-authoring/grizzly_rooms.py")
    check = ("python3", "fixture/final_check.py", "--check")

    def require_current_callback(command, events):
        if command == callback:
            assert events[-2] == native, "callback witness requires the final native provider"
        if command == death:
            assert events[-2] == callback, "W01 refresh requires the canonical callback witness"
        if command == rooms:
            assert callback in events[:-1] and death in events[:-1], "consumers need both current witnesses"

    for checks in (False, True):
        events = []
        assert full_generation_trace(events, require_current_callback, checks=checks) == 0
        assert events.count(callback) == events.count(death) == 1
        final_native = max(i for i, command in enumerate(events) if command == native)
        assert final_native < events.index(callback) < events.index(death) < events.index(rooms)
        assert not any("--source-cache" in command or "--canary-tree" in command for command in events)
        if checks:
            assert events.index(death) < events.index(check)


def test_callback_or_w01_witness_refusal_stops_later_generation() -> None:
    callback = ("python3", "tools/content-schema/native-gameplay/build_native_callback_witness.py")
    death = ("python3", "tools/content-schema/native-gameplay/build_native_death_event_witness.py", "--refresh-native-pin")
    later = (
        ("python3", "tools/content-schema/native-gameplay/build_srx3_rotten_blood_witness.py"),
        ("python3", "tools/content-schema/quest-authoring/grizzly_rooms.py"),
        ("world-metadata",),
        ("python3", "fixture/final_check.py", "--check"),
    )
    for checks in (False, True):
        for refused in (callback, death):
            def refuse(command, _events):
                if command == refused:
                    raise subprocess.CalledProcessError(23, list(command))

            events = []
            try:
                full_generation_trace(events, refuse, checks=checks)
            except subprocess.CalledProcessError as error:
                assert error.returncode == 23 and error.cmd == list(refused)
            else:
                raise AssertionError("a refused canonical witness must stop regeneration")
            assert events.count(refused) == 1
            assert not any(command in events for command in later)
            if refused == callback:
                assert death not in events


def test_rotten_death_witnesses_follow_final_native_provider_before_consumers() -> None:
    native = ("python3", "tools/content-migration/register_spell_families.py")
    witness = ("python3", "tools/content-schema/native-gameplay/build_srx3_rotten_blood_witness.py")
    death = ("python3", "tools/content-schema/native-gameplay/build_srx3_rotten_death_events.py", "--refresh-native-pin")
    rooms = ("python3", "tools/content-schema/quest-authoring/grizzly_rooms.py")
    check = ("python3", "fixture/final_check.py", "--check")

    def require_final_native(command, events):
        if command == witness:
            assert native in events[:-1], "Rotten witness requires the final native provider"
        if command == death:
            assert events[-2] == witness, "death pin requires the strict Rotten witness first"
        if command == rooms:
            assert witness in events[:-1] and death in events[:-1], "consumers need both fresh Rotten pins"

    for checks in (False, True):
        events = []
        assert full_generation_trace(events, require_final_native, checks=checks) == 0
        assert events.count(witness) == events.count(death) == 1
        final_native = max(i for i, command in enumerate(events) if command == native)
        assert final_native < events.index(witness) < events.index(death) < events.index(rooms)
        assert not any("--source-root" in command or "--tree" in command for command in events)
        if checks:
            assert events.index(death) < events.index(check)


def test_rotten_death_witness_refusal_stops_every_later_consumer() -> None:
    chain = (
        ("python3", "tools/content-migration/register_spell_families.py"),
        ("python3", "tools/content-schema/native-gameplay/build_srx3_rotten_blood_witness.py"),
        ("python3", "tools/content-schema/native-gameplay/build_srx3_rotten_death_events.py", "--refresh-native-pin"),
        ("python3", "tools/content-schema/quest-authoring/grizzly_rooms.py"),
    )
    check = ("python3", "fixture/final_check.py", "--check")
    for checks in (False, True):
        for index, refused in enumerate(chain[:3]):
            def refuse(command, _events):
                if command == refused:
                    raise subprocess.CalledProcessError(1, list(command))

            events = []
            try:
                full_generation_trace(events, refuse, checks=checks)
            except subprocess.CalledProcessError as error:
                assert error.cmd == list(refused)
            else:
                raise AssertionError("a refused native witness must stop regeneration")
            assert events.count(refused) == 1
            assert not any(command in events for command in chain[index + 1:])
            assert ("world-metadata",) not in events
            assert check not in events


def test_downstream_quest_producers_follow_inputs_and_precede_consumers() -> None:
    commands = downstream_quest_commands()
    native = ("python3", "tools/content-migration/register_spell_families.py")
    variant = ("python3", "tools/content-schema/reward-claim-authoring/reward_claim_variant_migration.py")
    check = ("python3", "fixture/final_check.py", "--check")

    def consume(command, events):
        if command == native:
            assert all(producer in events[:-1] for producer in commands[:2]), "native consumer needs final Quest bindings"
        if command == commands[-1]:
            # Replaying unpinned capture metadata is allowed; pinned semantic
            # feedback must remain exactly the evidence already in the Quest tree.
            write(rc.ROOT, "tools/content-schema/quest-authoring/samples/donor-source/npc-index.json", {"fresh": True})

    events = []
    assert full_generation_trace(events, consume, checks=True) == 0
    assert all(events.count(command) == 1 for command in commands)
    authored = [("python3", tool, "content") for tool in rc.AUTHORING_TOOLS]
    last_authored = max(i for i, command in enumerate(events) if command in authored)
    assert last_authored < events.index(commands[0]) < events.index(commands[1]) < events.index(native)
    final_inputs = max(events.index(native), events.index(("world-metadata",)), events.index(variant))
    indices = [events.index(command) for command in commands[2:]]
    assert indices == sorted(indices) and final_inputs < indices[0]
    assert indices[-1] < events.index(check), "freshness checks must see all six real producers"


def test_downstream_quest_producer_refusal_stops_later_work() -> None:
    commands = downstream_quest_commands()
    check = ("python3", "fixture/final_check.py", "--check")
    for index, refused in enumerate(commands):
        def refuse(command, _events):
            if command == refused:
                raise subprocess.CalledProcessError(1, list(command))

        events = []
        try:
            full_generation_trace(events, refuse, checks=True)
        except subprocess.CalledProcessError as error:
            assert error.cmd == list(refused)
        else:
            raise AssertionError("a failed Quest producer must stop regeneration")
        assert events.count(refused) == 1
        assert not any(command in events for command in commands[index + 1:])
        assert check not in events


def test_donor_feedback_guard_rejects_empty_duplicate_or_stale_inputs_before_capture() -> None:
    donor = downstream_quest_commands()[-1]
    index_path = "content/quests/definitions/index.json"
    for invalid in ("empty", "duplicate", "stale", "escape"):
        def prepare(root):
            index = json.loads((root / index_path).read_bytes())
            descriptor = index["authoring_sources"][0]
            if invalid == "empty":
                index["authoring_sources"] = []
            elif invalid == "duplicate":
                index["authoring_sources"].append(dict(descriptor))
            elif invalid == "stale":
                write(root, descriptor["path"], {"unexpected_source_change": True})
            else:
                descriptor["path"] = "tools/content-schema/quest-authoring/samples/donor-source/../outside.json"
            write(root, index_path, index)

        events = []
        try:
            full_generation_trace(events, lambda *_: None, prepare=prepare)
        except ValueError as error:
            assert "Quest donor feedback" in str(error)
        else:
            raise AssertionError("unproved donor feedback reached capture: " + invalid)
        assert donor not in events


def test_donor_feedback_guard_rejects_changed_source_and_repin_even_with_skip_checks() -> None:
    donor = downstream_quest_commands()[-1]
    index_path = "content/quests/definitions/index.json"
    check = ("python3", "fixture/final_check.py", "--check")
    for checks in (False, True):
        for repin in (False, True):
            def change_feedback(command, _events):
                if command != donor:
                    return
                index = json.loads((rc.ROOT / index_path).read_bytes())
                descriptor = index["authoring_sources"][0]
                write(rc.ROOT, descriptor["path"], {"unexpected_source_change": True})
                if repin:
                    descriptor["sha256"] = hashlib.sha256((rc.ROOT / descriptor["path"]).read_bytes()).hexdigest()
                    write(rc.ROOT, index_path, index)

            events = []
            try:
                full_generation_trace(events, change_feedback, checks=checks)
            except ValueError as error:
                assert "Quest donor feedback" in str(error)
            else:
                raise AssertionError("changed donor feedback was silently accepted")
            assert events.count(donor) == 1
            assert check not in events


def test_item_bootstrap_precedes_materializer_and_refusal_stops_world_work() -> None:
    store = ("python3", "tools/content-schema/store-authoring/build_runtime_catalogue.py", "--refresh-owner-offers")
    imbuement = ("python3", "tools/content-schema/imbuement-authoring/imbuement_authoring.py", "build")
    bootstrap = ("python3", "tools/content-migration/world_project_v2_to_tree.py", "--items-only")
    materializer = ("materialize-world",)
    tree = ("python3", "tools/content-migration/world_project_v2_to_tree.py")
    ordered = (store, imbuement, bootstrap, materializer, tree)
    events = []
    assert full_generation_trace(events, lambda *_: None) == 0
    assert all(events.count(step) == 1 for step in ordered)
    assert [events.index(step) for step in ordered] == sorted(events.index(step) for step in ordered)
    for refused in ordered[:3]:
        def refuse(command, _events):
            if command == refused:
                raise subprocess.CalledProcessError(1, list(command))

        events = []
        try:
            full_generation_trace(events, refuse)
        except subprocess.CalledProcessError as error:
            assert error.cmd == list(refused)
        else:
            raise AssertionError("a failed Item prerequisite must stop before materializing World")
        assert events.count(refused) == 1
        assert materializer not in events and tree not in events
        assert not any(command in events for command in downstream_quest_commands())


def item_index_fixture(root: Path) -> tuple[dict, list[str]]:
    shards = []
    for start, count in ((0, 500), (500, 2)):
        end = start + count - 1
        path = f"content/items/definitions/items-{start:05d}-{end:05d}.json"
        shards.append(path)
        write(root, path, {
            "schema": "OTERYN_ITEM_AUTHORING_SHARD/v1", "family": "Item",
            "source_legacy_role": "content/world/definitions/reference.json",
            "shard": {"index": start // 500, "start": start, "end": end, "count": count},
            "records": [{"definition": {"kind": "Item", "identity": {
                "family": "Item", "key": f"oteryn:item.fixture.i{ordinal}", "revision": "r1",
            }}, "authoring": {"preserved_fixture_fact": ordinal}}
                for ordinal in range(start, end + 1)],
        })
    index = {
        "schema": "OTERYN_FAMILY_INDEX/v1", "family": "Item",
        "record_count": 501, "shard_size": 500,
        "shards": [shards[0], "content/items/definitions/items-00500-00500.json"],
        "legacy_source": {"path": "content/world/definitions/reference.json", "git_blob_sha": "pinned-source"},
        "attached_authoring_entries": 7, "attached_editor_entries": 3,
        "attached_source_bindings": 5,
        "spell_imports": [{"path": "content/items/definitions/spell-native-profiles.json", "sha256": "pinned-overlay"}],
    }
    write(root, "content/items/index.json", index)
    return index, shards


def test_materializer_bootstraps_complete_item_index_before_running() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        index, shards = item_index_fixture(root)
        before = {path: (root / path).read_bytes() for path in shards}
        expected = {**index, "record_count": 502, "shards": shards}
        world = root / "content/world"
        world.mkdir()
        calls = []

        def materialize(command):
            calls.append(command)
            assert json.loads((root / "content/items/index.json").read_bytes()) == expected, \
                "materializer must receive the complete actual Item inventory"
            assert all((root / path).is_file() for path in expected["shards"])
            assert command[:7] == ["cargo", "run", "--locked", "-p", "oteryn-game-server", "--example", "materialize_content_world_project_v2"]
            output = Path(command[command.index("--output-root") + 1])
            output.mkdir()
            (output / "project.json").write_bytes(b"{}\n")

        with patch.object(rc, "ROOT", root), patch.object(rc, "WORLD", world), \
                patch.object(rc, "export_predecessor") as export, \
                patch.object(rc, "document_locators", return_value=["project.json"]), \
                patch.object(rc, "run", side_effect=materialize):
            rc.materialize_world()
        assert len(calls) == 1
        export.assert_called_once()
        assert (world / "project.json").read_bytes() == b"{}\n"
        assert {path: (root / path).read_bytes() for path in shards} == before
        # Only the three canonical inventory fields may change; source pins and
        # independently registered overlays remain the original evidence.
        actual = json.loads((root / "content/items/index.json").read_bytes())
        fields = {"record_count", "shard_size", "shards"}
        assert {key: value for key, value in actual.items() if key not in fields} == {
            key: value for key, value in index.items() if key not in fields
        }


def test_item_inventory_rejects_incomplete_or_ambiguous_shards_before_materialization() -> None:
    for invalid in ("gap", "overlap", "duplicate", "count", "bool", "name", "schema", "family", "empty"):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            _index, shards = item_index_fixture(root)
            tail = json.loads((root / shards[-1]).read_bytes())
            if invalid == "gap":
                (root / shards[0]).unlink()
            elif invalid == "overlap":
                write(root, "content/items/definitions/items-00500-00502.json", tail)
            elif invalid == "duplicate":
                first = json.loads((root / shards[0]).read_bytes())["records"][0]
                tail["records"][0] = first
            elif invalid == "count":
                tail["shard"]["count"] = 3
            elif invalid == "bool":
                tail["shard"]["index"] = True
            elif invalid == "name":
                (root / shards[-1]).rename(root / "content/items/definitions/items-tail.json")
            elif invalid == "schema":
                tail["schema"] = "unqualified"
            elif invalid == "family":
                tail["records"][0]["definition"]["identity"]["family"] = "Creature"
            else:
                for path in shards:
                    (root / path).unlink()
            if invalid in {"duplicate", "count", "bool", "schema", "family"}:
                write(root, shards[-1], tail)
            before = {path: path.read_bytes() for path in root.rglob("*.json")}
            with patch.object(rc, "ROOT", root), \
                    patch.object(rc, "export_predecessor") as export, \
                    patch.object(rc, "run") as run:
                try:
                    rc.materialize_world()
                except ValueError as error:
                    assert "ITEM_INDEX_" in str(error), str(error)
                else:
                    raise AssertionError("invalid Item inventory reached materialization: " + invalid)
            export.assert_not_called()
            run.assert_not_called()
            assert {path: path.read_bytes() for path in root.rglob("*.json")} == before


def test_item_inventory_emitter_matches_converter_format_and_explicit_output_set() -> None:
    from item_tree_index import item_index_fields
    from world_project_v2_to_tree import canonical_bytes

    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        index, shards = item_index_fixture(root)
        expected = {"record_count": 502, "shard_size": 500, "shards": shards}
        assert item_index_fields(root) == expected
        assert item_index_fields(root, shards) == expected
        assert canonical_bytes({**index, **item_index_fields(root)}) == canonical_bytes({**index, **expected})
        # The converter's explicit freshly written output list must exclude a
        # superseded tail even while that old file is still present;
        # bootstrap has no such list and must reject overlapping actual shards.
        stale = "content/items/definitions/items-00500-00500.json"
        write(root, stale, {"superseded": True})
        assert item_index_fields(root, shards) == expected
        try:
            item_index_fields(root)
        except ValueError as error:
            assert "ITEM_INDEX_" in str(error)
        else:
            raise AssertionError("bootstrap silently selected one of two tails")


def main() -> None:
    tests = [
        test_item_bootstrap_precedes_materializer_and_refusal_stops_world_work,
        test_normal_generation_orders_native_callback_producers,


        test_materializer_bootstraps_complete_item_index_before_running,
        test_item_inventory_rejects_incomplete_or_ambiguous_shards_before_materialization,
        test_item_inventory_emitter_matches_converter_format_and_explicit_output_set,
        test_callback_and_w01_witnesses_follow_final_provider_in_dependency_order,
        test_callback_or_w01_witness_refusal_stops_later_generation,
        test_rotten_death_witnesses_follow_final_native_provider_before_consumers,
        test_rotten_death_witness_refusal_stops_every_later_consumer,
        test_downstream_quest_producers_follow_inputs_and_precede_consumers,
        test_downstream_quest_producer_refusal_stops_later_work,
        test_donor_feedback_guard_rejects_empty_duplicate_or_stale_inputs_before_capture,
        test_donor_feedback_guard_rejects_changed_source_and_repin_even_with_skip_checks,
        test_imbuement_candidate_precedes_content_consumers_and_refusal_stops_them,
        test_reward_variant_packet_follows_final_claim_item_and_metadata_inputs,
        test_source_cohort_includes_installed_successors,
        test_normal_repin_updates_raw_extent_without_rewriting_history_checks,
        test_source_cohort_extent_separates_npc_profiles_and_tracks_successors,
        test_no_stage_checks_binary_conflicts_without_decoding,
        test_worker_conflict_scan_accepts_binary_and_rejects_unresolved_paths,
        test_import_conflict_fails_closed_and_keeps_both_sides,
        test_interactions_index_conflict_fails_closed,
        test_derived_only_conflict_is_resolved,
        test_world_pin_conflict_is_derived,
        test_world_pin_conflict_on_a_moved_entry_start_fails_closed,
        test_git_failure_while_enumerating_conflicts_fails_closed,
        test_spells_manifest_conflict_merges_keys_and_rehashes_pins,
        test_spells_manifest_conflict_on_one_key_fails_closed,
        test_callback_proof_refresh_preserves_reviewed_snapshots,
        test_unstaged_binary_conflict_does_not_require_text_decoding,
    ]
    for test in tests:
        test()
    import test_item_bootstrap

    suite = unittest.defaultTestLoader.loadTestsFromModule(test_item_bootstrap)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if not result.wasSuccessful():
        raise SystemExit(1)
    print(f"PASS {len(tests)} checks and {result.testsRun} Item bootstrap tests")


if __name__ == "__main__":
    main()
