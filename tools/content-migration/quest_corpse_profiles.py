"""Source-qualified corpses needed by active authored deaths and corpse actions.

Existing native Item, temporal decay, corpse MINT and loot-table owners remain
unchanged. Select the finite rule/objective/Creature corpse join, then qualify its
full pinned XML decay closure. No ordinary loot table or canonical hold is changed.
"""

from __future__ import annotations

import copy
import hashlib
import json
from dataclasses import dataclass, field
from pathlib import Path

import grizzly_shop_profiles as grizzly
import quest_reward_item_semantics as custody
from native_profile_composition import encode_native_profile
import quest_shared_boss_corpses
from encounter_corpse_items import RAT_STAGES

ROOT = Path(__file__).resolve().parents[2]
XML = "imports/canary/items-xml/items.xml"
XML_SHA = "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2"
OBJECTIVES = "content/quests/authored-death-objectives.json"
RULES = "content/quests/authored-death-loot.json"
REQUIREMENTS = "tools/content-schema/native-gameplay/quest-corpse-objectives.json"
CREATURES = "content/creatures/definitions/spell-native-profiles.json"
# CORPSE48 (owner 2026-11-17): every corpse holds up to 48 entries, as in Tibia Global;
# crates/gameplay/domain/src/domain/loot_data.rs CORPSE_CONTAINER_ENTRIES_MAX.
CORPSE_CAPACITY_MAX = 48
BUTTERFLY_CORPSES = {4991, 4992, 4993}
BUTTERFLY_REMAINS = 4378
GROUPS = ("physical", "stack", "container", "temporal", "classification")
# The admitted Dwarf Letter bodies share one additional source root, Item 6007.
MAX_RULES, MAX_ROOTS, MAX_CHAIN, MAX_ITEMS = 32, 73, 8, 256
RAT_DEATH_BODY = 5964
# Global Dead Rat documents the movable appearances in order. Animate Dead
# Rune documents the initial ten-second window. The pinned XML binds the IDs
# and edges, and the client appearance flags bind movement and pickup.
# https://tibia.fandom.com/wiki/Dead_Rat
# https://tibia.fandom.com/wiki/Animate_Dead_Rune
RAT_MOVABLE_STAGES = {3994: (300, 6300), 3995: (300, 4400), 3996: (60, 3000)}


def item_id(reference):
    key = reference.get("key", "")
    prefix = "oteryn:item.tibia.i"
    tail = key.removeprefix(prefix)
    if (
        reference.get("family") != "Item"
        or reference.get("revision") != "definition-r1"
        or not key.startswith(prefix)
        or not tail.isascii()
        or not tail.isdecimal()
        or not 1 <= int(tail) <= 65535
    ):
        raise ValueError("QUEST_CORPSE_ITEM_IDENTITY")
    return int(tail)


def required(root=ROOT, creatures=None):
    """Only joined active rule/objective requests admit a corpse, never a name guess."""
    roots, _ = required_with_loot_scope(root, creatures)
    return roots


