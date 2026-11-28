"""Admit the Fandom Golden Box identity using its matching official client object."""
import hashlib
import json
import xml.etree.ElementTree as ET

import qengine_reward_item_profiles as items

ITEM = 50063
KEY = f"oteryn:item.tibia.i{ITEM}"
CITATION = "https://tibia.fandom.com/?curid=107913"
REVISION = "fandom-page107913-20260727"
SHARD = "content/items/definitions/items-27500-27999.json"


def admit(root, generated, profile_path, manifest_path, order, canonical):
    source = items.source
    client = (root / items.CLIENT).read_bytes()
    xml = (root / items.XML).read_bytes()
    if hashlib.sha256(client).hexdigest() != items.CLIENT_SHA:
        raise ValueError("Golden Box client appearance source drift")
    if hashlib.sha256(xml).hexdigest() != items.XML_SHA:
        raise ValueError("Golden Box client policy source drift")
    obj = source.engine_items.load_appearance_objects(client)[ITEM]
    flags = obj["flags"]
    if (obj["name"].casefold() != "demon in a golden box"
            or not flags.get("flags.cumulative")
            or not flags.get("flags.take")
            or any(flags.get("flags." + flag) for flag in ("unmove", "container", "unpass"))):
        raise ValueError("Golden Box client identity or portable flags drift")
    definitions = json.loads((root / SHARD).read_bytes())["records"]
    definition = next(row["definition"] for row in definitions
                      if row["definition"]["identity"]["key"] == KEY)
    semantics = definition["semantics"]
    if semantics["physical"]["value"]["weight"] != source.K(200) or not (
        semantics["stack"]["value"]["stackable"] == source.K(True)
    ):
        raise ValueError("Golden Box Global physical facts drift")
    # Adapt the Global weight to the existing native Item normalizer.
    node = ET.Element("item", id=str(ITEM), name=obj["name"])
    ET.SubElement(node, "attribute", key="weight", value="200")
    physical, = source.policies(client, xml, {ITEM})
    record = source.profile(ITEM, node, obj, physical, {}, None)
    record["semantics"] = items.accepted.accepted_first(semantics, record["semantics"])
    record["production_definition"] = {
        "family": "Item", "production_key": KEY, "revision_ref": "definition-r1",
    }
    record["production_binding"] = {
        "source_key": "oteryn:source.tibiawiki",
        "source_revision": REVISION,
        "identity_namespace": "tibiawiki/item_id",
        "external_id": str(ITEM),
        "disposition": "EXACT",
        "target": definition["identity"],
    }
    record["production_binding_qualification"] = None
    generated["tools/content-schema/native-gameplay/s06-golden-box-profile.json"] = (
        json.dumps({"url": CITATION, "appearance_sha256": items.CLIENT_SHA,
                    "records": [record]}, indent=2) + "\n"
    ).encode()
    items.admit_records(root, generated, profile_path, manifest_path, order, canonical, [record])
