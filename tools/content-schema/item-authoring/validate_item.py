"""Validate one Item authoring definition and its exact dependency catalog."""

import argparse
import hashlib
import json
import re
from datetime import datetime
from decimal import Decimal, InvalidOperation
from fractions import Fraction
from math import gcd
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker
from proficiency_profiles import (
    MAGIC_SWORD_PROFICIENCY_PAYLOAD,
    MAGIC_SWORD_PROFICIENCY_REF,
    MAGIC_SWORD_PROFICIENCY_SOURCE_IDENTITIES,
)
from referencing import Registry, Resource
from source_field_catalogs import TIBIAWIKI_ITEM_BINDINGS

ROOT = Path(__file__).resolve().parent
if "date-time" not in FormatChecker().checkers:
    raise SystemExit(
        "date-time format checking needs rfc3339-validator; pip install -r requirements.txt"
    )
SCHEMA_NAMES = (
    "item.schema.json",
    "item-dependencies.schema.json",
    "item-import-readiness.schema.json",
    "real-source-evidence.schema.json",
)
SCHEMAS = {
    name: json.loads((ROOT / name).read_text(encoding="utf-8"), parse_float=Decimal)
    for name in SCHEMA_NAMES
}
REGISTRY = Registry().with_resources(
    (schema["$id"], Resource.from_contents(schema)) for schema in SCHEMAS.values()
)
PROFILE_CATALOG = json.loads(
    (ROOT / "profile-catalog.json").read_text(encoding="utf-8")
)
EXPECTED_CAPABILITIES = {
    row["profile_id"]: tuple(row["common_capabilities"])
    for row in PROFILE_CATALOG["profiles"]
}
WIKI_CATALOG = json.loads(
    (ROOT / "wiki-field-dispositions.json").read_text(encoding="utf-8")
)
WIKI_FIELD_DISPOSITIONS = {row["source_field"]: row for row in WIKI_CATALOG["fields"]}
ATTRIB_PROMOTION_PREFIXES = tuple(WIKI_CATALOG["attrib_promotion_prefixes"])
SOURCE_CATALOG_FILES = (
    "canary-field-dispositions.json",
    "crystal-field-dispositions.json",
    "fandom-field-dispositions.json",
    "wiki-real-item-field-supplement.json",
    "fandom-real-item-field-supplement.json",
)
SOURCE_CATALOGS = {
    catalog["source_profile"]: catalog
    for catalog in (
        json.loads((ROOT / name).read_text(encoding="utf-8"))
        for name in SOURCE_CATALOG_FILES
    )
}
SOURCE_CATALOGS[WIKI_CATALOG["source_profile"]] = WIKI_CATALOG
PERCENT_WITHIN_100 = {
    "hit_chance_percent",
    "hit_chance_modifier_percent",
    "max_hit_chance_percent",
    "break_chance_percent",
    "falloff_percent",
    "reduction_percent",
    "critical_chance_percent",
    "chance_percent",
    "amount_percent",
    "cleave_percent",
    "percent",
}
ASSET_FIELDS = {
    "projectile_effect",
    "attack_effect",
    "color_binding",
}
ASSET_ARRAYS = {"sounds", "effects"}
APPEARANCE_SOURCE_PINS = {
    "canary_47dfd51_item_definition_v1": {
        "repository": "opentibiabr/canary",
        "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
        "path": "data/items/appearances.dat",
        "digest_sha256": "aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50",
    },
    "crystal_ff7ede5_item_definition_v1": {
        "repository": "zimbadev/crystalserver",
        "revision": "ff7ede593c69d4c658b382c97443e8155926924a",
        "path": "data/items/appearances.dat",
        "digest_sha256": "6adb790d1064c2d31ffb2e5ce1a7aef376942ba672edea2adb6cafc620dd18f1",
    },
}
CANARY_PROFILE = "canary_47dfd51_item_definition_v1"
CRYSTAL_PROFILE = "crystal_ff7ede5_item_definition_v1"
BR_REAL_ITEM_PROFILE = "tibiawiki_br_real_item_pages_20260927_v1"
FANDOM_REAL_ITEM_PROFILE = "tibia_fandom_real_item_pages_20260927_v1"
DEFINITION_SOURCE_PINS = {
    CANARY_PROFILE: {
        "repository": "opentibiabr/canary",
        "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
        "path": "data/items/items.xml",
        "digest_sha256": "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2",
    },
    CRYSTAL_PROFILE: {
        "repository": "zimbadev/crystalserver",
        "revision": "ff7ede593c69d4c658b382c97443e8155926924a",
        "path": "data/items/items.xml",
        "digest_sha256": "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb",
    },
}
REAL_ITEM_SOURCE_OBSERVATION_DIGESTS = {
    "oteryn:item.registry.i00003167": "4424a710831a58d59637a76a85c2117cc0401c8b0312f4f10344eaa2da2dd5da",
    "oteryn:item.equipment.armor.demon": "6b62dc0882b526d9b53313c0c50797bf65f60907c6c3ed57ec8e38a9d788fdf2",
    "oteryn:item.container.backpack": "dc7be4977a7a67686c6ed806e2989c38b86326304232d7023e95b70703bd22a3",
    "oteryn:item.food.red-apple": "ea390265b539184ec45ad15921b0fdd9e35cfa68b9d4aca1cf377a6f294c8a08",
    "oteryn:item.rune.sudden-death": "d791f47099f8c8c585f56e2497f1816d45d4155f3c29f8f319c1df435e682aa4",
    "oteryn:item.fluid-container.vial": "aaa6065a2b35b24d2121cae84a522ff6ec245c73c78f488c41f271da7854b850",
}
MAGIC_SWORD_PROFICIENCY_IDENT = (
    MAGIC_SWORD_PROFICIENCY_REF["family"],
    MAGIC_SWORD_PROFICIENCY_REF["key"],
    MAGIC_SWORD_PROFICIENCY_REF["revision"],
)
ADMITTED_PROFICIENCY_PROFILES = {
    MAGIC_SWORD_PROFICIENCY_IDENT: {
        "sources": MAGIC_SWORD_PROFICIENCY_SOURCE_IDENTITIES,
        "payload": MAGIC_SWORD_PROFICIENCY_PAYLOAD,
    }
}
ADMITTED_PROFICIENCY_CROSSWALKS = {
    source: MAGIC_SWORD_PROFICIENCY_IDENT
    for source in MAGIC_SWORD_PROFICIENCY_SOURCE_IDENTITIES
}
ENGINE_ORIGIN_PATHS = {
    "xml_item_root": "data/items/items.xml",
    "xml_item_attribute": "data/items/items.xml",
    "nested_script_attribute": "data/items/items.xml",
    "appearance": "data/items/appearances.dat",
    "reverse_bag_relation": "data/items/bags.xml",
}
EVIDENCE_SOURCE_FIELD_ALIASES = {
    ("engine_items_xml", "weaponType"): "weapontype",
    ("engine_items_xml", "allowpickUpAble"): "allowpickupable",
}
EVIDENCE_ENUM_VALUE_NORMALIZATIONS = {
    ("/item/equipment/patterns/0/hands", "hands", "Uma"): 1,
    ("/item/equipment/patterns/0/hands", "hands", "One"): 1,
    ("/item/equipment/patterns/1/hands", "hands", "Uma"): 1,
    ("/item/equipment/patterns/1/hands", "hands", "One"): 1,
    ("/item/equipment/slot", "slot", "Body"): "armor",
    ("/item/fluid/role", "holdsliquid", "yes"): "container",
    ("/item/taxonomy/primary", "primarytype", "Sword Weapons"): "sword",
    ("/item/taxonomy/primary", "primarytype", "sword weapons"): "sword",
    ("/item/taxonomy/primary", "primarytype", "Armors"): "armor",
    ("/item/taxonomy/primary", "primarytype", "armors"): "armor",
    ("/item/taxonomy/primary", "primarytype", "Recipientes"): "container",
    ("/item/taxonomy/primary", "primarytype", "Containers"): "container",
    ("/item/taxonomy/primary", "primarytype", "containers"): "container",
    ("/item/taxonomy/primary", "primarytype", "Food"): "food",
    ("/item/taxonomy/primary", "primarytype", "food"): "food",
    ("/item/taxonomy/primary", "primarytype", "Attack Runes"): "attack",
    ("/item/taxonomy/primary", "primarytype", "attack runes"): "attack",
    ("/item/taxonomy/secondary", "secondarytype", "Backpacks"): "backpack",
    ("/item/trade/market_category", "market.category", 20): "swords",
    ("/item/weapon/weapon_type", "type", "Espada"): "sword",
}
NON_SOURCE_DEFAULT_DESTINATIONS = {
    "AUTHOR_SELECTED_FROM_TYPED_ITEM_CAPABILITIES": ("/item/family_profile",),
    "AUTHOR_SELECTED_DELIVERY_TASK_ELIGIBLE": ("/item/delivery_task_eligible",),
    "AUTHOR_SELECTED_DELIVERY_TASK_INELIGIBLE": ("/item/delivery_task_eligible",),
    "SOURCE_WEIGHT_UNIT_NORMALIZATION": ("/item/physical/weight/unit",),
    "PROFILE_ENFORCEMENT_NORMALIZATION": ("/item/requirements/enforcement_mode",),
    "AUTHORING_PATTERN_ID": ("/item/equipment/patterns/*/pattern_id",),
    "ENGINE_NORMALIZATION_FROM_AMBIGUOUS_SLOT_HAND": (
        "/item/equipment/patterns/*/slot",
    ),
    "ENGINE_NORMALIZATION_FROM_SLOT_HAND": ("/item/equipment/patterns/*/slot",),
    "PROFILE_DEFAULT_NO_CONSUMPTION": ("/item/weapon/consumption_mode",),
    "SCHEMA_NORMALIZATION_FROM_STACKABLE_FALSE": ("/item/stack/max_count",),
    "SCHEMA_NORMALIZATION_FOR_NON_HAND_SLOT": ("/item/equipment/hands",),
    "AUTHOR_SELECTED_FROM_CONTAINER_CAPABILITY": ("/item/taxonomy/item_class",),
    "NORMALIZATION_FROM_CONTAINER_TAXONOMY": ("/item/container/content_kind",),
    "AUTHOR_SELECTED_FROM_FOOD_CAPABILITY": ("/item/taxonomy/item_class",),
    "PROFILE_DEFAULT_NOT_SOURCE_VERIFIED": (
        "/item/stack/max_count",
        "/item/consumable/consume_count",
    ),
    "NORMALIZATION_FROM_SINGLE_TARGET_CONSUMPTION": ("/item/use/use_with",),
    "AUTHOR_SELECTED_FROM_FLUID_CAPABILITY": ("/item/taxonomy/item_class",),
    "AUTHOR_SELECTED_CANONICAL_KIND": ("/item/taxonomy/primary",),
}


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON object key " + repr(key))
        result[key] = value
    return result


