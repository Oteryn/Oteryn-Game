"""Correct only15 proven imported mayhem names using own numeric official objects."""

import argparse
import json
import xml.etree.ElementTree as ET

import appearance_membership as membership
import lower_wiki_movable_packet as movable
import lower_wiki_stack_default_packet as base
from engine_items import decode_appearance_object, protobuf_fields
from lower_client_market_packet import WORLD_FLAGS

ROOT = base.ROOT
COMPILER = "tools/content-schema/item-authoring/lower_item_name15_packet.py"
PROOF = "docs/agents/evidence/OTV2-20261002-item-name15-source-qualification-v1.json"
PROOF_SHA = "4f9de51763e524148ab41e2f34a4ecfa51059a597c56fea6df0c3f07d6b7c8ed"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261002-item-name15-promotion-v1.json"
IDS = (
    23577,
    23583,
    23589,
    23596,
    23605,
    23609,
    23619,
    23624,
    23638,
    23641,
    23644,
    23656,
    23659,
    23662,
    23665,
)
HEADERS = ("kind", "client_projection", "materializable", "stack_class")


def official(raw, source):
    fields = list(protobuf_fields(raw))
    names = [v for tag, v in fields if tag == 4]
    ids = [v for tag, v in fields if tag == 1]
    value = decode_appearance_object(raw)
    if (
        ids != [source["source_item_id"]]
        or len(names) != 1
        or names[0].decode("utf-8", "strict") != source["name"]
        or value.get("name") != source["name"]
        or base.sha(raw) != source["official_object_sha256"]
        or value["flags"].get("flags.take") is not True
        or any(
            value["flags"].get(k) is True
            for k in (
                *WORLD_FLAGS,
                "flags.unmove",
                "flags.ground",
                "flags.border",
                "flags.bottom",
                "flags.top",
            )
        )
    ):
        raise ValueError("own numeric official literal/domain drift")
    return value


def qualify(source, definition, owners):
    target, name = source["target"], source["name"]
    iid = source["source_item_id"]
    if (
        type(iid) is not int
        or iid not in IDS
        or target
        != {
            "family": "Item",
            "key": f"oteryn:item.tibia.i{iid}",
            "revision": "definition-r1",
        }
        or definition.get("identity") != target
        or membership.canonical_bytes({k: definition.get(k) for k in HEADERS})
        != membership.canonical_bytes(source["native_header_guard"])
        or source["previous_imported_name"] != "weapon of mayhem"
        or not name
        or name != name.strip()
        or len(name.encode()) > 46
        or any(ord(c) < 32 or ord(c) == 127 for c in name)
    ):
        raise ValueError("closed15 identity/header/imported literal/value drift")
    matching = [o for o in owners if o["item"]["key"] == target["key"]]
    if len(matching) > 1 or any(
        o["item"] != target or o.get("presentation") is not None for o in matching
    ):
        raise ValueError("current GameOwned presentation/duplicate owner conflict")
    if movable.leaf(definition, "presentation.name") not in (
        {"state": "KNOWN", "value": "weapon of mayhem"},
        {"state": "KNOWN", "value": name},
    ):
        raise ValueError("only exact proven imported or identical Known name allowed")
    return {
        "target": target,
        "headers": source["native_header_guard"],
        "previous_imported_name": "weapon of mayhem",
        "name": name,
    }


def load_inputs(root, proof):
    for path, digest in proof["input_digests"].items():
        base.checked(root, path, digest)
    historical = proof["historical_source"]
    old = json.loads(base.checked(root, historical["path"], historical["sha256"]))
    if (
        old["baseline"] != proof["authoring_baseline"]
        or proof["current_parent_receipt"]["commit"] != proof["authoring_baseline"]
    ):
        raise ValueError("historical/current receipt authority drift")
    frame = proof["global_own_id_frame"]
    indexed, _ = movable.own_index(
        json.loads(base.checked(root, frame["path"], frame["sha256"]))
    )
    if any(indexed.get(i) for i in IDS):
        raise ValueError(
            "own Wiki identity now present; official-only exception not applicable"
        )
    bindings = json.loads((root / base.BINDINGS).read_text())["bindings"]
    exact = base.exact_bindings(bindings, proof["source_revisions"])
    admitted, manifests = membership.load_admitted(
        out_dir=root / "imports/official/appearance-membership"
    )
    if (
        admitted["newest"] != "client-15.30"
        or admitted["files"][-1]["appearances_sha256"] != base.CLIENT_SHA
    ):
        raise ValueError("newest official membership drift")
    members = {k: {r[0]: r for r in v["entries"]} for k, v in manifests.items()}
    official_raw = {}
    for tag, raw in protobuf_fields(base.checked(root, base.CLIENT, base.CLIENT_SHA)):
        if tag == 1:
            obj = decode_appearance_object(raw)
            if type(obj["id"]) is not int or obj["id"] in official_raw:
                raise ValueError("whole official numeric identity uniqueness drift")
            official_raw[obj["id"]] = raw
    if len(official_raw) != 43516:
        raise ValueError("whole official object scope drift")
    definitions = {}
    for path in json.loads((root / "content/items/index.json").read_text())["shards"]:
        shard = json.loads((root / path).read_text())
        if (
            shard["schema"] != "OTERYN_ITEM_AUTHORING_SHARD/v1"
            or shard["family"] != "Item"
        ):
            raise ValueError("native shard family/schema drift")
        for row in shard["records"]:
            d = row["definition"]
            key = d["identity"]["key"]
            if key in definitions:
                raise ValueError("duplicate current Item")
            definitions[key] = d
    owners = json.loads(
        (root / "content/world/definitions/declarations.json").read_text()
    )["item_authoring"]
    routed, maps = set(), {}
    for family in ("objects", "terrain"):
        for path in sorted((root / f"content/world/{family}").glob("*.json")):
            maps[str(path.relative_to(root))] = base.sha(path.read_bytes())
            for row in json.loads(path.read_text()).get("records", []):
                pointer = row.get("provenance", {}).get("item_pointer")
                if pointer:
                    routed.add(pointer["key"])
    return (
        definitions,
        owners,
        maps,
        exact,
        bindings,
        members,
        official_raw,
        routed,
        old,
    )


