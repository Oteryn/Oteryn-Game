#!/usr/bin/env python3
"""Fail-closed tests for the ITEM-ID-1a alias table and D149 tombstones.

Synthetic cases cover each §4.2/§4.5 rule; the committed-data cases prove the table is
total, regenerates byte-identically, and that every old Item key still named in
`content/` or `apps/` resolves through an alias or a tombstone.
"""

from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import item_id_alias_table as table  # noqa: E402

ROW = {
    "key": "oteryn:item.registry.i00000007",
    "source_revision": "ff7ede593c69d4c658b382c97443e8155926924a",
    "source_item_id": 3031,
    "basis": "binding",
}
INDEX = {
    "files": [{"label": "crystal-ff7ede5"}, {"label": "client-15.30"}],
    "retired_ids": [9],
}
P = "a" * 64


def views(ff7: dict, client: dict) -> dict:
    return {"crystal-ff7ede5": ff7, "client-15.30": client}


def expect_error(code: str, fn, *args) -> None:
    try:
        fn(*args)
    except table.AliasTableError as exc:
        assert str(exc).startswith(code), f"expected {code}, got {exc}"
    else:
        raise AssertionError(f"expected AliasTableError {code}")


def synthetic_rules() -> None:
    both = views({3031: [3031, P, "r1"]}, {3031: [3031, P, "r2"]})
    entry = table.entry_for(ROW, INDEX, both, {})
    assert (
        entry["state"] == table.ALIAS and entry["target"] == "oteryn:item.tibia.i3031"
    ), entry
    assert entry["evidence"]["comparison_appearances"] == "client-15.30", entry
    assert entry["evidence"]["record"] == "EVOLVED", entry

    retired_id = views({3031: [3031, P, "r"]}, {})
    entry = table.entry_for(ROW, INDEX, retired_id, {})
    assert entry["evidence"]["comparison_appearances"] == "crystal-ff7ede5", entry
    assert entry["evidence"]["record"] == "UNCHANGED", entry

    tombstones = {ROW["key"]: {"definition_sha256": "b" * 64}}
    entry = table.entry_for(ROW, INDEX, views({}, {}), tombstones)
    assert entry["state"] == table.RETIRED and entry["tombstone_sha256"] == "b" * 64, (
        entry
    )
    assert "target" not in entry, entry

    expect_error("TOMBSTONE_MISSING", table.entry_for, ROW, INDEX, views({}, {}), {})
    expect_error(
        "UNPROVEN_ADMITTED_ID",
        table.entry_for,
        ROW,
        INDEX,
        views({}, {3031: [3031, P, "r"]}),
        {},
    )
    expect_error(
        "ARCHITECTURE_ESCALATION_REQUIRED:CONTINUITY_BREAK",
        table.entry_for,
        ROW,
        INDEX,
        views({3031: [3031, P, "r"]}, {3031: [3031, "c" * 64, "r"]}),
        {},
    )
    expect_error(
        "UNPINNED_SOURCE_REVISION",
        table.entry_for,
        {**ROW, "source_revision": "0" * 40},
        INDEX,
        both,
        {},
    )
    expect_error("INVALID_TIBIA_ID", table.tibia_key, 0)


def synthetic_table_checks() -> None:
    def alias(key: str, object_id: int, target: str | None = None) -> dict:
        return {
            "key": key,
            "state": table.ALIAS,
            "target": target or table.tibia_key(object_id),
            "evidence": {"source_item_id": object_id},
        }

    expect_error(
        "ALIAS_TARGET_NOT_OWN_ID",
        table.verify_table,
        [alias("oteryn:item.registry.i00000001", 5, "oteryn:item.tibia.i6")],
        INDEX,
    )
    expect_error(
        "CANONICAL_KEY_RETIRED",
        table.verify_table,
        [alias("oteryn:item.tibia.i5", 5)],
        INDEX,
    )
    collision = [
        alias("oteryn:item.registry.i00000001", 5),
        {
            **alias("oteryn:item.registry.i00000002", 6),
            "target": "oteryn:item.tibia.i5",
        },
    ]
    expect_error("ALIAS_TARGET_NOT_OWN_ID", table.verify_table, collision, INDEX)
    same_item = [
        alias("oteryn:item.registry.i00000001", 5),
        alias("oteryn:item.currency.x", 5),
    ]
    expect_error("COUNT_MISMATCH", table.verify_table, same_item, INDEX)


def committed_table() -> dict:
    table_bytes, tombstone_bytes = table.generate()
    assert table.ALIAS_TABLE.read_bytes() == table_bytes, "alias table drift"
    assert table.TOMBSTONES.read_bytes() == tombstone_bytes, "tombstone drift"
    document = json.loads(table_bytes)
    entries = document["entries"]
    keys = [entry["key"] for entry in entries]
    assert len(keys) == len(set(keys)) == table.EXPECTED_RETIRED_KEYS, len(keys)
    assert all(entry["version"] == 1 for entry in entries)
    tombstones = json.loads(tombstone_bytes)
    archived = {record["key"]: record for record in tombstones["records"]}
    for entry in entries:
        if entry["state"] == table.RETIRED:
            record = archived[entry["key"]]
            assert record["definition_sha256"] == entry["tombstone_sha256"], entry[
                "key"
            ]
            assert (
                table.sha256_hex(table.canonical_bytes(record["definition"]))
                == record["definition_sha256"]
            ), entry["key"]
    assert set(archived) == {e["key"] for e in entries if e["state"] == table.RETIRED}
    return {entry["key"]: entry for entry in entries}


def old_refs_resolve(entries: dict) -> int:
    """Every retired-namespace key still named in content/ or apps/ has an entry."""
    registry = re.compile(r"oteryn:item\.registry\.i\d{8}")
    named = {key for key in entries if not key.startswith("oteryn:item.registry.")}
    named_pattern = re.compile(
        "|".join(re.escape(key) for key in sorted(named, key=len, reverse=True))
    )
    seen: set[str] = set()
    for base in ("content", "apps"):
        for directory, _dirs, files in os.walk(table.ROOT / base):
            if "content/assets/files" in Path(directory).as_posix():
                continue
            for name in files:
                if not name.endswith((".json", ".rs", ".toml", ".md")):
                    continue
                path = Path(directory) / name
                if path == table.ALIAS_TABLE:
                    continue
                text = path.read_text(encoding="utf-8")
                seen.update(registry.findall(text))
                seen.update(named_pattern.findall(text))
    # Synthetic keys in Rust tests that were never allocated are not retired keys.
    synthetic = {"oteryn:item.registry.i00099999", "oteryn:item.registry.i99999999"}
    unresolved = sorted(seen - set(entries) - synthetic)
    assert not unresolved, f"old keys without an alias entry: {unresolved[:5]}"
    return len(seen)


def main() -> None:
    synthetic_rules()
    synthetic_table_checks()
    entries = committed_table()
    resolved = old_refs_resolve(entries)
    print(
        f"item_id_alias_table self-test: PASS entries={len(entries)} old_refs_resolved={resolved}"
    )


if __name__ == "__main__":
    main()