def required_with_loot_scope(root, creatures):
    rules = json.loads((root / RULES).read_bytes())
    objectives = json.loads((root / OBJECTIVES).read_bytes())
    if rules.get("schema") != 1 or objectives.get("schema") != 1 or len(rules["rules"]) > MAX_RULES:
        raise ValueError("QUEST_CORPSE_RULE_BOUNDS")
    by_objective = {row["key"]: row for row in objectives["objectives"]}
    if len(by_objective) != len(objectives["objectives"]):
        raise ValueError("QUEST_CORPSE_DUPLICATE_OBJECTIVE")
    creatures = creatures or json.loads((root / CREATURES).read_bytes())
    by_creature = {row["profile"]["target"]["key"]: row["profile"] for row in creatures["records"]}
    if len(by_creature) != len(creatures["records"]):
        raise ValueError("QUEST_CORPSE_DUPLICATE_CREATURE")
    ordinary = json.loads((root / REQUIREMENTS).read_bytes())
    if (
        not {"schema", "objectives", "sources"} <= set(ordinary)
        or set(ordinary)
        - {"schema", "objectives", "sources", "capacity_projections", "loot_creatures"}
        or ordinary.get("schema") != 1
        or not isinstance(ordinary["objectives"], list)
        or len(ordinary["objectives"]) > MAX_RULES
        or any(
            not isinstance(key, str) or not key.startswith("oteryn:quest-death/")
            for key in ordinary["objectives"]
        )
        or len(set(ordinary["objectives"])) != len(ordinary["objectives"])
        or not isinstance(ordinary["sources"], list)
        or (ordinary["objectives"] and not ordinary["sources"])
        or any(
            not isinstance(source, str) or not 1 <= len(source) <= 1024
            for source in ordinary["sources"]
        )
    ):
        raise ValueError("QUEST_CORPSE_ORDINARY_REQUIREMENTS")
    roots, seen = set(), set()
    for rule in rules["rules"]:
        key = rule["objective"]
        if key in seen or key not in by_objective:
            raise ValueError("QUEST_CORPSE_RULE_OBJECTIVE_BINDING")
        seen.add(key)
        objective = by_objective[key]
        profile = by_creature.get(objective["creature"])
        if profile is None or profile["target"] != {
            "family": "Creature",
            "key": objective["creature"],
            "revision": "definition-r1",
        }:
            raise ValueError("QUEST_CORPSE_RULE_CREATURE_BINDING")
        corpse = profile["data"]["profile"]["details"].get("corpse_item")
        if corpse is None:
            raise ValueError("QUEST_CORPSE_REQUIRED_SOURCE_ABSENT")
        roots.add(item_id(corpse))
    for key in ordinary["objectives"]:
        if key not in by_objective:
            raise ValueError("QUEST_CORPSE_ORDINARY_OBJECTIVE_BINDING")
        objective = by_objective[key]
        profile = by_creature.get(objective["creature"])
        if profile is None or profile["target"] != {
            "family": "Creature",
            "key": objective["creature"],
            "revision": "definition-r1",
        }:
            raise ValueError("QUEST_CORPSE_ORDINARY_CREATURE_BINDING")
        corpse = profile["data"]["profile"]["details"].get("corpse_item")
        if corpse is None:
            raise ValueError("QUEST_CORPSE_REQUIRED_SOURCE_ABSENT")
        roots.add(item_id(corpse))
    loot_creatures = ordinary.get("loot_creatures", {})
    if (
        not isinstance(loot_creatures, dict)
        or len(loot_creatures) > MAX_ROOTS
        or (loot_creatures and not ordinary["sources"])
    ):
        raise ValueError("QUEST_CORPSE_LOOT_REQUIREMENTS")
    loot_roots = set()
    for key, number in loot_creatures.items():
        profile = by_creature.get(key)
        if profile is None or profile["target"] != {
            "family": "Creature",
            "key": key,
            "revision": "definition-r1",
        }:
            raise ValueError("QUEST_CORPSE_LOOT_CREATURE_BINDING")
        details = profile["data"]["profile"]["details"]
        corpse = details.get("corpse_item")
        if (
            type(number) is not int
            or corpse is None
            or item_id(corpse) != number
            or details["system_eligibility"]["reward_boss"]
        ):
            raise ValueError("QUEST_CORPSE_LOOT_SOURCE_BODY_CHANGED")
        loot_roots.add(number)
    roots.update(quest_shared_boss_corpses.roots(by_creature))
    import questpc7_corpse_requirements

    roots.update(questpc7_corpse_requirements.roots(root, by_creature))
    # Crystal skinning.lua:123-146 selects the rabbit's second decay stage.
    for key in ("oteryn:creature.killer_rabbit", "oteryn:creature.rabbit"):
        profile = by_creature.get(key)
        if profile is None:
            continue
        corpse = profile["data"]["profile"]["details"].get("corpse_item")
        if corpse is None or item_id(corpse) != 6017:
            raise ValueError("QUEST_CORPSE_RABBIT_SOURCE")
        roots.add(6017)
    required_roots = roots | loot_roots
    if len(required_roots) > MAX_ROOTS:
        raise ValueError("QUEST_CORPSE_ROOT_BOUNDS")
    return required_roots, loot_roots - roots


