"""Qualify the fortress's covered hole as ground from its pinned Item and appearance."""

import argparse
import hashlib
import json
import sys
from functools import lru_cache
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-migration"))
import source_gate_item_profiles as source_gate

IDENTIFIER = 593
PROFILE = "OTERYN_EROTH_FORTRESS_GROUND/v1"
KEY = "oteryn:terrain.tibia.i593"
SHARD = "content/world/terrain/terrain-08751-08751.json"


@lru_cache(maxsize=1)
def record():
    xml = (ROOT / source_gate.XML).read_bytes()
    client = (ROOT / source_gate.CLIENT).read_bytes()
    if (
        hashlib.sha256(xml).hexdigest() != source_gate.XML_SHA
        or hashlib.sha256(client).hexdigest() != source_gate.CLIENT_SHA
    ):
        raise ValueError("Eroth fortress ground source digest differs")
    source = source_gate.source
    entries = source.xml_index(xml)
    if (
        source.attrs(entries[IDENTIFIER]).get("floorchange") is not None
        or source.attrs(entries[594]).get("floorchange") != "down"
        or source.attrs(entries[594]).get("decayto") != str(IDENTIFIER)
    ):
        raise ValueError("Eroth fortress hole source transition differs")
    (facts,) = source.policies(client, xml, {IDENTIFIER})
    if (
        not facts["ground"]
        or facts["block_solid"]
        or facts["movable"]
        or facts["pickupable"]
        or facts["ground_speed"] != 160
    ):
        raise ValueError("Eroth fortress ground appearance differs")
    pointer = source.binding_index((ROOT / source_gate.BINDINGS).read_bytes())[
        str(IDENTIFIER)
    ]["target"]
    if pointer != {
        "family": "Item",
        "key": "oteryn:item.tibia.i593",
        "revision": "definition-r1",
    }:
        raise ValueError("Eroth fortress ground Item binding differs")
    known = lambda value: {"state": "KNOWN", "value": value}
    return {
        "identity": {
            "family": "Terrain",
            "key": KEY,
            "revision": "definition-r1",
        },
        "kind": known("ground"),
        "walkable": known(True),
        "ground_speed": known(facts["ground_speed"]),
        "blocks_projectile": known(facts["block_projectile"]),
        "blocks_sight": {"state": "UNKNOWN"},
        "floor_change": {"state": "UNKNOWN"},
        "automap": {"state": "UNKNOWN"},
        "client_projection": known({"appearance_id": IDENTIFIER}),
        "provenance": {
            "source": {
                "engine": "canary",
                "repository": "opentibiabr/canary",
                "revision": "04b83b512114bfd888000d6e1433ed8ecaec7c5b",
                "profile": PROFILE,
                "items_xml_sha256": source_gate.XML_SHA,
                "appearances_sha256": source_gate.CLIENT_SHA,
            },
            "source_item_id": IDENTIFIER,
            "item_pointer": pointer,
            "route": {"owner": "Terrain", "reason": "ground_or_border"},
            "fields": {
                "kind": ["appearance:flags.bank"],
                "walkable": ["appearance:flags.unpass"],
                "ground_speed": ["appearance:bank.waypoints"],
                "blocks_projectile": ["appearance:flags.unsight"],
                "client_projection": ["appearance:id"],
            },
        },
    }


def output():
    document = {"family": "Terrain", "records": [record()]}
    return (json.dumps(document, sort_keys=True, separators=(",", ":")) + "\n").encode()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    shard = ROOT / SHARD
    if args.check:
        if shard.read_bytes() != output():
            raise SystemExit("Eroth fortress ground catalogue differs")
    else:
        shard.write_bytes(output())
