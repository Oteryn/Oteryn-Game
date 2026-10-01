"""Lower explicit agreed wiki non-stackability without admitting Item identity classes."""

import argparse
import hashlib
import json
from pathlib import Path

from engine_items import load_appearance_objects

ROOT = Path(__file__).resolve().parents[3]
COMPILER = "tools/content-schema/item-authoring/lower_wiki_stack_false_packet.py"
SNAPSHOT = "imports/tibiawiki/facts/items-stats.json"
CLIENT_SHA = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
CLIENT = f"content/assets/files/appearances-{CLIENT_SHA}.dat"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261001-item-stack-false-promotion-v1.json"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def qualify(snapshot, definitions, bound, routed, appearances):
    rows, holds = [], []
    for record in sorted(snapshot["records"].values(), key=lambda r: r["item_id"]):
        iid = record["item_id"]
        key = f"oteryn:item.tibia.i{iid}"
        definition = definitions.get(key, {})
        observations = [o for o in record["observations"] if "stackable" in o["fields"]]
        values = [o["fields"]["stackable"].casefold() for o in observations]
        if "no" not in values or "semantics" not in definition:
            continue
        stack = definition["semantics"]["stack"]
        old = stack.get("value", {}).get("stackable", {})
        reason = None
        if any(v not in {"yes", "no"} for v in values):
            reason = "MALFORMED_WIKI_VALUE"
        elif len(set(values)) != 1:
            reason = "WIKI_PAGE_DISAGREEMENT"
        elif key not in bound:
            reason = "NO_EXACT_SOURCE_BINDING"
        elif key in routed:
            reason = "EXISTING_MAP_OWNER"
        elif appearances.get(iid, {}).get("flags", {}).get("flags.cumulative") is True:
            reason = "CLIENT_CUMULATIVE_TRUE"
        elif stack["state"] in {"CONFLICT", "NOT_APPLICABLE"} or old.get("state") in {
            "CONFLICT",
            "NOT_APPLICABLE",
        }:
            reason = "BLOCKED_EVIDENCE_STATE"
        elif (
            definition["stack_class"] not in {"Unknown", "NonStackable"}
            or old.get("value") is True
        ):
            reason = "KNOWN_STACK_CONFLICT"
        sources = [
            {
                field: o[field]
                for field in ("page_id", "revision_id", "content_sha256", "url")
            }
            | {"stackable": o["fields"]["stackable"]}
            for o in observations
        ]
        row = {"item_key": key, "sources": sources}
        if reason:
            holds.append(row | {"reason": reason})
        else:
            rows.append(row | {"stackable": False})
    return rows, holds


def build(root=ROOT):
    snapshot = json.loads((root / SNAPSHOT).read_text())
    digest = sha(
        json.dumps(
            snapshot["records"],
            sort_keys=True,
            ensure_ascii=False,
            separators=(",", ":"),
        ).encode()
    )
    if digest != snapshot["snapshot_sha256"]:
        raise ValueError("wiki snapshot digest drift")
    data = (root / CLIENT).read_bytes()
    if sha(data) != CLIENT_SHA:
        raise ValueError("client appearance digest drift")
    binding_path = "imports/crystalserver/bindings/items.json"
    bound = {
        r["target"]["key"]
        for r in json.loads((root / binding_path).read_text())["bindings"]
        if r["disposition"] == "EXACT"
    }
    routed, map_sources = set(), {}
    for family in ("terrain", "objects"):
        for path in sorted((root / f"content/world/{family}").glob("*.json")):
            map_sources[str(path.relative_to(root))] = sha(path.read_bytes())
            for row in json.loads(path.read_text()).get("records", []):
                pointer = row.get("provenance", {}).get("item_pointer")
                if pointer:
                    routed.add(pointer["key"])
    definitions = {}
    for shard in json.loads((root / "content/items/index.json").read_text())["shards"]:
        for row in json.loads((root / shard).read_text())["records"]:
            definition = row["definition"]
            definitions[definition["identity"]["key"]] = definition
    rows, holds = qualify(
        snapshot, definitions, bound, routed, load_appearance_objects(data)
    )
    if len(rows) != 2381:
        raise ValueError("bounded stack-false scope drift")
    return {
        "schema": "OTERYN_ITEM_STACK_FALSE_PROMOTION/v1",
        "compiler": {
            "path": COMPILER,
            "sha256": sha((root / COMPILER).read_bytes()),
            "client_decoder_sha256": sha(
                (
                    root / "tools/content-schema/item-authoring/engine_items.py"
                ).read_bytes()
            ),
        },
        "sources": {
            "wiki_path": SNAPSHOT,
            "snapshot_sha256": digest,
            "client_path": CLIENT,
            "client_sha256": CLIENT_SHA,
            "bindings_path": binding_path,
            "bindings_sha256": sha((root / binding_path).read_bytes()),
            "map_owner_inputs": map_sources,
        },
        "policy": "EXPLICIT_NO_ALL_PRESENT_WIKI_PAGES_AGREE_NO_IDENTITY_CLASS_ADMISSION",
        "baseline_repair": {
            "head": "340a6be43b1beead30c145a4223a146a1024cca9",
            "unknown_to_known_false": 2345,
            "already_known_false": 35,
        },
        "physical_followup": {
            "parent": "5cb90c84013faa4c07fc35e3f660e67db6d691d2",
            "new_explicit_false_ids": [20129],
        },
        "counts": {"promotions": len(rows), "holds": len(holds)},
        "promotions": rows,
        "holds": holds,
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
            raise SystemExit("stack-false packet drift")
    else:
        OUTPUT.write_bytes(data)
    print(json.dumps(packet["counts"]))


if __name__ == "__main__":
    main()
