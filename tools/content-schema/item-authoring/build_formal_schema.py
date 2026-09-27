"""Build the Item authoring schema candidate v4. This is not runtime serialization."""

import copy
import json
from pathlib import Path

from proficiency_profiles import (
    CANARY_PROFICIENCY_SOURCE,
    CRYSTAL_PROFICIENCY_SOURCE,
)
from real_item_examples import build_real_item_examples
from source_field_catalogs import (
    BR_PROFILE,
    BR_REAL_ITEM_PROFILE,
    CANARY_PROFILE,
    CRYSTAL_PROFILE,
    FANDOM_PROFILE,
    FANDOM_REAL_ITEM_PROFILE,
    build_br_real_item_supplement,
    build_engine_catalog,
    build_fandom_catalog,
    build_fandom_real_item_supplement,
)

ROOT = Path(__file__).resolve().parent
TEMPLATES = ROOT / "templates"
REPOSITORY_ROOT = ROOT.parents[2]
WIKI_CENSUS = (
    REPOSITORY_ROOT
    / "docs/agents/evidence/OTV2-20260925-tibiawiki-item-master-field-census-v1.json"
)
DIALECT = "https://json-schema.org/draft/2020-12/schema"
ITEM_ID = "urn:oteryn:item-authoring:candidate:4"
DEPS_ID = "urn:oteryn:item-dependencies:candidate:4"
MANIFEST_ID = "urn:oteryn:item-import-readiness:candidate:4"
EVIDENCE_ID = "urn:oteryn:item-real-source-evidence:candidate:4"


def obj(properties, required=(), **extra):
    return {
        "type": "object",
        "additionalProperties": False,
        "properties": properties,
        "required": list(required),
        **extra,
    }


def array(items, minimum=0, unique=False, **extra):
    return {
        "type": "array",
        "items": items,
        "minItems": minimum,
        **({"uniqueItems": True} if unique else {}),
        **extra,
    }


def integer(minimum=0, maximum=None, **extra):
    return {
        "type": "integer",
        "minimum": minimum,
        **({"maximum": maximum} if maximum is not None else {}),
        **extra,
    }


def text(minimum=1, **extra):
    return {"type": "string", "minLength": minimum, **extra}


def enum(*values):
    return {"enum": list(values)}


def use(name):
    return {"$ref": "#/$defs/" + name}


def forbid(*names):
    return {"not": {"anyOf": [{"required": [name]} for name in names]}}


def reference(family):
    return obj(
        {"family": {"const": family}, "key": use("key"), "revision": use("revision")},
        ("family", "key", "revision"),
    )


PROFILES = {
    "equipment_armor": [
        "physical",
        "equipment",
        "protection",
        "modifiers",
        "imbuement",
        "forge",
        "trade",
    ],
    "equipment_offhand": [
        "physical",
        "equipment",
        "protection",
        "modifiers",
        "charges",
        "temporal",
        "imbuement",
        "forge",
        "trade",
    ],
    "container_equipment": ["physical", "equipment", "container", "trade"],
    "weapon_melee": [
        "physical",
        "equipment",
        "weapon",
        "modifiers",
        "imbuement",
        "forge",
        "proficiency",
        "trade",
    ],
    "weapon_distance": [
        "physical",
        "equipment",
        "weapon",
        "stack",
        "modifiers",
        "imbuement",
        "forge",
        "trade",
    ],
    "weapon_magic": [
        "physical",
        "equipment",
        "weapon",
        "use",
        "requirements",
        "modifiers",
        "trade",
    ],
    "rune": ["physical", "stack", "charges", "use", "requirements", "trade"],
    "document": ["physical", "readable", "presentation", "trade"],
    "container": ["physical", "container", "trade"],
    "decoration": ["physical", "presentation", "use", "light", "lifecycle", "trade"],
    "event_collectible": ["physical", "presentation", "trade"],
    "progression_material": ["physical", "stack", "use", "trade"],
    "transformation_item": [
        "physical",
        "lifecycle",
        "use",
        "temporal",
        "charges",
        "trade",
    ],
    "quest_item": [
        "physical",
        "use",
        "readable",
        "container",
        "fluid",
        "light",
        "lifecycle",
        "presentation",
    ],
    "material_valuable": ["physical", "stack", "trade"],
    "trash": ["physical", "use"],
    "key": ["physical", "use"],
    "light_source": [
        "physical",
        "light",
        "use",
        "temporal",
        "lifecycle",
        "presentation",
    ],
    "tool": ["physical", "use", "charges", "temporal", "lifecycle", "trade"],
    "food": ["physical", "stack", "consumable", "use", "trade"],
    "fluid": ["physical", "fluid", "use", "requirements", "trade"],
    "plant": ["physical", "stack", "consumable", "use", "trade"],
}


COMMON_CAPABILITIES = {
    "equipment_armor": ["physical", "equipment", "protection", "trade"],
    "equipment_offhand": ["physical", "equipment", "trade"],
    "container_equipment": ["physical", "equipment", "container", "trade"],
    "weapon_melee": ["physical", "equipment", "weapon", "trade"],
    "weapon_distance": ["physical", "weapon", "trade"],
    "weapon_magic": ["physical", "equipment", "weapon", "use", "requirements", "trade"],
    "rune": ["physical", "stack", "charges", "use", "requirements", "trade"],
    "document": ["physical", "readable", "presentation", "trade"],
    "container": ["physical", "container", "trade"],
    "decoration": ["physical", "presentation", "trade"],
    "event_collectible": ["physical", "presentation", "trade"],
    "progression_material": ["physical", "trade"],
    "transformation_item": ["physical", "lifecycle", "use"],
    "quest_item": ["physical"],
    "material_valuable": ["physical", "trade"],
    "trash": ["physical"],
    "key": ["physical", "use"],
    "light_source": ["physical", "light"],
    "tool": ["physical", "use", "trade"],
    "food": ["physical", "consumable", "use", "trade"],
    "fluid": ["physical", "fluid", "use", "trade"],
    "plant": ["physical", "trade"],
}


FAMILY_ASSIGNMENTS = {
    "equipment_armor": ["Capacetes", "Botas", "Armaduras", "Calças"],
    "equipment_offhand": [
        "Escudos",
        "Spellbooks",
        "Extra Slot",
        "Amuletos e Colares",
        "Anéis",
    ],
    "container_equipment": ["Aljavas"],
    "weapon_melee": ["Machados", "Clavas", "Espadas", "Punhos", "Réplicas de Armas"],
    "weapon_distance": ["Distância", "Munição"],
    "weapon_magic": ["Wands", "Rods", "Antigas Wands e Rods"],
    "rune": ["Runas", "Runas de Decoração"],
    "document": ["Livros", "Documentos e Papéis"],
    "container": ["Recipientes"],
    "decoration": [
        "Decorações",
        "Dolls e Bears",
        "Troféus",
        "Instrumentos Musicais",
        "Jogos e Diversão",
    ],
    "event_collectible": ["Prêmios de Eventos", "Itens de Fansites", "Itens de Festa"],
    "progression_material": [
        "Itens de Addons",
        "Itens de Imbuements",
        "Delivery Tasks",
    ],
    "transformation_item": ["Itens Encantados"],
    "quest_item": ["Itens de Quest"],
    "material_valuable": ["Cristais (Itens)", "Valiosos", "Produtos de Criaturas"],
    "trash": ["Lixos"],
    "key": ["Chaves"],
    "light_source": ["Fontes de Luz"],
    "tool": ["Ferramentas", "Ferramentas de Cozinha", "Itens de Domar"],
    "food": ["Comidas"],
    "fluid": ["Líquidos"],
    "plant": ["Plantas e Ervas"],
}


