"""Replay qualified Item Details facts after native profile admission."""

import json

from import_cyclopedia_global import load_weights


WEIGHT_IDS = (43501, 43502, 43503, 43504, 43899, 43900, 43946, 43947, 44048, 44727, 44728)
WEIGHT_FACTS = "tools/content-schema/native-gameplay/item-detail-weight-facts.json"
GLOBAL_MODIFIER_PROMOTIONS = "docs/agents/evidence/OTV2-20261002-item-mantra-bond-promotion-v1.json"


def item_key(identifier):
    return f"oteryn:item.tibia.i{identifier}"


def known(value):
    return {"state": "KNOWN", "value": value}


def selected_modifier_vectors(root):
    from lower_mantra_bond_modifier_packet import whole_vector
    from lower_elemental_magic_modifier_packet import native_order

    # The existing Global promotion supplies the whole vectors for sanguine
    # trousers and ethereal coned hat. Crystal has 10 for the hat, Global has 12.
    packet = json.loads((root / GLOBAL_MODIFIER_PROMOTIONS).read_bytes())
    keys = {item_key(50146), item_key(50188)}
    vectors = {
        row["item_key"]: row["typed_value"]["value"]
        for row in packet["promotions"]
        if row["item_key"] in keys and row["field_path"] == "skill_modifiers.modifiers"
    }
    if vectors.keys() != keys:
        raise ValueError("ITEM_DETAILS_MISSING_GLOBAL_MODIFIER_VECTOR")

    # Crystal 201189f871d23597b82caf88fd611454a2ddefb8, data/items/items.xml.
    # Soulgarb's explicit server value is 19. The Global promotion records 17.
    order = native_order()
    for identifier, fields in (
        (50254, {"mantra": ["19"], "attrib": ["magic level +2, fist fighting +4"]}),
        (50269, {"mantra": ["3"], "attrib": ["fist fighting +1"]}),
    ):
        vector = whole_vector(fields, order)
        if vector is None:
            raise ValueError(f"ITEM_DETAILS_INVALID_CRYSTAL_MODIFIER_VECTOR:{identifier}")
        vectors[item_key(identifier)] = vector["value"]
    return vectors


def qualified_weights(root):
    captured_weights = load_weights(root)
    return {identifier: captured_weights[identifier] for identifier in WEIGHT_IDS}


def load_weight_facts(root, generated):
    raw = generated.get(WEIGHT_FACTS)
    if raw is None:
        raw = (root / WEIGHT_FACTS).read_bytes()
    weights = qualified_weights(root)
    if json.loads(raw) != {str(identifier): value for identifier, value in weights.items()}:
        raise ValueError("ITEM_DETAILS_WEIGHT_FACTS_DRIFT")
    return weights


def apply_weight(record, identifier, weights):
    # These donor overlays can encode the loader's default zero as weight.
    # The verified Global capture has positive measurements for every entry.
    value = weights[identifier]
    physical = record["semantics"]["physical"]["value"]
    if value <= 0 or physical["weight"] not in (known(0), known(value)):
        raise ValueError(f"ITEM_DETAILS_WEIGHT_SOURCE_CONFLICT:{identifier}")
    physical["weight"] = known(value)


def apply(root, profiles, weights):
    records = {row["production_definition"]["production_key"]: row for row in profiles["records"]}
    for identifier in weights:
        apply_weight(records[item_key(identifier)], identifier, weights)

    for key, modifiers in selected_modifier_vectors(root).items():
        semantics = records[key]["semantics"]
        selected = known({"modifiers": known(modifiers)})
        if semantics["skill_modifiers"] not in ({"state": "UNKNOWN"}, selected):
            raise ValueError(f"ITEM_DETAILS_MODIFIER_SOURCE_CONFLICT:{key}")
        semantics["skill_modifiers"] = selected
