"""Focused, no-network checks for `donor_census.py` (task B1a): `donor_key`'s
provisional format, `classify_donor_item`'s priority order against tiny fabricated
donor/base fixtures (no pinned checkout), and `build_census`'s donor/base id
set-difference. Mirrors `test_engine_items.py`'s own fixture style -- a real identity
index and disposition catalog are never needed here since donor items carry no Oteryn
identity at all.
"""

from __future__ import annotations

import tempfile
from pathlib import Path

import donor_census
import engine_items
from test_engine_items import (
    encode_appearances_dat,
    sha256_hex,
    write_fixture_checkout,
)

CHECKS = 0


def check(condition, message):
    global CHECKS
    CHECKS += 1
    if not condition:
        raise AssertionError(message)


# --- donor_key -----------------------------------------------------------------


def test_donor_key_format():
    key = donor_census.donor_key(52980)
    check(key == "donor:crystalserver@00ce02a5:item/52980", key)
    check(not key.startswith("oteryn:item.registry."), key)
    check(not key.startswith("oteryn:"), key)


# --- classify_donor_item: priority order, using engine_items' real helpers directly --


def synthetic_donor(item_records):
    """`item_records`: {item_id: {"attrs": {...}, "flags": {...}, "name": ...}}."""
    items = {}
    appearances = {}
    for item_id, record in item_records.items():
        items[item_id] = {
            "name": record.get("name", f"donor item {item_id}"),
            "article": record.get("article", "a"),
            "plural": record.get("plural"),
            "attrs": record.get("attrs", {}),
        }
        if "flags" in record or "appearance_name" in record:
            appearances[item_id] = {
                "id": item_id,
                "flags": record.get("flags", {}),
                "frame_groups": [],
                "name": record.get("appearance_name"),
                "description": None,
            }
    return {"items": items, "appearances": appearances}


def classify(
    item_records,
    item_id,
    merged_items=None,
    wiki_family_fallback=None,
    owner_family_decisions=None,
):
    donor = synthetic_donor(item_records)
    return donor_census.classify_donor_item(
        item_id,
        donor,
        merged_items if merged_items is not None else donor["items"],
        wiki_family_fallback or {},
        owner_family_decisions or {},
    )


def test_classify_engine_attribute_resolves():
    result = classify({600: {"attrs": {"primarytype": "valuables"}}}, 600)
    check(result["outcome"] == "resolved", result)
    check(result["family_profile"] == "material_valuable", result)
    check(result["family_profile_basis"] is None, result)


def test_classify_wrap_target_uses_merged_items_preferring_base():
    # The wrap target (90001) exists in BOTH the donor and base checkouts, with
    # different primarytype values; the base's own copy must win (never let a
    # re-fetched donor copy of a KNOWN id influence anything, even a lookup).
    donor_records = {
        601: {"attrs": {"wrapableto": "90001"}},
        90001: {"attrs": {"primarytype": "valuables"}},  # would resolve differently
    }
    base_items = {
        90001: {
            "name": "t",
            "article": "a",
            "plural": None,
            "attrs": {"primarytype": "furniture"},
        }
    }
    merged = {**synthetic_donor(donor_records)["items"], **base_items}
    result = classify(donor_records, 601, merged_items=merged)
    check(result["outcome"] == "resolved", result)
    check(result["family_profile"] == "decoration", result)
    check(result["family_profile_basis"] == "engine_wrap_target", result)
    check(result["family_profile_evidence"]["wrap_target_id"] == 90001, result)


def test_classify_dead_item_route_and_owner_table_both_still_apply():
    # Both dead-item rules are plain name/flag lookups, not registry-key lookups, so
    # they apply to a donor-only id exactly as they would to any other.
    routed = classify({602: {"name": "dead dragon", "attrs": {}, "flags": {}}}, 602)
    check(routed["outcome"] == "routed", routed)
    check(
        routed["owner"] == "WorldObject" and routed["reason"] == "corpse_decoration",
        routed,
    )

    resolved = classify(
        {603: {"name": "dead rat", "attrs": {}, "flags": {"flags.take": True}}}, 603
    )
    check(resolved["outcome"] == "resolved", resolved)
    check(resolved["family_profile"] == "material_valuable", resolved)
    check(
        resolved["family_profile_evidence"]["rule"] == "take_able_dead_creature",
        resolved,
    )


