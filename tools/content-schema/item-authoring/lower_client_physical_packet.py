"""Qualify affirmative client pickup facts and the bounded official magic-rune limit."""

import argparse
import hashlib
import json
from collections import defaultdict
from pathlib import Path

import d289_holds
from engine_items import decode_appearance_object, protobuf_fields

ROOT = Path(__file__).resolve().parents[3]
COMPILER = "tools/content-schema/item-authoring/lower_client_physical_packet.py"
PROOF = "docs/agents/evidence/OTV2-20261001-item-physical-source-qualification-v1.json"
PROOF_SHA = "7b8a0be20602f730fa991b838c2b987e08136773ac1035cc638184f4faa8d67c"
DECODER = "tools/content-schema/item-authoring/engine_items.py"
DECODER_SHA = "0a494fd64d7774507b4d9e493492a7ffc9d99d751dd2ed4fec73561cb06f5b06"
CLIENT_SHA = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
CLIENT = f"content/assets/files/appearances-{CLIENT_SHA}.dat"
BINDINGS = "imports/crystalserver/bindings/items.json"
WIKI = "imports/tibiawiki/facts/items-stats.json"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261001-item-physical-promotion-v1.json"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def checked(root, path, digest):
    data = (root / path).read_bytes()
    if sha(data) != digest:
        raise ValueError(f"source digest drift: {path}")
    return data


def leaf(semantics, group, name, expected):
    state = semantics.get(group, {"state": "UNKNOWN"})
    if state["state"] == "UNKNOWN":
        return
    if state["state"] != "KNOWN":
        raise ValueError(f"blocked {group}")
    value = state["value"][name]
    if value["state"] != "UNKNOWN" and value != {"state": "KNOWN", "value": expected}:
        raise ValueError(f"blocked or conflicting {group}.{name}")


def build(root=ROOT):
    proof = json.loads(checked(root, PROOF, PROOF_SHA))
    checked(root, DECODER, DECODER_SHA)
    objects = {}
    for tag, raw in protobuf_fields(checked(root, CLIENT, CLIENT_SHA)):
        if tag == 1:
            obj = decode_appearance_object(raw)
            iid = obj["id"]
            if iid in objects:
                raise ValueError("duplicate client object identity")
            objects[iid] = obj | {"object_sha256": sha(raw)}
    snapshot = json.loads((root / WIKI).read_text())
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
    wiki = {r["item_id"]: r for r in snapshot["records"].values()}
    pages = defaultdict(set)
    for iid, record in wiki.items():
        for obs in record["observations"]:
            pages[obs["page_id"]].add(iid)
    definitions = {}
    for shard in json.loads((root / "content/items/index.json").read_text())["shards"]:
        for row in json.loads((root / shard).read_text())["records"]:
            d = row["definition"]
            definitions[d["identity"]["key"]] = d
    routed, maps = set(), {}
    for family in ("terrain", "objects"):
        for path in sorted((root / f"content/world/{family}").glob("*.json")):
            maps[str(path.relative_to(root))] = sha(path.read_bytes())
            for row in json.loads(path.read_text()).get("records", []):
                pointer = row.get("provenance", {}).get("item_pointer")
                if pointer:
                    routed.add(pointer["key"])
    bound = {}
    for b in json.loads((root / BINDINGS).read_text())["bindings"]:
        if b["disposition"] != "EXACT":
            continue
        if (
            b["identity_namespace"] != "ots/item_server_id"
            or b["source_key"] != "oteryn:source.crystalserver"
            or b["source_revision"] not in proof["bridge"]["source_revisions"]
            or b["target"]["family"] != "Item"
            or b["target"]["revision"] != "definition-r1"
        ):
            raise ValueError("unqualified exact binding")
        iid = int(b["external_id"])
        if not 0 < iid <= 65535 or iid in bound:
            raise ValueError("duplicate or out-of-range source identity")
        bound[iid] = b
    rows, holds = [], []
    for iid, obj in sorted(objects.items()):
        if obj["flags"].get("flags.take") is not True:
            continue  # Absence is UNKNOWN, never false.
        b = bound.get(iid)
        key = b["target"]["key"] if b else None
        if not b or key not in definitions or key in routed:
            holds.append(
                {
                    "appearance_id": iid,
                    "item_key": key,
                    "reason": "EXISTING_MAP_OWNER"
                    if key in routed
                    else "NO_EXACT_BOUND_ITEM",
                }
            )
            continue
        if key in d289_holds.NATIVE_CORE_HOLD_KEYS:
            if iid in proof["rune_ids"]:
                raise ValueError(f"D289 held magic rune: {iid}")
            # D289: the accepted Native core hold wins; no pickupability is promoted.
            holds.append(
                {
                    "appearance_id": iid,
                    "item_key": key,
                    "reason": d289_holds.NATIVE_CORE_HOLD,
                }
            )
            continue
        d = definitions[key]
        leaf(d.get("semantics", {}), "physical", "pickupable", True)
        source = {
            "binding": b,
            "appearance_id": iid,
            "object_sha256": obj["object_sha256"],
        }
        rows.append({"item_key": key, "kind": "PICKUPABLE_TRUE", "source": source})
        if iid not in proof["rune_ids"]:
            continue
        obs = wiki[iid]["observations"]
        if (
            obj["flags"].get("flags.cumulative") is not True
            or any(
                o["fields"].get("objectclass") != "Runes"
                or o["fields"].get("primarytype")
                not in {"Attack Runes", "Healing Runes", "Support Runes"}
                or o["fields"].get("stackable") != "yes"
                or pages[o["page_id"]] != {iid}
                for o in obs
            )
            or d["stack_class"] == "NonStackable"
        ):
            raise ValueError(f"magic-rune qualification drift: {iid}")
        leaf(d.get("semantics", {}), "stack", "stackable", True)
        leaf(d.get("semantics", {}), "stack", "stack_max", 100)
        rows.append(
            {
                "item_key": key,
                "kind": "RUNE_STACK_100",
                "source": source,
                "wiki_observations": obs,
                "continuity": "DERIVED",
            }
        )
    kinds = {
        k: sum(r["kind"] == k for r in rows)
        for k in ("PICKUPABLE_TRUE", "RUNE_STACK_100")
    }
    d289_holds.require_hits(
        d289_holds.NATIVE_CORE_HOLD_KEYS,
        [h["item_key"] for h in holds if h["reason"] == d289_holds.NATIVE_CORE_HOLD],
        "physical",
    )
    if kinds != {"PICKUPABLE_TRUE": 6755, "RUNE_STACK_100": 39} or len(holds) != 497:
        raise ValueError(f"bounded physical scope drift: {kinds}, holds={len(holds)}")
    return {
        "schema": "OTERYN_ITEM_PHYSICAL_PROMOTION/v1",
        "compiler": {"path": COMPILER, "sha256": sha((root / COMPILER).read_bytes())},
        "sources": {
            p: sha((root / p).read_bytes())
            for p in (PROOF, DECODER, CLIENT, BINDINGS, WIKI)
        },
        "map_owner_inputs": maps,
        "baseline_head": "5cb90c84013faa4c07fc35e3f660e67db6d691d2",
        "policy": proof["policy"],
        "counts": {
            "promotions": len(rows),
            "fields": 6833,
            "distinct_items": 6755,
            "holds": len(holds),
            **kinds,
        },
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
            raise SystemExit("physical packet drift")
    else:
        OUTPUT.write_bytes(data)
    print(json.dumps(packet["counts"]))


if __name__ == "__main__":
    main()
