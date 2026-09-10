#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sys
import tempfile
from types import ModuleType, SimpleNamespace

import census


class Tile:
    def __init__(self, x, y, z, visible):
        self.position = SimpleNamespace(x=x, y=y, z=z)
        self.ground = visible[0] if visible else None
        self.items = tuple(visible[1:])
        self.house_id = None
        self.flags = 0


class Town:
    def __init__(self, town_id, name, x, y, z):
        self.town_id, self.name = town_id, name
        self.temple = SimpleNamespace(x=x, y=y, z=z)


class Waypoint:
    def __init__(self, name, x, y, z):
        self.name = name
        self.position = SimpleNamespace(x=x, y=y, z=z)


def item(server_id, **kwargs):
    defaults = dict(action_id=None, unique_id=None, teleport_destination=None, house_door_id=None, children=())
    defaults.update(kwargs)
    return SimpleNamespace(server_id=server_id, **defaults)


class Producer:
    PRODUCER_API = "fake-v0"

    def is_tile(self, _runtime, value): return isinstance(value, Tile)
    def is_town(self, _runtime, value): return isinstance(value, Town)
    def is_waypoint(self, _runtime, value): return isinstance(value, Waypoint)
    def native_floor(self, value): return -value.position.z

    def project_tile_bytes(self, _runtime, tile):
        presentations = []
        appearance_ids, sprite_ids, unresolved = set(), set(), set()
        for order, entry in enumerate(([tile.ground] if tile.ground else []) + list(tile.items)):
            appearance_ids.add(entry.server_id)
            missing = entry.server_id == 999
            if missing:
                unresolved.add(entry.server_id)
            presentations.append({
                "appearance_source_id": entry.server_id,
                "export_record_id": f"p-{tile.position.x}-{tile.position.y}-{order}",
                "presentation_order": {"order": order, "plane": 0},
                "resolved_primitives": [] if missing else [{
                    "sprite_source_id": entry.server_id + 1000,
                    "visual_coverage_offsets": (
                        [{"dx_tiles": -1, "dy_tiles": 0}]
                        if entry.server_id == 2 else [{"dx_tiles": 0, "dy_tiles": 0}]
                    ),
                }],
                **({"presentation_resolution_state": "UNRESOLVED_APPEARANCE"} if missing else {}),
            })
            if not missing:
                sprite_ids.add(entry.server_id + 1000)
        record = {
            "position": {"floor": -tile.position.z, "x": tile.position.x, "y": tile.position.y},
            "presentation": presentations,
        }
        data = (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()
        return data, {
            "appearance_ids": appearance_ids,
            "sprite_ids": sprite_ids,
            "presentation_count": len(presentations),
            "unresolved_appearance_ids": unresolved,
            "unresolved_presentation_count": len(unresolved),
        }


def test_boundaries_floor_order_losslessness_and_stream_framing():
    producer = Producer()
    window = census.Window("x", 10, 20, -7, "s", "r")
    selected = [Tile(10, 20, 7, [item(2), item(1)]), Tile(41, 51, 7, [item(999)])]
    records = [
        Tile(9, 20, 7, [item(8)]), selected[0], selected[1], Tile(42, 51, 7, [item(7)]),
        Tile(10, 20, 8, [item(6)]), Town(3, "inside", 10, 20, 7), Waypoint("outside-floor", 10, 20, 8),
    ]
    summary = census.measure_window(producer, object(), window, records)
    encoded = [producer.project_tile_bytes(object(), tile)[0] for tile in selected]
    stream = b"".join(encoded)

    assert summary["bounds"] == {"x_min": 10, "x_max_exclusive": 42, "y_min": 20, "y_max_exclusive": 52}
    assert summary["tile_records"] == 2
    assert summary["ordered_presentations"] == 3
    assert summary["max_presentations_per_cell"] == 2
    assert summary["unresolved_presentation_count"] == 1
    assert summary["unresolved_appearance_ids"] == [999]
    assert len(summary["unique_resolved_presentation_ids"]) == 2
    assert summary["unique_appearance_source_ids"] == [1, 2, 999]
    assert len(summary["candidate_composite_diagnostics"]) == 1
    assert [x["kind"] for x in summary["source_landmarks"]] == ["town"]
    assert summary["aggregate_encoded_byte_count"] == len(stream) == sum(map(len, encoded))
    assert summary["max_record_encoded_bytes"] == max(map(len, encoded))
    assert summary["ordered_stream_sha256"] == hashlib.sha256(stream).hexdigest()
    assert summary["boundary_occupancy"] == ["east", "north", "south", "west"]
    assert summary["boundary_occupancy_classification"] == "EDGE_OCCUPANCY_ONLY_NOT_CLIPPING_PROOF"
    assert summary["window_expansion_required"] == []
    assert summary["window_result"] == "PASS"
    assert not any(isinstance(value, bytes) for value in summary.values())


def test_empty_window_has_canonical_empty_stream():
    window = census.Window("empty", 10, 20, -7, "s", "r")
    summary = census.measure_window(Producer(), object(), window, [])

    assert summary["tile_records"] == 0
    assert summary["ordered_presentations"] == 0
    assert summary["aggregate_encoded_byte_count"] == 0
    assert summary["max_record_encoded_bytes"] == 0
    assert summary["ordered_stream_sha256"] == hashlib.sha256(b"").hexdigest()
    assert summary["window_expansion_required"] == []
    assert summary["window_result"] == "PASS"


def test_transition_structure_and_determinism():
    dest = SimpleNamespace(x=8, y=9, z=6)
    nested = item(4, house_door_id=2)
    visible = item(3, action_id=50, unique_id=60, teleport_destination=dest, children=(nested,))
    window = census.Window("x", 0, 0, 0, "s", "r")
    one = census.measure_window(Producer(), object(), window, [Tile(0, 0, 0, [visible])])
    two = census.measure_window(Producer(), object(), window, [Tile(0, 0, 0, [visible])])
    assert census.canonical_summary_bytes(one) == census.canonical_summary_bytes(two)
    observations = one["transition_like_records"]
    assert [x["structural_kind"] for x in observations] == [
        "ACTION_ID", "HOUSE_DOOR_ID", "TELEPORT_DESTINATION", "UNIQUE_ID"
    ]


def test_checked_overflow_and_digest_failure():
    try:
        census.checked_add(census.U64_MAX, 1, "test")
    except census.CensusError as exc:
        assert "overflow" in str(exc)
    else:
        raise AssertionError("overflow accepted")

    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "input"
        path.write_bytes(b"wrong")
        try:
            census.require_sha256(path, "0" * 64, "fixture")
        except census.CensusError as exc:
            assert "digest mismatch" in str(exc)
        else:
            raise AssertionError("digest mismatch accepted")


def test_edge_occupancy_is_not_automatic_expansion():
    window = census.Window("x", 64, 96, -7, "f-7-r3-c2", "lookup")
    summary = census.measure_window(Producer(), object(), window, [Tile(64, 96, 7, [item(1)])])
    assert summary["boundary_occupancy"] == ["north", "west"]
    assert summary["window_result"] == "PASS"
    assert summary["window_expansion_required"] == []


def test_expansion_requirement_is_exact_and_fail_closed():
    window = census.Window("x", 64, 96, -7, "f-7-r3-c2", "lookup")
    request = census.expansion_requirement(
        window, x=63, y=96, floor=-7, reason="exercised path fixture crosses west boundary"
    )
    assert request == {
        "classification": "WINDOW_EXPANSION_REQUIRED",
        "reason": "exercised path fixture crosses west boundary",
        "proposed_floor": -7,
        "proposed_semantic_shard": "f-7-r3-c1",
    }
    try:
        census.expansion_requirement(window, x=64, y=96, floor=-7, reason="inside")
    except census.CensusError as exc:
        assert "outside" in str(exc)
    else:
        raise AssertionError("inside point accepted as expansion")


def test_preexisting_parser_module_fails_closed():
    name = "tools.otbm_atlas.synthetic_contamination"
    previous = sys.modules.get(name)
    sys.modules[name] = ModuleType(name)
    try:
        try:
            census.require_fresh_legacy_import_context()
        except census.CensusError as exc:
            assert "LEGACY_PARSER_REVISION_MISMATCH" in str(exc)
        else:
            raise AssertionError("pre-existing parser module accepted")
    finally:
        if previous is None:
            sys.modules.pop(name, None)
        else:
            sys.modules[name] = previous


def test_checkout_identity_and_dirty_state_fail_closed():
    original_git = census._git
    original_revision = census.LEGACY_REVISION
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp).resolve()
        revision = "a" * 40
        census.LEGACY_REVISION = revision

        def clean_git(_repo, *args):
            if args == ("rev-parse", "--show-toplevel"):
                return str(root)
            if args == ("rev-parse", "HEAD"):
                return revision
            if args == ("status", "--porcelain=v1", "--untracked-files=all"):
                return ""
            raise AssertionError(args)

        census._git = clean_git
        assert census.verify_legacy_checkout(root) == root

        def dirty_git(repo, *args):
            if args == ("status", "--porcelain=v1", "--untracked-files=all"):
                return "?? injected.py"
            return clean_git(repo, *args)

        census._git = dirty_git
        try:
            census.verify_legacy_checkout(root)
        except census.CensusError as exc:
            assert "LEGACY_PARSER_REVISION_MISMATCH" in str(exc)
        else:
            raise AssertionError("dirty legacy worktree accepted")

    census._git = original_git
    census.LEGACY_REVISION = original_revision


