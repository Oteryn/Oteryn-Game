#!/usr/bin/env python3
"""SOULPIT: import bounded core facts through the existing native Item profile importer.

Reads pinned local references only. No original XML, DAT or sprites are redistributed.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path

import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, binding_index, bind_profiles

ROOT = Path(__file__).resolve().parents[3]
CANARY = "47dfd51f45280a59a1d3e50ba7edd573d7234446"
CRYSTAL = "201189f871d23597b82caf88fd611454a2ddefb8"
PROFILES = ROOT / "content/items/definitions/spell-native-profiles.json"
IDS = ROOT / "crates/gameplay/items/src/soul_core_ids.json"
PROOF = Path(__file__).with_name("soul-core-source-proof.json")
MANIFEST = ROOT / "content/spells.manifest.json"
EXPECTED_BLOBS = {
    (CANARY, "data/items/items.xml"):
        "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2",
    (CANARY, "data/items/appearances.dat"):
        "aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50",
    (CRYSTAL, "data/items/items.xml"):
        "8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509",
}


def blob(repo, revision, path):
    # The lane supplies immutable source exports, which need not contain Git metadata.
    if revision not in (CANARY, CRYSTAL):
        raise ValueError("Unallocated source revision")
    raw = (repo / path).read_bytes()
    if hashlib.sha256(raw).hexdigest() != EXPECTED_BLOBS[(revision, path)]:
        raise ValueError(f"Pinned source export drift: {revision}:{path}")
    return raw


def capture(canary, crystal, check):
    xml_path = "data/items/items.xml"
    dat_path = "data/items/appearances.dat"
    xml_raw = blob(canary, CANARY, xml_path)
    dat_raw = blob(canary, CANARY, dat_path)
    crystal_xml = blob(crystal, CRYSTAL, xml_path)
    xml = source.xml_index(xml_raw)
    crystal_index = source.xml_index(crystal_xml)
    bindings = binding_index(BINDINGS.read_bytes())
    keys = {row[0] for row in json.loads((ROOT / "tools/content-schema/native-gameplay/item-keys.json").read_bytes())["records"]}
    candidates = {i for i, node in xml.items()
                  if source.attrs(node).get("primarytype") == "soul cores"
                  and " soul core" in node.get("name", "")
                  and node.get("name") != "ominous soul core"}
    ids = sorted(i for i in candidates if str(i) in bindings
                 and bindings[str(i)]["target"]["key"] in keys and i in crystal_index)
    objects = source.engine_items.load_appearance_objects(dat_raw)
    admitted = set(ids) | {37110, 49163, 49164}
    physical = {row["server_item_id"]: row for row in source.policies(dat_raw, xml_raw, admitted)}
    records = []
    name_differences = []
    weight_differences = []
    for i in sorted(admitted):
        if xml[i].get("name") != crystal_index[i].get("name"):
            name_differences.append({"id": i, "canary": xml[i].get("name"),
                                     "crystal": crystal_index[i].get("name"), "selected": "canary"})
        node = xml[i]
        if source.attrs(node).get("weight") != source.attrs(crystal_index[i]).get("weight"):
            # The Summer 2025 species cores are 10 oz in the Global Wiki, matching Crystal.
            # Preserve the separate Canary appearance flags and explicit source precedence.
            node = copy.deepcopy(node)
            weight = source.attrs(crystal_index[i])["weight"]
            weight_differences.append({"id": i, "canary": source.attrs(xml[i])["weight"],
                                       "crystal": weight, "selected": "crystal"})
            for attribute in node:
                if attribute.get("key") == "weight":
                    attribute.set("value", weight)
        row = source.profile(i, node, objects[i], physical[i], bindings, None)
        if not (row["admission"]["stack_class"] == "StackCapable"
                and row["admission"]["legal_destinations"] == ["CharacterInventory", "Ground"]):
            raise ValueError(f"Unsupported core physical shape: {i}")
        records.append(row)
    bound = bind_profiles({"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": records}, bindings, admitted)
    document = json.loads(PROFILES.read_bytes())
    document["records"] = [row for row in document["records"]
                           if int(row["production_binding"]["external_id"]) not in admitted] + bound["records"]
    encoded = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()
    sources = [{"repository": repo, "revision": revision, "path": path,
                "sha256": hashlib.sha256(raw).hexdigest()}
               for repo, revision, path, raw in [
                   ("opentibiabr/canary", CANARY, xml_path, xml_raw),
                   ("opentibiabr/canary", CANARY, dat_path, dat_raw),
                   ("Crystal (owner-supplied reference)", CRYSTAL, xml_path, crystal_xml)]]
    proof = {"schema": "OTERYN_SOUL_CORE_SOURCE_FACTS/v1", "sources": sources,
             "producer": str(Path(__file__).relative_to(ROOT)), "core_count": len(ids), "special_items": [37110, 49163, 49164],
             "excluded_unqualified_ids": sorted(candidates - set(ids)),
             "name_differences": name_differences, "weight_differences": weight_differences,
             "weight_sources": ["https://tibia.fandom.com/wiki/Ink_Splash_Soul_Core"],
             "scope": "680 admitted species-core identities and physical Item facts, with pinned name/weight precedence. No arena or mastery activation.",
             "excluded_identity_evidence": [{"id": 47899, "name": "Falcon Knight Soul Core",
                 "global_url": "https://tibia.fandom.com/wiki/Item_IDs",
                 "reason": "Present in Canary and Global Wiki but absent in pinned Crystal XML and current canonical Item binding index. Requires owning Item identity admission rather than fabricating an alias."}]}
    outputs = {PROFILES: encoded, IDS: (json.dumps(ids, indent=2) + "\n").encode(),
               PROOF: (json.dumps(proof, indent=2) + "\n").encode()}
    manifest = json.loads(MANIFEST.read_bytes())
    manifest["item_profiles"]["sha256"] = hashlib.sha256(encoded).hexdigest()
    outputs[MANIFEST] = (json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n").encode()
    for path, raw in outputs.items():
        if check:
            if path.read_bytes() != raw:
                raise ValueError(f"Stale Soulpit output: {path.relative_to(ROOT)}")
        else:
            path.write_bytes(raw)
    print(f"SOULPIT core profiles: {len(ids)}; excluded unqualified: {len(candidates - set(ids))}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--canary", type=Path, required=True)
    parser.add_argument("--crystal", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    capture(args.canary, args.crystal, args.check)