WIKI_FORMAL_DESTINATIONS = {
    "name": ["/item/display_name"],
    "mana": ["/item/use/mana_cost"],
    "skillboost": ["/item/modifiers/skill_boost"],
    "resist": ["/item/protection/resistances"],
    "modificadores": ["/item/modifiers"],
    "volume": ["/item/container/capacity"],
    "armor": ["/item/protection/armor"],
    "attack": ["/item/weapon/attack"],
    "elementattack": ["/item/weapon/elemental_attack"],
    "range": ["/item/weapon/range_cells"],
    "hit": ["/item/weapon/hit_chance_percent"],
    "defense": ["/item/weapon/defense"],
    "defensemod": ["/item/weapon/extra_defense"],
    "charges": ["/item/charges/count"],
    "mantra": ["/item/modifiers/mantra"],
    "vocrequired": [
        "/item/requirements/vocations",
        "/item/equipment/patterns/*/vocations",
    ],
    "levelrequired": [
        "/item/requirements/min_level",
        "/item/equipment/patterns/*/min_level",
    ],
    "augments": ["/item/proficiency/augments"],
    "imbuement": ["/item/imbuement/slot_count"],
    "classificacao": ["/item/forge/classification"],
    "weight": ["/item/physical/weight", "/item/presentation/display_weight"],
    "elemental_bond": ["/item/modifiers/elemental_bond"],
    "flavortext": ["/item/presentation/flavor_text"],
    "attrib": ["/item/source_observations/attributes_text"],
    "hands": ["/item/equipment/hands", "/item/equipment/patterns/*/hands"],
    "enchantable": ["/item/lifecycle/enchantable"],
    "enchanted": ["/item/lifecycle/enchanted_variant"],
    "stackable": ["/item/stack/stackable"],
    "duration": ["/item/temporal/duration_ms"],
    "damagetype": ["/item/use/damage_type"],
    "damage": ["/item/use/damage"],
    "destructible": ["/item/lifecycle/destructible"],
    "writable": ["/item/readable/writable", "/item/readable/write_policy"],
    "readable": ["/item/readable/readable"],
    "edible": ["/item/consumable/edible"],
    "sounds": ["/item/presentation/sounds"],
    "mercado": ["/item/trade/marketable"],
    "notes": ["/item/editor/notes"],
    "perk1": ["/item/proficiency/levels/*/perks/*"],
    "perk2": ["/item/proficiency/levels/*/perks/*"],
    "perk3": ["/item/proficiency/levels/*/perks/*"],
    "perk4": ["/item/proficiency/levels/*/perks/*"],
    "perk5": ["/item/proficiency/levels/*/perks/*"],
    "perk6": ["/item/proficiency/levels/*/perks/*"],
    "perk7": ["/item/proficiency/levels/*/perks/*"],
    "regenseconds": ["/item/consumable/regeneration_seconds"],
    "primarytype": ["/item/taxonomy/primary"],
    "secondarytype": ["/item/taxonomy/secondary"],
    "tertiarytype": ["/item/taxonomy/tertiary"],
    "itemclass": ["/item/taxonomy/item_class"],
    "type": ["/item/weapon/weapon_type"],
    "writechars": ["/item/readable/max_characters"],
    "value": ["/item/editor/estimated_value"],
}


ATTRIB_PROMOTION_PREFIXES = (
    "/item/light/",
    "/item/use/",
    "/item/lifecycle/",
    "/item/presentation/",
    "/item/stack/",
    "/item/equipment/",
    "/item/readable/",
    "/item/temporal/",
)


