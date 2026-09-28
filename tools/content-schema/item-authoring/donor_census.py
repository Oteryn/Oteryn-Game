"""Census the new ids a Crystal donor revision adds over the pinned engine (task B1a).

The owner's rule: an upstream donor checkout is only a source of FACTS (items.xml,
appearances.dat); Oteryn decides classification with its own rules, never upstream's.
This tool never mints Oteryn identity. It reuses `engine_items`'s already-factored
classification helpers directly -- `non_item_route`, `classify_family_profile`,
`immovable_non_item_route`, `resolve_wrap_target_profile`,
`resolve_dead_item_route_or_profile`, `resolve_fluid_type_route`,
`resolve_late_placeholder_route`, `resolve_no_client_appearance_route`,
`resolve_empty_client_object_route`, `fallback_entry_matches_name` -- in the exact same
priority order `engine_items.convert_item` uses, but starting *after* identity
resolution (there is none: these ids have no CW2 B1 allocator key at all) instead of
before it. `engine_items.py` itself is not forked or modified for this.

Scope: family-profile classification only -- never delivery-task eligibility, field
mapping or Presentation binding, all of which need a real Oteryn identity that this
task is explicitly forbidden from minting (B1b, a separate reviewed Content/World
step, assigns that). The wiki-evidence fallback and the owner leftover-family table
(`owner-item-family-decisions.json`) are both keyed by Oteryn registry key. Since B1b
(#1179) the committed Crystal bindings carry an epoch-2 registry key for every minted
donor id (`engine_items.build_identity_index`), so both joins are tried under that key
(task B2); a held id (PROBABLE_MATCH/AMBIGUOUS, no key) keeps the provisional
`donor_key` and can never match either. This tool itself mints nothing and does not
touch the network; `tools/content-census/item_wiki_family_capture.py --donor-only`
captures the wiki evidence.

Every id classified here is, by construction, absent from both pinned engines'
`items.xml` (verified against the committed `population-*.json` censuses, which stay
byte-identical -- see `--check` on `population_census.py` for both engines). No
existing id is ever re-classified, overwritten, or has its outcome influenced by this
tool.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path

from engine_items import (
    classify_family_profile,
    convert_item,
    fallback_entry_matches_name,
    immovable_non_item_route,
    load_appearance_objects,
    load_engine_sources,
    load_items_xml,
    non_item_route,
    read_verified_artifact,
    resolve_dead_item_route_or_profile,
    resolve_empty_client_object_route,
    resolve_fluid_type_route,
    resolve_late_placeholder_route,
    resolve_no_client_appearance_route,
    resolve_wrap_target_profile,
)

ROOT = Path(__file__).resolve().parent

# --- pinned donor commit (task B1a, 2026-09-28) -------------------------------------
DONOR_REPOSITORY = "zimbadev/crystalserver"
DONOR_BRANCH = "summer-update"
DONOR_COMMIT = "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
DONOR_COMMIT_SHORT = "00ce02a5"
DONOR_ITEMS_XML_SHA256 = (
    "13a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece"
)
DONOR_APPEARANCES_SHA256 = (
    "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1"
)
# The 15.30 client's own new appearance-id range (owner-measured 2026-09-28); every
# other new id predates 15.25 and was simply absent from the pinned ff7ede5 revision.
NEW_APPEARANCE_RANGE = range(52977, 55118)

# The live census (task B2: rows carry each id's committed key). The B1a census,
# `donor-census-crystal-summer-update-00ce02a5.json`, is the frozen B1b epoch-2 input
# pinned by exact bytes in `apps/game-server/src/content/cw2_b1_import.rs` and
# `g4_item_crystal_binding_generator.py`: never regenerated or edited.
DEFAULT_SAMPLE_NAME = "donor-census-crystal-summer-update-00ce02a5-keyed.json"
SELF_CHECK_ROUTED_ID = (
    54335  # "slain iceplume strider", flags.corpse: WorldObject/corpse
)
SELF_CHECK_ATTRIBUTE_ID = 35500  # "magic portal", engine attribute -> material_valuable
SELF_CHECK_WRAP_TARGET_ID = 50213  # "sickbed", wrapableto and wiki -> decoration
# B2 wiki evidence by exact itemid join under the epoch-2 key (Fandom primarytype).
SELF_CHECK_WIKI_EVIDENCE = {
    53695: "material_valuable",  # "lunar ascension orb", Valuables
    54480: "material_valuable",  # "auric moon sigil", Valuables
    54638: "tool",  # "skewered fish", Taming Items
    54651: "tool",  # "cloud in a bottle", Taming Items
}
# B1b held ids (PROBABLE_MATCH 35500, AMBIGUOUS 53380): no epoch-2 key.
SELF_CHECK_HELD_IDS = (35500, 53380)

# Only the exact numeric-itemid join is even attempted (owner instruction, task B1a):
# the title/appearance_title/actualname joins all require a broader wiki-title index
# walk than "does this exact id have a page", and are explicitly out of scope here.
DONOR_WIKI_MATCH_BASES = ("itemid",)


def donor_key(item_id):
    """Provisional, clearly non-canonical key: never `oteryn:item.registry.*`. No
    Oteryn identity is minted by this tool (hard constraint, task B1a)."""
    return f"donor:crystalserver@{DONOR_COMMIT_SHORT}:item/{item_id}"


def registry_key(item_id, identity_index):
    """The id's committed registry key (epoch 2, B1b) when it has one, else the
    provisional `donor_key`. Never mints: `identity_index` is read-only."""
    entry = (identity_index or {}).get(item_id)
    return entry[0] if entry else donor_key(item_id)


def load_donor_artifacts(donor_source, digests=None):
    """`digests` overrides the pinned donor SHA-256 map; used only by fixture tests."""
    items_digest = (
        digests["data/items/items.xml"] if digests else DONOR_ITEMS_XML_SHA256
    )
    appearances_digest = (
        digests["data/items/appearances.dat"] if digests else DONOR_APPEARANCES_SHA256
    )
    items_bytes, items_mode = read_verified_artifact(
        donor_source, "data/items/items.xml", items_digest
    )
    appearances_bytes, appearances_mode = read_verified_artifact(
        donor_source, "data/items/appearances.dat", appearances_digest
    )
    return {
        "items": load_items_xml(items_bytes.decode("utf-8")),
        "appearances": load_appearance_objects(appearances_bytes),
        "artifact_digests": {
            "data/items/items.xml": {
                "sha256": items_digest,
                "digest_mode": items_mode,
            },
            "data/items/appearances.dat": {
                "sha256": appearances_digest,
                "digest_mode": appearances_mode,
            },
        },
    }


def classify_donor_item(
    item_id,
    donor,
    merged_items,
    wiki_family_fallback,
    owner_family_decisions,
    identity_index=None,
):
    """Mirrors `engine_items.convert_item`'s classification order exactly, starting
    after (non-existent) identity resolution. Returns one of:
    `{"outcome": "routed", "name", "owner", "reason"}`,
    `{"outcome": "resolved", "name", "family_profile", "family_profile_basis",
    "family_profile_evidence"}`, or `{"outcome": "unresolved", "name", "blocker"}`.
    """
    xml_record = donor["items"].get(item_id)
    appearance = donor["appearances"].get(item_id)
    attrs = dict(xml_record["attrs"]) if xml_record else {}
    flags = dict(appearance["flags"]) if appearance else {}
    name = (
        (xml_record["name"] if xml_record else None)
        or (appearance.get("name") if appearance else None)
        or f"item {item_id}"
    )

    non_item = non_item_route(xml_record, attrs, flags)
    if non_item is not None:
        owner, reason = non_item
        return {"outcome": "routed", "name": name, "owner": owner, "reason": reason}

    primarytype = attrs.get("primarytype")
    family_profile = classify_family_profile(
        attrs, primarytype, flags.get("clothes.slot")
    )
    family_profile_basis = None
    family_profile_evidence = None

    if family_profile is not None:
        return {
            "outcome": "resolved",
            "name": name,
            "family_profile": family_profile,
            "family_profile_basis": None,
            "family_profile_evidence": None,
        }

    immovable_route = immovable_non_item_route(flags)
    if immovable_route is not None:
        owner, reason = immovable_route
        return {"outcome": "routed", "name": name, "owner": owner, "reason": reason}

    key = registry_key(item_id, identity_index)
    # Both dicts are keyed by Oteryn registry key: a minted donor id joins under its
    # epoch-2 key (B1b), a held id's provisional key can never match -- see module
    # docstring.
    fallback_entry = wiki_family_fallback.get(key)
    if fallback_entry_matches_name(
        fallback_entry, name, appearance, DONOR_WIKI_MATCH_BASES
    ):
        family_profile = fallback_entry["profile"]
        family_profile_basis = "wiki_evidence_fallback"
        family_profile_evidence = fallback_entry["evidence"]
    else:
        wrap_target = resolve_wrap_target_profile(merged_items, attrs)
        if wrap_target is not None:
            wrap_profile, wrap_target_id, wrap_target_primarytype = wrap_target
            family_profile = wrap_profile
            family_profile_basis = "engine_wrap_target"
            family_profile_evidence = {
                "wrap_target_id": wrap_target_id,
                "wrap_target_primarytype": wrap_target_primarytype,
            }
        else:
            dead_route = resolve_dead_item_route_or_profile(name, flags)
            if dead_route is not None and dead_route[0] == "route":
                _kind, owner, reason = dead_route
                return {
                    "outcome": "routed",
                    "name": name,
                    "owner": owner,
                    "reason": reason,
                }
            if dead_route is not None and dead_route[0] == "profile":
                family_profile = dead_route[1]
                family_profile_basis = "owner_name_rule"
                family_profile_evidence = {
                    "rule": "take_able_dead_creature",
                    "name": name.strip().lower(),
                }

        if (
            family_profile is None
            and (decision := owner_family_decisions.get(key)) is not None
            and name.strip().lower() == decision["name"]
        ):
            family_profile = decision["profile"]
            family_profile_basis = "owner_name_rule"
            family_profile_evidence = {
                "rule": "owner_leftover_review_2026_09_28",
                "name": decision["name"],
            }

    if family_profile is None:
        for route_fn, args in (
            (resolve_fluid_type_route, (name, xml_record, appearance)),
            (resolve_late_placeholder_route, (name,)),
            (resolve_no_client_appearance_route, (appearance,)),
            (resolve_empty_client_object_route, (appearance,)),
        ):
            route = route_fn(*args)
            if route is not None:
                owner, reason = route
                return {
                    "outcome": "routed",
                    "name": name,
                    "owner": owner,
                    "reason": reason,
                }
        return {
            "outcome": "unresolved",
            "name": name,
            "blocker": "family_profile_unresolved",
        }

    return {
        "outcome": "resolved",
        "name": name,
        "family_profile": family_profile,
        "family_profile_basis": family_profile_basis,
        "family_profile_evidence": family_profile_evidence,
    }


def build_census(donor_source, base_source, donor_digests=None, base_digests=None):
    """`donor_digests`/`base_digests` override the pinned SHA-256 maps; used only by
    fixture tests."""
    donor = load_donor_artifacts(donor_source, digests=donor_digests)
    base_sources = load_engine_sources("crystal", base_source, digests=base_digests)
    base_items = base_sources["items"]
    wiki_family_fallback = base_sources["wiki_family_fallback"]
    owner_family_decisions = base_sources["owner_family_decisions"]
    identity_index = base_sources["identity_index"]

    new_ids = sorted(set(donor["items"]) - set(base_items))
    # Prefer the pinned base's own record for any id both checkouts share (wrap-target
    # lookups only): never let a re-fetched donor copy of a KNOWN id override pinned
    # base reality, even for this internal, read-only lookup.
    merged_items = {**donor["items"], **base_items}

    by_profile = Counter()
    by_basis = Counter()
    by_rule = Counter()
    routed_counts = Counter()
    blocker_counts = Counter()
    rows = {}
    for item_id in new_ids:
        result = classify_donor_item(
            item_id,
            donor,
            merged_items,
            wiki_family_fallback,
            owner_family_decisions,
            identity_index,
        )
        key = registry_key(item_id, identity_index)
        if result["outcome"] == "resolved":
            by_profile[result["family_profile"]] += 1
            by_basis[result["family_profile_basis"] or "engine_attribute"] += 1
            evidence = result.get("family_profile_evidence") or {}
            rule = evidence.get("rule")
            if rule:
                by_rule[rule] += 1
            rows[str(item_id)] = {
                "key": key,
                "name": result["name"],
                "outcome": "resolved",
                "family_profile": result["family_profile"],
                "family_profile_basis": result["family_profile_basis"],
            }
        elif result["outcome"] == "routed":
            owner_key = f"{result['owner']}:{result['reason']}"
            routed_counts[owner_key] += 1
            rows[str(item_id)] = {
                "key": key,
                "name": result["name"],
                "outcome": "routed",
                "owner": result["owner"],
                "reason": result["reason"],
            }
        else:
            blocker_counts[result["blocker"]] += 1
            rows[str(item_id)] = {
                "key": key,
                "name": result["name"],
                "outcome": "unresolved",
                "blocker": result["blocker"],
            }

    new_range_ids = [i for i in new_ids if i in NEW_APPEARANCE_RANGE]
    older_ids = [i for i in new_ids if i not in NEW_APPEARANCE_RANGE]

    result_doc = {
        "schema": "OTERYN_ITEM_DONOR_CENSUS/v1",
        "donor": {
            "repository": DONOR_REPOSITORY,
            "branch": DONOR_BRANCH,
            "commit": DONOR_COMMIT,
            "artifact_digests": donor["artifact_digests"],
        },
        "base": {
            "engine": "crystal",
            "repository": base_sources["repository"],
            "revision": base_sources["revision"],
        },
        "scope": (
            "Family-profile classification only, for donor-only ids (present in the "
            "donor items.xml, absent from the pinned base items.xml). No Oteryn "
            "identity is minted; the committed epoch-2 keys (B1b) are only read. "
            "Delivery-task eligibility, field mapping and Presentation binding are "
            "out of scope. "
            "The wiki-evidence fallback and the owner leftover-family table are both "
            "keyed by Oteryn registry key: a minted donor id joins under its epoch-2 "
            "key (B1b), a held id (no key) never matches -- see the module docstring."
        ),
        "principle": (
            "The donor is a source of facts only (items.xml, appearances.dat); "
            "Oteryn decides classification with its own rules, never upstream's."
        ),
        "totals": {
            "donor_items_xml_ids": len(donor["items"]),
            "base_items_xml_ids": len(base_items),
            "new_ids": len(new_ids),
            "new_ids_in_15_30_appearance_range": len(new_range_ids),
            "new_ids_older_than_15_25": len(older_ids),
        },
        "outcome": {
            "resolved": sum(by_profile.values()),
            "routed": sum(routed_counts.values()),
            "unresolved": sum(blocker_counts.values()),
        },
        "by_profile": dict(sorted(by_profile.items())),
        "family_profile_basis": dict(sorted(by_basis.items())),
        "family_profile_rule": dict(sorted(by_rule.items())),
        "wiki_evidence_fallback_resolved": by_basis.get("wiki_evidence_fallback", 0),
        "owner_leftover_table_resolved": by_rule.get(
            "owner_leftover_review_2026_09_28", 0
        ),
        "routed_non_item": [
            {"owner_reason": owner_key, "items": count}
            for owner_key, count in sorted(
                routed_counts.items(), key=lambda row: (-row[1], row[0])
            )
        ],
        "blockers_by_item_count": [
            {"blocker": blocker, "items": count}
            for blocker, count in sorted(
                blocker_counts.items(), key=lambda row: (-row[1], row[0])
            )
        ],
        "items": dict(sorted(rows.items(), key=lambda row: int(row[0]))),
    }
    return result_doc, new_ids


def census_document_bytes(result):
    return (
        json.dumps(result, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    ).encode("utf-8")


def check_converter_parity(base_source):
    """Prove `classify_donor_item` still mirrors `engine_items.convert_item`.

    Runs both over every pinned base id, with the base acting as its own donor, and
    requires identical outcomes. Ids that `convert_item` resolves through a
    registry-key-keyed source (wiki evidence, the owner leftover table) or cannot key
    at all are skipped: a donor id can never have a registry key.
    """
    base_sources = load_engine_sources("crystal", base_source)
    base = {"items": base_sources["items"], "appearances": base_sources["appearances"]}
    compared = 0
    mismatches = []
    for item_id in sorted(base_sources["items"]):
        item, _deps, report = convert_item(base_sources, item_id)
        evidence = (item or {}).get("family_profile_evidence") or {}
        if (item or {}).get("family_profile_basis") == "wiki_evidence_fallback":
            continue
        if evidence.get("rule") == "owner_leftover_review_2026_09_28":
            continue
        if "identity_not_in_b1_catalog" in report.get("blockers", ()):
            continue
        if report.get("routed_non_item"):
            expected = (
                "routed",
                report["routed_non_item"]["owner"],
                report["routed_non_item"]["reason"],
            )
        elif item:
            expected = ("resolved", item["family_profile"])
        else:
            expected = ("unresolved",)
        got = classify_donor_item(item_id, base, base_sources["items"], {}, {})
        if got["outcome"] == "routed":
            actual = ("routed", got["owner"], got["reason"])
        elif got["outcome"] == "resolved":
            actual = ("resolved", got["family_profile"])
        else:
            actual = ("unresolved",)
        compared += 1
        if actual != expected:
            mismatches.append((item_id, expected, actual))
    assert not mismatches, f"donor/convert_item parity broken: {mismatches[:5]}"
    return compared


def self_check(result_doc, new_ids):
    totals = result_doc["totals"]
    assert totals["new_ids"] == 412, totals
    assert totals["new_ids_in_15_30_appearance_range"] == 244, totals
    assert totals["new_ids_older_than_15_25"] == 168, totals
    assert str(SELF_CHECK_ROUTED_ID) in result_doc["items"], "known routed id missing"
    routed_row = result_doc["items"][str(SELF_CHECK_ROUTED_ID)]
    assert routed_row["outcome"] == "routed", routed_row
    assert routed_row["owner"] == "WorldObject" and routed_row["reason"] == "corpse", (
        routed_row
    )

    attribute_row = result_doc["items"][str(SELF_CHECK_ATTRIBUTE_ID)]
    assert attribute_row["outcome"] == "resolved", attribute_row
    assert attribute_row["family_profile"] == "material_valuable", attribute_row
    assert attribute_row["family_profile_basis"] is None, attribute_row

    # Since B2 its own itemid page (primarytype Furniture) outranks the wrap target,
    # exactly as in `convert_item`; the profile is the same either way.
    wrap_row = result_doc["items"][str(SELF_CHECK_WRAP_TARGET_ID)]
    assert wrap_row["outcome"] == "resolved", wrap_row
    assert wrap_row["family_profile"] == "decoration", wrap_row
    assert wrap_row["family_profile_basis"] == "wiki_evidence_fallback", wrap_row

    # B2: a minted donor id joins the wiki evidence under its epoch-2 key; these four
    # carry an admitted primarytype on their own itemid page. The owner leftover table
    # has no epoch-2 key at all, and a held id keeps its provisional key.
    for item_id, profile in SELF_CHECK_WIKI_EVIDENCE.items():
        row = result_doc["items"][str(item_id)]
        assert row["outcome"] == "resolved", row
        assert row["family_profile"] == profile, row
        assert row["family_profile_basis"] == "wiki_evidence_fallback", row
        assert row["key"].startswith("oteryn:item.registry.i000"), row
    for item_id in SELF_CHECK_HELD_IDS:
        assert result_doc["items"][str(item_id)]["key"] == donor_key(item_id)
    assert result_doc["owner_leftover_table_resolved"] == 0, result_doc
    print(
        json.dumps(
            {
                "self_check": "ok",
                "new_ids": totals["new_ids"],
                "sample_ids": [
                    SELF_CHECK_ROUTED_ID,
                    SELF_CHECK_ATTRIBUTE_ID,
                    SELF_CHECK_WRAP_TARGET_ID,
                ],
            }
        )
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--donor-source", type=Path, required=True)
    parser.add_argument("--base-source", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--self-check", action="store_true")
    parser.add_argument(
        "--check",
        action="store_true",
        help=(
            "regenerate the census in memory and diff it against the committed "
            "samples file; exits 1 on drift and never writes"
        ),
    )
    args = parser.parse_args()

    result_doc, new_ids = build_census(args.donor_source, args.base_source)
    out_path = args.out or ROOT / "samples" / DEFAULT_SAMPLE_NAME
    candidate_bytes = census_document_bytes(result_doc)

    if args.check:
        if not out_path.is_file():
            raise SystemExit(
                f"no committed donor census at {out_path} to check against"
            )
        committed_bytes = out_path.read_bytes()
        if candidate_bytes != committed_bytes:
            raise SystemExit(
                f"donor census drift detected: regenerated census differs from "
                f"committed {out_path}"
            )
        print(json.dumps({"check": "ok", "out": str(out_path)}))
    else:
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_bytes(candidate_bytes)
        print(json.dumps({"totals": result_doc["totals"], "out": str(out_path)}))

    if args.self_check:
        self_check(result_doc, new_ids)
        compared = check_converter_parity(args.base_source)
        print(json.dumps({"converter_parity": "ok", "compared": compared}))


if __name__ == "__main__":
    main()