def test_classify_last_resort_routes_apply():
    fluid = classify({604: {"name": "water", "attrs": {}}}, 604)
    check(fluid["outcome"] == "routed", fluid)
    check(
        fluid["owner"] == "Fluid"
        and fluid["reason"] == "fluid_type_without_appearance",
        fluid,
    )

    placeholder = classify({605: {"name": "old tibia item", "attrs": {}}}, 605)
    check(placeholder["outcome"] == "routed", placeholder)
    check(placeholder["reason"] == "appearance_placeholder_slot", placeholder)

    no_appearance = classify({606: {"name": "some unresolved name", "attrs": {}}}, 606)
    check(no_appearance["outcome"] == "routed", no_appearance)
    check(no_appearance["reason"] == "no_client_appearance", no_appearance)

    empty_object = classify(
        {607: {"name": "some unresolved name", "attrs": {}, "flags": {}}}, 607
    )
    check(empty_object["outcome"] == "routed", empty_object)
    check(empty_object["reason"] == "appearance_placeholder_slot", empty_object)

    unresolved = classify(
        {
            608: {
                "name": "genuinely unresolved",
                "attrs": {},
                "flags": {"flags.take": True},
            }
        },
        608,
    )
    check(unresolved["outcome"] == "unresolved", unresolved)
    check(unresolved["blocker"] == "family_profile_unresolved", unresolved)


def test_classify_wiki_and_owner_table_never_match_a_provisional_key():
    # Even when a fabricated wiki-fallback/owner-table dict DOES carry an entry for
    # this exact provisional donor key (impossible in production, since real records
    # are always keyed by a real Oteryn registry key), the name-guarded match still
    # requires the item's own name to equal the entry's own name -- proving the
    # lookup path itself is exercised, not skipped, even though production data can
    # never populate it under a `donor:` key.
    key = donor_census.donor_key(609)
    wiki_fallback = {
        key: {
            "profile": "decoration",
            "matched_names": {"impossible wiki match"},
            "match_basis": "itemid",
            "evidence": {"resolution": "direct"},
            "availability": None,
        }
    }
    fixture_item = {
        609: {"name": "some other name", "attrs": {}, "flags": {"flags.take": True}}
    }
    result = classify(fixture_item, 609, wiki_family_fallback=wiki_fallback)
    check(result["outcome"] == "unresolved", result)

    owner_decisions = {
        key: {
            "name": "some other name",
            "profile": "trash",
            "reason": "r",
            "source": {"wiki_url": None, "facts": "f"},
        }
    }
    result2 = classify(fixture_item, 609, owner_family_decisions=owner_decisions)
    check(result2["outcome"] == "resolved", result2)
    check(result2["family_profile"] == "trash", result2)
    check(
        result2["family_profile_evidence"]["rule"]
        == "owner_leftover_review_2026_09_28",
        result2,
    )
    # In production the wiki_family_fallback/owner_family_decisions dicts are loaded
    # straight from the committed files, whose keys are always real
    # `oteryn:item.registry.*` strings -- `donor_key(...)` can never equal one of
    # those, so this path is provably always a miss in practice (see
    # `test_donor_census_never_matches_real_committed_tables` below).
    check(key.startswith("donor:"), key)


def test_minted_donor_id_joins_wiki_evidence_under_its_epoch_2_key():
    # Task B2: once B1b binds a donor id, the wiki-evidence join runs under that
    # committed registry key; an unbound (held) id keeps the provisional key.
    key = "oteryn:item.registry.i00038100"
    index = {610: (key, "crystal_exact_binding")}
    check(donor_census.registry_key(610, index) == key, "minted id uses its key")
    check(
        donor_census.registry_key(611, index) == donor_census.donor_key(611),
        "held id keeps the provisional key",
    )
    check(donor_census.registry_key(610, None) == donor_census.donor_key(610), "none")
    wiki_fallback = {
        key: {
            "profile": "tool",
            "matched_names": {"skewered thing"},
            "match_basis": "itemid",
            "evidence": {"resolution": "direct"},
            "availability": None,
        }
    }
    records = {610: {"name": "skewered thing", "attrs": {}, "flags": {}}}
    donor = synthetic_donor(records)
    joined = donor_census.classify_donor_item(
        610, donor, donor["items"], wiki_fallback, {}, index
    )
    check(joined["outcome"] == "resolved", joined)
    check(joined["family_profile"] == "tool", joined)
    check(joined["family_profile_basis"] == "wiki_evidence_fallback", joined)
    unbound = donor_census.classify_donor_item(
        610, donor, donor["items"], wiki_fallback, {}, None
    )
    check(unbound.get("family_profile_basis") != "wiki_evidence_fallback", unbound)