def read(path):
    return json.loads(
        Path(path).read_text(encoding="utf-8"),
        parse_float=Decimal,
        object_pairs_hook=unique_object,
    )


def exact_numbers(value):
    if isinstance(value, float):
        return Decimal(str(value))
    if isinstance(value, dict):
        return {key: exact_numbers(child) for key, child in value.items()}
    if isinstance(value, list):
        return [exact_numbers(child) for child in value]
    return value


def pointer(path):
    return "/" + "/".join(
        str(key).replace("~", "~0").replace("/", "~1") for key in path
    )


def walk(value, path=()):
    yield path, value
    if isinstance(value, dict):
        for key, child in value.items():
            yield from walk(child, path + (key,))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from walk(child, path + (index,))


def structural(name, data):
    validator = Draft202012Validator(
        SCHEMAS[name], registry=REGISTRY, format_checker=FormatChecker()
    )
    return [
        f"{name}{pointer(error.absolute_path)}: {error.message}"
        for error in sorted(
            validator.iter_errors(exact_numbers(data)),
            key=lambda error: str(list(error.absolute_path)),
        )
    ]


def ident(value):
    return value["family"], value["key"], value["revision"]


def item_ident(value):
    return "Item", value["key"], value["revision"]


def fraction(value):
    return Fraction(value["numerator"], value["denominator"])


def same_value(left, right):
    """JSON equality: booleans never equal numbers (Python treats True == 1)."""
    if isinstance(left, bool) or isinstance(right, bool):
        return type(left) is type(right) and left == right
    return left == right


def unique(values, fields, label, errors):
    keys = [tuple(value[field] for field in fields) for value in values]
    if len(keys) != len(set(keys)):
        errors.append(label + ": duplicate " + "/".join(fields))


def catalog_rules(catalog):
    rules = {}
    base_profile = catalog.get("base_profile")
    if base_profile:
        rules.update(catalog_rules(SOURCE_CATALOGS[base_profile]))
    rules.update({row["source_field"]: row for row in catalog["fields"]})
    return rules


def uses_wiki_base_rule(source_profile, source_field):
    catalog = SOURCE_CATALOGS.get(source_profile)
    return (
        catalog is not None
        and (
            source_profile == WIKI_CATALOG["source_profile"]
            or catalog.get("base_profile") == WIKI_CATALOG["source_profile"]
        )
        and source_field in WIKI_FIELD_DISPOSITIONS
    )


def structural_errors(item, dependencies):
    return structural("item.schema.json", item) + structural(
        "item-dependencies.schema.json", dependencies
    )


