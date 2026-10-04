"""Qualify closed103 intrinsic weapon source metadata without native gameplay writes."""

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
PROOF = "docs/agents/evidence/OTV2-20261002-item-weapon-metadata-source-qualification-v2.json"
PROOF_SHA = "a9fa62e51f822251d2cd7c2b465e8397d02113f2380def178da8341e1654c2ae"
ALIASES = (
    "tools/content-schema/item-authoring/fandom-weapon-metadata-alias-supplement.json"
)
OUTPUT = (
    ROOT / "docs/agents/evidence/OTV2-20261002-item-weapon-metadata-promotion-v1.json"
)


def same(a, b):
    return json.dumps(a, sort_keys=True, separators=(",", ":")) == json.dumps(
        b, sort_keys=True, separators=(",", ":")
    )


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


def source_name(params):
    """Present empty actualname cannot fall back to a display label."""
    names = params.get("actualname", params.get("name", []))
    if len(names) != 1 or not names[0].strip():
        raise ValueError("unique nonempty actualname/name required")
    return names[0].strip().casefold()


def latest_frame(index, proof):
    frame = proof["official_current"]
    if (
        index["newest"] != "client-15.30"
        or index["newest"] != frame["label"]
        or index["files"][-1] != frame["descriptor"]
        or frame["descriptor"]["label"] != index["newest"]
        or frame["descriptor"]["appearances_sha256"] != proof["client_sha256"]
    ):
        raise ValueError("latest admitted official frame drift")
    return frame["label"]


def check_parent_receipt(root, proof):
    """Historical facts stay immutable; current guards allow only sealed Hit28."""
    frames = [
        json.loads(base.checked(root, f["path"], f["sha256"]))
        for f in proof["immutable_source_frames"]
    ]
    old = frames[0]["records"] + [r["source"] for r in frames[1]["qualified"]]
    historical = {s["source_item_id"]: s for s in old}
    if (
        len(old) != len(historical)
        or len(old) != 103
        or set(historical) != {s["source_item_id"] for s in proof["records"]}
    ):
        raise ValueError("closed103 exact source scope drift")
    receipt = proof["current_parent_receipt"]
    # The full reference digest is an immutable actual-parent observation.
    # Current identity/class/name/whole-weapon guards below allow unrelated
    # NativeStack successors without rewriting that historical receipt.
    packet = json.loads(
        base.checked(root, receipt["hit_packet_path"], receipt["hit_packet_sha256"])
    )
    hits = {
        r["target"]["key"]: r
        for r in packet["promotions"]
        if "hit_chance" in r["facts"]
    }
    deltas = {r["item_id"]: r for r in receipt["expected_hit_deltas"]}
    if (
        len(deltas) != 28
        or sorted(deltas) != receipt["expected_hit_delta_ids"]
        or receipt["parent_commit"] != proof["authoring_baseline"]
    ):
        raise ValueError("explicit current parent Hit28 receipt drift")
    immutable = (
        "source_item_id",
        "target",
        "binding",
        "official_name",
        "official_object_sha256",
        "current_membership",
        "binding_membership",
        "own_tokens",
        "field",
        "value",
        "native_name_guard",
    )
    for source in proof["records"]:
        prior = historical[source["source_item_id"]]
        if any(not same(source[k], prior[k]) for k in immutable):
            raise ValueError("immutable weapon source facts changed")
        expected = prior["native_weapon_guard"]
        if source["source_item_id"] in deltas:
            row = deltas[source["source_item_id"]]
            if not same(row["before_native_weapon_guard"], expected) or not same(
                row["explicit_Hit67_row"], hits.get(source["target"]["key"])
            ):
                raise ValueError("Hit28 predecessor packet opposition")
            expected = json.loads(json.dumps(expected))
            expected["value"]["hit_chance"] = {
                "state": "KNOWN",
                "value": row["explicit_Hit67_row"]["facts"]["hit_chance"],
            }
        if not same(source["native_weapon_guard"], expected):
            raise ValueError("unapproved native weapon projection delta")
    parents = {r["item"]["key"]: r for r in receipt["parent_authoring"]}
    current = json.loads((root / receipt["parent_declarations_path"]).read_text())[
        "item_authoring"
    ]
    seen, targets = set(), {s["target"]["key"]: s["target"] for s in proof["records"]}
    for owner in current:
        key = owner["item"]["key"]
        if key in seen or key not in parents and key not in targets:
            raise ValueError("unexpected/duplicate source authoring owner")
        seen.add(key)
        if key in targets:
            source = next(s for s in proof["records"] if s["target"]["key"] == key)
            expected = {FIELDS[source["field"]][0]: source["value"]}
            for prop, value in owner.items():
                if prop in {f[0] for f in FIELDS.values()} and (
                    prop not in expected or not same(value, expected[prop])
                ):
                    raise ValueError("unqualified current weapon metadata property")
        stripped = {
            k: v
            for k, v in owner.items()
            if not (key in targets and k in {f[0] for f in FIELDS.values()})
        }
        if not same(stripped, parents.get(key, {"item": targets.get(key)})):
            raise ValueError("current parent authoring sibling drift")
    if (
        not set(parents).issubset(seen)
        or len(parents) != receipt["parent_authoring_count"]
    ):
        raise ValueError("current parent source owners missing")


