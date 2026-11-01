"""Mint the client-only Crystal market Items: identity plus what the pinned client proves.

E06 (owner decision 2026-10-31): the 68 Crystal market ids (`flags.market`,
zimbadev/crystalserver@201189f) that had no Oteryn Item definition get an Item record. 67 are
approved PENDING_MINT rows of the ITEM-ID-1b family decision register; 47899 is cited by the
monster optional-field decisions (an unbound falcon knight soul core). No donor items.xml
defines any of them.

Facts come only from the pinned 15.30 client appearance: the name, `flags.take` (movable and
pickupable) and `flags.cumulative` (stackability; a missing proto2 boolean is its getter
default, as in the reward Item admission; the engine stack ceiling is 100). The client carries
no container capacity, weight or other gameplay fact, so they stay UNKNOWN, and the records are
not materializable (so their runtime stack class stays Unknown). Marketability is promoted by the Crystal market parity packet, not here.
"""

import argparse
import json
from pathlib import Path

from engine_items import decode_appearance_object, protobuf_fields
from lower_wiki_stack_default_packet import checked, sha

ROOT = Path(__file__).resolve().parents[3]
COMPILER = "tools/content-schema/item-authoring/lower_client_market_mint_packet.py"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261031-item-client-market-mint-v1.json"
CLIENT_SHA = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
CLIENT = f"content/assets/files/appearances-{CLIENT_SHA}.dat"
REGISTER = "docs/agents/evidence/OTV2-20260929-item-id-1b-q13d-family-decisions.json"
CITATION = "docs/agents/evidence/monster-optional-field-fill-20261002/decisions-162.json"
PARITY = "docs/agents/evidence/OTV2-20261031-item-market-crystal-parity-v1.json"
IDS = (
    42085, 42175, 43892, 43893, 43948, 43949, 43950, 44157, 44179, 44755,
    47899, 48115, 48179, 48197, 48480, 48481, 48482, 48483, 48582, 48586,
    49160, 49176, 49272, 49273, 50051, 50053, 50062, 50063, 50312, 50335,
    50336, 50337, 50338, 51314, 51315, 51316, 51317, 51318, 51319, 51751,
    51752, 51753, 51754, 51755, 53142, 53143, 53144, 53145, 53146, 53147,
    53148, 53149, 53150, 53151, 53152, 53153, 53154, 53155, 53633, 53634,
    53635, 53636, 53637, 54510, 54511, 54512, 54513, 54514,
)  # fmt: skip
CITED_IDS = frozenset({47899})
GROUPS = (
    "charges", "classification", "container", "equipment", "fluid", "imbuement", "physical",
    "presentation", "protection", "readable_writeable", "skill_modifiers", "stack", "temporal",
    "trade_restrictions", "use_transform", "weapon",
)  # fmt: skip
UNKNOWN = {"state": "UNKNOWN"}


def known(value):
    return {"state": "KNOWN", "value": value}


def item_key(item_id):
    return f"oteryn:item.tibia.i{item_id}"


def register_rows(root):
    rows = {}
    for row in json.loads((root / REGISTER).read_text())["rows"]:
        if row["id"] in rows:
            raise ValueError(f"duplicate register row {row['id']}")
        rows[row["id"]] = row
    return rows


def cited_soul_cores(root):
    data = json.loads((root / CITATION).read_text())
    return {
        row["source"]["source_item_id"]: row
        for row in data["unbound_conditional_soul_cores"]
    }


def definition(item_id, obj):
    flags = obj["flags"]
    if flags.get("flags.take") is not True or flags.get("flags.unmove", False):
        raise ValueError(f"{item_id}: not a portable client object")
    stackable = flags.get("flags.cumulative", False)
    semantics = {group: dict(UNKNOWN) for group in GROUPS}
    semantics["presentation"] = known({"description": UNKNOWN, "name": known(obj["name"])})
    semantics["physical"] = known(
        {"movable": known(True), "pickupable": known(True), "weight": UNKNOWN}
    )
    semantics["stack"] = known(
        {
            "stackable": known(stackable),
            "stack_max": known(100) if stackable else UNKNOWN,
        }
    )
    return {
        "client_projection": "ClientSafe",
        "identity": {"family": "Item", "key": item_key(item_id), "revision": "definition-r1"},
        "kind": "Item",
        "materializable": False,
        "semantics": semantics,
        # Runtime stack admission belongs to materializable Items, as for the other 33,817.
        "stack_class": "Unknown",
    }


def build(root=ROOT):
    objects = {}
    for tag, raw in protobuf_fields(checked(root, CLIENT, CLIENT_SHA)):
        if tag == 1:
            obj = decode_appearance_object(raw)
            if obj["id"] in objects:
                raise ValueError("duplicate client appearance identity")
            objects[obj["id"]] = (obj, sha(raw))
    crystal = set(json.loads((root / PARITY).read_text())["crystal_market_ids"])
    register = register_rows(root)
    cited = cited_soul_cores(root)
    if list(IDS) != sorted(set(IDS)) or not set(IDS) <= crystal:
        raise ValueError("mint ids are not sorted Crystal market ids")
    records = []
    for item_id in IDS:
        obj, object_sha = objects[item_id]
        if obj["flags"].get("flags.market") is not True or not obj.get("name"):
            raise ValueError(f"{item_id}: client object has no market flag or name")
        if item_id in CITED_IDS:
            row = cited[item_id]["source"]
            if row["source_item_name"] != obj["name"] or item_id in register:
                raise ValueError(f"{item_id}: cited name or register drift")
            basis = {"path": CITATION, "reason": cited[item_id]["reason"]}
        else:
            row = register[item_id]
            if (
                row["applied"] != "PENDING_MINT"
                or not row["status"].startswith("APPROVED_")
                or row["family"] != "Item"
                or row["key"] != item_key(item_id)
                or row["name"] != obj["name"]
            ):
                raise ValueError(f"{item_id}: register row is not an approved pending mint")
            basis = {"path": REGISTER, "profile": row["profile"], "status": row["status"]}
        records.append(
            {
                "source_item_id": item_id,
                "object_sha256": object_sha,
                "basis": basis,
                "definition": definition(item_id, obj),
            }
        )
    return {
        "schema": "OTERYN_ITEM_CLIENT_MARKET_MINT/v1",
        "compiler": {"path": COMPILER, "sha256": sha((root / COMPILER).read_bytes())},
        "sources": {
            "client_path": CLIENT,
            "client_sha256": CLIENT_SHA,
            "register_path": REGISTER,
            "register_sha256": sha((root / REGISTER).read_bytes()),
            "citation_path": CITATION,
            "citation_sha256": sha((root / CITATION).read_bytes()),
        },
        "decision": "E06 owner decision 2026-10-31",
        "counts": {"records": len(records)},
        "records": records,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    packet = build()
    data = (
        json.dumps(packet, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("client market mint packet drift")
    else:
        OUTPUT.write_bytes(data)
    print(json.dumps(packet["counts"]), sha(data))


if __name__ == "__main__":
    main()