def validate(item, dependencies, manifest=None):
    errors = structural_errors(item, dependencies)
    if manifest is not None:
        errors += structural("item-import-readiness.schema.json", manifest)
    if errors:
        return errors, []

    semantic_warnings = []
    local_identity = item_ident(item["identity"])
    definitions = {local_identity}
    declared_definitions = set()
    for reference in dependencies["definitions"]:
        key = ident(reference)
        if key in definitions:
            errors.append(
                "dependencies/definitions: duplicate exact definition " + repr(key)
            )
        definitions.add(key)
        declared_definitions.add(key)
    presentations = {}
    for presentation in dependencies["presentations"]:
        key = ident(presentation["identity"])
        if key in definitions:
            errors.append(
                "dependencies/presentations: duplicate exact definition " + repr(key)
            )
            continue
        definitions.add(key)
        declared_definitions.add(key)
        presentations[key] = presentation
        source = presentation["source"]
        pin = APPEARANCE_SOURCE_PINS[source["source_profile"]]
        if any(source[field] != pin[field] for field in pin):
            errors.append(
                "dependencies/presentations: appearance source differs from its pinned profile"
            )
        frame_group_keys = [
            (group["kind"], group["source_group_id"])
            for group in presentation["frame_groups"]
        ]
        if len(frame_group_keys) != len(set(frame_group_keys)):
            errors.append("dependencies/presentations: duplicate kind/source_group_id")
        for group in presentation["frame_groups"]:
            geometry = group["geometry"]
            expected_sprite_count = (
                geometry["pattern_width"]
                * geometry["pattern_height"]
                * geometry["pattern_depth"]
                * geometry["layers"]
                * geometry["phase_count"]
            )
            if len(group["sprite_ids"]) != expected_sprite_count:
                errors.append(
                    "dependencies/presentations: sprite count differs from geometry/layers/phases"
                )
        if "sprite_atlas" not in presentation:
            semantic_warnings.append(
                "presentation "
                + presentation["identity"]["key"]
                + " has no admitted sprite_atlas; sprite IDs are evidence, not renderable pixels"
            )
    crosswalk_targets = {}
    crosswalk_sources = set()
    for crosswalk in dependencies["proficiency_crosswalks"]:
        source_key = (
            crosswalk["source_profile"],
            crosswalk["external_id"],
            crosswalk["source_version"],
        )
        target_key = ident(crosswalk["target"])
        if ADMITTED_PROFICIENCY_CROSSWALKS.get(source_key) != target_key:
            errors.append(
                "dependencies/proficiency_crosswalks: exact source-to-target mapping is not in the pinned admitted index"
            )
        if source_key in crosswalk_sources:
            errors.append(
                "dependencies/proficiency_crosswalks: duplicate source identity"
            )
        crosswalk_sources.add(source_key)
        crosswalk_targets.setdefault(target_key, set()).add(source_key)
    assets = set(dependencies["assets"])
    used_definitions = set()
    used_assets = set()

    for path, value in walk(item):
        location = "item" + pointer(path)
        if isinstance(value, dict) and set(value) == {"family", "key", "revision"}:
            key = ident(value)
            used_definitions.add(key)
            if key not in definitions:
                errors.append(location + ": unresolved exact definition " + repr(key))
            if key == local_identity:
                errors.append(
                    location
                    + ": direct self-reference is not a valid lifecycle/dependency edge"
                )
        if isinstance(value, dict) and set(value) == {"numerator", "denominator"}:
            if gcd(abs(value["numerator"]), value["denominator"]) != 1:
                errors.append(
                    location + ": ratio must be in lowest terms (zero is 0/1)"
                )
            parent = str(path[-1]) if path else ""
            if parent in PERCENT_WITHIN_100 and abs(fraction(value)) > 100:
                errors.append(location + ": percentage must be within -100..100")
        if (
            path
            and path[-1] in ASSET_FIELDS
            and isinstance(value, str)
            and value not in assets
        ):
            errors.append(location + ": unresolved asset " + value)
        if path and path[-1] in ASSET_FIELDS and isinstance(value, str):
            used_assets.add(value)
        if (
            len(path) >= 2
            and path[-2] in ASSET_ARRAYS
            and isinstance(value, str)
            and value not in assets
        ):
            errors.append(location + ": unresolved asset " + value)
        if len(path) >= 2 and path[-2] in ASSET_ARRAYS and isinstance(value, str):
            used_assets.add(value)
    appearance_ref = item.get("presentation", {}).get("appearance_binding")
    if appearance_ref is not None and ident(appearance_ref) not in presentations:
        errors.append(
            "item/presentation/appearance_binding: missing presentation payload"
        )
    proficiency_ref = item.get("proficiency", {}).get("profile_binding")
    if proficiency_ref is not None:
        proficiency_key = ident(proficiency_ref)
        admitted_profile = ADMITTED_PROFICIENCY_PROFILES.get(proficiency_key)
        if proficiency_key not in crosswalk_targets:
            errors.append(
                "item/proficiency/profile_binding: missing admitted source identity crosswalk"
            )
        elif admitted_profile is None:
            errors.append(
                "item/proficiency/profile_binding: target has no admitted canonical profile"
            )
        else:
            actual_sources = crosswalk_targets[proficiency_key]
            if actual_sources != admitted_profile["sources"]:
                errors.append(
                    "item/proficiency/profile_binding: exact Canary and Crystal source corroboration is required"
                )
            actual_payload = {
                key: value
                for key, value in item["proficiency"].items()
                if key != "profile_binding"
            }
            if actual_payload != admitted_profile["payload"]:
                errors.append(
                    "item/proficiency: inline profile differs from its admitted canonical payload"
                )
    unused_crosswalks = set(crosswalk_targets) - used_definitions
    if unused_crosswalks:
        errors.append(
            "dependencies/proficiency_crosswalks: unused crosswalks "
            + repr(sorted(unused_crosswalks))
        )
    unused_definitions = declared_definitions - used_definitions
    if unused_definitions:
        errors.append(
            "dependencies/definitions: unused exact definitions "
            + repr(sorted(unused_definitions))
        )
    unused_assets = assets - used_assets
    if unused_assets:
        errors.append(
            "dependencies/assets: unused asset bindings " + repr(sorted(unused_assets))
        )

    weapon = item.get("weapon", {})
    damage = weapon.get("damage_range")
    if damage and damage["minimum"] > damage["maximum"]:
        errors.append("item/weapon/damage_range: minimum exceeds maximum")
    use = item.get("use", {})
    use_damage = use.get("damage")
    if use_damage and use_damage["minimum"] > use_damage["maximum"]:
        errors.append("item/use/damage: minimum exceeds maximum")
    if use.get("usable") is False and set(use) - {"usable", "use_with"}:
        errors.append(
            "item/use: unusable item must not declare executable use bindings"
        )
    if use.get("use_with") and not use.get("usable"):
        errors.append("item/use: use_with requires usable=true")

    fluid = item.get("fluid")
    if fluid and fluid["role"] == "content" and "fluid_type" not in fluid:
        errors.append("item/fluid: content role requires fluid_type")
    equipment = item.get("equipment")
    equipment_patterns = []
    if equipment:
        equipment_patterns = equipment.get("patterns", [equipment])
    for pattern in equipment_patterns:
        hand_slots = {"right_hand", "left_hand"}
        if pattern["hands"] > 0 and pattern["slot"] not in hand_slots:
            errors.append("item/equipment: hands > 0 requires a hand slot")
        reserved = pattern.get("reserved_slots", [])
        if pattern["slot"] in reserved:
            errors.append("item/equipment: an item must not reserve its own slot")
        if pattern["slot"] in hand_slots:
            other_hand = (
                "left_hand" if pattern["slot"] == "right_hand" else "right_hand"
            )
            if pattern["hands"] == 2 and other_hand not in reserved:
                errors.append(
                    "item/equipment: two-handed item must reserve the other hand"
                )
            if pattern["hands"] < 2 and other_hand in reserved:
                errors.append(
                    "item/equipment: only a two-handed item may reserve the other hand"
                )
    pattern_ids = [
        pattern.get("pattern_id")
        for pattern in equipment_patterns
        if "pattern_id" in pattern
    ]
    if len(pattern_ids) != len(set(pattern_ids)):
        errors.append("item/equipment/patterns: duplicate pattern_id")

    protection = item.get("protection", {})
    unique(
        protection.get("resistances", []),
        ("damage_type", "scope"),
        "item/protection/resistances",
        errors,
    )
    unique(
        protection.get("condition_suppressions", []),
        ("condition", "scope"),
        "item/protection/condition_suppressions",
        errors,
    )
    modifiers = item.get("modifiers", {})
    unique(
        modifiers.get("skill_boost", []),
        ("skill",),
        "item/modifiers/skill_boost",
        errors,
    )
    unique(
        modifiers.get("resource_capacity", []),
        ("resource",),
        "item/modifiers/resource_capacity",
        errors,
    )
    unique(
        modifiers.get("regeneration", []),
        ("resource",),
        "item/modifiers/regeneration",
        errors,
    )
    unique(modifiers.get("leech", []), ("resource",), "item/modifiers/leech", errors)
    unique(
        modifiers.get("magic_level", []),
        ("combat_type",),
        "item/modifiers/magic_level",
        errors,
    )
    unique(
        modifiers.get("reflection", []),
        ("damage_type",),
        "item/modifiers/reflection",
        errors,
    )
    if "elemental_attack" in weapon:
        unique(
            weapon["elemental_attack"],
            ("damage_type",),
            "item/weapon/elemental_attack",
            errors,
        )
    imbuement = item.get("imbuement", {})
    allowed_imbuements = {
        entry["family"] for entry in imbuement.get("allowed_family_tiers", [])
    }
    excluded_imbuements = set(imbuement.get("excluded_families", []))
    if allowed_imbuements & excluded_imbuements:
        errors.append("item/imbuement: family cannot be both allowed and excluded")
    lifecycle = item.get("lifecycle", {})
    unique(
        lifecycle.get("transforms", []),
        ("trigger",),
        "item/lifecycle/transforms",
        errors,
    )
    decay_transforms = [
        entry
        for entry in lifecycle.get("transforms", [])
        if entry["trigger"] == "decay"
    ]
    decay_target = item.get("temporal", {}).get("decay_target")
    if (
        decay_target
        and decay_transforms
        and ident(decay_target) != ident(decay_transforms[0]["target"])
    ):
        errors.append(
            "item/lifecycle: decay transform conflicts with temporal.decay_target"
        )
    proficiency = item.get("proficiency", {})
    unique(proficiency.get("levels", []), ("level",), "item/proficiency/levels", errors)
    unique(
        proficiency.get("augments", []), ("key",), "item/proficiency/augments", errors
    )
    for level in proficiency.get("levels", []):
        unique(level["perks"], ("key",), "item/proficiency/levels/perks", errors)
        selection_slots = [augment.get("selection_slot") for augment in level["perks"]]
        if any(slot is None for slot in selection_slots):
            errors.append(
                "item/proficiency/levels/perks: every selectable perk requires selection_slot"
            )
        elif len(selection_slots) != len(set(selection_slots)):
            errors.append("item/proficiency/levels/perks: duplicate selection_slot")
        elif selection_slots != list(range(1, len(selection_slots) + 1)):
            errors.append(
                "item/proficiency/levels/perks: selection_slot must be contiguous in source order"
            )
        if level["selection_count"] > len(level["perks"]):
            errors.append(
                "item/proficiency/levels: selection_count exceeds available perks"
            )
        for augment in level["perks"]:
            if "value" not in augment:
                errors.append(
                    "item/proficiency/levels/perks: every selectable perk requires a typed value"
                )
            unique(
                augment.get("rank_values", []),
                ("rank",),
                "item/proficiency/levels/perks/rank_values",
                errors,
            )
    for augment in proficiency.get("augments", []):
        unique(
            augment.get("rank_values", []),
            ("rank",),
            "item/proficiency/augments/rank_values",
            errors,
        )

    if manifest is not None:
        document = {"item": item, "dependencies": dependencies}
        for source in manifest["sources"]:
            validate_source_identity(source, errors)
            validate_capture_timestamp(source, errors)
        manifest_source_tuples = {
            (
                source["source_profile"],
                source.get("repository"),
                source["revision"],
                source.get("path"),
                source["digest_sha256"],
            )
            for source in manifest["sources"]
        }
        for presentation in dependencies["presentations"]:
            source = presentation["source"]
            source_tuple = (
                source["source_profile"],
                source["repository"],
                source["revision"],
                source["path"],
                source["digest_sha256"],
            )
            if source_tuple not in manifest_source_tuples:
                errors.append(
                    "dependencies/presentations: source tuple is absent from the manifest"
                )
        expected_fields = {
            (source_index, field["source_locator"], field["source_field"])
            for source_index, source in enumerate(manifest["sources"])
            for field in source["field_inventory"]
        }
        actual_fields = [
            (entry["source_index"], entry["source_locator"], entry["source_field"])
            for entry in manifest["entries"]
        ]
        if len(actual_fields) != len(set(actual_fields)):
            errors.append("manifest: duplicate source-field disposition")
        if set(actual_fields) != expected_fields:
            errors.append(
                "manifest: dispositions must equal the complete source field inventory"
            )
        for entry in manifest["entries"]:
            if entry["source_index"] >= len(manifest["sources"]):
                errors.append("manifest: invalid source_index")
                continue
            status = entry["status"]
            kind = entry["kind"]
            if kind in ("terrain", "world_object", "instance_state") and status not in (
                "external_domain",
                "approved_omission",
            ):
                errors.append("manifest: " + kind + " cannot map into portable Item")
            if kind == "relationship" and status not in (
                "reverse_relation",
                "external_domain",
                "approved_omission",
            ):
                errors.append(
                    "manifest: relationship must remain with its owning relation domain"
                )
            if kind == "provenance" and status != "provenance_only":
                errors.append(
                    "manifest: provenance field requires provenance_only disposition"
                )
            if kind == "editor" and status not in (
                "mapped",
                "editor_only",
                "approved_omission",
            ):
                errors.append(
                    "manifest: editor field requires mapped, editor_only or approved_omission disposition"
                )
            if (
                kind == "editor"
                and status == "mapped"
                and not entry.get("destination", "").startswith("/item/editor/")
            ):
                errors.append("manifest: mapped editor field must target /item/editor")
            if kind == "external_domain" and status != "external_domain":
                errors.append(
                    "manifest: external-domain field requires external_domain disposition"
                )
            if kind == "template_control" and status != "approved_omission":
                errors.append("manifest: template control requires approved omission")
            if kind == "source_defect" and status != "pinned_no_effect":
                errors.append(
                    "manifest: pinned source defect requires pinned_no_effect disposition"
                )
            source = manifest["sources"][entry["source_index"]]
            if uses_wiki_base_rule(source["source_profile"], entry["source_field"]):
                validate_wiki_disposition(entry, errors)
            else:
                validate_catalog_disposition(entry, source, document, errors)
            if status in (
                "unsupported_source_field",
                "unresolved_semantics",
                "conflict",
            ):
                errors.append(
                    "manifest: "
                    + status
                    + " "
                    + entry["source_locator"]
                    + " "
                    + entry["source_field"]
                )
            if status == "mapped":
                if "destination" not in entry:
                    errors.append("manifest: mapped entry requires destination")
                    continue
                try:
                    resolved = resolve_pointer(document, entry["destination"])
                except (KeyError, IndexError, TypeError, ValueError):
                    errors.append(
                        "manifest: missing destination " + entry["destination"]
                    )
                    continue
                if resolved is None:
                    errors.append("manifest: null destination " + entry["destination"])
                validate_routed_destination_value(entry, source, resolved, errors)
                if (
                    uses_wiki_base_rule(source["source_profile"], entry["source_field"])
                    and entry["source_field"] == "modificadores"
                ):
                    typed_modifiers = set(item.get("modifiers", {})) - {"editor_notes"}
                    if not typed_modifiers:
                        errors.append(
                            "manifest: modificadores requires at least one typed modifier; editor_notes alone is insufficient"
                        )
                    if item.get("source_observations", {}).get(
                        "unparsed_modifier_clauses"
                    ):
                        errors.append(
                            "manifest: modificadores has unresolved clauses and is not import-ready"
                        )
            elif "destination" in entry:
                errors.append("manifest: only mapped entries may declare destination")
            promotions = entry.get("promotions", [])
            if promotions and not (kind == "raw_text" and status == "mapped"):
                errors.append(
                    "manifest: typed promotions are allowed only for mapped raw_text"
                )
            for promotion in promotions:
                try:
                    promoted = resolve_pointer(document, promotion)
                except (KeyError, IndexError, TypeError, ValueError):
                    errors.append(
                        "manifest: missing promotion destination " + promotion
                    )
                    continue
                if promoted is None:
                    errors.append("manifest: null promotion destination " + promotion)
                if not promotion.startswith(ATTRIB_PROMOTION_PREFIXES):
                    errors.append(
                        "manifest: raw-text promotion targets an unapproved capability"
                    )

    expected = EXPECTED_CAPABILITIES[item["family_profile"]]
    warnings = semantic_warnings + [
        "profile "
        + item["family_profile"]
        + " normally expects capability "
        + capability
        for capability in expected
        if capability not in item
    ]
    return errors, warnings