def test_loaded_module_origin_outside_pinned_tree_fails_closed():
    original_git = census._git
    saved = {name: sys.modules.get(name) for name in (
        "tools", "tools.otbm_atlas", "tools.otbm_atlas.assets", "tools.otbm_atlas.semantic"
    )}
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp).resolve()
        inside = root / "tools" / "otbm_atlas"
        inside.mkdir(parents=True)
        outside = root.parent / "outside-semantic.py"

        for name, path in (
            ("tools", root / "tools" / "__init__.py"),
            ("tools.otbm_atlas", inside / "__init__.py"),
            ("tools.otbm_atlas.assets", inside / "assets.py"),
            ("tools.otbm_atlas.semantic", outside),
        ):
            module = ModuleType(name)
            module.__file__ = str(path)
            sys.modules[name] = module

        def fake_git(_repo, *args):
            if args[0] == "ls-files":
                return args[-1]
            if args[0] == "hash-object":
                return "b" * 40
            if args[0] == "rev-parse":
                return "b" * 40
            raise AssertionError(args)

        census._git = fake_git
        try:
            census.verify_loaded_legacy_modules(root)
        except census.CensusError as exc:
            assert "LEGACY_PARSER_REVISION_MISMATCH" in str(exc)
            assert "outside" in str(exc)
        else:
            raise AssertionError("outside parser module accepted")

        sys.modules["tools.otbm_atlas.semantic"].__file__ = str(inside / "semantic.py")

        def blob_mismatch_git(_repo, *args):
            if args[0] == "ls-files":
                return args[-1]
            if args[0] == "hash-object":
                return "a" * 40
            if args[0] == "rev-parse":
                return "b" * 40
            raise AssertionError(args)

        census._git = blob_mismatch_git
        try:
            census.verify_loaded_legacy_modules(root)
        except census.CensusError as exc:
            assert "LEGACY_PARSER_REVISION_MISMATCH" in str(exc)
            assert "blob mismatch" in str(exc)
        else:
            raise AssertionError("loaded parser blob mismatch accepted")

    census._git = original_git
    for name, previous in saved.items():
        if previous is None:
            sys.modules.pop(name, None)
        else:
            sys.modules[name] = previous


def main():
    test_boundaries_floor_order_losslessness_and_stream_framing()
    test_empty_window_has_canonical_empty_stream()
    test_transition_structure_and_determinism()
    test_checked_overflow_and_digest_failure()
    test_edge_occupancy_is_not_automatic_expansion()
    test_expansion_requirement_is_exact_and_fail_closed()
    test_preexisting_parser_module_fails_closed()
    test_checkout_identity_and_dirty_state_fail_closed()
    test_loaded_module_origin_outside_pinned_tree_fails_closed()
    print("reference-world-corridor-census self-test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
