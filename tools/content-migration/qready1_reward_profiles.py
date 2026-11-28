"""Reuse the six exact QUESTPC5 native reward profiles needed by QREADY1."""

import hashlib
import json
from copy import deepcopy

import equipment_eligibility
import forge_item_profiles
import imbuement_item_profiles
import quest_callback_profiles as accepted
from srx1_profile_updates import compact_item_record

PACKET = "tools/content-schema/native-gameplay/qready1-reward-item-profiles.json"
IDS = {3068, 3323, 3383, 3579, 3734, 4871}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PACKET).read_bytes())
    records = packet["records"]
    if {r["production_definition"]["production_key"] for r in records} != {
        f"oteryn:item.tibia.i{i}" for i in IDS
    } or len(records) != len(IDS):
        raise ValueError("QREADY1_NATIVE_REWARD_SCOPE")
    raw = (root / accepted.SOURCE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != accepted.SOURCE_SHA256:
        raise ValueError("QREADY1_NATIVE_REWARD_SOURCE_PIN")
    xml = accepted.ET.fromstring(raw).findall("item")
    definitions = {
        r["definition"]["identity"]["key"]: r["definition"]
        for p in (root / "content/items/definitions").glob("items-*.json")
        for r in json.loads(p.read_bytes())["records"]
    }
    bindings = json.loads(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )["bindings"]
    document = json.loads(generated[profile_path])
    present = {r['production_definition']['production_key']: r for r in document['records']}
    forge_owners = None
    eligibility = {}
    for record in records:
        key = record["production_definition"]["production_key"]
        definition = definitions[key]
        (binding,) = [b for b in bindings if b["target"] == definition["identity"]]
        item_id = int(binding["external_id"])
        (node,) = [
            n
            for n in xml
            if int(n.get("id", n.get("fromid", "0")))
            <= item_id
            <= int(n.get("id", n.get("toid", "0")))
        ]
        speeds = [
            a for a in node.findall("attribute") if a.get("key", "").lower() == "speed"
        ]
        attributes = {"speed_bonus": int(speeds[0].get("value")) if speeds else 0}
        if (
            not definition["materializable"]
            or binding["disposition"] != "EXACT"
            or record["semantics"] != accepted.grant_semantics(root, definition)
            or record["attributes"] != attributes
            or record["authoring"]["item"] != definition["identity"]
        ):
            raise ValueError("QREADY1_NATIVE_REWARD_CANONICAL_DRIFT:" + key)
        accepted.complete_native_stack(root, record, definition, binding)
        if key in present and present[key] != record:
            # Replay the final provider's Forge, Crystal imbuement, and Crystal
            # equipment eligibility overlays before comparing the full record.
            if forge_owners is None:
                forge_owners = forge_item_profiles.owners(root)
                for fact in equipment_eligibility.load_facts(root):
                    eligibility.setdefault(fact['key'], []).append(fact)
            qualified = {'records': [deepcopy(record)]}
            forge_item_profiles.project(qualified, forge_owners)
            imbuement_item_profiles.apply_crystal_audit(root, qualified, require_all=False)
            equipment_eligibility.complete(qualified, eligibility.get(key, ()))
            if compact_item_record(deepcopy(present[key])) != compact_item_record(qualified['records'][0]):
                raise ValueError('QREADY1_NATIVE_REWARD_CONFLICT:' + key)
        else:
            present[key] = record
    document['records'] = sorted(present.values(), key=order)
    generated[profile_path] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(
        generated[profile_path]
    ).hexdigest()
    generated[manifest_path] = canonical(manifest)
