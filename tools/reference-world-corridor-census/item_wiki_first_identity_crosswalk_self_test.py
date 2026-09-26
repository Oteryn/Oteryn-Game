#!/usr/bin/env python3
from __future__ import annotations

import copy
import importlib.util
from pathlib import Path

MODULE_PATH = Path(__file__).with_name(
    "item_wiki_first_identity_crosswalk.py"
)
spec = importlib.util.spec_from_file_location(
    "item_wiki_first_identity_crosswalk", MODULE_PATH
)
if spec is None or spec.loader is None:
    raise RuntimeError("crosswalk import failed")
crosswalk = importlib.util.module_from_spec(spec)
spec.loader.exec_module(crosswalk)


def reject(action, marker: str) -> None:
    try:
        action()
    except crosswalk.CrosswalkError as exc:
        assert marker in str(exc), (marker, str(exc))
    else:
        raise AssertionError(f"expected {marker}")


def profile(profile_id: str, **signals: int) -> dict:
    reverse = {
        "attack": "attack",
        "defense": "defense",
        "defensemod": "extra_defense",
        "range": "range",
        "hit": "hit_chance",
        "armor": "armor",
        "charges": "charge_count",
        "volume": "capacity",
    }
    return {
        "profile_id": profile_id,
        "candidate_observations": [
            {
                "native_field": reverse[wiki_field],
                "source_value": str(value),
                "nested_values": [],
            }
            for wiki_field, value in signals.items()
        ],
    }


def identity(source_id: int, native_key: str, profile_id: str) -> dict:
    return {
        "source_item_id": source_id,
        "native_key": native_key,
        "source_profile_id": profile_id,
        "identity_origin": "OPAQUE_REGISTRY_ALLOCATION",
    }


def page(
    page_id: int,
    title: str,
    shape: str = "INFOBOX_ITEM",
    **fields,
) -> dict:
    return {
        "page_id": page_id,
        "title": title,
        "source_shape": shape,
        "source_parse_error": (
            "INFOBOX_DUPLICATE_CONFLICT:attack"
            if shape == "INFOBOX_ITEM_PARSE_ERROR"
            else None
        ),
        "normalized_fields": (
            {
                key: {"state": "VALUE", "value": value}
                for key, value in fields.items()
            }
            if shape != "INFOBOX_ITEM_PARSE_ERROR"
            else {}
        ),
        "unmapped_infobox_fields": {},
        "infobox_present": shape != "NO_INFOBOX_ITEM",
    }


def index(records, profiles, names):
    old = crosswalk.EXPECTED_SOURCE_COUNT
    try:
        crosswalk.EXPECTED_SOURCE_COUNT = len(records)
        return crosswalk.build_identity_index(
            {"records": records, "source_profiles": profiles}, names
        )
    finally:
        crosswalk.EXPECTED_SOURCE_COUNT = old


def test_exact_name_plus_signal_is_exact() -> None:
    identities, names, profiles = index(
        [identity(1, "oteryn:item.a", "p1")],
        [profile("p1", armor=18)],
        {1: "falcon plate"},
    )
    result = crosswalk.classify_page(
        page(10, "Falcon Plate", armor=18),
        identities=identities,
        name_index=names,
        profiles=profiles,
    )
    assert result["disposition"] == "EXACT_MATCH"


def test_name_only_never_exact() -> None:
    identities, names, profiles = index(
        [identity(1, "oteryn:item.a", "p1")],
        [profile("p1")],
        {1: "falcon plate"},
    )
    result = crosswalk.classify_page(
        page(10, "Falcon Plate"),
        identities=identities,
        name_index=names,
        profiles=profiles,
    )
    assert result["disposition"] == "PROBABLE_MATCH"


def test_two_plausible_identities_are_ambiguous() -> None:
    identities, names, profiles = index(
        [
            identity(1, "oteryn:item.a", "p1"),
            identity(2, "oteryn:item.b", "p2"),
        ],
        [profile("p1"), profile("p2")],
        {1: "ring", 2: "ring"},
    )
    result = crosswalk.classify_page(
        page(10, "Ring"),
        identities=identities,
        name_index=names,
        profiles=profiles,
    )
    assert result["disposition"] == "AMBIGUOUS"


