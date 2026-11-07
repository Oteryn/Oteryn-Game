#!/usr/bin/env python3
"""READ-WRITE-1a: separate Item reading owner from pinned Canary XML.

Sealed Item shards and materializable admission remain unchanged. Only explicit
XML leaves (plus Canary parseWriteables' writeable -> readable assignment) are
qualified. The closed Look slice excludes flag ids 2018..2020.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
SOURCE = "imports/canary/items-xml/items.xml"
SOURCE_SHA256 = "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2"
REVISION = "47dfd51f45280a59a1d3e50ba7edd573d7234446"
PATH = "content/items/readable-profiles.json"
IDS = {*range(2012, 2018), *range(2021, 2025), 2597, 2598, 2602, 2605,
       6121, 6122, 10322, 12782, 12783}
FIELDS = {"allowdistread": "distance_read", "readable": "readable",
          "writeable": "writeable", "maxtextlen": "max_text_length"}


def encode(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def source_facts(raw):
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError("READABLE_SOURCE_DIGEST")
    result = {}
    for item in ET.fromstring(raw).findall("item"):
        low = int(item.get("id", item.get("fromid", "0")))
        high = int(item.get("id", item.get("toid", str(low))))
        ids = IDS.intersection(range(low, high + 1))
        if not ids:
            continue
        facts, seen = {}, set()
        for attr in item.findall("attribute"):
            key = attr.get("key", "").lower()
            if key not in FIELDS:
                continue
            if key in seen:
                raise ValueError("READABLE_DUPLICATE_ATTRIBUTE")
            seen.add(key)
            value = attr.get("value")
            if key == "maxtextlen":
                value = int(value)
                if not 0 < value <= 65535:
                    raise ValueError("READABLE_TEXT_LENGTH")
            else:
                if value not in {"0", "1"}:
                    raise ValueError("READABLE_BOOLEAN")
                value = value == "1"
            facts[FIELDS[key]] = value
            # Canary item_parse.cpp:263-265, in source attribute order.
            if key == "writeable":
                facts["readable"] = value
        if facts.get("distance_read") is not True:
            raise ValueError("READABLE_DISTANCE_FACT")
        for item_id in ids:
            if item_id in result:
                raise ValueError("READABLE_DUPLICATE_ITEM")
            result[item_id] = {"facts": facts.copy(), "name": item.attrib["name"]}
    if set(result) != IDS:
        raise ValueError("READABLE_MISSING_ITEM")
    return result


def item_rows(root):
    index = json.loads((root / "content/items/index.json").read_bytes())
    return {r["definition"]["identity"]["key"]: r["definition"]
            for path in index["shards"]
            for r in json.loads((root / path).read_bytes())["records"]}


def outputs(root=ROOT):
    facts = source_facts((root / SOURCE).read_bytes())
    rows = item_rows(root)
    records = []
    for item_id in sorted(facts):
        item = rows[f"oteryn:item.tibia.i{item_id}"]
        records.append({"item": item["identity"], **facts[item_id]})
    return {PATH: encode({"schema": "OTERYN_READABLE_ITEM_PROFILES/v1",
                          "source": {"path": SOURCE, "revision": REVISION, "sha256": SOURCE_SHA256},
                          "records": records})}


def apply_native(root, generated, profile_path, manifest_path, order, canonical):
    expected = outputs(root)[PATH]
    if (root / PATH).read_bytes() != expected:
        raise ValueError("READABLE_PROFILE_DRIFT")
    rows = item_rows(root)
    bindings = {r["target"]["key"]: r for r in json.loads(
        (root / "imports/crystalserver/bindings/items.json").read_bytes())["bindings"]}
    document = json.loads(generated[profile_path])
    policies = {r["production_definition"]["production_key"]: r for r in document["records"]}
    for profile in json.loads(expected)["records"]:
        identity, facts = profile["item"], profile["facts"]
        key = identity["key"]
        source = rows[key]
        binding = bindings[key]
        if binding["target"] != identity or binding["disposition"] != "EXACT":
            raise ValueError("READABLE_BINDING")
        # Native admission has no Unknown stack variant. These nonmaterializable
        # placements use its nonstack carrier; no stack fact or mint permission is added.
        if source["materializable"] is not False:
            raise ValueError("READABLE_MATERIALIZABLE_SCOPE")
        if key not in policies:
            record = {"authoring": {"item": identity}, "semantics": copy.deepcopy(source.get("semantics", {})),
                      "attributes": {}, "admission": {"materializable": source["materializable"],
                      "stack_class": "NonStackable", "legal_destinations": ["Ground"]},
                      "production_definition": {"family": "Item", "production_key": key,
                                                "revision_ref": identity["revision"]},
                      "production_binding": binding, "production_binding_qualification": None}
            document["records"].append(record)
        else:
            record = policies[key]
        presentation = record["semantics"].get("presentation", {"state": "UNKNOWN"})
        if presentation["state"] == "UNKNOWN":
            record["semantics"]["presentation"] = {"state": "KNOWN", "value": {
                "name": {"state": "KNOWN", "value": profile["name"]}, "description": {"state": "UNKNOWN"}}}
        elif presentation["state"] != "KNOWN" or presentation["value"]["name"] != {"state": "KNOWN", "value": profile["name"]}:
            raise ValueError("READABLE_PRESENTATION_CONFLICT")
        group = record["semantics"].get("readable_writeable", {"state": "UNKNOWN"})
        if group["state"] == "UNKNOWN":
            group = {"state": "KNOWN", "value": {field: {"state": "UNKNOWN"} for field in
                ("readable", "writeable", "distance_read", "max_text_length", "write_once_target")}}
        if group["state"] != "KNOWN":
            raise ValueError("READABLE_GROUP_CONFLICT")
        for field, value in facts.items():
            old = group["value"].get(field, {"state": "UNKNOWN"})
            if old != {"state": "UNKNOWN"} and old != {"state": "KNOWN", "value": value}:
                raise ValueError("READABLE_LEAF_CONFLICT")
            group["value"][field] = {"state": "KNOWN", "value": value}
        record["semantics"]["readable_writeable"] = group
    document["records"].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()
    generated[profile_path] = raw
    generated[PATH] = expected
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for path, raw in outputs().items():
        if args.check:
            if (ROOT / path).read_bytes() != raw:
                raise SystemExit("READABLE_PROFILE_DRIFT")
        else:
            (ROOT / path).write_bytes(raw)
    print(f"PASS readable Item profiles={len(IDS)}; admission unchanged")


if __name__ == "__main__":
    main()
