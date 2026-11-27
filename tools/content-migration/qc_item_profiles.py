"""Lane qc: source-qualified physical custody for Twenty Miles and Lizard Sanctuary items.

Refresh with --source-root pointing at Crystal 201189f's pinned items.xml.
Qualification belongs to the existing batch-three importer and native merger.
"""

import argparse
import copy
import hashlib
import json
import xml.etree.ElementTree as ET
from pathlib import Path

import qengine_reward_item_profiles as shared
import quest_batch3_item_profiles as batch
from coin_converter_profiles import SOURCE as RETAINED_ITEM_XML
from coin_converter_profiles import SOURCE_SHA256 as RETAINED_ITEM_XML_SHA256
from import_cyclopedia_global import apply_profile_weights, load_weights

PATH = "tools/content-schema/native-gameplay/qc-item-profiles.json"
SCHEMA = "OTERYN_QC_ITEM_PROFILES/v1"
# The nine source-qualified records are frozen, including nested record shape.
# Authoring-use pointers can change independently when new placements use them.
RECORDS_SHA256 = "4907e22b4cc1754ea6d297618d1370f2550b5679a3da554e8fe89baa7264b4f2"
INPUTS = (
    "content/quests/npc-callbacks/qc-twenty-miles.json",
    "content/quests/world-callbacks/qc-lizard-sanctuary.json",
    "content/quests/world-callbacks/qc-twenty-miles.json",
)
KEYS = {
    f"oteryn:item.tibia.i{i}"
    for i in (21469, 21745, 21746, 21753, 21754, 21770, 21795, 21861, 21862)
}
# Brandon's wedding ring is a ring: the existing equipment importer owns its facts.
EQUIPMENT = {21745}
# Crystal omits the nacre slab weight; the Global client weight (ITEMGLOB) is the
# canonical 1.00 oz. Admit only while the client weight equals the canonical one.
UNWEIGHED = {21753, 21861, 21862}


def unweighed(source_root, root=batch.ROOT):
    source = batch.grizzly.source
    xml_raw = (source_root / "items.xml").read_bytes()
    client_raw = (root / batch.reward.CLIENT).read_bytes()
    if (
        hashlib.sha256(xml_raw).hexdigest() != batch.XML_SHA256
        or hashlib.sha256(client_raw).hexdigest() != batch.reward.CLIENT_SHA
    ):
        raise ValueError("QC_ITEM_SOURCE_PIN")
    xml = source.xml_index(xml_raw)
    appearances = source.engine_items.load_appearance_objects(client_raw)
    physical = {
        row["server_item_id"]: row
        for row in source.policies(client_raw, xml_raw, UNWEIGHED)
    }
    bindings = source.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    keys = {f"oteryn:item.tibia.i{i}" for i in UNWEIGHED}
    definitions = {
        row["definition"]["identity"]["key"]: row["definition"]
        for path in sorted((root / "content/items/definitions").glob("items-*.json"))
        for row in json.loads(path.read_bytes())["records"]
        if row["definition"]["identity"]["key"] in keys
    }
    weights = load_weights(root)
    records = []
    for item_id in sorted(UNWEIGHED):
        definition = definitions[f"oteryn:item.tibia.i{item_id}"]
        canonical = definition["semantics"]["physical"]["value"]["weight"]
        if "weight" in batch.grizzly.source.attrs(xml[item_id]) or canonical != {
            "state": "KNOWN",
            "value": weights[item_id],
        }:
            raise ValueError("QC_SLAB_WEIGHT_DECISION_DRIFT")
        donor = copy.deepcopy(definition)
        donor["semantics"]["physical"]["value"]["weight"] = {
            "state": "KNOWN",
            "value": 0,
        }
        record = batch.qualify(
            item_id,
            xml[item_id],
            appearances[item_id],
            physical[item_id],
            bindings,
            donor,
        )
        apply_profile_weights([record], root)
        if record["semantics"]["physical"] != definition["semantics"]["physical"]:
            raise ValueError("QC_SLAB_PHYSICAL_DRIFT")
        records.append(record)
    return records