def leaf_pointers(value, path=()):
    # An empty object still declares a capability; an empty list asserts nothing.
    if isinstance(value, dict) and not value:
        yield pointer(path)
    elif isinstance(value, dict):
        for key, child in value.items():
            yield from leaf_pointers(child, path + (key,))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from leaf_pointers(child, path + (index,))
    else:
        yield pointer(path)


def evidence_route_value(rule, raw_value):
    route_values = [
        route["source_value"] for route in rule.get("source_value_routes", [])
    ]
    if any(isinstance(value, bool) for value in route_values) and isinstance(
        raw_value, str
    ):
        lowered = raw_value.casefold()
        if lowered in ("yes", "sim", "true", "1"):
            return True
        if lowered in ("no", "não", "false", "0"):
            return False
    return raw_value


def normalize_evidence_boolean(raw_value):
    if isinstance(raw_value, bool):
        return raw_value
    if isinstance(raw_value, int) and raw_value in (0, 1):
        return bool(raw_value)
    if isinstance(raw_value, str):
        lowered = raw_value.casefold()
        if lowered in ("yes", "sim", "true", "1"):
            return True
        if lowered in ("no", "não", "false", "0"):
            return False
    raise ValueError("unsupported evidence boolean")


def normalize_evidence_integer(raw_value):
    if isinstance(raw_value, bool):
        raise TypeError("boolean is not an evidence integer")
    if isinstance(raw_value, int):
        return raw_value
    if isinstance(raw_value, str) and re.fullmatch(r"[+-]?[0-9]+", raw_value):
        return int(raw_value)
    raise ValueError("unsupported evidence integer")


def normalize_evidence_weight(observation):
    raw_value = observation["raw_value"]
    try:
        value = Decimal(str(raw_value))
    except (InvalidOperation, TypeError, ValueError) as error:
        raise ValueError("unsupported evidence weight") from error
    if not value.is_finite():
        raise ValueError("non-finite evidence weight")
    if observation["source"] == "engine_items_xml":
        if value != value.to_integral_value():
            raise ValueError("engine weight must use integral centi-ounces")
        value /= 100
    elif observation["source"] in ("br", "fandom"):
        if value * 100 != (value * 100).to_integral_value():
            raise ValueError("Wiki weight must be exact to centi-ounces")
    else:
        raise ValueError("unsupported evidence weight source")
    return f"{value:.2f}"