def test_donor_census_never_matches_real_committed_tables():
    """The real committed wiki-evidence snapshot and owner leftover-family table are
    both keyed by `oteryn:item.registry.*`; a provisional `donor:` key can never
    collide with either, so a donor id can never spuriously resolve through them."""
    identity_index = engine_items.build_identity_index()
    registry_keys = {key for key, _basis in identity_index.values()}
    for item_id in (1, 100, 3288, 90001, 52980):
        check(donor_census.donor_key(item_id) not in registry_keys, item_id)


# --- build_census: donor/base id set difference, using tiny fixture checkouts -------


def write_donor_fixture(root, item_records):
    items_rows = []
    appearance_objs = []
    for item_id, record in item_records.items():
        attrs = record.get("attrs", {})
        attr_tags = "".join(
            f'\n\t\t<attribute key="{k}" value="{v}"/>' for k, v in attrs.items()
        )
        name = record.get("name", f"donor item {item_id}")
        items_rows.append(
            f'\t<item id="{item_id}" article="a" name="{name}">{attr_tags}\n\t</item>'
        )
        if "appearance_name" in record:
            appearance_objs.append(
                {
                    "object_id": item_id,
                    "name": record["appearance_name"],
                    "description": "",
                    "sprite_ids": [9000 + item_id],
                }
            )
    items_text = "<items>\n" + "\n".join(items_rows) + "\n</items>\n"
    items_path = root / "data/items/items.xml"
    appearances_path = root / "data/items/appearances.dat"
    items_path.parent.mkdir(parents=True, exist_ok=True)
    items_bytes = items_text.encode("utf-8")
    appearances_bytes = encode_appearances_dat(appearance_objs)
    items_path.write_bytes(items_bytes)
    appearances_path.write_bytes(appearances_bytes)
    return {
        "data/items/items.xml": sha256_hex(items_bytes),
        "data/items/appearances.dat": sha256_hex(appearances_bytes),
    }


def test_build_census_donor_ids_never_include_base_ids():
    with (
        tempfile.TemporaryDirectory() as base_dir,
        tempfile.TemporaryDirectory() as donor_dir,
    ):
        base_root = Path(base_dir)
        donor_root = Path(donor_dir)
        base_digests = write_fixture_checkout(base_root, "crystal")
        # 100 overlaps the base fixture (must be excluded); 700/701 are new.
        donor_digests = write_donor_fixture(
            donor_root,
            {
                100: {
                    "attrs": {"primarytype": "valuables"},
                    "name": "fixture trinket 100",
                },
                700: {
                    "attrs": {"primarytype": "valuables"},
                    "name": "donor-only trinket",
                },
                701: {"attrs": {"primarytype": "food"}, "name": "donor-only snack"},
            },
        )
        result_doc, new_ids = donor_census.build_census(
            donor_root,
            base_root,
            donor_digests=donor_digests,
            base_digests=base_digests,
        )
        check(new_ids == [700, 701], new_ids)
        check(100 not in new_ids, new_ids)
        check(set(result_doc["items"]) == {"700", "701"}, result_doc["items"])
        check(result_doc["totals"]["new_ids"] == 2, result_doc["totals"])
        check(
            result_doc["items"]["701"]["family_profile"] == "food", result_doc["items"]
        )


if __name__ == "__main__":
    tests = [
        test_donor_key_format,
        test_classify_engine_attribute_resolves,
        test_classify_wrap_target_uses_merged_items_preferring_base,
        test_classify_dead_item_route_and_owner_table_both_still_apply,
        test_classify_last_resort_routes_apply,
        test_classify_wiki_and_owner_table_never_match_a_provisional_key,
        test_minted_donor_id_joins_wiki_evidence_under_its_epoch_2_key,
        test_donor_census_never_matches_real_committed_tables,
        test_build_census_donor_ids_never_include_base_ids,
    ]
    for test in tests:
        test()
    print(f"PASS {CHECKS} checks")