def capture(source_root):
    packet = batch.capture(source_root, additional_inputs=INPUTS)
    packet["records"] = [
        r
        for r in packet["records"]
        if r["production_definition"]["production_key"] in KEYS
    ]
    packet["classification"] = [r for r in packet["classification"] if r["key"] in KEYS]
    equipment = shared.capture(item_ids=EQUIPMENT)
    packet["records"].extend(equipment["records"])
    packet["records"].extend(unweighed(source_root))
    packet["records"].sort(
        key=lambda r: int(
            r["production_definition"]["production_key"].rsplit("i", 1)[1]
        )
    )
    packet["sources"].extend(equipment["sources"])
    for row in packet["classification"]:
        item_id = int(row["key"].rsplit("i", 1)[1])
        if item_id in EQUIPMENT:
            row.update(
                classification="QUALIFIED_NATIVE_EQUIPMENT",
                reason=None,
                profile_source=equipment["sources"],
            )
        elif item_id in UNWEIGHED:
            row.update(classification="QUALIFIED_GLOBAL_WEIGHT", reason=None)
    packet.update(
        schema=SCHEMA,
        producer="tools/content-migration/qc_item_profiles.py",
        runtime_enabled=True,
        decision="Lane qc: Twenty Miles and Lizard Sanctuary hand-in items",
        purpose="Physical custody for source-authored quest hand-ins; no custom use effects.",
    )
    return packet


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PATH).read_bytes())
    records = packet["records"]
    encoded_records = json.dumps(
        records, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    )
    if hashlib.sha256(encoded_records.encode()).hexdigest() != RECORDS_SHA256:
        raise ValueError("QC_ITEM_RECORD_QUALIFICATION")
    if (
        packet["schema"] != SCHEMA
        or len(records) != len(KEYS)
        or {r["production_definition"]["production_key"] for r in records} != KEYS
        or len(packet["classification"]) != len(KEYS)
        or {r["key"] for r in packet["classification"]} != KEYS
        or any(
            r["classification"]
            not in (
                "QUALIFIED_PLAIN_PORTABLE",
                "QUALIFIED_NATIVE_EQUIPMENT",
                "QUALIFIED_GLOBAL_WEIGHT",
                "EXISTING_NATIVE",
            )
            for r in packet["classification"]
        )
    ):
        raise ValueError("QC_ITEM_QUALIFICATION")
    shared.admit_records(
        root, generated, profile_path, manifest_path, order, canonical, records
    )
    admit_lantern_facts(root, generated, profile_path, manifest_path, canonical)


def admit_lantern_facts(root, generated, profile_path, manifest_path, canonical):
    # Crystal 201189f8 data/items/items.xml and the retained XML agree: both
    # registered lantern forms are generic lights with no automatic decay.
    raw = (root / RETAINED_ITEM_XML).read_bytes()
    if hashlib.sha256(raw).hexdigest() != RETAINED_ITEM_XML_SHA256:
        raise ValueError("QC_LANTERN_XML_PIN")
    numbers = {"23734", "23738"}
    items = {
        item.get("id"): item
        for item in ET.fromstring(raw).findall("item")
        if item.get("id") in numbers
    }
    if items.keys() != numbers:
        raise ValueError("QC_LANTERN_XML_IDENTITIES")
    keys = {f"oteryn:item.tibia.i{number}": item for number, item in items.items()}
    document = json.loads(generated[profile_path])
    profiles = {
        row["production_definition"]["production_key"]: row
        for row in document["records"]
        if row["production_definition"]["production_key"] in keys
    }
    if profiles.keys() != keys.keys():
        raise ValueError("QC_LANTERN_NATIVE_IDENTITIES")
    for key, item in keys.items():
        attributes = {
            attribute.get("key", "").lower() for attribute in item.findall("attribute")
        }
        if attributes & {"type", "duration", "decayto", "stopduration"}:
            raise ValueError(f"QC_LANTERN_GROUND_FACTS:{key}")
        semantics = profiles[key]["semantics"]
        for field in ("temporal", "classification"):
            if semantics[field]["state"] not in {"UNKNOWN", "NOT_APPLICABLE"}:
                raise ValueError(f"QC_LANTERN_GROUND_CONFLICT:{key}:{field}")
            semantics[field] = {"state": "NOT_APPLICABLE"}
        # The admitted Extra slot has no equip-level or vocation gate
        # (ITEM-MOVE-WIRE-1 §4). Use requirements remain independent.
        equipment = semantics["equipment"]
        patterns = equipment.get("value", {}).get("patterns", {})
        slots = patterns.get("value", [])
        if (
            equipment["state"] != "KNOWN"
            or patterns.get("state") != "KNOWN"
            or len(slots) != 1
            or slots[0]["primary_slot"] != {"state": "KNOWN", "value": "EXTRA"}
        ):
            raise ValueError(f"QC_LANTERN_EQUIPMENT_PATTERN:{key}")
        for field, value in (("level", 0), ("vocations", [])):
            expected = {"state": "KNOWN", "value": value}
            if slots[0][field] not in ({"state": "UNKNOWN"}, expected):
                raise ValueError(f"QC_LANTERN_EQUIPMENT_CONFLICT:{key}:{field}")
            slots[0][field] = expected
    generated[profile_path] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(
        generated[profile_path]
    ).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    args = parser.parse_args()
    (batch.ROOT / PATH).write_bytes(batch.grizzly.encode(capture(args.source_root)))
