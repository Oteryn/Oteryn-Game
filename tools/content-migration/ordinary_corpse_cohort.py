"""Third-quarter Crystal corpse data for the existing native Item and corpse owners.

The retained capture uses data/items/items.xml and data/items/appearances.dat
at Crystal 201189f8. Absent overrides use src/items/items.hpp:292,300,323,384
for decay, weight, container capacity and stop-time defaults.
"""

import json
from pathlib import Path

from qengine_reward_item_profiles import source

ROOT = Path(__file__).resolve().parents[2]
PACKET = "tools/content-schema/native-gameplay/ordinary-corpse-cohort.json"
PREFIX = "oteryn:item.tibia.i"
CREATURE_PREFIX = "oteryn:creature."
TENTACLE = 35136
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
K = source.K
NA = source.NA


def packet(root):
    result = json.loads((root / PACKET).read_bytes())
    if result["schema"] != 1 or result["crystal_revision"] != REVISION:
        raise ValueError("Unexpected ordinary corpse source")
    return result


def item_ref(number):
    return {"family": "Item", "key": PREFIX + str(number), "revision": "definition-r1"}


def select_binding(number):
    return {
        "source_key": "oteryn:source.crystalserver",
        "source_revision": REVISION,
        "identity_namespace": "ots/item_server_id",
        "external_id": str(number),
        "target": item_ref(number),
        "disposition": "EXACT",
    }


def bind_creatures(profiles, bindings):
    remaining = dict(bindings)
    for profile in profiles:
        key = profile["target"]["key"]
        creature = key.removeprefix(CREATURE_PREFIX)
        if creature not in remaining or key == creature:
            continue
        expected = item_ref(remaining.pop(creature)["corpse"])
        details = profile["data"]["profile"]["details"]
        if details.setdefault("corpse_item", expected) != expected:
            raise ValueError(f"Conflicting corpse Item for {key}")
    if remaining:
        raise ValueError(f"Missing Creature profiles: {sorted(remaining)}")


def extend_item_references(records, root):
    """Append the appearance-backed Tentacle Item after the existing tree successors."""
    from hashlib import sha256

    from boss_conditional_item_reference import definition
    from qengine_reward_item_profiles import CLIENT, CLIENT_SHA

    raw = (root / CLIENT).read_bytes()
    if sha256(raw).hexdigest() != CLIENT_SHA:
        raise ValueError("Tentacle appearance bank changed")
    flags = source.engine_items.load_appearance_objects(raw)[TENTACLE]["flags"]
    if (
        not all(
            flags.get("flags." + name) for name in ("corpse", "container", "unmove")
        )
        or flags.get("flags.take")
        or flags.get("flags.cumulative")
    ):
        raise ValueError("Tentacle appearance custody changed")

    expected = definition(PREFIX + str(TENTACLE))
    existing = [row for row in records if row["identity"] == expected["identity"]]
    if existing and existing != [expected]:
        raise ValueError("Tentacle Item reference changed")
    if not existing:
        records.append(expected)


def item_profile(number, facts, bindings):
    identity = item_ref(number)
    capacity = facts["capacity"]
    semantics = {
        "classification": K(
            {"item_type": K("CONTAINER") if capacity else NA, "capabilities": source.U}
        ),
        "physical": K(
            {key: K(facts[key]) for key in ("weight", "movable", "pickupable")}
        ),
        "stack": K({"stackable": K(False), "stack_max": NA}),
        "container": K({"capacity": K(capacity)}) if capacity else NA,
    }
    if duration := facts["duration_millis"]:
        target = facts["target"]
        semantics["temporal"] = K(
            {
                "consumption_mode": K("DURABLE_ABSOLUTE_DEADLINE"),
                "duration": K(duration),
                "stop_duration": K(False),
                "decay_target": K(
                    {"key": item_ref(target)["key"], "revision": "definition-r1"}
                )
                if target
                else NA,
            }
        )
    destinations = (
        ["CharacterInventory", "Ground"]
        if facts["movable"] and facts["pickupable"]
        else ["Ground"]
    )
    return {
        "authoring": {"item": identity},
        "semantics": semantics,
        "attributes": {
            "speed_bonus": 0,
            "has_height": facts["has_height"],
            **{
                name: facts.get(name, False)
                for name in (
                    "blocks_movement",
                    "blocks_projectile",
                    "immovable_block_solid",
                )
            },
        },
        "admission": {
            "materializable": True,
            "stack_class": "NonStackable",
            "legal_destinations": destinations,
        },
        "production_definition": {
            "family": "Item",
            "production_key": identity["key"],
            "revision_ref": identity["revision"],
        },
        "production_binding": bindings.get(str(number)) or select_binding(number),
        "production_binding_qualification": None,
    }


def apply(root, creatures, items, order):
    import house_decoration_actions

    data = packet(root)
    bind_creatures((row["profile"] for row in creatures["records"]), data["bindings"])
    profiles = {
        row["profile"]["target"]["key"]: row["profile"]["data"]["profile"]["details"]
        for row in creatures["records"]
    }
    for name, (prior, corrected) in data["binding_corrections"].items():
        details = profiles[CREATURE_PREFIX + name]
        if details["corpse_item"] != item_ref(prior):
            raise ValueError(f"Unexpected inherited corpse Item for {name}")
        details["corpse_item"] = item_ref(corrected)
    bindings = source.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    existing = {
        row["production_definition"]["production_key"]: row for row in items["records"]
    }
    admitted = []
    for number, facts in data["items"].items():
        record = item_profile(int(number), facts, bindings)
        key = record["production_definition"]["production_key"]
        if key in existing and existing[key]["admission"]["materializable"]:
            # Shared bodies already have native custody. Keep that owner, including
            # earlier portable/ground decisions and authorised Item uses.
            continue
        if key in existing:
            items["records"].remove(existing[key])
        admitted.append(record)
    # Use the same source rendering facts as the base provider. The later SRX
    # composition remains a small set of specialized updates.
    house_decoration_actions.apply_top_order(
        admitted, json.loads((root / house_decoration_actions.PROFILES).read_bytes())
    )
    items["records"].extend(admitted)
    items["records"].sort(key=order)
