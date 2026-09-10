#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
from pathlib import Path
from types import SimpleNamespace
import tempfile

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
            if missing: unresolved.add(entry.server_id)
            presentations.append({
                "appearance_source_id": entry.server_id,
                "export_record_id": f"p-{tile.position.x}-{tile.position.y}-{order}",
                "presentation_order": {"order": order, "plane": 0},
                "resolved_primitives": [] if missing else [{"sprite_source_id": entry.server_id + 1000, "visual_coverage_offsets": [{"dx_tiles": -1, "dy_tiles": 0}] if entry.server_id == 2 else [{"dx_tiles": 0, "dy_tiles": 0}]}],
                **({"presentation_resolution_state": "UNRESOLVED_APPEARANCE"} if missing else {}),
            })
            if not missing: sprite_ids.add(entry.server_id + 1000)
        record = {"position": {"floor": -tile.position.z, "x": tile.position.x, "y": tile.position.y}, "presentation": presentations}
        data = (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()
        return data, {"appearance_ids": appearance_ids, "sprite_ids": sprite_ids, "presentation_count": len(presentations), "unresolved_appearance_ids": unresolved, "unresolved_presentation_count": len(unresolved)}


def test_boundaries_floor_order_and_losslessness():
    window = census.Window("x", 10, 20, -7, "s", "r")
    records = [
        Tile(9, 20, 7, [item(8)]), Tile(10, 20, 7, [item(2), item(1)]),
        Tile(41, 51, 7, [item(999)]), Tile(42, 51, 7, [item(7)]),
        Tile(10, 20, 8, [item(6)]),
        Town(3, "inside", 10, 20, 7), Waypoint("outside-floor", 10, 20, 8),
    ]
    summary = census.measure_window(Producer(), object(), window, records)
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


def test_transition_structure_and_determinism():
    dest = SimpleNamespace(x=8, y=9, z=6)
    nested = item(4, house_door_id=2)
    visible = item(3, action_id=50, unique_id=60, teleport_destination=dest, children=(nested,))
    window = census.Window("x", 0, 0, 0, "s", "r")
    one = census.measure_window(Producer(), object(), window, [Tile(0, 0, 0, [visible])])
    two = census.measure_window(Producer(), object(), window, [Tile(0, 0, 0, [visible])])
    assert census.canonical_summary_bytes(one) == census.canonical_summary_bytes(two)
    observations = one["transition_like_records"]
    assert [x["structural_kind"] for x in observations] == ["ACTION_ID", "HOUSE_DOOR_ID", "TELEPORT_DESTINATION", "UNIQUE_ID"]


def test_checked_overflow_and_identity_failure():
    try:
        census.checked_add(census.U64_MAX, 1, "test")
    except census.CensusError as exc:
        assert "overflow" in str(exc)
    else: raise AssertionError("overflow accepted")
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "input"
        path.write_bytes(b"wrong")
        try: census.require_sha256(path, "0" * 64, "fixture")
        except census.CensusError as exc: assert "digest mismatch" in str(exc)
        else: raise AssertionError("digest mismatch accepted")


def test_expansion_is_explicitly_ambiguous():
    window = census.Window("x", 64, 96, -7, "s", "r")
    summary = census.measure_window(Producer(), object(), window, [Tile(64, 96, 7, [item(1)])])
    assert summary["expansion_order"] == []
    assert summary["clipping"] == [{"boundary": "north+west", "classification": "AMBIGUOUS_EXPANSION", "reason": "source tile records occupy the bounded edge, but Phase-A source structure alone does not prove which adjacent shard is semantically required"}]


def main():
    test_boundaries_floor_order_and_losslessness()
    test_transition_structure_and_determinism()
    test_checked_overflow_and_identity_failure()
    test_expansion_is_explicitly_ambiguous()
    print("reference-world-corridor-census self-test: PASS")
    return 0

if __name__ == "__main__": raise SystemExit(main())
