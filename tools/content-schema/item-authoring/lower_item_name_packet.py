"""Correct139 proven imported generic names; preserve every other Known name."""

import argparse
import json
import re

import lower_wiki_movable_packet as global_frame
import lower_wiki_stack_default_packet as base
from engine_items import decode_appearance_object, protobuf_fields
from lower_client_market_packet import WORLD_FLAGS

ROOT = base.ROOT
COMPILER = "tools/content-schema/item-authoring/lower_item_name_packet.py"
PROOF = "docs/agents/evidence/OTV2-20261002-item-name-source-qualification-v1.json"
PROOF_SHA = "9a22bb2e4238c42ebcbd5714d6c6f08c39f5a140ae67ee61c78223d7952ca7fc"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261002-item-name-promotion-v1.json"


def witness_params(source, indexed, pages, cutoff):
    """Actual own-ID witnesses, complete opposition, and normal literal name guard."""
    iid, parsed, observed = source["source_item_id"], [], set()
    for witness in source["sources"]:
        pid, bi = witness["page_id"], witness["box_index"]
        page = pages[pid]
        if (
            (pid, bi) in observed
            or any(
                witness[k] != page[k]
                for k in (
                    "revision_id",
                    "revision_timestamp",
                    "content_sha256",
                    "source_part",
                    "source_part_sha256",
                    "source_page_ordinal",
                )
            )
            or page["revision_timestamp"] > cutoff
        ):
            raise ValueError("name source coordinate/cutoff drift")
        observed.add((pid, bi))
        raw = witness["raw_infobox"]
        if (
            base.sha(raw.encode()) != witness["raw_infobox_sha256"]
            or not witness["balanced"]
            or witness["inside_comment"]
            or not witness["positive_exact_infobox_object_match"]
        ):
            raise ValueError("raw own-Object witness drift")
        params = base.raw_parameters(raw)
        box = next(b for b in page["own_objects"] if b["box_index"] == bi)
        if (
            params.get("itemid") != [str(iid)]
            or params["itemid"] != box["raw_itemid_values"]
            or any(len(params.get(k, [])) > 1 for k in ("name", "actualname"))
        ):
            raise ValueError("complete singleton own identity/duplicate name required")
        names = (
            params.get("actualname")
            if any(params.get("actualname", []))
            else params.get("name", [])
        )
        if (
            len(names) != 1
            or names[0].strip().casefold() != source["official_name"].strip().casefold()
        ):
            raise ValueError("actual/official source disagreement")
        parsed.append(params)
    expected = {
        (pid, b["box_index"])
        for pid in indexed[iid]
        for b in pages[pid]["own_objects"]
        if any(str(iid) in re.findall(r"\d+", v) for v in b["raw_itemid_values"])
    }
    if not observed or observed != expected:
        raise ValueError("complete own-ID priority drift")
    return parsed