def test_unique_structural_survivor_is_exact() -> None:
    identities, names, profiles = index(
        [
            identity(1, "oteryn:item.a", "p1"),
            identity(2, "oteryn:item.b", "p2"),
        ],
        [profile("p1", armor=18), profile("p2", armor=20)],
        {1: "plate", 2: "plate"},
    )
    result = crosswalk.classify_page(
        page(10, "Plate", armor=18),
        identities=identities,
        name_index=names,
        profiles=profiles,
    )
    assert result["disposition"] == "EXACT_MATCH"
    assert result["selected_source_item_id"] == 1


def test_strong_contradiction_is_conflict() -> None:
    identities, names, profiles = index(
        [identity(1, "oteryn:item.a", "p1")],
        [profile("p1", armor=17)],
        {1: "plate"},
    )
    result = crosswalk.classify_page(
        page(10, "Plate", armor=18),
        identities=identities,
        name_index=names,
        profiles=profiles,
    )
    assert result["disposition"] == "CONFLICT"


def test_zero_candidates_is_no_match() -> None:
    identities, names, profiles = index(
        [identity(1, "oteryn:item.a", "p1")],
        [profile("p1")],
        {1: "plate"},
    )
    result = crosswalk.classify_page(
        page(10, "Unknown"),
        identities=identities,
        name_index=names,
        profiles=profiles,
    )
    assert result["disposition"] == "NO_MATCH"


def test_alias_can_discover_but_needs_independent_signal() -> None:
    identities, names, profiles = index(
        [identity(1, "oteryn:item.a", "p1")],
        [profile("p1", armor=18)],
        {1: "falcon plate"},
    )
    result = crosswalk.classify_page(
        page(
            10,
            "Old Falcon",
            aliases="Falcon Plate",
            armor=18,
        ),
        identities=identities,
        name_index=names,
        profiles=profiles,
    )
    assert result["disposition"] == "EXACT_MATCH"
    assert "WIKI_ALIAS" in result["candidates"][0]["name_match_origins"]


def test_duplicate_target_protection_marks_variant() -> None:
    records = [
        {
            "disposition": "EXACT_MATCH",
            "reason": "x",
            "title": "Falcon Plate",
            "selected_native_key": "oteryn:item.a",
            "selected_source_item_id": 1,
            "candidates": [
                {"source_item_id": 1, "crystal_name": "falcon plate"}
            ],
        },
        {
            "disposition": "EXACT_MATCH",
            "reason": "x",
            "title": "Old Falcon Plate",
            "selected_native_key": "oteryn:item.a",
            "selected_source_item_id": 1,
            "candidates": [
                {"source_item_id": 1, "crystal_name": "falcon plate"}
            ],
        },
    ]
    crosswalk.apply_duplicate_target_protection(records)
    assert records[0]["disposition"] == "EXACT_MATCH"
    assert records[1]["disposition"] == "ALIAS_OR_DUPLICATE"


def test_duplicate_source_page_rejected() -> None:
    reject(
        lambda: crosswalk.validate_source_pages(
            [page(1, "A"), page(1, "B")]
        ),
        "DUPLICATE_WIKI_PAGE_ID",
    )


def test_no_infobox_item_not_dropped() -> None:
    identities, names, profiles = index(
        [identity(1, "oteryn:item.a", "p1")],
        [profile("p1")],
        {1: "book"},
    )
    result = crosswalk.classify_page(
        page(10, "Book", shape="NO_INFOBOX_ITEM"),
        identities=identities,
        name_index=names,
        profiles=profiles,
    )
    assert result["disposition"] == "PROBABLE_MATCH"


def test_parse_error_conflicting_value_not_used() -> None:
    identities, names, profiles = index(
        [identity(1, "oteryn:item.a", "p1")],
        [profile("p1", attack=40)],
        {1: "blade"},
    )
    result = crosswalk.classify_page(
        page(
            10,
            "Blade",
            shape="INFOBOX_ITEM_PARSE_ERROR",
            attack=40,
        ),
        identities=identities,
        name_index=names,
        profiles=profiles,
    )
    assert result["disposition"] == "PROBABLE_MATCH"
    assert result["candidates"][0]["matched_structural_signals"] == []