def decay_closure(roots, xml):
    selected = set()
    for root in sorted(roots):
        chain, current = set(), root
        while current:
            if current in chain or len(chain) >= MAX_CHAIN or current not in xml:
                raise ValueError("QUEST_CORPSE_DECAY_CLOSURE")
            chain.add(current)
            node = xml[current]
            attributes = grizzly.source.attrs(node)
            if any(
                attribute.get("key", "").lower() == "script"
                for attribute in node.findall("attribute")
            ) or set(attributes) - {
                "duration",
                "decayto",
                "stopduration",
                "containersize",
                "weight",
                "fluidsource",
                "description",
                "primarytype",
            }:
                raise ValueError("QUEST_CORPSE_UNSUPPORTED_SOURCE_SYSTEM")
            # The butterfly's final remains have no further decay edge.
            if root in BUTTERFLY_CORPSES and current == BUTTERFLY_REMAINS:
                if "duration" in attributes or "decayto" in attributes:
                    raise ValueError("QUEST_CORPSE_BUTTERFLY_REMAINS")
                break
            if (
                "duration" not in attributes
                or "decayto" not in attributes
                or not 1 <= int(attributes["duration"]) <= 366 * 24 * 60 * 60
                or attributes.get("stopduration", "0") not in ("0", "false")
            ):
                raise ValueError("QUEST_CORPSE_TEMPORAL_FACTS")
            if "containersize" in attributes and not 1 <= int(attributes["containersize"]) <= 255:
                raise ValueError("QUEST_CORPSE_SOURCE_CAPACITY")
            current = int(attributes["decayto"])
            if not 0 <= current <= 65535:
                raise ValueError("QUEST_CORPSE_SOURCE_DECAY_TARGET")
        selected.update(chain)
    if len(selected) > MAX_ITEMS:
        raise ValueError("QUEST_CORPSE_CLOSURE_BOUNDS")
    return selected


def definitions(root, selected):
    wanted = {f"oteryn:item.tibia.i{number}" for number in selected}
    found = {}
    for path in sorted((root / "content/items/definitions").glob("items-*.json")):
        for record in json.loads(path.read_bytes())["records"]:
            definition = record["definition"]
            key = definition["identity"]["key"]
            if key in wanted:
                if key in found:
                    raise ValueError("QUEST_CORPSE_DUPLICATE_ITEM")
                found[key] = definition
    if set(found) != wanted:
        raise ValueError("QUEST_CORPSE_CANONICAL_ITEM_MISSING")
    return found


def world_objects(root, selected):
    wanted = {f"oteryn:world-object.tibia.i{number}" for number in selected}
    found = {}
    for path in sorted((root / "content/world/objects").glob("objects-*.json")):
        for record in json.loads(path.read_bytes())["records"]:
            if record["identity"]["key"] in wanted:
                number = int(record["identity"]["key"].rsplit("i", 1)[1])
                if number in found:
                    raise ValueError("QUEST_CORPSE_DUPLICATE_WORLD_OBJECT")
                found[number] = record
    return found