def build_item_schema():
    d = {}
    d["key"] = text(
        pattern=r"^[a-z0-9_.-]+:[a-z0-9_.:/-]+$",
        maxLength=512,
        description="Namespaced Oteryn authoring key; source numeric IDs are provenance only.",
    )
    d["revision"] = text(pattern=r"^[A-Za-z0-9_.:-]+$", maxLength=512)
    d["identity"] = obj(
        {"key": use("key"), "revision": use("revision")}, ("key", "revision")
    )
    for family in (
        "Item",
        "Ability",
        "Effect",
        "Interaction",
        "Document",
        "Proficiency",
        "Presentation",
    ):
        d[family + "Ref"] = reference(family)
    d["bool"] = {"type": "boolean"}
    d["ms"] = integer(
        1, description="Positive duration in milliseconds; no implicit unit conversion."
    )
    d["ratio"] = obj(
        {"numerator": {"type": "integer"}, "denominator": integer(1)},
        ("numerator", "denominator"),
        description="Exact rational in lowest terms.",
    )
    d["nonnegativeRatio"] = copy.deepcopy(d["ratio"])
    d["nonnegativeRatio"]["properties"]["numerator"] = integer()
    d["positiveRatio"] = copy.deepcopy(d["ratio"])
    d["positiveRatio"]["properties"]["numerator"] = integer(1)
    d["percent"] = copy.deepcopy(d["nonnegativeRatio"])
    d["percent"]["description"] = (
        "Exact percentage points in [0,100], enforced semantically."
    )
    d["damageType"] = enum(
        "physical",
        "energy",
        "earth",
        "fire",
        "life_drain",
        "mana_drain",
        "drowning",
        "ice",
        "holy",
        "death",
        "agony",
        "neutral",
        "healing",
    )
    d["assetBinding"] = use("key")
    d["weight"] = obj(
        {
            "value": text(pattern=r"^(?:0|[1-9][0-9]*)\.[0-9]{2}$", maxLength=64),
            "unit": enum("oz", "g"),
        },
        ("value", "unit"),
        description=(
            "Exact nonnegative decimal string with exactly two fractional digits"
            " plus explicit unit; 42.00 oz is engine weight 4200 (centi-ounces)."
        ),
    )
    d["displayWeight"] = obj(
        {
            "value": text(
                pattern=r"^(?!-0\.00$)-?(?:0|[1-9][0-9]*)\.[0-9]{2}$", maxLength=64
            ),
            "unit": enum("oz", "g"),
        },
        ("value", "unit"),
        description="Signed source display override only; never gameplay mass.",
    )
    d["displayFlags"] = obj(
        {
            "stack_count": use("bool"),
            "duration": use("bool"),
            "attributes": use("bool"),
            "client_expiry_timer": use("bool"),
            "client_wear_counter": use("bool"),
        }
    )
    d["presentation"] = obj(
        {
            "grammar": obj({"article": text(0), "plural": text()}),
            "inspection_description": text(),
            "flavor_text": text(),
            "sounds": array(d["assetBinding"], unique=True),
            "appearance_binding": use("PresentationRef"),
            "effects": array(d["assetBinding"], unique=True),
            "projectile_effect": d["assetBinding"],
            "attack_effect": d["assetBinding"],
            "variants": array(use("ItemRef"), unique=True),
            "display_weight": use("displayWeight"),
            "display_flags": use("displayFlags"),
        }
    )
    d["taxonomy"] = obj(
        {
            "item_class": text(pattern=r"^[a-z][a-z0-9_]*$"),
            "primary": text(),
            "secondary": text(),
            "tertiary": text(),
            "tags": array(text(pattern=r"^[a-z][a-z0-9_.-]*$"), unique=True),
        },
        ("item_class", "primary"),
    )
    d["physical"] = obj(
        {"weight": use("weight"), "movable": use("bool"), "pickupable": use("bool")}
    )
    d["stack"] = obj(
        {"stackable": use("bool"), "max_count": integer(1)},
        ("stackable", "max_count"),
        allOf=[
            {
                "if": {
                    "properties": {"stackable": {"const": True}},
                    "required": ["stackable"],
                },
                "then": {"properties": {"max_count": {"minimum": 2}}},
                "else": {"properties": {"max_count": {"const": 1}}},
            }
        ],
    )
    d["requirements"] = obj(
        {
            "min_level": integer(),
            "min_magic_level": integer(),
            "vocations": array(text(pattern=r"^[a-z][a-z0-9_]*$"), unique=True),
            "context": text(),
            "enforcement_mode": enum("on_equip", "on_use", "both"),
            "premium_only": use("bool"),
            "level_magic_shortfall": obj(
                {
                    "policy": {"const": "allow_with_multiplicative_damage_penalty"},
                    "damage_multiplier_per_failed_check": obj(
                        {"numerator": {"const": 1}, "denominator": {"const": 2}},
                        ("numerator", "denominator"),
                    ),
                },
                ("policy", "damage_multiplier_per_failed_check"),
            ),
        },
        ("enforcement_mode",),
    )
    d["equipmentPattern"] = obj(
        {
            "pattern_id": integer(),
            "slot": enum(
                "head",
                "neck",
                "back",
                "armor",
                "right_hand",
                "left_hand",
                "legs",
                "feet",
                "ring",
                "ammo",
                "extra",
            ),
            "hands": integer(0, 2),
            "reserved_slots": array(
                enum(
                    "head",
                    "neck",
                    "back",
                    "armor",
                    "right_hand",
                    "left_hand",
                    "legs",
                    "feet",
                    "ring",
                    "ammo",
                    "extra",
                ),
                unique=True,
            ),
            "groups": array(text(pattern=r"^[a-z][a-z0-9_]*$"), unique=True),
            "vocations": array(text(pattern=r"^[a-z][a-z0-9_]*$"), unique=True),
            "min_level": integer(),
        },
        ("pattern_id", "slot", "hands"),
    )
    d["equipment"] = obj(
        {
            **copy.deepcopy(d["equipmentPattern"]["properties"]),
            "patterns": array(use("equipmentPattern"), 1),
            "dual_wielding": use("bool"),
            "activation_events": array(enum("equip", "unequip"), 1, unique=True),
        },
        oneOf=[
            {"required": ["slot", "hands"], "not": {"required": ["patterns"]}},
            {
                "required": ["patterns"],
                "not": {
                    "anyOf": [
                        {"required": [name]}
                        for name in (
                            "pattern_id",
                            "slot",
                            "hands",
                            "reserved_slots",
                            "groups",
                            "vocations",
                            "min_level",
                        )
                    ]
                },
            },
        ],
        description="Use the compact slot/hands form for one pattern or patterns[] when equip rules differ by vocation/level.",
    )
    d["elementalAttack"] = obj(
        {"damage_type": use("damageType"), "amount": integer()},
        ("damage_type", "amount"),
    )
    d["damageRange"] = obj(
        {"minimum": {"type": "integer"}, "maximum": {"type": "integer"}},
        ("minimum", "maximum"),
    )
    d["chainTargeting"] = obj(
        {
            "max_targets": integer(2),
            "range_cells": integer(1),
            "falloff_percent": use("percent"),
        },
        ("max_targets", "range_cells"),
    )
    d["chain"] = {
        "oneOf": [
            obj({"mode": {"const": "disabled"}}, ("mode",)),
            obj(
                {
                    "mode": {"const": "override"},
                    "skill_formula_coefficient": use("positiveRatio"),
                    "targeting": use("chainTargeting"),
                },
                ("mode", "skill_formula_coefficient"),
            ),
        ],
        "description": "Absent inherits engine defaults; disabled and explicit coefficient are distinct.",
    }
    d["weapon"] = obj(
        {
            "weapon_type": enum(
                "sword",
                "axe",
                "club",
                "fist",
                "distance_launcher",
                "ammunition",
                "thrown_missile",
                "wand",
                "rod",
                "shield",
                "spellbook",
            ),
            "ammunition_kind": text(pattern=r"^[a-z][a-z0-9_]*$"),
            "attack": {"type": "integer"},
            "attack_modifier": {"type": "integer"},
            "defense": {"type": "integer"},
            "extra_defense": {"type": "integer"},
            "range_cells": integer(),
            "hit_chance_percent": use("percent"),
            "hit_chance_modifier_percent": use("ratio"),
            "max_hit_chance_percent": use("percent"),
            "damage_range": use("damageRange"),
            "damage_type": use("damageType"),
            "elemental_attack": array(use("elementalAttack")),
            "consumption_mode": enum(
                "none",
                "consume_ammunition",
                "consume_charge",
                "consume_item",
                "move_item_to_impact_tile",
            ),
            "break_chance_percent": use("percent"),
            "chain": use("chain"),
        },
        ("weapon_type",),
    )
    d["resistance"] = obj(
        {
            "damage_type": use("damageType"),
            "reduction_percent": use("ratio"),
            "scope": enum("direct", "field", "all"),
        },
        ("damage_type", "reduction_percent", "scope"),
    )
    d["conditionSuppression"] = obj(
        {
            "condition": text(pattern=r"^[a-z][a-z0-9_]*$"),
            "scope": enum("direct", "field", "all"),
        },
        ("condition", "scope"),
    )
    d["protection"] = obj(
        {
            "armor": {"type": "integer"},
            "resistances": array(use("resistance")),
            "condition_suppressions": array(use("conditionSuppression")),
        }
    )
    d["skillBoost"] = obj(
        {"skill": text(pattern=r"^[a-z][a-z0-9_]*$"), "amount": {"type": "integer"}},
        ("skill", "amount"),
    )
    d["capacityModifier"] = obj(
        {
            "resource": enum("health", "mana", "capacity", "soul"),
            "flat": {"type": "integer"},
            "percent": use("ratio"),
        },
        ("resource",),
        anyOf=[{"required": ["flat"]}, {"required": ["percent"]}],
    )
    d["regenerationModifier"] = obj(
        {
            "resource": enum("health", "mana"),
            "amount": integer(1),
            "interval_ms": use("ms"),
        },
        ("resource", "amount", "interval_ms"),
    )
    d["leechModifier"] = obj(
        {
            "resource": enum("health", "mana"),
            "chance_percent": use("percent"),
            "amount_percent": use("percent"),
        },
        ("resource", "chance_percent", "amount_percent"),
    )
    d["magicLevelModifier"] = obj(
        {
            "combat_type": {"oneOf": [use("damageType"), {"const": "all"}]},
            "amount": {"type": "integer"},
            "percent": use("ratio"),
        },
        ("combat_type",),
        oneOf=[
            {"required": ["amount"], "not": {"required": ["percent"]}},
            {"required": ["percent"], "not": {"required": ["amount"]}},
        ],
    )
    d["reflection"] = obj(
        {
            "damage_type": use("damageType"),
            "flat": {"type": "integer"},
            "percent": use("percent"),
        },
        ("damage_type",),
        anyOf=[{"required": ["flat"]}, {"required": ["percent"]}],
    )
    d["magicShieldCapacity"] = obj(
        {"flat": {"type": "integer"}, "percent": use("ratio")},
        anyOf=[{"required": ["flat"]}, {"required": ["percent"]}],
    )
    d["movementSpeed"] = obj(
        {
            "flat": {
                "type": "integer",
                "minimum": -2147483648,
                "maximum": 2147483647,
            },
            "unit": {"const": "speed_points"},
        },
        ("flat", "unit"),
    )
    d["mantra"] = obj(
        {
            "points": {"type": "integer", "minimum": -32768, "maximum": 32767},
            "damage_types": array(
                enum("energy", "fire", "earth", "ice"), 1, unique=True
            ),
        },
        ("points", "damage_types"),
    )
    d["elementalBond"] = obj(
        {"damage_type": enum("physical", "energy", "earth")},
        ("damage_type",),
    )
    d["modifiers"] = obj(
        {
            "skill_boost": array(use("skillBoost")),
            "resource_capacity": array(use("capacityModifier")),
            "regeneration": array(use("regenerationModifier")),
            "critical_chance_percent": use("percent"),
            "critical_damage_percent": use("ratio"),
            "leech": array(use("leechModifier")),
            "magic_level": array(use("magicLevelModifier")),
            "mana_shield_enabled": use("bool"),
            "magic_shield_capacity": use("magicShieldCapacity"),
            "perfect_shot_bonus": array(
                obj(
                    {"range_cells": integer(1), "damage": {"type": "integer"}},
                    ("range_cells", "damage"),
                )
            ),
            "cleave_percent": use("percent"),
            "reflection": array(use("reflection")),
            "movement_speed": use("movementSpeed"),
            "invisibility_enabled": use("bool"),
            "mantra": use("mantra"),
            "elemental_bond": use("elementalBond"),
            "editor_notes": array(
                text(),
                unique=True,
                description="Non-executable bounded notes; not a gameplay attribute bag.",
            ),
        }
    )
    d["charges"] = obj({"count": integer(1), "show_count": use("bool")}, ("count",))
    d["temporal"] = obj(
        {
            "duration_ms": use("ms"),
            "consumption_mode": enum("on_use", "on_equip", "continuous"),
            "stop_duration_while_unequipped": use("bool"),
            "decay_target": use("ItemRef"),
        },
        ("duration_ms", "consumption_mode"),
    )
    d["container"] = obj(
        {
            "capacity": integer(1),
            "content_kind": enum("items", "ammunition", "fluids"),
            "accepts": array(text(pattern=r"^[a-z][a-z0-9_.-]*$"), unique=True),
        },
        ("capacity", "content_kind"),
    )
    d["imbuementTier"] = obj(
        {"family": text(pattern=r"^[a-z][a-z0-9_]*$"), "tier": integer(1)},
        ("family", "tier"),
    )
    d["imbuement"] = obj(
        {
            "slot_count": integer(),
            "allowed_family_tiers": array(use("imbuementTier")),
            "excluded_families": array(text(pattern=r"^[a-z][a-z0-9_]*$"), unique=True),
        },
        ("slot_count",),
    )
    d["forge"] = obj(
        {"classification": enum(1, 2, 3, 4), "max_tier": integer(1)},
        ("classification", "max_tier"),
    )
    d["augmentValue"] = {
        "oneOf": [
            obj(
                {"kind": {"const": "boolean"}, "value": use("bool")}, ("kind", "value")
            ),
            obj(
                {"kind": {"const": "signed_points"}, "value": {"type": "integer"}},
                ("kind", "value"),
            ),
            obj(
                {"kind": {"const": "rational_percent"}, "value": use("ratio")},
                ("kind", "value"),
            ),
            obj(
                {"kind": {"const": "milliseconds"}, "value": use("ms")},
                ("kind", "value"),
            ),
            obj(
                {
                    "kind": {"const": "signed_milliseconds"},
                    "value": {"type": "integer"},
                },
                ("kind", "value"),
            ),
            obj({"kind": {"const": "cells"}, "value": integer()}, ("kind", "value")),
            obj({"kind": {"const": "count"}, "value": integer()}, ("kind", "value")),
        ]
    }
    d["augmentTarget"] = {
        "oneOf": [
            obj(
                {"kind": {"const": "ability"}, "ability": use("AbilityRef")},
                ("kind", "ability"),
            ),
            obj({"kind": {"const": "auto_attack"}}, ("kind",)),
            obj(
                {
                    "kind": {"const": "skill_scaled_auto_attack"},
                    "skill": text(pattern=r"^[a-z][a-z0-9_]*$"),
                },
                ("kind", "skill"),
            ),
            obj(
                {
                    "kind": {"const": "skill_scaled_spell_healing"},
                    "skill": text(pattern=r"^[a-z][a-z0-9_]*$"),
                },
                ("kind", "skill"),
            ),
            obj({"kind": {"const": "weapon_shield_modifier"}}, ("kind",)),
            obj({"kind": {"const": "offensive_rune"}}, ("kind",)),
            obj(
                {"kind": {"const": "creature_class"}, "class_key": text()},
                ("kind", "class_key"),
            ),
            obj(
                {"kind": {"const": "generic"}, "target_key": text()},
                ("kind", "target_key"),
            ),
        ]
    }
    d["augmentRankValue"] = obj(
        {"rank": integer(), "value": use("augmentValue")}, ("rank", "value")
    )
    d["augment"] = obj(
        {
            "selection_slot": integer(1),
            "key": text(pattern=r"^[a-z][a-z0-9_.-]*$"),
            "target": use("augmentTarget"),
            "effect": use("EffectRef"),
            "value": use("augmentValue"),
            "rank_values": array(use("augmentRankValue")),
        },
        ("key", "target"),
    )
    d["proficiencyLevel"] = obj(
        {
            "level": integer(1),
            "selection_count": {"const": 1},
            "perks": array(use("augment"), 1),
        },
        ("level", "selection_count", "perks"),
    )
    d["proficiencyShaping"] = obj(
        {
            "max_rank": integer(),
            "replace_slots": integer(),
            "refine_enabled": use("bool"),
            "reshape_enabled": use("bool"),
            "clear_enabled": use("bool"),
            "lunar_ascension_enabled": use("bool"),
            "cost_service": use("InteractionRef"),
        },
        (
            "max_rank",
            "replace_slots",
            "refine_enabled",
            "reshape_enabled",
            "clear_enabled",
            "lunar_ascension_enabled",
        ),
    )
    d["proficiency"] = obj(
        {
            "profile_binding": use("ProficiencyRef"),
            "levels": array(use("proficiencyLevel")),
            "shaping": use("proficiencyShaping"),
            "augments": array(use("augment")),
        },
        anyOf=[
            {"required": ["profile_binding"]},
            {"required": ["levels"]},
            {"required": ["augments"]},
        ],
    )
    d["consumable"] = obj(
        {
            "edible": use("bool"),
            "regeneration_seconds": integer(),
            "consume_count": integer(1),
        },
        ("edible", "consume_count"),
    )
    d["fluid"] = obj(
        {
            "role": enum("content", "container"),
            "fluid_type": text(pattern=r"^[a-z][a-z0-9_]*$"),
            "default_content": text(pattern=r"^[a-z][a-z0-9_]*$"),
        },
        ("role",),
    )
    d["readable"] = obj(
        {
            "readable": use("bool"),
            "writable": use("bool"),
            "write_policy": enum("none", "write_once", "rewrite"),
            "max_characters": integer(1),
            "distance_readable": use("bool"),
            "document_binding": use("DocumentRef"),
            "write_once_target": use("ItemRef"),
        },
        ("readable", "writable", "write_policy"),
        allOf=[
            {
                "if": {
                    "properties": {"writable": {"const": False}},
                    "required": ["writable"],
                },
                "then": {
                    "properties": {"write_policy": {"const": "none"}},
                    **forbid("write_once_target"),
                },
                "else": {
                    "properties": {"write_policy": {"enum": ["write_once", "rewrite"]}},
                    "required": ["max_characters"],
                },
            },
            {
                "if": {
                    "properties": {"write_policy": {"const": "write_once"}},
                    "required": ["write_policy"],
                },
                "then": {"required": ["write_once_target"]},
                "else": forbid("write_once_target"),
            },
            {
                "if": {
                    "properties": {"writable": {"const": True}},
                    "required": ["writable"],
                },
                "then": {"properties": {"readable": {"const": True}}},
            },
        ],
    )
    d["light"] = obj(
        {
            "emits": use("bool"),
            "color_binding": d["assetBinding"],
            "intensity": integer(),
            "radius_cells": integer(),
            "when_equipped": use("bool"),
            "toggleable": use("bool"),
        },
        ("emits",),
        allOf=[
            {
                "if": {"properties": {"emits": {"const": True}}, "required": ["emits"]},
                "then": {"required": ["color_binding", "intensity"]},
                "else": forbid(
                    "color_binding",
                    "intensity",
                    "radius_cells",
                    "when_equipped",
                    "toggleable",
                ),
            }
        ],
    )
    d["use"] = obj(
        {
            "usable": use("bool"),
            "use_with": use("bool"),
            "interactions": array(use("InteractionRef"), unique=True),
            "ability": use("AbilityRef"),
            "effect": use("EffectRef"),
            "damage": use("damageRange"),
            "damage_type": use("damageType"),
            "mana_cost": integer(),
            "default_action": text(pattern=r"^[a-z][a-z0-9_.-]*$"),
        },
        ("usable", "use_with"),
    )
    d["transform"] = obj(
        {
            "trigger": enum(
                "use", "equip", "unequip", "decay", "destroy", "wrap", "unwrap"
            ),
            "target": use("ItemRef"),
        },
        ("trigger", "target"),
    )
    d["wrapping"] = obj(
        {
            "wrap_target": use("ItemRef"),
            "unwrap_target": use("ItemRef"),
            "preserve_contents": use("bool"),
            "is_wrap_kit": use("bool"),
            "wrap_enabled": use("bool"),
            "unwrap_enabled": use("bool"),
        },
        anyOf=[
            {"required": ["wrap_target"]},
            {"required": ["unwrap_target"]},
            {"required": ["is_wrap_kit"]},
            {"required": ["wrap_enabled"]},
            {"required": ["unwrap_enabled"]},
        ],
    )
    d["lifecycle"] = obj(
        {
            "enchantable": use("bool"),
            "enchanted_variant": use("ItemRef"),
            "destructible": use("bool"),
            "transforms": array(use("transform")),
            "wrapping": use("wrapping"),
        }
    )
    d["trade"] = obj(
        {
            "tradeable": use("bool"),
            "marketable": use("bool"),
            "market_category": text(pattern=r"^[a-z][a-z0-9_.-]*$"),
            "vocations": array(text(pattern=r"^[a-z][a-z0-9_]*$"), unique=True),
        },
        anyOf=[
            {"required": ["tradeable"]},
            {"required": ["marketable"]},
            {"required": ["market_category"]},
            {"required": ["vocations"]},
        ],
        allOf=[
            {
                "if": {
                    "anyOf": [
                        {"required": ["market_category"]},
                        {"required": ["vocations"]},
                    ]
                },
                "then": {
                    "properties": {"marketable": {"const": True}},
                    "required": ["marketable"],
                },
            }
        ],
    )
    d["sourceObservations"] = obj(
        {
            "attributes_text": text(),
            "unparsed_clauses": array(text(), unique=True),
            "unparsed_modifier_clauses": array(text(), unique=True),
        },
        description="Preserved non-executable source text; never interpreted as runtime logic.",
    )
    d["editor"] = obj(
        {
            "notes": array(text(), unique=True),
            "estimated_value": obj(
                {
                    "amount": text(pattern=r"^(?:0|[1-9][0-9]*)(?:\.[0-9]+)?$"),
                    "unit": text(),
                },
                ("amount", "unit"),
            ),
        },
        description="Editor metadata only; not economy or durable value truth.",
    )
    d["wikiEvidenceSource"] = obj(
        {
            "source_id": {"const": "fandom"},
            "page_id": integer(1),
            "title": text(),
            "url": text(pattern=r"^https?://"),
            "revision": text(pattern=r"^[1-9][0-9]*$"),
            "revision_timestamp": text(format="date-time"),
            "captured_at": text(format="date-time"),
            "revision_sha1": text(pattern=r"^[0-9a-f]{40}$"),
            "content_sha256": text(pattern=r"^[0-9a-f]{64}$"),
        },
        (
            "source_id",
            "page_id",
            "title",
            "url",
            "revision",
            "revision_timestamp",
            "captured_at",
            "revision_sha1",
            "content_sha256",
        ),
        description=(
            "One captured English TibiaWiki (tibia.fandom.com) page revision cited as "
            "family_profile evidence; see items-family-fallback.json."
        ),
    )
    d["wikiEvidenceCandidate"] = obj(
        {
            "field": enum("primarytype", "objectclass"),
            "value": text(),
            "wiki_source": use("wikiEvidenceSource"),
        },
        ("field", "value", "wiki_source"),
    )
    d["familyProfileEvidence"] = obj(
        {
            "resolution": enum("direct", "disambiguation"),
            "field": enum("primarytype", "objectclass"),
            "value": text(),
            "wiki_source": use("wikiEvidenceSource"),
            "candidates": array(use("wikiEvidenceCandidate"), 2, unique=True),
        },
        ("resolution",),
        description=(
            "Present only when family_profile_basis is wiki_evidence_fallback: the "
            "admitted-mapping fact that resolved family_profile when no engine "
            "attribute did. 'direct' cites the one matched page's field/value; "
            "'disambiguation' cites 2+ candidate pages that all resolved to the same "
            "profile (the family invariant across candidates)."
        ),
        **{
            "if": {"properties": {"resolution": {"const": "direct"}}},
            "then": {"required": ["field", "value", "wiki_source"]},
            "else": {"required": ["candidates"]},
        },
    )
    properties = {
        "identity": use("identity"),
        "display_name": text(),
        "aliases": array(text(), unique=True),
        "family_profile": enum(*PROFILES),
        "family_profile_basis": enum("wiki_evidence_fallback"),
        "family_profile_evidence": use("familyProfileEvidence"),
        "delivery_task_eligible": use("bool"),
        "presentation": use("presentation"),
        "taxonomy": use("taxonomy"),
        "physical": use("physical"),
        "stack": use("stack"),
        "requirements": use("requirements"),
        "equipment": use("equipment"),
        "weapon": use("weapon"),
        "protection": use("protection"),
        "modifiers": use("modifiers"),
        "charges": use("charges"),
        "temporal": use("temporal"),
        "container": use("container"),
        "imbuement": use("imbuement"),
        "forge": use("forge"),
        "proficiency": use("proficiency"),
        "consumable": use("consumable"),
        "fluid": use("fluid"),
        "readable": use("readable"),
        "light": use("light"),
        "use": use("use"),
        "lifecycle": use("lifecycle"),
        "trade": use("trade"),
        "source_observations": use("sourceObservations"),
        "editor": use("editor"),
    }
    return {
        "$schema": DIALECT,
        "$id": ITEM_ID,
        "title": "Oteryn Item authoring candidate v4",
        "description": "Portable Item definition only. Terrain, placed WorldObject and mutable ItemInstance state are outside this schema.",
        **obj(
            properties,
            (
                "identity",
                "display_name",
                "family_profile",
                "delivery_task_eligible",
                "taxonomy",
            ),
            dependentRequired={
                "family_profile_basis": ["family_profile_evidence"],
                "family_profile_evidence": ["family_profile_basis"],
            },
        ),
        "$defs": d,
    }


