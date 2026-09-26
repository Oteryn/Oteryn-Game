"""Focused positive/negative verification for the Item authoring schema candidate v1."""

import json
from importlib.metadata import version
from pathlib import Path

from build_formal_schema import main as build
from jsonschema import Draft202012Validator
from jsonschema.exceptions import SchemaError

ROOT = Path(__file__).resolve().parent
build()

from validate_item import read, validate


def ref(family, slug):
    return {
        "family": family,
        "key": "oteryn:" + family.lower() + ".template." + slug,
        "revision": "definition-r1",
    }


def fixture():
    ability = ref("Ability", "magic-shot")
    effect = ref("Effect", "energy-hit")
    interaction = ref("Interaction", "target-creature")
    variant = ref("Item", "magic-wand-charged")
    item = {
        "identity": {"key": "oteryn:item.template.magic-wand", "revision": "definition-r1"},
        "display_name": "Template Magic Wand",
        "aliases": [],
        "family_profile": "weapon_magic",
        "presentation": {
            "grammar": {"article": "a", "plural": "template magic wands"},
            "appearance_binding": "oteryn:asset.item.magic-wand",
            "projectile_effect": "oteryn:asset.effect.energy-shot",
            "variants": [variant],
        },
        "taxonomy": {"item_class": "weapon", "primary": "wand", "tags": ["magic"]},
        "physical": {
            "weight": {"value": "21.50", "unit": "oz"},
            "movable": True,
            "pickupable": True,
        },
        "requirements": {
            "min_level": 20,
            "min_magic_level": 5,
            "vocations": ["sorcerer"],
            "context": "combat",
            "enforcement_mode": "both",
        },
        "equipment": {
            "slot": "right_hand",
            "hands": 1,
            "reserved_slots": [],
            "groups": ["magic_weapon"],
        },
        "weapon": {
            "weapon_type": "wand",
            "attack_modifier": 2,
            "range_cells": 4,
            "hit_chance_percent": {"numerator": 100, "denominator": 1},
            "damage_range": {"minimum": 10, "maximum": 20},
            "damage_type": "energy",
            "consumption_mode": "none",
        },
        "modifiers": {
            "resource_capacity": [{"resource": "mana", "flat": 10, "percent": {"numerator": 5, "denominator": 1}}],
            "critical_chance_percent": {"numerator": 5, "denominator": 1},
            "critical_damage_percent": {"numerator": 50, "denominator": 1},
            "mana_shield_enabled": True,
            "magic_shield_capacity": {"flat": 25, "percent": {"numerator": 10, "denominator": 1}},
            "reflection": [{"damage_type": "energy", "flat": 1, "percent": {"numerator": 5, "denominator": 1}}],
        },
        "proficiency": {
            "augments": [
                {
                    "key": "energy_damage",
                    "target": {"kind": "ability", "ability": ability},
                    "effect": effect,
                    "rank_values": [{"rank": 1, "value": {"kind": "signed_points", "value": 2}}],
                }
            ]
        },
        "use": {
            "usable": True,
            "use_with": True,
            "interactions": [interaction],
            "ability": ability,
            "effect": effect,
            "mana_cost": 4,
        },
        "lifecycle": {
            "enchantable": True,
            "enchanted_variant": variant,
            "destructible": False,
            "transforms": [{"trigger": "use", "target": variant}],
        },
        "trade": {"tradeable": True, "marketable": True, "market_category": "weapons"},
    }
    dependencies = {
        "definitions": [ability, effect, interaction, variant],
        "assets": ["oteryn:asset.item.magic-wand", "oteryn:asset.effect.energy-shot"],
    }
    manifest = {
        "sources": [
            {
                "kind": "git",
                "repository": "opentibiabr/canary",
                "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
                "path": "data/items/items.xml",
                "captured_at": "2026-09-26T20:00:00Z",
                "digest_sha256": "0" * 64,
                "field_inventory": [
                    {"source_locator": "item[@id='template']", "source_field": "weaponType"},
                    {"source_locator": "item[@id='template']", "source_field": "floorChange"},
                ],
            }
        ],
        "entries": [
            {
                "source_index": 0,
                "source_locator": "item[@id='template']",
                "source_field": "weaponType",
                "kind": "definition",
                "status": "mapped",
                "destination": "/item/weapon/weapon_type",
                "reason": "typed portable definition fact",
            },
            {
                "source_index": 0,
                "source_locator": "item[@id='template']",
                "source_field": "floorChange",
                "kind": "terrain",
                "status": "external_domain",
                "reason": "Terrain/WorldObject owns placed world traversal",
            },
        ],
    }
    return item, dependencies, manifest


