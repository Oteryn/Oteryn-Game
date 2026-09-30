#!/usr/bin/env python3
"""Generate explicit EXACT Crystal -> canonical Item identity bindings.

Since ITEM-ID-1b (decision `A12-ITEM-IDENTITY-TIBIA-ID-V1`, D146-D149) the canonical key of a
CipSoft item is its Tibia id, `oteryn:item.tibia.i<id>`. Crystal `items.xml` carries no
`clientid`, so a Crystal server id is the appearance object id, and a bound row targets the
Tibia key of its own id (A12 §4.2).

Every binding row is first reproduced in the historical key space, from the protected inputs
that minted the retired keys, and the historical epoch-1 bytes must still match their
digest:

- epoch 1: the CW2-B1 allocator (`apps/game-server/src/content/cw2_b1_import.rs`) walks the
  protected catalogue
  (`docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json`,
  source `zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a`
  `data/items/items.xml`): the 64 `NATIVE_ITEM_BATCH` rows keep their semantic key, every
  other row gets `oteryn:item.registry.i%08d`, and each declared identity promotion
  (`<PREFIX>_SOURCE_ITEM_ID` / `<PREFIX>_OLD_KEY` / `<PREFIX>_KEY`, e.g. R7 P04 gold coin)
  renames one key in place;
- epoch 2 (decision `A8-DONOR-ITEM-IDENTITY-EPOCH-V1`): the donor-only ids of the Crystal
  `summer-update` census that pass the alias gate below as `NO_MATCH` get opaque keys from
  sequence 38,094.

Each historical row is then requalified through the append-only alias table
(`content/items/aliases.json`), whose entry for the row's retired key carries that row's own
§4.2 evidence (membership and identity-projection continuity):

- `ALIAS` -> an `EXACT` binding to the alias target, which must be `tibia.i<external_id>`;
- `RETIRED_WITHOUT_SUCCESSOR` (D149, no CipSoft appearance in any admitted file) -> no
  binding; the id stays `UNKNOWN` and its crosswalk record is the alias-table entry.

Per `docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md` section 3, an
external identifier becomes an explicit, typed
`(target, source, identity_namespace, external_id, disposition)` binding record, written as
`imports/crystalserver/bindings/items.json`. Nothing here mints identity: every key is the
rule applied to a proven id.

The alias gate (`A8-ALIAS-GATE-V1`, `resolve_alias_gate`) is the G4 promotion
discipline applied to a donor id against the existing Items: names only discover
candidates (G4 rule 9); identity rests on non-name, non-presentation signals
(article/plural and the full items.xml attribute set). The appearance sprite
signature is presentation: it can corroborate an alias, but a sprite-only
difference never proves a distinct identity, so such an id is held (no key, no
alias) instead of minted. Its result is committed as crosswalk evidence, and the default and
`--check` paths consume that evidence offline. `--build-alias-crosswalk` and
`--verify-alias-crosswalk` recompute it from local checkouts of the two pinned
Crystal revisions.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import defaultdict
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
ALIAS_TABLE = ROOT / "content/items/aliases.json"
TIBIA_KEY_PREFIX = "oteryn:item.tibia.i"
TIBIA_KEY = re.compile(r"oteryn:item\.tibia\.i([1-9][0-9]*)")

ITEM_AUTHORING = ROOT / "tools/content-schema/item-authoring"
DONOR_CENSUS = (
    ITEM_AUTHORING / "samples/donor-census-crystal-summer-update-00ce02a5.json"
)
ALIAS_CROSSWALK = (
    ROOT
    / "docs/agents/evidence/OTV2-20260928-item-donor-identity-b1b-alias-crosswalk.json"
)

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

# Epoch 1 as committed on `main` before epoch 2: the canonical bytes of the frozen
# allocator's 38,157 bindings. Regenerating epoch 1 must reproduce this digest.
EXPECTED_EPOCH1_OUTPUT_BYTES = 10_864_254
EXPECTED_EPOCH1_OUTPUT_SHA256 = (
    "74ba56c5e624e5e3ebc3f4ebff5f4ccbb73cee5e2802d79a5b9e6b1d5884b810"
)
# A12: 38,157 epoch-1 rows less the 4,590 D149 rows, plus the 404 epoch-2 rows.
EXPECTED_BOUND_EPOCH1 = 33_567
EXPECTED_BOUND = 33_971
EXPECTED_D149_UNBOUND = 4_590

# Epoch 2 (A8 decision). Every value below is cross-checked against the Rust pins.
EPOCH2_DECISION = "A8-DONOR-ITEM-IDENTITY-EPOCH-V1"
EPOCH2_GATE_RULE = "A8-ALIAS-GATE-V1"
EPOCH2_CROSSWALK_SCHEMA = "OTERYN_ITEM_DONOR_ALIAS_CROSSWALK/v1"
EPOCH2_CENSUS_SCHEMA = "OTERYN_ITEM_DONOR_CENSUS/v1"
EPOCH2_MINTING_STATE = "NO_MATCH"
EPOCH2_BOUND_STATES = ("EXACT", "ACCEPTED_ALIAS")
EPOCH2_UNBOUND_STATES = ("PROBABLE_MATCH", "AMBIGUOUS", "CONFLICT")
EPOCH2_STATES = (EPOCH2_MINTING_STATE, *EPOCH2_BOUND_STATES, *EPOCH2_UNBOUND_STATES)
EPOCH2_DONOR_ITEMS_XML = "data/items/items.xml"
EPOCH2_DONOR_APPEARANCES = "data/items/appearances.dat"

# Golden cross-checks: (crystal source_item_id, expected canonical key,
# TibiaWiki mediawiki page_id already bound EXACT to that same key). These
# prove this generator cannot silently disagree with an already-accepted
# TibiaWiki EXACT binding that targets the same canonical Item -- at minimum
# Magic Sword (i00003167 / Crystal 3288 / TibiaWiki page 5810).
CROSS_CHECKS: tuple[tuple[int, str, str], ...] = (
    (3288, "oteryn:item.tibia.i3288", "5810"),
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


def current_appearance_ids() -> set[int]:
    """Ids of the newest admitted CipSoft appearance manifest (A12 section 4.1)."""
    sys.path.insert(0, str(ROOT / "tools/content-schema/item-authoring"))
    from appearance_membership import load_admitted

    index, manifests = load_admitted()
    return {entry[0] for entry in manifests[index["newest"]]["entries"]}


def rule_only_definition_keys(
    definition_keys: set[str], crystal_ids: set[str], current_ids: set[int]
) -> set[str]:
    """Tibia keys of current CipSoft ids that no Crystal row names: records keyed by the
    section 4.1 rule alone, with no Crystal binding (ITEM-ADD-1 owner decision 2a)."""
    keys = set()
    for key in definition_keys:
        match = TIBIA_KEY.fullmatch(key)
        if match and match.group(1) not in crystal_ids and int(match.group(1)) in current_ids:
            keys.add(key)
    return keys


def load_tibiawiki_targets() -> dict[str, set[str]]:
    payload = json.loads(read_text(TIBIAWIKI_BINDINGS))
    targets: dict[str, set[str]] = {}
    for binding in payload.get("bindings", []):
        targets.setdefault(binding["target"]["key"], set()).add(binding["external_id"])
    return targets


def verify_allocations(
    bound_epoch1: list[tuple[int, str]],
    definition_keys: set[str],
    tibiawiki_targets: dict[str, set[str]],
) -> None:
    if len(bound_epoch1) != EXPECTED_BOUND_EPOCH1:
        raise GeneratorError("ALLOCATION_COUNT_MISMATCH")
    allocation_keys = {key for _, key in bound_epoch1}
    if len(allocation_keys) != EXPECTED_BOUND_EPOCH1:
        raise GeneratorError("ALLOCATION_KEY_UNIQUENESS")
    missing = allocation_keys - definition_keys
    if missing:
        raise GeneratorError(
            f"ALLOCATED_KEY_MISSING_FROM_DEFINITIONS:{sorted(missing)[:5]}"
        )
    extra = definition_keys - allocation_keys
    if extra:
        raise GeneratorError(f"DEFINITION_KEY_WITHOUT_ALLOCATION:{sorted(extra)[:5]}")

    by_source = dict(bound_epoch1)
    for source_id, expected_key, page_id in CROSS_CHECKS:
        actual_key = by_source.get(source_id)
        if actual_key != expected_key:
            raise GeneratorError(f"CROSS_CHECK_KEY_MISMATCH:{source_id}:{actual_key}")
        wiki_ids = tibiawiki_targets.get(expected_key)
        if not wiki_ids or page_id not in wiki_ids:
            raise GeneratorError(f"CROSS_CHECK_TIBIAWIKI_TARGET_MISSING:{expected_key}")


def load_alias_entries() -> dict[str, dict[str, Any]]:
    """The current (latest-version) alias-table entry of every retired Item key."""
    current: dict[str, dict[str, Any]] = {}
    for entry in json.loads(read_text(ALIAS_TABLE))["entries"]:
        current[entry["key"]] = entry
    return current


def requalify(
    rows: list[dict[str, Any]], aliases: dict[str, dict[str, Any]]
) -> tuple[list[dict[str, Any]], int]:
    """Historical rows -> Tibia-key bindings; D149 rows emit none (A12 §4.2, §4.5)."""
    bound = []
    unbound = 0
    for row in rows:
        entry = aliases.get(row["target"]["key"])
        if entry is None:
            raise GeneratorError(f"HISTORICAL_KEY_WITHOUT_ALIAS:{row['target']['key']}")
        evidence = entry["evidence"]
        if (
            str(evidence["source_item_id"]) != row["external_id"]
            or evidence["source_revision"] != row["source_revision"]
        ):
            raise GeneratorError(f"ALIAS_EVIDENCE_NOT_THIS_ROW:{row['target']['key']}")
        if entry["state"] == "RETIRED_WITHOUT_SUCCESSOR":
            unbound += 1
            continue
        target = f"{TIBIA_KEY_PREFIX}{int(row['external_id'])}"
        if entry["state"] != "ALIAS" or entry["target"] != target:
            raise GeneratorError(f"ALIAS_TARGET_NOT_OWN_ID:{row['target']['key']}")
        bound.append(
            {
                **row,
                "disposition": "EXACT",
                "target": {**row["target"], "key": target},
            }
        )
    return bound, unbound


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


def rust_literal(text: str, name: str) -> str | int:
    """Read one `pub const NAME: T = <string or integer literal>;` from the Rust source."""
    match = re.search(rf'pub const {name}: [^=]+=\s*(?:"([^"]*)"|([0-9_]+))\s*;', text)
    if not match:
        raise GeneratorError(f"RUST_CONST_NOT_FOUND:{name}")
    if match.group(1) is not None:
        return match.group(1)
    return int(match.group(2).replace("_", ""))


def parse_epoch2_pins(text: str) -> dict[str, Any]:
    """Epoch-2 pins declared beside the frozen import; never hand-copied here."""
    names = {
        "source_revision": "SOURCE_REVISION",
        "items_xml_sha256": "ITEMS_XML_SHA256",
        "census_bytes": "CENSUS_BYTES",
        "census_sha256": "CENSUS_SHA256",
        "census_id_count": "CENSUS_ID_COUNT",
        "crosswalk_bytes": "CROSSWALK_BYTES",
        "crosswalk_sha256": "CROSSWALK_SHA256",
        "minted_count": "MINTED_COUNT",
        "allocation_digest_sha256": "ALLOCATION_DIGEST_SHA256",
        "revision": "REVISION",
    }
    return {
        key: rust_literal(text, f"CW2_B1_DONOR_EPOCH2_{suffix}")
        for key, suffix in names.items()
    }


# --- epoch 2: alias gate ------------------------------------------------------------


def normalize_name(name: str | None) -> str:
    return " ".join((name or "").split()).casefold()


def visual_signature(appearance: dict[str, Any] | None) -> str | None:
    """Sprite ids and geometry of every frame group; `None` when there is nothing to compare."""
    groups = (appearance or {}).get("frame_groups") or []
    if not groups:
        return None
    return json.dumps(
        [[group["sprite_ids"], group["geometry"]] for group in groups],
        sort_keys=True,
        separators=(",", ":"),
    )


def compare_alias_signals(
    donor: dict[str, Any], base: dict[str, Any]
) -> tuple[list[str], list[str]]:
    """Return `(matched, contradicted)` non-name signals of one donor/base pair.

    Each dict carries `article`, `plural`, `attrs` and `visual`. A signal that is not
    comparable (no appearance on either side) is neither matched nor contradicted.
    """
    matched: list[str] = []
    contradicted: list[str] = []
    for signal, equal in (
        (
            "article_plural",
            (donor["article"], donor["plural"]) == (base["article"], base["plural"]),
        ),
        ("attributes", donor["attrs"] == base["attrs"]),
    ):
        (matched if equal else contradicted).append(signal)
    if donor["visual"] is not None and base["visual"] is not None:
        (matched if donor["visual"] == base["visual"] else contradicted).append(
            "visual"
        )
    return matched, contradicted


IDENTITY_SIGNALS = ("article_plural", "attributes")


def counterpart(row: dict[str, Any]) -> bool:
    """Both non-presentation signals agree. Presentation (`visual`) is never an identity signal."""
    return all(signal in row["matched"] for signal in IDENTITY_SIGNALS)


def resolve_alias_gate(
    candidates: list[dict[str, Any]],
) -> tuple[str, str, list[dict[str, Any]]]:
    """`A8-ALIAS-GATE-V1`: crosswalk state of one donor id from its same-name base items.

    `candidates` are the existing base Items whose normalized name equals the donor's
    (discovery only; names never decide, G4 rule 9). Each carries `base_source_item_id`,
    `matched` and `contradicted` from `compare_alias_signals`. Returns
    `(state, reason, evidence_rows)`.

    Identity rests on non-presentation facts only (`IDENTITY_SIGNALS`). A sprite or
    appearance change never remints an Item identity (G4 decision, identity layers), so
    a differing sprite signature is never evidence of a distinct identity:
    - a candidate whose article/plural and full attribute set agree is a counterpart;
    - a unique counterpart is `ACCEPTED_ALIAS` only when the visual signature also
      agrees, and otherwise a held `PROBABLE_MATCH` (no key, no binding);
    - several counterparts are `AMBIGUOUS`;
    - a candidate that shares the visual signature but contradicts a non-presentation
      signal is a conflicting counterpart (`CONFLICT` when alone);
    - only a candidate contradicted by non-presentation facts, or no candidate at all,
      leaves `NO_MATCH`.
    """
    rows = [
        {
            "base_source_item_id": row["base_source_item_id"],
            "matched": list(row["matched"]),
            "contradicted": list(row["contradicted"]),
        }
        for row in candidates
    ]
    if not rows:
        return "NO_MATCH", "NO_SAME_NAME_BASE_ITEM", rows
    counterparts = [row for row in rows if counterpart(row)]
    conflicting = [
        row for row in rows if not counterpart(row) and "visual" in row["matched"]
    ]
    if not counterparts:
        if conflicting:
            return "CONFLICT", "SAME_VISUAL_OBJECT_CONTRADICTORY_FACTS", rows
        return (
            "NO_MATCH",
            "SAME_NAME_CANDIDATES_CONTRADICTED_BY_NON_PRESENTATION_FACTS",
            rows,
        )
    if len(counterparts) > 1 or conflicting:
        return "AMBIGUOUS", "MULTIPLE_COUNTERPART_CANDIDATES", rows
    if "visual" in counterparts[0]["matched"]:
        return (
            "ACCEPTED_ALIAS",
            "UNIQUE_COUNTERPART_NON_PRESENTATION_FACTS_AND_VISUAL_AGREE",
            rows,
        )
    if "visual" in counterparts[0]["contradicted"]:
        return "PROBABLE_MATCH", "SPRITE_ONLY_DIFFERENCE_HELD", rows
    return "PROBABLE_MATCH", "UNIQUE_COUNTERPART_WITHOUT_VISUAL_SIGNAL", rows


def load_census() -> tuple[dict[str, Any], bytes]:
    payload = DONOR_CENSUS.read_bytes()
    census = json.loads(payload)
    if census.get("schema") != EPOCH2_CENSUS_SCHEMA:
        raise GeneratorError("CENSUS_SCHEMA_MISMATCH")
    return census, payload


def census_ids(census: dict[str, Any]) -> list[int]:
    """Census donor ids in ascending order; the census is the frozen epoch-2 corpus."""
    rows = census.get("items")
    if not isinstance(rows, dict) or not rows:
        raise GeneratorError("CENSUS_ITEMS_INVALID")
    ids = sorted(int(key) for key in rows)
    if len(set(ids)) != len(ids):
        raise GeneratorError("CENSUS_ID_DUPLICATE")
    return ids


# --- epoch 2: allocation ------------------------------------------------------------

# The highest sequence allocated by any earlier epoch: epoch 1 is contiguous 1..38,093.
EPOCH1_HIGHEST_SEQUENCE = EXPECTED_OPAQUE


def allocate_epoch2(
    rows: list[dict[str, Any]], namespace: str
) -> list[tuple[int, str]]:
    """`NO_MATCH` ids, ascending by donor source id, numbered after the earlier epochs."""
    previous: int | None = None
    minting: list[int] = []
    for row in rows:
        source_id = row["source_item_id"]
        if not isinstance(source_id, int) or isinstance(source_id, bool):
            raise GeneratorError("CROSSWALK_SOURCE_ID_INVALID")
        if previous is not None and previous >= source_id:
            raise GeneratorError("CROSSWALK_SOURCE_ID_ORDER_VIOLATION")
        previous = source_id
        if row["state"] not in EPOCH2_STATES:
            raise GeneratorError(f"CROSSWALK_STATE_INVALID:{source_id}")
        if row["state"] == EPOCH2_MINTING_STATE:
            minting.append(source_id)
    return [
        (source_id, opaque_item_key(namespace, EPOCH1_HIGHEST_SEQUENCE + rank))
        for rank, source_id in enumerate(minting, start=1)
    ]


def allocation_digest(allocations: list[tuple[int, str]]) -> str:
    """Same construction as the frozen import: `id NUL key LF`, hashed with SHA-256."""
    payload = b"".join(
        str(source_id).encode() + b"\x00" + key.encode() + b"\n"
        for source_id, key in allocations
    )
    return sha256_hex(payload)


def build_alias_crosswalk(
    donor_root: Path,
    base_root: Path,
    text: str,
    epoch1_keys_by_source: dict[int, str],
) -> bytes:
    """Recompute the alias gate for every census id from the two pinned checkouts."""
    pins = parse_epoch2_pins(text)
    sys.path.insert(0, str(ITEM_AUTHORING))
    try:
        import engine_items  # type: ignore[import-not-found]
    finally:
        sys.path.pop(0)

    census, census_payload = load_census()
    ids = census_ids(census)
    donor_meta = census["donor"]
    base_meta = census["base"]
    if donor_meta.get("commit") != pins["source_revision"] or base_meta.get(
        "revision"
    ) != parse_source_revision(text):
        raise GeneratorError("CENSUS_REVISION_MISMATCH")
    base_digests = engine_items.ENGINE_ARTIFACT_DIGESTS[engine_items.CRYSTAL_PROFILE]
    digests = {
        "donor": {
            path: donor_meta["artifact_digests"][path]["sha256"]
            for path in (EPOCH2_DONOR_ITEMS_XML, EPOCH2_DONOR_APPEARANCES)
        },
        "base": {
            path: base_digests[path]
            for path in (EPOCH2_DONOR_ITEMS_XML, EPOCH2_DONOR_APPEARANCES)
        },
    }

    def load(root: Path, role: str) -> tuple[dict[int, Any], dict[int, Any]]:
        items_bytes, _ = engine_items.read_verified_artifact(
            root, EPOCH2_DONOR_ITEMS_XML, digests[role][EPOCH2_DONOR_ITEMS_XML]
        )
        appearance_bytes, _ = engine_items.read_verified_artifact(
            root, EPOCH2_DONOR_APPEARANCES, digests[role][EPOCH2_DONOR_APPEARANCES]
        )
        return (
            engine_items.load_items_xml(items_bytes.decode("utf-8")),
            engine_items.load_appearance_objects(appearance_bytes),
        )

    donor_items, donor_appearances = load(donor_root, "donor")
    base_items, base_appearances = load(base_root, "base")
    if sorted(set(donor_items) - set(base_items)) != ids:
        raise GeneratorError("CENSUS_IDS_NOT_DONOR_ONLY_SET")
    if digests["donor"][EPOCH2_DONOR_ITEMS_XML] != pins["items_xml_sha256"]:
        raise GeneratorError("DONOR_ITEMS_XML_DIGEST_MISMATCH")

    def signals(record: dict[str, Any], appearance: dict[str, Any] | None) -> dict:
        return {
            "article": record["article"],
            "plural": record["plural"],
            "attrs": record["attrs"],
            "visual": visual_signature(appearance),
        }

    by_name: dict[str, list[int]] = defaultdict(list)
    for base_id in sorted(base_items):
        by_name[normalize_name(base_items[base_id]["name"])].append(base_id)

    rows: list[dict[str, Any]] = []
    for donor_id in ids:
        record = donor_items[donor_id]
        donor_signals = signals(record, donor_appearances.get(donor_id))
        candidates = []
        for base_id in by_name.get(normalize_name(record["name"]), []):
            matched, contradicted = compare_alias_signals(
                donor_signals,
                signals(base_items[base_id], base_appearances.get(base_id)),
            )
            candidates.append(
                {
                    "base_source_item_id": base_id,
                    "matched": matched,
                    "contradicted": contradicted,
                }
            )
        state, reason, evidence_rows = resolve_alias_gate(candidates)
        row: dict[str, Any] = {
            "name": census["items"][str(donor_id)]["name"],
            "reason": reason,
            "source_item_id": donor_id,
            "state": state,
        }
        if evidence_rows:
            row["same_name_base_items"] = evidence_rows
        if state in EPOCH2_BOUND_STATES:
            target = next(c for c in evidence_rows if counterpart(c))
            row["alias_target_source_item_id"] = target["base_source_item_id"]
            row["alias_target_key"] = epoch1_keys_by_source[
                target["base_source_item_id"]
            ]
        rows.append(row)

    allocations = allocate_epoch2(rows, parse_opaque_namespace(text))
    evidence = {
        "base": {
            "appearances_sha256": digests["base"][EPOCH2_DONOR_APPEARANCES],
            "items_xml_sha256": digests["base"][EPOCH2_DONOR_ITEMS_XML],
            "repository": base_meta["repository"],
            "revision": base_meta["revision"],
        },
        "census": {
            "bytes": len(census_payload),
            "path": DONOR_CENSUS.relative_to(ROOT).as_posix(),
            "sha256": sha256_hex(census_payload),
        },
        "counts": {
            "by_state": {
                state: sum(1 for row in rows if row["state"] == state)
                for state in EPOCH2_STATES
            },
            "census_ids": len(ids),
            "minted": len(allocations),
        },
        "decision": EPOCH2_DECISION,
        "donor": {
            "appearances_sha256": digests["donor"][EPOCH2_DONOR_APPEARANCES],
            "branch": donor_meta["branch"],
            "commit": donor_meta["commit"],
            "items_xml_sha256": digests["donor"][EPOCH2_DONOR_ITEMS_XML],
            "repository": donor_meta["repository"],
        },
        "epoch_2": {
            "allocation_digest_sha256": allocation_digest(allocations),
            "first_sequence": EPOCH1_HIGHEST_SEQUENCE + 1,
            "last_sequence": EPOCH1_HIGHEST_SEQUENCE + len(allocations),
            "namespace": parse_opaque_namespace(text),
        },
        "gate": {
            "rule": EPOCH2_GATE_RULE,
            "signals": {
                "article_plural": "identity signal: article and plural attributes equal",
                "attributes": "identity signal: the complete items.xml attribute set equal",
                "name": "discovery only, never counted as agreement (G4 rule 9)",
                "visual": "presentation, never an identity signal: it can corroborate an alias but a difference never proves a distinct identity",
            },
            "states": {
                "ACCEPTED_ALIAS": "unique counterpart; article/plural and attributes agree and the visual signature agrees; binds to the existing key and mints nothing",
                "AMBIGUOUS": "several counterparts; held: mints and binds nothing",
                "CONFLICT": "same visual object with contradictory non-presentation facts; held: mints and binds nothing",
                "NO_MATCH": "no same-name item, or every same-name item contradicted by non-presentation facts; the only state that mints",
                "PROBABLE_MATCH": "unique counterpart on non-presentation facts whose visual signature differs or is absent; held: mints and binds nothing until non-presentation evidence proves a distinct identity or an alias",
            },
        },
        "rows": rows,
        "schema": EPOCH2_CROSSWALK_SCHEMA,
    }
    return (
        json.dumps(evidence, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    ).encode("utf-8")


def load_alias_crosswalk(pins: dict[str, Any]) -> dict[str, Any]:
    payload = ALIAS_CROSSWALK.read_bytes()
    if (
        len(payload) != pins["crosswalk_bytes"]
        or sha256_hex(payload) != pins["crosswalk_sha256"]
    ):
        raise GeneratorError("ALIAS_CROSSWALK_DIGEST_MISMATCH")
    crosswalk = json.loads(payload)
    if crosswalk.get("schema") != EPOCH2_CROSSWALK_SCHEMA:
        raise GeneratorError("ALIAS_CROSSWALK_SCHEMA_MISMATCH")
    return crosswalk


def epoch2_allocations(
    census: dict[str, Any],
    census_payload: bytes,
    crosswalk: dict[str, Any],
    pins: dict[str, Any],
    epoch1_allocations: list[tuple[int, str]],
    namespace: str,
) -> list[tuple[int, str]]:
    """Fail-closed epoch-2 allocation from the frozen census and alias-gate evidence."""
    if (
        len(census_payload) != pins["census_bytes"]
        or sha256_hex(census_payload) != pins["census_sha256"]
    ):
        raise GeneratorError("CENSUS_DIGEST_MISMATCH")
    donor = census["donor"]
    if (
        donor.get("commit") != pins["source_revision"]
        or donor["artifact_digests"][EPOCH2_DONOR_ITEMS_XML]["sha256"]
        != pins["items_xml_sha256"]
    ):
        raise GeneratorError("CENSUS_DONOR_PIN_MISMATCH")
    ids = census_ids(census)
    if len(ids) != pins["census_id_count"]:
        raise GeneratorError("CENSUS_ID_COUNT_MISMATCH")
    if crosswalk["census"]["sha256"] != pins["census_sha256"]:
        raise GeneratorError("ALIAS_CROSSWALK_CENSUS_BINDING_MISMATCH")
    rows = crosswalk["rows"]
    if [row["source_item_id"] for row in rows] != ids:
        raise GeneratorError("ALIAS_CROSSWALK_ROWS_NOT_THE_CENSUS_IDS")

    epoch1_by_source = dict(epoch1_allocations)
    if set(ids) & set(epoch1_by_source):
        raise GeneratorError("EPOCH2_SOURCE_ID_SHARED_WITH_EPOCH1")
    for row in rows:
        source_id = row["source_item_id"]
        if row["state"] in EPOCH2_BOUND_STATES:
            if epoch1_by_source.get(row.get("alias_target_source_item_id")) != row.get(
                "alias_target_key"
            ):
                raise GeneratorError(f"ALIAS_TARGET_NOT_EPOCH1_KEY:{source_id}")
        elif "alias_target_key" in row:
            raise GeneratorError(f"UNBOUND_STATE_WITH_TARGET:{source_id}")

    allocations = allocate_epoch2(rows, namespace)
    keys = [key for _, key in allocations]
    if len(set(keys)) != len(keys) or set(keys) & {
        key for _, key in epoch1_allocations
    }:
        raise GeneratorError("EPOCH2_KEY_COLLISION")
    if len(allocations) != pins["minted_count"]:
        raise GeneratorError("EPOCH2_MINTED_COUNT_MISMATCH")
    if allocation_digest(allocations) != pins["allocation_digest_sha256"]:
        raise GeneratorError("EPOCH2_ALLOCATION_DIGEST_MISMATCH")
    if (
        crosswalk["epoch_2"]["allocation_digest_sha256"]
        != pins["allocation_digest_sha256"]
    ):
        raise GeneratorError("ALIAS_CROSSWALK_ALLOCATION_DIGEST_MISMATCH")
    return allocations


def epoch2_bindings(
    allocations: list[tuple[int, str]],
    crosswalk: dict[str, Any],
    source_revision: str,
    revision: str,
) -> list[dict[str, Any]]:
    """Ascending-source-id bindings: EXACT for minted ids, ACCEPTED_ALIAS for aliases.

    Ids in `PROBABLE_MATCH`, `AMBIGUOUS` or `CONFLICT` get neither a key nor a binding.
    """
    minted = dict(allocations)
    bindings = []
    for row in crosswalk["rows"]:
        source_id = row["source_item_id"]
        if row["state"] == EPOCH2_MINTING_STATE:
            key, disposition = minted[source_id], "EXACT"
        elif row["state"] in EPOCH2_BOUND_STATES:
            key, disposition = row["alias_target_key"], "ACCEPTED_ALIAS"
        else:
            continue
        bindings.append(
            {
                "disposition": disposition,
                "external_id": str(source_id),
                "identity_namespace": IDENTITY_NAMESPACE,
                "source_key": SOURCE_KEY,
                "source_revision": source_revision,
                "target": {"family": "Item", "key": key, "revision": revision},
            }
        )
    return bindings


def generate() -> tuple[dict[str, Any], bytes]:
    text = read_text(RUST_SOURCE)
    pins = parse_epoch2_pins(text)
    source_revision = parse_source_revision(text)
    namespace = parse_opaque_namespace(text)
    epoch1_allocations = apply_identity_promotions(
        allocate_keys(load_identity_records(), parse_native_batch(text), namespace),
        parse_identity_promotions(text),
    )
    census, census_payload = load_census()
    crosswalk = load_alias_crosswalk(pins)
    allocations = epoch2_allocations(
        census, census_payload, crosswalk, pins, epoch1_allocations, namespace
    )
    epoch1 = build_bindings(epoch1_allocations, source_revision)
    if len(epoch1) != EXPECTED_TOTAL:
        raise GeneratorError("BINDING_COUNT_MISMATCH")
    if (
        len({(row["external_id"], row["target"]["key"]) for row in epoch1})
        != EXPECTED_TOTAL
    ):
        raise GeneratorError("BINDING_UNIQUENESS")
    # History: the historical epoch-1 rows must still be the bytes #1279 retired.
    epoch1_bytes = canonical_bytes(
        {"schema": SCHEMA, "family": "Item", "bindings": epoch1}
    )
    if (
        len(epoch1_bytes) != EXPECTED_EPOCH1_OUTPUT_BYTES
        or sha256_hex(epoch1_bytes) != EXPECTED_EPOCH1_OUTPUT_SHA256
    ):
        raise GeneratorError("EPOCH1_BINDINGS_DRIFT")
    epoch2 = epoch2_bindings(
        allocations, crosswalk, pins["source_revision"], pins["revision"]
    )
    if {row["external_id"] for row in epoch1} & {row["external_id"] for row in epoch2}:
        raise GeneratorError("BINDING_SOURCE_ID_SHARED_ACROSS_EPOCHS")
    if {row["target"]["key"] for row in epoch2 if row["disposition"] == "EXACT"} & {
        row["target"]["key"] for row in epoch1
    }:
        raise GeneratorError("EPOCH2_KEY_COLLIDES_WITH_BOUND_KEY")

    aliases = load_alias_entries()
    bound_epoch1, unbound = requalify(epoch1, aliases)
    bound_epoch2, unbound_epoch2 = requalify(epoch2, aliases)
    if unbound != EXPECTED_D149_UNBOUND or unbound_epoch2:
        raise GeneratorError(f"D149_UNBOUND_COUNT:{unbound}:{unbound_epoch2}")
    # Every epoch-1 target is exactly the definition key set less the epoch-2 targets and the
    # keys of current CipSoft ids no Crystal row names (ITEM-ADD-1, A12 section 4.1).
    definition_keys = load_definition_keys()
    epoch2_targets = {row["target"]["key"] for row in bound_epoch2}
    rule_only = rule_only_definition_keys(
        definition_keys - epoch2_targets,
        {row["external_id"] for row in epoch1 + epoch2},
        current_appearance_ids(),
    )
    verify_allocations(
        [(int(row["external_id"]), row["target"]["key"]) for row in bound_epoch1],
        definition_keys - epoch2_targets - rule_only,
        load_tibiawiki_targets(),
    )
    bindings = sorted(bound_epoch1 + bound_epoch2, key=canonical_bytes)
    if (
        len(bindings) != EXPECTED_BOUND
        or len({row["target"]["key"] for row in bindings}) != EXPECTED_BOUND
    ):
        raise GeneratorError("BOUND_TARGET_UNIQUENESS")
    output = {"schema": SCHEMA, "family": "Item", "bindings": bindings}
    return output, canonical_bytes(output)


def alias_crosswalk_main(args: argparse.Namespace) -> int:
    if not args.donor_root or not args.base_root:
        raise GeneratorError("DONOR_AND_BASE_ROOT_REQUIRED")
    text = read_text(RUST_SOURCE)
    epoch1_allocations = apply_identity_promotions(
        allocate_keys(
            load_identity_records(),
            parse_native_batch(text),
            parse_opaque_namespace(text),
        ),
        parse_identity_promotions(text),
    )
    payload = build_alias_crosswalk(
        args.donor_root, args.base_root, text, dict(epoch1_allocations)
    )
    digest = sha256_hex(payload)
    if args.verify_alias_crosswalk:
        if not ALIAS_CROSSWALK.exists() or ALIAS_CROSSWALK.read_bytes() != payload:
            raise GeneratorError("ALIAS_CROSSWALK_DRIFT")
        print(
            f"g4_item_crystal_binding_generator --verify-alias-crosswalk: PASS bytes={len(payload)} sha256={digest}"
        )
        return 0
    ALIAS_CROSSWALK.write_bytes(payload)
    print(
        f"g4_item_crystal_binding_generator --build-alias-crosswalk: wrote bytes={len(payload)} sha256={digest}"
    )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="Regenerate in memory and require the on-disk output to already match, without writing.",
    )
    parser.add_argument(
        "--build-alias-crosswalk",
        action="store_true",
        help="Recompute the epoch-2 alias-gate evidence from --donor-root/--base-root and write it.",
    )
    parser.add_argument(
        "--verify-alias-crosswalk",
        action="store_true",
        help="Recompute the alias-gate evidence and require the committed file to match, without writing.",
    )
    parser.add_argument("--donor-root", type=Path, help="Checkout of the donor commit.")
    parser.add_argument(
        "--base-root", type=Path, help="Checkout of the pinned base revision."
    )
    args = parser.parse_args()

    if args.build_alias_crosswalk or args.verify_alias_crosswalk:
        return alias_crosswalk_main(args)

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
