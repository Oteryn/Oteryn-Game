"""No-network checks for `lower_bed_packet.py` (BED-CONTENT-1, ITEM-SEM-BED-PACKET-1 §1.5).

Synthetic Canary data covers the part/direction mapping, the occupied fallback, the "no change"
targets (item 743 against the real source), the holds and their set-rule cascade; the committed
packet must rebuild byte for byte.
"""

from __future__ import annotations

import json

import lower_bed_packet as lower
from lower_equip_abilities_packet import canary_pin, load_canary_top_level
from lower_wiki_stats_packet import content_item_ids


def bed(part, direction, **targets):
    attrs = {"type": "bed", "bedpart": part, "partnerdirection": direction}
    attrs.update({k: str(v) for k, v in targets.items()})
    return attrs


def key(item_id):
    return f"oteryn:item.tibia.i{item_id}"


def value(promotions, item_id):
    (row,) = [p for p in promotions if p["item_key"] == key(item_id)]
    assert row["field_path"] == "bed" and row["typed_value"]["kind"] == "BED"
    return row["typed_value"]["value"]


def test_part_direction_and_occupied_fallback():
    canary = {
        10: bed("pillow", "south", maletransformto=20),
        11: bed("blanket", "north", femaletransformto=21),
        20: bed("pillow", "south"),
        21: bed("blanket", "north"),
        99: {"type": "container"},
    }
    promotions, holds, _, counts = lower.build(canary, {10, 11, 20, 21, 99})
    assert value(promotions, 10) == {
        "part": "HEAD",
        "partner_direction": "SOUTH",
        "occupied_male": key(20),
        "occupied_female": key(20),  # an absent female key falls back to the male one
    }
    assert value(promotions, 11)["part"] == "FOOT"
    assert value(promotions, 20)["occupied_male"] == key(20)  # no target: no change
    assert holds == [] and counts["items"] == 4 and counts["fields"] == 4


def test_non_bed_and_zero_targets_mean_no_change_and_are_listed():
    canary = {
        1: bed("pillow", "south", maletransformto=0, femaletransformto=88),
        88: {"type": "trashholder"},
        2: bed("blanket", "north", maletransformto=88, femaletransformto=1),
        3: bed("blanket", "north"),
    }
    promotions, holds, no_change, _ = lower.build(canary, {1, 2, 3, 88})
    one = value(promotions, 1)
    assert one["occupied_male"] == key(1) and one["occupied_female"] == key(1)
    # the foot's female target is a head: another part, so the foot is held, not lowered
    assert [h["item_key"] for h in holds] == [key(2)]
    assert {
        "item_key": key(1),
        "field": "occupied_female",
        "source_target": "88",
        "source_type": "trashholder",
    } in no_change


def test_unknown_part_or_direction_or_missing_definition_is_held():
    canary = {
        3: bed("bolster", "south"),
        4: bed("blanket", "up"),
        5: bed("pillow", "east"),
    }
    promotions, holds, _, _ = lower.build(canary, {3, 4})
    assert promotions == []
    assert {h["item_key"]: h["reasons"] for h in holds} == {
        key(3): ["bedpart_unknown"],
        key(4): ["partnerdirection_unknown"],
        key(5): ["no_content_definition"],
    }


def test_set_rule_holds_the_source_of_a_held_target():
    canary = {
        1: bed("pillow", "south", maletransformto=2),
        2: bed("pillow", "north", maletransformto=1),  # direction differs from 1
        3: bed("pillow", "north", maletransformto=1),
    }
    promotions, holds, _, _ = lower.build(canary, set(canary))
    assert promotions == [] and len(holds) == 3


def test_item_743_female_target_is_a_trashholder_so_no_change():
    canary = load_canary_top_level()
    assert canary[727]["type"] != "bed"
    promotions, _, no_change, _ = lower.build(canary, content_item_ids())
    assert value(promotions, 743)["occupied_female"] == key(743)
    assert any(
        n["item_key"] == key(743) and n["source_target"] == "727" for n in no_change
    )


def test_committed_packet_rebuilds():
    data = lower.packet_bytes(load_canary_top_level(), content_item_ids(), canary_pin())
    assert lower.OUTPUT.read_bytes() == data
    assert json.loads(data)["schema"] == lower.SCHEMA


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("PASS")
