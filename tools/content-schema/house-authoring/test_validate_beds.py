"""Fixture-house tests for `validate_beds.py` (BED-CONTENT-1): the active bundle places no beds yet."""

from __future__ import annotations

import json

import validate_beds as v

# Item 1 = head (partner south), 2 = foot (partner north); 3 = head (partner east), 4 = foot (west).
FACTS = {
    1: ("head", "south"),
    2: ("foot", "north"),
    3: ("head", "east"),
    4: ("foot", "west"),
}


def house(source_id, beds, tiles):
    return {
        "source_id": source_id,
        "beds": beds,
        "tiles": [(x, y, 7) for x, y in tiles],
    }


GRID = [(x, y) for x in range(10, 14) for y in range(10, 14)]
BED_A = [(1, 10, 10, 7), (2, 10, 11, 7)]
BED_B = [(3, 12, 10, 7), (4, 13, 10, 7)]


def codes(report):
    return sorted(e["code"] for e in report["errors"])


def test_valid_pairs_match_the_catalogue():
    report = v.validate([house(1, 2, GRID)], BED_A + BED_B, FACTS, set())
    assert report["errors"] == [] and report["pairs"] == {1: 2}


def test_count_mismatch_fails_unless_excepted():
    houses = [house(1, 3, GRID)]
    assert codes(v.validate(houses, BED_A + BED_B, FACTS, set())) == [
        "BED_COUNT_MISMATCH"
    ]
    report = v.validate(houses, BED_A + BED_B, FACTS, {1})
    assert report["errors"] == []
    assert report["excepted"] == [{"house": 1, "beds": 3, "valid_pairs": 2}]


def test_a_listed_house_that_agrees_must_leave_the_list():
    assert codes(v.validate([house(1, 2, GRID)], BED_A + BED_B, FACTS, {1})) == [
        "STALE_EXCEPTION"
    ]


def test_unknown_exception_house_fails():
    assert codes(v.validate([house(1, 0, GRID)], [], FACTS, {9})) == [
        "UNKNOWN_EXCEPTION_HOUSE"
    ]


def test_head_without_foot_and_foot_without_head_fail():
    report = v.validate([house(1, 1, GRID)], [BED_A[0]], FACTS, set())
    assert "NO_MATCHING_PARTNER" in codes(report)
    report = v.validate([house(1, 1, GRID)], [BED_A[1]], FACTS, set())
    assert "NO_MATCHING_PARTNER" in codes(report)


def test_partner_that_does_not_name_back_fails():
    # the foot at (10, 11) faces west, not north, so it does not name the head back
    placed = [(1, 10, 10, 7), (4, 10, 11, 7)]
    assert "NO_MATCHING_PARTNER" in codes(
        v.validate([house(1, 1, GRID)], placed, FACTS, set())
    )


def test_pair_across_two_houses_fails():
    left = house(1, 1, [(10, 10)])
    right = house(2, 0, [(10, 11)])
    report = v.validate([left, right], BED_A, FACTS, set())
    assert "PARTNER_OUTSIDE_HOUSE" in codes(report)


def test_two_feet_for_one_head_are_ambiguous():
    placed = BED_A + [(2, 10, 11, 7)]
    assert "AMBIGUOUS_PARTNER" in codes(
        v.validate([house(1, 1, GRID)], placed, FACTS, set())
    )


def test_beds_outside_any_house_and_other_floors_are_ignored():
    placed = BED_A + [(1, 50, 50, 7), (2, 50, 51, 7), (1, 10, 10, 6)]
    report = v.validate([house(1, 1, GRID)], placed, FACTS, set())
    assert report["errors"] == []


def test_exception_list_is_the_84_known_discrepancy_houses():
    listed = v.load_exceptions()
    sample = json.loads(
        (v.ROOT / "samples" / "otbm-tile-check.json").read_text(encoding="utf-8")
    )
    divergent = {
        row[0] for row in sample["bed_divergence_source_id_beds_engine_bed_items"]
    }
    assert len(listed) == 84 and listed == divergent
    assert listed <= {h["source_id"] for h in v.load_houses()}


def test_a_placed_part_without_group_19_is_a_finding():
    houses = [house(1, 2, GRID)]
    report = v.validate(houses, BED_A + BED_B + [(9, 11, 12, 7)], FACTS, set(), {9})
    assert codes(report) == ["PART_WITHOUT_GROUP_19"]
    assert report["errors"][0]["tile"] == [11, 12, 7]
    assert v.validate(houses, BED_A + BED_B, FACTS, set(), {9})["errors"] == []


def test_committed_facts_load_and_cover_both_parts():
    facts = v.load_facts()
    assert {part for part, _ in facts.values()} == {"head", "foot"}


def test_empty_bundle_reports_every_bedded_house_as_mismatch_except_excepted():
    houses = v.load_houses()
    report = v.validate(houses, [], v.load_facts(), v.load_exceptions())
    bedded = {h["source_id"] for h in houses if h["beds"]}
    failing = {
        e["house"] for e in report["errors"] if e["code"] == "BED_COUNT_MISMATCH"
    }
    assert failing == bedded - v.load_exceptions()


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("PASS")
