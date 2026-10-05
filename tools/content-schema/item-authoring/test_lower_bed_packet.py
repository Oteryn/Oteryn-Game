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


def test_pair_maps_to_head_and_foot():
    canary = {
        10: bed("pillow", 11, "south", maletransformto=20),
        11: bed("blanket", 10, "north", femaletransformto=21),
        20: bed("pillow", 21, "south"),
        21: bed("blanket", 20, "north"),
        99: {"type": "container"},
    }
    rows, holds, counts = lower.build(canary, {10, 11, 20, 21, 99})
    by_key = {r["item_key"]: r for r in rows}
    assert by_key[key(10)] == {
        "item_key": key(10),
        "part": "head",
        "partner_direction": "south",
        "partner_item_key": key(11),
        "male_transform_to": key(20),
        "female_transform_to": None,
    }
    assert by_key[key(11)]["part"] == "foot"
    assert by_key[key(11)]["female_transform_to"] == key(21)
    assert holds == [] and counts["rows"] == 4 and key(99) not in by_key


def test_missing_and_foreign_facts_are_held_not_guessed():
    canary = {
        1: bed("pillow", None, "south"),
        2: bed("blanket", 77, "north"),
        3: bed("bolster", 4, "up", maletransformto=88),
        4: bed("blanket", 3, "down"),
    }
    rows, holds, _ = lower.build(canary, set(canary))
    reasons = {h["item_key"]: h["reasons"] for h in holds}
    assert reasons[key(1)] == ["bedpartof_missing"]
    assert reasons[key(2)] == ["partner_not_a_bed_item"]
    assert reasons[key(3)] == [
        "bedpart_unknown",
        "male_transform_to_not_a_bed_item",
        "partnerdirection_unknown",
    ]
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
