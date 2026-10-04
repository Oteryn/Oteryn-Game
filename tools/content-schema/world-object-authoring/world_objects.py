"""WorldObject/Terrain authoring tooling (WO-1) for the WO-0 format decision.

Builds one Terrain or WorldObject catalog record per routed engine id, validates it and
writes a deterministic census. It mints no identity: a family key is a pure function of
the Tibia Item key (A12 §4.6) and the converter route (D93), and it writes nothing under
`content/` (population is WO-2).

- Routing is `engine_items.convert_item`'s own `routed_non_item` decision, unchanged.
- D94 exclusions: empty appearance slots, ids without a client appearance and the Fluid
  kinds get no family key.
- Facts: a boolean `appearances.dat` flag is complete for an id that has an appearance
  (an unset optional bool is false), so it is KNOWN either way. An `items.xml`
  attribute is sparse: absent means UNKNOWN, never an engine default.
- Relations to other items (`rotateto`, bed parts and transforms) stay source ids,
  resolved to typed references by WO-2.

Architecture: docs/architecture/reviews/
OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parent
ITEM_AUTHORING = ROOT.parent / "item-authoring"
sys.path.insert(0, str(ITEM_AUTHORING))

import engine_items

DEFAULT_SAMPLE = ROOT / "samples" / "census-crystal-ff7ede5.json"
DEFINITION_REVISION = "definition-r1"
ITEM_KEY = re.compile(r"^oteryn:item\.tibia\.i([1-9][0-9]*)$")
FAMILY_PREFIX = {
    "Terrain": "oteryn:terrain.tibia.i",
    "WorldObject": "oteryn:world-object.tibia.i",
}
FAMILY_KEY = {
    family: re.compile("^" + re.escape(prefix) + "([1-9][0-9]*)$")
    for family, prefix in FAMILY_PREFIX.items()
}
# D94: no family key for these routes (the Fluid kinds stay Item fluid).
EXCLUDED_ROUTES = {
    ("WorldObject", "appearance_placeholder_slot"),
    ("WorldObject", "no_client_appearance"),
    ("Fluid", "fluid_type_without_appearance"),
}
TERRAIN_KINDS = ("ground", "border", "wall", "field", "roof")
WORLD_OBJECT_KINDS = (
    "object",
    "door",
    "ladder",
    "bed",
    "container_fixture",
    "teleport",
    "corpse",
    "decoration",
)
# items.xml `type` -> WorldObject kind (WO-0 §4.3).
WORLD_OBJECT_TYPE_KINDS = {
    "door": "door",
    "ladder": "ladder",
    "bed": "bed",
    "teleport": "teleport",
    "depot": "container_fixture",
    "mailbox": "container_fixture",
    "trashholder": "container_fixture",
    "rewardchest": "container_fixture",
    "carpet": "decoration",  # owner decision WO-2c 5a
}
TERRAIN_FIELD_TYPES = {"magicfield"}
# items.xml `type` on a Terrain tile -> the Interaction behaviour it carries (owner
# decisions WO-2c 2a, 4a): the tile stays its own kind; Interaction owns the behaviour.
TERRAIN_BEHAVIORS = {"trashholder": "trash_holder", "teleport": "teleport"}
HOOK_DIRECTIONS = {1: "south", 2: "east"}  # appearances.proto HOOK_TYPE
SCHEMA_FILES = {
    "Terrain": "terrain.schema.json",
    "WorldObject": "world-object.schema.json",
    "RoutedItemPointer": "routed-item-pointer.schema.json",
}


# --- identity (D93) -----------------------------------------------------------------


def family_key(item_key, family):
    """Return the D93 family key for a Tibia Item key (A12 §4.6); raise on anything else."""
    if family not in FAMILY_PREFIX:
        raise ValueError(f"no family key for family {family!r}")
    match = ITEM_KEY.match(item_key or "")
    if match is None:
        raise ValueError(f"not a Tibia Item key: {item_key!r}")
    return FAMILY_PREFIX[family] + match.group(1)


def family_for_route(owner, reason):
    """Return the family of a routed id, or None when D94 excludes the route."""
    if (owner, reason) in EXCLUDED_ROUTES:
        return None
    if owner not in FAMILY_PREFIX:
        raise ValueError(f"unexpected route owner {owner!r} ({reason})")
    return owner


# --- fact wrappers ------------------------------------------------------------------


def known(value):
    return {"state": "KNOWN", "value": value}


UNKNOWN = {"state": "UNKNOWN"}


class Facts:
    """Reads one id's facts and records which source field backs each output field."""

    def __init__(self, xml_record, appearance):
        self.attrs = dict(xml_record["attrs"]) if xml_record else {}
        self.appearance = appearance
        self.flags = dict(appearance["flags"]) if appearance else {}
        self.sources = defaultdict(list)

    def _note(self, field, source):
        if source not in self.sources[field]:
            self.sources[field].append(source)

    def flag(self, field, flag):
        """A boolean appearance flag: KNOWN whenever the id has an appearance."""
        if self.appearance is None:
            return dict(UNKNOWN)
        self._note(field, f"appearance:{flag}")
        return known(bool(self.flags.get(flag)))

    def flag_value(self, field, flag, value_key, convert=None, unset=None):
        """A value carried by an appearance flag. An unset flag is the complete fact
        `unset` when one is given (no hook, no elevation), else UNKNOWN; a set flag
        without its value is UNKNOWN."""
        if self.appearance is None:
            return dict(UNKNOWN)
        if not self.flags.get(flag):
            if unset is None:
                return dict(UNKNOWN)
            self._note(field, f"appearance:{flag}")
            return known(unset)
        if value_key not in self.flags:
            return dict(UNKNOWN)
        self._note(field, f"appearance:{value_key}")
        value = self.flags[value_key]
        return known(convert(value) if convert else value)

    def attr_text(self, field, name):
        value = self.attrs.get(name)
        if value is None or str(value).strip() == "":
            return dict(UNKNOWN)
        self._note(field, f"items_xml:{name}")
        return known(str(value).strip())

    def attr_bool(self, field, name):
        value = self.attrs.get(name)
        if value is None:
            return dict(UNKNOWN)
        self._note(field, f"items_xml:{name}")
        return known(engine_items.truthy(value))

    def attr_int(self, field, name):
        value = self.attrs.get(name)
        if value is None:
            return dict(UNKNOWN)
        number = int(str(value).strip())
        if number < 0:
            raise ValueError(f"negative {name}={value!r}")
        self._note(field, f"items_xml:{name}")
        return known(number)

    def attr_source_ref(self, field, name):
        value = self.attrs.get(name)
        if value is None:
            return dict(UNKNOWN)
        number = int(str(value).strip())
        if number < 1:
            return dict(UNKNOWN)
        self._note(field, f"items_xml:{name}")
        return known({"source_item_id": number})

    def client_projection(self, item_id):
        if self.appearance is None:
            return dict(UNKNOWN)
        self._note("client_projection", "appearance:id")
        return known({"appearance_id": item_id})


