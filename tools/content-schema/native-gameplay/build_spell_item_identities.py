"""Bind authored Item references to existing source identity rows.

This importer does not allocate canonical identities or grant materialization.
It consumes the admitted Crystal binding table and exact source XML. The
resulting explicit row becomes part of the native artifact's hashed payload.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import subprocess
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
BINDINGS = ROOT / "imports/crystalserver/bindings/items.json"
BINDINGS_SHA = "1d19709c28a649d8a9f32bf53e5bf43ad847d7901d6c6254833f456154d2de5a"
CRYSTAL_PIN = "ff7ede593c69d4c658b382c97443e8155926924a"
CANARY_PIN = "99902524e052f37574194466c2949c576e4ab269"
XML_PATH = "data/items/items.xml"
XML_SHA = {
    "canary": "bf773d2c8f1d2767b31a20749fd5278ae48f2ed5b05757a678e3788e2a70bf85",
    "crystal": "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb",
}


def binding_index(data: bytes, expected_sha: str = BINDINGS_SHA) -> dict:
    if hashlib.sha256(data).hexdigest() != expected_sha:
        raise ValueError("Item binding table digest mismatch")
    document = json.loads(data)
    if set(document) != {"schema", "family", "bindings"} or document["schema"] != "OTERYN_SOURCE_IDENTITY_BINDINGS/v1" or document["family"] != "Item":
        raise ValueError("Invalid Item binding document")
    index, targets = {}, set()
    for row in document["bindings"]:
        if set(row) != {"source_key", "source_revision", "identity_namespace", "external_id", "target", "disposition"}:
            raise ValueError("Unexpected binding fields")
        external = row["external_id"]
        target = row["target"]
        if (row["source_key"] != "oteryn:source.crystalserver" or not re.fullmatch(r"[0-9a-f]{40}", row["source_revision"])
                or row["identity_namespace"] != "ots/item_server_id" or row["disposition"] != "EXACT"
                or not isinstance(external, str) or not re.fullmatch(r"[1-9][0-9]*", external)
                or int(external) > 0xffffffff
                or target != {"family": "Item", "key": f"oteryn:item.tibia.i{external}", "revision": "definition-r1"}
                or external in index or target["key"] in targets):
            raise ValueError("Invalid or ambiguous exact Item binding")
        index[external] = copy.deepcopy(row)
        targets.add(target["key"])
    if not index:
        raise ValueError("Empty Item binding table")
    return index


def source_xml(root: Path, server: str) -> tuple[set[int], dict]:
    pin = {"canary": CANARY_PIN, "crystal": CRYSTAL_PIN}[server]
    actual = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    if actual != pin:
        raise ValueError(f"{server} source HEAD mismatch")
    data = subprocess.check_output(["git", "show", f"{pin}:{XML_PATH}"], cwd=root)
    digest = hashlib.sha256(data).hexdigest()
    if digest != XML_SHA[server]:
        raise ValueError(f"{server} source XML digest mismatch")
    ids = set()
    for node in ET.fromstring(data):
        low = int(node.get("id", node.get("fromid", "-1")))
        high = int(node.get("id", node.get("toid", "-1")))
        if not (0 < low <= 65535 and 0 < high <= 65535):
            raise ValueError("Invalid source Item range")
        # Items::loadFromXml uses `while (id <= toId)`. The pinned files
        # contain reversed ranges; they contribute no engine Item policy.
        if low > high:
            continue
        ids.update(range(low, high+1))
    return ids, {"server": server, "revision": pin, "path": XML_PATH, "sha256": digest}


def bind_profiles(document: dict, bindings: dict, source_ids: set[int], candidate: dict | None = None) -> dict:
    if document.get("schema") != "OTERYN_NATIVE_ITEM_PROFILES/v1" or not isinstance(document.get("records"), list):
        raise ValueError("Invalid native Item profile document")
    result = copy.deepcopy(document)
    bindings = copy.deepcopy(bindings)
    if candidate is not None:
        from canary_generic_item_identity import PACKET
        if candidate != json.loads(PACKET.read_bytes()):
            raise ValueError("Unqualified generic Item candidate packet")
        for binding in candidate["bindings"]:
            external = binding["external_id"]
            if external in bindings:
                raise ValueError("Candidate identity conflicts with existing binding")
            bindings[external] = binding
    seen = set()
    for record in result["records"]:
        item = record["authoring"]["item"]
        if item.get("family") != "Item" or not isinstance(item.get("revision"), str) or not item["revision"]:
            raise ValueError("Invalid authored Item reference")
        key = item.get("key", "")
        match = re.fullmatch(r"(?:candidate:item/|oteryn:item\.tibia\.i)([1-9][0-9]*)", key)
        if not match or match[1] not in bindings:
            raise ValueError(f"Unbound authored Item reference: {key}")
        external = match[1]
        if external in seen:
            raise ValueError("Duplicate production Item target")
        seen.add(external)
        binding = bindings[external]
        generic = external == "40450" and candidate is not None
        if not generic and binding["source_revision"] != CRYSTAL_PIN:
            raise ValueError("Selected Item identity requires independently qualified source revision")
        if generic:
            policy = candidate["qualification"]["policy"]
            if record["admission"] != {name: policy[name] for name in ("materializable", "stack_class", "legal_destinations")}:
                raise ValueError("Generic source Item admission exceeds exact factory policy")
            known = lambda value: {"state": "KNOWN", "value": value}
            semantics = record["semantics"]
            if (semantics.get("physical") != known({"weight": known(0), "movable": known(False), "pickupable": known(False)})
                or semantics.get("stack") != known({"stackable": known(False), "stack_max": {"state": "NOT_APPLICABLE"}})):
                raise ValueError("Generic source Item physical/stack policy mismatch")
            if (record["attributes"].get("speed_bonus") != 0 or record["attributes"].get("field_condition") is not None
                or any(record["attributes"].get(flag) is not False for flag in
                    ("blocks_movement", "blocks_projectile", "immovable_block_solid"))):
                raise ValueError("Generic source Item attributes exceed exact loader defaults")
        elif record["admission"]["materializable"] and int(external) not in source_ids:
            raise ValueError(f"Item {external} has no qualified XML materialization policy")
        target = binding["target"]
        definition = {"family": "Item", "production_key": target["key"], "revision_ref": target["revision"]}
        for field, value in (("production_binding", binding), ("production_definition", definition),
            ("production_binding_qualification", candidate if generic else None)):
            if field in record and record[field] != value:
                raise ValueError("Conflicting existing production binding")
            record[field] = copy.deepcopy(value)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--canary", type=Path, default=Path("/workspace/spell-sources/canary"))
    parser.add_argument("--crystal", type=Path, default=Path("/workspace/spell-sources/crystal"))
    parser.add_argument("--evidence", type=Path)
    args = parser.parse_args()
    bindings = binding_index(BINDINGS.read_bytes())
    canary, canary_source = source_xml(args.canary, "canary")
    crystal, crystal_source = source_xml(args.crystal, "crystal")
    from canary_generic_item_identity import candidate
    generic = candidate(args.canary)
    result = bind_profiles(json.loads(args.input.read_bytes()), bindings, canary & crystal, generic)
    encoded = (json.dumps(result, indent=2, ensure_ascii=False)+"\n").encode()
    args.output.write_bytes(encoded)
    if args.evidence:
        args.evidence.write_text(json.dumps({"schema": "OTERYN_SPELL_ITEM_IDENTITY_IMPORT_EVIDENCE/v1",
            "binding_table": {"path": str(BINDINGS.relative_to(ROOT)), "sha256": BINDINGS_SHA},
            "sources": [canary_source, crystal_source], "output_sha256": hashlib.sha256(encoded).hexdigest(),
            "candidate_generic_source": generic["qualification"],
            "records": len(result["records"]), "scope": "Explicit existing identity rows plus source-qualified local candidate40450; source membership does not grant gameplay, current authority or production activation."}, indent=2)+"\n")


if __name__ == "__main__":
    main()