def load_inputs(root, proof):
    """Current exact targets and official/source witnesses, without name-based identity."""
    for path, digest in proof["input_digests"].items():
        base.checked(root, path, digest)
    frame = proof["global_own_id_frame"]
    indexed, pages = global_frame.own_index(
        json.loads(base.checked(root, frame["path"], frame["sha256"]))
    )
    bindings = base.exact_bindings(
        json.loads((root / base.BINDINGS).read_text())["bindings"],
        proof["source_revisions"],
    )
    definitions = {}
    for shard in json.loads((root / "content/items/index.json").read_text())["shards"]:
        for row in json.loads((root / shard).read_text())["records"]:
            definition = row["definition"]
            key = definition["identity"]["key"]
            if key in definitions:
                raise ValueError("duplicate current Item identity")
            definitions[key] = definition
    objects = {}
    for tag, raw in protobuf_fields(base.checked(root, base.CLIENT, base.CLIENT_SHA)):
        if tag == 1:
            obj = decode_appearance_object(raw)
            if obj["id"] in objects:
                raise ValueError("duplicate current official identity")
            objects[obj["id"]] = obj | {"raw_sha256": base.sha(raw)}
    membership = json.loads(
        (
            root
            / f"imports/official/appearance-membership/appearances-{base.CLIENT_SHA}.json"
        ).read_text()
    )["entries"]
    owners = json.loads(
        (root / "content/world/definitions/declarations.json").read_text()
    )["item_authoring"]
    routed, maps = set(), {}
    for family in ("objects", "terrain"):
        for path in sorted((root / f"content/world/{family}").glob("*.json")):
            maps[str(path.relative_to(root))] = base.sha(path.read_bytes())
            routed.update(
                r["provenance"]["item_pointer"]["key"]
                for r in json.loads(path.read_text()).get("records", [])
                if r.get("provenance", {}).get("item_pointer")
            )
    for source in proof["records"]:
        iid, target = source["source_item_id"], source["target"]
        binding, obj = bindings.get(target["key"]), objects[iid]
        if (
            not binding
            or binding != source["binding"]
            or binding["target"] != target
            or binding["external_id"] != str(iid)
            or definitions.get(target["key"], {}).get("identity") != target
            or target["family"] != "Item"
            or target["revision"] != "definition-r1"
            or target["key"] in routed
        ):
            raise ValueError("current exact Item/World identity drift")
        if (
            obj.get("name") != source["official_name"]
            or obj["raw_sha256"] != source["official_object_sha256"]
            or [e[2] for e in membership if e[0] == iid] != [obj["raw_sha256"]]
            or obj["flags"].get("flags.take") is not True
            or any(obj["flags"].get(k) is True for k in (*WORLD_FLAGS, "flags.unmove"))
        ):
            raise ValueError("current portable official raw membership drift")
        witness_params(source, indexed, pages, proof["qualification_cutoff"])
    return definitions, owners, maps, indexed, pages


def qualify(source, definition, owners):
    target, incoming = source["target"], source["facts"]["name"]
    previous = source["imported_name_proof"]["typed_value"]["value"]
    matched = [o for o in owners if o["item"]["key"] == target["key"]]
    if len(matched) > 1 or any(
        o["item"] != target or o.get("presentation") is not None for o in matched
    ):
        raise ValueError("name authoring identity/duplicate/GameOwned guard")
    name = global_frame.leaf(definition, "presentation.name")
    if incoming != source["official_name"] or name not in (
        {"state": "KNOWN", "value": previous},
        {"state": "KNOWN", "value": incoming},
    ):
        raise ValueError("closed imported Known-name exception")
    return {
        "target": target,
        "previous_imported_name": previous,
        "name": incoming,
    }


def build(root=ROOT):
    proof = json.loads(base.checked(root, PROOF, PROOF_SHA))
    definitions, owners, maps, _, _ = load_inputs(root, proof)
    imported = json.loads(
        (
            root
            / "tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json"
        ).read_text()
    )["promotions"]
    aliases = json.loads((root / "content/items/aliases.json").read_text())["entries"]
    rows, seen = [], set()
    for source in proof["records"]:
        iid, target, old = (
            source["source_item_id"],
            source["target"],
            source["imported_name_proof"],
        )
        if (
            iid in seen
            or [
                r
                for r in imported
                if r["field_path"] == "presentation.name" and r["source_item_id"] == iid
            ]
            != [old]
            or old["typed_value"]["kind"] != "TEXT"
            or old["typed_value"]["value"] not in ("weapon of carving", "event item")
            or source["protected_source_xml_witness"]["name_literal"]
            != old["typed_value"]["value"]
            or [
                a["target"]
                for a in aliases
                if a["key"] == old["native_key"] and a["state"] == "ALIAS"
            ]
            != [target["key"]]
        ):
            raise ValueError("closed historical imported literal/alias drift")
        seen.add(iid)
        rows.append(qualify(source, definitions[target["key"]], owners))
    if len(rows) != 139 or any(set(r["facts"]) != {"name"} for r in proof["records"]):
        raise ValueError("closed139 name scope drift")
    return {
        "schema": "OTERYN_ITEM_IMPORTED_NAME_PROMOTION/v1",
        "compiler": {
            "path": COMPILER,
            "sha256": base.sha((root / COMPILER).read_bytes()),
        },
        "sources": {
            "proof_path": PROOF,
            "proof_sha256": PROOF_SHA,
            "world_owner_inputs": maps,
        },
        "counts": {"fields": 139, "items": 139},
        "promotions": rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = (
        json.dumps(build(), sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("name packet drift")
    else:
        OUTPUT.write_bytes(data)
    print("closed139 names/139Items")


if __name__ == "__main__":
    main()