# --- kind rules ---------------------------------------------------------------------


def terrain_kind(attrs, flags, name=""):
    """WO-0 §4.2 kind, first matching rule wins; None when no rule applies."""
    if attrs.get("type") in TERRAIN_FIELD_TYPES or "field" in attrs:
        return "field"
    if flags.get("flags.bank") or flags.get("flags.fullbank"):
        return "ground"
    if flags.get("flags.clip"):
        return "border"
    if flags.get("flags.bottom") or attrs.get("primarytype") == "walls":
        return "wall"
    if attrs.get("primarytype") == "fields":
        return "field"
    # Owner decision WO-2c 1a: a tile named as a roof, with no ground, border or wall flag.
    if "roof" in name.lower().split():
        return "roof"
    return None


def world_object_kind(reason, attrs, flags):
    """WO-0 §4.3 kind, first matching rule wins; None when no rule applies."""
    if reason in ("corpse", "corpse_decoration"):
        return "corpse"
    type_value = attrs.get("type")
    if type_value is not None:
        return WORLD_OBJECT_TYPE_KINDS.get(type_value)
    interactive = any(
        flags.get(flag)
        for flag in (
            "flags.unpass",
            "flags.usable",
            "flags.forceuse",
            "flags.multiuse",
            "flags.container",
        )
    )
    return "object" if interactive else "decoration"


def type_outside_family(family, attrs):
    """An items.xml `type` the family's kind set does not express (contested route)."""
    type_value = attrs.get("type")
    if type_value is None:
        return None
    if family == "Terrain":
        expressed = TERRAIN_FIELD_TYPES | set(TERRAIN_BEHAVIORS)
        return None if type_value in expressed else type_value
    return None if type_value in WORLD_OBJECT_TYPE_KINDS else type_value


