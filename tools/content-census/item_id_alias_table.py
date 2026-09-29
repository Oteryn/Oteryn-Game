#!/usr/bin/env python3
"""Generate the append-only Item key alias table and the D149 tombstones (ITEM-ID-1a).

Decision `A12-ITEM-IDENTITY-TIBIA-ID-V1` (§4.5) retires every `oteryn:item.registry.i*`
key (epochs 1 and 2) and every named Item key. Each retired key gets exactly one current
entry in `content/items/aliases.json`:

- `ALIAS` to `oteryn:item.tibia.i<id>`, where `<id>` is the retired key's own source row id
  and the §4.2 identity evidence holds: the id is an appearance object in the source row's
  pinned CipSoft file (Crystal `items.xml` carries no `clientid`, so the server id is the
  appearance object id), and its identity projection digest there equals the one in the
  comparison file (the newest admitted file, or for a retired id the last file holding it);
- `RETIRED_WITHOUT_SUCCESSOR` when the id is not in the admitted CipSoft id set (D149). Its
  last authored definition and that definition's digest are kept in the tombstone archive
  outside authored content.

The retired keys and their source rows are not read from the committed bindings: they are
re-derived by `g4_item_crystal_binding_generator.generate()` from the protected CW2-B1
catalogue, the epoch-2 crosswalk and the Rust pins, plus each declared identity promotion
(R7-P04: `i00002921` was Crystal 3031 before it became `currency.gold_coin`). Nothing is
mapped by position or in bulk. A continuity break stops with
`ARCHITECTURE_ESCALATION_REQUIRED`. `--check` regenerates both files in memory and
requires the committed bytes.
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

import g4_item_crystal_binding_generator as bindings_generator  # noqa: E402
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


def source_rows() -> list[dict[str, Any]]:
    """Every retired key with its own source row: `{key, source_revision, source_item_id, basis}`."""
    output, _payload = bindings_generator.generate()
    rows = []
    for binding in output["bindings"]:
        if binding["disposition"] != "EXACT":
            raise AliasTableError(
                f"UNEXPECTED_BINDING_DISPOSITION:{binding['disposition']}"
            )
        rows.append(
            {
                "key": binding["target"]["key"],
                "source_revision": binding["source_revision"],
                "source_item_id": int(binding["external_id"]),
                "basis": "binding",
            }
        )
    text = bindings_generator.read_text(bindings_generator.RUST_SOURCE)
    revision = bindings_generator.parse_source_revision(text)
    for source_id, (old_key, new_key) in sorted(
        bindings_generator.parse_identity_promotions(text).items()
    ):
        promoted = [row for row in rows if row["key"] == new_key]
        if len(promoted) != 1 or promoted[0]["source_item_id"] != source_id:
            raise AliasTableError(f"PROMOTION_ROW_MISMATCH:{old_key}")
        rows.append(
            {
                "key": old_key,
                "source_revision": revision,
                "source_item_id": source_id,
                "basis": f"promotion:{new_key}",
            }
        )
    keys = [row["key"] for row in rows]
    if len(set(keys)) != len(keys):
        raise AliasTableError("RETIRED_KEY_DUPLICATE")
    return sorted(rows, key=lambda row: row["key"])


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


def definitions() -> dict[str, dict[str, Any]]:
    records: dict[str, dict[str, Any]] = {}
    for path in sorted(ROOT.glob(DEFINITIONS_GLOB)):
        for record in json.loads(path.read_text(encoding="utf-8"))["records"]:
            key = record["definition"]["identity"]["key"]
            if key in records:
                raise AliasTableError(f"DEFINITION_DUPLICATE:{key}")
            records[key] = record
    return records


def build_tombstones(rows: list[dict], views: dict) -> tuple[dict, bytes]:
    union = set().union(*(set(view) for view in views.values()))
    records = definitions()
    tombstones = {}
    for row in rows:
        if row["source_item_id"] in union:
            continue
        record = records.get(row["key"])
        if record is None:
            raise AliasTableError(f"TOMBSTONE_DEFINITION_MISSING:{row['key']}")
        tombstones[row["key"]] = {
            "key": row["key"],
            "definition": record,
            "definition_sha256": sha256_hex(canonical_bytes(record)),
        }
    ordered = [tombstones[key] for key in sorted(tombstones)]
    payload = canonical_bytes(
        {
            "schema": TOMBSTONE_SCHEMA,
            "decision": DECISION,
            "owner_decision": "D149",
            "record_count": len(ordered),
            "records_sha256": sha256_hex(canonical_bytes(ordered)),
            "records": ordered,
        }
    )
    return tombstones, payload


def verify_table(entries: list[dict], index: dict) -> dict[str, int]:
    union_retired = set(index["retired_ids"])
    by_target: dict[str, set[int]] = defaultdict(set)
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
    for target, ids in by_target.items():
        if len(ids) != 1:
            raise AliasTableError(f"ALIAS_TARGET_COLLISION:{target}")
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


def generate() -> tuple[bytes, bytes]:
    index, manifests = load_admitted()
    views = manifest_views(manifests)
    rows = source_rows()
    tombstones, tombstone_payload = build_tombstones(rows, views)
    entries = [entry_for(row, index, views, tombstones) for row in rows]
    counts = verify_table(entries, index)
    table = {
        "schema": ALIAS_SCHEMA,
        "decision": DECISION,
        "key_rule": KEY_RULE,
        "append_only": "an entry is never edited; a later version supersedes a RETIRED_WITHOUT_SUCCESSOR entry only with new Tibia-id evidence",
        "admitted_set": {
            "index": f"imports/official/appearance-membership/{INDEX_NAME}",
            "index_sha256": sha256_hex((OUT_DIR / INDEX_NAME).read_bytes()),
        },
        "tombstones": {
            "archive": TOMBSTONES.relative_to(ROOT).as_posix(),
            "archive_sha256": sha256_hex(tombstone_payload),
        },
        "counts": counts,
        "entries_sha256": sha256_hex(canonical_bytes(entries)),
        "entries": entries,
    }
    return canonical_bytes(table), tombstone_payload


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--check", action="store_true", help="verify instead of writing"
    )
    args = parser.parse_args()
    table, tombstones = generate()
    if args.check:
        for path, payload in ((ALIAS_TABLE, table), (TOMBSTONES, tombstones)):
            if not path.is_file() or path.read_bytes() != payload:
                raise AliasTableError(f"DRIFT:{path.relative_to(ROOT).as_posix()}")
    else:
        ALIAS_TABLE.write_bytes(table)
        TOMBSTONES.write_bytes(tombstones)
    counts = json.loads(table)["counts"]
    print(
        f"item_id_alias_table{' --check' if args.check else ''}: PASS {json.dumps(counts, sort_keys=True)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
