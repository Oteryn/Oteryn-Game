"""Qualify the bounded unknown-capacity repair from wiki and client evidence.

The admitted scope contains 17 source-bound Items with no map owner, explicit wiki
volume and an affirmative client container flag. It does not admit materialization,
physical behavior or engine capacity hypotheses. Known conflicting values are held.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from engine_items import load_appearance_objects
from key_ring5801_source_selection import load_context, select

ROOT = Path(__file__).resolve().parents[3]
COMPILER = "tools/content-schema/item-authoring/lower_wiki_capacity_packet.py"
SNAPSHOT = "imports/tibiawiki/facts/items-stats.json"
CLIENT_DIGEST = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
CLIENT = f"content/assets/files/appearances-{CLIENT_DIGEST}.dat"
CORRECTIONS = {53074: (20, 22)}
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261001-item-capacity-promotion-v1.json"
# Explicit audited repair scope; regeneration must not expand native admission.
SCOPE = frozenset(
    [12644, 12763, 12764, 31935, 31936]
    + list(range(42273, 42279))
    + list(range(42328, 42332))
    + [42334, 42335]
)


def known_capacity(definition):
    return (
        definition.get("semantics", {})
        .get("container", {})
        .get("value", {})
        .get("capacity", {})
        .get("value")
    )


def qualify(snapshot, definitions, bound, routed, appearances, temporal_context=None):
    promotions, holds = [], []
    for record in sorted(snapshot["records"].values(), key=lambda r: r["item_id"]):
        iid = record["item_id"]
        key = f"oteryn:item.tibia.i{iid}"
        if key not in definitions or "semantics" not in definitions[key]:
            continue
        observations = [
            o
            for o in select(record, "container.capacity", temporal_context)
            if "volume" in o["fields"]
        ]
        if not observations:
            continue
        raw = [o["fields"]["volume"] for o in observations]
        parsed = [int(v) if v.isascii() and v.isdigit() else None for v in raw]
        old = known_capacity(definitions[key])
        container = definitions[key]["semantics"].get("container", {})
        blocked = container.get("state") in {
            "CONFLICT",
            "NOT_APPLICABLE",
        } or container.get("value", {}).get("capacity", {}).get("state") in {
            "CONFLICT",
            "NOT_APPLICABLE",
        }
        reason = None
        if blocked:
            reason = "BLOCKED_CAPACITY_EVIDENCE_STATE"
        elif any(v is None or not 1 <= v <= 65535 for v in parsed):
            reason = "MALFORMED_WIKI_VOLUME"
        elif len(set(parsed)) != 1:
            reason = "WIKI_PAGE_DISAGREEMENT"
        elif iid in CORRECTIONS and (old, parsed[0]) not in (
            CORRECTIONS[iid],
            (CORRECTIONS[iid][1], CORRECTIONS[iid][1]),
        ):
            reason = "CORRECTION_PRECONDITION_DRIFT"
        elif old is not None and old != parsed[0] and iid not in CORRECTIONS:
            reason = "KNOWN_CAPACITY_CONFLICT"
        elif iid not in SCOPE | CORRECTIONS.keys() and old is not None:
            continue
        elif key not in bound:
            reason = "NO_SOURCE_BINDING"
        elif key in routed:
            reason = "EXISTING_MAP_OWNER"
        elif not appearances.get(iid, {}).get("flags", {}).get("flags.container"):
            reason = "NO_AFFIRMATIVE_CLIENT_CONTAINER_FLAG"
        elif iid not in SCOPE | CORRECTIONS.keys():
            reason = "OUTSIDE_BOUNDED_REPAIR_SCOPE"
        sources = [
            {
                name: o[name]
                for name in ("page_id", "revision_id", "content_sha256", "url")
            }
            | {"volume": o["fields"]["volume"]}
            for o in observations
        ]
        row = {"item_key": key, "sources": sources}
        if reason:
            holds.append(
                row | {"reason": reason, "known_capacity": old, "wiki_volumes": raw}
            )
        else:
            promotions.append(
                row
                | {
                    "capacity": parsed[0],
                    "client_container_flag": True,
                    "expected_known_capacity": CORRECTIONS.get(iid, (None,))[0],
                }
            )
    if {
        int(r["item_key"].rsplit(".i", 1)[1]) for r in promotions
    } != SCOPE | CORRECTIONS.keys():
        raise ValueError("capacity repair scope lost qualification")
    return promotions, holds


def build(root=ROOT):
    snapshot = json.loads((root / SNAPSHOT).read_text())
    digest = hashlib.sha256(
        json.dumps(
            snapshot["records"],
            sort_keys=True,
            ensure_ascii=False,
            separators=(",", ":"),
        ).encode()
    ).hexdigest()
    if digest != snapshot["snapshot_sha256"]:
        raise ValueError("wiki snapshot digest mismatch")
    data = (root / CLIENT).read_bytes()
    if hashlib.sha256(data).hexdigest() != CLIENT_DIGEST:
        raise ValueError("client appearance digest mismatch")
    bindings = "imports/crystalserver/bindings/items.json"
    bound = {
        r["target"]["key"]
        for r in json.loads((root / bindings).read_text())["bindings"]
        if r["disposition"] == "EXACT"
    }
    routed, map_sources = set(), {}
    for family in ("terrain", "objects"):
        for path in sorted((root / f"content/world/{family}").glob("*.json")):
            map_sources[str(path.relative_to(root))] = hashlib.sha256(
                path.read_bytes()
            ).hexdigest()
            for row in json.loads(path.read_text()).get("records", []):
                pointer = row.get("provenance", {}).get("item_pointer")
                if pointer:
                    routed.add(pointer["key"])
    definitions = {}
    for shard in json.loads((root / "content/items/index.json").read_text())["shards"]:
        for row in json.loads((root / shard).read_text())["records"]:
            definition = row["definition"]
            definitions[definition["identity"]["key"]] = definition
    appearances = load_appearance_objects(data)
    temporal_context = load_context(root, snapshot, definitions, routed, appearances)
    promotions, holds = qualify(
        snapshot, definitions, bound, routed, appearances, temporal_context
    )
    return {
        "schema": "OTERYN_ITEM_CAPACITY_PROMOTION/v1",
        "compiler": {
            "path": COMPILER,
            "sha256": hashlib.sha256((root / COMPILER).read_bytes()).hexdigest(),
            "client_decoder_sha256": hashlib.sha256(
                (
                    root / "tools/content-schema/item-authoring/engine_items.py"
                ).read_bytes()
            ).hexdigest(),
        },
        "source": {
            "temporal_qualification": temporal_context["qualification"],
            "path": SNAPSHOT,
            "snapshot_sha256": digest,
            "client_path": CLIENT,
            "client_sha256": CLIENT_DIGEST,
            "map_owner_inputs": map_sources,
            "bindings_path": bindings,
            "bindings_sha256": hashlib.sha256(
                (root / bindings).read_bytes()
            ).hexdigest(),
        },
        "policy": "BOUNDED_UNKNOWN_REPAIR_ALL_PRESENT_WIKI_PAGES_AGREE_CLIENT_CORROBORATED_NO_MAP_OWNER_EXCEPT_EXACT_ITEM5801_CURRENT_SOURCE",
        "correction_evidence": {
            "oteryn:item.tibia.i53074": {
                "url": "https://www.tibiawiki.com.br/wiki/Adventurer_Backpack",
                "revision_id": 433126,
                "volume": 22,
                "request_id": "41bcbc97-06cb-4155-aabd-b950818a18a4",
                "retained_response_sha256": "14822ec274064ce35fd1b97ab3011daec8d0bcc32d21b5fdcf9e93fab8218fde",
                "raw_content_sha256": "539afa26096fec17fd997b1187b4147f46bbd2833f18a4210b4d475e1fc398af",
                "classification": "PROVEN_WIKI_AGREEMENT_UNIQUE_BOUND_IDENTITY_OTS_CAPACITY_HYPOTHESIS_REPLACED",
            }
        },
        "counts": {"promotions": len(promotions), "holds": len(holds)},
        "promotions": promotions,
        "holds": holds,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    packet = build()
    data = (
        json.dumps(packet, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("capacity packet drift")
    else:
        OUTPUT.write_bytes(data)
    print(json.dumps(packet["counts"]))


if __name__ == "__main__":
    main()
