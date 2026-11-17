#!/usr/bin/env python3
"""No-network regression checks for the exact Koshei platinum reward overlay."""

import copy

import reward_claim_authoring as rca

PILOT = "canary:reward-claim/quest/u8_1/koshei_the_deathless/koshei_the_deathless_platinum"
POSITION = {"x": 33261, "y": 32448, "z": 12}
ITEM = "oteryn:item.tibia.i3035"
IDS = [{"server": "canary", "unique_id": 6241},
       {"server": "crystalserver", "unique_id": 6241}]


def fixture():
    claim = {
        "identity": {"key": PILOT, "revision": "pilot-r1"},
        "claim": {"per": "character", "repeat": {"kind": "once"}},
        "placements": [{"position": copy.deepcopy(POSITION),
            "appearance": {"family": "Item", "key": "canary:item/1740"},
            "reward": {"items": [{"item": {"family": "Item", "key": "canary:item/3035"},
                                   "count": 100}]}}],
    }
    manifest = {"entries": [{"destination": PILOT, "position": list(POSITION.values()),
        "sources": [{"source": i["server"], "uid": i["unique_id"]} for i in IDS]}]}
    items = {ITEM: {"materializable": True, "stack_class": "StackCapable"}}
    return claim, manifest, items


def test_exact_source_normalizes_to_fifty():
    import koshei_platinum as kp

    assert kp.quantity(PILOT, POSITION, ITEM, 100, IDS) == 50


def test_source_drift_is_refused():
    import koshei_platinum as kp

    for position, item, count, ids in [
        ({**POSITION, "x": 33262}, ITEM, 100, IDS),
        (POSITION, "oteryn:item.tibia.i3031", 100, IDS),
        (POSITION, ITEM, 50, IDS),
        (POSITION, ITEM, 100, IDS[:1]),
        (POSITION, ITEM, 100, [IDS[0], {**IDS[1], "unique_id": 6242}]),
        (POSITION, ITEM, 100, IDS + [IDS[0]]),
    ]:
        try:
            kp.quantity(PILOT, position, item, count, ids)
        except ValueError as error:
            assert "KOSHEI_PLATINUM_SOURCE_DRIFT" in str(error)
        else:
            raise AssertionError("Koshei source drift was admitted")


def test_sibling_claims_are_unchanged():
    import koshei_platinum as kp

    for pilot in [PILOT.replace("_platinum", "_amulet"), PILOT + "/sibling"]:
        assert kp.quantity(pilot, POSITION, ITEM, 100, IDS) == 100


def test_authoring_normalizes_without_mutating_donor():
    claim, manifest, items = fixture()
    before = copy.deepcopy((claim, manifest))
    records, checks = rca.build_records([claim], manifest, items, [], set())
    assert checks == []
    assert records[0]["definition"]["placements"][0]["reward"]["items"][0]["count"] == 50
    assert (claim, manifest) == before
    assert rca.validate(records, items, checks, [claim], []) == []
    assert rca.validate(records, items, checks, charge_evidence=[]) == []
    records[0]["definition"]["placements"][0]["reward"]["items"][0]["count"] = 100
    assert any("quantity" in error for error in rca.validate(records, items, checks, [claim], []))
    assert any("quantity" in error for error in rca.validate(records, items, checks, charge_evidence=[]))


def test_authoring_refuses_drift_in_real_source():
    claim, manifest, items = fixture()
    claim["placements"][0]["reward"]["items"][0]["count"] = 50
    try:
        rca.build_records([claim], manifest, items, [], set())
    except ValueError as error:
        assert "KOSHEI_PLATINUM_SOURCE_DRIFT" in str(error)
    else:
        raise AssertionError("authoring silently accepted changed donor quantity")


if __name__ == "__main__":
    for name, test in sorted(globals().copy().items()):
        if name.startswith("test_") and callable(test):
            test()
            print(f"ok {name}")
