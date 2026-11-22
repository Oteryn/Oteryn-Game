"""Compose two independently qualified overlays without changing either receipt."""

import copy
import hashlib
import json

from bind_monster_source_definitions import CREATURES, bind_definitions
from import_spell_families import canonical_bytes
from cake_native_successor import (
    PRESENTATIONS, admit as admit_cake_successor, read_derived_shard, read_project_document,
)
from build_canonical_monster_profiles import union_profiles
from raid_catalogue import ORC_SAM_CREATURE, ORC_SAM_SOURCE, load
from monster_seven_spell_overlay import ALLOWED_PATHS, apply_overlay


def identity(row):
    return canonical_bytes(row["profile"]["target"])


def indexed(document):
    rows = {identity(row): row for row in document["records"]}
    if len(rows) != len(document["records"]):
        raise ValueError("COMPOSE_MONSTER_DUPLICATE_IDENTITY")
    return rows


def compose(root, baseline, current):
    # Both stages retain their original full-file and per-record source guards.
    adopted = apply_overlay(root, baseline)
    base_doc = json.loads(baseline[CREATURES])
    current_doc = json.loads(current[CREATURES])
    result_doc = json.loads(adopted[CREATURES])
    base_rows, current_rows, result_rows = map(
        indexed, (base_doc, current_doc, result_doc)
    )
    if (
        {k: v for k, v in base_doc.items() if k != "records"}
        != {k: v for k, v in current_doc.items() if k != "records"}
        or base_rows.keys() != current_rows.keys()
        or not base_rows.keys() <= result_rows.keys()
    ):
        raise ValueError("COMPOSE_MONSTER_CURRENT_SCOPE_CHANGED")
    for key, before in base_rows.items():
        incoming, retained = current_rows[key], result_rows[key]
        if {k: v for k, v in before.items() if k != "monster_melee"} != {
            k: v for k, v in incoming.items() if k != "monster_melee"
        }:
            raise ValueError("COMPOSE_MONSTER_NON_MELEE_CHANGE")
        old_melee, new_melee = before.get("monster_melee"), incoming.get("monster_melee")
        if old_melee == new_melee:
            continue
        if new_melee is None or retained.get("monster_melee") not in (
            old_melee,
            new_melee,
        ):
            raise ValueError("COMPOSE_MONSTER_MELEE_CONFLICT")
        retained["monster_melee"] = copy.deepcopy(new_melee)
    result = dict(current)
    # Retain every adopted appearance, variant, Wiki field and source mechanic.
    for path in ALLOWED_PATHS:
        result[path] = adopted[path]
    result[CREATURES] = canonical_bytes(result_doc)
    # Player catalog/selection pins belong to the separately qualified current
    # source stage; Monster/presentation pins belong to the adopted stage.
    manifest_path = "content/spells.manifest.json"
    manifest = json.loads(result[manifest_path])
    current_manifest = json.loads(current[manifest_path])
    for section in ("catalog", "source_selection"):
        manifest[section] = current_manifest[section]
    result[manifest_path] = canonical_bytes(manifest)
    # W05d: the immutable adopted overlay predates canonical Cake admission.
    # Extend only that source-complete Creature through the normal union.
    result = admit_cake_successor(root, result)
    return bind_definitions(root, admit_orc_sam(root, result))

