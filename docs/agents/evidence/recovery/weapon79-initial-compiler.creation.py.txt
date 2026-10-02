"""Qualify closed79 intrinsic weapon source metadata without native gameplay writes."""
import argparse
import json
import re
from collections import Counter

import appearance_membership
import lower_wiki_movable_packet as movable
import lower_wiki_stack_default_packet as base
from engine_items import decode_appearance_object, protobuf_fields
from lower_client_market_packet import WORLD_FLAGS
from weapon_metadata_source_aliases import FIELDS, build_alias_catalog, formal_weapon

ROOT = base.ROOT
COMPILER = "tools/content-schema/item-authoring/lower_item_weapon_metadata_packet.py"
PROOF = "docs/agents/evidence/OTV2-20261002-item-weapon-metadata-source-qualification-v1.json"
PROOF_SHA = "de0b84740007ea684342f8a789c26c3f8a9f78a09fb1d350bf1b98f711b7bff6"
ALIASES = "tools/content-schema/item-authoring/fandom-weapon-metadata-alias-supplement.json"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261002-item-weapon-metadata-promotion-v1.json"


def parse_value(field, raw):
    grammar = r"([+-]?[0-9]+)" if field == "atk_mod" else r"([0-9]+)%"
    match = re.fullmatch(grammar, raw)
    if not match:
        raise ValueError("explicit weapon scalar grammar required")
    value = int(match[1])
    if field == "atk_mod" and -(2**31) <= value < 2**31:
        return value
    if field == "hit_chance" and 0 <= value <= 100:
        return {"numerator": value, "denominator": 1}
    raise ValueError("weapon scalar domain overflow")


def qualify(source, record, binding, obj, own, routed):
    iid, target, field = source["source_item_id"], source["target"], source["field"]
    if field not in FIELDS or not record or binding != source["binding"] or binding["target"] != target or record["definition"]["identity"] != target or target["key"] in routed:
        raise ValueError("full native/binding/World identity drift")
    if binding["external_id"] != str(iid) or target != {"family": "Item", "key": f"oteryn:item.tibia.i{iid}", "revision": "definition-r1"}:
        raise ValueError("EXACT own source ID required")
    flags = obj["flags"]
    if flags.get("flags.take") is not True or any(flags.get(k) is True for k in (*WORLD_FLAGS, "flags.unmove")):
        raise ValueError("portable Item domain not affirmed")
    weapon = movable.leaf(record["definition"], "weapon")
    expected = "DISTANCE" if field == "atk_mod" else "AMMUNITION"
    if weapon != source["native_weapon_guard"] or weapon.get("value", {}).get("weapon_type") != {"state": "KNOWN", "value": expected}:
        raise ValueError("native weapon type/projection drift")
    name = movable.leaf(record["definition"], "presentation.name")
    official = (obj.get("name") or "").strip().casefold()
    if not official or name != source["native_name_guard"] or name["state"] not in {"UNKNOWN", "KNOWN"} or (name["state"] == "KNOWN" and name["value"].strip().casefold() != official) or official != source["official_name"].strip().casefold():
        raise ValueError("source/native/official name conflict")
    values = []
    for page, box, params in own:
        if not box["balanced"] or box["inside_comment"] or not box["positive_exact_infobox_object_match"] or params.get("itemid") != [str(iid)] or any(len(v) != 1 for v in params.values()):
            raise ValueError("shared/malformed/duplicate own source")
        for key in ("name", "actualname"):
            if key in params and params[key][0].strip().casefold() != official:
                raise ValueError("all-present own source names must agree")
        if not params.get("actualname", params.get("name")):
            raise ValueError("own source name absent")
        if field in params:
            if params.get("primarytype") != ["Distance Weapons" if expected == "DISTANCE" else "Ammunition"]:
                raise ValueError("source family outside closed weapon scope")
            values.append(parse_value(field, params[field][0]))
    if not values or any(v != source["value"] for v in values):
        raise ValueError("whole own-ID present values disagree")
    owner, formal = FIELDS[field]
    old = (record.get("authoring") or {}).get(owner)
    if old is not None and old != source["value"]:
        raise ValueError("existing authoring weapon value conflict")
    facts = {owner: source["value"]}
    if formal_weapon(facts) != {formal.rsplit("/", 1)[1]: source["value"]}:
        raise ValueError("explicit formal weapon route lost")
    return {"target": target, "official_name": obj["name"], "facts": facts}


