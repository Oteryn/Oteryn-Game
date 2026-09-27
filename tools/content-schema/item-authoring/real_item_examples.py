"""Six evidence-backed Item authoring examples from pinned engine and Wiki sources."""

from copy import deepcopy

from proficiency_profiles import (
    BERSERK_REF,
    CANARY_PROFICIENCY_SOURCE,
    CRYSTAL_PROFICIENCY_SOURCE,
    INTENSE_WOUND_CLEANSING_REF,
    MAGIC_SWORD_PROFICIENCY_REF,
    magic_sword_proficiency,
    proficiency_crosswalk,
)
from source_field_catalogs import (
    CANARY_PROFILE,
    CRYSTAL_PROFILE,
    build_br_real_item_supplement,
    build_fandom_real_item_supplement,
)

CANARY_REVISION = "47dfd51f45280a59a1d3e50ba7edd573d7234446"
CRYSTAL_REVISION = "ff7ede593c69d4c658b382c97443e8155926924a"
CANARY_APPEARANCE_DIGEST = (
    "aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50"
)
CRYSTAL_APPEARANCE_DIGEST = (
    "6adb790d1064c2d31ffb2e5ce1a7aef376942ba672edea2adb6cafc620dd18f1"
)
CANARY_ITEMS_DIGEST = "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2"
CRYSTAL_ITEMS_DIGEST = (
    "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb"
)

APPEARANCES = {
    2854: (1, 1, [195739]),
    2874: (
        4,
        3,
        [
            192686,
            192711,
            195768,
            195769,
            192677,
            195770,
            195771,
            195772,
            195773,
            195774,
            195775,
            193294,
        ],
    ),
    3155: (1, 1, [196224]),
    3288: (1, 1, [196366]),
    3388: (1, 1, [196476]),
    3585: (
        4,
        2,
        [196798, 196799, 196800, 196801, 196802, 196803, 196803, 196803],
    ),
}

