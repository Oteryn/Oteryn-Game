#!/usr/bin/env python3
"""No-network tests for charm_authoring.py. Run with `python test_charm_authoring.py`."""

from __future__ import annotations

import copy
import json

import charm_authoring as ca

PAGE = """{{Infobox Charm|List={{{1|}}}|GetValue={{{GetValue|}}}
| name          = Wound
| type          = Major
| cost          = 240 / 360 / 1,200
| effect        = Each attack on a creature has a 5% / 10% / 11% chance to trigger and deal 5% of its maximum [[Hit Point]]s as [[Physical Damage]] once.
| implemented   = 11.50.6055
| notes         = Damage is limited to 2 times the character's level (applied before resistances).
}}
"""


def expect_error(fn, *args, fragment: str) -> None:
    try:
        fn(*args)
    except ValueError as error:
        assert fragment in str(error), f"{fragment!r} not in {error}"
        return
    raise AssertionError(f"expected ValueError containing {fragment!r}")


def committed() -> dict:
    return json.loads(ca.CATALOGUE.read_text(encoding="utf-8"))


def test_wiki_facts() -> None:
    facts = ca.wiki_facts("Wound", 1, 2, "t", PAGE)
    assert facts["cost"] == [240, 360, 1200] and facts["stage_values"] == [5, 10, 11]
    assert facts["type"] == "major" and facts["wikitext_sha256"] == ca.sha256(
        PAGE.encode()
    )
    comma = PAGE.replace("240 / 360 / 1,200", "240, 360, 1200")
    assert ca.wiki_facts("Wound", 1, 2, "t", comma)["cost"] == [240, 360, 1200]
    expect_error(
        ca.wiki_facts,
        "Wound",
        1,
        2,
        "t",
        PAGE.replace("Infobox Charm", "Infobox Item"),
        fragment="no Infobox",
    )
    expect_error(
        ca.wiki_facts,
        "Wound",
        1,
        2,
        "t",
        PAGE.replace("240 / ", ""),
        fragment="unexpected cost",
    )
    expect_error(
        ca.wiki_facts,
        "Wound",
        1,
        2,
        "t",
        PAGE.replace("5% / 10%", "5% or 10%"),
        fragment="percent triple",
    )
    expect_error(
        ca.wiki_facts,
        "Wound",
        1,
        2,
        "t",
        PAGE.replace("2 times", "3 times"),
        fragment="page text lacks",
    )
    expect_error(
        ca.wiki_facts,
        "Wound",
        1,
        2,
        "t",
        PAGE.replace("| type          = Major", "| type = Rare"),
        fragment="unexpected type",
    )


def test_canary_digest_is_pinned() -> None:
    expect_error(ca.canary_facts, b"local charms = {}", fragment="digest mismatch")


def test_committed_build_is_current_and_valid() -> None:
    sources = json.loads(ca.SOURCES.read_text(encoding="utf-8"))
    catalogue, report = ca.build(sources)
    assert catalogue == committed()
    assert report == json.loads(ca.REPORT.read_text(encoding="utf-8"))
    assert len(catalogue["charms"]) == 25 and ca.validate(catalogue) == []
    by_key = {c["key"]: c for c in catalogue["charms"]}
    assert by_key["oteryn:charm.voids_call"]["name"] == "Void's Call"
    assert (
        by_key["oteryn:charm.curse"]["sources"]["tibiawiki"]["title"] == "Curse (Charm)"
    )
    assert sum(c["category"] == "major" for c in catalogue["charms"]) == 14


def test_build_rejects_unmatched_names() -> None:
    sources = json.loads(ca.SOURCES.read_text(encoding="utf-8"))
    broken = copy.deepcopy(sources)
    broken["canary"]["charms"][0]["name"] = "Renamed"
    expect_error(ca.build, broken, fragment="no Canary charm")
    extra = copy.deepcopy(sources)
    extra["canary"]["charms"].append({**extra["canary"]["charms"][0], "name": "Extra"})
    expect_error(ca.build, extra, fragment="without a wiki page")


def test_validate_negatives() -> None:
    cases = [
        (lambda c: c["charms"][0].update(extra=1), "Additional properties"),
        (lambda c: c["charms"][1].update(key=c["charms"][0]["key"]), "duplicate key"),
        (
            lambda c: c["charms"][0].update(key="oteryn:charm.other"),
            "key does not follow",
        ),
        (lambda c: c["charms"][0].update(cost_currency="charm_points"), "must cost"),
        (lambda c: c["charms"][0].update(stage_value="effect_percent"), "stages carry"),
        (lambda c: c["charms"][0]["stages"].reverse(), "stages must be 1, 2, 3"),
        (lambda c: c["charms"][0]["stages"][2].update(cost=1), "costs must increase"),
        (lambda c: c["charms"][0]["stages"][2].update(value=101), "cannot exceed 100%"),
        (lambda c: c["charms"][0]["stages"].pop(), "is too short"),
        (lambda c: c["charms"][0]["effect"].update(duration_ms=0), "effect"),
        (
            lambda c: c["charms"][0]["sources"]["canary"].update(
                charm_id=c["charms"][1]["sources"]["canary"]["charm_id"]
            ),
            "duplicate charm_id",
        ),
    ]
    for mutate, fragment in cases:
        catalogue = committed()
        mutate(catalogue)
        errors = ca.validate(catalogue)
        assert any(fragment in e for e in errors), f"{fragment!r} not in {errors}"


if __name__ == "__main__":
    tests = [
        value for name, value in sorted(globals().items()) if name.startswith("test_")
    ]
    for test in tests:
        test()
    print(f"{len(tests)} tests passed")