def build(root=ROOT):
    proof = json.loads(base.checked(root, PROOF, PROOF_SHA))
    if json.loads((root / ALIASES).read_text()) != build_alias_catalog():
        raise ValueError("weapon alias catalog drift")
    for path, digest in proof["input_digests"].items():
        base.checked(root, path, digest)
    bridge = proof["global_own_index_bridge"]
    global_proof = json.loads(base.checked(root, bridge["path"], bridge["sha256"]))
    identity = global_proof["identity_bridge"]
    identity_proof = json.loads(base.checked(root, identity["path"], identity["sha256"]))
    if identity_proof["bridge"]["source_revisions"] != proof["source_revisions"]:
        raise ValueError("official/server ID bridge drift")
    indexed, pages = movable.own_index(global_proof)
    _, manifests = appearance_membership.load_admitted(out_dir=root / "imports/official/appearance-membership")
    members = {k: {v[0]: v for v in m["entries"]} for k, m in manifests.items()}
    objects = {}
    for tag, raw in protobuf_fields(base.checked(root, proof["client_path"], proof["client_sha256"])):
        if tag == 1:
            obj = decode_appearance_object(raw)
            if obj["id"] in objects:
                raise ValueError("duplicate official Item ID")
            objects[obj["id"]] = obj | {"object_sha256": base.sha(raw)}
    bindings = base.exact_bindings(json.loads((root / base.BINDINGS).read_text())["bindings"], proof["source_revisions"])
    records = {}
    for shard in json.loads((root / "content/items/index.json").read_text())["shards"]:
        for record in json.loads((root / shard).read_text())["records"]:
            key = record["definition"]["identity"]["key"]
            if key in records:
                raise ValueError("duplicate native Item")
            records[key] = record
    routed, maps = set(), {}
    for family in ("objects", "terrain"):
        for path in sorted((root / f"content/world/{family}").glob("*.json")):
            maps[str(path.relative_to(root))] = base.sha(path.read_bytes())
            routed.update(r["provenance"]["item_pointer"]["key"] for r in json.loads(path.read_text()).get("records", []) if r.get("provenance", {}).get("item_pointer"))
    rows, seen = [], set()
    for source in proof["records"]:
        iid, key = source["source_item_id"], source["target"]["key"]
        obj = objects[iid]
        label = "crystal-donor-00ce02a5" if source["binding"]["source_revision"].startswith("00ce") else "crystal-ff7ede5"
        current, old = members["client-15.30"].get(iid), members[label].get(iid)
        if iid in seen or current != source["current_membership"] or old != source["binding_membership"] or not current or not old or current[1] != old[1] or obj["object_sha256"] != source["official_object_sha256"]:
            raise ValueError("closed scope/artifact/membership drift")
        seen.add(iid)
        tokens, own = set(), []
        for pid in indexed[iid]:
            page = pages[pid]
            for box in page["own_objects"]:
                if not any(str(iid) in re.findall(r"\d+", v) for v in box["raw_itemid_values"]):
                    continue
                token = f"{pid}:{box['box_index']}"
                witness = proof["competing_own_infobox_witnesses"][token]
                raw = witness["raw_own_infobox"]
                params = base.raw_parameters(raw)
                if base.sha(raw.encode()) != witness["raw_own_infobox_sha256"] or params.get("itemid") != box["raw_itemid_values"] or witness["source_capture_sha256"] != page["source_part_sha256"] or witness["source_capture_name"] != page["source_part"] or page["revision_timestamp"] > proof["qualification_cutoff"]:
                    raise ValueError("raw own template/coordinate/cutoff drift")
                tokens.add(token)
                own.append((page, box, params))
        if tokens != set(source["own_tokens"]):
            raise ValueError("missing competing own-ID source")
        rows.append(qualify(source, records.get(key), bindings.get(key), obj, own, routed))
    counts = Counter(s["field"] for s in proof["records"])
    if len(seen) != 79 or counts != {"atk_mod": 54, "hit_chance": 25} or Counter(s["native_name_guard"]["state"] for s in proof["records"])["UNKNOWN"] != 4:
        raise ValueError("closed79 scope drift")
    return {"schema": "OTERYN_ITEM_WEAPON_METADATA_PROMOTION/v1", "compiler": {"path": COMPILER, "sha256": base.sha((root / COMPILER).read_bytes())}, "sources": {"proof_path": PROOF, "proof_sha256": PROOF_SHA, "alias_catalog_sha256": base.sha((root / ALIASES).read_bytes())}, "world_owner_inputs": maps, "counts": {"fields": 79, "items": 79, "attack_modifier": 54, "absolute_hit_percent": 25}, "promotions": rows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--write-alias-catalog", action="store_true")
    args = parser.parse_args()
    if args.write_alias_catalog:
        (ROOT / ALIASES).write_text(json.dumps(build_alias_catalog(), sort_keys=True, indent=2) + "\n")
        return
    data = (json.dumps(build(), sort_keys=True, ensure_ascii=False, separators=(",", ":")) + "\n").encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("weapon metadata packet drift")
    else:
        OUTPUT.write_bytes(data)
    print("source-only weapon metadata79 fields/79Items")


if __name__ == "__main__":
    main()
