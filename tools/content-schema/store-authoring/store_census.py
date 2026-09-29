"""Census of the converted Store catalog: per-engine counts and a cross-engine
difference summary. Evidence tooling, not Game truth — see `convert_store.py`.

Needs a pinned Canary and/or Crystal checkout (`--canary`/`--crystal`); there is no
network or bundled copy, so `--check`/`--self-check` are local-only, like the Item
package's population census.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path

import convert_store as cs
from store_constants import OFFER_TYPES

ROOT = Path(__file__).resolve().parent
CANARY_REVISION = "47dfd51f45280a59a1d3e50ba7edd573d7234446"
CRYSTAL_REVISION = "ff7ede593c69d4c658b382c97443e8155926924a"
SAMPLE_PATHS = {
    "canary": ROOT / "samples" / "store-census-canary-47dfd51f.json",
    "crystal": ROOT / "samples" / "store-census-crystal-ff7ede5.json",
}


def census_one(engine: str, root: Path) -> dict:
    result = cs.convert(engine, root)
    catalog, report = result["catalog"], result["conversion_report"]
    offers = catalog["offers"]
    return {
        "engine": engine,
        "source_revision": CANARY_REVISION if engine == "canary" else CRYSTAL_REVISION,
        "categories": len(catalog["categories"]),
        "leaf_categories": sum(
            1 for c in catalog["categories"] if c["shape"] == "leaf"
        ),
        "group_categories": sum(
            1 for c in catalog["categories"] if c["shape"] == "group"
        ),
        "offers": len(offers),
        "offers_per_type": dict(
            sorted(Counter(o["offer_type"] for o in offers).items())
        ),
        "offers_per_product_kind": dict(
            sorted(Counter(o["product"]["kind"] for o in offers).items())
        ),
        "item_backed_offers": sum(1 for o in offers if o["product"]["kind"] == "item"),
        "item_refs_resolved": sum(
            len(o["product"]["item"]["item_refs"])
            for o in offers
            if o["product"]["kind"] == "item"
        ),
        "item_refs_unresolved": len(report["unresolved_item_refs"]),
        "unresolved_item_refs": report["unresolved_item_refs"],
        "unresolved_engine_offer_ids": report["unresolved_engine_offer_ids"],
        "excluded_offers": report["excluded_offers"],
    }, catalog


def cross_engine_diff(canary_catalog: dict, crystal_catalog: dict) -> dict:
    def names(cat_list):
        return {c["display_name"] for c in cat_list}

    def offer_multiset(offers):
        return Counter(
            (o["category"].removeprefix("oteryn:store.category."), o["display_name"])
            for o in offers
        )

    canary_cats, crystal_cats = (
        names(canary_catalog["categories"]),
        names(crystal_catalog["categories"]),
    )
    canary_offers, crystal_offers = (
        offer_multiset(canary_catalog["offers"]),
        offer_multiset(crystal_catalog["offers"]),
    )
    only_canary_offers = [
        list(pair) for pair in (canary_offers - crystal_offers).elements()
    ]
    only_crystal_offers = [
        list(pair) for pair in (crystal_offers - canary_offers).elements()
    ]

    canary_by_type = Counter(o["offer_type"] for o in canary_catalog["offers"])
    crystal_by_type = Counter(o["offer_type"] for o in crystal_catalog["offers"])
    type_diff = {
        t: {
            "canary": canary_by_type.get(t, 0),
            "crystal": crystal_by_type.get(t, 0),
            "delta": canary_by_type.get(t, 0) - crystal_by_type.get(t, 0),
        }
        for t in sorted(set(canary_by_type) | set(crystal_by_type))
        if canary_by_type.get(t, 0) != crystal_by_type.get(t, 0)
    }
    numbering_diff = [
        {"offer_type": name, "canary_id": ids[0], "crystal_id": ids[1]}
        for name, ids in OFFER_TYPES.items()
        if ids[0] is not None and ids[1] is not None and ids[0] != ids[1]
    ]
    engine_only = [
        {"offer_type": name, "canary_id": ids[0], "crystal_id": ids[1]}
        for name, ids in OFFER_TYPES.items()
        if ids[0] is None or ids[1] is None
    ]
    return {
        "category_names_only_in_canary": sorted(canary_cats - crystal_cats),
        "category_names_only_in_crystal": sorted(crystal_cats - canary_cats),
        "offer_count_by_category_name_offer_name_only_in_canary": sorted(
            only_canary_offers
        ),
        "offer_count_by_category_name_offer_name_only_in_crystal": sorted(
            only_crystal_offers
        ),
        "offer_count_diff_by_type": type_diff,
        "offer_type_numbering_diff": numbering_diff,
        "offer_type_engine_only": engine_only,
        "coin_type_default_rule_diff": (
            "Canary: an offer with no explicit coinType always defaults to transferable. "
            "Crystal: defaults to coin, unless the offer description contains the literal "
            "marker '{transferableprice}', in which case transferable. Neither engine's "
            "catalog data sets coinType explicitly."
        ),
    }


SELF_CHECK_EXPECTED = {
    "canary": [
        {
            "offer": "Kraken Buoy Lamp",
            "price_coins": 60,
            "offer_type": "house",
            "count": 1,
        },
        {
            "offer": "Great Health Potion",
            "price_coins": 18,
            "offer_type": "stackable",
            "count": 100,
        },
    ],
    "crystal": [
        {
            "offer": "Great Health Potion",
            "price_coins": 18,
            "offer_type": "stackable",
            "count": 100,
        },
    ],
}


def self_check(engine: str, catalog: dict) -> None:
    for expected in SELF_CHECK_EXPECTED.get(engine, []):
        matches = [
            o for o in catalog["offers"] if o["display_name"] == expected["offer"]
        ]
        match = next(
            (o for o in matches if o["price_coins"] == expected["price_coins"]), None
        )
        assert match is not None, (
            f"{engine}: {expected['offer']!r} at price {expected['price_coins']} not found"
        )
        assert match["offer_type"] == expected["offer_type"], match
        assert match["product"]["item"]["count"] == expected["count"], match
        key = match["product"]["item"]["item_refs"][0]["key"]
        assert key, match
        print(
            f"self-check ok: {engine} {expected['offer']!r} @ {expected['price_coins']} -> {key}"
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--canary", help="pinned Canary checkout root")
    parser.add_argument("--crystal", help="pinned Crystal checkout root")
    parser.add_argument(
        "--check",
        action="store_true",
        help="regenerate in memory and diff against the committed samples",
    )
    parser.add_argument(
        "--self-check", action="store_true", help="assert a few exact known offers"
    )
    args = parser.parse_args()

    catalogs = {}
    censuses = {}
    for engine, root_arg in (("canary", args.canary), ("crystal", args.crystal)):
        if root_arg:
            census, catalog = census_one(engine, Path(root_arg))
            censuses[engine] = census
            catalogs[engine] = catalog

    if not censuses:
        parser.error("at least one of --canary/--crystal is required")

    if len(catalogs) == 2:
        diff = cross_engine_diff(catalogs["canary"], catalogs["crystal"])
        for engine, census in censuses.items():
            censuses[engine] = {**census, "cross_engine_differences": diff}

    if args.self_check:
        for engine, catalog in catalogs.items():
            self_check(engine, catalog)

    if args.check:
        ok = True
        for engine, census in censuses.items():
            committed = json.loads(SAMPLE_PATHS[engine].read_text(encoding="utf-8"))
            fresh = json.loads(
                json.dumps(census)
            )  # normalize tuples etc. to their JSON shape
            if committed != fresh:
                ok = False
                print(f"DRIFT: {engine} census differs from {SAMPLE_PATHS[engine]}")
        if not ok:
            raise SystemExit(1)
        print("PASS: --check (no drift)")
        return

    for engine, census in censuses.items():
        SAMPLE_PATHS[engine].write_text(
            json.dumps(census, indent=2, sort_keys=False) + "\n", encoding="utf-8"
        )
        print(f"wrote {SAMPLE_PATHS[engine]}")


if __name__ == "__main__":
    main()
