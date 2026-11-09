"""Qualify Cradle's named official appearances against captured Wiki weight facts.

These four identities are official appearance IDs, not invented OTS server-ID bindings.
The selected Oteryn crafting route uses stable prepared ingredients; source expiry
observations remain in the accompanying reference evidence.
"""

import argparse
import hashlib
import json
import xml.etree.ElementTree as ET
from pathlib import Path

import build_source_item_profiles as native
import build_world_change_item_profiles as sources

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = Path(__file__).with_name("qready1-cradle-item-profiles.json")
ITEMS = {
    40016: ("Fresh Hydra Herbs", "0.50"),
    40598: ("Bottled Yolk", "1.50"),
    40601: ("Nutrient Solution", "1.50"),
    40603: ("Flask with Anti-Mutagen", "1.50"),
}
MEMBERSHIP = (
    "imports/official/appearance-membership/appearances-"
    + sources.APPEARANCES_SHA256
    + ".json"
)


def build(root=ROOT):
    appearances = (root / sources.APPEARANCES).read_bytes()
    if hashlib.sha256(appearances).hexdigest() != sources.APPEARANCES_SHA256:
        raise ValueError("CRADLE_OFFICIAL_APPEARANCE_DRIFT")
    admitted = json.loads(
        (root / "imports/official/appearance-membership/admitted.json").read_bytes()
    )
    manifest = (root / MEMBERSHIP).read_bytes()
    entry = next(
        row for row in admitted["files"]
        if row["appearances_sha256"] == sources.APPEARANCES_SHA256
    )
    if (
        admitted["key_rule"] != "oteryn:item.tibia.i<id>"
        or hashlib.sha256(manifest).hexdigest() != entry["manifest_sha256"]
    ):
        raise ValueError("CRADLE_OFFICIAL_IDENTITY_DRIFT")
    if not set(ITEMS) <= {row[0] for row in json.loads(manifest)["entries"]}:
        raise ValueError("CRADLE_OFFICIAL_MEMBER_MISSING")
    objects = native.engine_items.load_appearance_objects(appearances)
    raw_stats = (root / "imports/tibiawiki/facts/items-stats.json").read_bytes()
    stats = json.loads(raw_stats)["records"]
    records = []
    evidence = []
    for item_id, (title, weight) in ITEMS.items():
        obj = objects[item_id]
        facts = [row for row in stats[str(item_id)]["observations"] if row["wiki_title"] == title]
        if len(facts) != 1:
            raise ValueError("CRADLE_AMBIGUOUS_WIKI_FACT")
        fact = facts[0]
        if stats[str(item_id)]["item_id"] != item_id:
            raise ValueError("CRADLE_WIKI_ITEM_ID_DRIFT")
        if obj["name"].casefold() != title.casefold() or fact["fields"]["weight"] != weight:
            raise ValueError("CRADLE_NAMED_FACT_DRIFT")
        flags = obj["flags"]
        stackable = fact["fields"].get("stackable") == "yes"
        if (
            flags.get("flags.cumulative", False) != stackable
            or not flags.get("flags.take", False)
            or flags.get("flags.unmove", False)
            or flags.get("flags.container", False)
            or flags.get("flags.unpass", False)
        ):
            raise ValueError("CRADLE_PLAIN_MATERIAL_FLAGS_DRIFT")
        # This element adapts captured Wiki facts to the existing normalizer;
        # it is not an XML donor observation or an OTS identity assertion.
        node = ET.Element("item", id=str(item_id), name=obj["name"])
        ET.SubElement(node, "attribute", key="weight", value=str(int(weight.replace(".", ""))))
        if obj.get("description"):
            ET.SubElement(node, "attribute", key="description", value=obj["description"])
        physical = {
            "movable": True, "pickupable": True,
            "block_solid": False, "block_projectile": False,
            "has_height": bool(flags.get("flags.height", False)),
        }
        record = native.profile(item_id, node, obj, physical, {}, None)
        # Match the existing House/source normalization of official draw-order flags.
        order = next((value for flag, value in [("clip", 1), ("top", 3), ("bottom", 2)]
                      if flags.get("flags." + flag, False)), 0)
        record["attributes"]["always_on_top"] = order != 0
        record["attributes"]["always_on_top_order"] = order
        record["authoring"]["item"]["revision"] = "qready1-cradle-r1"
        record["production_definition"] = {
            "family": "Item", "production_key": f"oteryn:item.tibia.i{item_id}",
            "revision_ref": "definition-r1",
        }
        record["production_binding"] = {
            "source_key": "oteryn:source.tibiawiki",
            "source_revision": f"fandom-r{fact['revision_id']}",
            "identity_namespace": "tibiawiki/item_id",
            "external_id": str(item_id),
            "disposition": "EXACT",
            "target": {
                "family": "Item", "key": f"oteryn:item.tibia.i{item_id}",
                "revision": "definition-r1",
            },
        }
        record["production_binding_qualification"] = None
        records.append(record)
        evidence.append({"appearance_id": item_id, "wiki_item_id": item_id, "official_name": obj["name"], "wiki_fact": fact})
    return {
        "schema": "OTERYN_QREADY1_CRADLE_ITEM_PROFILES/v1",
        "appearance_sha256": sources.APPEARANCES_SHA256,
        "official_membership_sha256": hashlib.sha256(manifest).hexdigest(),
        "wiki_stats_sha256": hashlib.sha256(raw_stats).hexdigest(),
        "identity_decision": admitted["decision"],
        "binding_basis": "EXPLICIT_WIKI_ITEM_ID_CORROBORATED_BY_OFFICIAL_NAMED_APPEARANCE",
        "recipe_scope": "CHOSEN_OTERYN_STABLE_PREPARED_INGREDIENTS",
        "evidence": evidence,
        "records": records,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = (json.dumps(build(args.root), indent=2, ensure_ascii=False) + "\n").encode()
    if args.check:
        if OUTPUT.read_bytes() != raw:
            raise SystemExit("QREADY1_CRADLE_ITEM_SOURCE_DRIFT")
    else:
        OUTPUT.write_bytes(raw)
