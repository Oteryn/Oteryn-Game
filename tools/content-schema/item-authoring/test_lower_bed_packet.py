"""No-network checks for `lower_bed_packet.py` (BED-CONTENT-1).

Synthetic Canary data covers the head/foot mapping, the partner and transform keys, the holds and
the content filter; the committed packet must rebuild byte for byte.
"""

from __future__ import annotations

import json

import lower_bed_packet as lower
from lower_equip_abilities_packet import canary_pin, load_canary_top_level
from lower_wiki_stats_packet import content_item_ids


def bed(part, of, direction, **extra):
    attrs = {"type": "bed", "bedpart": part, "partnerdirection": direction}
    if of is not None:
        attrs["bedpartof"] = str(of)
    attrs.update({k: str(v) for k, v in extra.items()})
    return attrs


def key(item_id):
    return f"oteryn:item.tibia.i{item_id}"


def test_pair_maps_to_head_and_foot_with_occupied_fallback():
    canary = {
        10: bed("pillow", 11, "south", maletransformto=20),
        11: bed("blanket", 10, "north", femaletransformto=21),
        20: bed("pillow", 21, "south"),
        21: bed("blanket", 20, "north"),
        99: {"type": "container"},
    }
    rows, holds, counts = lower.build(canary, {10, 11, 20, 21, 99})
    by_key = {r["item_key"]: r for r in rows}
    # a missing own-sex key falls back to the other sex's key
    assert by_key[key(10)] == {
        "item_key": key(10),
        "part": "head",
        "partner_direction": "south",
        "occupied_male": key(20),
        "occupied_female": key(20),
    }
    assert by_key[key(11)]["part"] == "foot"
    assert by_key[key(11)]["occupied_male"] == key(21)
    assert by_key[key(20)]["occupied_male"] is None  # no target: no change
    assert holds == [] and counts["rows"] == 4 and key(99) not in by_key


def test_zero_and_non_bed_targets_mean_no_change():
    canary = {
        1: bed("pillow", None, "south", maletransformto=0, femaletransformto=88),
        2: bed("blanket", None, "north", maletransformto=88, femaletransformto=1),
    }
    rows, holds, _ = lower.build(canary, set(canary))
    by_key = {r["item_key"]: r for r in rows}
    assert (
        by_key[key(1)]["occupied_male"] is None
        and by_key[key(1)]["occupied_female"] is None
    )
    assert (
        by_key[key(2)]["occupied_male"] == key(1) == by_key[key(2)]["occupied_female"]
    )
    assert holds == []


def test_unknown_part_or_direction_is_held_not_guessed():
    canary = {3: bed("bolster", 4, "up"), 4: bed("blanket", 3, "down")}
    rows, holds, _ = lower.build(canary, set(canary))
    assert {h["item_key"]: h["reasons"] for h in holds} == {
        key(3): ["bedpart_unknown", "partnerdirection_unknown"],
        key(4): ["partnerdirection_unknown"],
    }
    assert {r["item_key"]: r["part"] for r in rows}[key(3)] is None


def test_items_without_a_definition_are_counted_not_emitted():
    canary = {5: bed("pillow", 6, "east"), 6: bed("blanket", 5, "west")}
    rows, _, counts = lower.build(canary, {5})
    assert [r["item_key"] for r in rows] == [key(5)] and counts["not_in_content"] == 1


def test_committed_packet_rebuilds():
    data = lower.packet_bytes(load_canary_top_level(), content_item_ids(), canary_pin())
    assert lower.OUTPUT.read_bytes() == data
    assert json.loads(data)["schema"] == lower.SCHEMA


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("PASS")
