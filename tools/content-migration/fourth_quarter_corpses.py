"""Compose exact Crystal 201189f8 body facts with the existing native providers."""

import hashlib
import json

from qengine_reward_item_profiles import source as item_source

K, NA, U = item_source.K, item_source.NA, item_source.U

SOURCE = "tools/content-schema/native-gameplay/fourth-quarter-corpse-source.json"


def item_ref(item):
    return {
        "family": "Item",
        "key": f"oteryn:item.tibia.i{item}",
        "revision": "definition-r1",
    }


def source(root):
    packet = json.loads((root / SOURCE).read_bytes())
    if packet["source_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8":
        raise ValueError("Unexpected corpse source revision")
    return packet


def extend_item_references(root, items):
    packet = source(root)
    directory = root / "imports/official/appearance-membership"
    manifest = json.loads((directory / "admitted.json").read_bytes())
    selection = next(row for row in manifest["files"] if row["label"] == "client-15.30")
    raw = (directory / selection["manifest"]).read_bytes()
    if hashlib.sha256(raw).hexdigest() != selection["manifest_sha256"]:
        raise ValueError("Official Item appearance membership changed")
    official_ids = {row[0] for row in json.loads(raw)["entries"]}
    existing = {item["identity"]["key"]: item for item in items}
    from boss_conditional_item_reference import definition as qualified_reference

    for item in packet["new_item_definitions"]:
        identity = item_ref(item)
        definition = {
            "identity": identity,
            "kind": "Item",
            "client_projection": "ClientSafe",
            "materializable": False,
            "stack_class": "Unknown",
        }
        if item == 35136:
            definition = qualified_reference(identity["key"])
        prior = existing.get(identity["key"])
        if item not in official_ids or (prior is not None and prior != definition):
            raise ValueError(f"Invalid corpse reference {item}")
        if prior is None:
            items.append(definition)
            existing[identity["key"]] = definition


def apply_items(root, items, provider_order, shared_ids, capacity_projections):
    from ordinary_corpse_profiles import (
        movable_stage_profiles,
        preserve_projected_capacity,
        qualify_inherited_portable_stages,
        qualify_native_decay_edges,
    )
    from ordinary_corpse_slice2 import merge_shared_item

    # Crystal items.cpp loads the appearance flags before the items.xml
    # overrides. ItemType starts with zero weight and eight container slots.
    packet = source(root)
    bindings = item_source.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    for item in packet["new_item_bindings"]:
        if str(item) in bindings:
            raise ValueError(f"Corpse binding already owned: {item}")
        bindings[str(item)] = {
            "source_key": "oteryn:source.crystalserver",
            "source_revision": packet["source_revision"],
            "identity_namespace": "ots/item_server_id",
            "external_id": str(item),
            "target": item_ref(item),
            "disposition": "EXACT",
        }

    existing = {
        row["production_definition"]["production_key"]: row for row in items["records"]
    }
    records = []
    for external, facts in packet["items"].items():
        binding = bindings[external]
        reference = item_ref(int(external))
        key = reference["key"]
        if binding["target"] != reference or (
            key in existing and int(external) not in shared_ids
        ):
            raise ValueError(f"Corpse Item identity already owned or mismatched: {key}")
        capacity = facts["capacity"]
        semantics = {
            "classification": K(
                {
                    "item_type": K("CONTAINER") if capacity is not None else NA,
                    "capabilities": U,
                }
            ),
            "physical": K(
                {name: K(facts[name]) for name in ("weight", "movable", "pickupable")}
            ),
            "stack": K({"stackable": K(False), "stack_max": NA}),
            "container": K({"capacity": K(capacity)}) if capacity is not None else NA,
            "charges": NA,
            "temporal": NA,
        }
        if facts["duration_ms"]:
            target = facts["decay_target"]
            semantics["temporal"] = K(
                {
                    "consumption_mode": K("DURABLE_ABSOLUTE_DEADLINE"),
                    "duration": K(facts["duration_ms"]),
                    "stop_duration": K(False),
                    "decay_target": K(
                        {"key": item_ref(target)["key"], "revision": "definition-r1"}
                    )
                    if target is not None
                    else NA,
                }
            )
        records.append(
            {
                "authoring": {"item": reference},
                "production_binding": binding,
                "production_definition": {
                    "family": "Item",
                    "production_key": key,
                    "revision_ref": reference["revision"],
                },
                "admission": {
                    "materializable": True,
                    "legal_destinations": ["Ground"],
                    "stack_class": "NonStackable",
                },
                "semantics": semantics,
                "attributes": {
                    "blocks_movement": facts["blocks_movement"],
                    "blocks_projectile": facts["blocks_projectile"],
                    "immovable_block_solid": facts["blocks_movement"]
                    and not facts["movable"],
                    "has_height": facts["has_height"],
                },
            }
        )
    from house_decoration_actions import PROFILES, apply_top_order

    apply_top_order(records, json.loads((root / PROFILES).read_bytes()))
    for record in records:
        key = record["production_definition"]["production_key"]
        preserve_projected_capacity(record, existing.get(key), capacity_projections)
        if key in existing:
            merge_shared_item(existing[key], record)
        else:
            items["records"].append(record)
    present = {
        row["production_definition"]["production_key"]: row for row in items["records"]
    }
    for record in movable_stage_profiles(root):
        key = record["production_definition"]["production_key"]
        if key in present:
            item = int(record["production_binding"]["external_id"])
            if item not in shared_ids:
                raise ValueError(f"Movable corpse stage already has an Item owner: {key}")
            preserve_projected_capacity(record, present[key], capacity_projections)
            merge_shared_item(present[key], record)
        else:
            items["records"].append(record)
            present[key] = record
    qualify_inherited_portable_stages(root, items["records"])
    qualify_native_decay_edges(root, items["records"])
    items["records"].sort(key=provider_order)


def bind_creatures(root, creatures):
    profiles = {
        row["profile"]["target"]["key"]: row["profile"]["data"]["profile"]["details"]
        for row in creatures["records"]
    }
    for creature, evidence in source(root)["bindings"].items():
        details = profiles[creature]
        if details.get("corpse_item") is not None:
            raise ValueError(f"Creature corpse binding already owned: {creature}")
        details["corpse_item"] = item_ref(evidence["item"])
