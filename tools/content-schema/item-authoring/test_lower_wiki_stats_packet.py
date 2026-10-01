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
                2: [(3, {"weight": "0.501"}), (4, {"range": "?"})],
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


def test_equipment_requirements_and_holds():
    fields = {
        "slot": "Both Hands",
        "hands": "Two",
        "levelrequired": "400",
        "vocrequired": "knights and paladins",
    }
    rows, report, _ = lower.build(snapshot({1: [(1, fields)]}), {1})
    pattern = rows_by(rows)[("oteryn:item.tibia.i1", "equipment.patterns")]["value"][0]
    assert pattern["primary_slot"] == lower.known("WEAPON")
    assert pattern["additional_reserved_slots"] == lower.known(["SHIELD"])
    assert pattern["level"] == lower.known(400)
    assert pattern["vocations"] == lower.known(["KNIGHT", "PALADIN"])
    assert pattern["mutually_exclusive_groups"] == {"state": "UNKNOWN"}
    assert not report["malformed"]
    _, missing = lower.equipment({"slot": "Head"})
    assert missing["value"][0]["vocations"] == {"state": "UNKNOWN"}
    assert missing["value"][0]["level"] == {"state": "UNKNOWN"}
    assert lower.equipment({"primarytype": "Attack Runes", "levelrequired": "27"}) == (
        None,
        None,
    )
    assert lower.equipment({"primarytype": "Ammunition", "slot": "Extra Slot"}) == (
        None,
        None,
    )
    for override in (
        {"hands": "One"},
        {"vocrequired": "without"},
        {"vocrequired": "knights and without"},
        {"levelrequired": "65536"},
    ):
        assert lower.equipment(fields | override)[1] == "MALFORMED"
    rows, report, _ = lower.build(
        snapshot(
            {
                1: [(1, fields), (2, fields | {"levelrequired": "300"})],
                2854: [(3, {"slot": "Container"})],
            }
        ),
        {1, 2854},
    )
    assert not rows and report["conflict"]["equipment.patterns"] == 1
    for primary in ("Attack Runes", "Ammunition"):
        excluded = {"primarytype": primary, "slot": "Extra Slot", "levelrequired": "27"}
        for pages in ([(1, fields), (2, excluded)], [(2, excluded), (1, fields)]):
            rows, report, _ = lower.build(snapshot({1: pages}), {1})
            assert not any(row["field_path"] == "equipment.patterns" for row in rows)
            assert report["equipment_holds"][0]["classification"] == "CONFLICT"
            assert {
                row["page_id"] for row in report["equipment_holds"][0]["sources"]
            } == {1, 2}
        _, report, _ = lower.build(snapshot({1: [(2, excluded)]}), {1})
        assert not report["equipment_holds"]