def build_dependencies_schema(item_schema):
    d = copy.deepcopy(item_schema["$defs"])
    d["appearanceSource"] = obj(
        {
            "source_profile": enum(CANARY_PROFILE, CRYSTAL_PROFILE),
            "repository": text(),
            "revision": text(),
            "path": text(),
            "digest_sha256": text(pattern=r"^[0-9a-f]{64}$"),
        },
        ("source_profile", "repository", "revision", "path", "digest_sha256"),
    )
    d["appearanceGeometry"] = obj(
        {
            "pattern_width": integer(1),
            "pattern_height": integer(1),
            "pattern_depth": integer(1),
            "layers": integer(1),
            "phase_count": integer(1),
            "is_opaque": use("bool"),
            "bounding_square": integer(),
        },
        (
            "pattern_width",
            "pattern_height",
            "pattern_depth",
            "layers",
            "phase_count",
            "is_opaque",
        ),
    )
    d["appearanceFrameGroup"] = obj(
        {
            "kind": {"const": "object_initial"},
            "source_group_id": {"const": 2},
            "geometry": use("appearanceGeometry"),
            "sprite_ids": array(integer(1), 1),
        },
        ("kind", "source_group_id", "geometry", "sprite_ids"),
    )
    d["presentationAsset"] = obj(
        {
            "identity": use("PresentationRef"),
            "source": use("appearanceSource"),
            "appearance_id": integer(1),
            "frame_groups": array(use("appearanceFrameGroup"), 1),
            "sprite_atlas": obj(
                {
                    "key": use("key"),
                    "revision": use("revision"),
                    "sha256": text(pattern=r"^[0-9a-f]{64}$"),
                },
                ("key", "revision", "sha256"),
                description="Exact raster atlas artifact. Omit until the matching client asset pack is admitted.",
            ),
        },
        ("identity", "source", "appearance_id", "frame_groups"),
    )
    proficiency_crosswalk_fields = (
        "source_profile",
        "repository",
        "revision",
        "path",
        "digest_sha256",
        "external_id",
        "source_version",
        "target",
    )

    def proficiency_crosswalk_shape(source):
        return obj(
            {
                "source_profile": {"const": source["source_profile"]},
                "repository": {"const": source["repository"]},
                "revision": {"const": source["revision"]},
                "path": {"const": source["path"]},
                "digest_sha256": {"const": source["digest_sha256"]},
                "external_id": text(pattern=r"^[1-9][0-9]*$"),
                "source_version": integer(1),
                "target": use("ProficiencyRef"),
            },
            proficiency_crosswalk_fields,
        )

    d["proficiencySourceCrosswalk"] = {
        "oneOf": [
            proficiency_crosswalk_shape(CANARY_PROFICIENCY_SOURCE),
            proficiency_crosswalk_shape(CRYSTAL_PROFICIENCY_SOURCE),
        ],
        "description": (
            "Pinned Canary or Crystal source-ID to admitted canonical Proficiency "
            "mapping; numeric IDs never become ContentKeys."
        ),
    }
    any_reference = {
        "oneOf": [
            use(family + "Ref")
            for family in (
                "Item",
                "Ability",
                "Effect",
                "Interaction",
                "Document",
                "Proficiency",
                "Presentation",
            )
        ]
    }
    return {
        "$schema": DIALECT,
        "$id": DEPS_ID,
        "title": "Oteryn Item authoring exact dependency catalog candidate v4",
        **obj(
            {
                "definitions": array(any_reference, unique=True),
                "assets": array(use("assetBinding"), unique=True),
                "presentations": array(use("presentationAsset")),
                "proficiency_crosswalks": array(use("proficiencySourceCrosswalk")),
            },
            (
                "definitions",
                "assets",
                "presentations",
                "proficiency_crosswalks",
            ),
        ),
        "$defs": d,
    }


