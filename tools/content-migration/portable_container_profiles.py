"""Project Crystal's ordinary portable containers into the native Item provider.

Capture from Crystal 201189f8 data/items/items.xml and data/items/appearances.dat.
The loader uses the appearance container group (src/items/items.cpp:172-175)
and take flag (:236), then applies XML overrides (item_parse.cpp:210-243).
"""

import argparse
import hashlib
import json
from pathlib import Path

from grizzly_shop_profiles import REVISION, ROOT, SOURCE_HASHES, source
from house_decoration_actions import PROFILES, apply_top_order
from quest_reward_item_semantics import CONTAINER_SUPERSESSIONS

FACTS = "tools/content-schema/native-gameplay/portable-container-facts.json"
IDS = {
    2481,
    2482,
    2858,
    2860,
    2861,
    2862,
    2864,
    2865,
    2866,
    2868,
    2869,
    2870,
    2871,
    2872,
    4282,
    5949,
    5950,
    6104,
    6509,
    8860,
    8861,
    9602,
    9603,
    9604,
    9605,
    10202,
    10324,
    10325,
    10327,
    14248,
    14249,
    14674,
    16099,
    16100,
    19159,
    21292,
    21411,
    22084,
    24395,
    24861,
    24862,
    35577,
    37536,
    37561,
    53074,
}
PROVENANCE = {
    "repository": "zimbadev/crystalserver",
    "revision": REVISION,
    "inputs": {f"data/items/{name}": digest for name, digest in SOURCE_HASHES.items()},
}


def capture(xml_path, appearance_path):
    xml, bank = xml_path.read_bytes(), appearance_path.read_bytes()
    for name, raw in (("items.xml", xml), ("appearances.dat", bank)):
        if hashlib.sha256(raw).hexdigest() != SOURCE_HASHES[name]:
            raise ValueError(f"Portable container donor digest differs: {name}")
    items = source.xml_index(xml)
    objects = source.engine_items.load_appearance_objects(bank)
    physical = {row["server_item_id"]: row for row in source.policies(bank, xml, IDS)}
    facts = []
    for item_id in sorted(IDS):
        item = items[item_id]
        attributes = source.attrs(item)
        flags = objects[item_id]["flags"]
        policy = physical[item_id]
        if (
            not all(policy[key] for key in ("container", "pickupable", "movable"))
            or any(
                flags.get(key)
                for key in ("flags.cumulative", "flags.corpse", "flags.player_corpse")
            )
            or "duration" in attributes
            or "decayto" in attributes
        ):
            raise ValueError(
                f"Donor Item {item_id} is no longer an ordinary portable container"
            )
        if "id" in item.attrib:
            marker = f'<item id="{item_id}" '.encode()
        else:
            low, high = item.get("fromid"), item.get("toid")
            marker = f'<item fromid="{low}" toid="{high}" '.encode()
        if xml.count(marker) != 1:
            raise ValueError(f"Donor Item {item_id} has no unique XML line")
        facts.append(
            {
                "id": item_id,
                "xml_line": xml.count(b"\n", 0, xml.index(marker)) + 1,
                "capacity": int(attributes["containersize"]),
                "weight": int(attributes["weight"]),
                "attributes": {
                    "blocks_movement": policy["block_solid"],
                    "blocks_projectile": policy["block_projectile"],
                    "has_height": policy["has_height"],
                    # Crystal Abilities initializes speed to zero (src/items/items.hpp:38).
                    "speed_bonus": int(attributes.get("speed", "0")),
                },
            }
        )
    return facts