FIELD_EVIDENCE_SPECS = {
    "Magic Sword": {
        "/item/display_name": ("/item/display_name", [("fandom", "name")]),
        "/item/taxonomy/item_class": (
            "/item/taxonomy/item_class",
            [("fandom", "objectclass")],
        ),
        "/item/taxonomy/primary": (
            "/item/taxonomy/primary",
            [("fandom", "primarytype"), ("engine_items_xml", "primarytype")],
        ),
        "/item/physical/weight/value": (
            "/item/physical/weight",
            [("br", "weight"), ("fandom", "weight"), ("engine_items_xml", "weight")],
        ),
        "/item/physical/movable": (
            "/item/physical/movable",
            [("fandom", "immobile")],
        ),
        "/item/physical/pickupable": (
            "/item/physical/pickupable",
            [("fandom", "pickupable")],
        ),
        "/item/requirements/min_level": (
            "/item/requirements/min_level",
            [
                ("br", "levelrequired"),
                ("fandom", "levelrequired"),
                ("engine_items_xml", "level"),
            ],
        ),
        "/item/equipment/patterns/0/hands": (
            "/item/equipment/patterns/0/hands",
            [("br", "hands"), ("fandom", "hands")],
        ),
        "/item/equipment/patterns/1/hands": (
            "/item/equipment/patterns/1/hands",
            [("br", "hands"), ("fandom", "hands")],
        ),
        "/item/weapon/weapon_type": (
            "/item/weapon/weapon_type",
            [
                ("br", "type"),
                ("fandom", "weapontype"),
                ("engine_items_xml", "weaponType"),
            ],
        ),
        "/item/weapon/attack": (
            "/item/weapon/attack",
            [("br", "attack"), ("fandom", "attack"), ("engine_items_xml", "attack")],
        ),
        "/item/weapon/defense": (
            "/item/weapon/defense",
            [("br", "defense"), ("fandom", "defense"), ("engine_items_xml", "defense")],
        ),
        "/item/weapon/extra_defense": (
            "/item/weapon/extra_defense",
            [
                ("br", "defensemod"),
                ("fandom", "defensemod"),
                ("engine_items_xml", "extradef"),
            ],
        ),
        "/item/stack/stackable": (
            "/item/stack/stackable",
            [("fandom", "stackable")],
        ),
        "/item/imbuement/slot_count": (
            "/item/imbuement/slot_count",
            [
                ("br", "imbuement"),
                ("fandom", "imbueslots"),
                ("engine_items_xml", "imbuementslot"),
            ],
        ),
        "/item/forge/classification": (
            "/item/forge/classification",
            [("br", "classificacao"), ("fandom", "upgradeclass")],
        ),
        "/item/forge/max_tier": ("/item/forge/max_tier", [("br", "max_tier")]),
        "/item/trade/marketable": (
            "/item/trade/marketable",
            [("fandom", "marketable")],
        ),
        "/item/trade/market_category": (
            "/item/trade/market_category",
            [("engine_appearance", "market.category")],
        ),
    },
    "Demon Armor": {
        "/item/display_name": ("/item/display_name", [("fandom", "name")]),
        "/item/taxonomy/item_class": (
            "/item/taxonomy/item_class",
            [("fandom", "objectclass")],
        ),
        "/item/taxonomy/primary": (
            "/item/taxonomy/primary",
            [("fandom", "primarytype"), ("engine_items_xml", "primarytype")],
        ),
        "/item/physical/weight/value": (
            "/item/physical/weight",
            [("br", "weight"), ("fandom", "weight"), ("engine_items_xml", "weight")],
        ),
        "/item/physical/movable": ("/item/physical/movable", [("fandom", "immobile")]),
        "/item/physical/pickupable": (
            "/item/physical/pickupable",
            [("fandom", "pickupable")],
        ),
        "/item/equipment/slot": (
            "/item/equipment/slot",
            [("fandom", "slot"), ("engine_items_xml", "slot")],
        ),
        "/item/protection/armor": (
            "/item/protection/armor",
            [("br", "armor"), ("fandom", "armor"), ("engine_items_xml", "armor")],
        ),
        "/item/stack/stackable": ("/item/stack/stackable", [("fandom", "stackable")]),
        "/item/imbuement/slot_count": (
            "/item/imbuement/slot_count",
            [
                ("br", "imbuement"),
                ("fandom", "imbueslots"),
                ("engine_items_xml", "imbuementslot"),
            ],
        ),
        "/item/forge/classification": (
            "/item/forge/classification",
            [("br", "classificacao"), ("fandom", "upgradeclass")],
        ),
        "/item/forge/max_tier": ("/item/forge/max_tier", [("br", "max_tier")]),
        "/item/trade/marketable": (
            "/item/trade/marketable",
            [("fandom", "marketable")],
        ),
    },
    "Backpack": {
        "/item/display_name": ("/item/display_name", [("fandom", "name")]),
        "/item/taxonomy/primary": (
            "/item/taxonomy/primary",
            [
                ("br", "primarytype"),
                ("fandom", "primarytype"),
                ("engine_items_xml", "primarytype"),
            ],
        ),
        "/item/taxonomy/secondary": (
            "/item/taxonomy/secondary",
            [("fandom", "secondarytype")],
        ),
        "/item/physical/weight/value": (
            "/item/physical/weight",
            [("br", "weight"), ("fandom", "weight"), ("engine_items_xml", "weight")],
        ),
        "/item/physical/movable": ("/item/physical/movable", [("fandom", "immobile")]),
        "/item/physical/pickupable": (
            "/item/physical/pickupable",
            [("fandom", "pickupable")],
        ),
        "/item/stack/stackable": ("/item/stack/stackable", [("fandom", "stackable")]),
        "/item/container/capacity": (
            "/item/container/capacity",
            [
                ("br", "volume"),
                ("fandom", "volume"),
                ("engine_items_xml", "containersize"),
            ],
        ),
        "/item/imbuement/slot_count": (
            "/item/imbuement/slot_count",
            [
                ("br", "imbuement"),
                ("fandom", "imbueslots"),
                ("engine_items_xml", "imbuementslot"),
            ],
        ),
        "/item/trade/marketable": (
            "/item/trade/marketable",
            [("fandom", "marketable")],
        ),
    },
    "Red Apple": {
        "/item/display_name": ("/item/display_name", [("fandom", "name")]),
        "/item/taxonomy/primary": (
            "/item/taxonomy/primary",
            [("fandom", "primarytype"), ("engine_items_xml", "primarytype")],
        ),
        "/item/physical/weight/value": (
            "/item/physical/weight",
            [("br", "weight"), ("fandom", "weight"), ("engine_items_xml", "weight")],
        ),
        "/item/physical/movable": ("/item/physical/movable", [("fandom", "immobile")]),
        "/item/physical/pickupable": (
            "/item/physical/pickupable",
            [("fandom", "pickupable")],
        ),
        "/item/stack/stackable": (
            "/item/stack/stackable",
            [("br", "stackable"), ("fandom", "stackable")],
        ),
        "/item/consumable/edible": ("/item/consumable/edible", [("br", "edible")]),
        "/item/consumable/regeneration_seconds": (
            "/item/consumable/regeneration_seconds",
            [("br", "regenseconds"), ("fandom", "regenseconds")],
        ),
        "/item/use/usable": ("/item/use/usable", [("fandom", "usable")]),
        "/item/trade/marketable": (
            "/item/trade/marketable",
            [("fandom", "marketable")],
        ),
    },
    "Sudden Death Rune": {
        "/item/display_name": ("/item/display_name", [("fandom", "name")]),
        "/item/taxonomy/item_class": (
            "/item/taxonomy/item_class",
            [("fandom", "objectclass"), ("engine_items_xml", "type")],
        ),
        "/item/taxonomy/primary": (
            "/item/taxonomy/primary",
            [("fandom", "primarytype"), ("engine_items_xml", "primarytype")],
        ),
        "/item/taxonomy/secondary": ("/item/taxonomy/secondary", [("br", "Subclass")]),
        "/item/physical/weight/value": (
            "/item/physical/weight",
            [("br", "weight"), ("fandom", "weight"), ("engine_items_xml", "weight")],
        ),
        "/item/physical/movable": ("/item/physical/movable", [("fandom", "immobile")]),
        "/item/physical/pickupable": (
            "/item/physical/pickupable",
            [("fandom", "pickupable")],
        ),
        "/item/stack/stackable": ("/item/stack/stackable", [("fandom", "stackable")]),
        "/item/charges/count": (
            "/item/charges/count",
            [("engine_items_xml", "charges")],
        ),
        "/item/requirements/min_level": (
            "/item/requirements/min_level",
            [("br", "levelrequired"), ("fandom", "levelrequired")],
        ),
        "/item/requirements/min_magic_level": (
            "/item/requirements/min_magic_level",
            [("br", "mlrequired"), ("fandom", "mlrequired")],
        ),
        "/item/requirements/premium_only": (
            "/item/requirements/premium_only",
            [("br", "premium")],
        ),
        "/item/trade/marketable": (
            "/item/trade/marketable",
            [("fandom", "marketable")],
        ),
    },
    "Vial": {
        "/item/display_name": ("/item/display_name", [("fandom", "name")]),
        "/item/physical/weight/value": (
            "/item/physical/weight",
            [("br", "weight"), ("fandom", "weight"), ("engine_items_xml", "weight")],
        ),
        "/item/physical/movable": ("/item/physical/movable", [("fandom", "immobile")]),
        "/item/physical/pickupable": (
            "/item/physical/pickupable",
            [("fandom", "pickupable"), ("engine_items_xml", "allowpickUpAble")],
        ),
        "/item/stack/stackable": ("/item/stack/stackable", [("fandom", "stackable")]),
        "/item/fluid/role": ("/item/fluid/role", [("fandom", "holdsliquid")]),
        "/item/trade/marketable": (
            "/item/trade/marketable",
            [("fandom", "marketable")],
        ),
    },
}

