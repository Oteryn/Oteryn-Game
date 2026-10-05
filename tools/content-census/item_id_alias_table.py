#!/usr/bin/env python3
"""Verify the append-only Item key alias table and the D149 tombstones as frozen history.

Decision `A12-ITEM-IDENTITY-TIBIA-ID-V1` (§4.5) retires every `oteryn:item.registry.i*`
key (epochs 1 and 2) and every named Item key. Each retired key has exactly one current
entry in `content/items/aliases.json`:

- `ALIAS` to `oteryn:item.tibia.i<id>`, where `<id>` is the retired key's own source row id
  and the §4.2 identity evidence recorded in the entry holds;
- `RETIRED_WITHOUT_SUCCESSOR` for a D149 key; its last authored definition and that
  definition's digest are kept in the tombstone archive outside authored content.

ITEM-ID-1a derived version 1 of every entry from the CW2-B1 catalogue, the epoch-2
crosswalk and the membership manifests (#1279, independently reproduced). Since ITEM-ID-1b
those inputs no longer name the retired keys, so the table is never re-derived: it is
history. This tool only verifies it:

- the version-1 entries and the tombstone records match the digests recorded below;
- with `--base-ref`, every entry and tombstone of that revision is still present unchanged
  (append-only);
- the §4.5 rules: one current entry per key, versions numbered from 1, a later version only
  supersedes `RETIRED_WITHOUT_SUCCESSOR` with an alias, every alias target is the key's own
  recorded id in the admitted CipSoft set, and no two retired keys share a target except the
  allowlisted R7-P04 pair;
- totality over the retired key space: registry sequences 1..38,497 and the named keys.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/content-schema/item-authoring"))

from appearance_membership import INDEX_NAME, OUT_DIR, load_admitted

ALIAS_TABLE = ROOT / "content/items/aliases.json"
TOMBSTONES = ROOT / "docs/agents/evidence/OTV2-20260929-item-id-1-d149-tombstones.json"

ALIAS_SCHEMA = "OTERYN_ITEM_KEY_ALIAS_TABLE/v1"
TOMBSTONE_SCHEMA = "OTERYN_ITEM_TOMBSTONE_ARCHIVE/v1"
DECISION = "A12-ITEM-IDENTITY-TIBIA-ID-V1"
KEY_RULE = "OTERYN_TIBIA_ID_KEY_RULE_V1"
TIBIA_KEY_PREFIX = "oteryn:item.tibia.i"
OTERYN_KEY_PREFIX = "oteryn:item.oteryn."
TIBIA_KEY = re.compile(r"^oteryn:item\.tibia\.i([1-9][0-9]*)$")
REGISTRY_KEY = re.compile(r"^oteryn:item\.registry\.i(\d{8})$")
NAMED_KEY = re.compile(r"^oteryn:item\.[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$")
ALIAS = "ALIAS"
RETIRED = "RETIRED_WITHOUT_SUCCESSOR"

# Frozen by ITEM-ID-1a (#1279): the version-1 entries and the tombstone records.
GENESIS_ENTRIES = 38_562
GENESIS_ENTRIES_SHA256 = (
    "fdf99248ed956b946c8fa4ceb081e6234c5d63eb39681a859bfc1cc63a5275e7"
)
GENESIS_TOMBSTONES = 4_590
GENESIS_TOMBSTONE_RECORDS_SHA256 = (
    "eec726dd36a60c045560e9e57e15b86ed7b382741a3406ff4366958a11f627fc"
)
# Epoch 1 allocated 1..38,093 and epoch 2 38,094..38,497 (cw2_b1_import.rs).
REGISTRY_SEQUENCES = range(1, 38_498)
# Epoch 1's 64 protected semantic keys plus R7-P04 `currency.gold_coin`.
NAMED_KEYS = 65
# The only target two retired keys may share: registry i00002921 was Crystal 3031 before
# R7-P04 renamed it to `currency.gold_coin`, one source row under two historical keys.
SHARED_TARGETS = {
    "oteryn:item.tibia.i3031": {
        "oteryn:item.currency.gold_coin",
        "oteryn:item.registry.i00002921",
    }
}


class AliasTableError(ValueError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode(
        "utf-8"
    )


def sha256_hex(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def tibia_key(object_id: int) -> str:
    if object_id <= 0:
        raise AliasTableError(f"INVALID_TIBIA_ID:{object_id}")
    return f"{TIBIA_KEY_PREFIX}{object_id}"


def current_entries(entries: list[dict]) -> dict[str, dict]:
    """The latest entry of every retired key, after checking the version chain."""
    chains: dict[str, list[dict]] = defaultdict(list)
    for entry in entries:
        chains[entry["key"]].append(entry)
    current = {}
    for key, chain in chains.items():
        versions = [entry["version"] for entry in chain]
        if versions != list(range(1, len(chain) + 1)):
            raise AliasTableError(f"VERSION_CHAIN:{key}:{versions}")
        for older in chain[:-1]:
            if older["state"] != RETIRED:
                raise AliasTableError(
                    f"ARCHITECTURE_ESCALATION_REQUIRED:ALIAS_SUPERSEDED:{key}"
                )
        if len(chain) > 1 and chain[-1]["state"] != ALIAS:
            raise AliasTableError(f"SUPERSEDED_WITHOUT_ALIAS:{key}")
        current[key] = chain[-1]
    return current


def verify_table(entries: list[dict], index: dict, union: set[int]) -> dict[str, int]:
    union_retired = set(index["retired_ids"])
    if [entry["key"] for entry in entries] != sorted(
        entry["key"] for entry in entries
    ) or [(e["key"], e["version"]) for e in entries] != sorted(
        (e["key"], e["version"]) for e in entries
    ):
        raise AliasTableError("ENTRY_ORDER")
    current = current_entries(entries)
    by_target: dict[str, set[str]] = defaultdict(set)
    counts = {ALIAS: 0, RETIRED: 0}
    sequences = set()
    named = 0
    for key, entry in current.items():
        if key.startswith((TIBIA_KEY_PREFIX, OTERYN_KEY_PREFIX)):
            raise AliasTableError(f"CANONICAL_KEY_RETIRED:{key}")
        registry = REGISTRY_KEY.match(key)
        if registry:
            sequences.add(int(registry.group(1)))
        elif NAMED_KEY.match(key):
            named += 1
        else:
            raise AliasTableError(f"RETIRED_KEY_FORMAT:{key}")
        if entry["state"] not in counts:
            raise AliasTableError(f"STATE:{key}:{entry['state']}")
        counts[entry["state"]] += 1
        if entry["state"] == ALIAS:
            object_id = entry["evidence"]["source_item_id"]
            if entry["target"] != tibia_key(object_id):
                raise AliasTableError(f"ALIAS_TARGET_NOT_OWN_ID:{key}")
            if object_id not in union:
                raise AliasTableError(f"ALIAS_TARGET_NOT_ADMITTED:{key}")
            by_target[entry["target"]].add(key)
        elif "target" in entry:
            raise AliasTableError(f"RETIRED_CARRIES_TARGET:{key}")
    for target, keys in by_target.items():
        if len(keys) > 1 and SHARED_TARGETS.get(target) != keys:
            raise AliasTableError(f"ALIAS_TARGET_COLLISION:{target}:{sorted(keys)}")
    for target, keys in SHARED_TARGETS.items():
        if by_target.get(target) != keys:
            raise AliasTableError(f"SHARED_TARGET_ALLOWLIST_STALE:{target}")
    if sequences != set(REGISTRY_SEQUENCES) or named != NAMED_KEYS:
        raise AliasTableError(f"RETIRED_KEY_SPACE:{len(sequences)}:{named}")
    return {
        "retired_keys": len(current),
        "alias": counts[ALIAS],
        "retired_without_successor": counts[RETIRED],
        "alias_targets": len(by_target),
        "alias_targets_on_retired_ids": sum(
            1
            for target in by_target
            if int(target[len(TIBIA_KEY_PREFIX) :]) in union_retired
        ),
    }


def verify_tombstones(
    table: dict, archive_bytes: bytes, current: dict[str, dict]
) -> None:
    if sha256_hex(archive_bytes) != table["tombstones"]["archive_sha256"]:
        raise AliasTableError("TOMBSTONE_ARCHIVE_DIGEST")
    archive = json.loads(archive_bytes)
    records = archive["records"]
    if (
        archive["schema"] != TOMBSTONE_SCHEMA
        or archive["decision"] != DECISION
        or archive["record_count"] != len(records)
        or archive["records_sha256"] != sha256_hex(canonical_bytes(records))
        or archive_bytes != canonical_bytes(archive)
    ):
        raise AliasTableError("TOMBSTONE_ARCHIVE_HEADER")
    genesis = records[:GENESIS_TOMBSTONES]
    if sha256_hex(canonical_bytes(genesis)) != GENESIS_TOMBSTONE_RECORDS_SHA256:
        raise AliasTableError("TOMBSTONE_GENESIS_DIGEST")
    archived = {}
    for record in records:
        if sha256_hex(canonical_bytes(record["definition"])) != record[
            "definition_sha256"
        ] or (record["key"] in archived):
            raise AliasTableError(f"TOMBSTONE_RECORD:{record['key']}")
        archived[record["key"]] = record
    for key, entry in current.items():
        if entry["state"] == RETIRED and (
            archived.get(key, {}).get("definition_sha256") != entry["tombstone_sha256"]
        ):
            raise AliasTableError(f"TOMBSTONE_MISSING:{key}")


def verify_append_only(entries: list[dict], base: dict, base_archive: dict) -> None:
    present = {(entry["key"], entry["version"]): entry for entry in entries}
    for entry in base["entries"]:
        if present.get((entry["key"], entry["version"])) != entry:
            raise AliasTableError(f"APPEND_ONLY:{entry['key']}:{entry['version']}")
    archive = json.loads(TOMBSTONES.read_bytes())["records"]
    if archive[: len(base_archive["records"])] != base_archive["records"]:
        raise AliasTableError("APPEND_ONLY:TOMBSTONES")


def git_show(ref: str, path: Path) -> bytes:
    relative = path.relative_to(ROOT).as_posix()
    return subprocess.run(
        ["git", "show", f"{ref}:{relative}"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    ).stdout


def verify(base_ref: str | None = None) -> dict[str, int]:
    table_bytes = ALIAS_TABLE.read_bytes()
    table = json.loads(table_bytes)
    entries = table["entries"]
    if (
        table["schema"] != ALIAS_SCHEMA
        or table["decision"] != DECISION
        or table["key_rule"] != KEY_RULE
        or table_bytes != canonical_bytes(table)
        or table["entries_sha256"] != sha256_hex(canonical_bytes(entries))
        or table["admitted_set"]["index"]
        != f"imports/official/appearance-membership/{INDEX_NAME}"
        or table["admitted_set"]["index_sha256"]
        != sha256_hex((OUT_DIR / INDEX_NAME).read_bytes())
        or table["tombstones"]["archive"] != TOMBSTONES.relative_to(ROOT).as_posix()
    ):
        raise AliasTableError("ALIAS_TABLE_HEADER")
    genesis = [entry for entry in entries if entry["version"] == 1]
    if (
        len(genesis) != GENESIS_ENTRIES
        or sha256_hex(canonical_bytes(genesis)) != GENESIS_ENTRIES_SHA256
    ):
        raise AliasTableError("ALIAS_GENESIS_DIGEST")
    index, manifests = load_admitted()
    union = set().union(
        *({entry[0] for entry in doc["entries"]} for doc in manifests.values())
    )
    counts = verify_table(entries, index, union)
    if table["counts"] != counts:
        raise AliasTableError(f"COUNTS:{counts}")
    verify_tombstones(table, TOMBSTONES.read_bytes(), current_entries(entries))
    if base_ref is not None:
        verify_append_only(
            entries,
            json.loads(git_show(base_ref, ALIAS_TABLE)),
            json.loads(git_show(base_ref, TOMBSTONES)),
        )
    return counts


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--check", action="store_true", help="verify (the only mode; kept for CI)"
    )
    parser.add_argument(
        "--base-ref", help="git revision whose table must survive unchanged"
    )
    args = parser.parse_args()
    counts = verify(args.base_ref)
    print(
        f"item_id_alias_table --check: PASS {json.dumps(counts, sort_keys=True)}"
        + (f" append_only_vs={args.base_ref}" if args.base_ref else "")
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