def build_real_source_evidence_schema(item_schema):
    d = copy.deepcopy(item_schema["$defs"])
    scalar = {"type": ["string", "number", "boolean"]}
    d["sourceIdentity"] = obj(
        {
            "identity_namespace": {"const": "client/appearance_id"},
            "external_id": text(pattern=r"^[1-9][0-9]*$"),
            "target": use("PresentationRef"),
        },
        ("identity_namespace", "external_id", "target"),
    )
    d["engineSource"] = obj(
        {
            "source_profile": enum(CANARY_PROFILE, CRYSTAL_PROFILE),
            "repository": text(),
            "revision": text(),
            "path": {"const": "data/items/appearances.dat"},
            "digest_sha256": text(pattern=r"^[0-9a-f]{64}$"),
            "item_id": integer(1),
            "appearance_id": integer(1),
            "sprite_ids": array(integer(1), 1),
        },
        (
            "source_profile",
            "repository",
            "revision",
            "path",
            "digest_sha256",
            "item_id",
            "appearance_id",
            "sprite_ids",
        ),
    )
    d["definitionSource"] = obj(
        {
            "source_profile": enum(CANARY_PROFILE, CRYSTAL_PROFILE),
            "repository": text(),
            "revision": text(),
            "path": {"const": "data/items/items.xml"},
            "digest_sha256": text(pattern=r"^[0-9a-f]{64}$"),
            "source_locator": text(pattern=r"^item\[@id='[1-9][0-9]*'\]$"),
        },
        (
            "source_profile",
            "repository",
            "revision",
            "path",
            "digest_sha256",
            "source_locator",
        ),
    )
    d["wikiSource"] = obj(
        {
            "source_id": enum("br", "fandom"),
            "page_id": integer(1),
            "title": text(),
            "url": text(pattern=r"^https?://"),
            "revision": text(pattern=r"^[1-9][0-9]*$"),
            "revision_timestamp": text(format="date-time"),
            "captured_at": text(format="date-time"),
            "revision_sha1": text(pattern=r"^[0-9a-f]{40}$"),
            "content_sha256": text(pattern=r"^[0-9a-f]{64}$"),
            "raw_fields": array(text(), 1, unique=True),
        },
        (
            "source_id",
            "page_id",
            "title",
            "url",
            "revision",
            "revision_timestamp",
            "captured_at",
            "revision_sha1",
            "content_sha256",
            "raw_fields",
        ),
    )
    d["fieldObservation"] = obj(
        {
            "source": enum(
                "br",
                "fandom",
                "engine_items_xml",
                "engine_appearance",
                "canary_appearance",
                "crystal_appearance",
            ),
            "field": text(),
            "catalog_field": text(),
            "raw_value": scalar,
        },
        ("source", "field", "catalog_field", "raw_value"),
    )
    d["fieldEvidence"] = obj(
        {
            "destination": text(pattern=r"^/item(?:/.*)?$"),
            "route_destination": text(pattern=r"^/item(?:/.*)?$"),
            "normalized_value": {},
            "normalization": text(),
            "observations": array(use("fieldObservation"), 1, unique=True),
        },
        (
            "destination",
            "route_destination",
            "normalized_value",
            "normalization",
            "observations",
        ),
    )
    d["nonSourceDefault"] = obj(
        {"destination": text(pattern=r"^/item(?:/.*)?$"), "state": text()},
        ("destination", "state"),
    )
    d["readiness"] = obj(
        {
            "state": {"const": "AUTHORING_EVIDENCE_COMPLETE_RUNTIME_BLOCKED"},
            "blockers": array(text(), 2, unique=True),
            "non_claims": array(text(), 1, unique=True),
        },
        ("state", "blockers", "non_claims"),
    )
    observation_map = {
        "type": "object",
        "minProperties": 1,
        "additionalProperties": scalar,
    }
    source_observations = obj(
        {
            "br": observation_map,
            "fandom": observation_map,
            "engine_items_xml": observation_map,
            "engine_appearance": observation_map,
            "canary_appearance": observation_map,
            "crystal_appearance": observation_map,
        },
        ("br", "fandom", "engine_items_xml"),
    )
    return {
        "$schema": DIALECT,
        "$id": EVIDENCE_ID,
        "title": "Oteryn real Item source evidence candidate v4",
        **obj(
            {
                "schema": {"const": "OTERYN_ITEM_REAL_SOURCE_EVIDENCE/candidate-3"},
                "item_key": use("key"),
                "source_identity": use("sourceIdentity"),
                "engine_sources": array(
                    use("engineSource"), 2, unique=True, maxItems=2
                ),
                "definition_sources": array(
                    use("definitionSource"), 2, unique=True, maxItems=2
                ),
                "wiki_sources": array(use("wikiSource"), 2, unique=True, maxItems=2),
                "source_observations": source_observations,
                "field_evidence": array(use("fieldEvidence"), 1),
                "non_source_defaults": array(use("nonSourceDefault"), unique=True),
                "readiness": use("readiness"),
            },
            (
                "schema",
                "item_key",
                "source_identity",
                "engine_sources",
                "definition_sources",
                "wiki_sources",
                "source_observations",
                "field_evidence",
                "non_source_defaults",
                "readiness",
            ),
        ),
        "$defs": d,
    }


