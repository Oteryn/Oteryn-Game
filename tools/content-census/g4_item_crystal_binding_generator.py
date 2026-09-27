#!/usr/bin/env python3
"""Generate explicit EXACT Crystal -> canonical Item identity bindings.

The committed CW2-B1 native-item allocator
(`apps/game-server/src/content/cw2_b1_import.rs`) already assigns every one of
the 38,157 `content/items/definitions/*.json` Item keys from an ascending walk
of the protected evidence catalogue
(`docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json`,
source `zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a`
`data/items/items.xml`): the 64 `NATIVE_ITEM_BATCH` rows keep their existing
semantic key, every other row gets `oteryn:item.registry.i%08d`. That mapping
is currently only implicit in the allocator's own logic.

A later protected identity promotion may rename one allocated key in place
(e.g. R7 P04: Crystal `3031` `oteryn:item.registry.i00002921` ->
`oteryn:item.currency.gold_coin`). Each promotion is declared in the same Rust
source as a `<PREFIX>_SOURCE_ITEM_ID` / `<PREFIX>_OLD_KEY` / `<PREFIX>_KEY`
constant triple; this script applies every such triple after allocation and
fails closed unless the allocator assigned exactly `<PREFIX>_OLD_KEY` to that
source id.

Per `docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md`
section 3, an external identifier must become an explicit, typed
`(target, source, identity_namespace, external_id, disposition)` binding
record rather than being left as an inference. This script makes the
allocator's already-made assignments explicit: it re-derives the identical
mapping independently (parsing `NATIVE_ITEM_BATCH` straight from the Rust
source, never hand-copying it) and writes it as
`imports/crystalserver/bindings/items.json`, in the same shape as the sibling
`imports/tibiawiki/bindings/items.json`.

This script mints no identity, resolves no ambiguity and performs no
crosswalk matching: every row is already fully determined by the frozen
allocator and the frozen evidence catalogue. It fails closed on any mismatch.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
RUST_SOURCE = ROOT / "apps/game-server/src/content/cw2_b1_import.rs"
EVIDENCE = (
    ROOT
    / "docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
)
DEFINITIONS_GLOB = "content/items/definitions/items-*.json"
TIBIAWIKI_BINDINGS = ROOT / "imports/tibiawiki/bindings/items.json"
OUTPUT = ROOT / "imports/crystalserver/bindings/items.json"

SCHEMA = "OTERYN_SOURCE_IDENTITY_BINDINGS/v1"
SOURCE_KEY = "oteryn:source.crystalserver"
# Crystal (an OTServer-family engine) `data/items/items.xml` `<item id="...">`
# is the server-side item type id, i.e. exactly doctrine's `ots/item_server_id`
# example namespace -- distinct from the same file's `clientid` attribute,
# which would bind `ots/item_client_id`/`client/appearance_id` to Presentation,
# never to Item. This binding only ever carries the `id` attribute value.
IDENTITY_NAMESPACE = "ots/item_server_id"
DEFINITION_REVISION = "definition-r1"
EXPECTED_EVIDENCE_BYTES = 16_877_870
EXPECTED_EVIDENCE_SHA256 = (
    "7836c78cad130a5c404f648e76e0823f53ae6a34c6952b9b88c8bed2e50d96a7"
)
EXPECTED_TOTAL = 38_157
EXPECTED_NATIVE_BATCH = 64
EXPECTED_OPAQUE = EXPECTED_TOTAL - EXPECTED_NATIVE_BATCH

# Golden cross-checks: (crystal source_item_id, expected canonical key,
# TibiaWiki mediawiki page_id already bound EXACT to that same key). These
# prove this generator cannot silently disagree with an already-accepted
# TibiaWiki EXACT binding that targets the same canonical Item -- at minimum
# Magic Sword (i00003167 / Crystal 3288 / TibiaWiki page 5810).
CROSS_CHECKS: tuple[tuple[int, str, str], ...] = (
    (3288, "oteryn:item.registry.i00003167", "5810"),
)


class GeneratorError(ValueError):
    pass


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def sha256_hex(payload: bytes) -> str:
    import hashlib

    return hashlib.sha256(payload).hexdigest()


def parse_source_revision(text: str) -> str:
    match = re.search(r'CW2_B1_SOURCE_REVISION: &str = "([0-9a-f]{40})";', text)
    if not match:
        raise GeneratorError("SOURCE_REVISION_NOT_FOUND")
    return match.group(1)


def parse_opaque_namespace(text: str) -> str:
    match = re.search(r'CW2_B1_OPAQUE_ITEM_NAMESPACE: &str = "([^"]+)";', text)
    if not match:
        raise GeneratorError("OPAQUE_NAMESPACE_NOT_FOUND")
    return match.group(1)


def parse_native_batch(text: str) -> dict[int, str]:
    count_match = re.search(r"CW2_B1_NATIVE_ITEM_BATCH_COUNT: usize = (\d+);", text)
    if not count_match or int(count_match.group(1)) != EXPECTED_NATIVE_BATCH:
        raise GeneratorError("NATIVE_BATCH_COUNT_CONST_MISMATCH")

    table_match = re.search(
        r"const NATIVE_ITEM_BATCH: \[NativeItemSpec; CW2_B1_NATIVE_ITEM_BATCH_COUNT\] = \[(.*?)\n\];",
        text,
        re.DOTALL,
    )
    if not table_match:
        raise GeneratorError("NATIVE_ITEM_BATCH_TABLE_NOT_FOUND")
    entries = re.findall(
        r"NativeItemSpec\s*\{\s*source_item_id:\s*(\d+),.*?native_key:\s*\"([^\"]+)\"",
        table_match.group(1),
        re.DOTALL,
    )
    if len(entries) != EXPECTED_NATIVE_BATCH:
        raise GeneratorError(f"NATIVE_ITEM_BATCH_ENTRY_COUNT_MISMATCH:{len(entries)}")
    mapping: dict[int, str] = {}
    for source_id_text, native_key in entries:
        source_id = int(source_id_text)
        if source_id in mapping:
            raise GeneratorError(f"NATIVE_ITEM_BATCH_DUPLICATE_SOURCE_ID:{source_id}")
        mapping[source_id] = native_key
    return mapping


def parse_identity_promotions(text: str) -> dict[int, tuple[str, str]]:
    """Return `{source_item_id: (old_key, new_key)}` for every declared promotion."""
    promotions: dict[int, tuple[str, str]] = {}
    for prefix, old_key in re.findall(
        r'pub const (\w+)_OLD_KEY: &str = "([^"]+)";', text
    ):
        new_match = re.search(rf'pub const {prefix}_KEY: &str = "([^"]+)";', text)
        id_match = re.search(
            rf"pub const {prefix}_SOURCE_ITEM_ID: u64 = ([0-9_]+);", text
        )
        if not new_match or not id_match:
            raise GeneratorError(f"IDENTITY_PROMOTION_INCOMPLETE:{prefix}")
        source_id = int(id_match.group(1).replace("_", ""))
        if source_id in promotions:
            raise GeneratorError(f"IDENTITY_PROMOTION_DUPLICATE_SOURCE_ID:{source_id}")
        promotions[source_id] = (old_key, new_match.group(1))
    return promotions


def apply_identity_promotions(
    allocations: list[tuple[int, str]], promotions: dict[int, tuple[str, str]]
) -> list[tuple[int, str]]:
    by_source = dict(allocations)
    for source_id, (old_key, _new_key) in promotions.items():
        if by_source.get(source_id) != old_key:
            raise GeneratorError(
                f"IDENTITY_PROMOTION_OLD_KEY_MISMATCH:{source_id}:{by_source.get(source_id)}"
            )
    return [
        (source_id, promotions[source_id][1] if source_id in promotions else key)
        for source_id, key in allocations
    ]


def opaque_item_key(namespace: str, sequence: int) -> str:
    return f"{namespace}.i{sequence:08d}"


def load_identity_records() -> list[dict[str, Any]]:
    payload = EVIDENCE.read_bytes()
    if (
        len(payload) != EXPECTED_EVIDENCE_BYTES
        or sha256_hex(payload) != EXPECTED_EVIDENCE_SHA256
    ):
        raise GeneratorError("PROTECTED_EVIDENCE_DIGEST_MISMATCH")
    evidence = json.loads(payload)
    records = evidence.get("semantic_catalog", {}).get("identity_records")
    if not isinstance(records, list) or len(records) != EXPECTED_TOTAL:
        raise GeneratorError("IDENTITY_RECORD_COUNT_MISMATCH")
    return records


def allocate_keys(
    records: list[dict[str, Any]], native_batch: dict[int, str], namespace: str
) -> list[tuple[int, str]]:
    """Reproduce `protected_cw2_b1_full_item_family_import`'s key assignment."""
    allocations: list[tuple[int, str]] = []
    seen_ids: set[int] = set()
    seen_keys: set[str] = set()
    previous_id: int | None = None
    opaque_sequence = 0
    preserved = 0
    for row in records:
        source_id = row.get("source_item_id")
        if not isinstance(source_id, int):
            raise GeneratorError("SOURCE_ITEM_ID_INVALID")
        if previous_id is not None and previous_id >= source_id:
            raise GeneratorError("SOURCE_ITEM_ID_ORDER_VIOLATION")
        if source_id in seen_ids:
            raise GeneratorError("SOURCE_ITEM_ID_DUPLICATE")
        seen_ids.add(source_id)
        previous_id = source_id

        native_key = native_batch.get(source_id)
        if native_key is not None:
            preserved += 1
            key = native_key
        else:
            opaque_sequence += 1
            key = opaque_item_key(namespace, opaque_sequence)
        if key in seen_keys:
            raise GeneratorError(f"NATIVE_KEY_DUPLICATE:{key}")
        seen_keys.add(key)
        allocations.append((source_id, key))

    if opaque_sequence != EXPECTED_OPAQUE or preserved != EXPECTED_NATIVE_BATCH:
        raise GeneratorError("ALLOCATION_CLOSURE_MISMATCH")
    return allocations


