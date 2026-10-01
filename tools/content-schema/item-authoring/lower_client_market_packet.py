"""Qualify affirmative official Market flags as static Item marketability only."""

import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

from engine_items import decode_appearance_object, protobuf_fields
from lower_wiki_stack_default_packet import checked, exact_bindings, raw_parameters, sha

ROOT = Path(__file__).resolve().parents[3]
COMPILER = "tools/content-schema/item-authoring/lower_client_market_packet.py"
PROOF = (
    "docs/agents/evidence/OTV2-20261001-item-market-true-source-qualification-v1.json"
)
PROOF_SHA = "5a6f623e3392ecba87ccf90388585b4d7cde64a0c09fc76c6817e044b82a242f"
CLIENT_SHA = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
CLIENT = f"content/assets/files/appearances-{CLIENT_SHA}.dat"
BINDINGS = "imports/crystalserver/bindings/items.json"
WIKI = "imports/tibiawiki/facts/items-stats.json"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261001-item-market-true-promotion-v1.json"
WORLD_FLAGS = (
    "flags.clip",
    "flags.corpse",
    "flags.player_corpse",
    "flags.liquidpool",
    "flags.bank",
)


def current(definition):
    group = definition.get("semantics", {}).get(
        "trade_restrictions", {"state": "UNKNOWN"}
    )
    if group["state"] == "UNKNOWN":
        return None
    if group["state"] != "KNOWN":
        return "BLOCKED_TRADE_GROUP"
    value = group["value"]["marketable"]
    if value["state"] == "UNKNOWN":
        return None
    if value == {"state": "KNOWN", "value": True}:
        return True
    return "BLOCKED_OR_CONFLICTING_MARKETABLE"


def qualify(definition, binding, obj, routed, absent, witness_reasons, retained):
    if not binding or not definition:
        return ["NO_EXACT_BOUND_NATIVE_ITEM"]
    key = binding["target"]["key"]
    why = []
    if binding["target"] != definition["identity"]:
        why.append("BINDING_TARGET_NATIVE_IDENTITY_DRIFT")
    if key in routed or key not in absent:
        why.append("EXISTING_OR_UNQUALIFIED_WORLD_OWNER")
    flags = obj["flags"]
    if flags.get("flags.market") is not True or flags.get("flags.take") is not True:
        why.append("NO_AFFIRMATIVE_MARKET_AND_TAKE")
    if any(flags.get(f) is True for f in (*WORLD_FLAGS, "flags.unmove")):
        why.append("AFFIRMATIVE_WORLD_DOMAIN_HOLD")
    presentation = definition.get("semantics", {}).get(
        "presentation", {"state": "UNKNOWN"}
    )
    name = presentation.get("value", {}).get("name", {"state": "UNKNOWN"})
    if presentation["state"] not in {"UNKNOWN", "KNOWN"} or name["state"] not in {
        "UNKNOWN",
        "KNOWN",
    }:
        why.append("BLOCKED_PRESENTATION")
    elif not obj.get("name") or (
        name["state"] == "KNOWN"
        and name["value"].strip().casefold() != obj["name"].strip().casefold()
    ):
        why.append("EXACT_OFFICIAL_NAME_CONFLICT")
    state = current(definition)
    if isinstance(state, str):
        why.append(state)
    why.extend(witness_reasons)
    if any(o["fields"].get("marketable") not in (None, "", "yes") for o in retained):
        why.append("RETAINED_WIKI_MARKET_OPPOSITION")
    return sorted(set(why))


def witness_holds(witness, cutoff):
    if sha(witness["own_object_raw"].encode()) != witness["raw_sha256"]:
        raise ValueError("own-template witness digest drift")
    if witness["revision_timestamp"] > cutoff:
        return ["WIKI_WITNESS_AFTER_CUTOFF"]
    fields = raw_parameters(witness["own_object_raw"])
    ids = fields.get("itemid", [])
    if len(ids) != 1 or not re.fullmatch(r"[1-9][0-9]*(?:\s*,\s*[1-9][0-9]*)*", ids[0]):
        return ["AMBIGUOUS_OPPOSING_WIKI_IDENTITY"]
    if (
        sorted({int(i.strip()) for i in ids[0].split(",")})
        != witness["source_item_ids"]
    ):
        raise ValueError("own-template witness source identity drift")
    values = fields.get("marketable", [])
    if len(values) != 1:
        return ["AMBIGUOUS_WIKI_MARKET_PARAMETER"]
    return (
        []
        if values[0] in ("", "yes")
        else ["EXPLICIT_OR_MALFORMED_WIKI_MARKET_OPPOSITION"]
    )


