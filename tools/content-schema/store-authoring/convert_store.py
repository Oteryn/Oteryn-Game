"""Converts the pinned Canary (`47dfd51f`) and Crystal (`ff7ede5`) GameStore catalog
sources into a Store offer authoring catalog (`store-offer.schema.json`).

Canary (version 2.0, actively maintained, 27 offer types) is the primary content source
for the authored catalog; Crystal (version 1.1, 28 offer types) is read for cross-engine
verification only (`store_census.py`), never merged into the authored catalog by this
converter. `--engine crystal` still runs the full pipeline (used by the census).

Reading: `data/libs/gamestore/constants.lua` (Canary) / `data/modules/scripts/gamestore/init.lua`
(both) for `GameStore.OfferTypes`/`States`/`SubActions`; `data/modules/scripts/gamestore/catalog/init.lua`
+ `parent_categories.lua` + each leaf module (Canary); `gamestore.lua`'s `GameStore.Categories`
literal (Crystal). Every source file is git-blob-digest verified before it is read
(`EXPECTED_DIGESTS`); a missing or drifted pinned file is a hard error. Parsing itself is
`lua_lite.py`, a small strict reader for exactly these table literals, never a Lua
interpreter — an unrecognized top-level shape is a hard parse error, not a silent skip.

Item identity is `item_bindings.py` (the committed Crystal id -> Item key bindings); an
`itemtype` id with no binding is reported in `conversion_report.unresolved_item_refs` and
its offer is excluded from `catalog.offers`, never invented. `HIRELING_SKILL`/
`HIRELING_OUTFIT` offers' `id` is a lookup-table reference (`HIRELING_SKILLS.*`/
`HIRELING_OUTFITS.*`) this reader cannot resolve; their `product.hireling_typed.typed_id`
is `null` and the offer is still emitted (an explicit, documented omission, not a
validation-blocking one) — see `store_constants.OFFER_TYPES`.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import lua_lite as ll
from item_bindings import item_ref, load_item_bindings
from store_constants import offer_type_id_by_engine, slug

REPO_ROOT = Path(__file__).resolve().parents[3]
DEFINITION_REVISION = "definition-r1"

EXPECTED_DIGESTS = {
    (
        "canary",
        "data/libs/gamestore/constants.lua",
    ): "0e4311e8d7fdf7bbf8084bfd3ebabc1d4806b521",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/init.lua",
    ): "1bcb0bcdafdce6f2d24449a133cd74ead9f9a43d",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/parent_categories.lua",
    ): "d902c465e910548ada8c6f9ab20d15144f24661d",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/beds.lua",
    ): "efadb4080798edd4027ef41b4497e9d8fc13ea9c",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/boost.lua",
    ): "19368571bf95f621c830c5e4266bb4b1357a547b",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/consumables_blessings.lua",
    ): "a139cf8bd945552495f0554a40602b72ebdad7bf",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/consumables_casks.lua",
    ): "f2ef15154ac2e806592aadf043f74f12d207bbfe",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/consumables_exercise_weapons.lua",
    ): "3c882347213fd77122e7cb9332949be9ad436a23",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/consumables_kegs.lua",
    ): "a09a9d748c5c0effefed9cfe623052ecf3f53966",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/consumables_potions.lua",
    ): "7ae4ba63f31cd9e03aa57b56fe30d74f9a1cfc99",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/consumables_runes.lua",
    ): "4c0b883c34c6b7c38c7dcd6b65210c6de89c9429",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/cosmetics_mounts.lua",
    ): "b8d96e32b5033c4ca18eb4b8295b4b4c913e77f1",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/cosmetics_outfits.lua",
    ): "c2dd68245b5907d319bb7ea1d0803234f46be9eb",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/extras_extras_services.lua",
    ): "98f84b784811beead7224a9790ac88d19f3676f2",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/extras_usefull_things.lua",
    ): "803612a0aa309bdf917f71706f52f4d7de63db36",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/house_decorations.lua",
    ): "d5ed3078f5c93c37a95943cb6647578116c191f7",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/house_furniture.lua",
    ): "137800ce7344d0965f4b38bfcb2059a21c370ec9",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/house_hireling_dresses.lua",
    ): "caa32f12e80431eb96f86488efe58710c3dc585c",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/house_hirelings.lua",
    ): "66200defef96547c8ddad98f78429b76d6a8b773",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/house_upgrades.lua",
    ): "ce601f20b2ef320a46936c7edec01b7fcd32ce74",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/premium_time.lua",
    ): "a4bff508cd1620d0c8dd43fff617afb175917710",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/tournament_exclusive_offers.lua",
    ): "ef85c87ae1cc5f46c4db89e07a9ac5cbf25ffeab",
    (
        "canary",
        "data/modules/scripts/gamestore/catalog/tournament_tickets.lua",
    ): "e19758e42b3a411652f2545423230b61ae55b897",
    (
        "crystal",
        "data/modules/scripts/gamestore/init.lua",
    ): "8f02b5d4c26d2457f002dcdb445a8a979f37f4c2",
    (
        "crystal",
        "data/modules/scripts/gamestore/gamestore.lua",
    ): "87f3d69499bb3f49e75c43fa452d3d18620d9e89",
}

CANARY_CONSTANTS_FILE = "data/libs/gamestore/constants.lua"
CRYSTAL_CONSTANTS_FILE = "data/modules/scripts/gamestore/init.lua"
STATE_NAME_BY_ID = {0: "none", 1: "new", 2: "sale", 3: "timed"}


class ConvertError(Exception):
    pass


def git_blob(data: bytes) -> str:
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def read_verified(engine: str, root: Path, rel_path: str) -> str:
    path = root / rel_path
    if not path.is_file():
        raise ConvertError(f"missing pinned source file: {rel_path}")
    data = path.read_bytes().replace(b"\r\n", b"\n")
    digest = git_blob(data)
    expected = EXPECTED_DIGESTS.get((engine, rel_path))
    if expected is None:
        raise ConvertError(f"no pinned digest recorded for ({engine}, {rel_path})")
    if digest != expected:
        raise ConvertError(f"{rel_path}: digest {digest} != pinned {expected}")
    return data.decode("utf-8")


def load_constants(engine: str, root: Path) -> dict:
    rel = CANARY_CONSTANTS_FILE if engine == "canary" else CRYSTAL_CONSTANTS_FILE
    chunk = ll.parse_chunk(read_verified(engine, root, rel))
    return ll.flatten_constants(chunk.assignments, {})


def load_canary_categories(root: Path) -> list[dict]:
    init_chunk = ll.parse_chunk(
        read_verified("canary", root, "data/modules/scripts/gamestore/catalog/init.lua")
    )
    modules = ll.resolve(init_chunk.assignments["modules"], {}, {})
    parent_chunk = ll.parse_chunk(
        read_verified(
            "canary",
            root,
            "data/modules/scripts/gamestore/catalog/parent_categories.lua",
        )
    )
    inline = ll.resolve(parent_chunk.assignments["__return__"], {}, {})
    categories = []
    for name in modules:
        if name in inline:
            categories.append(inline[name])
            continue
        rel = f"data/modules/scripts/gamestore/catalog/{name}.lua"
        chunk = ll.parse_chunk(read_verified("canary", root, rel))
        locals_ = {k: v for k, v in chunk.assignments.items() if k != "__return__"}
        # resolve locals first (string.format's dependent locals must resolve before use)
        resolved_locals: dict = {}
        for k, v in locals_.items():
            resolved_locals[k] = ll.resolve(v, resolved_locals, {})
        categories.append((chunk.assignments["__return__"], resolved_locals, rel))
    return categories, modules


def load_crystal_categories(root: Path) -> list:
    chunk = ll.parse_chunk(
        read_verified("crystal", root, "data/modules/scripts/gamestore/gamestore.lua")
    )
    locals_: dict = {}
    for k, v in chunk.assignments.items():
        if k in ("GameStore.Categories", "__return__"):
            continue
        locals_[k] = ll.resolve(v, locals_, {})
    return [
        (
            chunk.assignments["GameStore.Categories"],
            locals_,
            "data/modules/scripts/gamestore/gamestore.lua",
        )
    ]


def default_coin_type(engine: str, description) -> str:
    if engine == "canary":
        return "transferable"
    if isinstance(description, str) and "{transferableprice}" in description:
        return "transferable"
    return "coin"


def convert(engine: str, root: Path, repo_root: Path = REPO_ROOT) -> dict:
    constants = load_constants(engine, root)
    offer_type_by_id = offer_type_id_by_engine(engine)
    item_index = load_item_bindings(repo_root)

    if engine == "canary":
        raw_categories, module_order = load_canary_categories(root)
        # Canary's leaf files are already (expr, locals, rel); parent-inline ones are
        # bare dicts (already fully literal, no locals needed).
        entries = []
        for name, item in zip(module_order, raw_categories):
            if isinstance(item, dict):
                entries.append(
                    (
                        ("table_record", {k: _lit(v) for k, v in item.items()}),
                        {},
                        "data/modules/scripts/gamestore/catalog/parent_categories.lua",
                    )
                )
            else:
                entries.append(item)
    else:
        cat_list_expr, locals_, rel = load_crystal_categories(root)[0]
        cats = ll.resolve(cat_list_expr, locals_, constants)
        entries = [
            (("table_record", {k: _lit(v) for k, v in c.items()}), {}, rel)
            for c in cats
        ]

    report = {
        "unresolved_item_refs": [],
        "unresolved_engine_offer_ids": [],
        "excluded_offers": [],
        "skipped_lua_constructs": [],
    }
    categories_out = []
    offers_out = []
    category_key_by_name: dict[str, str] = {}
    offer_key_counts: dict[str, int] = {}

    for cat_expr, locals_, rel in entries:
        cat = ll.resolve(cat_expr, locals_, constants)
        name = cat["name"]
        cat_key = f"oteryn:store.category.{slug(name)}"
        if name in category_key_by_name:
            raise ConvertError(f"duplicate category name {name!r}")
        category_key_by_name[name] = cat_key

    for cat_expr, locals_, rel in entries:
        cat = ll.resolve(cat_expr, locals_, constants)
        name = cat["name"]
        cat_key = category_key_by_name[name]
        parent_name = cat.get("parent")
        parent_key = category_key_by_name.get(parent_name) if parent_name else None
        if parent_name and parent_key is None:
            raise ConvertError(
                f"category {name!r} references unknown parent {parent_name!r}"
            )
        state = _state_name(cat.get("state"))
        has_offers = "offers" in cat
        has_subclasses = "subclasses" in cat
        if has_offers == has_subclasses:
            raise ConvertError(
                f"category {name!r} must have exactly one of offers/subclasses"
            )
        categories_out.append(
            {
                "identity": {"key": cat_key, "revision": DEFINITION_REVISION},
                "display_name": name,
                "parent": parent_key,
                "icons": cat["icons"],
                "rookgaard": bool(cat["rookgaard"]),
                "state": state,
                "shape": "group" if has_subclasses else "leaf",
                "subclass_names": cat.get("subclasses"),
            }
        )
        if not has_offers:
            continue
        for offer in cat["offers"]:
            built = _build_offer(
                engine,
                offer,
                cat_key,
                cat_key_slug=slug(name),
                offer_key_counts=offer_key_counts,
                rel=rel,
                name=name,
                item_index=item_index,
                offer_type_by_id=offer_type_by_id,
                report=report,
            )
            if built is not None:
                offers_out.append(built)

    return {
        "catalog": {"categories": categories_out, "offers": offers_out},
        "conversion_report": report,
    }


def _lit(value):
    """Wraps an already-resolved Python value back into a lua_lite expr node so a
    dict produced by `ll.resolve` (parent_categories.lua's inline group tables, or
    Crystal's fully-resolved category list) can be re-resolved uniformly with the
    leaf-file code path above."""
    if isinstance(value, ll.Unresolved):
        return ("unresolved", value.text)
    if isinstance(value, dict):
        return ("table_record", {k: _lit(v) for k, v in value.items()})
    if isinstance(value, list):
        return ("table_array", [_lit(v) for v in value])
    if isinstance(value, bool) or value is None:
        return ("lit", value)
    if isinstance(value, (int, float)):
        return ("num", value)
    if isinstance(value, str):
        return ("str", value)
    raise ConvertError(f"cannot re-literal-wrap value {value!r}")


def _state_name(value) -> str:
    if value is None:
        return "none"
    if isinstance(value, int) and value in STATE_NAME_BY_ID:
        return STATE_NAME_BY_ID[value]
    raise ConvertError(f"unresolvable state value {value!r}")


def _build_offer(
    engine,
    offer,
    cat_key,
    cat_key_slug,
    offer_key_counts,
    rel,
    name,
    item_index,
    offer_type_by_id,
    report,
):
    offer_name = offer["name"]
    base_key = f"oteryn:store.offer.{cat_key_slug}.{slug(offer_name)}"
    n = offer_key_counts.get(base_key, 0) + 1
    offer_key_counts[base_key] = n
    offer_key = base_key if n == 1 else f"{base_key}-{n}"

    raw_type = offer.get("type")
    type_id = raw_type if raw_type is not None else 0
    if isinstance(type_id, ll.Unresolved) or type_id not in offer_type_by_id:
        raise ConvertError(
            f"offer {offer_name!r} ({rel}): unresolvable/unknown offer_type {raw_type!r}"
        )
    offer_type = offer_type_by_id[type_id]

    description = offer.get("description")
    coin_type = default_coin_type(engine, description)
    state = _state_name(offer.get("state"))

    engine_id = offer.get("id")
    engine_offer_id = engine_id if isinstance(engine_id, int) else None
    if isinstance(engine_id, ll.Unresolved):
        report["unresolved_engine_offer_ids"].append(
            {"category": name, "offer": offer_name, "source": engine_id.text}
        )

    home = bool(offer.get("home", False))
    number = offer.get("number")
    hireling_slot_number = number if isinstance(number, int) else None

    product = _build_product(
        offer_type, offer, engine_id, item_index, name, offer_name, report
    )
    if product is None:
        report["excluded_offers"].append(
            {
                "category": name,
                "offer": offer_name,
                "offer_type": offer_type,
                "reason": "unresolved product reference",
            }
        )
        return None

    return {
        "identity": {"key": offer_key, "revision": DEFINITION_REVISION},
        "category": cat_key,
        "display_name": offer_name,
        "icons": offer["icons"],
        "price_coins": offer["price"],
        "coin_type": coin_type,
        "offer_type": offer_type,
        "state": state,
        "description": description if isinstance(description, str) else None,
        "home": home,
        "hireling_slot_number": hireling_slot_number,
        "engine_offer_id": engine_offer_id,
        "product": product,
    }


def _resolve_items(itemtype, item_index, category, offer_name, report):
    ids = itemtype if isinstance(itemtype, list) else [itemtype]
    refs = []
    for item_id in ids:
        ref = item_ref(item_index, item_id) if isinstance(item_id, int) else None
        if ref is None:
            report["unresolved_item_refs"].append(
                {"category": category, "offer": offer_name, "itemtype": item_id}
            )
            return None
        refs.append(ref)
    return refs


def _build_product(
    offer_type, offer, engine_id, item_index, category, offer_name, report
):
    if offer_type in ("item", "stackable", "house", "item_bed", "item_unique"):
        refs = _resolve_items(
            offer.get("itemtype"), item_index, category, offer_name, report
        )
        if refs is None:
            return None
        return {
            "kind": "item",
            "item": {
                "item_refs": refs,
                "count": offer.get("count")
                if isinstance(offer.get("count"), int)
                else None,
                "charges": None,
            },
        }
    if offer_type == "charges":
        refs = _resolve_items(
            offer.get("itemtype"), item_index, category, offer_name, report
        )
        if refs is None:
            return None
        # Canary's own catalog is inconsistent here: every `charges` offer uses the
        # `charges` field except "Ultimate Health Keg" (itemtype 25906), which uses
        # `count` for the same use-count meaning. Both are accepted.
        uses = (
            offer.get("charges")
            if offer.get("charges") is not None
            else offer.get("count")
        )
        return {
            "kind": "item",
            "item": {"item_refs": refs, "count": None, "charges": uses},
        }
    if offer_type in ("outfit", "outfit_addon"):
        sex = offer.get("sexId") or {}
        return {
            "kind": "outfit",
            "outfit": {
                "female_look_type": sex.get("female"),
                "male_look_type": sex.get("male"),
                "addon": offer.get("addon", 0),
            },
        }
    if offer_type == "mount":
        if not isinstance(engine_id, int):
            return None
        return {"kind": "mount", "mount": {"mount_id": engine_id}}
    if offer_type == "hireling":
        sex = offer.get("sexId") or {}
        return {
            "kind": "hireling",
            "hireling": {
                "female_look_type": sex.get("female"),
                "male_look_type": sex.get("male"),
            },
        }
    if offer_type in ("hireling_skill", "hireling_outfit"):
        typed_id = engine_id if isinstance(engine_id, int) else None
        return {"kind": "hireling_typed", "hireling_typed": {"typed_id": typed_id}}
    if offer_type == "blessings":
        bless_id = offer.get("blessid")
        count = offer.get("count", 1)
        if not isinstance(bless_id, int):
            return None
        return {
            "kind": "blessing",
            "blessing": {
                "bless_id": bless_id,
                "count": count if isinstance(count, int) else 1,
            },
        }
    if offer_type == "allblessings":
        count = offer.get("count", 1)
        return {
            "kind": "blessing_bundle",
            "blessing_bundle": {"count": count if isinstance(count, int) else 1},
        }
    if offer_type == "premium":
        days = offer.get("validUntil")
        if not isinstance(days, int):
            return None
        return {"kind": "premium_time", "premium_time": {"days": days}}
    return {"kind": "service"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", choices=("canary", "crystal"), default="canary")
    parser.add_argument("--root", required=True, help="pinned engine checkout root")
    parser.add_argument(
        "--out", help="write the {catalog, conversion_report} JSON here"
    )
    args = parser.parse_args()

    result = convert(args.engine, Path(args.root))
    text = json.dumps(result, indent=2, sort_keys=False) + "\n"
    if args.out:
        Path(args.out).write_text(text, encoding="utf-8")
        print(f"wrote {args.out}")
    else:
        print(text)


if __name__ == "__main__":
    main()
