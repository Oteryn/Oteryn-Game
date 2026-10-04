"""Closed optional wand/rod/rune source observations; no execution lowering."""

import argparse
import json
import re
from collections import Counter

import appearance_membership
import lower_wiki_movable_packet as movable
import lower_wiki_stack_default_packet as base
from engine_items import decode_appearance_object, protobuf_fields
from fandom_use_observation_aliases import build_fandom_use_observation_alias_supplement
from lower_client_market_packet import WORLD_FLAGS

ROOT = base.ROOT
COMPILER = "tools/content-schema/item-authoring/lower_item_use_observation_packet.py"
PROOF = "docs/agents/evidence/OTV2-20261002-item-use-observation-source-qualification-v1.json"
PROOF_SHA = "d4a19507c3a3e92729455b182b2e5b401d972191e841635ab8c61d150cce6bc1"
OUTPUT = (
    ROOT / "docs/agents/evidence/OTV2-20261002-item-use-observation-promotion-v1.json"
)
ALIASES = (
    "tools/content-schema/item-authoring/fandom-use-observation-alias-supplement.json"
)
FIELDS = {"damagerange": "damage", "damagetype": "damage_type", "manacost": "mana_cost"}


def parse_value(parameter, raw, iid):
    if parameter == "manacost":
        grammar = r"\+?[0-9]+" if iid == 52336 else r"[0-9]+"
        if re.fullmatch(grammar, raw) and int(raw) <= 2**32 - 1:
            return int(raw)
    elif parameter == "damagetype":
        if raw.lower() in {
            "physical",
            "energy",
            "earth",
            "fire",
            "ice",
            "death",
            "holy",
        }:
            return raw
    elif re.fullmatch(r"[+-]?[0-9]+", raw) and -(2**63) <= int(raw) < 2**63:
        return {"kind": "Integer", "value": int(raw)}
    elif re.fullmatch(r"[0-9]+-[0-9]+", raw):
        lo, hi = map(int, raw.split("-"))
        if lo <= hi < 2**63:
            return {"kind": "Range", "value": {"min": lo, "max": hi}}
    elif (
        raw
        and raw.strip() == raw
        and len(raw.encode()) <= 4096
        and not any(ord(c) < 32 or ord(c) == 127 for c in raw)
    ):
        return {"kind": "Text", "value": raw}
    raise ValueError("present empty/unsupported observation")


def qualify(source, record, binding, obj, own, routed, prospective=False):
    iid, target, facts = source["source_item_id"], source["target"], source["facts"]
    if (
        not record
        or binding != source["binding"]
        or binding["target"] != target
        or record["definition"]["identity"] != target
        or target["key"] in routed
    ):
        raise ValueError("exact full native/binding/World identity drift")
    if target != {
        "family": "Item",
        "key": f"oteryn:item.tibia.i{iid}",
        "revision": "definition-r1",
    } or binding["external_id"] != str(iid):
        raise ValueError("canonical Item identity drift")
    flags = obj["flags"]
    if flags.get("flags.take") is not True or any(
        flags.get(k) is True for k in (*WORLD_FLAGS, "flags.unmove")
    ):
        raise ValueError("positive portable Item domain required")
    if (
        not obj.get("name")
        or obj["name"].strip().casefold() != source["official_name"].strip().casefold()
    ):
        raise ValueError("current official name drift")
    name = movable.leaf(record["definition"], "presentation.name")
    if name["state"] not in {"UNKNOWN", "KNOWN"}:
        raise ValueError("native presentation blocked")
    if (
        name["state"] == "KNOWN"
        and name["value"].strip().casefold() != obj["name"].strip().casefold()
    ):
        pending = source.get("conditional_name_successor")
        if not (
            prospective
            and pending
            and name == pending["prior"]
            and pending["incoming"].strip().casefold() == obj["name"].strip().casefold()
            and pending["approved_source_proof_sha256"]
            == "57cd9d92425b8e71f8b4755fa2023597ae5820e437aacd70184227d0e907bb23"
            and not record.get("authoring", {}).get("presentation")
        ):
            raise ValueError(
                "native official name conflict; approved successor required"
            )
    values = {k: [] for k in facts}
    for page, box, params in own:
        if (
            not box["balanced"]
            or box["inside_comment"]
            or not box["positive_exact_infobox_object_match"]
            or params.get("itemid") != [str(iid)]
            or any(len(v) != 1 for v in params.values())
        ):
            raise ValueError("ambiguous/shared/duplicate own source")
        names = params.get("actualname", params.get("name", []))
        if (
            len(names) != 1
            or names[0].strip().casefold() != obj["name"].strip().casefold()
        ):
            raise ValueError("whole own source name conflict")
        if "damage" in params or "mana" in params:
            raise ValueError("competing legacy alias must be independently qualified")
        for parameter, field in FIELDS.items():
            if field not in facts or parameter not in params:
                continue
            allowed = {"Rods", "Wands"} | (
                {"Attack Runes"} if field != "damage" else set()
            )
            if params.get("primarytype", []) not in [[f] for f in allowed]:
                raise ValueError("source observation family not admitted")
            values[field].append(parse_value(parameter, params[parameter][0], iid))
    old = record.get("authoring", {}).get("use_observation") or {}
    for field, value in facts.items():
        if not values[field] or any(v != value for v in values[field]):
            raise ValueError("whole all-present observation opposition")
        if old.get(field) is not None and old[field] != value:
            raise ValueError("existing authoring observation conflict")
    return {"target": target, "official_name": obj["name"], "facts": facts}


