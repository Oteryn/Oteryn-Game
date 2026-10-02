"""Explicit local candidate identity from pinned loader/factory closure.

Only the source-created Divine Empowerment marker is qualified. This is not
numeric-ID promotion, XML membership inference or production activation.
"""
from __future__ import annotations
import hashlib
import json
import subprocess
from pathlib import Path

PIN = "99902524e052f37574194466c2949c576e4ab269"
ROOT = Path(__file__).resolve().parents[3]
PACKET = ROOT / "imports/canary/bindings/spell-items-candidate.json"
SOURCE_HASHES = {
    "data/items/appearances.dat": "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1",
    "src/protobuf/appearances.proto": "f03674197041967fc095e48462edaa1ab27c481385aad3d85d27a7d25af325c4",
    "src/items/items.cpp": "f695ed647e111923cf8e4d487c4a0706f19e7edbfb7896655c20494d6d927fd7",
    "src/items/item.cpp": "9bbf46dbecd43748737a2636eb2200f39f5a35d7fa524a800adabaa10473b3bb",
    "src/items/items.hpp": "b8c12f45122eeb575ad548719c5fb178a96460d8a41ad882d0d88b2e66d797e6",
    "data/items/items.xml": "bf773d2c8f1d2767b31a20749fd5278ae48f2ed5b05757a678e3788e2a70bf85",
    "data/scripts/spells/support/divine_empowerment.lua": "105872cef06f9b9dda720a9eff1af24884c3e0cc6979be6f7a7e21891bf4830b",
}


def _varint(data, offset):
    value = shift = 0
    while offset < len(data) and shift <= 63:
        byte = data[offset]; offset += 1
        value |= (byte & 127) << shift
        if byte < 128:
            return value, offset
        shift += 7
    raise ValueError("Invalid protobuf varint")


def _fields(data):
    offset, result = 0, {}
    while offset < len(data):
        tag, offset = _varint(data, offset)
        number, wire = tag >> 3, tag & 7
        if number == 0:
            raise ValueError("Invalid protobuf field")
        if wire == 0:
            value, offset = _varint(data, offset)
        elif wire in (1, 2, 5):
            if wire == 2:
                size, offset = _varint(data, offset)
            else:
                size = {1: 8, 5: 4}[wire]
            value = data[offset:offset+size]; offset += size
            if offset > len(data):
                raise ValueError("Truncated protobuf field")
        else:
            raise ValueError("Unsupported protobuf wire")
        result.setdefault(number, []).append(value)
    return result


def prove(root: Path) -> dict:
    if subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip() != PIN:
        raise ValueError("Canary source HEAD mismatch")
    data = {}
    for path, expected in SOURCE_HASHES.items():
        blob = subprocess.check_output(["git", "show", f"{PIN}:{path}"], cwd=root)
        if hashlib.sha256(blob).hexdigest() != expected:
            raise ValueError(f"Canary source digest mismatch: {path}")
        data[path] = blob
    enum_path = "src/utils/utils_definitions.hpp"
    enum = subprocess.check_output(["git", "show", f"{PIN}:{enum_path}"], cwd=root)
    if "ITEM_DIVINE_EMPOWERMENT = 40450," not in enum.decode():
        raise ValueError("Source marker identity changed")
    records = [_fields(row) for row in _fields(data["data/items/appearances.dat"])[1]
               if _fields(row).get(1) == [40450]]
    if len(records) != 1 or len(records[0].get(3, [])) != 1:
        raise ValueError("Source marker appearance identity is ambiguous")
    flags = _fields(records[0][3][0])
    if flags != {14: [1], 23: [bytes.fromhex("080210cb01")], 33: [1]}:
        raise ValueError("Source marker flags changed")
    # Exact source hashes fix every factory branch/default and XML omission.
    # These statements make the exercised semantic boundary explicit.
    assertions = {
        "src/items/items.hpp": ["group = ITEM_GROUP_NONE;", "type = ITEM_TYPE_NONE;",
            "uint32_t decayTime = 0;", "uint32_t charges = 0;", "int32_t weight = 0;",
            "std::shared_ptr<ConditionDamage> conditionDamage;"],
        "src/items/items.cpp": ["iType.id = static_cast<uint16_t>(object.id());",
            "iType.movable = object.flags().unmove() == false;",
            "iType.stackable = object.flags().cumulative();", "iType.pickupable = object.flags().take();",
            "iType.blockSolid = object.flags().unpass();", "iType.blockProjectile = object.flags().unsight();"],
        "src/items/item.cpp": ["if (it.id != 0)", "newItem = std::make_shared<Item>(type, count);"],
        "data/scripts/spells/support/divine_empowerment.lua": ["Game.createItem(ITEM_DIVINE_EMPOWERMENT, 1, pos)",
            "addEvent(removeEmpowermentItem, 5000, position)", "item:setAttribute(ITEM_ATTRIBUTE_OWNER, creature:getId())"],
    }
    for path, snippets in assertions.items():
        if any(snippet not in data[path].decode() for snippet in snippets):
            raise ValueError(f"Source loader/factory closure changed: {path}")
    binding = {"source_key": "oteryn:source.canary", "source_revision": PIN,
        "identity_namespace": "ots/item_server_id", "external_id": "40450",
        "target": {"family": "Item", "key": "oteryn:item.tibia.i40450", "revision": "definition-r1"},
        "disposition": "EXACT"}
    return {"schema": "OTERYN_SOURCE_ITEM_IDENTITY_CANDIDATE/v1", "family": "Item", "bindings": [binding],
        "qualification": {"source_revision": PIN,
            "sources": [{"path": path, "sha256": digest} for path, digest in SOURCE_HASHES.items()]
                + [{"path": enum_path, "sha256": hashlib.sha256(enum).hexdigest()}],
            "method": "Pinned protobuf flags + exact Items loader + ItemType defaults + Item factory + explicit source producer",
            "items_xml_present": False, "appearance_flags": {"unmove": True, "light_brightness": 2,
                "light_color": 203, "ignore_look": True},
            "policy": {"materializable": True, "stack_class": "NonStackable", "legal_destinations": ["Ground"],
                "movable": False, "pickupable": False, "block_solid": False, "block_projectile": False,
                "immovable_block_solid": False, "weight": 0, "speed_bonus": 0, "charges": 0,
                "decay_seconds": 0, "container": False, "magic_field": False, "field_condition": None,
                "source_removal_delay_ms": 5000},
            "status": "LOCAL_CANDIDATE; new explicit identity, not an existing Crystal binding or production activation"}}


def candidate(root: Path) -> dict:
    actual = prove(root)
    if actual != json.loads(PACKET.read_bytes()):
        raise ValueError("Candidate Item identity differs from source qualification packet")
    return actual


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--canary", type=Path, default=Path("/workspace/spell-sources/canary"))
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        PACKET.write_text(json.dumps(prove(args.canary), indent=2)+"\n")
    else:
        candidate(args.canary)
