#!/usr/bin/env python3
"""Fail-closed tests for the frozen ITEM-ID-1 alias table and D149 tombstones.

Synthetic cases cover each §4.5 rule the verifier enforces. The committed-data case
verifies the table and derives the named retired keys independently of the table: from
the string literals of the historical key reproduction in `cw2_b1_import.rs` (the 64
`native_key` fields and the R7-P04 target).
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import item_id_alias_table as table

RUST_SOURCE = table.ROOT / "apps/game-server/src/content/cw2_b1_import.rs"
INDEX = {"retired_ids": [9]}
UNION = {5, 6, 9, 3031}


def expect_error(code: str, fn, *args) -> None:
    try:
        fn(*args)
    except table.AliasTableError as exc:
        assert str(exc).startswith(code), f"expected {code}, got {exc}"
    else:
        raise AssertionError(f"expected AliasTableError {code}")


def alias(key: str, object_id: int, version: int = 1, target: str | None = None):
    return {
        "key": key,
        "version": version,
        "state": table.ALIAS,
        "target": target or table.tibia_key(object_id),
        "evidence": {"source_item_id": object_id},
    }


def retired(key: str, version: int = 1) -> dict:
    return {"key": key, "version": version, "state": table.RETIRED}


def space(*extra: dict) -> list[dict]:
    """A synthetic table over the whole retired key space; `extra` rows replace or append."""
    rows = {
        f"oteryn:item.registry.i{sequence:08}": retired(
            f"oteryn:item.registry.i{sequence:08}"
        )
        for sequence in table.REGISTRY_SEQUENCES
    }
    rows["oteryn:item.registry.i00002921"] = alias(
        "oteryn:item.registry.i00002921", 3031
    )
    rows["oteryn:item.currency.gold_coin"] = alias(
        "oteryn:item.currency.gold_coin", 3031
    )
    for index in range(table.NAMED_KEYS - 1):
        key = f"oteryn:item.named.k{index:02}"
        rows[key] = retired(key)
    entries = {(entry["key"], entry["version"]): entry for entry in rows.values()}
    for row in extra:
        entries[(row["key"], row["version"])] = row
    return [entries[slot] for slot in sorted(entries)]


def synthetic_table_checks() -> None:
    counts = table.verify_table(space(), INDEX, UNION)
    assert counts["alias"] == 2 and counts["alias_targets"] == 1, counts

    expect_error(
        "ALIAS_TARGET_NOT_OWN_ID",
        table.verify_table,
        space(
            alias("oteryn:item.registry.i00000001", 5, target="oteryn:item.tibia.i6")
        ),
        INDEX,
        UNION,
    )
    expect_error(
        "ALIAS_TARGET_NOT_ADMITTED",
        table.verify_table,
        space(alias("oteryn:item.registry.i00000001", 7)),
        INDEX,
        UNION,
    )
    expect_error(
        "CANONICAL_KEY_RETIRED",
        table.verify_table,
        space(alias("oteryn:item.tibia.i5", 5)),
        INDEX,
        UNION,
    )
    expect_error(
        "ALIAS_TARGET_COLLISION",
        table.verify_table,
        space(
            alias("oteryn:item.registry.i00000001", 5),
            alias("oteryn:item.registry.i00000002", 5),
        ),
        INDEX,
        UNION,
    )
    # The allowlisted pair is exact: a third key on i3031 is a collision.
    expect_error(
        "ALIAS_TARGET_COLLISION",
        table.verify_table,
        space(alias("oteryn:item.registry.i00000003", 3031)),
        INDEX,
        UNION,
    )
    expect_error(
        "SHARED_TARGET_ALLOWLIST_STALE",
        table.verify_table,
        space(retired("oteryn:item.currency.gold_coin")),
        INDEX,
        UNION,
    )
    # A later alias may supersede RETIRED_WITHOUT_SUCCESSOR; nothing supersedes an alias.
    superseded = space(alias("oteryn:item.registry.i00000004", 6, version=2))
    assert table.verify_table(superseded, INDEX, UNION)["alias"] == 3
    expect_error(
        "ARCHITECTURE_ESCALATION_REQUIRED:ALIAS_SUPERSEDED",
        table.verify_table,
        space(
            alias("oteryn:item.registry.i00002921", 3031, version=1),
            alias("oteryn:item.registry.i00002921", 3031, version=2),
        ),
        INDEX,
        UNION,
    )
    expect_error(
        "VERSION_CHAIN",
        table.verify_table,
        space(alias("oteryn:item.registry.i00000004", 6, version=3)),
        INDEX,
        UNION,
    )
    missing = [e for e in space() if e["key"] != "oteryn:item.registry.i00000010"]
    expect_error("RETIRED_KEY_SPACE", table.verify_table, missing, INDEX, UNION)
    expect_error(
        "RETIRED_KEY_FORMAT",
        table.verify_table,
        space(retired("oteryn:item.Bad Key")),
        INDEX,
        UNION,
    )
    expect_error("ENTRY_ORDER", table.verify_table, space()[::-1], INDEX, UNION)
    expect_error("INVALID_TIBIA_ID", table.tibia_key, 0)


def named_keys_from_rust() -> set[str]:
    text = RUST_SOURCE.read_text(encoding="utf-8")
    batch = set(re.findall(r'\bnative_key: "(oteryn:item\.[^"]+)"', text))
    assert len(batch) == 64, len(batch)
    gold = re.findall(r'pub const R7_P04_GOLD_COIN_KEY: &str = "([^"]+)";', text)
    assert gold == ["oteryn:item.currency.gold_coin"], gold
    return batch | set(gold)


def committed_table() -> dict[str, int]:
    counts = table.verify()
    entries = json.loads(table.ALIAS_TABLE.read_bytes())["entries"]
    named = {
        e["key"] for e in entries if not e["key"].startswith("oteryn:item.registry.")
    }
    derived = named_keys_from_rust()
    assert named == derived, sorted(named ^ derived)[:5]
    assert all(e["state"] == table.ALIAS for e in entries if e["key"] in named)
    return counts


def main() -> None:
    synthetic_table_checks()
    counts = committed_table()
    print(
        f"item_id_alias_table self-test: PASS retired_keys={counts['retired_keys']} named={table.NAMED_KEYS}"
    )


if __name__ == "__main__":
    main()