# D316/D341: main's quest-reward admission overlay reaches content/items after promotion, so
# the Native guards compare against the pre-overlay stage that the Rust promotion applies to.
PRE_OVERLAY = "content/world/definitions/reference.json"


def pre_overlay_definitions(root, current):
    """Pre-overlay Item definitions; exactly the current Item identities, no others."""
    definitions = {}
    for row in json.loads((root / PRE_OVERLAY).read_text())["records"]:
        if row["identity"]["family"] != "Item":
            continue
        if row["identity"]["key"] in definitions:
            raise ValueError("duplicate pre-overlay Item identity")
        definitions[row["identity"]["key"]] = row
    if set(definitions) != set(current):
        raise ValueError("pre-overlay/current Item identity set drift")
    return definitions


def qualify(source, record, binding, obj, own, routed):
    iid, target, field = source["source_item_id"], source["target"], source["field"]
    if (
        field not in FIELDS
        or not record
        or binding != source["binding"]
        or binding["target"] != target
        or record["definition"]["identity"] != target
        or target["key"] in routed
    ):
        raise ValueError("full native/binding/World identity drift")
    if binding["external_id"] != str(iid) or target != {
        "family": "Item",
        "key": f"oteryn:item.tibia.i{iid}",
        "revision": "definition-r1",
    }:
        raise ValueError("EXACT own source ID required")
    flags = obj["flags"]
    if flags.get("flags.take") is not True or any(
        flags.get(k) is True for k in (*WORLD_FLAGS, "flags.unmove")
    ):
        raise ValueError("portable Item domain not affirmed")
    weapon = movable.leaf(record["definition"], "weapon")
    expected = "DISTANCE" if field == "atk_mod" else "AMMUNITION"
    if not same(weapon, source["native_weapon_guard"]) or weapon.get("value", {}).get(
        "weapon_type"
    ) != {"state": "KNOWN", "value": expected}:
        raise ValueError("native weapon type/projection drift")
    header_keys = ("kind", "stack_class", "materializable", "client_projection")
    header_guard = source.get("native_class_guard")
    if not isinstance(header_guard, dict) or set(header_guard) != set(header_keys):
        raise ValueError("missing or malformed closed native class guard")
    if not same(
        {key: record["definition"].get(key) for key in header_keys}, header_guard
    ):
        raise ValueError("current native class/admission/projection drift")
    name = movable.leaf(record["definition"], "presentation.name")
    official = (obj.get("name") or "").strip().casefold()
    if (
        not official
        or name != source["native_name_guard"]
        or name["state"] not in {"UNKNOWN", "KNOWN"}
        or (name["state"] == "KNOWN" and name["value"].strip().casefold() != official)
        or official != source["official_name"].strip().casefold()
    ):
        raise ValueError("source/native/official name conflict")
    values = []
    for page, box, params in own:
        if (
            not box["balanced"]
            or box["inside_comment"]
            or not box["positive_exact_infobox_object_match"]
            or params.get("itemid") != [str(iid)]
            or any(len(v) != 1 for v in params.values())
        ):
            raise ValueError("shared/malformed/duplicate own source")
        if source_name(params) != official:
            raise ValueError("selected own source actualname/name conflict")
        if field in params:
            if params.get("primarytype") != [
                "Distance Weapons" if expected == "DISTANCE" else "Ammunition"
            ]:
                raise ValueError("source family outside closed weapon scope")
            values.append(parse_value(field, params[field][0]))
    if not values or any(not same(v, source["value"]) for v in values):
        raise ValueError("whole own-ID present values disagree")
    owner, formal = FIELDS[field]
    old = (record.get("authoring") or {}).get(owner)
    if old is not None and not same(old, source["value"]):
        raise ValueError("existing authoring weapon value conflict")
    siblings = {
        k: v
        for k, v in (record.get("authoring") or {}).items()
        if k not in {f[0] for f in FIELDS.values()}
    }
    if "parent_authoring_guard" in source and not same(
        siblings, source["parent_authoring_guard"]
    ):
        raise ValueError("canonical authoring sibling drift")
    facts = {owner: source["value"]}
    if formal_weapon(facts) != {formal.rsplit("/", 1)[1]: source["value"]}:
        raise ValueError("explicit formal weapon route lost")
    return {"target": target, "official_name": obj["name"], "facts": facts}