# --- records ------------------------------------------------------------------------


def provenance(source_meta, item_id, item_key, owner, reason, facts):
    return {
        "source": dict(source_meta),
        "source_item_id": item_id,
        "item_pointer": {
            "family": "Item",
            "key": item_key,
            "revision": DEFINITION_REVISION,
        },
        "route": {"owner": owner, "reason": reason},
        "fields": {
            field: list(sources) for field, sources in sorted(facts.sources.items())
        },
    }


def build_terrain(item_id, item_key, reason, xml_record, appearance, source_meta):
    facts = Facts(xml_record, appearance)
    name = (xml_record.get("name") if xml_record else None) or ""
    kind = terrain_kind(facts.attrs, facts.flags, name)
    if kind is not None:
        facts._note("kind", "converter:terrain_kind")
    unpass = facts.flag("walkable", "flags.unpass")
    record = {
        "identity": {
            "family": "Terrain",
            "key": family_key(item_key, "Terrain"),
            "revision": DEFINITION_REVISION,
        },
        "kind": known(kind) if kind else dict(UNKNOWN),
        "walkable": known(not unpass["value"])
        if unpass["state"] == "KNOWN"
        else unpass,
        "blocks_projectile": facts.attr_bool("blocks_projectile", "blockprojectile"),
        "blocks_sight": facts.flag("blocks_sight", "flags.unsight"),
        "floor_change": facts.attr_text("floor_change", "floorchange"),
        "automap": automap(facts),
        "client_projection": facts.client_projection(item_id),
    }
    if kind == "ground":
        record["ground_speed"] = facts.flag_value(
            "ground_speed", "flags.bank", "bank.waypoints"
        )
    if kind == "field":
        field_type = facts.attr_text("field", "field")
        record["field"] = (
            known({"field_type": field_type["value"]})
            if field_type["state"] == "KNOWN"
            else field_type
        )
    behavior = TERRAIN_BEHAVIORS.get(facts.attrs.get("type"))
    if behavior is not None:
        facts._note("behavior", "items_xml:type")
        record["behavior"] = known(behavior)
    record["provenance"] = provenance(
        source_meta, item_id, item_key, "Terrain", reason, facts
    )
    return record


def automap(facts):
    if facts.appearance is None:
        return dict(UNKNOWN)
    facts._note("automap", "appearance:flags.automap")
    if not facts.flags.get("flags.automap"):
        return known({"shown": False})
    value = {"shown": True}
    if "automap.color" in facts.flags:
        facts._note("automap", "appearance:automap.color")
        value["color"] = facts.flags["automap.color"]
    return known(value)


def build_world_object(item_id, item_key, reason, xml_record, appearance, source_meta):
    facts = Facts(xml_record, appearance)
    kind = world_object_kind(reason, facts.attrs, facts.flags)
    if kind is not None:
        facts._note("kind", "converter:world_object_kind")
    unmove = facts.flag("movable", "flags.unmove")
    record = {
        "identity": {
            "family": "WorldObject",
            "key": family_key(item_key, "WorldObject"),
            "revision": DEFINITION_REVISION,
        },
        "kind": known(kind) if kind else dict(UNKNOWN),
        "collision": {
            "blocks_movement": facts.flag("collision.blocks_movement", "flags.unpass"),
            "avoid": facts.flag("collision.avoid", "flags.avoid"),
            "blocks_projectile": facts.attr_bool(
                "collision.blocks_projectile", "blockprojectile"
            ),
            "blocks_sight": facts.flag("collision.blocks_sight", "flags.unsight"),
        },
        "movable": known(not unmove["value"]) if unmove["state"] == "KNOWN" else unmove,
        "placement": {
            "hangable": facts.flag("placement.hangable", "flags.hang"),
            "hook_direction": facts.flag_value(
                "placement.hook_direction",
                "flags.hook",
                "hook.direction",
                convert=hook_direction,
                unset="none",
            ),
            "rotatable": facts.flag("placement.rotatable", "flags.rotate"),
            "rotate_to": facts.attr_source_ref("placement.rotate_to", "rotateto"),
            "elevation": facts.flag_value(
                "placement.elevation", "flags.height", "height.elevation", unset=0
            ),
            "walk_stack": facts.attr_bool("placement.walk_stack", "walkstack"),
        },
        "floor_change": facts.attr_text("floor_change", "floorchange"),
        "fluid_source": {
            "fluid_name": facts.attr_text("fluid_source.fluid_name", "fluidsource"),
            "liquid_pool": facts.flag("fluid_source.liquid_pool", "flags.liquidpool"),
        },
        "client_projection": facts.client_projection(item_id),
    }
    if kind == "bed":
        record["bed"] = {
            "part": facts.attr_text("bed.part", "bedpart"),
            "part_of": facts.attr_source_ref("bed.part_of", "bedpartof"),
            "partner_direction": facts.attr_text(
                "bed.partner_direction", "partnerdirection"
            ),
            "male_sleeper": facts.attr_source_ref("bed.male_sleeper", "malesleeper"),
            "female_sleeper": facts.attr_source_ref(
                "bed.female_sleeper", "femalesleeper"
            ),
            "male_transform_to": facts.attr_source_ref(
                "bed.male_transform_to", "maletransformto"
            ),
            "female_transform_to": facts.attr_source_ref(
                "bed.female_transform_to", "femaletransformto"
            ),
        }
    if kind == "corpse":
        # Decay target, duration and capacity stay on the routed Item record (WO-0 §4.3).
        record["corpse"] = {
            "corpse": facts.flag("corpse.corpse", "flags.corpse"),
            "player_corpse": facts.flag("corpse.player_corpse", "flags.player_corpse"),
        }
    if kind == "door":
        record["door"] = {
            "level": facts.attr_int("door.level", "leveldoor"),
            "house_guests": facts.attr_bool("door.house_guests", "usedbyhouseguests"),
        }
    record["provenance"] = provenance(
        source_meta, item_id, item_key, "WorldObject", reason, facts
    )
    return record


