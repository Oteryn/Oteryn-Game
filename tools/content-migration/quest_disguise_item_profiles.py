"""Qualify Crystal's mummy disguise for the existing temporary outfit caller.

This bounded overlay retains the accepted Item identity and uses the existing
source Item importer. It grants no NPC reward, appearance entitlement or new
Item owner. Unsupported Item systems remain UNKNOWN.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/content-schema/native-gameplay"))
import build_source_item_profiles as source

PATH = "tools/content-schema/native-gameplay/quest-disguise-item-profiles.json"
PRODUCER = "tools/content-migration/quest_disguise_item_profiles.py"
SCHEMA = "OTERYN_NATIVE_QUEST_DISGUISE_ITEM_PROFILES/v1"
CRYSTAL_PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
CRYSTAL_INPUTS = {
    "data/items/items.xml": "8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509",
    "data/items/appearances.dat": "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1",
    "src/items/items.hpp": "06d52ee83250ba852fe804c6b006b0aac150ceb26893e9079fa8c93502d209ca",
    "src/items/items.cpp": "f39d8fc6c1c81b8ec6f749e1f152dd945472ffd5a36b9cf3cd9a3a64b2f19b1d",
    "src/items/functions/item/item_parse.cpp": "08687bc0e096ebfcce55daa6a523b78bca0191f25ce4134c133927d4a10dfce7",
    "data-global/scripts/quests/what_a_foolish_quest/action_disguise.lua": "98a44030a86eff80f16cd24b04cbcaa7ae6e3f4b3236b1f0245be38f5ffd3427",
}
ITEM_ID = 144
ITEM_KEY = "oteryn:item.tibia.i144"
BINDINGS_PATH = "imports/crystalserver/bindings/items.json"
RUNTIME_OWNER = (
    "crates/gameplay/dispatch/src/gameplay_transport/item_use/quest_appearance.rs"
)
DONOR_RULE = (
    f"zimbadev/crystalserver@{CRYSTAL_PIN}:"
    "data-global/scripts/quests/what_a_foolish_quest/action_disguise.lua:1-12"
)
# These source systems have no qualified interpretation in this overlay.
UNKNOWN_SECTIONS = (
    "weapon",
    "protection",
    "imbuement",
    "use_transform",
    "trade_restrictions",
    "fluid",
    "readable_writeable",
)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def encoded(value):
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode()


def provenance():
    return [
        {"path": path, "revision": CRYSTAL_PIN, "sha256": sha}
        for path, sha in CRYSTAL_INPUTS.items()
    ]


def canonical_identity(root):
    """Discover the existing declaration without rewriting its identity or facts."""
    found = []
    for path in sorted((root / "content/items/definitions").glob("items-*.json")):
        raw = path.read_bytes()
        for record in json.loads(raw)["records"]:
            definition = record["definition"]
            if definition["identity"]["key"] == ITEM_KEY:
                found.append(
                    (
                        definition,
                        {
                            "path": path.relative_to(root).as_posix(),
                            "sha256": digest(raw),
                        },
                    )
                )
    if len(found) != 1:
        raise ValueError("QUEST_DISGUISE_CANONICAL_IDENTITY")
    return found[0]


def qualified_record(source_row, bindings, definition):
    """Replay the standard importer; never accept a hand-edited profile body."""
    if set(source_row) != {"server_item_id", "xml", "appearance", "physical"}:
        raise ValueError("QUEST_DISGUISE_SOURCE_ROW")
    node = ET.fromstring(source_row["xml"])
    if (
        source_row["server_item_id"] != ITEM_ID
        or node.attrib != {"id": "144", "article": "a", "name": "mummy disguise"}
        or source.attrs(node) != {"primarytype": "quest items", "weight": "900"}
        or len(node) != 2
        or source_row["physical"]["server_item_id"] != ITEM_ID
    ):
        raise ValueError("QUEST_DISGUISE_SOURCE_ROW")
    binding = bindings[str(ITEM_ID)]
    if definition["identity"] != binding["target"]:
        raise ValueError("QUEST_DISGUISE_CANONICAL_IDENTITY")
    profile = source.profile(
        ITEM_ID,
        node,
        source_row["appearance"],
        source_row["physical"],
        bindings,
        None,
    )
    semantics = profile["semantics"]
    if (
        semantics["physical"] != definition["semantics"]["physical"]
        or semantics["presentation"]["value"]["name"]
        != definition["semantics"]["presentation"]["value"]["name"]
        or definition["semantics"]["stack"]["value"]["stackable"] != source.K(False)
        or semantics["stack"]
        != source.K(
            {
                "stackable": source.K(False),
                "stack_max": dict(source.NA),
            }
        )
        or semantics["container"] != source.NA
        or any(semantics[name] != source.U for name in UNKNOWN_SECTIONS)
        or profile["admission"]
        != {
            "materializable": True,
            "stack_class": "NonStackable",
            "legal_destinations": ["CharacterInventory", "Ground"],
        }
    ):
        raise ValueError("QUEST_DISGUISE_PROFILE_QUALIFICATION")
    # The canonical identity keeps its previously accepted source_revision.
    # New physical/action evidence is the independent Crystal pin above.
    return source.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [profile]},
        bindings,
        {ITEM_ID},
    )["records"][0]


def capture(crystal_root, root=ROOT):
    raw = {
        path: subprocess.check_output(
            [
                "git",
                "-C",
                str(crystal_root),
                "show",
                f"{CRYSTAL_PIN}:{path}",
            ]
        )
        for path in CRYSTAL_INPUTS
    }
    if any(digest(data) != CRYSTAL_INPUTS[path] for path, data in raw.items()):
        raise ValueError("QUEST_DISGUISE_CRYSTAL_SOURCE_PIN")
    hpp = raw["src/items/items.hpp"].decode()
    cpp = raw["src/items/items.cpp"].decode()
    if (
        any(
            text not in hpp
            for text in (
                "int32_t weight = 0",
                "uint8_t stackSize = 100",
                "uint16_t maxItems = 8",
                "uint32_t decayTime = 0",
                "int32_t speed = 0",
            )
        )
        or "iType.stackable = object.flags().cumulative()" not in cpp
    ):
        raise ValueError("QUEST_DISGUISE_LOADER_DEFAULTS")
    xml = raw["data/items/items.xml"]
    appearances = raw["data/items/appearances.dat"]
    nodes = source.xml_index(xml)
    objects = source.engine_items.load_appearance_objects(appearances)
    physical = {
        row["server_item_id"]: row
        for row in source.policies(
            appearances,
            xml,
            {ITEM_ID},
        )
    }
    if ITEM_ID not in nodes or ITEM_ID not in objects or ITEM_ID not in physical:
        raise ValueError("QUEST_DISGUISE_SOURCE_ROW_MISSING")
    definition, canonical_input = canonical_identity(root)
    bindings = source.binding_index((root / BINDINGS_PATH).read_bytes())
    source_row = {
        "server_item_id": ITEM_ID,
        "xml": ET.tostring(nodes[ITEM_ID], encoding="unicode"),
        "appearance": objects[ITEM_ID],
        "physical": physical[ITEM_ID],
    }
    record = qualified_record(source_row, bindings, definition)
    # Invoke the existing exact independent membership check as well: it
    # qualifies 201189f facts without relabelling the historical identity.
    source.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [record]},
        bindings,
        {ITEM_ID},
        independent_crystal_source=(CRYSTAL_PIN, xml),
    )
    return {
        "schema": SCHEMA,
        "producer": PRODUCER,
        "source_revision": CRYSTAL_PIN,
        "runtime_owner": RUNTIME_OWNER,
        "donor_rule": DONOR_RULE,
        "canonical_inputs": [canonical_input],
        "sources": provenance(),
        "source_row": source_row,
        "records": [record],
    }


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    if (
        overlay.get("schema") != SCHEMA
        or len(overlay.get("records", [])) != 1
        or overlay["records"][0].get("production_definition", {}).get("production_key")
        != ITEM_KEY
    ):
        raise ValueError("QUEST_DISGUISE_PROFILE_SET")
    if (
        overlay.get("producer") != PRODUCER
        or overlay.get("source_revision") != CRYSTAL_PIN
        or overlay.get("sources") != provenance()
        or overlay.get("runtime_owner") != RUNTIME_OWNER
        or overlay.get("donor_rule") != DONOR_RULE
    ):
        raise ValueError("QUEST_DISGUISE_SOURCE_PROVENANCE")
    definition, canonical_input = canonical_identity(root)
    if overlay.get("canonical_inputs") != [canonical_input]:
        raise ValueError("QUEST_DISGUISE_CANONICAL_DRIFT")
    bindings = source.binding_index((root / BINDINGS_PATH).read_bytes())
    expected = qualified_record(overlay["source_row"], bindings, definition)
    if overlay["records"] != [expected]:
        raise ValueError("QUEST_DISGUISE_PROFILE_QUALIFICATION")
    document = json.loads(generated[profile_path])
    if any(
        row["production_definition"]["production_key"] == ITEM_KEY
        for row in document["records"]
    ):
        raise ValueError("QUEST_DISGUISE_PROFILE_DUPLICATE")
    document["records"] = sorted(document["records"] + [expected], key=order)
    data = encoded(document)
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = digest(data)
    manifest_data = canonical(manifest)
    # Both output bytes are ready before mutating the caller's planned files.
    generated[profile_path] = data
    generated[manifest_path] = manifest_data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crystal-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=ROOT / PATH)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = encoded(capture(args.crystal_root))
    if args.check:
        if not args.output.exists() or args.output.read_bytes() != data:
            raise SystemExit("QUEST_DISGUISE_PROFILE_DRIFT")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(data)


if __name__ == "__main__":
    main()