def test_charges_and_duration_qualification():
    key = "oteryn:item.tibia.i1"
    definition = {
        "semantics": {"charges": {"state": "UNKNOWN"}, "temporal": {"state": "UNKNOWN"}}
    }
    snap = snapshot({1: [(1, {"charges": "250", "duration": "7.5 minutes"})]})
    rows, report, _ = lower.build(snap, {1}, {key: definition})
    got = rows_by(rows)
    assert got[(key, "charges.count")] == {"kind": "COUNT_U32", "value": 250}
    assert got[(key, "temporal.duration")] == {"kind": "DURATION_MS", "value": 450000}
    assert not report["physical_field_holds"]
    assert lower.duration("1 hour") == lower.duration("60 minutes") == 3600000
    assert lower.duration("19 days") == 1641600000
    for raw in (
        "unknown",
        "1 second",
        "0 minutes",
        "1/2 hours",
        "0.000001 minutes",
        "999999999999999999999 days",
    ):
        assert lower.duration(raw) is None
    for raw in ("0", "-1", "4294967296", "unknown"):
        assert lower.positive_count(raw) is None
    assert lower.positive_count("4294967295") == 4294967295
    try:
        lower.packet_bytes(snap, {1}, "0" * 64)
    except ValueError as error:
        assert "wiki snapshot digest mismatch" in str(error)
    else:
        raise AssertionError("modified snapshot records kept a stale source digest")
    assert lower.build(snapshot({1: [(1, {})]}), {1}, {key: definition})[0] == []
    for field, member, raw in (
        ("charges", "count", "250"),
        ("duration", "duration", "10 minutes"),
    ):
        group = "charges" if field == "charges" else "temporal"
        for state in ("CONFLICT", "NOT_APPLICABLE"):
            blocked = {"semantics": {group: {"state": state}}}
            _, held, _ = lower.build(
                snapshot({1: [(1, {field: raw})]}), {1}, {key: blocked}
            )
            assert held["physical_field_holds"][0]["reason"] == "BLOCKED_EVIDENCE_STATE"
            blocked["semantics"][group] = {
                "state": "KNOWN",
                "value": {member: {"state": state}},
            }
            _, held, _ = lower.build(
                snapshot({1: [(1, {field: raw})]}), {1}, {key: blocked}
            )
            assert held["physical_field_holds"][0]["reason"] == "BLOCKED_EVIDENCE_STATE"
        for value, reason in (
            (1, "KNOWN_FIELD_CONFLICT"),
            (250 if field == "charges" else 600000, None),
        ):
            known = {
                "semantics": {
                    group: {"state": "KNOWN", "value": {member: lower.known(value)}}
                }
            }
            rows, held, _ = lower.build(
                snapshot({1: [(1, {field: raw})]}), {1}, {key: known}
            )
            assert bool(rows) == (reason is None)
            if reason:
                assert held["physical_field_holds"][0]["reason"] == reason
    for pages, reason in (
        (
            [(1, {"duration": "10 minutes"}), (2, {"duration": "20 minutes"})],
            "WIKI_PAGE_DISAGREEMENT",
        ),
        ([(1, {"duration": "unknown"})], "MALFORMED_WIKI_VALUE"),
    ):
        rows, held, _ = lower.build(snapshot({1: pages}), {1}, {key: definition})
        assert not rows and held["physical_field_holds"][0]["reason"] == reason
    agreed = snapshot(
        {1: [(1, {"duration": "1 hour"}), (2, {"duration": "60 minutes"})]}
    )
    rows, held, _ = lower.build(agreed, {1}, {key: definition})
    assert rows_by(rows)[(key, "temporal.duration")]["value"] == 3600000
    assert not held["physical_field_holds"]
    # A source ID outside the candidate and binding sets still makes a page shared.
    shared = snapshot({1: [(1, {"duration": "10 minutes"})], 999: [(1, {})]})
    rows, held, _ = lower.build(shared, {1}, {key: definition})
    assert not rows and held["physical_field_holds"][0]["sources"][0][
        "page_item_ids"
    ] == [1, 999]
    assert (
        held["physical_field_holds"][0]["reason"] == "SHARED_PAGE_VARIANT_UNQUALIFIED"
    )
    rows, held, _ = lower.build(snap, {1}, {key: definition}, {key})
    assert not rows and {row["reason"] for row in held["physical_field_holds"]} == {
        "EXISTING_MAP_OWNER"
    }


def main():
    for raw, expected in (("22", 2200), ("0.5", 50), ("1.3", 130), ("41.00", 4100)):
        assert lower.weight(raw) == expected
    for raw in ("0.501", "1.", "1e2", "-1", "unknown", "1,30"):
        assert lower.weight(raw) is None
    test_lowering_types_values()
    test_disagreement_malformed_and_non_items()
    test_equipment_requirements_and_holds()
    test_charges_and_duration_qualification()
    counts = test_committed_packet_rebuilds()
    print(
        f"lower_wiki_stats_packet tests: PASS fields={counts['fields']} items={counts['items']}"
    )


if __name__ == "__main__":
    main()