def load_definition_keys() -> set[str]:
    keys: set[str] = set()
    paths = sorted(ROOT.glob(DEFINITIONS_GLOB))
    if not paths:
        raise GeneratorError("NO_ITEM_DEFINITION_SHARDS_FOUND")
    for path in paths:
        payload = json.loads(read_text(path))
        if payload.get("family") != "Item":
            raise GeneratorError(f"UNEXPECTED_SHARD_FAMILY:{path.name}")
        for record in payload.get("records", []):
            identity = record.get("definition", {}).get("identity", {})
            if identity.get("family") != "Item":
                raise GeneratorError(f"UNEXPECTED_RECORD_FAMILY:{path.name}")
            keys.add(identity["key"])
    return keys


def load_tibiawiki_targets() -> dict[str, set[str]]:
    payload = json.loads(read_text(TIBIAWIKI_BINDINGS))
    targets: dict[str, set[str]] = {}
    for binding in payload.get("bindings", []):
        targets.setdefault(binding["target"]["key"], set()).add(binding["external_id"])
    return targets


def verify_allocations(
    allocations: list[tuple[int, str]],
    definition_keys: set[str],
    tibiawiki_targets: dict[str, set[str]],
) -> None:
    if len(allocations) != EXPECTED_TOTAL:
        raise GeneratorError("ALLOCATION_COUNT_MISMATCH")
    allocation_keys = {key for _, key in allocations}
    if len(allocation_keys) != EXPECTED_TOTAL:
        raise GeneratorError("ALLOCATION_KEY_UNIQUENESS")
    missing = allocation_keys - definition_keys
    if missing:
        raise GeneratorError(
            f"ALLOCATED_KEY_MISSING_FROM_DEFINITIONS:{sorted(missing)[:5]}"
        )
    extra = definition_keys - allocation_keys
    if extra:
        raise GeneratorError(f"DEFINITION_KEY_WITHOUT_ALLOCATION:{sorted(extra)[:5]}")

    by_source = dict(allocations)
    for source_id, expected_key, page_id in CROSS_CHECKS:
        actual_key = by_source.get(source_id)
        if actual_key != expected_key:
            raise GeneratorError(f"CROSS_CHECK_KEY_MISMATCH:{source_id}:{actual_key}")
        wiki_ids = tibiawiki_targets.get(expected_key)
        if not wiki_ids or page_id not in wiki_ids:
            raise GeneratorError(f"CROSS_CHECK_TIBIAWIKI_TARGET_MISSING:{expected_key}")