def hook_direction(value):
    if value not in HOOK_DIRECTIONS:
        raise ValueError(f"unknown hook direction {value!r}")
    return HOOK_DIRECTIONS[value]


def build_record(
    family, item_id, item_key, reason, xml_record, appearance, source_meta
):
    if family == "Terrain":
        return build_terrain(
            item_id, item_key, reason, xml_record, appearance, source_meta
        )
    if family == "WorldObject":
        return build_world_object(
            item_id, item_key, reason, xml_record, appearance, source_meta
        )
    raise ValueError(f"no record builder for family {family!r}")


def routed_item_pointer(item_key, family):
    """The WO-0 §4.1 typed `routed_to` pointer the routed Item record will carry."""
    return {
        "identity": {
            "family": "Item",
            "key": item_key,
            "revision": DEFINITION_REVISION,
        },
        "materializable": False,
        "routed_to": {
            "family": family,
            "key": family_key(item_key, family),
            "revision": DEFINITION_REVISION,
        },
    }


# --- validation ---------------------------------------------------------------------


def _registry():
    resources = []
    for name in SCHEMA_FILES.values():
        schema = json.loads((ROOT / name).read_text(encoding="utf-8"))
        resources.append((schema["$id"], Resource.from_contents(schema)))
    return Registry().with_resources(resources)


_VALIDATORS = {}


def validator(name):
    if name not in _VALIDATORS:
        registry = _registry()
        schema = json.loads((ROOT / SCHEMA_FILES[name]).read_text(encoding="utf-8"))
        Draft202012Validator.check_schema(schema)
        _VALIDATORS[name] = Draft202012Validator(schema, registry=registry)
    return _VALIDATORS[name]


def schema_errors(name, document):
    return sorted(
        f"{'/'.join(str(part) for part in error.absolute_path) or '<root>'}: {error.message}"
        for error in validator(name).iter_errors(document)
    )


def sequence_number(key, family):
    match = (ITEM_KEY if family == "Item" else FAMILY_KEY[family]).match(key)
    return match.group(1) if match else None