def admit_orc_sam(root, generated):
    packet = load(root, ORC_SAM_SOURCE)
    reference = read_project_document(root, "definitions/reference.json")["records"]
    expected = {row["identity"]["key"]: row for row in packet["records"]}
    actual = {row["identity"]["key"]: row for row in reference if row["identity"]["key"] in expected}
    if not actual:
        # Source-only changes are qualified before the PC emits canonical shards.
        return generated
    if actual != expected:
        raise ValueError("ORC_SAM_CANONICAL_SOURCE_RECORDS")
    declarations = read_project_document(root, "definitions/declarations.json")["authoring_profiles"]
    rows = [
        row
        for path in sorted((root / "content/creatures/definitions").glob("creatures-*.json"))
        for row in read_derived_shard(path)["records"]
        if row["definition"]["identity"]["key"] == ORC_SAM_CREATURE
    ]
    profile, = [row for row in declarations if row["target"]["key"] == ORC_SAM_CREATURE]
    if (len(rows) != 1 or rows[0]["definition"] != expected[ORC_SAM_CREATURE]
            or rows[0]["authoring"] != profile["data"]):
        raise ValueError("ORC_SAM_CANONICAL_CREATURE")
    active = json.loads(generated[CREATURES])
    presentations = json.loads(generated[PRESENTATIONS])
    creatures, presentations, added = union_profiles(
        active, presentations, rows, declarations,
        preserve_existing_display_aliases=True,
        display_aliases={ORC_SAM_CREATURE: "oteryn:creature.orc"},
    )
    if added != [rows[0]["definition"]["identity"]]:
        raise ValueError("ORC_SAM_NATIVE_IDENTITY")
    raid, = [row for row in creatures["records"] if row["profile"]["target"]["key"] == ORC_SAM_CREATURE]
    attack, = raid["behavior"]["data"]["profile"]["attacks"]
    raid["monster_melee"] = {
        **attack, **packet["melee"],
        **{f"source_{field}": value for field, value in packet["source"].items()},
        "execution_status": "approximate_nonlethal_physical_melee",
    }
    result = dict(generated)
    result[CREATURES] = canonical_bytes(creatures)
    result[PRESENTATIONS] = canonical_bytes(presentations)
    path = "content/spells.manifest.json"
    manifest = json.loads(result[path])
    manifest["presentation_profiles"]["sha256"] = hashlib.sha256(result[PRESENTATIONS]).hexdigest()
    result[path] = canonical_bytes(manifest)
    return result


def admit_orc_sam_corpse(root, items, order):
    """Admit the exact Crystal decay chain through the existing native Item decoders."""
    import boss_corpse_profiles as boss
    import quest_corpse_profiles as corpse

    packet = load(root, ORC_SAM_SOURCE)
    evidence = packet["corpse_source"]
    if boss.SOURCE_HASHES.get(evidence["path"]) != evidence["sha256"]:
        raise ValueError("ORC_SAM_CORPSE_SOURCE_PIN")
    source = corpse.grizzly.source
    raw = (root / corpse.XML).read_bytes()
    client = (root / corpse.custody.CLIENT).read_bytes()
    if (hashlib.sha256(raw).hexdigest() != corpse.XML_SHA
            or hashlib.sha256(client).hexdigest() != corpse.custody.CLIENT_SHA):
        raise ValueError("ORC_SAM_CORPSE_DECODER_INPUT")
    xml = source.xml_index(raw)
    fresh = corpse.item_id(packet["profiles"][0]["data"]["profile"]["details"]["corpse_item"])
    ids = corpse.decay_closure({fresh}, xml)
    if {str(i): source.attrs(xml[i]) for i in ids} != packet["corpse_chain"]:
        raise ValueError("ORC_SAM_CORPSE_DECAY_SOURCE")
    appearances = source.engine_items.load_appearance_objects(client)
    policies = {row["server_item_id"]: row for row in source.policies(client, raw, ids)}
    definitions = corpse.definitions(root, ids)
    world_objects = corpse.world_objects(root, ids)
    bindings = source.binding_index((root / "imports/crystalserver/bindings/items.json").read_bytes())
    by_key = {row["production_definition"]["production_key"]: row for row in items["records"]}
    for number in sorted(ids):
        key = f"oteryn:item.tibia.i{number}"
        if packet["corpse_chain"][str(number)]["decayto"] == "0":
            # The terminal bones are an ordinary Item with their own source weight.
            row = source.profile(number, xml[number], appearances[number], policies[number], bindings, None)
            row, = source.bind_profiles(
                {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [row]}, bindings, {number}
            )["records"]
        else:
            row = corpse.qualify(number, definitions[key], world_objects.get(number), xml[number],
                                 appearances[number], policies[number], bindings, number == fresh)
        prior = by_key.get(key)
        if prior is not None and prior != row:
            raise ValueError(f"ORC_SAM_CORPSE_NATIVE_CONFLICT:{key}")
        if prior is None:
            items["records"].append(row)
    items["records"].sort(key=order)
