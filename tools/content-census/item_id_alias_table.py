#!/usr/bin/env python3
"""Verify the frozen append-only Item key alias table and the D149 tombstones (ITEM-ID-1).

Decision `A12-ITEM-IDENTITY-TIBIA-ID-V1` (§4.5) retires every `oteryn:item.registry.i*`
key (epochs 1 and 2) and every named Item key. Each retired key has exactly one current
entry in `content/items/aliases.json`:

- `ALIAS` to `oteryn:item.tibia.i<id>`, where `<id>` is the retired key's own source row id
  and the §4.2 identity evidence holds: the id is an appearance object in the source row's
  pinned CipSoft file (Crystal `items.xml` carries no `clientid`, so the server id is the
  appearance object id), and its identity projection digest there equals the one in the
  comparison file (the newest admitted file, or for a retired id the last file holding it);
- `RETIRED_WITHOUT_SUCCESSOR` when the id is not in the admitted CipSoft id set (D149). Its
  last authored definition and that definition's digest are kept in the tombstone archive
  outside authored content.

ITEM-ID-1a generated both files from the pre-switch registry. ITEM-ID-1b switched every key
to the Tibia rule, so the inputs that produced them no longer exist: the files are history.
They are pinned by exact bytes (the frozen version-1 generation). `--check` verifies the
pins and re-derives every entry from its own recorded evidence (source revision, source id,
basis) against the committed membership manifests and the tombstone archive. Nothing is
mapped by position or in bulk. A later version may only append entries, and it must move the
pins in the same change.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/content-schema/item-authoring"))

from appearance_membership import INDEX_NAME, OUT_DIR, load_admitted  # noqa: E402

ALIAS_TABLE = ROOT / "content/items/aliases.json"
TOMBSTONES = ROOT / "docs/agents/evidence/OTV2-20260929-item-id-1-d149-tombstones.json"
DEFINITIONS_GLOB = "content/items/definitions/items-*.json"

ALIAS_SCHEMA = "OTERYN_ITEM_KEY_ALIAS_TABLE/v1"
TOMBSTONE_SCHEMA = "OTERYN_ITEM_TOMBSTONE_ARCHIVE/v1"
DECISION = "A12-ITEM-IDENTITY-TIBIA-ID-V1"
KEY_RULE = "OTERYN_TIBIA_ID_KEY_RULE_V1"
TIBIA_KEY_PREFIX = "oteryn:item.tibia.i"
OTERYN_KEY_PREFIX = "oteryn:item.oteryn."
REGISTRY_KEY = re.compile(r"^oteryn:item\.registry\.i\d{8}$")
ALIAS = "ALIAS"
RETIRED = "RETIRED_WITHOUT_SUCCESSOR"
# A12 §2 D149 and §3: the counts the decision was taken on.
EXPECTED_RETIRED_KEYS = 38_562
EXPECTED_WITHOUT_SUCCESSOR = 4_590
# Frozen version-1 generation (ITEM-ID-1a #1279): exact committed bytes.
FROZEN_ALIAS_TABLE_SHA256 = (
    "128bc354816199702c26f83120e6fb7846fb2608fb5ee1c16085f63268e592f2"
)
FROZEN_TOMBSTONES_SHA256 = (
    "9e2dac1cdc231e792c7f69811e3998e4240c81f5440c4956e53d210fa996030d"
)
# Retired keys that alias to the same Tibia key (§4.5 "no two retired keys that meant
# different items map to the same key"): only R7-P04, where `registry.i00002921` became
# `currency.gold_coin` for the same Crystal 3031 row.
SHARED_TARGET_ALLOWLIST = {
    "oteryn:item.tibia.i3031": (
        "oteryn:item.currency.gold_coin",
        "oteryn:item.registry.i00002921",
    ),
}

# Pinned CipSoft appearance file of each source revision (appearance_membership labels).
SOURCE_APPEARANCES = {
    "ff7ede593c69d4c658b382c97443e8155926924a": "crystal-ff7ede5",
    "00ce02a57ca5a12e48f32a3476e37471167e4c3f": "crystal-donor-00ce02a5",
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


def recorded_rows(entries: list[dict]) -> list[dict[str, Any]]:
    """Each retired key's own source row, as recorded in its entry's evidence."""
    rows = []
    for entry in entries:
        evidence = entry["evidence"]
        rows.append(
            {
                "key": entry["key"],
                "source_revision": evidence["source_revision"],
                "source_item_id": evidence["source_item_id"],
                "basis": evidence["basis"],
            }
        )
    keys = [row["key"] for row in rows]
    if len(set(keys)) != len(keys):
        raise AliasTableError("RETIRED_KEY_DUPLICATE")
    if keys != sorted(keys):
        raise AliasTableError("RETIRED_KEY_ORDER")
    return rows


def manifest_views(manifests: dict[str, dict]) -> dict[str, dict[int, list]]:
    return {
        label: {entry[0]: entry for entry in document["entries"]}
        for label, document in manifests.items()
    }


def comparison_label(object_id: int, index: dict, views: dict) -> str | None:
    """The newest admitted file holding the id (the newest file for a current id)."""
    for file in reversed(index["files"]):
        if object_id in views[file["label"]]:
            return file["label"]
    return None


def entry_for(row: dict, index: dict, views: dict, tombstones: dict) -> dict[str, Any]:
    source_label = SOURCE_APPEARANCES.get(row["source_revision"])
    if source_label is None:
        raise AliasTableError(f"UNPINNED_SOURCE_REVISION:{row['source_revision']}")
    object_id = row["source_item_id"]
    source_entry = views[source_label].get(object_id)
    comparison = comparison_label(object_id, index, views)
    evidence: dict[str, Any] = {
        "basis": row["basis"],
        "source_appearances": source_label,
        "source_item_id": object_id,
        "source_revision": row["source_revision"],
    }
    if source_entry is None:
        if comparison is not None:
            # Absent from its own pinned file but present elsewhere: not proven, never guessed.
            raise AliasTableError(f"UNPROVEN_ADMITTED_ID:{row['key']}:{object_id}")
        if row["key"] not in tombstones:
            raise AliasTableError(f"TOMBSTONE_MISSING:{row['key']}")
        evidence["admitted_set"] = "ABSENT"
        return {
            "key": row["key"],
            "version": 1,
            "state": RETIRED,
            "evidence": evidence,
            "tombstone_sha256": tombstones[row["key"]]["definition_sha256"],
        }
    comparison_entry = views[comparison][object_id]
    if source_entry[1] != comparison_entry[1]:
        raise AliasTableError(
            f"ARCHITECTURE_ESCALATION_REQUIRED:CONTINUITY_BREAK:{row['key']}:{object_id}"
        )
    evidence["comparison_appearances"] = comparison
    evidence["identity_projection_sha256"] = source_entry[1]
    evidence["record"] = (
        "UNCHANGED" if source_entry[2] == comparison_entry[2] else "EVOLVED"
    )
    return {
        "key": row["key"],
        "version": 1,
        "state": ALIAS,
        "target": tibia_key(object_id),
        "evidence": evidence,
    }


def archived_tombstones(payload: bytes) -> dict[str, dict[str, Any]]:
    """The D149 archive, each record re-digested from its own definition."""
    document = json.loads(payload)
    records = document["records"]
    if (
        document.get("schema") != TOMBSTONE_SCHEMA
        or document.get("decision") != DECISION
        or document.get("owner_decision") != "D149"
        or document.get("record_count") != len(records)
        or document.get("records_sha256") != sha256_hex(canonical_bytes(records))
        or canonical_bytes(document) != payload
    ):
        raise AliasTableError("TOMBSTONE_ARCHIVE_HEADER")
    tombstones = {}
    for record in records:
        key = record["key"]
        if (
            key in tombstones
            or record["definition"]["definition"]["identity"]["key"] != key
            or sha256_hex(canonical_bytes(record["definition"]))
            != record["definition_sha256"]
        ):
            raise AliasTableError(f"TOMBSTONE_RECORD:{key}")
        tombstones[key] = record
    if list(tombstones) != sorted(tombstones):
        raise AliasTableError("TOMBSTONE_ORDER")
    return tombstones


def verify_table(entries: list[dict], index: dict) -> dict[str, int]:
    union_retired = set(index["retired_ids"])
    by_target: dict[str, set[int]] = defaultdict(set)
    keys_by_target: dict[str, list[str]] = defaultdict(list)
    counts = {ALIAS: 0, RETIRED: 0}
    for entry in entries:
        key = entry["key"]
        if not (REGISTRY_KEY.match(key) or key.startswith("oteryn:item.")):
            raise AliasTableError(f"RETIRED_KEY_FORMAT:{key}")
        if key.startswith((TIBIA_KEY_PREFIX, OTERYN_KEY_PREFIX)):
            raise AliasTableError(f"CANONICAL_KEY_RETIRED:{key}")
        counts[entry["state"]] += 1
        if entry["state"] == ALIAS:
            object_id = entry["evidence"]["source_item_id"]
            if entry["target"] != tibia_key(object_id):
                raise AliasTableError(f"ALIAS_TARGET_NOT_OWN_ID:{key}")
            by_target[entry["target"]].add(object_id)
            keys_by_target[entry["target"]].append(key)
    for target, ids in by_target.items():
        if len(ids) != 1:
            raise AliasTableError(f"ALIAS_TARGET_COLLISION:{target}")
    for target, keys in keys_by_target.items():
        allowed = SHARED_TARGET_ALLOWLIST.get(target)
        if len(keys) > 1 and tuple(sorted(keys)) != allowed:
            raise AliasTableError(f"ALIAS_TARGET_SHARED:{target}")
    for target, allowed in SHARED_TARGET_ALLOWLIST.items():
        if tuple(
            sorted(keys_by_target.get(target, ()))
        ) != allowed and entries_are_full(entries):
            raise AliasTableError(f"ALIAS_ALLOWLIST_UNUSED:{target}")
    if (
        len(entries) != EXPECTED_RETIRED_KEYS
        or counts[RETIRED] != EXPECTED_WITHOUT_SUCCESSOR
    ):
        raise AliasTableError(f"COUNT_MISMATCH:{len(entries)}:{counts}")
    return {
        "retired_keys": len(entries),
        "alias": counts[ALIAS],
        "retired_without_successor": counts[RETIRED],
        "alias_targets": len(by_target),
        "alias_targets_on_retired_ids": sum(
            1
            for target in by_target
            if int(target[len(TIBIA_KEY_PREFIX) :]) in union_retired
        ),
    }


def entries_are_full(entries: list[dict]) -> bool:
    return len(entries) == EXPECTED_RETIRED_KEYS


def verify_frozen() -> dict[str, int]:
    table_bytes = ALIAS_TABLE.read_bytes()
    tombstone_bytes = TOMBSTONES.read_bytes()
    if sha256_hex(table_bytes) != FROZEN_ALIAS_TABLE_SHA256:
        raise AliasTableError("DRIFT:content/items/aliases.json")
    if sha256_hex(tombstone_bytes) != FROZEN_TOMBSTONES_SHA256:
        raise AliasTableError(f"DRIFT:{TOMBSTONES.relative_to(ROOT).as_posix()}")
    document = json.loads(table_bytes)
    entries = document["entries"]
    index, manifests = load_admitted()
    expected_header = {
        "schema": ALIAS_SCHEMA,
        "decision": DECISION,
        "key_rule": KEY_RULE,
        "admitted_set": {
            "index": f"imports/official/appearance-membership/{INDEX_NAME}",
            "index_sha256": sha256_hex((OUT_DIR / INDEX_NAME).read_bytes()),
        },
        "tombstones": {
            "archive": TOMBSTONES.relative_to(ROOT).as_posix(),
            "archive_sha256": sha256_hex(tombstone_bytes),
        },
        "entries_sha256": sha256_hex(canonical_bytes(entries)),
    }
    for field, value in expected_header.items():
        if document.get(field) != value:
            raise AliasTableError(f"HEADER_MISMATCH:{field}")
    if canonical_bytes(document) != table_bytes:
        raise AliasTableError("NON_CANONICAL_TABLE")
    tombstones = archived_tombstones(tombstone_bytes)
    views = manifest_views(manifests)
    derived = [
        entry_for(row, index, views, tombstones) for row in recorded_rows(entries)
    ]
    if derived != entries:
        raise AliasTableError("ENTRY_NOT_REDERIVABLE")
    retired = {e["key"] for e in entries if e["state"] == RETIRED}
    if set(tombstones) != retired:
        raise AliasTableError("TOMBSTONE_SET_MISMATCH")
    counts = verify_table(entries, index)
    if document.get("counts") != counts:
        raise AliasTableError("HEADER_MISMATCH:counts")
    return counts


def load_successors() -> dict[str, str | None]:
    """Retired key -> its Tibia key, or `None` (D149), from the byte-pinned committed table."""
    payload = ALIAS_TABLE.read_bytes()
    if sha256_hex(payload) != FROZEN_ALIAS_TABLE_SHA256:
        raise AliasTableError("DRIFT:content/items/aliases.json")
    successors: dict[str, str | None] = {}
    for entry in json.loads(payload)["entries"]:
        if entry["state"] == ALIAS:
            if entry["target"] != tibia_key(entry["evidence"]["source_item_id"]):
                raise AliasTableError(f"ALIAS_TARGET_NOT_OWN_ID:{entry['key']}")
            successors[entry["key"]] = entry["target"]
        else:
            successors[entry["key"]] = None
    return successors


def rekey_retired_strings(value: Any, successors: dict[str, str | None]) -> Any:
    """Copy of `value` with every retired Item key (alone or as `key@revision`) re-keyed.

    Pinned evidence packets stay byte-identical history; consumers translate their decoded
    value through this function only. A D149 key fails closed: no admitted content names one.
    """
    if isinstance(value, str):
        if not value.startswith("oteryn:item."):
            return value
        key, at, revision = value.partition("@")
        if key not in successors:
            return value
        target = successors[key]
        if target is None:
            raise AliasTableError(f"D149_KEY_IN_ADMITTED_INPUT:{key}")
        return f"{target}{at}{revision}"
    if isinstance(value, list):
        return [rekey_retired_strings(item, successors) for item in value]
    if isinstance(value, dict):
        return {
            field: rekey_retired_strings(item, successors)
            for field, item in value.items()
        }
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify the frozen table (the only mode; kept for CI symmetry)",
    )
    parser.parse_args()
    counts = verify_frozen()
    print(f"item_id_alias_table --check: PASS {json.dumps(counts, sort_keys=True)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