def build_manifest_schema(item_schema):
    d = copy.deepcopy(item_schema["$defs"])
    d["sourceField"] = obj(
        {"source_locator": text(), "source_field": text()},
        ("source_locator", "source_field"),
    )
    d["source"] = obj(
        {
            "kind": enum("git", "wiki", "web", "local"),
            "source_profile": enum(
                BR_PROFILE,
                FANDOM_PROFILE,
                BR_REAL_ITEM_PROFILE,
                FANDOM_REAL_ITEM_PROFILE,
                CANARY_PROFILE,
                CRYSTAL_PROFILE,
            ),
            "url": text(pattern=r"^https?://"),
            "repository": text(),
            "revision": text(),
            "revision_sha1": text(pattern=r"^[0-9a-f]{40}$"),
            "page_id": integer(1),
            "revision_timestamp": text(format="date-time"),
            "path": text(),
            "captured_at": text(format="date-time"),
            "digest_sha256": text(pattern=r"^[0-9a-f]{64}$"),
            "field_inventory": array(use("sourceField"), 1, unique=True),
        },
        (
            "kind",
            "source_profile",
            "revision",
            "captured_at",
            "digest_sha256",
            "field_inventory",
        ),
    )
    d["entry"] = obj(
        {
            "source_index": integer(),
            "source_locator": text(),
            "source_field": text(),
            "source_value": {"type": ["string", "number", "boolean"]},
            "kind": enum(
                "definition",
                "presentation",
                "relationship",
                "provenance",
                "editor",
                "raw_text",
                "external_domain",
                "template_control",
                "instance_state",
                "world_object",
                "terrain",
                "source_defect",
            ),
            "status": enum(
                "mapped",
                "approved_omission",
                "external_domain",
                "reverse_relation",
                "provenance_only",
                "editor_only",
                "pinned_no_effect",
                "unsupported_source_field",
                "unresolved_semantics",
                "conflict",
            ),
            "destination": text(pattern=r"^/(?:item|dependencies)(?:/.*)?$"),
            "promotions": array(text(pattern=r"^/item(?:/.*)?$"), unique=True),
            "reason": text(),
        },
        ("source_index", "source_locator", "source_field", "kind", "status", "reason"),
    )
    return {
        "$schema": DIALECT,
        "$id": MANIFEST_ID,
        "title": "Oteryn Item import readiness candidate v4",
        **obj(
            {"sources": array(use("source"), 1), "entries": array(use("entry"), 1)},
            ("sources", "entries"),
        ),
        "$defs": d,
    }