def validate_evidence_observation_value(
    observation, destination, resolved, catalog_normalizations, errors
):
    if catalog_normalizations:
        expected = catalog_normalizations[0]
        if any(value != expected for value in catalog_normalizations[1:]):
            errors.append(
                "evidence/field_evidence: source profiles disagree on the pinned value normalization"
            )
            return
    else:
        key = (
            destination,
            observation["catalog_field"],
            observation["raw_value"],
        )
        if key in EVIDENCE_ENUM_VALUE_NORMALIZATIONS:
            expected = EVIDENCE_ENUM_VALUE_NORMALIZATIONS[key]
        elif (
            destination == "/item/physical/weight/value"
            and observation["catalog_field"] == "weight"
        ):
            try:
                expected = normalize_evidence_weight(observation)
            except (TypeError, ValueError):
                errors.append(
                    "evidence/field_evidence: raw weight has no admitted exact normalization"
                )
                return
        elif isinstance(resolved, bool):
            try:
                expected = normalize_evidence_boolean(observation["raw_value"])
            except ValueError:
                errors.append(
                    "evidence/field_evidence: raw boolean has no admitted normalization"
                )
                return
        elif isinstance(resolved, int):
            try:
                expected = normalize_evidence_integer(observation["raw_value"])
            except (TypeError, ValueError):
                errors.append(
                    "evidence/field_evidence: raw integer has no admitted normalization"
                )
                return
        elif isinstance(resolved, str) and observation["raw_value"] == resolved:
            expected = resolved
        else:
            errors.append(
                "evidence/field_evidence: observation has no admitted value normalization"
            )
            return
    if expected != resolved:
        errors.append(
            "evidence/field_evidence: raw value does not normalize to Item destination"
        )


def validate_evidence_observation_route(
    observation, route_destination, destination, resolved, document, errors
):
    if not (
        route_destination == destination
        or destination.startswith(route_destination + "/")
    ):
        errors.append(
            "evidence/field_evidence: route_destination is unrelated to destination"
        )
        return
    source_id = observation["source"]
    expected_catalog_field = EVIDENCE_SOURCE_FIELD_ALIASES.get(
        (source_id, observation["field"]), observation["field"]
    )
    if observation["catalog_field"] != expected_catalog_field:
        errors.append(
            "evidence/field_evidence: catalog_field differs from the admitted raw-field alias"
        )
        return
    profiles = {
        "br": (BR_REAL_ITEM_PROFILE,),
        "fandom": (FANDOM_REAL_ITEM_PROFILE,),
        "engine_items_xml": (CANARY_PROFILE, CRYSTAL_PROFILE),
        "engine_appearance": (CANARY_PROFILE, CRYSTAL_PROFILE),
        "canary_appearance": (CANARY_PROFILE,),
        "crystal_appearance": (CRYSTAL_PROFILE,),
    }[source_id]
    catalog_normalizations = []
    for profile in profiles:
        catalog = SOURCE_CATALOGS[profile]
        rule = catalog_rules(catalog).get(observation["catalog_field"])
        if rule is None:
            errors.append(
                "evidence/field_evidence: observation field is absent from its pinned catalog"
            )
            continue
        effective_rule = rule
        source_value = evidence_route_value(rule, observation["raw_value"])
        entry = {
            "source_field": observation["catalog_field"],
            "source_value": source_value,
            "destination": route_destination,
        }
        source = {"source_profile": profile}
        if source_id == "engine_items_xml":
            source["path"] = "data/items/items.xml"
        elif source_id in ("engine_appearance", "canary_appearance"):
            source["path"] = "data/items/appearances.dat"
        if uses_wiki_base_rule(profile, observation["catalog_field"]):
            if rule["disposition"] not in (
                "ITEM_TYPED",
                "ITEM_AUTHORING",
                "PRESENTATION_EDITOR",
                "SOURCE_TEXT_PRESERVE_AND_PARSE",
            ):
                errors.append(
                    "evidence/field_evidence: non-mappable Wiki field cannot prove an Item destination"
                )
                continue
            entry.update({"kind": "definition", "status": "mapped"})
            validate_wiki_disposition(entry, errors)
        else:
            if "source_value_routes" in rule:
                effective_rule = next(
                    (
                        route
                        for route in rule["source_value_routes"]
                        if route["source_value"] == source_value
                    ),
                    None,
                )
                if effective_rule is None:
                    errors.append(
                        "evidence/field_evidence: raw value has no pinned source-value route"
                    )
                    continue
            if effective_rule["status"] != "mapped":
                errors.append(
                    "evidence/field_evidence: external, unresolved or omitted field cannot prove an Item destination"
                )
                continue
            entry.update(
                {"kind": effective_rule["kind"], "status": effective_rule["status"]}
            )
            validate_catalog_disposition(entry, source, document, errors)
        try:
            routed_value = resolve_pointer(document, route_destination)
        except (KeyError, IndexError, TypeError, ValueError):
            errors.append(
                "evidence/field_evidence: route_destination does not resolve in Item"
            )
            continue
        validate_routed_destination_value(entry, source, routed_value, errors)
        if "destination_value_equals" in effective_rule:
            catalog_normalizations.append(effective_rule["destination_value_equals"])
        elif effective_rule.get("destination_value_transform") == (
            "boolean_not_source"
        ) and isinstance(source_value, bool):
            catalog_normalizations.append(not source_value)
    validate_evidence_observation_value(
        observation, destination, resolved, catalog_normalizations, errors
    )


def validate_non_source_default(entry, resolved, item, errors):
    patterns = NON_SOURCE_DEFAULT_DESTINATIONS.get(entry["state"])
    if patterns is None or not any(
        pointer_matches(entry["destination"], pattern) for pattern in patterns
    ):
        errors.append(
            "evidence/non_source_defaults: state is not admitted for this Item leaf"
        )
        return
    expected_values = {
        "SOURCE_WEIGHT_UNIT_NORMALIZATION": "oz",
        "PROFILE_DEFAULT_NO_CONSUMPTION": "none",
        "SCHEMA_NORMALIZATION_FOR_NON_HAND_SLOT": 0,
        "AUTHOR_SELECTED_FROM_CONTAINER_CAPABILITY": "container",
        "NORMALIZATION_FROM_CONTAINER_TAXONOMY": "items",
        "AUTHOR_SELECTED_FROM_FOOD_CAPABILITY": "consumable",
        "NORMALIZATION_FROM_SINGLE_TARGET_CONSUMPTION": False,
        "AUTHOR_SELECTED_FROM_FLUID_CAPABILITY": "fluid_container",
        "AUTHOR_SELECTED_CANONICAL_KIND": "vial",
    }
    if entry["state"] in expected_values and not same_value(
        resolved, expected_values[entry["state"]]
    ):
        errors.append(
            "evidence/non_source_defaults: value differs from the admitted normalization"
        )
    delivery_task_values = {
        "AUTHOR_SELECTED_DELIVERY_TASK_ELIGIBLE": True,
        "AUTHOR_SELECTED_DELIVERY_TASK_INELIGIBLE": False,
    }
    if (
        entry["state"] in delivery_task_values
        and resolved is not delivery_task_values[entry["state"]]
    ):
        errors.append(
            "evidence/non_source_defaults: delivery task eligibility differs from the author decision"
        )
    if entry["state"] == "PROFILE_ENFORCEMENT_NORMALIZATION" and resolved not in (
        "on_equip",
        "on_use",
    ):
        errors.append(
            "evidence/non_source_defaults: unsupported enforcement normalization"
        )
    if entry["state"] in (
        "ENGINE_NORMALIZATION_FROM_AMBIGUOUS_SLOT_HAND",
        "ENGINE_NORMALIZATION_FROM_SLOT_HAND",
    ) and resolved not in ("right_hand", "left_hand"):
        errors.append(
            "evidence/non_source_defaults: unsupported hand-slot normalization"
        )
    if (
        entry["state"]
        in (
            "SCHEMA_NORMALIZATION_FROM_STACKABLE_FALSE",
            "PROFILE_DEFAULT_NOT_SOURCE_VERIFIED",
        )
        and entry["destination"] == "/item/stack/max_count"
    ):
        expected = 100 if item["stack"]["stackable"] else 1
        if resolved != expected:
            errors.append(
                "evidence/non_source_defaults: max_count differs from stackability normalization"
            )
    if (
        entry["state"] == "PROFILE_DEFAULT_NOT_SOURCE_VERIFIED"
        and entry["destination"] == "/item/consumable/consume_count"
        and resolved != 1
    ):
        errors.append(
            "evidence/non_source_defaults: consume_count differs from the admitted default"
        )


