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
        approval = {"proof_inputs": proof, "approved_core_digests": ["core"], "approved_graphs": ["graph"]}
        receipt = {"proof_inputs": proof, "changes": ["reviewed"],
                   "approval": {"path": directory + "source-fix-approval.json", "sha256": "old"}}
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

def test_normal_generation_orders_native_callback_producers() -> None:
    commands: list[list[str]] = []
    with (
        patch.object(rc, "run", side_effect=commands.append),
        patch.multiple(rc, materialize_world=lambda: None,
                       refresh_quest_callback_proof=lambda: None,
                       refresh_source_cohort_extent=lambda: None,
                       repin_rust_inventory=lambda: None,
                       refresh_world_metadata=lambda: None,
                       AUTHORING_TOOLS=()),
        patch.object(sys, "argv", ["regenerate_content.py", "--skip-checks"]),
        patch.dict(os.environ, {"OTERYN_SPELL_SOURCE_ROOT": ""}),
        patch("builtins.print"),
    ):
        assert rc.main() == 0
    native = "tools/content-schema/native-gameplay/"
    tree = ["python3", "tools/content-migration/world_project_v2_to_tree.py"]
    pairs = [
        (["python3", native + producer], ["python3", native + consumer, "--refresh-native-pin"])
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


def main() -> None:
    tests = [
        test_normal_generation_orders_native_callback_producers,
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
    print(f"PASS {len(tests)} checks")


if __name__ == "__main__":
    main()