def identity(slug):
    return {"key": "oteryn:item.template." + slug, "revision": "definition-r1"}


def base(slug, name, profile, item_class, primary):
    return {
        "identity": identity(slug),
        "display_name": name,
        "family_profile": profile,
        "delivery_task_eligible": False,
        "taxonomy": {"item_class": item_class, "primary": primary, "tags": []},
        "physical": {
            "weight": {"value": "1.00", "unit": "oz"},
            "movable": True,
            "pickupable": True,
        },
    }


def build_templates():
    result = {}
    item = base(
        "generic", "Template Portable Item", "material_valuable", "material", "generic"
    )
    item.update({"trade": {"tradeable": True, "marketable": True}})
    result["generic-portable.json"] = item
    item = base("armor", "Template Armor", "equipment_armor", "equipment", "armor")
    item.update(
        {
            "equipment": {
                "slot": "armor",
                "hands": 0,
                "reserved_slots": [],
                "groups": [],
            },
            "protection": {"armor": 1, "resistances": [], "condition_suppressions": []},
            "trade": {"tradeable": True, "marketable": True},
        }
    )
    result["equipment-armor.json"] = item
    item = base(
        "accessory", "Template Accessory", "equipment_offhand", "equipment", "accessory"
    )
    item.update(
        {
            "equipment": {
                "slot": "ring",
                "hands": 0,
                "reserved_slots": [],
                "groups": [],
            },
            "charges": {"count": 1, "show_count": True},
            "trade": {"tradeable": True, "marketable": True},
        }
    )
    result["equipment-accessory.json"] = item
    item = base("melee", "Template Melee Weapon", "weapon_melee", "weapon", "melee")
    item.update(
        {
            "equipment": {
                "slot": "right_hand",
                "hands": 1,
                "reserved_slots": [],
                "groups": [],
            },
            "weapon": {
                "weapon_type": "sword",
                "attack": 1,
                "defense": 1,
                "consumption_mode": "none",
            },
            "trade": {"tradeable": True, "marketable": True},
        }
    )
    result["weapon-melee.json"] = item
    item = base(
        "distance", "Template Distance Weapon", "weapon_distance", "weapon", "distance"
    )
    item.update(
        {
            "equipment": {
                "slot": "right_hand",
                "hands": 2,
                "reserved_slots": ["left_hand"],
                "groups": [],
            },
            "weapon": {
                "weapon_type": "distance_launcher",
                "ammunition_kind": "arrow",
                "attack_modifier": 1,
                "hit_chance_modifier_percent": {"numerator": 0, "denominator": 1},
                "range_cells": 5,
                "consumption_mode": "consume_ammunition",
            },
            "trade": {"tradeable": True, "marketable": True},
        }
    )
    result["weapon-distance.json"] = item
    item = base(
        "ammunition",
        "Template Ammunition",
        "weapon_distance",
        "ammunition",
        "ammunition",
    )
    item.update(
        {
            "stack": {"stackable": True, "max_count": 100},
            "equipment": {
                "slot": "ammo",
                "hands": 0,
                "reserved_slots": [],
                "groups": [],
            },
            "weapon": {
                "weapon_type": "ammunition",
                "ammunition_kind": "arrow",
                "attack": 1,
                "consumption_mode": "consume_item",
            },
            "trade": {"tradeable": True, "marketable": True},
        }
    )
    result["ammunition.json"] = item
    item = base(
        "magic-weapon", "Template Magic Weapon", "weapon_magic", "weapon", "magic"
    )
    item.update(
        {
            "requirements": {
                "min_level": 1,
                "min_magic_level": 1,
                "vocations": [],
                "enforcement_mode": "on_use",
            },
            "equipment": {
                "slot": "right_hand",
                "hands": 1,
                "reserved_slots": [],
                "groups": [],
            },
            "weapon": {
                "weapon_type": "wand",
                "damage_range": {"minimum": 1, "maximum": 2},
                "damage_type": "energy",
                "consumption_mode": "none",
            },
            "use": {"usable": True, "use_with": False},
            "trade": {"tradeable": True, "marketable": True},
        }
    )
    result["weapon-magic.json"] = item
    item = base("rune", "Template Rune", "rune", "rune", "rune")
    item.update(
        {
            "stack": {"stackable": True, "max_count": 100},
            "requirements": {
                "min_level": 1,
                "min_magic_level": 1,
                "vocations": [],
                "enforcement_mode": "on_use",
            },
            "charges": {"count": 1, "show_count": True},
            "use": {"usable": True, "use_with": True},
            "trade": {"tradeable": True, "marketable": True},
        }
    )
    result["rune.json"] = item
    item = base(
        "container", "Template Container", "container", "container", "container"
    )
    item.update(
        {
            "container": {"capacity": 20, "content_kind": "items", "accepts": []},
            "trade": {"tradeable": True, "marketable": True},
        }
    )
    result["container.json"] = item
    item = base("food", "Template Food", "food", "consumable", "food")
    item.update(
        {
            "stack": {"stackable": True, "max_count": 100},
            "consumable": {
                "edible": True,
                "regeneration_seconds": 60,
                "consume_count": 1,
            },
            "use": {"usable": True, "use_with": False},
            "trade": {"tradeable": True, "marketable": True},
        }
    )
    result["food.json"] = item
    item = base("fluid", "Template Fluid", "fluid", "fluid", "fluid")
    item.update(
        {
            "requirements": {"enforcement_mode": "on_use"},
            "fluid": {"role": "content", "fluid_type": "water"},
            "use": {"usable": True, "use_with": False},
            "trade": {"tradeable": True, "marketable": True},
        }
    )
    result["fluid.json"] = item
    item = base("document", "Template Document", "document", "document", "document")
    item.update(
        {
            "presentation": {"grammar": {"article": "a", "plural": "documents"}},
            "readable": {
                "readable": True,
                "writable": False,
                "write_policy": "none",
                "distance_readable": False,
            },
            "trade": {"tradeable": True, "marketable": False},
        }
    )
    result["document.json"] = item
    item = base(
        "decoration-kit",
        "Template Decoration Kit",
        "decoration",
        "decoration",
        "decoration",
    )
    item.update(
        {
            "presentation": {},
            "use": {"usable": True, "use_with": False},
            "lifecycle": {
                "enchantable": False,
                "destructible": False,
                "transforms": [],
            },
            "trade": {"tradeable": True, "marketable": True},
        }
    )
    result["portable-decoration-kit.json"] = item
    return result


