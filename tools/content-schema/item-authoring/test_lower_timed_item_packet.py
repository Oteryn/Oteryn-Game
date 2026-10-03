"""No-network checks for `lower_timed_item_packet.py` (TIMED-CONTENT-1).

Synthetic wiki and Canary data cover wiki-first precedence, the Canary fallback, the equip-paired and
continuous modes, the limits, non-stackable definitions and the report; the committed packet must
rebuild byte for byte.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import lower_timed_item_packet as lower


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


def build(records, canary, ids=None, stackable=()):
    ids = set(canary) | set(records) if ids is None else ids
    rows, report, _counts = lower.build(snapshot(records), canary, ids, set(stackable))
    return {(r["item_key"], r["field_path"]): r for r in rows}, report


def key(item_id):
    return f"oteryn:item.tibia.i{item_id}"


RING_PAIR = {
    1: ("life ring", {"primarytype": "rings", "stopduration": "1", "transformequipto": "2"}),
    2: (
        "life ring",
        {
            "primarytype": "rings",
            "duration": "1200",
            "decayto": "0",
            "transformdeequipto": "1",
        },
    ),
}


def test_ring_pair_is_on_equip_with_inactive_equip_transform():
    rows, report = build({}, RING_PAIR)
    active = {p: r["typed_value"]["value"] for (k, p), r in rows.items() if k == key(2)}
    assert active == {
        "temporal.duration_ms": 1_200_000,
        "temporal.consumption_mode": "ON_EQUIP",
        "temporal.stop_duration_while_unequipped": True,
        "transform.unequip": key(1),
    }
    inactive = {p: r["typed_value"]["value"] for (k, p), r in rows.items() if k == key(1)}
    assert inactive == {"transform.equip": key(2)}
    assert rows[(key(2), "temporal.duration_ms")]["evidence"] == "OTS_HYPOTHESIS_ONLY"
    assert not report["skipped"]


def test_wiki_first_with_canary_fallback_and_disagreement_report():
    canary = {
        3: ("strange talisman", {"primarytype": "amulets and necklaces", "charges": "100"}),
        4: ("bronze amulet", {"primarytype": "amulets and necklaces", "charges": "7"}),
    }
    rows, report = build({3: [(1, {"charges": "200"})]}, canary)
    assert rows[(key(3), "charges.count")]["typed_value"] == {"kind": "COUNT_U32", "value": 200}
    assert rows[(key(3), "charges.count")]["evidence"] == "TIBIAWIKI"
    assert rows[(key(4), "charges.count")]["evidence"] == "OTS_HYPOTHESIS_ONLY"
    assert report["wiki_canary_disagree"] == {"charges": 1}


def test_wiki_duration_on_inactive_form_applies_to_active_form():
    rows, report = build({1: [(9, {"duration": "7.5 minutes"})]}, RING_PAIR)
    assert rows[(key(2), "temporal.duration_ms")]["typed_value"]["value"] == 450_000
    assert rows[(key(2), "temporal.duration_ms")]["evidence"] == "TIBIAWIKI"
    assert (key(1), "temporal.duration_ms") not in rows
    assert report["wiki_canary_disagree"] == {"duration": 1}


def test_wiki_conflict_and_malformed_never_become_rows():
    canary = {5: ("necklace", {"primarytype": "amulets and necklaces", "charges": "9"})}
    rows, report = build(
        {5: [(1, {"charges": "10"}), (2, {"charges": "11"})]},
        canary,
    )
    assert report["wiki_conflict"] == {"charges": 1}
    assert not rows  # a wiki conflict is reported, never a row and never a silent fallback
    rows, report = build({5: [(1, {"charges": "ten"})]}, {5: ("x", {"primarytype": "rings"})})
    assert report["wiki_malformed"] == {"charges": 1} and not rows


def test_lit_torch_is_continuous_and_decays():
    canary = {
        10: ("torch", {"primarytype": "light sources", "stopduration": "1"}),
        11: ("lit torch", {"primarytype": "light sources", "duration": "600", "decayto": "12"}),
        12: ("burnt down torch", {"primarytype": "light sources"}),
    }
    rows, _report = build({}, canary)
    assert rows[(key(11), "temporal.consumption_mode")]["typed_value"]["value"] == "CONTINUOUS"
    assert rows[(key(11), "temporal.stop_duration_while_unequipped")]["typed_value"]["value"] is False
    assert rows[(key(11), "transform.decay")]["typed_value"]["value"] == key(12)
    assert not [k for k in rows if k[0] in (key(10), key(12))]


def test_limits_stackable_and_undetermined_mode_are_reported_not_rows():
    canary = {
        20: ("ring", {"primarytype": "rings", "duration": "604801", "transformdeequipto": "1"}),
        21: ("ring", {"primarytype": "rings", "charges": "65536"}),
        22: ("ring", {"primarytype": "rings", "duration": "30"}),
        23: ("necklace", {"primarytype": "amulets and necklaces", "charges": "5"}),
        24: ("ring", {"primarytype": "rings", "charges": "65535"}),
    }
    rows, report = build({}, canary, stackable={23})
    assert report["skipped"] == {
        "CHARGES_ABOVE_RL_01": 1,
        "DURATION_ABOVE_RL_02": 1,
        "MODE_UNDETERMINED": 1,
        "STACKABLE_TIMED_DEFINITION": 1,
    }
    assert [k for k in rows] == [(key(24), "charges.count")]


def test_exercise_weapons_and_non_content_items_are_out_of_scope():
    canary = {
        30: ("exercise sword", {"primarytype": "exercise weapons", "charges": "500"}),
        31: ("ring", {"primarytype": "rings", "charges": "5"}),
    }
    rows, _report = build({}, canary, ids={30})
    assert not rows


def test_committed_packet_rebuilds():
    data = lower.packet_bytes(
        json.loads(lower.SNAPSHOT.read_text(encoding="utf-8")),
        lower.load_canary(),
        *lower.content_items(),
        hashlib.sha256(Path(lower.__file__).read_bytes()).hexdigest(),
    )
    assert lower.OUTPUT.read_bytes() == data
    packet = json.loads(data)
    assert packet["schema"] == lower.SCHEMA
    assert packet["counts"]["fields"] == len(packet["promotions"])
    for row in packet["promotions"]:
        assert row["evidence"] in ("TIBIAWIKI", "OTS_HYPOTHESIS_ONLY")
        if row["field_path"] == "charges.count":
            assert 1 <= row["typed_value"]["value"] <= lower.MAX_CHARGES
        if row["field_path"] == "temporal.duration_ms":
            assert 1 <= row["typed_value"]["value"] <= lower.MAX_DURATION_MS
        if row["field_path"] == "temporal.consumption_mode":
            assert row["typed_value"]["value"] in ("ON_EQUIP", "CONTINUOUS")


if __name__ == "__main__":
    failed = 0
    for name, function in sorted(globals().items()):
        if name.startswith("test_"):
            try:
                function()
                print(f"PASS {name}")
            except AssertionError as error:
                failed += 1
                print(f"FAIL {name}: {error}")
    raise SystemExit(1 if failed else 0)
