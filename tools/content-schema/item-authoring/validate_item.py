"""Validate one Item authoring definition and its exact dependency catalog."""

import argparse
import json
import re
from datetime import datetime
from decimal import Decimal, InvalidOperation
from fractions import Fraction
from math import gcd
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parent
SCHEMA_NAMES = (
    "item.schema.json",
    "item-dependencies.schema.json",
    "item-import-readiness.schema.json",
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
)
SOURCE_CATALOGS = {
    catalog["source_profile"]: catalog
    for catalog in (
        json.loads((ROOT / name).read_text(encoding="utf-8"))
        for name in SOURCE_CATALOG_FILES
    )
}
SOURCE_CATALOGS[WIKI_CATALOG["source_profile"]] = WIKI_CATALOG
PERCENT_AT_MOST_100 = {
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
    "appearance_binding",
    "projectile_effect",
    "attack_effect",
    "color_binding",
}
ASSET_ARRAYS = {"sounds", "effects"}


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"), parse_float=Decimal)


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


def unique(values, fields, label, errors):
    keys = [tuple(value[field] for field in fields) for value in values]
    if len(keys) != len(set(keys)):
        errors.append(label + ": duplicate " + "/".join(fields))


def validate(item, dependencies, manifest=None):
    errors = structural("item.schema.json", item)
    errors += structural("item-dependencies.schema.json", dependencies)
    if manifest is not None:
        errors += structural("item-import-readiness.schema.json", manifest)
    if errors:
        return errors, []

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
            if parent in PERCENT_AT_MOST_100 and fraction(value) > 100:
                errors.append(location + ": percentage must not exceed 100")
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
        if pattern["hands"] == 2:
            other_hand = (
                "left_hand" if pattern["slot"] == "right_hand" else "right_hand"
            )
            if other_hand not in pattern.get("reserved_slots", []):
                errors.append(
                    "item/equipment: two-handed item must reserve the other hand"
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
        for augment in level["perks"]:
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
            if source["source_profile"] == WIKI_CATALOG["source_profile"]:
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
                    source["source_profile"] == WIKI_CATALOG["source_profile"]
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
    warnings = [
        "profile "
        + item["family_profile"]
        + " normally expects capability "
        + capability
        for capability in expected
        if capability not in item
    ]
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
    rules = {row["source_field"]: row for row in catalog["fields"]}
    rule = rules.get(entry["source_field"])
    if rule is None:
        errors.append(
            "manifest: unsupported field for source_profile "
            + source["source_profile"]
            + ": "
            + entry["source_field"]
        )
        return
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
    rule = next(
        (
            row
            for row in catalog["fields"]
            if row["source_field"] == entry["source_field"]
        ),
        None,
    )
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