def reference_item_defs(schema, item_schema):
    """Drop copied Item $defs and reference them in item.schema.json instead."""
    local = {
        name: value
        for name, value in schema["$defs"].items()
        if item_schema["$defs"].get(name) != value
    }
    prefix = "#/$defs/"

    def rewrite(node):
        if isinstance(node, list):
            return [rewrite(value) for value in node]
        if not isinstance(node, dict):
            return node
        ref = node.get("$ref")
        if ref and ref.startswith(prefix) and ref[len(prefix) :] not in local:
            return {**node, "$ref": ITEM_ID + ref}
        return {key: rewrite(value) for key, value in node.items()}

    result = rewrite({key: value for key, value in schema.items() if key != "$defs"})
    result["$defs"] = rewrite(local)
    return result


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def main():
    item = build_item_schema()
    dependencies = reference_item_defs(build_dependencies_schema(item), item)
    evidence = reference_item_defs(build_real_source_evidence_schema(item), item)
    manifest = reference_item_defs(build_manifest_schema(item), item)
    write_json(ROOT / "item.schema.json", item)
    write_json(ROOT / "item-dependencies.schema.json", dependencies)
    write_json(ROOT / "real-source-evidence.schema.json", evidence)
    write_json(ROOT / "item-import-readiness.schema.json", manifest)
    write_json(
        ROOT / "profile-catalog.json",
        {
            "schema": "OTERYN_ITEM_AUTHORING_PROFILE_CATALOG/candidate-3",
            "profiles": [
                {
                    "profile_id": profile,
                    "expected_capabilities": capabilities,
                    "common_capabilities": COMMON_CAPABILITIES[profile],
                    "optional_capabilities": [
                        capability
                        for capability in capabilities
                        if capability not in COMMON_CAPABILITIES[profile]
                    ],
                    "navigation_families": FAMILY_ASSIGNMENTS[profile],
                    "additional_capabilities_allowed": True,
                }
                for profile, capabilities in PROFILES.items()
            ],
        },
    )
    for name, value in build_templates().items():
        write_json(TEMPLATES / name, value)
    census = json.loads(WIKI_CENSUS.read_text(encoding="utf-8"))
    mapped_fields = {
        row["source_parameter"]
        for row in census["field_mappings"]
        if row["disposition"]
        in (
            "ITEM_TYPED",
            "ITEM_AUTHORING",
            "PRESENTATION_EDITOR",
            "SOURCE_TEXT_PRESERVE_AND_PARSE",
        )
    }
    if mapped_fields != set(WIKI_FORMAL_DESTINATIONS):
        raise ValueError(
            "Wiki formal destination registry does not exactly cover mapped census fields"
        )
    write_json(
        ROOT / "wiki-field-dispositions.json",
        {
            "schema": "OTERYN_ITEM_AUTHORING_WIKI_FIELD_DISPOSITIONS/candidate-3",
            "source_profile": BR_PROFILE,
            "authority": "CURRENT_FIELD_CENSUS_REFERENCE_ONLY",
            "source_url": "https://www.tibiawiki.com.br/index.php?stableid=424807&title=Predefini%C3%A7%C3%A3o%3AInfobox_Item",
            "revision": "stableid:424807",
            "source": WIKI_CENSUS.relative_to(REPOSITORY_ROOT).as_posix(),
            "attrib_promotion_prefixes": list(ATTRIB_PROMOTION_PREFIXES),
            "fields": [
                {
                    "source_field": row["source_parameter"],
                    "disposition": row["disposition"],
                    "canonical_path": row.get("canonical_path"),
                    "allowed_destinations": WIKI_FORMAL_DESTINATIONS.get(
                        row["source_parameter"], []
                    ),
                    **(
                        {"source_value_router": "signed_weight"}
                        if row["source_parameter"] == "weight"
                        else {}
                    ),
                }
                for row in census["field_mappings"]
            ],
        },
    )
    write_json(
        ROOT / "canary-field-dispositions.json", build_engine_catalog(CANARY_PROFILE)
    )
    write_json(
        ROOT / "crystal-field-dispositions.json", build_engine_catalog(CRYSTAL_PROFILE)
    )
    write_json(ROOT / "fandom-field-dispositions.json", build_fandom_catalog())
    write_json(
        ROOT / "wiki-real-item-field-supplement.json",
        build_br_real_item_supplement(),
    )
    write_json(
        ROOT / "fandom-real-item-field-supplement.json",
        build_fandom_real_item_supplement(),
    )
    write_json(
        ROOT / "item-dependencies-template.json",
        {
            "definitions": [],
            "assets": [],
            "presentations": [],
            "proficiency_crosswalks": [],
        },
    )
    write_json(ROOT / "real-source-examples.json", build_real_item_examples())


if __name__ == "__main__":
    main()