def build(root=ROOT):
    proof = json.loads(base.checked(root, PROOF, PROOF_SHA))
    definitions, owners, maps, exact, bindings, members, raws, routed, old = (
        load_inputs(root, proof)
    )
    imported = json.loads(
        (
            root
            / "tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json"
        ).read_text()
    )
    aliases = json.loads((root / "content/items/aliases.json").read_text())["entries"]
    xml = proof["protected_XML_source"]
    if (
        xml != old["protected_XML_source"]
        or base.sha(xml["raw_range"].encode()) != xml["raw_range_sha256"]
        or ET.fromstring(xml["raw_range"]).attrib
        != {"fromid": "23577", "toid": "23667", "name": "weapon of mayhem"}
        or imported["protected_lineage"]["source_artifact_digests"][
            "data/items/items.xml"
        ]["sha256"]
        != xml["whole_raw_sha256"]
        or imported["protected_lineage"]["source_revision"] != xml["source_revision"]
    ):
        raise ValueError("literal protected XML/import lineage drift")
    rows, seen = [], set()
    historical = {r["source_item_id"]: r for r in old["name15"]}
    for source in proof["records"]:
        iid, target, binding = (
            source["source_item_id"],
            source["target"],
            source["binding"],
        )
        if iid in seen or iid not in IDS:
            raise ValueError("duplicate or outside closed15 source")
        seen.add(iid)
        prior = historical[iid]
        if (
            prior["target"] != target
            or prior["facts"] != {"name": source["name"]}
            or prior["current"]["binding"] != binding
            or prior["imported_name_proof"] != source["imported_name_proof"]
            or prior["own_Wiki_target_ID_set"] != []
            or exact.get(target["key"]) != binding
            or binding["target"] != target
            or binding["external_id"] != str(iid)
            or [b for b in bindings if b["target"]["key"] == target["key"]] != [binding]
            or [
                b
                for b in bindings
                if all(
                    b[k] == binding[k]
                    for k in (
                        "source_key",
                        "source_revision",
                        "identity_namespace",
                        "external_id",
                    )
                )
            ]
            != [binding]
            or target["key"] in routed
        ):
            raise ValueError("historical/full forward-reverse identity/World drift")
        current = members["client-15.30"].get(iid)
        bound = members["crystal-ff7ede5"].get(iid)
        if (
            not current
            or not bound
            or current[1] != bound[1]
            or current[1] != source["official_identity_projection_sha256"]
            or current[2] != source["official_object_sha256"]
        ):
            raise ValueError("current/bound membership drift")
        official(raws[iid], source)
        prior_import = source["imported_name_proof"]
        if (
            [
                r
                for r in imported["promotions"]
                if r["field_path"] == "presentation.name" and r["source_item_id"] == iid
            ]
            != [prior_import]
            or prior_import["typed_value"]
            != {"kind": "TEXT", "value": "weapon of mayhem"}
            or prior_import["source_value"] != "weapon of mayhem"
            or [
                a["target"]
                for a in aliases
                if a["key"] == prior_import["native_key"] and a["state"] == "ALIAS"
            ]
            != [target["key"]]
        ):
            raise ValueError("original exact imported name/alias provenance drift")
        rows.append(qualify(source, definitions[target["key"]], owners))
    if (
        seen != set(IDS)
        or proof["counts"]["items"] != 15
        or proof["counts"]["fields"] != 15
    ):
        raise ValueError("closed15 scope drift")
    return {
        "schema": "OTERYN_ITEM_IMPORTED_NAME15_PROMOTION/v1",
        "compiler": {
            "path": COMPILER,
            "sha256": base.sha((root / COMPILER).read_bytes()),
        },
        "sources": {
            "proof_path": PROOF,
            "proof_sha256": PROOF_SHA,
            "world_owner_inputs": maps,
        },
        "counts": {"items": 15, "fields": 15},
        "promotions": rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = membership.canonical_bytes(build())
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("name15 packet drift")
    else:
        OUTPUT.write_bytes(data)
    print("closed15 official numeric imported-name corrections; family NAV unchanged")


if __name__ == "__main__":
    main()