def validate_record(record):
    """Structural plus semantic errors for one Terrain or WorldObject record."""
    family = (record.get("identity") or {}).get("family")
    if family not in ("Terrain", "WorldObject"):
        return [f"identity/family: unsupported family {family!r}"]
    errors = schema_errors(family, record)
    if errors:
        return errors
    identity_number = sequence_number(record["identity"]["key"], family)
    pointer_number = sequence_number(
        record["provenance"]["item_pointer"]["key"], "Item"
    )
    if identity_number != pointer_number:
        errors.append(
            "identity/key: sequence number differs from the Item pointer (D93)"
        )
    if record["provenance"]["route"]["owner"] != family:
        errors.append("provenance/route/owner: differs from the record family")
    kind = record["kind"].get("value")
    sections = (
        {"ground_speed": "ground", "field": "field"}
        if family == "Terrain"
        else {"bed": "bed", "corpse": "corpse", "door": "door"}
    )
    for section, section_kind in sections.items():
        if section in record and kind != section_kind:
            errors.append(f"{section}: only allowed on kind {section_kind!r}")
        if section not in record and kind == section_kind:
            errors.append(f"{section}: required on kind {section_kind!r}")
    import official_corpses
    import qualified_world

    source = record["provenance"]["source"]
    if (
        source.get("profile") in qualified_world.PROFILES
        or record["provenance"]["source_item_id"] in qualified_world.IDS
        or record["provenance"]["item_pointer"]["key"] in qualified_world.KEYS
    ):
        errors.extend(qualified_world.validate_record(record))
    if (
        source.get("engine") == "official_client"
        and source.get("profile") not in qualified_world.PROFILES
        or source.get("profile") == official_corpses.PROFILE
        or record["provenance"]["source_item_id"] in official_corpses.IDS
        or record["provenance"]["item_pointer"]["key"] in official_corpses.KEYS
    ):
        errors.extend(official_corpses.validate_record(record))
    for field in known_fields(record):
        if field not in record["provenance"]["fields"]:
            errors.append(f"provenance/fields: no source for KNOWN {field}")
    return errors


def known_fields(record, prefix=""):
    """Dotted names of every KNOWN wrapper in a record (provenance excluded)."""
    names = []
    for name, value in record.items():
        if name in ("identity", "provenance") or not isinstance(value, dict):
            continue
        dotted = f"{prefix}{name}"
        if value.get("state") == "KNOWN":
            names.append(dotted)
        elif "state" not in value:
            names.extend(known_fields(value, prefix=f"{dotted}."))
    return names


def validate_routed_item_pointer(pointer):
    errors = schema_errors("RoutedItemPointer", pointer)
    if errors:
        return errors
    item_number = sequence_number(pointer["identity"]["key"], "Item")
    family = pointer["routed_to"]["family"]
    if sequence_number(pointer["routed_to"]["key"], family) != item_number:
        errors.append("routed_to/key: sequence number differs from the Item key (D93)")
    return errors


# --- census -------------------------------------------------------------------------


def canonical_bytes(value):
    return json.dumps(
        value, sort_keys=True, ensure_ascii=False, separators=(",", ":")
    ).encode("utf-8")


def source_meta_of(sources):
    return {
        "engine": sources["engine"],
        "repository": sources["repository"],
        "revision": sources["revision"],
        "profile": sources["profile"],
    }


def iter_routed(sources):
    """Yield (item_id, item_key, owner, reason) for every routed id, ascending."""
    for item_id in sorted(sources["items"]):
        _item, _dependencies, report = engine_items.convert_item(sources, item_id)
        routed = report.get("routed_non_item")
        if routed:
            yield item_id, report.get("key"), routed["owner"], routed["reason"]


