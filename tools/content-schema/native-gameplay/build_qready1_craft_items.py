"""Normalize source-qualified Cursed Crystal World-tool Items, without guessed flags."""

import argparse
import hashlib
import json
import sys
from pathlib import Path

import build_source_item_profiles as native
import build_world_change_item_profiles as sources

from build_spell_item_identities import historical_binding_bytes, PRE_CAKE_BINDINGS_SHA

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-migration"))
import grizzly_shop_profiles as accepted

IDS = frozenset({8997, 9108, 9109, 21506, 21507, 21573})
OUTPUT = Path(__file__).with_name("qready1-craft-item-profiles.json")


def build(root=ROOT):
    xml_raw = sources.read_xml(root)
    appearance_raw = (root / sources.APPEARANCES).read_bytes()
    sources.verify_sources(xml_raw, appearance_raw)
    xml = native.xml_index(xml_raw)
    objects = native.engine_items.load_appearance_objects(appearance_raw)
    physical = {
        r["server_item_id"]: r for r in native.policies(appearance_raw, xml_raw, IDS)
    }
    binding_raw = (root / native.BINDINGS.relative_to(ROOT)).read_bytes()
    bindings = native.binding_index(binding_raw)
    records = [
        native.profile(i, xml[i], objects[i], physical[i], bindings, None)
        for i in sorted(IDS)
    ]
    # Preserve accepted canonical facts shared with other native item cohorts.
    # The older donor XML differs in presentation punctuation, not custody.
    keys = {f"oteryn:item.tibia.i{i}" for i in IDS}
    definitions = {
        row["definition"]["identity"]["key"]: row["definition"]
        for path in sorted((root / "content/items/definitions").glob("items-*.json"))
        for row in json.loads(path.read_bytes())["records"]
        if row["definition"]["identity"]["key"] in keys
    }
    if definitions.keys() != keys:
        raise ValueError("QREADY1_CRAFT_ITEM_CANONICAL_SET")
    for item, record in zip(sorted(IDS), records, strict=True):
        key = f"oteryn:item.tibia.i{item}"
        semantics = definitions[key].get("semantics", {})
        for group in ("physical", "stack", "container", "temporal"):
            if accepted.known_conflict(semantics.get(group, native.U), record["semantics"][group]):
                raise ValueError("QREADY1_CRAFT_ITEM_CANONICAL_CONFLICT:" + key)
        record["semantics"] = accepted.accepted_first(semantics, record["semantics"])
    document = native.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": records}, bindings, IDS
    )
    return {
        "schema": "OTERYN_QREADY1_CRAFT_ITEM_PROFILES/v1",
        "xml_sha256": sources.XML_SHA256,
        "appearance_sha256": sources.APPEARANCES_SHA256,
        "binding_sha256": hashlib.sha256(historical_binding_bytes(binding_raw, PRE_CAKE_BINDINGS_SHA)).hexdigest(),
        "records": document["records"],
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = (json.dumps(build(), indent=2, ensure_ascii=False) + "\n").encode()
    if args.check:
        if OUTPUT.read_bytes() != raw:
            raise SystemExit("QREADY1_CRAFT_ITEM_SOURCE_DRIFT")
    else:
        OUTPUT.write_bytes(raw)