def build(root=ROOT, prospective=False):
    proof = json.loads(base.checked(root, PROOF, PROOF_SHA))
    contract = (
        root / "docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md"
    )
    if (
        "source-only use observations for legacy/current Infobox `damage`, `damagetype` and `mana`"
        not in contract.read_text()
    ):
        raise ValueError("accepted source-only observation owner missing")
    if (
        json.loads((root / ALIASES).read_text())
        != build_fandom_use_observation_alias_supplement()
    ):
        raise ValueError("explicit source alias catalog drift")
    for path, digest in proof["input_digests"].items():
        base.checked(root, path, digest)
    bridge = proof["global_own_index_bridge"]
    global_proof = json.loads(base.checked(root, bridge["path"], bridge["sha256"]))
    identity = global_proof["identity_bridge"]
    identity_proof = json.loads(
        base.checked(root, identity["path"], identity["sha256"])
    )
    if identity_proof["bridge"]["source_revisions"] != proof["source_revisions"]:
        raise ValueError("accepted official/server-ID bridge revision drift")
    indexed, pages = movable.own_index(global_proof)
    _, manifests = appearance_membership.load_admitted(
        out_dir=root / "imports/official/appearance-membership"
    )
    members = {k: {v[0]: v for v in m["entries"]} for k, m in manifests.items()}
    objects = {}
    for tag, raw in protobuf_fields(
        base.checked(root, proof["client_path"], proof["client_sha256"])
    ):
        if tag == 1:
            obj = decode_appearance_object(raw)
            if obj["id"] in objects:
                raise ValueError("duplicate official Item ID")
            objects[obj["id"]] = obj | {"object_sha256": base.sha(raw)}
    bindings = base.exact_bindings(
        json.loads((root / base.BINDINGS).read_text())["bindings"],
        proof["source_revisions"],
    )
    records = {}
    for shard in json.loads((root / "content/items/index.json").read_text())["shards"]:
        for record in json.loads((root / shard).read_text())["records"]:
            key = record["definition"]["identity"]["key"]
            if key in records:
                raise ValueError("duplicate current native Item")
            records[key] = record
    routed, maps = set(), {}
    for source in proof["world_owner_baseline"]["input_digests"]:
        base.checked(root, source["path"], source["sha256"])
    for family in ("objects", "terrain"):
        for path in sorted((root / f"content/world/{family}").glob("*.json")):
            maps[str(path.relative_to(root))] = base.sha(path.read_bytes())
            routed.update(
                r["provenance"]["item_pointer"]["key"]
                for r in json.loads(path.read_text()).get("records", [])
                if r.get("provenance", {}).get("item_pointer")
            )
    absent = set(proof["world_owner_baseline"]["candidate_absence_keys"])
    frozen = proof["frozen_complete_source_world_owner_projection"]
    frozen_keys = frozen["keys"]
    if (
        len(frozen_keys) != frozen["pointer_key_count"]
        or frozen_keys != sorted(set(frozen_keys))
        or base.sha((json.dumps(frozen_keys, separators=(",", ":")) + "\n").encode())
        != frozen["keys_sha256"]
    ):
        raise ValueError("complete retained Source World projection drift")
    routed.update(frozen_keys)
    rows, seen = [], set()
    for source in proof["records"]:
        iid, key = source["source_item_id"], source["target"]["key"]
        obj = objects[iid]
        label = (
            "crystal-donor-00ce02a5"
            if source["binding"]["source_revision"].startswith("00ce")
            else "crystal-ff7ede5"
        )
        current, old = members["client-15.30"].get(iid), members[label].get(iid)
        if (
            iid in seen
            or key not in absent
            or current != source["current_membership"]
            or old != source["binding_membership"]
            or not current
            or not old
            or current[1] != old[1]
            or obj["object_sha256"] != source["official_object_sha256"]
        ):
            raise ValueError("closed scope/full artifact/membership/World drift")
        seen.add(iid)
        tokens, own = set(), []
        for pid in indexed[iid]:
            page = pages[pid]
            for box in page["own_objects"]:
                if not any(
                    str(iid) in re.findall(r"\d+", v) for v in box["raw_itemid_values"]
                ):
                    continue
                token = f"{pid}:{box['box_index']}"
                witness = proof["competing_own_infobox_witnesses"][token]
                raw = witness["raw_own_infobox"]
                params = base.raw_parameters(raw)
                if (
                    base.sha(raw.encode()) != witness["raw_own_infobox_sha256"]
                    or params.get("itemid", []) != box["raw_itemid_values"]
                    or witness["source_capture_sha256"] != page["source_part_sha256"]
                    or witness["source_capture_name"] != page["source_part"]
                    or page["revision_timestamp"] > proof["qualification_cutoff"]
                ):
                    raise ValueError("raw/coordinate/cutoff drift")
                tokens.add(token)
                own.append((page, box, params))
        if tokens != set(source["own_tokens"]):
            raise ValueError("missing global opposing own-ID witness")
        rows.append(
            qualify(
                source,
                records.get(key),
                bindings.get(key),
                obj,
                own,
                routed,
                prospective,
            )
        )
    counts = Counter(k for r in rows for k in r["facts"])
    if len(seen) != 139 or counts != {
        "damage": 111,
        "damage_type": 138,
        "mana_cost": 96,
    }:
        raise ValueError("closed345/139 scope drift")
    return {
        "schema": "OTERYN_ITEM_USE_OBSERVATION_PROMOTION/v1",
        "compiler": {
            "path": COMPILER,
            "sha256": base.sha((root / COMPILER).read_bytes()),
        },
        "sources": {
            "proof_path": PROOF,
            "proof_sha256": PROOF_SHA,
            "alias_catalog_sha256": base.sha((root / ALIASES).read_bytes()),
        },
        "world_owner_inputs": maps,
        "counts": {"fields": 345, "items": 139, "by_field": dict(counts)},
        "promotions": rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument(
        "--prospective-name-successor",
        action="store_true",
        help="Source-only checkpoint; Rust still requires corrected current names",
    )
    parser.add_argument("--write-alias-catalog", action="store_true")
    args = parser.parse_args()
    if args.write_alias_catalog:
        (ROOT / ALIASES).write_text(
            json.dumps(
                build_fandom_use_observation_alias_supplement(),
                sort_keys=True,
                indent=2,
            )
            + "\n"
        )
        return
    data = (
        json.dumps(
            build(prospective=args.prospective_name_successor),
            sort_keys=True,
            ensure_ascii=False,
            separators=(",", ":"),
        )
        + "\n"
    ).encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("use observation packet drift")
    else:
        OUTPUT.write_bytes(data)
    print("source-only use observation345 fields/139Items")


if __name__ == "__main__":
    main()