def qualify(
    number,
    definition,
    world,
    node,
    appearance,
    physical,
    bindings,
    fresh,
    capacity_limit=None,
):
    identity = definition["identity"]
    binding = bindings.get(str(number))
    if (
        item_id(identity) != number
        or binding is None
        or binding["disposition"] != "EXACT"
        or binding["target"] != identity
    ):
        raise ValueError("QUEST_CORPSE_EXACT_BINDING")
    flags = appearance["flags"]
    corpse = bool(flags.get("flags.corpse", False))
    plain = number in BUTTERFLY_CORPSES
    player_corpse = bool(flags.get("flags.player_corpse", False))
    # The XML orc decay chain ends in the same loose bones used by player corpses.
    # Keep that appearance's client flag and admit it solely as a terminal decay stage.
    attributes = grizzly.source.attrs(node)
    terminal_bones = (
        number == 4317
        and not fresh
        and not corpse
        and attributes.get("decayto") == "0"
        and "containersize" not in attributes
    )
    if (
        (player_corpse and not terminal_bones)
        or bool(flags.get("flags.cumulative", False))
        or (fresh and (not corpse or (not plain and not flags.get("flags.container", False))))
    ):
        raise ValueError("QUEST_CORPSE_CLIENT_KIND")
    if world is not None:
        if (
            world["kind"] != grizzly.source.K("corpse")
            or world["corpse"]["corpse"] != grizzly.source.K(corpse)
            or world["corpse"]["player_corpse"] != grizzly.source.K(player_corpse)
            or world["provenance"].get("item_pointer") != identity
            or world["client_projection"] != grizzly.source.K({"appearance_id": number})
            or world["movable"] != grizzly.source.K(physical["movable"])
        ):
            raise ValueError("QUEST_CORPSE_WORLD_OBJECT_BINDING")
    else:
        # Some later decay containers are already canonical Items rather than WorldObjects.
        # They cannot authorize a fresh corpse: only this verified root's decay edge admits
        # them, and their accepted container fact must exactly equal their source XML.
        capacity = definition.get("semantics", {}).get("container")
        source_capacity = attributes.get("containersize")
        rabbit_remains = number in (4302, 4303) and source_capacity is None
        if (
            fresh
            or corpse
            or (
                not rabbit_remains
                and (
                    source_capacity is None
                    or capacity
                    != grizzly.source.K({"capacity": grizzly.source.K(int(source_capacity))})
                )
            )
        ):
            raise ValueError("QUEST_CORPSE_DECAY_ITEM_OWNER")
    source = grizzly.source.profile(number, node, appearance, physical, bindings, None)
    # The shared Ground-only corpse owner takes decay-stage capacity from XML.
    # A missing capacity is non-container; the generic DAT container default of
    # eight slots must not replace that accepted terminal-body policy.
    if (not fresh or plain) and "containersize" not in grizzly.source.attrs(node):
        source["semantics"]["container"] = copy.deepcopy(grizzly.source.NA)
    expected = source["semantics"]
    if terminal_bones:
        # The shared bones already have an accepted weight. This admission supplies decay.
        canonical_weight = definition["semantics"]["physical"]["value"]["weight"]
        source_weight = expected["physical"]["value"]["weight"]
        if canonical_weight != grizzly.source.K(10000) or source_weight != grizzly.source.K(60000):
            raise ValueError("QUEST_CORPSE_TERMINAL_BONES_WEIGHT")
        expected["physical"]["value"]["weight"] = copy.deepcopy(canonical_weight)
    # The Global Dead Rat archive gives the visible stages' weight and
    # container facts. DAT still supplies pickupability and movement.
    if number in RAT_STAGES:
        capacity, _, _, weight = RAT_STAGES[number]
        expected['physical']['value']['weight'] = grizzly.source.K(weight)
        expected['container'] = (grizzly.source.NA if capacity is None else
                                 grizzly.source.K({'capacity': grizzly.source.K(capacity)}))
    for group in GROUPS:
        accepted = definition.get("semantics", {}).get(group, grizzly.source.U)
        if grizzly.known_conflict(accepted, expected[group]):
            raise ValueError("QUEST_CORPSE_CANONICAL_" + group.upper())
    if definition["stack_class"] not in ("Unknown", source["admission"]["stack_class"]):
        raise ValueError("QUEST_CORPSE_CANONICAL_STACK_CLASS")
    source["semantics"] = {
        group: grizzly.accepted_first(
            definition.get("semantics", {}).get(group, grizzly.source.U), value
        )
        if group in GROUPS
        else copy.deepcopy(definition.get("semantics", {}).get(group, grizzly.source.U))
        for group, value in expected.items()
    }
    result = grizzly.source.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [source]},
        bindings,
        {number},
    )["records"][0]
    if (
        fresh
        and not plain
        and (
            result["semantics"]["container"]["state"] != "KNOWN"
            or "containersize" not in grizzly.source.attrs(node)
        )
    ):
        raise ValueError("QUEST_CORPSE_FRESH_CONTAINER_CAPACITY")
    if plain or number == BUTTERFLY_REMAINS:
        if result["semantics"]["container"] != grizzly.source.NA:
            raise ValueError("QUEST_CORPSE_BUTTERFLY_CONTAINER")
        result["admission"]["legal_destinations"] = ["Ground"]
    if capacity_limit is not None:
        capacity = result["semantics"]["container"]["value"]["capacity"]["value"]
        if (
            not fresh
            or capacity_limit != CORPSE_CAPACITY_MAX
            or not CORPSE_CAPACITY_MAX < capacity <= 65535
        ):
            raise ValueError("QUEST_CORPSE_NATIVE_CAPACITY_PROJECTION")
        result["semantics"]["container"]["value"]["capacity"] = grizzly.source.K(capacity_limit)
    return result