SOURCE_FIELD_ALIASES = {
    ("engine_items_xml", "weaponType"): "weapontype",
    ("engine_items_xml", "allowpickUpAble"): "allowpickupable",
}


def presentation_ref(appearance_id):
    return {
        "family": "Presentation",
        "key": f"oteryn:presentation.crystal.item.{appearance_id}",
        "revision": f"{CRYSTAL_REVISION}.appearance-v1",
    }


def presentation_asset(appearance_id):
    width, height, sprite_ids = APPEARANCES[appearance_id]
    return {
        "identity": presentation_ref(appearance_id),
        "source": {
            "source_profile": CRYSTAL_PROFILE,
            "repository": "zimbadev/crystalserver",
            "revision": CRYSTAL_REVISION,
            "path": "data/items/appearances.dat",
            "digest_sha256": CRYSTAL_APPEARANCE_DIGEST,
        },
        "appearance_id": appearance_id,
        "frame_groups": [
            {
                "kind": "object_initial",
                "source_group_id": 2,
                "geometry": {
                    "pattern_width": width,
                    "pattern_height": height,
                    "pattern_depth": 1,
                    "layers": 1,
                    "phase_count": 1,
                    "is_opaque": False,
                },
                "sprite_ids": sprite_ids,
            }
        ],
    }