def test_donor_internal_conflict_is_ignored() -> None:
    value = profile("p1", attack=40)
    value["candidate_observations"].append(
        {
            "native_field": "attack",
            "source_value": "41",
            "nested_values": [],
        }
    )
    signals, conflicts = crosswalk.profile_structural_signals(value)
    assert "attack" not in signals
    assert conflicts == ["attack"]


def test_deterministic_sorting_output_and_partition() -> None:
    old_wiki = crosswalk.EXPECTED_WIKI_COUNT
    old_source = crosswalk.EXPECTED_SOURCE_COUNT
    try:
        crosswalk.EXPECTED_WIKI_COUNT = 2
        crosswalk.EXPECTED_SOURCE_COUNT = 2
        classification = {
            "records": [
                identity(1, "oteryn:item.a", "p1"),
                identity(2, "oteryn:item.b", "p2"),
            ],
            "source_profiles": [
                profile("p1", armor=1),
                profile("p2"),
            ],
        }
        census = {
            "pages": [
                page(20, "Beta"),
                page(10, "Alpha", armor=1),
            ]
        }
        names = {1: "alpha", 2: "beta"}
        first = crosswalk.compile_crosswalk(
            census, classification, names
        )
        second = crosswalk.compile_crosswalk(
            copy.deepcopy(census),
            copy.deepcopy(classification),
            dict(names),
        )
        assert (
            crosswalk.canonical_bytes(first)
            == crosswalk.canonical_bytes(second)
        )
        assert sum(first["counts"]["dispositions"].values()) == 2
    finally:
        crosswalk.EXPECTED_WIKI_COUNT = old_wiki
        crosswalk.EXPECTED_SOURCE_COUNT = old_source


def test_census_digest_mismatch_fails_closed() -> None:
    old_count = crosswalk.EXPECTED_WIKI_COUNT
    old_digest = crosswalk.CENSUS_STABLE_SHA256
    try:
        crosswalk.EXPECTED_WIKI_COUNT = 1
        crosswalk.CENSUS_STABLE_SHA256 = "0" * 64
        reject(
            lambda: crosswalk.verify_census(
                {
                    "schema": crosswalk.CENSUS_SCHEMA,
                    "pages": [page(1, "A")],
                    "retrieval_timestamp": "x",
                }
            ),
            "CENSUS_STABLE_DIGEST_MISMATCH",
        )
    finally:
        crosswalk.EXPECTED_WIKI_COUNT = old_count
        crosswalk.CENSUS_STABLE_SHA256 = old_digest


def test_no_identity_minting_and_no_semantic_promotion() -> None:
    old_wiki = crosswalk.EXPECTED_WIKI_COUNT
    old_source = crosswalk.EXPECTED_SOURCE_COUNT
    try:
        crosswalk.EXPECTED_WIKI_COUNT = 1
        crosswalk.EXPECTED_SOURCE_COUNT = 1
        full = crosswalk.compile_crosswalk(
            {"pages": [page(1, "A", armor=2)]},
            {
                "records": [identity(1, "oteryn:item.a", "p1")],
                "source_profiles": [profile("p1", armor=2)],
            },
            {1: "a"},
        )
        assert (
            full["records"][0]["selected_native_key"]
            == "oteryn:item.a"
        )
        manifest = crosswalk.build_manifest(
            full, compiler_sha256="1" * 64
        )
        assert (
            manifest["invariants"]["identity_minting_performed"]
            is False
        )
        assert (
            manifest["invariants"]["semantic_promotion_performed"]
            is False
        )
    finally:
        crosswalk.EXPECTED_WIKI_COUNT = old_wiki
        crosswalk.EXPECTED_SOURCE_COUNT = old_source


def test_crystal_digest_mismatch_fails_closed() -> None:
    reject(
        lambda: crosswalk.parse_crystal_names(b"<items />"),
        "CRYSTAL_ITEMS_DIGEST_MISMATCH",
    )


def main() -> int:
    tests = [
        value
        for name, value in sorted(globals().items())
        if name.startswith("test_") and callable(value)
    ]
    for test in tests:
        test()
    print(
        "item-wiki-first-identity-crosswalk self-test: PASS "
        f"tests={len(tests)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