def admit(root, items, order):
    packet = json.loads((root / FACTS).read_bytes())
    facts = packet["records"]
    if packet["source"] != PROVENANCE or [row["id"] for row in facts] != sorted(IDS):
        raise ValueError("Portable container source identity set differs")

    bindings = source.binding_index(
        (root / source.BINDINGS.relative_to(ROOT)).read_bytes()
    )
    identities = {bindings[str(item_id)]["target"]["key"] for item_id in IDS}
    definitions = {}
    for path in sorted((root / "content/items/definitions").glob("items-*.json")):
        for row in json.loads(path.read_bytes())["records"]:
            definition = row["definition"]
            key = definition["identity"]["key"]
            if key in identities:
                if key in definitions:
                    raise ValueError(f"Duplicate portable container definition {key}")
                definitions[key] = definition
    existing = {
        row["production_definition"]["production_key"]: row for row in items["records"]
    }

    records = []
    known = source.K
    for fact in facts:
        item_id, capacity, weight = (
            fact[field] for field in ("id", "capacity", "weight")
        )
        if not 1 <= capacity <= 20 or not 1 <= weight <= 0xFFFFFFFF:
            raise ValueError(
                f"Invalid portable container capacity or weight: {item_id}"
            )
        identity = bindings[str(item_id)]["target"]
        definition = definitions[identity["key"]]
        if definition["identity"] != identity or definition["stack_class"] not in (
            "Unknown",
            "NonStackable",
        ):
            raise ValueError(f"Conflicting portable container definition: {item_id}")
        semantics = {
            **dict.fromkeys(source.SECTIONS, source.U),
            **definition.get("semantics", {}),
        }
        capacity_fact = known({"capacity": known(capacity)})
        supersession = CONTAINER_SUPERSESSIONS.get(item_id)
        if supersession is not None:
            donor_capacity, accepted_capacity = supersession[-3], supersession[-1]
            if donor_capacity != capacity or semantics["container"] != known(
                {"capacity": known(accepted_capacity)}
            ):
                raise ValueError(
                    f"Conflicting portable container supersession: {item_id}"
                )
            # The accepted 22-slot Adventurer Backpack exceeds the carried-view bound.
            if accepted_capacity > 20:
                continue
        if semantics["container"] not in (source.U, capacity_fact):
            raise ValueError(f"Conflicting portable container capacity: {item_id}")
        semantics.update(
            container=capacity_fact,
            physical=known(
                {
                    "weight": known(weight),
                    "movable": known(True),
                    "pickupable": known(True),
                }
            ),
            stack=known({"stackable": known(False), "stack_max": source.NA}),
            temporal=source.NA,
        )
        records.append(
            {
                "authoring": {"item": identity},
                "semantics": semantics,
                "attributes": fact["attributes"],
                "admission": {
                    "materializable": True,
                    "stack_class": "NonStackable",
                    "legal_destinations": ["CharacterInventory", "Ground"],
                },
            }
        )
    bound = source.bind_profiles(
        {"schema": items["schema"], "records": records},
        bindings,
        {int(row["authoring"]["item"]["key"].rsplit(".i", 1)[1]) for row in records},
    )
    # Compose House facts into the base rows so SRX-1 only carries its own changes.
    house = json.loads((root / PROFILES).read_bytes())
    apply_top_order(bound["records"], house)
    for record in bound["records"]:
        key = record["production_definition"]["production_key"]
        if key not in existing:
            items["records"].append(record)
            continue
        # Preserve the seasonal Christmas present, feedbag loot metadata, the
        # Explorer Society crown backpack, and packed BUY's shopping bag when
        # their custody agrees.
        prior = existing[key]
        policy_fields = ("container", "physical", "stack", "temporal")
        if key == "oteryn:item.tibia.i21411" and prior["semantics"]["temporal"] == source.U:
            # The packed BUY admission leaves duration unknown. Crystal
            # qualifies the shopping bag as a non-decaying container.
            policy_fields = ("container", "physical", "stack")
        if (
            key not in {
                "oteryn:item.tibia.i6509",
                "oteryn:item.tibia.i9605",
                "oteryn:item.tibia.i21292",
                "oteryn:item.tibia.i21411",
            }
            or any(
                prior[field] != record[field]
                for field in (
                    "admission",
                    "production_binding",
                    "production_definition",
                )
            )
            or any(
                prior["semantics"][field] != record["semantics"][field]
                for field in policy_fields
            )
            or any(
                prior["attributes"].get(field, value) != value
                for field, value in record["attributes"].items()
            )
        ):
            raise ValueError(f"Conflicting portable container native policy: {key}")
        prior["semantics"]["temporal"] = record["semantics"]["temporal"]
        prior["attributes"].update(record["attributes"])
    items["records"].sort(key=order)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("items_xml", type=Path)
    parser.add_argument("appearances_dat", type=Path)
    args = parser.parse_args()
    facts = capture(args.items_xml, args.appearances_dat)
    rows = ",\n".join("    " + json.dumps(row) for row in facts)
    header = json.dumps({"source": PROVENANCE}, indent=2)
    (ROOT / FACTS).write_text(header[:-2] + ',\n  "records": [\n' + rows + "\n  ]\n}\n")