def validate_real_example(example):
    item = example["item"]
    dependencies = example["dependencies"]
    evidence = example["evidence"]
    evidence_errors = structural("real-source-evidence.schema.json", evidence)
    errors, warnings = validate(item, dependencies)
    errors.extend(evidence_errors)
    if evidence_errors or structural_errors(item, dependencies):
        return errors, warnings

    document = {"item": item}
    if evidence["item_key"] != item["identity"]["key"]:
        errors.append("evidence/item_key: differs from Item identity")
    expected_observation_digest = REAL_ITEM_SOURCE_OBSERVATION_DIGESTS.get(
        item["identity"]["key"]
    )
    try:
        observation_payload = json.dumps(
            evidence["source_observations"],
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8")
    except (TypeError, ValueError):
        observation_digest = None
    else:
        observation_digest = hashlib.sha256(observation_payload).hexdigest()
    if (
        expected_observation_digest is None
        or observation_digest != expected_observation_digest
    ):
        errors.append(
            "evidence/source_observations: canonical value matrix differs from the pinned source extraction"
        )

    presentations = dependencies["presentations"]
    if "appearance_binding" not in item.get("presentation", {}):
        errors.append("evidence: real Item example requires an appearance binding")
        return errors, warnings
    if len(presentations) != 1:
        errors.append(
            "evidence: real Item example requires exactly one Presentation payload"
        )
        return errors, warnings
    presentation = presentations[0]
    appearance_ref = item["presentation"]["appearance_binding"]
    source_identity = evidence["source_identity"]
    if source_identity["target"] != appearance_ref:
        errors.append(
            "evidence/source_identity/target: differs from Item appearance binding"
        )
    if presentation["identity"] != appearance_ref:
        errors.append(
            "evidence: Presentation payload identity differs from Item binding"
        )
    if source_identity["external_id"] != str(presentation["appearance_id"]):
        errors.append(
            "evidence/source_identity/external_id: differs from appearance_id"
        )

    sprite_ids = [
        sprite_id
        for group in presentation["frame_groups"]
        for sprite_id in group["sprite_ids"]
    ]
    engine_sources = {
        source["source_profile"]: source for source in evidence["engine_sources"]
    }
    if set(engine_sources) != set(APPEARANCE_SOURCE_PINS):
        errors.append(
            "evidence/engine_sources: must contain exact Canary and Crystal pins"
        )
    for profile, pin in APPEARANCE_SOURCE_PINS.items():
        source = engine_sources.get(profile)
        if source is None:
            continue
        if any(source[field] != pin[field] for field in pin):
            errors.append(
                "evidence/engine_sources: source differs from pinned appearance artifact"
            )
        if source["item_id"] != presentation["appearance_id"]:
            errors.append("evidence/engine_sources: item_id differs from appearance_id")
        if source["appearance_id"] != presentation["appearance_id"]:
            errors.append(
                "evidence/engine_sources: source appearance_id differs from Presentation"
            )
        if source["sprite_ids"] != sprite_ids:
            errors.append(
                "evidence/engine_sources: ordered sprite IDs differ from Presentation"
            )

    definition_sources = {
        source["source_profile"]: source for source in evidence["definition_sources"]
    }
    if set(definition_sources) != set(DEFINITION_SOURCE_PINS):
        errors.append(
            "evidence/definition_sources: must contain exact Canary and Crystal pins"
        )
    expected_locator = f"item[@id='{presentation['appearance_id']}']"
    for profile, pin in DEFINITION_SOURCE_PINS.items():
        source = definition_sources.get(profile)
        if source is None:
            continue
        if any(source[field] != pin[field] for field in pin):
            errors.append(
                "evidence/definition_sources: source differs from pinned definition artifact"
            )
        if source["source_locator"] != expected_locator:
            errors.append(
                "evidence/definition_sources: locator differs from Presentation appearance_id"
            )

    wiki_profiles = {"br": BR_REAL_ITEM_PROFILE, "fandom": FANDOM_REAL_ITEM_PROFILE}
    wiki_sources = {source["source_id"]: source for source in evidence["wiki_sources"]}
    if set(wiki_sources) != set(wiki_profiles):
        errors.append("evidence/wiki_sources: must contain exact BR and Fandom pages")
    for source_id, profile in wiki_profiles.items():
        source = wiki_sources.get(source_id)
        if source is None:
            continue
        page = next(
            (
                candidate
                for candidate in SOURCE_CATALOGS[profile]["pages"]
                if candidate["title"] == item["display_name"]
            ),
            None,
        )
        if page is None or source != {"source_id": source_id, **page}:
            errors.append(
                "evidence/wiki_sources: page identity or complete raw inventory differs from its pinned supplement"
            )

    observations = evidence["source_observations"]
    if "profile_binding" in item.get("proficiency", {}):
        crosswalks_by_profile = {
            crosswalk["source_profile"]: crosswalk
            for crosswalk in dependencies["proficiency_crosswalks"]
        }
        for source_id, source_profile in (
            ("canary_appearance", CANARY_PROFILE),
            ("crystal_appearance", CRYSTAL_PROFILE),
        ):
            observed_id = observations.get(source_id, {}).get(
                "proficiency.proficiency_id"
            )
            crosswalk = crosswalks_by_profile.get(source_profile)
            if crosswalk is None or str(observed_id) != crosswalk["external_id"]:
                errors.append(
                    "evidence/source_observations: proficiency ID differs from its exact source crosswalk"
                )
    for source_id, profile in wiki_profiles.items():
        source = wiki_sources.get(source_id)
        if source is None:
            continue
        unknown_fields = set(observations[source_id]) - set(source["raw_fields"])
        if unknown_fields:
            errors.append(
                "evidence/source_observations: Wiki field is absent from the pinned page inventory"
            )
        rules = catalog_rules(SOURCE_CATALOGS[profile])
        if set(observations[source_id]) - set(rules):
            errors.append(
                "evidence/source_observations: Wiki field is absent from the pinned disposition catalog"
            )
    engine_origins = {
        "engine_items_xml": (
            {"xml_item_root", "xml_item_attribute", "nested_script_attribute"},
            (CANARY_PROFILE, CRYSTAL_PROFILE),
        ),
        "engine_appearance": (
            {"appearance"},
            (CANARY_PROFILE, CRYSTAL_PROFILE),
        ),
        "canary_appearance": ({"appearance"}, (CANARY_PROFILE,)),
        "crystal_appearance": ({"appearance"}, (CRYSTAL_PROFILE,)),
    }
    for source_id, (allowed_origins, profiles) in engine_origins.items():
        for field in observations.get(source_id, {}):
            catalog_field = EVIDENCE_SOURCE_FIELD_ALIASES.get((source_id, field), field)
            for profile in profiles:
                rule = catalog_rules(SOURCE_CATALOGS[profile]).get(catalog_field)
                if rule is None:
                    errors.append(
                        "evidence/source_observations: engine field is absent from the pinned catalog"
                    )
                elif not (set(rule["origins"]) & allowed_origins):
                    errors.append(
                        "evidence/source_observations: engine field is assigned to the wrong artifact"
                    )

    destinations = [entry["destination"] for entry in evidence["field_evidence"]]
    if len(destinations) != len(set(destinations)):
        errors.append("evidence/field_evidence: duplicate destination")
    for entry in evidence["field_evidence"]:
        try:
            resolved = resolve_pointer(document, entry["destination"])
        except (KeyError, IndexError, TypeError, ValueError):
            errors.append(
                "evidence/field_evidence: missing destination " + entry["destination"]
            )
            continue
        if not same_value(resolved, entry["normalized_value"]):
            errors.append(
                "evidence/field_evidence: normalized value differs from Item destination"
            )
        if isinstance(resolved, (dict, list)):
            errors.append(
                "evidence/field_evidence: destination must identify one scalar leaf"
            )
        for observation in entry["observations"]:
            source_values = observations.get(observation["source"], {})
            if observation["field"] not in source_values:
                errors.append(
                    "evidence/field_evidence: observation is absent from source_observations"
                )
            elif source_values[observation["field"]] != observation["raw_value"]:
                errors.append(
                    "evidence/field_evidence: raw value differs from source_observations"
                )
            validate_evidence_observation_route(
                observation,
                entry["route_destination"],
                entry["destination"],
                resolved,
                document,
                errors,
            )

    default_destinations = [
        entry["destination"] for entry in evidence["non_source_defaults"]
    ]
    if len(default_destinations) != len(set(default_destinations)):
        errors.append("evidence/non_source_defaults: duplicate destination")
    if set(destinations) & set(default_destinations):
        errors.append(
            "evidence: a leaf cannot be both source-evidenced and a non-source default"
        )
    for destination in default_destinations:
        try:
            resolved = resolve_pointer(document, destination)
        except (KeyError, IndexError, TypeError, ValueError):
            errors.append(
                "evidence/non_source_defaults: missing destination " + destination
            )
            continue
        if isinstance(resolved, (dict, list)):
            errors.append(
                "evidence/non_source_defaults: destination must identify one scalar leaf"
            )
        validate_non_source_default(
            next(
                entry
                for entry in evidence["non_source_defaults"]
                if entry["destination"] == destination
            ),
            resolved,
            item,
            errors,
        )
    expected_leaves = {
        leaf
        for key, value in item.items()
        if key != "identity"
        and not (
            key == "proficiency"
            and isinstance(value, dict)
            and "profile_binding" in value
        )
        for leaf in leaf_pointers(value, ("item", key))
        if not leaf.startswith("/item/presentation/appearance_binding/")
    }
    actual_leaves = set(destinations) | set(default_destinations)
    if actual_leaves != expected_leaves:
        errors.append(
            "evidence: field_evidence and non_source_defaults must exactly partition every authored Item leaf"
        )

    expected_blockers = {
        "sprite_atlas_not_admitted",
        "runtime_lowering_not_implemented",
    }
    has_proficiency_source_id = any(
        "proficiency.proficiency_id" in observations.get(source_id, {})
        for source_id in ("canary_appearance", "crystal_appearance")
    )
    if has_proficiency_source_id and "profile_binding" not in item.get(
        "proficiency", {}
    ):
        expected_blockers.add("proficiency_id_238_crosswalk_not_admitted")
    if "sounds" in observations.get("br", {}):
        expected_blockers.add("yum_sound_asset_not_admitted")
    if item["family_profile"] == "rune":
        expected_blockers.update(
            {"ability_binding_not_admitted", "combat_formula_not_admitted"}
        )
    if item["family_profile"] == "fluid":
        expected_blockers.add("fluid_interaction_binding_not_admitted")
    br_page = next(
        (source for source in evidence["wiki_sources"] if source["source_id"] == "br"),
        None,
    )
    canonical_key = br_page and TIBIAWIKI_ITEM_BINDINGS.get(str(br_page["page_id"]))
    if canonical_key is None:
        expected_blockers.add("canonical_item_identity_not_bound")
    elif item["identity"]["key"] != canonical_key:
        errors.append(
            "item/identity/key: differs from the canonical Item bound to its TibiaWiki page"
        )
    if set(evidence["readiness"]["blockers"]) != expected_blockers:
        errors.append(
            "evidence/readiness/blockers: differs from unresolved source state"
        )
    expected_non_claims = {
        "not_runtime_imported",
        "not_gameplay_parity_proven",
        "not_renderable_without_matching_sprite_atlas",
    }
    if set(evidence["readiness"]["non_claims"]) != expected_non_claims:
        errors.append(
            "evidence/readiness/non_claims: differs from the authoring boundary"
        )
    return errors, warnings


def validate_wiki_disposition(entry, errors):
    rule = WIKI_FIELD_DISPOSITIONS.get(entry["source_field"])
    if rule is None:
        errors.append("manifest: unknown Wiki Item field " + entry["source_field"])
        return
    disposition = rule["disposition"]
    kind = entry["kind"]
    status = entry["status"]
    allowed_destinations = rule["allowed_destinations"]
    if rule.get("source_value_router") == "signed_weight":
        expected = signed_weight_destination(entry, errors)
        if expected is None:
            return
        allowed_destinations = [expected]
    if status == "mapped" and not any(
        pointer_matches(entry.get("destination", ""), pattern)
        for pattern in allowed_destinations
    ):
        errors.append(
            "manifest: Wiki field destination is not its allowed formal Item path"
        )
    if disposition == "RELATIONSHIP" and (kind, status) != (
        "relationship",
        "reverse_relation",
    ):
        errors.append(
            "manifest: Wiki relationship field requires relationship/reverse_relation"
        )
    elif disposition == "EXTERNAL_DOMAIN" and (kind, status) != (
        "external_domain",
        "external_domain",
    ):
        errors.append(
            "manifest: Wiki external field requires external_domain disposition"
        )
    elif disposition == "PROVENANCE" and (kind, status) != (
        "provenance",
        "provenance_only",
    ):
        errors.append(
            "manifest: Wiki provenance field requires provenance/provenance_only"
        )
    elif disposition == "TEMPLATE_CONTROL" and (kind, status) != (
        "template_control",
        "approved_omission",
    ):
        errors.append("manifest: Wiki template control requires approved omission")
    elif disposition == "SOURCE_TEXT_PRESERVE_AND_PARSE" and (kind, status) != (
        "raw_text",
        "mapped",
    ):
        errors.append("manifest: Wiki source text must be retained as mapped raw_text")
    elif disposition in ("ITEM_TYPED", "ITEM_AUTHORING"):
        if kind not in ("definition", "presentation") or status != "mapped":
            errors.append(
                "manifest: Wiki Item field requires a typed authoring mapping"
            )
    elif disposition == "PRESENTATION_EDITOR":
        expected_kind = (
            "editor" if entry["source_field"] in ("notes", "value") else "presentation"
        )
        if kind != expected_kind or status != "mapped":
            errors.append(
                "manifest: Wiki presentation/editor field has the wrong owner disposition"
            )


def validate_catalog_disposition(entry, source, document, errors):
    catalog = SOURCE_CATALOGS.get(source["source_profile"])
    if catalog is None:
        errors.append("manifest: unknown source_profile " + source["source_profile"])
        return
    rules = catalog_rules(catalog)
    rule = rules.get(entry["source_field"])
    if rule is None:
        errors.append(
            "manifest: unsupported field for source_profile "
            + source["source_profile"]
            + ": "
            + entry["source_field"]
        )
        return
    if "repository" in catalog:
        allowed_paths = {
            ENGINE_ORIGIN_PATHS[origin]
            for origin in rule["origins"]
            if origin in ENGINE_ORIGIN_PATHS
        }
        if source.get("path") not in allowed_paths:
            errors.append(
                "manifest: engine source path differs from the field's pinned origin"
            )
    effective_rule = rule
    if rule.get("source_value_router") == "signed_weight":
        expected = signed_weight_destination(entry, errors)
        if expected is None:
            return
        effective_rule = {**rule, "allowed_destinations": [expected]}
    if (
        rule.get("source_value_router") == "chain_mode"
        and chain_source_mode(entry, errors) is None
    ):
        return
    if "source_value_routes" in rule:
        if "source_value" not in entry:
            errors.append(
                "manifest: value-dependent source field requires source_value"
            )
            return
        matches = [
            route
            for route in rule["source_value_routes"]
            if route["source_value"] == entry["source_value"]
        ]
        if not matches:
            errors.append(
                "manifest: unsupported source_value for value-dependent source field"
            )
            return
        effective_rule = matches[0]
    if entry["kind"] != effective_rule["kind"]:
        errors.append("manifest: source field kind differs from its pinned disposition")
    if entry["status"] != effective_rule["status"]:
        errors.append(
            "manifest: source field status differs from its pinned disposition"
        )
    if entry["status"] == "mapped" and not any(
        pointer_matches(entry.get("destination", ""), pattern)
        for pattern in effective_rule["allowed_destinations"]
    ):
        errors.append(
            "manifest: source field destination is not an allowed formal Item path"
        )


def signed_weight_destination(entry, errors):
    if "source_value" not in entry:
        errors.append("manifest: signed weight routing requires source_value")
        return None
    try:
        value = Decimal(str(entry["source_value"]))
    except (InvalidOperation, TypeError, ValueError):
        errors.append("manifest: signed weight source_value must be an exact decimal")
        return None
    if not value.is_finite():
        errors.append("manifest: signed weight source_value must be finite")
        return None
    return "/item/presentation/display_weight" if value < 0 else "/item/physical/weight"


def chain_source_mode(entry, errors):
    if "source_value" not in entry:
        errors.append("manifest: chain routing requires source_value")
        return None
    raw_value = entry["source_value"]
    if raw_value is False:
        return "disabled"
    if isinstance(raw_value, bool):
        errors.append("manifest: chain source_value=true has no pinned meaning")
        return None
    try:
        value = Decimal(str(raw_value))
    except (InvalidOperation, TypeError, ValueError):
        errors.append("manifest: chain source_value must be false or an exact number")
        return None
    if not value.is_finite() or value < 0:
        errors.append("manifest: chain source_value must be finite and nonnegative")
        return None
    return "disabled" if value == 0 else "override"


def validate_routed_destination_value(entry, source, resolved, errors):
    catalog = SOURCE_CATALOGS.get(source["source_profile"])
    if catalog is None:
        return
    rule = catalog_rules(catalog).get(entry["source_field"])
    if not rule:
        return
    effective_rule = rule
    if "source_value_routes" in rule and "source_value" in entry:
        effective_rule = next(
            (
                route
                for route in rule["source_value_routes"]
                if route["source_value"] == entry["source_value"]
            ),
            rule,
        )
    if rule.get("source_value_router") == "signed_weight":
        try:
            source_value = Decimal(str(entry["source_value"]))
            target_value = Decimal(resolved["value"])
        except (InvalidOperation, KeyError, TypeError, ValueError):
            errors.append(
                "manifest: signed weight destination must contain an exact decimal value"
            )
            return
        if not source_value.is_finite() or not target_value.is_finite():
            errors.append("manifest: signed weight values must be finite")
            return
        if (source_value < 0) != (target_value < 0):
            errors.append(
                "manifest: signed weight source and destination signs disagree"
            )
    if rule.get("source_value_router") == "chain_mode":
        expected_mode = chain_source_mode(entry, errors)
        if expected_mode is None:
            return
        if not isinstance(resolved, dict) or resolved.get("mode") != expected_mode:
            errors.append("manifest: chain destination mode differs from source_value")
            return
        if expected_mode == "override":
            try:
                source_ratio = Fraction(str(entry["source_value"]))
                target_ratio = fraction(resolved["skill_formula_coefficient"])
            except (KeyError, TypeError, ValueError, ZeroDivisionError):
                errors.append("manifest: chain override requires an exact coefficient")
                return
            if source_ratio != target_ratio:
                errors.append("manifest: chain coefficient differs from source_value")
    if effective_rule.get("destination_value_transform") == "boolean_not_source" and (
        not isinstance(entry.get("source_value"), bool)
        or resolved is not (not entry["source_value"])
    ):
        errors.append(
            "manifest: mapped destination does not satisfy the pinned boolean inversion"
        )
    if (
        "destination_value_equals" in effective_rule
        and resolved != effective_rule["destination_value_equals"]
    ):
        errors.append(
            "manifest: mapped destination value differs from the pinned normalization"
        )
    if "destination_array_contains" in effective_rule and (
        not isinstance(resolved, list)
        or effective_rule["destination_array_contains"] not in resolved
    ):
        errors.append("manifest: mapped destination omits the pinned normalized value")


def validate_capture_timestamp(source, errors):
    value = source["captured_at"]
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        errors.append(
            "manifest: captured_at must be a valid timezone-qualified date-time"
        )
        return
    if parsed.tzinfo is None:
        errors.append("manifest: captured_at must include a timezone")


def validate_source_identity(source, errors):
    catalog = SOURCE_CATALOGS.get(source["source_profile"])
    if catalog is None:
        errors.append("manifest: unknown source_profile " + source["source_profile"])
        return
    if "repository" in catalog:
        if source.get("kind") != "git":
            errors.append("manifest: engine source_profile requires kind=git")
        if source.get("repository") != catalog["repository"]:
            errors.append(
                "manifest: source repository differs from pinned source_profile"
            )
        if source.get("revision") != catalog["revision"]:
            errors.append(
                "manifest: source revision differs from pinned source_profile"
            )
        artifact_digest = catalog["artifact_digests"].get(source.get("path"))
        if artifact_digest is None:
            errors.append(
                "manifest: engine source path is not an admitted Item artifact"
            )
        elif source.get("digest_sha256") != artifact_digest:
            errors.append(
                "manifest: engine source digest differs from the pinned artifact"
            )
    if catalog.get("authority") == "HISTORICAL_CORROBORATION_ONLY":
        if source.get("kind") not in ("wiki", "web"):
            errors.append("manifest: Fandom source_profile requires kind=wiki or web")
        if source.get("revision") != str(catalog["revision_id"]):
            errors.append(
                "manifest: Fandom source revision differs from pinned historical revision"
            )
        if source.get("url") != catalog["source_url"]:
            errors.append("manifest: Fandom URL differs from pinned historical source")
        if source.get("revision_sha1") != catalog["revision_sha1"]:
            errors.append(
                "manifest: Fandom revision SHA-1 differs from pinned historical revision"
            )
    if catalog.get("authority") == "CURRENT_FIELD_CENSUS_REFERENCE_ONLY":
        if source.get("kind") not in ("wiki", "web"):
            errors.append(
                "manifest: TibiaWiki BR source_profile requires kind=wiki or web"
            )
        if source.get("url") != catalog["source_url"]:
            errors.append(
                "manifest: TibiaWiki BR URL differs from pinned field census source"
            )
        if source.get("revision") != catalog["revision"]:
            errors.append(
                "manifest: TibiaWiki BR revision differs from pinned stable source"
            )
    if catalog.get("authority") == "PINNED_REAL_ITEM_PAGE_FIXTURE_SUPPLEMENT":
        if source.get("kind") != "wiki":
            errors.append("manifest: real-item page supplement requires kind=wiki")
        matches = [
            page
            for page in catalog["pages"]
            if source.get("url") == page["url"]
            and source.get("page_id") == page["page_id"]
            and source.get("revision") == page["revision"]
            and source.get("revision_timestamp") == page["revision_timestamp"]
            and source.get("revision_sha1") == page["revision_sha1"]
            and source.get("digest_sha256") == page["content_sha256"]
        ]
        if len(matches) != 1:
            errors.append(
                "manifest: source identity differs from every pinned real-item page"
            )
            return
        inventory = {field["source_field"] for field in source["field_inventory"]}
        if inventory != set(matches[0]["raw_fields"]):
            errors.append(
                "manifest: real-item page field inventory differs from the pinned revision"
            )


def pointer_matches(pointer_value, pattern):
    pointer_parts = pointer_value.split("/")
    pattern_parts = pattern.split("/")
    return len(pointer_parts) == len(pattern_parts) and all(
        expected == "*" or actual == expected
        for actual, expected in zip(pointer_parts, pattern_parts)
    )


def resolve_pointer(document, value):
    if not isinstance(value, str) or not value.startswith("/"):
        raise ValueError("expected absolute JSON pointer")
    current = document
    for part in value[1:].split("/"):
        if re.search(r"~(?:[^01]|$)", part):
            raise ValueError("invalid JSON pointer escape")
        key = part.replace("~1", "/").replace("~0", "~")
        if isinstance(current, list):
            if re.fullmatch(r"0|[1-9][0-9]*", key) is None:
                raise ValueError("invalid JSON array index")
            current = current[int(key)]
        elif isinstance(current, dict):
            current = current[key]
        else:
            raise TypeError("pointer traverses scalar")
    return current


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("item")
    parser.add_argument("dependencies")
    parser.add_argument("--manifest")
    arguments = parser.parse_args()
    issues, warnings = validate(
        read(arguments.item),
        read(arguments.dependencies),
        read(arguments.manifest) if arguments.manifest else None,
    )
    print(
        json.dumps(
            {
                "valid": not issues,
                "scope": "portable Item authoring structure and declared dependency closure; no runtime qualification",
                "import_manifest_checked": arguments.manifest is not None,
                "runtime_qualified": False,
                "warnings": warnings,
                "errors": issues,
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    raise SystemExit(bool(issues))
