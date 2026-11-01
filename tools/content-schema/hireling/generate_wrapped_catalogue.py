#!/usr/bin/env python3
"""Reproduce the bounded furniture targets from the complete pinned native capture.

The ordinary Hireling catalogue preserves literal source IDs. This companion
owns only the source createItemBatch conversion into kit23398 + unWrapId.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
CAPTURE = ROOT / "tools/content-schema/native-gameplay/hireling-item-profiles.json"
CATALOGUE = ROOT / "crates/gameplay/domain/src/domain/hireling/catalogue/generated.rs"
RUST = ROOT / "crates/gameplay/domain/src/domain/hireling/catalogue/wrapped.rs"
MIGRATION = ROOT / "apps/game-server/migrations/0702_hireling_services.sql"
RUST_BEGIN = "// BEGIN GENERATED HIRELING FURNITURE TARGETS"
RUST_END = "// END GENERATED HIRELING FURNITURE TARGETS"
SQL_BEGIN = "-- BEGIN GENERATED HIRELING FURNITURE KITS"
SQL_END = "-- END GENERATED HIRELING FURNITURE KITS"


def generate(document, catalogue):
    if (document["schema"] != "OTERYN_NATIVE_HIRELING_ITEM_PROFILES/v1"
            or document["source_revision"] != REVISION
            or document["catalogue"]["sha256"] != hashlib.sha256(catalogue.encode()).hexdigest()):
        raise ValueError("HIRELING_FURNITURE_SOURCE_PIN")
    sources = {row["path"]: row["sha256"] for row in document["source_files"]}
    for path in ("data/items/items.xml", "data/items/appearances.dat", "src/items/items.hpp", "src/game/game.cpp"):
        if not re.fullmatch("[0-9a-f]{64}", sources.get(path, "")):
            raise ValueError("HIRELING_FURNITURE_SOURCE_PROOF")
    rows = [tuple(map(int, row)) for row in re.findall(
        r"TradeRow \{ category: (\d+), item_id: (\d+), buy: (\d+), sell: (\d+), sub_type: (\d+), source_line: (\d+) \}", catalogue)]
    if len(rows) != 1675:
        raise ValueError("HIRELING_FURNITURE_CATALOGUE_COUNT")
    captured = document["wrapped_trade_items"]
    ids = sorted(row["offered_item_id"] for row in captured)
    if (len(ids) != len(set(ids)) or len(ids) != 18
            or any(row["delivered_item_id"] != 23398 or row["unwrap_item_id"] != row["offered_item_id"] for row in captured)
            or not set(ids) <= set(document["required_ids"])):
        raise ValueError("HIRELING_FURNITURE_KIT_COMPOSITION")
    selected = [(index, row) for index, row in enumerate(rows) if row[1] in ids and row[2] > 0]
    if len(selected) != len(ids) or any(row[4] != 0 for _, row in selected):
        raise ValueError("HIRELING_FURNITURE_EXACT_BUY_ROWS")
    lines = ["    " + ", ".join(map(str, ids[index:index + 10])) + "," for index in range(0, len(ids), 10)]
    rust = RUST_BEGIN + "\n#[rustfmt::skip]\npub const WRAPPED_TRADE_ITEM_IDS: &[u32] = &[\n" + "\n".join(lines) + "\n];\n" + RUST_END
    values = [f"({index},'oteryn:item.tibia.i{row[1]}',{row[2]},{row[5]})" for index, row in selected]
    sql = SQL_BEGIN + "\nINSERT INTO game_house_furniture_kit_catalogue(source_row,target_key,price_gold,source_line) VALUES\n" + ",\n".join(values) + ";\n" + SQL_END
    return rust, sql


def replace_section(path, begin, end, generated, check):
    old = path.read_text()
    if old.count(begin) != 1 or old.count(end) != 1:
        raise ValueError(f"HIRELING_FURNITURE_GENERATED_SECTION:{path}")
    start, stop = old.index(begin), old.index(end) + len(end)
    new = old[:start] + generated + old[stop:]
    if check and new != old:
        raise ValueError(f"HIRELING_FURNITURE_GENERATED_DRIFT:{path}")
    if not check:
        path.write_text(new)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    rust, sql = generate(json.loads(CAPTURE.read_bytes()), CATALOGUE.read_text())
    replace_section(RUST, RUST_BEGIN, RUST_END, rust, args.check)
    replace_section(MIGRATION, SQL_BEGIN, SQL_END, sql, args.check)


if __name__ == "__main__":
    main()
