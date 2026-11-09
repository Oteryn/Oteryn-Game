"""Normalize source-qualified Cursed Crystal World-tool Items, without guessed flags."""

import argparse
import hashlib
import json
from pathlib import Path

import build_source_item_profiles as native
import build_world_change_item_profiles as sources

ROOT = Path(__file__).resolve().parents[3]
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
    document = native.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": records}, bindings, IDS
    )
    return {
        "schema": "OTERYN_QREADY1_CRAFT_ITEM_PROFILES/v1",
        "xml_sha256": sources.XML_SHA256,
        "appearance_sha256": sources.APPEARANCES_SHA256,
        "binding_sha256": hashlib.sha256(binding_raw).hexdigest(),
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