def dependencies(
    appearance_id,
    definitions=None,
    proficiency_crosswalks=None,
):
    return {
        "definitions": deepcopy(definitions or []),
        "assets": [],
        "presentations": [presentation_asset(appearance_id)],
        "proficiency_crosswalks": deepcopy(proficiency_crosswalks or []),
    }


def physical(weight):
    return {
        "weight": {"value": weight, "unit": "oz"},
        "movable": True,
        "pickupable": True,
    }


def item_base(key, name, profile, item_class, primary, appearance_id, weight):
    return {
        "identity": {"key": key, "revision": "real-source-example-r1"},
        "display_name": name,
        "family_profile": profile,
        "presentation": {"appearance_binding": presentation_ref(appearance_id)},
        "taxonomy": {"item_class": item_class, "primary": primary},
        "physical": physical(weight),
    }


def _page(catalog, title):
    return deepcopy(next(page for page in catalog["pages"] if page["title"] == title))


def _resolve_item_pointer(item, destination):
    current = {"item": item}
    for part in destination[1:].split("/"):
        current = current[int(part)] if isinstance(current, list) else current[part]
    return deepcopy(current)


def _field_evidence(title, item, source_values):
    records = []
    for destination, (route_destination, references) in FIELD_EVIDENCE_SPECS[
        title
    ].items():
        records.append(
            {
                "destination": destination,
                "route_destination": route_destination,
                "normalized_value": _resolve_item_pointer(item, destination),
                "normalization": "typed normalization from pinned source observations",
                "observations": [
                    {
                        "source": source,
                        "field": field,
                        "catalog_field": SOURCE_FIELD_ALIASES.get(
                            (source, field), field
                        ),
                        "raw_value": source_values[source][field],
                    }
                    for source, field in references
                ],
            }
        )
    return records


def source_evidence(title, item, appearance_id, source_values, blockers, defaults):
    br_catalog = build_br_real_item_supplement()
    fandom_catalog = build_fandom_real_item_supplement()
    sprite_ids = APPEARANCES[appearance_id][2]
    return {
        "schema": "OTERYN_ITEM_REAL_SOURCE_EVIDENCE/candidate-3",
        "item_key": item["identity"]["key"],
        "source_identity": {
            "identity_namespace": "client/appearance_id",
            "external_id": str(appearance_id),
            "target": presentation_ref(appearance_id),
        },
        "engine_sources": [
            {
                "source_profile": CANARY_PROFILE,
                "repository": "opentibiabr/canary",
                "revision": CANARY_REVISION,
                "path": "data/items/appearances.dat",
                "digest_sha256": CANARY_APPEARANCE_DIGEST,
                "item_id": appearance_id,
                "appearance_id": appearance_id,
                "sprite_ids": sprite_ids,
            },
            {
                "source_profile": CRYSTAL_PROFILE,
                "repository": "zimbadev/crystalserver",
                "revision": CRYSTAL_REVISION,
                "path": "data/items/appearances.dat",
                "digest_sha256": CRYSTAL_APPEARANCE_DIGEST,
                "item_id": appearance_id,
                "appearance_id": appearance_id,
                "sprite_ids": sprite_ids,
            },
        ],
        "definition_sources": [
            {
                "source_profile": CANARY_PROFILE,
                "repository": "opentibiabr/canary",
                "revision": CANARY_REVISION,
                "path": "data/items/items.xml",
                "digest_sha256": CANARY_ITEMS_DIGEST,
                "source_locator": f"item[@id='{appearance_id}']",
            },
            {
                "source_profile": CRYSTAL_PROFILE,
                "repository": "zimbadev/crystalserver",
                "revision": CRYSTAL_REVISION,
                "path": "data/items/items.xml",
                "digest_sha256": CRYSTAL_ITEMS_DIGEST,
                "source_locator": f"item[@id='{appearance_id}']",
            },
        ],
        "wiki_sources": [
            {"source_id": "br", **_page(br_catalog, title)},
            {"source_id": "fandom", **_page(fandom_catalog, title)},
        ],
        "source_observations": source_values,
        "field_evidence": _field_evidence(title, item, source_values),
        "non_source_defaults": defaults,
        "readiness": {
            "state": "AUTHORING_EVIDENCE_COMPLETE_RUNTIME_BLOCKED",
            "blockers": [
                "sprite_atlas_not_admitted",
                "runtime_lowering_not_implemented",
            ]
            + blockers,
            "non_claims": [
                "not_runtime_imported",
                "not_gameplay_parity_proven",
                "not_renderable_without_matching_sprite_atlas",
            ],
        },
    }