@dataclass
class Qualification:
    roots: set[int]
    records: dict[str, dict]
    projected_source_capacities: dict[str, int] = field(default_factory=dict)
    ordinary_loot_only: set[int] = field(default_factory=set)


def projection_limits(root, roots, xml):
    """Explicit source-to-native projections apply only above the accepted48-entry corpse bound."""
    records = json.loads((root / REQUIREMENTS).read_bytes()).get("capacity_projections", [])
    if not isinstance(records, list) or len(records) > MAX_RULES:
        raise ValueError("QUEST_CORPSE_CAPACITY_PROJECTION_BOUNDS")
    limits, source_capacities = {}, {}
    for record in records:
        if set(record) != {"item", "source_capacity", "native_capacity", "sources"}:
            raise ValueError("QUEST_CORPSE_CAPACITY_PROJECTION_SHAPE")
        number = item_id({"family": "Item", "key": record["item"], "revision": "definition-r1"})
        capacity = grizzly.source.attrs(xml[number]).get("containersize") if number in xml else None
        if (
            number not in roots
            or number in limits
            or record["native_capacity"] != CORPSE_CAPACITY_MAX
            or type(record["source_capacity"]) is not int
            or not CORPSE_CAPACITY_MAX < record["source_capacity"] <= 65535
            or capacity != str(record["source_capacity"])
            or not record["sources"]
            or any(not isinstance(s, str) or not s for s in record["sources"])
        ):
            raise ValueError("QUEST_CORPSE_CAPACITY_PROJECTION_BINDING")
        limits[number] = CORPSE_CAPACITY_MAX
        source_capacities[record["item"]] = record["source_capacity"]
    return limits, source_capacities


def rat_stage_inputs(number, node, definition):
    """Project Global timings and physical facts through the pinned corpse chain."""
    node = copy.deepcopy(node)
    fields = {a.get('key').lower(): a for a in node.findall('attribute')}
    if number == RAT_DEATH_BODY:
        # Keep the existing death-loot capacity. Its original lifetime was a
        # single 60-second retirement with no decay successor.
        definition = copy.deepcopy(definition)
        semantics = definition['semantics']
        capacity = semantics['container']['value']['capacity']['value']
        fields['containersize'].set('value', str(capacity))
        fields['duration'].set('value', '10')
        semantics['temporal'] = grizzly.source.U
    else:
        duration, weight = RAT_MOVABLE_STAGES[number]
        fields['duration'].set('value', str(duration))
        fields['weight'].set('value', str(weight))
        if number == 3994:
            fields['containersize'].set('value', '5')
        elif number == 3995:
            # Global gives the container volume for 3994 only. This stage
            # has no container appearance flag, so discard the XML capacity.
            node.remove(fields['containersize'])
    return node, definition


def qualifications(root=ROOT, creatures=None, *, include_rat=False):
    roots, ordinary_loot_roots = required_with_loot_scope(root, creatures)
    raw = (root / XML).read_bytes()
    if hashlib.sha256(raw).hexdigest() != XML_SHA:
        raise ValueError("QUEST_CORPSE_XML_PIN")
    xml = grizzly.source.xml_index(raw)
    if 50029 in roots:
        quest_shared_boss_corpses.apply_arbaziloth_corpse_facts(xml)
    limits, source_capacities = projection_limits(root, roots, xml)
    selected = decay_closure(roots, xml)
    ordinary_loot_only = selected - decay_closure(roots - ordinary_loot_roots, xml)
    if include_rat:
        rat = next(r['profile'] for r in creatures['records']
                   if r['profile']['target']['key'] == 'oteryn:creature.rat')
        if (item_id(rat['data']['profile']['details']['corpse_item']) != RAT_DEATH_BODY
                or decay_closure({RAT_DEATH_BODY}, xml) != {RAT_DEATH_BODY, *RAT_MOVABLE_STAGES}):
            raise ValueError('ANIMATE_DEAD_RAT_CORPSE_CHAIN')
        roots.add(RAT_DEATH_BODY)
        selected.update({RAT_DEATH_BODY, *RAT_MOVABLE_STAGES})
    if not selected:
        return Qualification(roots, {})
    client = (root / custody.CLIENT).read_bytes()
    if (
        len(client) != custody.CLIENT_BYTES
        or hashlib.sha256(client).hexdigest() != custody.CLIENT_SHA
    ):
        raise ValueError("QUEST_CORPSE_CLIENT_PIN")
    appearances = grizzly.source.engine_items.load_appearance_objects(client)
    policies = {
        row["server_item_id"]: row for row in grizzly.source.policies(client, raw, selected)
    }
    accepted = definitions(root, selected)
    objects = world_objects(root, selected)
    bindings = grizzly.source.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    records = {}
    for number in sorted(selected):
        key = f"oteryn:item.tibia.i{number}"
        if number not in appearances or number not in policies:
            raise ValueError("QUEST_CORPSE_CLIENT_ITEM_ABSENT")
        node, definition = xml[number], accepted[key]
        if number == RAT_DEATH_BODY or number in RAT_MOVABLE_STAGES:
            node, definition = rat_stage_inputs(number, node, definition)
        records[key] = qualify(
            number,
            definition,
            objects.get(number),
            node,
            appearances[number],
            policies[number],
            bindings,
            number in roots,
            limits.get(number),
        )
    return Qualification(roots, records, source_capacities, ordinary_loot_only)


