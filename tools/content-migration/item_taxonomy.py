"""Extend navigation taxonomy from the admitted wiki snapshot, without minting Items."""

import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/content-schema/item-authoring"))
from engine_items import resolve_wiki_family_value

SNAPSHOT = "imports/tibiawiki/facts/items-stats.json"
# The census omitted this BR navigation label; both the weapons and schema already
# distinguish throwing/distance weapons from melee weapons.
PROFILE_ALIASES = {"Armas de Arremesso": "weapon_distance"}


def legacy_profile(primary, assignments):
    return PROFILE_ALIASES.get(primary, assignments.get(primary))


def build_taxonomy(
    definitions, authoring, assignments, snapshot, bound_keys, routed_keys
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
    return snapshot, bound_keys, routed_keys