def build_census(sources, records_dir=None, on_record=None):
    """Return the deterministic census dict; optionally write every record.

    `on_record(family, record)` receives every validated record in ascending source id
    order; the WO-2 catalogue builder collects them through it.
    """
    meta = source_meta_of(sources)
    routes = Counter()
    excluded = Counter()
    kinds = Counter()
    unknown_kind_examples = defaultdict(list)
    outside = Counter()
    outside_examples = defaultdict(list)
    unknown_facts = Counter()
    no_appearance = Counter()
    examples = defaultdict(list)
    keys = set()
    digest = hashlib.sha256()
    for item_id, item_key, owner, reason in iter_routed(sources):
        routes[f"{owner}:{reason}"] += 1
        family = family_for_route(owner, reason)
        if family is None:
            excluded[f"{owner}:{reason}"] += 1
            continue
        if item_key is None:
            raise SystemExit(f"routed id {item_id} has no CW2-B1 Item key")
        xml_record = sources["items"].get(item_id)
        appearance = sources["appearances"].get(item_id)
        record = build_record(
            family, item_id, item_key, reason, xml_record, appearance, meta
        )
        errors = validate_record(record)
        if errors:
            raise SystemExit(f"id {item_id}: invalid {family} record: {errors[:3]}")
        pointer_errors = validate_routed_item_pointer(
            routed_item_pointer(item_key, family)
        )
        if pointer_errors:
            raise SystemExit(
                f"id {item_id}: invalid routed Item pointer: {pointer_errors}"
            )
        key = record["identity"]["key"]
        if key in keys:
            raise SystemExit(f"family key collision: {key}")
        keys.add(key)
        if on_record is not None:
            on_record(family, record)
        kind = record["kind"].get("value") or "UNKNOWN"
        kinds[f"{family}:{kind}"] += 1
        if kind == "UNKNOWN" and len(unknown_kind_examples[family]) < 5:
            unknown_kind_examples[family].append(key)
        if len(examples[f"{family}:{kind}"]) < 3:
            examples[f"{family}:{kind}"].append(key)
        attrs = dict(xml_record["attrs"]) if xml_record else {}
        type_value = type_outside_family(family, attrs)
        if type_value:
            outside[f"{family}:{type_value}"] += 1
            if len(outside_examples[f"{family}:{type_value}"]) < 3:
                outside_examples[f"{family}:{type_value}"].append(key)
        if appearance is None:
            no_appearance[family] += 1
        for field in unknown_field_names(record):
            unknown_facts[f"{family}:{field}"] += 1
        record_bytes = canonical_bytes(record)
        digest.update(record_bytes + b"\n")
        if records_dir is not None:
            path = Path(records_dir) / family / f"{item_id}.json"
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(record_bytes + b"\n")
    emitted = Counter(kind.split(":", 1)[0] for kind in kinds.elements())
    return {
        "schema": "OTERYN_WORLD_OBJECT_TERRAIN_CENSUS/v1",
        "decision": "WO0-WORLD-OBJECT-TERRAIN-FORMAT-V1 (D93, D94)",
        "scope": (
            "Every id engine_items routes away from Item in this engine's pinned "
            "items.xml; not a placed map object. No identity is minted."
        ),
        "source": {
            **meta,
            "artifact_digests": sources["artifact_digests"],
        },
        "routes": dict(sorted(routes.items())),
        "excluded_routes": dict(sorted(excluded.items())),
        "records": {
            "total": sum(emitted.values()),
            "by_family": dict(sorted(emitted.items())),
            "by_kind": dict(sorted(kinds.items())),
            "examples": {name: keys_ for name, keys_ in sorted(examples.items())},
            "digest_sha256": digest.hexdigest(),
        },
        "unknown_kind_examples": dict(sorted(unknown_kind_examples.items())),
        "type_outside_family": {
            name: {"items": count, "examples": outside_examples[name]}
            for name, count in sorted(outside.items())
        },
        "no_appearance": dict(sorted(no_appearance.items())),
        "unknown_facts": dict(sorted(unknown_facts.items())),
    }


def unknown_field_names(record, prefix=""):
    names = []
    for name, value in record.items():
        if name in ("identity", "provenance") or not isinstance(value, dict):
            continue
        dotted = f"{prefix}{name}"
        if value.get("state") == "UNKNOWN":
            names.append(dotted)
        elif "state" not in value:
            names.extend(unknown_field_names(value, prefix=f"{dotted}."))
    return names


def census_document_bytes(result):
    return (
        json.dumps(result, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    ).encode("utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--source", type=Path, help="pinned Crystal checkout (ff7ede5)")
    parser.add_argument("--out", type=Path, default=DEFAULT_SAMPLE)
    parser.add_argument("--records", type=Path, help="also write every record here")
    parser.add_argument(
        "--check",
        action="store_true",
        help="regenerate in memory and diff against --out; exits 1 on drift",
    )
    parser.add_argument(
        "--validate",
        nargs="+",
        type=Path,
        metavar="RECORD",
        help="validate Terrain/WorldObject record files instead of running the census",
    )
    args = parser.parse_args()

    if args.validate:
        failed = False
        for path in args.validate:
            errors = validate_record(json.loads(path.read_text(encoding="utf-8")))
            for error in errors:
                print(f"{path}: {error}")
            failed = failed or bool(errors)
        raise SystemExit(1 if failed else 0)

    if args.source is None:
        parser.error("--source is required for the census")
    sources = engine_items.load_engine_sources("crystal", args.source)
    result = build_census(sources, records_dir=args.records)
    candidate = census_document_bytes(result)
    if args.check:
        committed = args.out.read_bytes()
        if committed != candidate:
            raise SystemExit(f"census drift against {args.out}")
        print(json.dumps({"check": "PASS", "records": result["records"]["total"]}))
        return
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(candidate)
    print(
        json.dumps(
            {"records": result["records"]["by_family"], "out": str(args.out)},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
