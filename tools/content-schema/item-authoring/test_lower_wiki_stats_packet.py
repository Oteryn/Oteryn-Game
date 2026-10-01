"""No-network checks for `lower_wiki_stats_packet.py` (ITEM-SEM-2b-1).

Synthetic snapshots cover parsing, agreement between pages, the malformed and conflict
reports, the content-Item filter, weight in hundredths of an ounce and weapon typing; the
committed packet must rebuild byte for byte.
"""

from __future__ import annotations

import hashlib
import json

import lower_wiki_stats_packet as lower


def snapshot(records):
    return {
        "batch_id": "test",
        "snapshot_sha256": "0" * 64,
        "records": {
            f"oteryn:item.tibia.i{item_id}": {
                "item_id": item_id,
                "observations": [
                    {"page_id": page_id, "revision_id": 1, "fields": fields}
                    for page_id, fields in pages
                ],
            }
            for item_id, pages in records.items()
        },
    }


def rows_by(rows):
    return {(row["item_key"], row["field_path"]): row["typed_value"] for row in rows}


def test_lowering_types_values():
    rows, report, _counts = lower.build(
        snapshot(
            {
                34086: [
                    (
                        1,
                        {
                            "attack": "6",
                            "defensemod": "+3",
                            "ice_attack": "46",
                            "fire_attack": "2",
                            "imbueslots": "2",
                            "weapontype": "Club",
                            "weight": "41.00",
                        },
                    )
                ],
                3074: [(2, {"primarytype": "Wands", "weight": "0.50"})],
            }
        ),
        {34086, 3074},
    )
    got = rows_by(rows)
    key = "oteryn:item.tibia.i34086"
    assert got[(key, "weapon.attack")] == {"kind": "SIGNED_POINTS", "value": 6}
    assert got[(key, "weapon.extra_defense")] == {"kind": "SIGNED_POINTS", "value": 3}
    assert got[(key, "weapon.elemental")]["value"] == [
        {"element": "FIRE", "points": 2},
        {"element": "ICE", "points": 46},
    ]
    assert got[(key, "physical.weight")] == {"kind": "WEIGHT_CENTI_OZ", "value": 4100}
    assert got[(key, "weapon.weapon_type")]["value"] == "CLUB"
    wand = "oteryn:item.tibia.i3074"
    assert got[(wand, "weapon.weapon_type")]["value"] == "WAND"
    assert got[(wand, "physical.weight")]["value"] == 50
    assert not report["malformed"] and not report["conflict"], report


def test_disagreement_malformed_and_non_items():
    rows, report, _counts = lower.build(
        snapshot(
            {
                1: [(1, {"weight": "1.00"}), (2, {"weight": "2.00"})],
                2: [(3, {"weight": "0.5"}), (4, {"range": "?"})],
                3: [(5, {"weight": "1.00"}), (6, {"weight": "1.00"})],
                4: [(7, {"attack": "9"})],
            }
        ),
        {1, 2, 3},
    )
    got = rows_by(rows)
    assert ("oteryn:item.tibia.i1", "physical.weight") not in got
    assert report["conflict"]["physical.weight"] == 1, report
    assert (
        report["malformed"]["physical.weight"] == 1
        and report["malformed"]["weapon.range_cells"] == 1
    )
    agreed = got[("oteryn:item.tibia.i3", "physical.weight")]
    assert agreed["value"] == 100
    sources = next(row for row in rows if row["item_key"] == "oteryn:item.tibia.i3")[
        "sources"
    ]
    assert [source["page_id"] for source in sources] == [5, 6]
    assert not any(row["item_key"] == "oteryn:item.tibia.i4" for row in rows), (
        "not an Item"
    )


def test_committed_packet_rebuilds():
    committed = lower.OUTPUT.read_bytes()
    snap = json.loads(lower.SNAPSHOT.read_text(encoding="utf-8"))
    compiler_sha256 = hashlib.sha256(
        (lower.ROOT / lower.COMPILER_PATH).read_bytes()
    ).hexdigest()
    rebuilt = lower.packet_bytes(snap, lower.content_item_ids(), compiler_sha256)
    assert rebuilt == committed, "committed packet drifted"
    packet = json.loads(committed)
    assert packet["source"]["snapshot_sha256"] == snap["snapshot_sha256"]
    # ITEM-ADD-1 (owner 2a): appearance-only Items (no source binding) take no stats.
    for item_id in (40522, 53197):
        assert item_id not in lower.content_item_ids()
        assert not any(
            row["item_key"] == f"oteryn:item.tibia.i{item_id}"
            for row in packet["promotions"]
        )
    return packet["counts"]


def main():
    test_lowering_types_values()
    test_disagreement_malformed_and_non_items()
    counts = test_committed_packet_rebuilds()
    print(
        f"lower_wiki_stats_packet tests: PASS fields={counts['fields']} items={counts['items']}"
    )


if __name__ == "__main__":
    main()
