"""WRITE-CONTENT-1: bounded document facts from the two owner-pinned sources.

This layer qualifies text semantics, never ItemInstance mint/custody permission.
Decision 25 makes transform targets fixed. Zero source lengths and unresolved
transform mappings remain evidence, never invented limits or transform targets.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET

import readable_item_profiles as reading

ROOT = Path(__file__).resolve().parents[2]
PATH = "content/items/writable-profiles.json"
CRYSTAL_PATH = "imports/crystalserver/writable-item-facts.json"
CRYSTAL_SHA256 = "d1d738adf2d3626219d02e0c9408a3c459552d7956dd97c8c631d4640cd1c55f"
MAX_CHARACTERS = 4_000  # WRITE0-RL-01; owner reserved row 1600.
FIELDS = {"readable", "writeable", "maxtextlen", "writeonceitemid"}
PRESERVED_NONWRITABLE = {*range(173, 180), 6533}
CANARY_LENGTH_OVERRIDES = {27932, 27933, 27934}
# The two-way source cycle needs a direction before either definition can be
# promoted as a writable source with a nonwritable target.
CYCLIC_BOOKS = {2816, 2837}
# Decision 25's identified rewrite items, plus the already accepted editable
# blackboards. Other source-writable items need a fixed target; an absent
# writeonceitemid must never silently turn a card/book/trophy into a rewrite.
REWRITE_IDS = {2598, 2605, 2813, 2814, 2815, 3505, 3507, 7879}


def source_facts(raw, expected_digest):
    if hashlib.sha256(raw).hexdigest() != expected_digest:
        raise ValueError("WRITABLE_SOURCE_DIGEST")
    result = {}
    for item in ET.fromstring(raw).findall("item"):
        facts = {}
        readable = False
        for attr in item.findall("attribute"):
            field = attr.get("key", "").lower()
            if field not in FIELDS:
                continue
            if field in facts:
                raise ValueError("WRITABLE_DUPLICATE_ATTRIBUTE")
            facts[field] = int(attr.get("value"))
            if field in ("readable", "writeable"):
                readable = facts[field] == 1
        if any(facts.get(field, 0) not in (0, 1) for field in ("readable", "writeable")):
            raise ValueError("WRITABLE_BOOLEAN")
        if facts.get("writeable") != 1 and facts.get("readable") != 1:
            continue
        if not 0 <= facts.get("maxtextlen", 0) <= MAX_CHARACTERS:
            raise ValueError("WRITABLE_MAX_CHARACTERS")
        low = int(item.get("id", item.get("fromid", "0")))
        high = int(item.get("id", item.get("toid", str(low))))
        if not 0 < low <= high <= 65535:
            raise ValueError("WRITABLE_ID_RANGE")
        for number in range(low, high + 1):
            if number in result:
                raise ValueError("WRITABLE_DUPLICATE_ITEM")
            result[number] = {"name": item.attrib["name"], "source_facts": facts.copy(),
                              "effective_readable": readable}
    return result


def outputs(root=ROOT):
    canary = source_facts((root / reading.SOURCE).read_bytes(), reading.SOURCE_SHA256)
    crystal_raw = (root / CRYSTAL_PATH).read_bytes()
    if hashlib.sha256(crystal_raw).hexdigest() != CRYSTAL_SHA256:
        raise ValueError("WRITABLE_CRYSTAL_DIGEST")
    crystal = json.loads(crystal_raw)
    other = {r["id"]: r["facts"] for r in crystal["records"]}
    rows = reading.item_rows(root)
    fixed_targets = {r["source_facts"]["writeonceitemid"] for number, r in canary.items()
                     if r["source_facts"].get("writeonceitemid")
                     and number not in CYCLIC_BOOKS | {2813}
                     and f'oteryn:item.tibia.i{r["source_facts"]["writeonceitemid"]}' in rows}
    records = []
    for number, source in sorted(canary.items()):
        facts = source["source_facts"]
        if other.get(number) != facts:
            raise ValueError(f"WRITABLE_REFERENCE_DISAGREEMENT:{number}")
        item = rows[f"oteryn:item.tibia.i{number}"]
        target = facts.get("writeonceitemid", 0)
        if number == 2813:
            # Decision 25 explicitly lists Blank Paper as rewritable. Only
            # Blank Parchment 2817 has the explicit Canary exception.
            target = 0
        maximum = facts.get("maxtextlen", 0)
        writable = facts.get("writeable", 0) == 1
        # Canary parseWriteables assigns readable = writeable in XML order.
        readable = source["effective_readable"]
        if number in CYCLIC_BOOKS:
            qualification = "PENDING_WRITE_ONCE_RULING"
        elif number in PRESERVED_NONWRITABLE:
            qualification = "PRESERVED_NONWRITABLE"
        elif number in fixed_targets:
            qualification = "FIXED_TEXT_SEMANTICS_ONLY"
            writable = False
            target = 0
        elif target:
            qualification = ("WRITE_ONCE_TEXT_SEMANTICS_ONLY"
                             if f"oteryn:item.tibia.i{target}" in rows
                             else "MISSING_WRITE_ONCE_TARGET")
        elif not writable or not readable or maximum == 0:
            qualification = "READ_ONLY_OR_ZERO_LIMIT"
        elif number not in REWRITE_IDS:
            qualification = "UNRESOLVED_WRITE_ONCE_TARGET"
        else:
            qualification = "REWRITE_TEXT_SEMANTICS_ONLY"
        old_group = item.get("semantics", {}).get("readable_writeable", {})
        if qualification == "REWRITE_TEXT_SEMANTICS_ONLY":
            if old_group.get("state") == "KNOWN":
                try:
                    check = copy.deepcopy(old_group["value"])
                    for field, value in [("readable", True), ("writeable", True),
                                         ("max_text_length", maximum), ("write_once_target", None)]:
                        qualify_leaf(check, field, value)
                except ValueError:
                    qualification = "EXISTING_SEMANTICS_CONFLICT"
            elif old_group.get("state") not in (None, "UNKNOWN"):
                qualification = "EXISTING_SEMANTICS_CONFLICT"
        target_identity = rows.get(f"oteryn:item.tibia.i{target}", {}).get("identity") if target else None
        records.append({
            "item": item["identity"], "name": source["name"],
            "readable": readable, "writable": writable,
            "write_policy": ("write_once" if target or qualification == "UNRESOLVED_WRITE_ONCE_TARGET"
                             else "rewrite" if writable else None),
            "max_characters": maximum, "write_once_to": target_identity,
            "source_write_once_id": facts.get("writeonceitemid") or None,
            "qualification": qualification,
        })
    # Some fixed targets have no readable/writeable source attribute and are
    # therefore absent from source_facts. The transform itself supplies the
    # carried text, and decision 25 supplies their nonwritable fact.
    present = {r["item"]["key"] for r in records}
    for number in sorted(fixed_targets):
        key = f"oteryn:item.tibia.i{number}"
        if key not in present:
            records.append({"item": rows[key]["identity"], "name": None,
                            "readable": True, "writable": False, "write_policy": None,
                            "max_characters": 0, "write_once_to": None,
                            "source_write_once_id": None,
                            "qualification": "FIXED_TEXT_SEMANTICS_ONLY"})
    records.sort(key=lambda record: record["item"]["key"])
    return {PATH: reading.encode({
        "schema": "OTERYN_WRITABLE_ITEM_PROFILES/v1",
        "sources": [
            {"path": reading.SOURCE, "revision": reading.REVISION, "sha256": reading.SOURCE_SHA256},
            {**crystal["source"], "capture": CRYSTAL_PATH, "capture_sha256": CRYSTAL_SHA256},
        ],
        "parity": "not verified against Global",
        "records": records,
    })}


def known(value):
    return {"state": "KNOWN", "value": value}


def qualify_leaf(group, field, value):
    expected = known(value) if value is not None else {"state": "NOT_APPLICABLE"}
    old = group.get(field, {"state": "UNKNOWN"})
    if old not in ({"state": "UNKNOWN"}, expected):
        raise ValueError("WRITABLE_LEAF_CONFLICT")
    group[field] = expected


def owner_replace(group, field, expected, allowed):
    if group.get(field, {"state": "UNKNOWN"}) not in allowed:
        raise ValueError("WRITABLE_OWNER_REPLACEMENT_CONFLICT")
    group[field] = expected


def apply_native(root, generated, profile_path, manifest_path, order, canonical):
    expected = outputs(root)[PATH]
    if (root / PATH).read_bytes() != expected:
        raise ValueError("WRITABLE_PROFILE_DRIFT")
    document = json.loads(generated[profile_path])
    policies = {r["production_definition"]["production_key"]: r for r in document["records"]}
    rows = reading.item_rows(root)
    bindings = {r["target"]["key"]: r for r in json.loads(
        (root / "imports/crystalserver/bindings/items.json").read_bytes())["bindings"]}
    for profile in json.loads(expected)["records"]:
        if profile["qualification"] not in {"REWRITE_TEXT_SEMANTICS_ONLY",
                                           "WRITE_ONCE_TEXT_SEMANTICS_ONLY",
                                           "FIXED_TEXT_SEMANTICS_ONLY",
                                           "UNRESOLVED_WRITE_ONCE_TARGET"}:
            continue
        identity = profile["item"]
        key = identity["key"]
        source, binding = rows[key], bindings[key]
        if binding["target"] != identity or binding["disposition"] != "EXACT":
            raise ValueError("WRITABLE_BINDING")
        if key not in policies:
            materializable = source["materializable"]
            if materializable:
                physical = source["semantics"].get("physical", {})
                if (source.get("stack_class") != "NonStackable" or physical.get("state") != "KNOWN"
                        or any(physical["value"][field] != known(True) for field in ("movable", "pickupable"))):
                    raise ValueError("WRITABLE_UNOWNED_ADMISSION")
            # Same nonmaterializable carrier as the READ-WRITE-1a sign layer.
            # Unknown stack/physics facts stay unknown. This is not mint authority.
            record = {
                "authoring": {"item": identity},
                "semantics": copy.deepcopy(source.get("semantics", {})), "attributes": {},
                "admission": {"materializable": materializable, "stack_class": "NonStackable",
                              "legal_destinations": ["CharacterInventory", "Ground"] if materializable else ["Ground"]},
                "production_definition": {"family": "Item", "production_key": key,
                                          "revision_ref": identity["revision"]},
                "production_binding": binding, "production_binding_qualification": None,
            }
            document["records"].append(record)
        else:
            record = policies[key]
        if record["admission"]["stack_class"] != "NonStackable":
            raise ValueError("WRITABLE_STACKABLE")
        stack = record["semantics"].get("stack", {"state": "UNKNOWN"})
        if stack["state"] == "KNOWN" and stack["value"]["stackable"] == known(True):
            raise ValueError("WRITABLE_STACKABLE")
        if record["admission"]["materializable"] and (
                stack["state"] != "KNOWN" or stack["value"]["stackable"] != known(False)):
            raise ValueError("WRITABLE_UNKNOWN_STACK")
        group = record["semantics"].get("readable_writeable", {"state": "UNKNOWN"})
        if group["state"] == "UNKNOWN":
            group = known({field: {"state": "UNKNOWN"} for field in
                           ("readable", "writeable", "distance_read", "max_text_length", "write_once_target")})
        if group["state"] != "KNOWN":
            raise ValueError("WRITABLE_GROUP_CONFLICT")
        number = int(key.rsplit("i", 1)[1])
        # These are the exact owner-authorized replacements of promoted source
        # facts. All other conflicting leaves still fail atomically.
        if profile["qualification"] == "FIXED_TEXT_SEMANTICS_ONLY":
            owner_replace(group["value"], "writeable", known(False),
                          [{"state": "UNKNOWN"}, known(True), known(False)])
            qualify_leaf(group["value"], "write_once_target", None)
        if number in CANARY_LENGTH_OVERRIDES:
            owner_replace(group["value"], "max_text_length", known(profile["max_characters"]),
                          [known(1499), known(1500)])
        unresolved = profile["qualification"] == "UNRESOLVED_WRITE_ONCE_TARGET"
        if unresolved:
            # Preserve readable/length evidence, without asserting a writable
            # runtime policy whose required fixed target cannot be qualified.
            owner_replace(group["value"], "writeable", {"state": "UNKNOWN"},
                          [{"state": "UNKNOWN"}, known(True)])
            owner_replace(group["value"], "write_once_target", {"state": "UNKNOWN"},
                          [{"state": "UNKNOWN"}, {"state": "NOT_APPLICABLE"}])
        target = profile["write_once_to"]
        target = {field: target[field] for field in ("key", "revision")} if target else None
        leaves = [("readable", True)]
        if not unresolved:
            leaves.extend([("writeable", profile["writable"]), ("write_once_target", target)])
        if profile["max_characters"]:
            leaves.append(("max_text_length", profile["max_characters"]))
        for field, value in leaves:
            try:
                qualify_leaf(group["value"], field, value)
            except ValueError as error:
                raise ValueError(f"WRITABLE_LEAF_CONFLICT:{key}:{field}") from error
        record["semantics"]["readable_writeable"] = group
    document["records"].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(raw).hexdigest()
    # No partial output if a later row conflicts.
    generated.update({profile_path: raw, manifest_path: canonical(manifest), PATH: expected})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for path, raw in outputs().items():
        if args.check:
            if (ROOT / path).read_bytes() != raw:
                raise SystemExit("WRITABLE_PROFILE_DRIFT")
        else:
            (ROOT / path).write_bytes(raw)
    print("PASS writable Item source profiles; no new mint or command authority")


if __name__ == "__main__":
    main()