def build(root=ROOT):
    proof = json.loads(checked(root, PROOF, PROOF_SHA))
    for path, digest in proof["input_digests"].items():
        checked(root, path, digest)
    if (
        proof["policy"]["source_flag"] != "flags.market"
        or proof["policy"]["source_value"] is not True
        or proof["policy"]["native_field"] != "trade_restrictions.marketable"
    ):
        raise ValueError("market source-field route drift")
    doc = (
        root / "docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md"
    ).read_text()
    if proof["policy"]["quote"] not in doc:
        raise ValueError("accepted independent marketability meaning drift")
    objects = {}
    for tag, raw in protobuf_fields(checked(root, CLIENT, CLIENT_SHA)):
        if tag == 1:
            obj = decode_appearance_object(raw)
            if obj["id"] in objects:
                raise ValueError("duplicate official appearance identity")
            objects[obj["id"]] = obj | {"object_sha256": sha(raw)}
    records = proof["official_market_records"]
    if len({r["source_item_id"] for r in records}) != len(records) or {
        r["source_item_id"] for r in records
    } != {i for i, o in objects.items() if o["flags"].get("flags.market") is True}:
        raise ValueError("complete affirmative Market cohort drift")
    bindings = exact_bindings(
        json.loads((root / BINDINGS).read_text())["bindings"], proof["source_revisions"]
    )
    bound = {int(b["external_id"]): b for b in bindings.values()}
    definitions = {
        r["definition"]["identity"]["key"]: r["definition"]
        for s in json.loads((root / "content/items/index.json").read_text())["shards"]
        for r in json.loads((root / s).read_text())["records"]
    }
    snapshot = json.loads((root / WIKI).read_text())
    if (
        sha(
            json.dumps(
                snapshot["records"],
                sort_keys=True,
                ensure_ascii=False,
                separators=(",", ":"),
            ).encode()
        )
        != snapshot["snapshot_sha256"]
    ):
        raise ValueError("retained Wiki snapshot digest drift")
    wiki = {r["item_id"]: r["observations"] for r in snapshot["records"].values()}
    opposition = defaultdict(list)
    for witness in proof["wiki_witnesses"]:
        why = witness_holds(witness, proof["qualification_cutoff"])
        for iid in witness["source_item_ids"]:
            opposition[iid].extend(why)
    routed, maps = set(), {}
    for family in ("terrain", "objects"):
        for path in sorted((root / f"content/world/{family}").glob("*.json")):
            maps[str(path.relative_to(root))] = sha(path.read_bytes())
            routed.update(
                r["provenance"]["item_pointer"]["key"]
                for r in json.loads(path.read_text()).get("records", [])
                if r.get("provenance", {}).get("item_pointer")
            )
    absent = set(proof["world_owner_baseline"]["candidate_absence_keys"])
    rows, holds = [], []
    for source in records:
        iid = source["source_item_id"]
        obj, binding = objects[iid], bound.get(iid)
        if obj["object_sha256"] != source["object_sha256"]:
            raise ValueError("official object record digest drift")
        key = binding["target"]["key"] if binding else None
        why = qualify(
            definitions.get(key),
            binding,
            obj,
            routed,
            absent,
            opposition[iid],
            wiki.get(iid, []),
        )
        row = {
            "source_item_id": iid,
            "item_key": key,
            "object_sha256": source["object_sha256"],
        }
        if why:
            holds.append(row | {"reasons": why})
        else:
            rows.append(row | {"marketable": True, "binding": binding})
    if len(records) != 5120 or len(rows) != 4893:
        raise ValueError(
            f"closed Market qualification drift: {len(records)} records, {len(rows)} rows, {Counter(r for h in holds for r in h['reasons'])}"
        )
    return {
        "schema": "OTERYN_ITEM_MARKET_TRUE_PROMOTION/v1",
        "compiler": {"path": COMPILER, "sha256": sha((root / COMPILER).read_bytes())},
        "sources": {
            "proof_path": PROOF,
            "proof_sha256": PROOF_SHA,
            "client_path": CLIENT,
            "client_sha256": CLIENT_SHA,
        },
        "policy": proof["policy"],
        "world_owner_inputs": maps,
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
            raise SystemExit("Market packet drift")
    else:
        OUTPUT.write_bytes(data)
    print(json.dumps(packet["counts"]))


if __name__ == "__main__":
    main()