def build_real_item_examples():
    examples = []

    item = item_base(
        "oteryn:item.weapon.sword.magic",
        "Magic Sword",
        "weapon_melee",
        "weapon",
        "sword",
        3288,
        "42.00",
    )
    item.update(
        {
            "requirements": {
                "min_level": 80,
                "enforcement_mode": "on_equip",
            },
            "equipment": {
                "patterns": [
                    {"pattern_id": 1, "slot": "right_hand", "hands": 1},
                    {"pattern_id": 2, "slot": "left_hand", "hands": 1},
                ]
            },
            "weapon": {
                "weapon_type": "sword",
                "attack": 48,
                "defense": 35,
                "extra_defense": 3,
                "consumption_mode": "none",
            },
            "stack": {"stackable": False, "max_count": 1},
            "imbuement": {"slot_count": 2},
            "forge": {"classification": 2, "max_tier": 2},
            "proficiency": magic_sword_proficiency(),
            "trade": {"marketable": True, "market_category": "swords"},
        }
    )
    examples.append(
        {
            "slug": "magic-sword",
            "item": item,
            "dependencies": dependencies(
                3288,
                definitions=[
                    MAGIC_SWORD_PROFICIENCY_REF,
                    INTENSE_WOUND_CLEANSING_REF,
                    BERSERK_REF,
                ],
                proficiency_crosswalks=[
                    proficiency_crosswalk(CANARY_PROFICIENCY_SOURCE),
                    proficiency_crosswalk(CRYSTAL_PROFICIENCY_SOURCE),
                ],
            ),
            "evidence": source_evidence(
                "Magic Sword",
                item,
                3288,
                {
                    "br": {
                        "levelrequired": "80",
                        "hands": "Uma",
                        "type": "Espada",
                        "attack": "48",
                        "defense": "35",
                        "defensemod": "+3",
                        "imbuement": "2",
                        "classificacao": "2",
                        "max_tier": "2",
                        "weight": "42.00",
                    },
                    "fandom": {
                        "name": "Magic Sword",
                        "itemid": "3288",
                        "objectclass": "Weapons",
                        "primarytype": "Sword Weapons",
                        "slot": "Weapon Hand",
                        "immobile": "no",
                        "pickupable": "yes",
                        "levelrequired": "80",
                        "hands": "One",
                        "weapontype": "Sword",
                        "attack": "48",
                        "defense": "35",
                        "defensemod": "+3",
                        "imbueslots": "2",
                        "upgradeclass": "2",
                        "weight": "42.00",
                        "stackable": "no",
                        "marketable": "yes",
                    },
                    "engine_items_xml": {
                        "primarytype": "sword weapons",
                        "weaponType": "sword",
                        "attack": "48",
                        "extradef": "3",
                        "defense": "35",
                        "weight": "4200",
                        "imbuementslot": "2",
                        "level": "80",
                        "slot": "hand",
                    },
                    "engine_appearance": {
                        "market.category": 20,
                    },
                    "canary_appearance": {
                        "proficiency.proficiency_id": 238,
                    },
                    "crystal_appearance": {
                        "proficiency.proficiency_id": 238,
                    },
                },
                [],
                [
                    {
                        "destination": "/item/family_profile",
                        "state": "AUTHOR_SELECTED_FROM_TYPED_ITEM_CAPABILITIES",
                    },
                    {
                        "destination": "/item/physical/weight/unit",
                        "state": "SOURCE_WEIGHT_UNIT_NORMALIZATION",
                    },
                    {
                        "destination": "/item/requirements/enforcement_mode",
                        "state": "PROFILE_ENFORCEMENT_NORMALIZATION",
                    },
                    {
                        "destination": "/item/equipment/patterns/0/pattern_id",
                        "state": "AUTHORING_PATTERN_ID",
                    },
                    {
                        "destination": "/item/equipment/patterns/0/slot",
                        "state": "ENGINE_NORMALIZATION_FROM_AMBIGUOUS_SLOT_HAND",
                    },
                    {
                        "destination": "/item/equipment/patterns/1/pattern_id",
                        "state": "AUTHORING_PATTERN_ID",
                    },
                    {
                        "destination": "/item/equipment/patterns/1/slot",
                        "state": "ENGINE_NORMALIZATION_FROM_SLOT_HAND",
                    },
                    {
                        "destination": "/item/weapon/consumption_mode",
                        "state": "PROFILE_DEFAULT_NO_CONSUMPTION",
                    },
                    {
                        "destination": "/item/stack/max_count",
                        "state": "SCHEMA_NORMALIZATION_FROM_STACKABLE_FALSE",
                    },
                ],
            ),
        }
    )

    item = item_base(
        "oteryn:item.equipment.armor.demon",
        "Demon Armor",
        "equipment_armor",
        "equipment",
        "armor",
        3388,
        "80.00",
    )
    item.update(
        {
            "equipment": {"slot": "armor", "hands": 0},
            "protection": {"armor": 16},
            "stack": {"stackable": False, "max_count": 1},
            "imbuement": {"slot_count": 2},
            "forge": {"classification": 2, "max_tier": 2},
            "trade": {"marketable": True},
        }
    )
    examples.append(
        {
            "slug": "demon-armor",
            "item": item,
            "dependencies": dependencies(3388),
            "evidence": source_evidence(
                "Demon Armor",
                item,
                3388,
                {
                    "br": {
                        "armor": "16",
                        "imbuement": "2",
                        "classificacao": "2",
                        "max_tier": "2",
                        "weight": "80.00",
                    },
                    "fandom": {
                        "name": "Demon Armor",
                        "itemid": "3388",
                        "objectclass": "Body Equipment",
                        "primarytype": "Armors",
                        "slot": "Body",
                        "immobile": "no",
                        "pickupable": "yes",
                        "imbueslots": "2",
                        "upgradeclass": "2",
                        "armor": "16",
                        "weight": "80.00",
                        "stackable": "no",
                        "marketable": "yes",
                    },
                    "engine_items_xml": {
                        "primarytype": "armors",
                        "armor": "16",
                        "weight": "8000",
                        "imbuementslot": "2",
                        "slot": "armor",
                    },
                },
                [],
                [
                    {
                        "destination": "/item/family_profile",
                        "state": "AUTHOR_SELECTED_FROM_TYPED_ITEM_CAPABILITIES",
                    },
                    {
                        "destination": "/item/physical/weight/unit",
                        "state": "SOURCE_WEIGHT_UNIT_NORMALIZATION",
                    },
                    {
                        "destination": "/item/equipment/hands",
                        "state": "SCHEMA_NORMALIZATION_FOR_NON_HAND_SLOT",
                    },
                    {
                        "destination": "/item/stack/max_count",
                        "state": "SCHEMA_NORMALIZATION_FROM_STACKABLE_FALSE",
                    },
                ],
            ),
        }
    )

    item = item_base(
        "oteryn:item.container.backpack",
        "Backpack",
        "container",
        "container",
        "container",
        2854,
        "18.00",
    )
    item.update(
        {
            "taxonomy": {
                "item_class": "container",
                "primary": "container",
                "secondary": "backpack",
            },
            "stack": {"stackable": False, "max_count": 1},
            "container": {"capacity": 20, "content_kind": "items"},
            "imbuement": {"slot_count": 1},
            "trade": {"marketable": True},
        }
    )
    examples.append(
        {
            "slug": "backpack",
            "item": item,
            "dependencies": dependencies(2854),
            "evidence": source_evidence(
                "Backpack",
                item,
                2854,
                {
                    "br": {
                        "primarytype": "Recipientes",
                        "volume": "20",
                        "imbuement": "1",
                        "weight": "18.00",
                    },
                    "fandom": {
                        "name": "Backpack",
                        "itemid": "2854",
                        "primarytype": "Containers",
                        "secondarytype": "Backpacks",
                        "slot": "Container",
                        "volume": "20",
                        "immobile": "no",
                        "pickupable": "yes",
                        "imbueslots": "1",
                        "weight": "18.00",
                        "stackable": "no",
                        "marketable": "yes",
                    },
                    "engine_items_xml": {
                        "primarytype": "containers",
                        "containersize": "20",
                        "weight": "1800",
                        "imbuementslot": "1",
                    },
                },
                [],
                [
                    {
                        "destination": "/item/family_profile",
                        "state": "AUTHOR_SELECTED_FROM_TYPED_ITEM_CAPABILITIES",
                    },
                    {
                        "destination": "/item/taxonomy/item_class",
                        "state": "AUTHOR_SELECTED_FROM_CONTAINER_CAPABILITY",
                    },
                    {
                        "destination": "/item/physical/weight/unit",
                        "state": "SOURCE_WEIGHT_UNIT_NORMALIZATION",
                    },
                    {
                        "destination": "/item/container/content_kind",
                        "state": "NORMALIZATION_FROM_CONTAINER_TAXONOMY",
                    },
                    {
                        "destination": "/item/stack/max_count",
                        "state": "SCHEMA_NORMALIZATION_FROM_STACKABLE_FALSE",
                    },
                ],
            ),
        }
    )

    item = item_base(
        "oteryn:item.food.red-apple",
        "Red Apple",
        "food",
        "consumable",
        "food",
        3585,
        "1.50",
    )
    item.update(
        {
            "stack": {"stackable": True, "max_count": 100},
            "consumable": {
                "edible": True,
                "regeneration_seconds": 72,
                "consume_count": 1,
            },
            "use": {"usable": True, "use_with": False},
            "trade": {"marketable": True},
        }
    )
    examples.append(
        {
            "slug": "red-apple",
            "item": item,
            "dependencies": dependencies(3585),
            "evidence": source_evidence(
                "Red Apple",
                item,
                3585,
                {
                    "br": {
                        "edible": "sim",
                        "regenseconds": "72",
                        "stackable": "sim",
                        "weight": "1.50",
                        "sounds": "Yum.",
                    },
                    "fandom": {
                        "name": "Red Apple",
                        "itemid": "3585",
                        "objectclass": "Plants, Animal Products, Food and Drink",
                        "primarytype": "Food",
                        "immobile": "no",
                        "pickupable": "yes",
                        "usable": "yes",
                        "weight": "1.50",
                        "stackable": "yes",
                        "consumable": "yes",
                        "regenseconds": "72",
                        "marketable": "yes",
                    },
                    "engine_items_xml": {
                        "primarytype": "food",
                        "weight": "150",
                    },
                },
                ["yum_sound_asset_not_admitted"],
                [
                    {
                        "destination": "/item/family_profile",
                        "state": "AUTHOR_SELECTED_FROM_TYPED_ITEM_CAPABILITIES",
                    },
                    {
                        "destination": "/item/taxonomy/item_class",
                        "state": "AUTHOR_SELECTED_FROM_FOOD_CAPABILITY",
                    },
                    {
                        "destination": "/item/physical/weight/unit",
                        "state": "SOURCE_WEIGHT_UNIT_NORMALIZATION",
                    },
                    {
                        "destination": "/item/consumable/consume_count",
                        "state": "PROFILE_DEFAULT_NOT_SOURCE_VERIFIED",
                    },
                    {
                        "destination": "/item/stack/max_count",
                        "state": "PROFILE_DEFAULT_NOT_SOURCE_VERIFIED",
                    },
                    {
                        "destination": "/item/use/use_with",
                        "state": "NORMALIZATION_FROM_SINGLE_TARGET_CONSUMPTION",
                    },
                ],
            ),
        }
    )

    item = item_base(
        "oteryn:item.rune.sudden-death",
        "Sudden Death Rune",
        "rune",
        "rune",
        "attack",
        3155,
        "0.70",
    )
    item.update(
        {
            "taxonomy": {
                "item_class": "rune",
                "primary": "attack",
                "secondary": "attack",
            },
            "stack": {"stackable": True, "max_count": 100},
            "charges": {"count": 3},
            "requirements": {
                "min_level": 45,
                "min_magic_level": 15,
                "premium_only": False,
                "enforcement_mode": "on_use",
            },
            "trade": {"marketable": True},
        }
    )
    examples.append(
        {
            "slug": "sudden-death-rune",
            "item": item,
            "dependencies": dependencies(3155),
            "evidence": source_evidence(
                "Sudden Death Rune",
                item,
                3155,
                {
                    "br": {
                        "Subclass": "Ataque",
                        "damagetype": "Death",
                        "premium": "não",
                        "mlrequired": "15",
                        "levelrequired": "45",
                        "makeqty": "3",
                        "weight": "0.70",
                    },
                    "fandom": {
                        "name": "Sudden Death Rune",
                        "itemid": "3155",
                        "objectclass": "Runes",
                        "primarytype": "Attack Runes",
                        "immobile": "no",
                        "pickupable": "yes",
                        "levelrequired": "45",
                        "mlrequired": "15",
                        "weight": "0.70",
                        "stackable": "yes",
                        "marketable": "yes",
                        "basepower": "150",
                    },
                    "engine_items_xml": {
                        "primarytype": "attack runes",
                        "type": "rune",
                        "runespellname": "adori gran mort",
                        "charges": "3",
                        "weight": "70",
                    },
                },
                ["ability_binding_not_admitted", "combat_formula_not_admitted"],
                [
                    {
                        "destination": "/item/family_profile",
                        "state": "AUTHOR_SELECTED_FROM_TYPED_ITEM_CAPABILITIES",
                    },
                    {
                        "destination": "/item/physical/weight/unit",
                        "state": "SOURCE_WEIGHT_UNIT_NORMALIZATION",
                    },
                    {
                        "destination": "/item/stack/max_count",
                        "state": "PROFILE_DEFAULT_NOT_SOURCE_VERIFIED",
                    },
                    {
                        "destination": "/item/requirements/enforcement_mode",
                        "state": "PROFILE_ENFORCEMENT_NORMALIZATION",
                    },
                ],
            ),
        }
    )

    item = item_base(
        "oteryn:item.fluid-container.vial",
        "Vial",
        "fluid",
        "fluid_container",
        "vial",
        2874,
        "1.80",
    )
    item.update(
        {
            "stack": {"stackable": False, "max_count": 1},
            "fluid": {"role": "container"},
            "trade": {"marketable": True},
        }
    )
    examples.append(
        {
            "slug": "vial",
            "item": item,
            "dependencies": dependencies(2874),
            "evidence": source_evidence(
                "Vial",
                item,
                2874,
                {
                    "br": {
                        "primarytype": "Recipientes Líquidos",
                        "attrib": "Carrega líquido.",
                        "weight": "1.80",
                    },
                    "fandom": {
                        "name": "Vial",
                        "itemid": "2874",
                        "objectclass": "Household Items",
                        "primarytype": "Fluid Containers",
                        "immobile": "no",
                        "pickupable": "yes",
                        "holdsliquid": "yes",
                        "weight": "1.80",
                        "stackable": "no",
                        "marketable": "yes",
                    },
                    "engine_items_xml": {
                        "primarytype": "fluid containers",
                        "allowpickUpAble": "1",
                        "weight": "180",
                    },
                },
                ["fluid_interaction_binding_not_admitted"],
                [
                    {
                        "destination": "/item/family_profile",
                        "state": "AUTHOR_SELECTED_FROM_TYPED_ITEM_CAPABILITIES",
                    },
                    {
                        "destination": "/item/taxonomy/item_class",
                        "state": "AUTHOR_SELECTED_FROM_FLUID_CAPABILITY",
                    },
                    {
                        "destination": "/item/taxonomy/primary",
                        "state": "AUTHOR_SELECTED_CANONICAL_KIND",
                    },
                    {
                        "destination": "/item/physical/weight/unit",
                        "state": "SOURCE_WEIGHT_UNIT_NORMALIZATION",
                    },
                    {
                        "destination": "/item/stack/max_count",
                        "state": "SCHEMA_NORMALIZATION_FROM_STACKABLE_FALSE",
                    },
                ],
            ),
        }
    )

    return {
        "schema": "OTERYN_ITEM_AUTHORING_REAL_SOURCE_EXAMPLES/candidate-3",
        "scope": "authoring examples and evidence only; not runtime import artifacts",
        "examples": examples,
    }
