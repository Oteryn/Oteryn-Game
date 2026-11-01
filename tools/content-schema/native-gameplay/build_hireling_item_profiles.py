#!/usr/bin/env python3
"""E12: normalize the complete pinned Hireling Item closure with the native importer.

No source assets are copied. The output contains bound identities, normalized
facts and their exact source hashes. Existing richer policies are merged by the
ordinary register_spell_families owner, never overwritten by this capture.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import subprocess
from pathlib import Path

import build_source_item_profiles as native

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = "tools/content-schema/native-gameplay/hireling-item-profiles.json"
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
CATALOGUE = "crates/gameplay/domain/src/domain/hireling/catalogue/generated.rs"
LAMP = 29432
COOK = set(range(29408, 29417))  # ANSWER Q10: carrot pie29409, never toolbox35173.
COMPOSITION = {2854, 2856, 3031, 3035, 3043, 23398, LAMP}
SOURCE_PATHS = ("data-global/npc/hireling.lua", "data/items/items.xml",
                "data/items/appearances.dat", "src/items/items.hpp",
                "src/items/items.cpp", "src/items/functions/item/item_parse.cpp",
                "src/game/game.cpp", "src/lua/functions/creatures/npc/npc_functions.cpp")


def catalogue_parser():
    path = ROOT / "tools/content-schema/hireling/generate_catalogue.py"
    spec = importlib.util.spec_from_file_location("hireling_catalogue_source", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def required_items(source):
    parser = catalogue_parser()
    _categories, rows, packages, disabled = parser.extract(source)
    trader = {row[1] for row in rows}
    ingredients = {item for _, _, items, _ in packages for item, _ in items}
    required = trader | ingredients | COOK | COMPOSITION
    if len(trader) != 1409 or len(ingredients) != 69:
        raise ValueError("HIRELING_ITEM_CLOSURE_COUNTS")
    return required, {"trade_rows": len(rows), "trader_items": len(trader),
                      "packages": len(packages), "package_ingredients": len(ingredients),
                      "cook_items": sorted(COOK), "composition_items": sorted(COMPOSITION),
                      "disabled_source_rows": len(disabled)}


def build(checkout):
    actual = subprocess.check_output(["git", "-C", str(checkout), "rev-parse", "HEAD"], text=True).strip()
    if actual != REVISION:
        raise ValueError("HIRELING_DONOR_REVISION")
    # Read the exact committed bytes even if an unrelated file in the reference
    # checkout is dirty; a working-tree substitution can never become evidence.
    captured = {path: subprocess.check_output(["git", "-C", str(checkout), "show", f"{REVISION}:{path}"])
                for path in SOURCE_PATHS}
    source = captured[SOURCE_PATHS[0]].decode()
    required, counts = required_items(source)
    rendered = catalogue_parser().generate(source).encode()
    if (ROOT / CATALOGUE).read_bytes() != rendered:
        raise ValueError("HIRELING_GENERATED_CATALOGUE_DRIFT")
    xml, appearances = captured[SOURCE_PATHS[1]], captured[SOURCE_PATHS[2]]
    index = native.xml_index(xml)
    objects = native.engine_items.load_appearance_objects(appearances)
    if required - set(index) or required - set(objects):
        raise ValueError(f"HIRELING_SOURCE_ITEM_MISSING:{sorted(required - (set(index) & set(objects)))}")
    bindings = native.binding_index(native.BINDINGS.read_bytes())
    physical = {row["server_item_id"]: row for row in native.policies(appearances, xml, required)}
    records = []
    nonportable = []
    wrapped = []
    _, trade_rows, _, _ = catalogue_parser().extract(source)
    buy_ids = {row[1] for row in trade_rows if row[2] > 0}
    for number in sorted(required):
        record = native.profile(number, index[number], objects[number], physical[number], bindings, None)
        if number == LAMP:
            facts = record["semantics"]
            if (facts["physical"]["value"]["movable"] != native.K(False)
                    or facts["physical"]["value"]["weight"] != native.K(1000)
                    or facts["stack"]["value"]["stackable"] != native.K(False)):
                raise ValueError("HIRELING_IMMOVABLE_LAMP_SOURCE")
            # Crystal's Store produces this nonmovable lamp in the Store Inbox.
            # This grants no carried inventory or Ground destination.
            record["admission"]["legal_destinations"] = ["CharacterInbox"]
        elif "CharacterInventory" not in record["admission"]["legal_destinations"]:
            # A literal trade row is not a portability grant. In particular,
            # Crystal sells wrappable furniture as kit23398, while stale source
            # sell-only rows must retain their actual nonpickupable facts.
            nonportable.append(number)
        flags = objects[number]["flags"]
        wrap_default = 23398 if flags.get("flags.wrap") or flags.get("flags.unwrap") else 0
        wrap_target = int(native.attrs(index[number]).get("wrapableto", wrap_default))
        if number in buy_ids and wrap_target > 0:
            # items.hpp::isWrappable and game.cpp::createItemBatch use this
            # predicate, then always create ITEM_DECORATION_KIT with unWrapId.
            wrapped.append({"offered_item_id": number, "delivered_item_id": 23398,
                            "unwrap_item_id": number})
        records.append(record)
    bound = native.bind_profiles({"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": records},
                                 bindings, set(index), independent_crystal_source=(REVISION, xml))
    lines = {}
    for line_number, line in enumerate(xml.decode().splitlines(), 1):
        if "<item " not in line:
            continue
        low = re.search(r'\b(?:id|fromid)="(\d+)"', line)
        high = re.search(r'\btoid="(\d+)"', line)
        if low:
            first, last = int(low[1]), int(high[1]) if high else int(low[1])
            for number in sorted(required):
                if first <= number <= last:
                    lines[str(number)] = f"data/items/items.xml:{line_number}"
    if set(map(int, lines)) != required:
        raise ValueError("HIRELING_SOURCE_ITEM_LINE_CLOSURE")
    return {"schema": "OTERYN_NATIVE_HIRELING_ITEM_PROFILES/v1", "source_revision": REVISION,
            "producer": "tools/content-schema/native-gameplay/build_hireling_item_profiles.py",
            "catalogue": {"path": CATALOGUE, "sha256": hashlib.sha256(rendered).hexdigest(), **counts},
            "source_files": [{"path": path, "sha256": hashlib.sha256(raw).hexdigest()}
                             for path, raw in captured.items()],
            "binding_table_sha256": hashlib.sha256(native.BINDINGS.read_bytes()).hexdigest(),
            "required_ids": sorted(required), "nonportable_ids": nonportable,
            "wrapped_trade_items": wrapped, "source_lines": lines, "records": bound["records"]}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("checkout", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    document = build(args.checkout)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()
    path = ROOT / OUTPUT
    if args.check:
        if path.read_bytes() != raw:
            raise SystemExit("HIRELING_ITEM_PROFILE_DRIFT")
    else:
        path.write_bytes(raw)
    print(f"HIRELING_NATIVE_ITEM_CLOSURE_VERIFIED items={len(document['required_ids'])} bytes={len(raw)}")
