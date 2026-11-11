"""M30a: qualify the one missing training weapon from pinned Canary Item facts."""

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FACTS = "tools/content-schema/native-gameplay/engenc-training-appearance-facts.json"
REVISION = "47dfd51f45280a59a1d3e50ba7edd573d7234446"
APPEARANCES = "aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50"
XML = "imports/canary/items-xml/items.xml"
XML_SHA256 = "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2"
KEY = "oteryn:item.tibia.i17110"


def importer(root):
    sys.path.insert(0, str(root / "tools/content-schema/native-gameplay"))
    import build_source_item_profiles

    return build_source_item_profiles


def capture(raw, root=ROOT):
    if hashlib.sha256(raw).hexdigest() != APPEARANCES:
        raise ValueError("ENGENC_APPEARANCE_SOURCE")
    xml = (root / XML).read_bytes()
    if hashlib.sha256(xml).hexdigest() != XML_SHA256:
        raise ValueError("ENGENC_XML_SOURCE")
    native = importer(root)
    obj = native.engine_items.load_appearance_objects(raw)[17110]
    physical = native.policies(raw, xml, {17110})[0]
    return {
        "schema": "OTERYN_ENGENC_TRAINING_APPEARANCE_FACTS/v1",
        "source_revision": REVISION,
        "source_path": "data/items/appearances.dat",
        "source_sha256": APPEARANCES,
        "xml_sha256": XML_SHA256,
        "item_id": 17110,
        "flags": obj["flags"],
        "physical": {
            key: physical[key]
            for key in (
                "movable",
                "pickupable",
                "block_solid",
                "block_projectile",
                "has_height",
            )
        },
    }


def validate(facts, xml):
    if (
        facts["schema"] != "OTERYN_ENGENC_TRAINING_APPEARANCE_FACTS/v1"
        or facts["source_revision"] != REVISION
        or facts["source_path"] != "data/items/appearances.dat"
        or facts["source_sha256"] != APPEARANCES
        or facts["xml_sha256"] != XML_SHA256
        or hashlib.sha256(xml).hexdigest() != XML_SHA256
        or facts["item_id"] != 17110
        or facts["flags"].get("flags.cumulative") is not True
        or facts["flags"].get("flags.take") is not True
        or facts["flags"].get("clothes.slot") != 6
        or facts["physical"]
        != {
            "movable": True,
            "pickupable": True,
            "block_solid": False,
            "block_projectile": False,
            "has_height": False,
        }
    ):
        raise ValueError("ENGENC_TRAINING_SOURCE_FACTS")


def admit(root, generated, profile_path, manifest_path, order, canonical):
    native = importer(root)
    facts = json.loads((root / FACTS).read_bytes())
    xml = (root / XML).read_bytes()
    validate(facts, xml)
    bindings = native.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    record = native.profile(
        17110,
        native.xml_index(xml)[17110],
        {"flags": facts["flags"]},
        facts["physical"],
        bindings,
        None,
    )
    record = native.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [record]},
        bindings,
        {17110},
    )["records"][0]
    if (
        record["production_definition"]["production_key"] != KEY
        or record["production_binding"]["disposition"] != "EXACT"
        or record["admission"]
        != {
            "materializable": True,
            "stack_class": "StackCapable",
            "legal_destinations": ["CharacterInventory", "Ground"],
        }
    ):
        raise ValueError("ENGENC_TRAINING_IDENTITY")
    from grizzly_shop_profiles import accepted_first, known_conflict

    definitions = [
        row["definition"]
        for path in (root / "content/items/definitions").glob("items-*.json")
        for row in json.loads(path.read_bytes())["records"]
        if row["definition"]["identity"]["key"] == KEY
    ]
    if len(definitions) != 1 or any(
        known_conflict(definitions[0]["semantics"][field], record["semantics"][field])
        for field in ("physical", "stack", "container")
    ):
        raise ValueError("ENGENC_TRAINING_CANONICAL_CONFLICT")
    record["semantics"] = accepted_first(
        definitions[0]["semantics"], record["semantics"]
    )
    document = json.loads(generated[profile_path])
    if any(
        row["production_definition"]["production_key"] == KEY
        for row in document["records"]
    ):
        raise ValueError("ENGENC_TRAINING_DUPLICATE")
    document["records"] = sorted(document["records"] + [record], key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture", required=True, type=Path)
    args = parser.parse_args()
    (ROOT / FACTS).write_text(
        json.dumps(capture(args.capture.read_bytes()), indent=2) + "\n"
    )