def build(root=ROOT):
    proof = json.loads(base.checked(root, PROOF, PROOF_SHA))
    check_parent_receipt(root, proof)
    if json.loads((root / ALIASES).read_text()) != build_alias_catalog():
        raise ValueError("weapon alias catalog drift")
    for path, digest in proof["input_digests"].items():
        base.checked(root, path, digest)
    bridge = proof["global_own_index_bridge"]
    global_proof = json.loads(base.checked(root, bridge["path"], bridge["sha256"]))
    identity = global_proof["identity_bridge"]
    identity_proof = json.loads(
        base.checked(root, identity["path"], identity["sha256"])
    )
    if identity_proof["bridge"]["source_revisions"] != proof["source_revisions"]:
        raise ValueError("official/server ID bridge drift")
    indexed, pages = movable.own_index(global_proof)
    frame = proof["official_current"]
    base.checked(root, frame["index_path"], frame["index_sha256"])
    base.checked(root, frame["manifest_path"], frame["manifest_sha256"])
    admitted, manifests = appearance_membership.load_admitted(
        out_dir=root / "imports/official/appearance-membership"
    )
    current_label = latest_frame(admitted, proof)
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
                raise ValueError("duplicate native Item")
            records[key] = record
    # D316/D341: the Native guards read the pre-overlay definition stage.
    definitions = pre_overlay_definitions(root, records)
    records = {
        key: record | {"definition": definitions[key]}
        for key, record in records.items()
    }
    routed, maps = set(), {}
    for family in ("objects", "terrain"):
        for path in sorted((root / f"content/world/{family}").glob("*.json")):
            maps[str(path.relative_to(root))] = base.sha(path.read_bytes())
            routed.update(
                r["provenance"]["item_pointer"]["key"]
                for r in json.loads(path.read_text()).get("records", [])
                if r.get("provenance", {}).get("item_pointer")
            )
    if maps != proof["world_owner_inputs"]:
        raise ValueError("current World owner inputs drift")
    rows, seen = [], set()
    for source in proof["records"]:
        iid, key = source["source_item_id"], source["target"]["key"]
        obj = objects[iid]
        label = (
            "crystal-donor-00ce02a5"
            if source["binding"]["source_revision"].startswith("00ce")
            else "crystal-ff7ede5"
        )
        current, old = members[current_label].get(iid), members[label].get(iid)
        if (
            iid in seen
            or current != source["current_membership"]
            or old != source["binding_membership"]
            or not current
            or not old
            or current[1] != old[1]
            or obj["object_sha256"] != source["official_object_sha256"]
        ):
            raise ValueError("closed scope/artifact/membership drift")
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
                    or params.get("itemid") != box["raw_itemid_values"]
                    or witness["source_capture_sha256"] != page["source_part_sha256"]
                    or witness["source_capture_name"] != page["source_part"]
                    or any(
                        witness[k] != page[k]
                        for k in (
                            "page_id",
                            "revision_id",
                            "revision_timestamp",
                            "content_sha256",
                            "title",
                        )
                    )
                    or page["revision_timestamp"] > proof["qualification_cutoff"]
                ):
                    raise ValueError("raw own template/coordinate/cutoff drift")
                tokens.add(token)
                own.append((page, box, params))
        if tokens != set(source["own_tokens"]):
            raise ValueError("missing competing own-ID source")
        rows.append(
            qualify(source, records.get(key), bindings.get(key), obj, own, routed)
        )
    counts = Counter(s["field"] for s in proof["records"])
    if (
        len(seen) != 103
        or counts != {"atk_mod": 78, "hit_chance": 25}
        or Counter(s["native_name_guard"]["state"] for s in proof["records"])["UNKNOWN"]
        != 4
    ):
        raise ValueError("closed103 scope drift")
    return {
        "schema": "OTERYN_ITEM_WEAPON_METADATA_PROMOTION/v1",
        "compiler": {
            "path": COMPILER,
            "sha256": base.sha((root / COMPILER).read_bytes()),
        },
        "sources": {
            "proof_path": PROOF,
            "proof_sha256": PROOF_SHA,
            "alias_catalog_sha256": base.sha((root / ALIASES).read_bytes()),
            "alias_helper_sha256": base.sha(
                (
                    root
                    / "tools/content-schema/item-authoring/weapon_metadata_source_aliases.py"
                ).read_bytes()
            ),
        },
        "world_owner_inputs": maps,
        "counts": {
            "fields": 103,
            "items": 103,
            "attack_modifier": 78,
            "absolute_hit_percent": 25,
        },
        "promotions": rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--write-alias-catalog", action="store_true")
    args = parser.parse_args()
    if args.write_alias_catalog:
        (ROOT / ALIASES).write_text(
            json.dumps(build_alias_catalog(), sort_keys=True, indent=2) + "\n"
        )
        return
    data = (
        json.dumps(build(), sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("weapon metadata packet drift")
    else:
        OUTPUT.write_bytes(data)
    print("source-only weapon metadata103 fields/103Items")


if __name__ == "__main__":
    main()
