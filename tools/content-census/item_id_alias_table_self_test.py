#!/usr/bin/env python3
"""Fail-closed tests for the ITEM-ID-1a alias table and D149 tombstones.

Synthetic cases cover each §4.2/§4.5 rule; the committed-data cases prove the frozen table
is total and re-derivable from its own evidence, and that no authored content or code names a
retired key (ITEM-ID-1b): old keys survive only in the alias table, the tombstone archive,
pinned history under `docs/` and the explicitly listed history verifiers.
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
    expect_error("ALIAS_TARGET_SHARED", table.verify_table, same_item, INDEX)


def committed_table() -> dict:
    counts = table.verify_frozen()
    assert counts["retired_keys"] == table.EXPECTED_RETIRED_KEYS, counts
    document = json.loads(table.ALIAS_TABLE.read_bytes())
    entries = document["entries"]
    assert all(entry["version"] == 1 for entry in entries)
    return {entry["key"]: entry for entry in entries}


def synthetic_frozen_checks() -> None:
    shared = [
        {
            "key": "oteryn:item.registry.i00000001",
            "state": table.ALIAS,
            "target": "oteryn:item.tibia.i5",
            "evidence": {"source_item_id": 5},
        },
        {
            "key": "oteryn:item.decor.five",
            "state": table.ALIAS,
            "target": "oteryn:item.tibia.i5",
            "evidence": {"source_item_id": 5},
        },
    ]
    expect_error("ALIAS_TARGET_SHARED", table.verify_table, shared, INDEX)
    tombstone = {
        "key": "oteryn:item.registry.i00000009",
        "definition": {
            "definition": {"identity": {"key": "oteryn:item.registry.i00000009"}}
        },
    }
    tombstone["definition_sha256"] = "0" * 64
    payload = table.canonical_bytes(
        {
            "schema": table.TOMBSTONE_SCHEMA,
            "decision": table.DECISION,
            "owner_decision": "D149",
            "record_count": 1,
            "records_sha256": table.sha256_hex(table.canonical_bytes([tombstone])),
            "records": [tombstone],
        }
    )
    expect_error("TOMBSTONE_RECORD", table.archived_tombstones, payload)


# Files that must keep retired keys because they verify pinned history: the retired
# allocations, the named-key batch and the R7-P04 packet contract (ITEM-ID-1b).
HISTORY_VERIFIERS = {
    "apps/game-server/src/content/cw2_b1_import.rs",
    "apps/game-server/tests/content_world_cw2_b1_import.rs",
}


def no_retired_refs(entries: dict) -> int:
    """No authored content names a retired key; code does so only in the history verifiers."""
    registry = re.compile(r"oteryn:item\.registry\.i\d{8}")
    named = {key for key in entries if not key.startswith("oteryn:item.registry.")}
    named_pattern = re.compile(
        "|".join(re.escape(key) for key in sorted(named, key=len, reverse=True))
    )
    offending: dict[str, list[str]] = {}
    verifier_refs = 0
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
                found = {
                    key
                    for key in set(registry.findall(text))
                    | set(named_pattern.findall(text))
                    if key in entries
                }
                if not found:
                    continue
                relative = path.relative_to(table.ROOT).as_posix()
                if relative in HISTORY_VERIFIERS:
                    verifier_refs += len(found)
                else:
                    offending[relative] = sorted(found)[:3]
    assert not offending, (
        f"retired Item keys outside the history verifiers: {offending}"
    )
    return verifier_refs


def main() -> None:
    synthetic_rules()
    synthetic_table_checks()
    synthetic_frozen_checks()
    entries = committed_table()
    verifier_refs = no_retired_refs(entries)
    print(
        f"item_id_alias_table self-test: PASS entries={len(entries)} history_verifier_refs={verifier_refs}"
    )


if __name__ == "__main__":
    main()