def build_bindings(
    allocations: list[tuple[int, str]], source_revision: str
) -> list[dict[str, Any]]:
    bindings = [
        {
            "disposition": "EXACT",
            "external_id": str(source_id),
            "identity_namespace": IDENTITY_NAMESPACE,
            "source_key": SOURCE_KEY,
            "source_revision": source_revision,
            "target": {"family": "Item", "key": key, "revision": DEFINITION_REVISION},
        }
        for source_id, key in allocations
    ]
    bindings.sort(key=canonical_bytes)
    return bindings


def generate() -> tuple[dict[str, Any], bytes]:
    text = read_text(RUST_SOURCE)
    source_revision = parse_source_revision(text)
    namespace = parse_opaque_namespace(text)
    native_batch = parse_native_batch(text)
    records = load_identity_records()
    allocations = apply_identity_promotions(
        allocate_keys(records, native_batch, namespace),
        parse_identity_promotions(text),
    )
    definition_keys = load_definition_keys()
    tibiawiki_targets = load_tibiawiki_targets()
    verify_allocations(allocations, definition_keys, tibiawiki_targets)
    bindings = build_bindings(allocations, source_revision)
    if len(bindings) != EXPECTED_TOTAL:
        raise GeneratorError("BINDING_COUNT_MISMATCH")
    if (
        len({(row["external_id"], row["target"]["key"]) for row in bindings})
        != EXPECTED_TOTAL
    ):
        raise GeneratorError("BINDING_UNIQUENESS")
    output = {"schema": SCHEMA, "family": "Item", "bindings": bindings}
    return output, canonical_bytes(output)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="Regenerate in memory and require the on-disk output to already match, without writing.",
    )
    args = parser.parse_args()

    output, payload = generate()
    bindings = output["bindings"]

    if args.check:
        if not OUTPUT.exists():
            raise GeneratorError("OUTPUT_MISSING")
        if OUTPUT.read_bytes() != payload:
            raise GeneratorError("OUTPUT_DRIFT")
        print(
            f"g4_item_crystal_binding_generator --check: PASS bindings={len(bindings)} bytes={len(payload)}"
        )
        return 0

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_bytes(payload)
    print(
        f"g4_item_crystal_binding_generator: PASS bindings={len(bindings)} bytes={len(payload)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
