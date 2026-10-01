"""Extend navigation taxonomy from the admitted wiki snapshot, without minting Items."""

import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/content-schema/item-authoring"))
from engine_items import (
    build_identity_index,
    load_appearance_objects,
    load_wiki_family_fallback,
    resolve_wiki_family_value,
)

SNAPSHOT = "imports/tibiawiki/facts/items-stats.json"
# The census omitted this BR navigation label; both the weapons and schema already
# distinguish throwing/distance weapons from melee weapons.
CLIENT_DIGEST = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
CLIENT_PATH = f"content/assets/files/appearances-{CLIENT_DIGEST}.dat"
FALLBACK_PATH = "imports/tibiawiki/facts/items-family-fallback.json"
# Only semantic clothing slots are admitted; hand slots do not identify a family.
MARKET_PROFILES = {
    1: "equipment_armor",
    3: "equipment_armor",
    7: "equipment_armor",
    8: "equipment_armor",
    2: "equipment_offhand",
    11: "equipment_offhand",
}
MARKET_CLOTHES_SLOT = {1: 4, 3: 8, 7: 1, 8: 7, 2: 2, 11: 9}
CLOTHES_PROFILES = {
    1: "equipment_armor",
    4: "equipment_armor",
    7: "equipment_armor",
    8: "equipment_armor",
    2: "equipment_offhand",
    9: "equipment_offhand",
}
PROFILE_ALIASES = {"Armas de Arremesso": "weapon_distance"}


def legacy_profile(primary, assignments):
    return PROFILE_ALIASES.get(primary, assignments.get(primary))


def build_taxonomy(
    definitions,
    authoring,
    assignments,
    snapshot,
    bound_keys,
    routed_keys,
    fallback=None,
    client=None,
):
    rows = {}
    for key, item in sorted(authoring.items()):
        if "taxonomy" in item:
            rows[key] = {
                "target": item["item"],
                "source_taxonomy": item["taxonomy"],
                "family_profile": legacy_profile(
                    item["taxonomy"]["primary"], assignments
                ),
            }
    for record in sorted(snapshot["records"].values(), key=lambda row: row["item_id"]):
        key = ("Item", f"oteryn:item.tibia.i{record['item_id']}", "definition-r1")
        if (
            key in rows
            or key not in definitions
            or key[1] not in bound_keys
            or key[1] in routed_keys
        ):
            continue
        observations = [
            row for row in record["observations"] if "primarytype" in row["fields"]
        ]
        profiles = {
            resolve_wiki_family_value("primarytype", row["fields"]["primarytype"])
            for row in observations
        }
        # Unknown categories and conflicting pages are holds, not a majority vote.
        if not observations or None in profiles or len(profiles) != 1:
            continue
        rows[key] = {
            "target": definitions[key]["identity"],
            "source_taxonomy": {"primary": observations[0]["fields"]["primarytype"]},
            "family_profile": profiles.pop(),
            "source_evidence": {
                "snapshot": SNAPSHOT,
                "snapshot_sha256": snapshot["snapshot_sha256"],
                "observations": [
                    {
                        field: row[field]
                        for field in ("page_id", "revision_id", "content_sha256", "url")
                    }
                    | {"primarytype": row["fields"]["primarytype"]}
                    for row in observations
                ],
            },
        }
    wiki_by_id = {row["item_id"]: row for row in snapshot["records"].values()}
    for key, definition in sorted(definitions.items()):
        if key in rows or key[1] not in bound_keys or key[1] in routed_keys:
            continue
        item_id = int(key[1].rsplit("i", 1)[1])
        observed = wiki_by_id.get(item_id, {}).get("observations", [])
        # An explicit conflicting or unadmitted category remains a hold even when
        # another source could supply a plausible profile.
        primary = [
            row["fields"]["primarytype"]
            for row in observed
            if "primarytype" in row["fields"]
        ]
        if primary:
            continue
        entry = (fallback or {}).get(key[1])
        if entry:
            evidence = entry["evidence"]
            rows[key] = {
                "target": definition["identity"],
                "source_taxonomy": {
                    "primary": evidence["value"]
                    if evidence["resolution"] == "direct"
                    else evidence["candidates"][0]["value"]
                },
                "family_profile": entry["profile"],
                "source_evidence": {
                    "snapshot": FALLBACK_PATH,
                    "snapshot_sha256": entry["snapshot_sha256"],
                    "qualified_fallback": evidence,
                },
            }
            continue
        appearance = (client or {}).get(item_id)
        if not appearance:
            continue
        flags = appearance["flags"]
        market = MARKET_PROFILES.get(flags.get("market.category"))
        clothes = CLOTHES_PROFILES.get(flags.get("clothes.slot"))
        if (
            not market
            or market != clothes
            or MARKET_CLOTHES_SLOT.get(flags.get("market.category"))
            != flags.get("clothes.slot")
            or flags.get("flags.take") is not True
        ):
            continue
        rows[key] = {
            "target": definition["identity"],
            "source_taxonomy": {
                "primary": f"client market category {flags['market.category']}"
            },
            "family_profile": market,
            "source_evidence": {
                "source": "official_client",
                "client_version": "15.30",
                "artifact": CLIENT_PATH,
                "artifact_sha256": CLIENT_DIGEST,
                "appearance_id": item_id,
                "appearance_name": appearance.get("name"),
                "flags": {
                    field: flags[field]
                    for field in ("flags.take", "market.category", "clothes.slot")
                },
            },
        }
    return [rows[key] for key in sorted(rows)]


def taxonomy_inputs(root=ROOT):
    snapshot = json.loads((root / SNAPSHOT).read_text(encoding="utf-8"))
    digest = hashlib.sha256(
        json.dumps(
            snapshot["records"],
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8")
    ).hexdigest()
    if digest != snapshot["snapshot_sha256"]:
        raise ValueError("TAXONOMY_SNAPSHOT_DIGEST")
    bindings = json.loads(
        (root / "imports/crystalserver/bindings/items.json").read_text(encoding="utf-8")
    )
    bound_keys = {row["target"]["key"] for row in bindings["bindings"]}
    routed_keys = set()
    for family in ("terrain", "objects"):
        for path in (root / "content/world" / family).glob("*.json"):
            for row in json.loads(path.read_text(encoding="utf-8")).get("records", []):
                pointer = row.get("provenance", {}).get("item_pointer", {})
                if pointer:
                    routed_keys.add(pointer["key"])
    fallback_path = root / FALLBACK_PATH
    fallback_digest = json.loads(fallback_path.read_text())["snapshot_sha256"]
    fallback = load_wiki_family_fallback(fallback_path, build_identity_index())
    for entry in fallback.values():
        entry["snapshot_sha256"] = fallback_digest
    client_bytes = (root / CLIENT_PATH).read_bytes()
    if hashlib.sha256(client_bytes).hexdigest() != CLIENT_DIGEST:
        raise ValueError("TAXONOMY_CLIENT_DIGEST")
    client = load_appearance_objects(client_bytes)
    return snapshot, bound_keys, routed_keys, fallback, client