def admit(root, generated, profile_path, manifest_path, order, canonical):
    creatures = json.loads(
        generated[CREATURES] if CREATURES in generated else (root / CREATURES).read_bytes()
    )
    qualified = qualifications(root, creatures, include_rat=True)
    document = json.loads(generated[profile_path])
    existing = {row["production_definition"]["production_key"]: row for row in document["records"]}
    if len(existing) != len(document["records"]):
        raise ValueError("QUEST_CORPSE_DUPLICATE_NATIVE_PROFILE")
    for key, record in qualified.records.items():
        if key in existing:
            prior = existing[key]
            if (
                prior["production_definition"] != record["production_definition"]
                or prior["production_binding"] != record["production_binding"]
                or prior["admission"] != record["admission"]
                or any(
                    prior["semantics"][group] != record["semantics"][group]
                    for group in GROUPS
                    if group != "classification"
                )
                or grizzly.known_conflict(
                    prior["semantics"]["classification"],
                    record["semantics"]["classification"],
                )
            ):
                raise ValueError("QUEST_CORPSE_EXISTING_NATIVE_CONFLICT")
            # Complete only UNKNOWN classification facts from this exact pinned
            # XML/DAT corpse closure. All other existing policy/attributes remain.
            prior["semantics"]["classification"] = grizzly.accepted_first(
                prior["semantics"]["classification"],
                record["semantics"]["classification"],
            )
            continue
        document["records"].append(record)
    document["records"].sort(key=order)
    generated[profile_path] = encode_native_profile(document)
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(generated[profile_path]).hexdigest()
    generated[manifest_path] = canonical(manifest)


def apply_loot_facts(facts, root=ROOT, creatures=None):
    """Called by the real loot generator before it selects named Item facts."""
    updates = []
    qualified = qualifications(root, creatures)
    for key, record in qualified.records.items():
        production = record["production_definition"]
        identity = {
            "family": production["family"],
            "key": production["production_key"],
            "revision": production["revision_ref"],
        }
        fact = facts.get(key)
        if fact is None or fact["item"] != identity:
            raise ValueError("QUEST_CORPSE_LOOT_ITEM_IDENTITY")
        container = record["semantics"]["container"]
        capacity = (
            container["value"]["capacity"]["value"] if container["state"] == "KNOWN" else None
        )
        if (
            fact["materializable"]
            and fact["container_capacity"] != capacity
            or fact["container_capacity"] is not None
            and fact["container_capacity"] != capacity
            and (
                fact["materializable"]
                or fact["container_capacity"] != qualified.projected_source_capacities.get(key)
            )
        ):
            raise ValueError("QUEST_CORPSE_LOOT_CAPACITY_CONFLICT")
        ordinary_only = (
            not fact["materializable"] and item_id(identity) in qualified.ordinary_loot_only
        )
        updates.append((fact, capacity, ordinary_only))
    for fact, capacity, ordinary_only in updates:
        fact["materializable"] = True
        fact["container_capacity"] = capacity
        if ordinary_only:
            fact["ordinary_corpse_only"] = True
