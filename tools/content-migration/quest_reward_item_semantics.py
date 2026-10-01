"""Apply digest-bound, data-only reward Item core admissions to the successor tree.

Equipment, use effects, instance text/subtypes and runtime readiness are not inferred.
The admitted 15.30 client proves portability/stack facts; pinned OTS excerpts prove
definition charges and capacities. Proto2 boolean defaults are explicit derivations.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import sys
from pathlib import Path

PACKET = "docs/agents/evidence/OTV2-20261001-quest-reward-item-admission.json"
SNAPSHOT = "imports/tibiawiki/quest-reward-items/fandom-snapshot-r1.json"
EXTRA_PACKET = "docs/agents/evidence/OTV2-20261001-quest-interaction-item-admission.json"
EXTRA_SNAPSHOT = "imports/tibiawiki/quest-reward-items/fandom-snapshot-r2.json"
CLIENT_SHA = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
CLIENT = f"content/assets/files/appearances-{CLIENT_SHA}.dat"
CLIENT_BYTES = 5017996
UNKNOWN = {"state": "UNKNOWN"}
CLAIMS = "tools/content-schema/quest-authoring/samples/chests/claims.json"


def canonical(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def known(value):
    return {"state": "KNOWN", "value": value}


def wiki_object_fields(text):
    start = re.search(r"{{Infobox Object\b", text, re.I)
    if not start:
        return {}
    cursor, depth = start.start(), 0
    lines = []
    while cursor < len(text):
        if depth == 1 and (cursor == 0 or text[cursor - 1] == "\n"):
            lines.append(text[cursor:text.find("\n", cursor) if "\n" in text[cursor:] else len(text)])
        if text.startswith("{{", cursor):
            depth += 1
            cursor += 2
        elif text.startswith("}}", cursor):
            depth -= 1
            cursor += 2
            if depth == 0:
                break
        else:
            cursor += 1
    if depth:
        return {}
    infobox = "\n".join(lines)
    fields = {}
    for field in ("itemid", "stackable", "pickupable", "immobile", "volume", "weight",
                  "name", "actualname", "charges", "slot"):
        match = re.search(r"^\|\s*" + field + r"\s*=([^\n]*)", infobox, re.M | re.I)
        if match:
            fields[field] = match[1].strip()
    return fields


def verify_wiki(observations, pages, item_id):
    for observation in observations:
        page = pages.get((observation["page_id"], observation["revision_id"]))
        if page is None:
            raise ValueError("WIKI_REVISION_MISSING")
        revision = page["revisions"][0]
        text = revision["slots"]["main"]["*"]
        fields = wiki_object_fields(text)
        ids = fields.get("itemid", "")
        if (hashlib.sha256(text.encode()).hexdigest() != observation["content_sha256"] or
                revision["timestamp"] != observation["revision_timestamp"] or
                fields != observation["fields"] or
                not re.fullmatch(r"\d+(?:\s*,\s*\d+)*", ids) or
                item_id not in {int(i) for i in re.findall(r"\d+", ids)}):
            raise ValueError("WIKI_FACT_BINDING")


def agree(rows, field):
    values = [row["attrs"].get(field) for row in rows]
    return values[0] if len(set(values)) == 1 else None


def item_facts(entry, appearance):
    """A missing protobuf boolean uses the pinned proto2 getter default, not a guess."""
    flags = appearance["flags"]
    if entry["client_flags"] != flags:
        raise ValueError("CLIENT_FLAGS_MISMATCH")
    rows = entry["xml"]
    if not rows or not set(rows) <= {"canary", "crystal", "crystal-summer"}:
        raise ValueError("XML_SOURCE_SET")
    if entry.get("owning_donors", sorted(rows)) != sorted(rows):
        raise ValueError("XML_OWNING_DONORS")
    if any(row["item_id"] != entry["item_id"] for row in rows.values()):
        raise ValueError("XML_ID_MISMATCH")
    rows = list(rows.values())
    if not flags.get("flags.take") or flags.get("flags.unmove", False):
        raise ValueError("NON_PORTABLE")
    if appearance.get("name") != entry["client_name"]:
        raise ValueError("CLIENT_NAME_MISMATCH")
    stackable = flags.get("flags.cumulative", False)
    capacity = agree(rows, "containersize")
    charges = agree(rows, "charges")
    hold = []
    if flags.get("flags.liquidcontainer") or flags.get("flags.liquidpool"):
        hold.append("INSTANCE_FLUID_SUBTYPE_LOWERING_MISSING")
    if any(row["attrs"].get("charges") for row in rows) and charges is None:
        hold.append("DEFINITION_CHARGES_CONFLICT")
    if charges and not stackable:
        hold.append("INSTANCE_CHARGE_ATTRIBUTE_LOWERING_MISSING")
    if flags.get("flags.container") and (capacity is None or not str(capacity).isdigit()):
        hold.append("CONTAINER_CAPACITY_UNKNOWN_OR_CONFLICT")
    if capacity is not None and not flags.get("flags.container", False):
        hold.append("CONTAINER_KIND_CLIENT_XML_CONFLICT")
        capacity = None
    wiki_capacities = {int(o["fields"]["volume"]) for o in entry.get("wiki", [])
        if str(o.get("fields", {}).get("volume", "")).isdigit()}
    if capacity is not None and any(value != int(capacity) for value in wiki_capacities):
        hold.append("WIKI_XML_CONTAINER_CAPACITY_CONFLICT")
        capacity = None
    if capacity is not None and not 1 <= int(capacity) <= 65535:
        raise ValueError("CONTAINER_CAPACITY_RANGE")
    if charges is not None and not 1 <= int(charges) <= 4294967295:
        raise ValueError("CHARGES_RANGE")
    facts = {
        "stackable": stackable,
        "capacity": int(capacity) if capacity is not None else None,
        "charges": int(charges) if charges is not None else None,
        "holds": hold,
    }
    if "native_core_hold" in entry:
        facts["native_core_hold"] = entry["native_core_hold"]
        facts["holds"].append("ACCEPTED_NATIVE_SOURCE_STACK_CLASS_CONFLICT")
    return facts


def put_group(definition, group, fields):
    semantics = definition.setdefault("semantics", {})
    old = semantics.get(group, UNKNOWN)
    value = copy.deepcopy(old.get("value", {})) if old["state"] == "KNOWN" else {}
    for field, fact in fields.items():
        previous = value.get(field, UNKNOWN)
        if previous["state"] == "KNOWN" and previous != fact:
            raise ValueError(f"EXISTING_FACT_CONFLICT:{definition['identity']['key']}:{group}.{field}")
        value[field] = fact
    semantics[group] = known(value)


def enrich(definition, facts, xml):
    if definition["identity"]["revision"] != "definition-r1" or definition["kind"] != "Item":
        raise ValueError("ITEM_IDENTITY_SCOPE")
    stack_class = "StackCapable" if facts["stackable"] else "NonStackable"
    if "native_core_hold" in facts:
        hold = facts["native_core_hold"]
        if (definition["stack_class"] != hold["accepted_stack_class"] or
                stack_class != hold["source_stack_class"] or
                hashlib.sha256(canonical(definition)).hexdigest() != hold["accepted_definition_sha256"]):
            raise ValueError("PROTECTED_NATIVE_CORE_HOLD_CHANGED")
        return definition
    if definition["stack_class"] not in {"Unknown", stack_class}:
        raise ValueError("EXISTING_STACK_CLASS_CONFLICT")
    definition["stack_class"] = stack_class
    put_group(definition, "stack", {"stackable": known(facts["stackable"]),
        "stack_max": known(100) if facts["stackable"] else UNKNOWN})
    physical = definition.get("semantics", {}).get("physical", {}).get("value", {})
    put_group(definition, "physical", {"movable": known(True), "pickupable": known(True),
        "weight": physical.get("weight", UNKNOWN)})
    if facts["capacity"] is not None:
        put_group(definition, "container", {"capacity": known(facts["capacity"])})
    if facts["charges"] is not None:
        put_group(definition, "charges", {"count": known(facts["charges"])})
    rows = list(xml.values())
    readable = agree(rows, "readable")
    writable = agree(rows, "writeable")
    if readable == "1" or writable == "1":
        old = definition.get("semantics", {}).get("readable_writeable", {}).get("value", {})
        fields = {name: old.get(name, UNKNOWN) for name in
            ("readable", "writeable", "distance_read", "max_text_length", "write_once_target")}
        fields["readable"] = known(True)
        if writable is not None:
            fields["writeable"] = known(writable == "1")
        length = agree(rows, "maxtextlen")
        if length is not None:
            fields["max_text_length"] = known(int(length))
        # A write-once transformation is a separate reference; it remains unknown here.
        put_group(definition, "readable_writeable", fields)
    # Keep an existing admission (notably i3081); its reward subtype stays explicitly held.
    if not facts["holds"]:
        definition["materializable"] = True
    return definition


def load_packet(root, packet_path, snapshot_path):
    packet = json.loads((root / packet_path).read_bytes())
    schema = ("OTERYN_QUEST_REWARD_ITEM_CORE_ADMISSION/v1" if packet_path == PACKET
        else "OTERYN_QUEST_INTERACTION_ITEM_CORE_ADMISSION/v1")
    if packet["schema"] != schema or packet["client_sha256"] != CLIENT_SHA:
        raise ValueError("ADMISSION_PIN")
    raw = (root / CLIENT).read_bytes()
    if len(raw) != CLIENT_BYTES or hashlib.sha256(raw).hexdigest() != CLIENT_SHA:
        raise ValueError("CLIENT_PIN")
    snapshot = (root / snapshot_path).read_bytes()
    if hashlib.sha256(snapshot).hexdigest() != packet["wiki_snapshot_sha256"]:
        raise ValueError("WIKI_SNAPSHOT_PIN")
    wiki_pages = {(p["pageid"], p["revisions"][0]["revid"]): p
        for p in json.loads(snapshot)["pages"]}
    sys.path.insert(0, str(root / "tools/content-schema/item-authoring"))
    from engine_items import load_appearance_objects
    appearances = load_appearance_objects(raw)
    admissions = {}
    for entry in packet["admissions"]:
        item_id = entry["item_id"]
        key = f"oteryn:item.tibia.i{item_id}"
        if key in admissions or entry["item_key"] != key or item_id not in appearances:
            raise ValueError("ADMISSION_KEY")
        verify_wiki(entry["wiki"], wiki_pages, item_id)
        facts = item_facts(entry, appearances[item_id])
        if facts != entry["facts"]:
            raise ValueError("ADMISSION_FACTS_MISMATCH")
        admissions[key] = (facts, entry["xml"])
    scope = packet["scope"]
    claims = json.loads((root / CLAIMS).read_bytes())["claims"] if "source_claim_count" in scope else []
    ids = set()
    def reward_ids(value):
        if isinstance(value, dict):
            if value.get("family") == "Item":
                match = re.fullmatch(r"(?:canary|crystalserver):item/(\d+)", value["key"])
                if not match:
                    raise ValueError("SOURCE_REWARD_ITEM_KEY")
                ids.add(int(match[1]))
            for child in value.values():
                reward_ids(child)
        elif isinstance(value, list):
            for child in value:
                reward_ids(child)
    for claim in claims:
        for placement in claim["placements"]:
            reward_ids(placement.get("reward", {}))
    if "extra_inventory_path" in scope:
        raw_inventory = (root / scope["extra_inventory_path"]).read_bytes()
        if hashlib.sha256(raw_inventory).hexdigest() != scope["extra_inventory_sha256"]:
            raise ValueError("EXTRA_INVENTORY_PIN")
        inventory = json.loads(raw_inventory)
        if inventory.get("schema") != "OTERYN_QUEST_ITEM_ROLE_INVENTORY/v1":
            raise ValueError("EXTRA_INVENTORY_SCHEMA")
        declared = {record["item_id"] for record in inventory["records"]}
        if declared != set(inventory["ids"]):
            raise ValueError("EXTRA_LITERAL_ID_CLOSURE")
        additions = set(inventory["admission_ids"])
        if not additions <= declared:
            raise ValueError("EXTRA_ADMISSION_NOT_SOURCE_REFERENCED")
        ids |= additions
    if (len(claims) != scope.get("source_claim_count", 0) or
            hashlib.sha256(canonical(sorted(ids))).hexdigest() != scope["reward_item_ids_sha256"] or
            set(admissions) != {f"oteryn:item.tibia.i{i}" for i in ids}):
        raise ValueError("REWARD_ITEM_SCOPE_MISMATCH")
    return admissions


def load_admissions(root):
    admissions = load_packet(root, PACKET, SNAPSHOT)
    if (root / EXTRA_PACKET).exists():
        extra = load_packet(root, EXTRA_PACKET, EXTRA_SNAPSHOT)
        if admissions.keys() & extra.keys():
            raise ValueError("OVERLAPPING_ADMISSION_PACKETS")
        admissions.update(extra)
    return admissions


def apply_admissions(definitions, root):
    admissions = load_admissions(root)
    seen = set()
    for definition in definitions:
        key = definition["identity"]["key"]
        if key in admissions:
            if key in seen:
                raise ValueError("DUPLICATE_ITEM_KEY")
            enrich(definition, *admissions[key])
            seen.add(key)
    if seen != admissions.keys():
        raise ValueError("ADMITTED_ITEM_MISSING")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--output-root", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    admissions = load_admissions(args.repo_root)
    changed = []
    seen = set()
    for path in sorted((args.repo_root / "content/items/definitions").glob("items-*.json")):
        shard = json.loads(path.read_bytes())
        for row in shard["records"]:
            key = row["definition"]["identity"]["key"]
            if key in admissions:
                if key in seen:
                    raise ValueError("DUPLICATE_ITEM_KEY")
                enrich(row["definition"], *admissions[key])
                seen.add(key)
        payload = canonical(shard)
        if payload != path.read_bytes():
            relative = path.relative_to(args.repo_root)
            changed.append(str(relative))
            if args.output_root:
                target = args.output_root / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(payload)
    if seen != admissions.keys():
        raise ValueError("ADMITTED_ITEM_MISSING")
    print(json.dumps({"admissions": len(admissions), "changed_shards": changed}))
    return 1 if args.check and changed else 0


if __name__ == "__main__":
    raise SystemExit(main())