def wiki_field(manifest, source_field, kind, status, destination=None, promotions=None):
    manifest["sources"][0].update(
        {
            "kind": "wiki",
            "field_inventory": [{"source_locator": "Infobox Item", "source_field": source_field}],
        }
    )
    entry = {
        "source_index": 0,
        "source_locator": "Infobox Item",
        "source_field": source_field,
        "kind": kind,
        "status": status,
        "reason": "focused Wiki disposition fixture",
    }
    if destination is not None:
        entry["destination"] = destination
    if promotions is not None:
        entry["promotions"] = promotions
    manifest["entries"] = [entry]


def main():
    results = []

    def case(name, mutate=None, expected_valid=False, expected_warning=None):
        item, dependencies, manifest = fixture()
        if mutate:
            mutate(item, dependencies, manifest)
        errors, warnings = validate(item, dependencies, manifest)
        passed = (not errors) == expected_valid
        if expected_warning is not None:
            passed = passed and any(expected_warning in warning for warning in warnings)
        results.append(
            {
                "name": name,
                "passed": passed,
                "expected_valid": expected_valid,
                "error_count": len(errors),
                "warning_count": len(warnings),
            }
        )

    case("complete item bundle validates", expected_valid=True)
    case(
        "light radius remains optional evidence",
        lambda item, dependencies, manifest: (
            item.__setitem__("light", {"emits": True, "color_binding": "oteryn:asset.color.warm", "intensity": 5}),
            dependencies["assets"].append("oteryn:asset.color.warm"),
        ),
        True,
    )
    case(
        "multiple equipment patterns validate",
        lambda item, dependencies, manifest: item.__setitem__(
            "equipment",
            {
                "patterns": [
                    {"pattern_id": 1, "slot": "right_hand", "hands": 1, "vocations": ["sorcerer"], "min_level": 20},
                    {"pattern_id": 2, "slot": "left_hand", "hands": 1, "vocations": ["druid"], "min_level": 20},
                ]
            },
        ),
        True,
    )
    case(
        "profile omissions are advisory, not incompatible schemas",
        lambda item, dependencies, manifest: item.pop("trade"),
        True,
        "normally expects capability trade",
    )

    templates = sorted((ROOT / "templates").glob("*.json"))
    for path in templates:
        item = read(path)
        errors, warnings = validate(item, {"definitions": [], "assets": []})
        results.append(
            {
                "name": "template validates: " + path.name,
                "passed": not errors,
                "error_count": len(errors),
                "warning_count": len(warnings),
            }
        )
    results.append({"name": "expected template count is 13", "passed": len(templates) == 13})

    for field, value in (
        ("server_id", 100),
        ("client_id", 200),
        ("source_page_id", 300),
        ("position", {"x": 1, "y": 2, "z": 3}),
        ("blocking", True),
        ("pathfinding", {"walkable": False}),
        ("instance_count", 7),
        ("instance_text", "mutable"),
        ("owner_character_id", "character-1"),
    ):
        case(
            "reject non-Item root field " + field,
            lambda item, dependencies, manifest, field=field, value=value: item.__setitem__(field, value),
        )

    case(
        "reject unresolved definition reference",
        lambda item, dependencies, manifest: dependencies["definitions"].pop(),
    )
    case(
        "reject unresolved asset",
        lambda item, dependencies, manifest: dependencies["assets"].pop(),
    )
    case(
        "reject noncanonical ratio",
        lambda item, dependencies, manifest: item["weapon"].__setitem__("hit_chance_percent", {"numerator": 2, "denominator": 2}),
    )
    case(
        "reject reflection without flat or percent value",
        lambda item, dependencies, manifest: item["modifiers"].__setitem__("reflection", [{"damage_type": "energy"}]),
    )
    case(
        "reject duplicate specialized magic combat type",
        lambda item, dependencies, manifest: item["modifiers"].__setitem__(
            "magic_level",
            [{"combat_type": "death", "amount": 2}, {"combat_type": "death", "amount": 3}],
        ),
    )
    case(
        "reject free-form proficiency augment",
        lambda item, dependencies, manifest: item.__setitem__("proficiency", {"augments": ["damage"]}),
    )
    case(
        "reject duplicate augment rank",
        lambda item, dependencies, manifest: item["proficiency"]["augments"][0].__setitem__(
            "rank_values",
            [
                {"rank": 1, "value": {"kind": "signed_points", "value": 2}},
                {"rank": 1, "value": {"kind": "signed_points", "value": 3}},
            ],
        ),
    )
    case(
        "reject percent above 100",
        lambda item, dependencies, manifest: item["weapon"].__setitem__("hit_chance_percent", {"numerator": 101, "denominator": 1}),
    )
    case(
        "reject inverted weapon damage range",
        lambda item, dependencies, manifest: item["weapon"].__setitem__("damage_range", {"minimum": 20, "maximum": 10}),
    )
    case(
        "reject two hands without reserving other hand",
        lambda item, dependencies, manifest: item["equipment"].update({"hands": 2, "reserved_slots": []}),
    )
    case(
        "reject two hands reserving the same hand",
        lambda item, dependencies, manifest: item["equipment"].update({"hands": 2, "reserved_slots": ["right_hand"]}),
    )
    case(
        "reject duplicate equipment pattern IDs",
        lambda item, dependencies, manifest: item.__setitem__(
            "equipment",
            {
                "patterns": [
                    {"pattern_id": 1, "slot": "right_hand", "hands": 1},
                    {"pattern_id": 1, "slot": "left_hand", "hands": 1},
                ]
            },
        ),
    )
    case(
        "reject fluid content without fluid_type",
        lambda item, dependencies, manifest: item.__setitem__("fluid", {"role": "content"}),
    )
    case(
        "reject placed fluid source role",
        lambda item, dependencies, manifest: item.__setitem__("fluid", {"role": "source", "fluid_type": "water"}),
    )
    case(
        "reject placed bed capability",
        lambda item, dependencies, manifest: item.__setitem__("bed", {"sleepable": True}),
    )
    case(
        "reject use_with when unusable",
        lambda item, dependencies, manifest: item.__setitem__("use", {"usable": False, "use_with": True}),
    )
    case(
        "reject direct self transformation",
        lambda item, dependencies, manifest: item["lifecycle"].__setitem__(
            "transforms",
            [{"trigger": "use", "target": {"family": "Item", **item["identity"]}}],
        ),
    )
    case(
        "reject unresolved manifest semantics",
        lambda item, dependencies, manifest: manifest["entries"][0].update({"status": "unresolved_semantics", "reason": "unknown", "destination": "/item/weapon/weapon_type"}),
    )
    case(
        "reject mapped manifest destination that does not exist",
        lambda item, dependencies, manifest: manifest["entries"][0].__setitem__("destination", "/item/weapon/not_a_field"),
    )
    case(
        "reject destination on external-domain manifest entry",
        lambda item, dependencies, manifest: manifest["entries"][1].__setitem__("destination", "/item/taxonomy"),
    )
    case(
        "reject terrain mapped into portable Item",
        lambda item, dependencies, manifest: manifest["entries"][1].update({"status": "mapped", "destination": "/item/display_name"}),
    )
    case(
        "reject duplicate source-field disposition",
        lambda item, dependencies, manifest: manifest["entries"].append(dict(manifest["entries"][0])),
    )
    case(
        "reject incomplete source-field disposition",
        lambda item, dependencies, manifest: manifest["entries"].pop(),
    )
    case(
        "reject unused exact dependency",
        lambda item, dependencies, manifest: dependencies["definitions"].append(ref("Item", "unused")),
    )
    case(
        "reject unused asset binding",
        lambda item, dependencies, manifest: dependencies["assets"].append("oteryn:asset.unused"),
    )
    case(
        "reject duplicate transform trigger",
        lambda item, dependencies, manifest: (
            item["lifecycle"]["transforms"].append({"trigger": "use", "target": ref("Item", "other")}),
            dependencies["definitions"].append(ref("Item", "other")),
        ),
    )
    case(
        "reject conflicting temporal and lifecycle decay targets",
        lambda item, dependencies, manifest: (
            item.__setitem__("temporal", {"duration_ms": 1000, "consumption_mode": "continuous", "decay_target": ref("Item", "magic-wand-charged")}),
            item["lifecycle"]["transforms"].append({"trigger": "decay", "target": ref("Item", "other")}),
            dependencies["definitions"].append(ref("Item", "other")),
        ),
    )
    case(
        "reject writable but unreadable document",
        lambda item, dependencies, manifest: item.__setitem__("readable", {"readable": False, "writable": True, "write_policy": "rewrite", "max_characters": 100}),
    )
    case(
        "reject write-once document without target",
        lambda item, dependencies, manifest: item.__setitem__("readable", {"readable": True, "writable": True, "write_policy": "write_once", "max_characters": 100}),
    )
    case(
        "reject imbuement family both allowed and excluded",
        lambda item, dependencies, manifest: item.__setitem__("imbuement", {"slot_count": 1, "allowed_family_tiers": [{"family": "fire", "tier": 1}], "excluded_families": ["fire"]}),
    )
    case(
        "reject Wiki reverse relation mislabeled as Item definition",
        lambda item, dependencies, manifest: (
            manifest["sources"][0].update({"kind": "wiki", "field_inventory": [{"source_locator": "Infobox Item", "source_field": "droppedby"}]}),
            manifest.__setitem__("entries", [{"source_index": 0, "source_locator": "Infobox Item", "source_field": "droppedby", "kind": "definition", "status": "mapped", "destination": "/item/display_name", "reason": "bad intrinsic mapping"}]),
        ),
    )
    case(
        "Wiki reverse relation routes outside Item",
        lambda item, dependencies, manifest: (
            manifest["sources"][0].update({"kind": "wiki", "field_inventory": [{"source_locator": "Infobox Item", "source_field": "droppedby"}]}),
            manifest.__setitem__("entries", [{"source_index": 0, "source_locator": "Infobox Item", "source_field": "droppedby", "kind": "relationship", "status": "reverse_relation", "reason": "Creature/Loot owns the relation"}]),
        ),
        True,
    )
    case(
        "reject Wiki attack mapped to the wrong Item leaf",
        lambda item, dependencies, manifest: wiki_field(manifest, "attack", "definition", "mapped", "/item/display_name"),
    )
    case(
        "Wiki attack maps to the formal attack leaf",
        lambda item, dependencies, manifest: (
            item["weapon"].__setitem__("attack", 12),
            wiki_field(manifest, "attack", "definition", "mapped", "/item/weapon/attack"),
        ),
        True,
    )
    case(
        "reject Wiki attrib mapped to the wrong Item leaf",
        lambda item, dependencies, manifest: (
            item.__setitem__("source_observations", {"attributes_text": "It emits light."}),
            wiki_field(manifest, "attrib", "raw_text", "mapped", "/item/display_name"),
        ),
    )
    case(
        "Wiki attrib retains raw text and resolves a typed promotion",
        lambda item, dependencies, manifest: (
            item.__setitem__("source_observations", {"attributes_text": "It emits light."}),
            item.__setitem__("light", {"emits": True, "color_binding": "oteryn:asset.color.warm", "intensity": 5}),
            dependencies["assets"].append("oteryn:asset.color.warm"),
            wiki_field(
                manifest,
                "attrib",
                "raw_text",
                "mapped",
                "/item/source_observations/attributes_text",
                ["/item/light/intensity"],
            ),
        ),
        True,
    )

    before = {
        path.relative_to(ROOT).as_posix(): path.read_bytes()
        for path in [
            ROOT / "item.schema.json",
            ROOT / "item-dependencies.schema.json",
            ROOT / "item-import-readiness.schema.json",
            ROOT / "profile-catalog.json",
            ROOT / "wiki-field-dispositions.json",
            ROOT / "item-dependencies-template.json",
            *templates,
        ]
    }
    build()
    after = {path: (ROOT / path).read_bytes() for path in before}
    results.append({"name": "generator is byte deterministic", "passed": before == after})
    census = json.loads((ROOT.parents[2] / "docs/agents/evidence/OTV2-20260925-tibiawiki-item-master-field-census-v1.json").read_text(encoding="utf-8"))
    wiki_catalog = read(ROOT / "wiki-field-dispositions.json")
    results.append(
        {
            "name": "Wiki disposition catalog exactly matches the protected 71-field census",
            "passed": [
                (row["source_parameter"], row["disposition"], row.get("canonical_path"))
                for row in census["field_mappings"]
            ]
            == [
                (row["source_field"], row["disposition"], row.get("canonical_path"))
                for row in wiki_catalog["fields"]
            ],
        }
    )
    results.append(
        {
            "name": "every mappable Wiki field has an allowed formal destination and routed fields do not",
            "passed": all(
                bool(row["allowed_destinations"])
                == (row["disposition"] in ("ITEM_TYPED", "ITEM_AUTHORING", "PRESENTATION_EDITOR", "SOURCE_TEXT_PRESERVE_AND_PARSE"))
                for row in wiki_catalog["fields"]
            ),
        }
    )
    for path in templates:
        errors, warnings = validate(read(path), {"definitions": [], "assets": []})
        results.append({"name": "official template is warning-free: " + path.name, "passed": not errors and not warnings})
    for schema_name in ("item.schema.json", "item-dependencies.schema.json", "item-import-readiness.schema.json"):
        try:
            Draft202012Validator.check_schema(read(ROOT / schema_name))
            valid_schema = True
        except SchemaError:
            valid_schema = False
        results.append({"name": "Draft 2020-12 metaschema accepts " + schema_name, "passed": valid_schema})

    item, dependencies, manifest = fixture()
    (ROOT / "synthetic-valid-item.json").write_text(
        json.dumps(item, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    (ROOT / "synthetic-valid-dependencies.json").write_text(
        json.dumps(dependencies, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    (ROOT / "synthetic-valid-import-readiness.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    report = {
        "scope": "portable Item authoring schemas and semantic validator; no runtime or corpus migration executed",
        "jsonschema_version": version("jsonschema"),
        "templates": len(templates),
        "checks": len(results),
        "passed": sum(result["passed"] for result in results),
        "failed": sum(not result["passed"] for result in results),
        "results": results,
    }
    (ROOT / "formal-schema-validation-report.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    print(json.dumps({key: value for key, value in report.items() if key != "results"}, ensure_ascii=False))
    for result in results:
        if not result["passed"]:
            print(json.dumps(result, ensure_ascii=False))
    raise SystemExit(report["failed"] > 0)


if __name__ == "__main__":
    main()
