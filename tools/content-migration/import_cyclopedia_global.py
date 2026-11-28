#!/usr/bin/env python3
"""Import captured Global Item facts and NPC shop prices into the content tree.

Weights are hundredths of an ounce. Alias records resolve through the capture's
explicit details target; constructor declarations follow the same client facts.
"""

import argparse
import hashlib
import json
import re
import subprocess
import unicodedata
from collections import Counter, defaultdict
from decimal import Decimal
from pathlib import Path


def load_capture(root):
    """Verify the capture and give every alias its target's detail record."""
    capture = root / "docs/reference/content-prep/item-cyclopedia-capture-1"
    raw = (capture / "items.jsonl").read_bytes()
    manifest = json.loads((capture / "manifest.json").read_bytes())
    if hashlib.sha256(raw).hexdigest() != manifest["items_jsonl_sha256"]:
        raise ValueError("Cyclopedia capture digest mismatch")
    records = [json.loads(line) for line in raw.splitlines()]
    rows = {row["id"]: row for row in records}
    if len(rows) != len(records) or len(rows) != manifest["records"]:
        raise ValueError("Cyclopedia record count or duplicate ID")
    corrections = json.loads((capture / "weight-corrections.json").read_bytes())
    for correction in corrections:
        row = rows[correction["id"]]
        if row["basic"]["Weight"] != correction["captured_weight"]:
            raise ValueError("Stale Cyclopedia weight correction")
        row["basic"]["Weight"] = correction["global_weight"]
    resolved = {}
    for identifier, row in rows.items():
        if row["compatibility"]["target"] != "tibia-global":
            raise ValueError("Unexpected compatibility target")
        seen = set()
        while "details_from_alias_id" in row["compatibility"]:
            target = row["compatibility"]["details_from_alias_id"]
            if target in seen:
                raise ValueError("Cyclopedia alias cycle")
            seen.add(target)
            row = rows[target]
        resolved[identifier] = row
    return resolved


def load_weights(root):
    return capture_weights(load_capture(root))


def capture_weights(rows):
    weights = {}
    for identifier, row in rows.items():
        value = Decimal(row["basic"]["Weight"].removesuffix(" oz")) * 100
        if value != int(value) or not 0 <= value <= 0xFFFFFFFF:
            raise ValueError("Unrepresentable weight")
        weights[identifier] = int(value)
    return weights


def apply_weight(definition, weights):
    """Set a KNOWN physical weight to the client value; return True when it changed."""
    identity = definition.get(
        "identity", definition.get("authoring", {}).get("item", {})
    )
    key = identity.get("key", "")
    prefix = (
        "oteryn:item.tibia.i"
        if key.startswith("oteryn:item.tibia.i")
        else "candidate:item/"
    )
    if not key.startswith(prefix) or not key[len(prefix) :].isdigit():
        return False
    identifier = int(key[len(prefix) :])
    physical = definition.get("semantics", {}).get("physical", {})
    if identifier not in weights or physical.get("state") != "KNOWN":
        return False
    desired = {"state": "KNOWN", "value": weights[identifier]}
    if physical["value"].get("weight") == desired:
        return False
    physical["value"]["weight"] = desired
    return True


def only_weight_changed(before, after):
    """True when after differs from before at most in a KNOWN physical weight leaf."""
    try:
        weight = after["semantics"]["physical"]["value"]["weight"]
        if (
            after["semantics"]["physical"]["state"] != "KNOWN"
            or weight.get("state") != "KNOWN"
            or type(weight.get("value")) is not int
            or not 0 <= weight["value"] <= 0xFFFFFFFF
            or before["semantics"]["physical"]["state"] != "KNOWN"
        ):
            return False
        expected = json.loads(json.dumps(before))
        expected["semantics"]["physical"]["value"]["weight"] = weight
        return expected == after
    except (KeyError, TypeError, AttributeError):
        return False


OVERLAY_DIR = "tools/content-schema/native-gameplay"
WEIGHT_PATTERN = re.compile(
    rb'("weight":\s*\{"state":\s*"KNOWN",\s*"value":\s*)(\d+)(\})'
)


def _overlay_profiles(node):
    """Yield (item id, weight leaf) for every overlay record carrying a KNOWN physical weight."""
    if isinstance(node, dict):
        key = (node.get("production_definition") or {}).get("production_key")
        physical = (node.get("semantics") or {}).get("physical")
        if (
            isinstance(key, str)
            and key.startswith("oteryn:item.tibia.i")
            and isinstance(physical, dict)
            and physical.get("state") == "KNOWN"
            and (physical.get("value") or {}).get("weight", {}).get("state") == "KNOWN"
        ):
            yield int(key.rsplit(".i", 1)[1]), physical["value"]["weight"]
        for value in node.values():
            yield from _overlay_profiles(value)
    elif isinstance(node, list):
        for value in node:
            yield from _overlay_profiles(value)


def apply_profile_weights(records, root):
    """Give profile records read from a faithful source capture the client weight."""
    weights = load_weights(root)
    for identifier, weight in _overlay_profiles(records):
        weight["value"] = weights.get(identifier, weight["value"])


def apply_overlay_weights(root, weights):
    """Refresh generated source overlays; donor captures are qualified by their consumers.

    Each overlay keeps its own byte layout: a record's weight is replaced on the
    line that names its key, and the parsed result must equal the expected edit."""
    changed = {}
    for path in sorted((root / OVERLAY_DIR).glob("*.json")):
        # These producers emit Global weights. Other files retain raw donor
        # facts and apply Global weights in memory when their owner admits them.
        if path.name not in ("coin-item-profiles.json", "event-item-profiles.json"):
            continue
        raw = path.read_bytes()
        document = json.loads(raw)
        edits = {}
        for identifier, weight in _overlay_profiles(document):
            if identifier in weights and weights[identifier] != weight["value"]:
                edits.setdefault(identifier, set()).add(weight["value"])
                weight["value"] = weights[identifier]
        if not edits:
            continue
        if (
            raw
            == (
                json.dumps(json.loads(raw), indent=2, ensure_ascii=False) + "\n"
            ).encode()
        ):
            updated = (
                json.dumps(document, indent=2, ensure_ascii=False) + "\n"
            ).encode()
        else:
            lines = raw.split(b"\n")
            for identifier, previous in edits.items():
                needle = f'"oteryn:item.tibia.i{identifier}"'.encode()
                weight = str(weights[identifier]).encode()
                hits = [n for n, line in enumerate(lines) if needle in line]
                for n in hits:
                    lines[n] = WEIGHT_PATTERN.sub(
                        lambda m, weight=weight, previous=previous: (
                            m.group(1) + weight + m.group(3)
                            if int(m.group(2)) in previous
                            else m.group(0)
                        ),
                        lines[n],
                    )
            updated = b"\n".join(lines)
        if json.loads(updated) != document:
            raise ValueError("CLIENT_WEIGHT_OVERLAY_LAYOUT:" + path.name)
        path.write_bytes(updated)
        changed[path.name] = len(edits)
    return changed


def refresh_overlay_weights(root, weights):
    """Replay weight admissions while retaining the verified donor profile closure."""
    import compose_monster_current_sources as composition
    import import_spell_families as spell_import
    import register_spell_families as registration

    generated = spell_import.baseline_outputs(root)
    baseline = dict(generated)
    generated = registration.current_sources.outputs(root, generated)
    generated = composition.compose(root, baseline, generated)
    # Validate the existing closure before changing its weight leaves or digest.
    registration.admit_overlay_profiles(root, generated)
    provider = json.loads(
        generated[spell_import.PROVIDER_DESTINATIONS["item_profiles"]]
    )
    for identifier, weight in _overlay_profiles(provider):
        weight["value"] = weights.get(identifier, weight["value"])
    raw = (json.dumps(provider, indent=2, ensure_ascii=False) + "\n").encode()
    changed = apply_overlay_weights(root, weights)
    for relative in (registration.COIN_PROFILES, registration.DOOR_PROFILES):
        path = root / relative
        document = json.loads(path.read_bytes())
        document["item_profiles_sha256"] = hashlib.sha256(raw).hexdigest()
        path.write_text(json.dumps(document, indent=2, ensure_ascii=False) + "\n")
    return changed


ITEM_PREFIX = "oteryn:item.tibia.i"
UNKNOWN = {"state": "UNKNOWN"}
SLOTS = dict(
    zip(
        (
            "head",
            "body",
            "legs",
            "feet",
            "weapon hand",
            "shield hand",
            "neck",
            "finger",
            "container",
            "extra slot",
            "both hands",
        ),
        (
            "HEAD",
            "TORSO",
            "LEGS",
            "FEET",
            "WEAPON",
            "SHIELD",
            "AMULET",
            "RING",
            "CONTAINER",
            "EXTRA",
            "WEAPON",
        ),
    )
)
VOCATIONS = dict(
    zip(
        (
            "druids",
            "knights",
            "monks",
            "paladins",
            "sorcerers",
            "players without vocation",
        ),
        ("DRUID", "KNIGHT", "MONK", "PALADIN", "SORCERER", "NONE"),
    )
)
# Rust enum order is required by ReferenceItemSemantics validation.
RESISTANCES = dict(
    zip(
        (
            "death",
            "drowning",
            "earth",
            "energy",
            "fire",
            "holy",
            "ice",
            "life drain",
            "mana drain",
            "physical",
            "poison",
            "fire field",
        ),
        (
            "DEATH",
            "DROWN",
            "EARTH",
            "ENERGY",
            "FIRE",
            "HOLY",
            "ICE",
            "LIFE_DRAIN",
            "MANA_DRAIN",
            "PHYSICAL",
            "POISON",
            "FIRE_FIELD",
        ),
    )
)
SKILL_KINDS = (
    "ELEMENTAL_BOND",
    "INVISIBILITY",
    "MANA_SHIELD",
    "MANTRA",
    "CLEAVE_PERCENT",
    "CRITICAL_HIT_CHANCE",
    "CRITICAL_HIT_DAMAGE",
    "DEATH_MAGIC_LEVEL_POINTS",
    "EARTH_MAGIC_LEVEL_POINTS",
    "ENERGY_MAGIC_LEVEL_POINTS",
    "FIRE_MAGIC_LEVEL_POINTS",
    "HEALING_MAGIC_LEVEL_POINTS",
    "HEALTH_GAIN",
    "HEALTH_TICKS",
    "HOLY_MAGIC_LEVEL_POINTS",
    "ICE_MAGIC_LEVEL_POINTS",
    "LIFE_LEECH_AMOUNT",
    "LIFE_LEECH_CHANCE",
    "MAGIC_LEVEL_POINTS",
    "MAGIC_SHIELD_CAPACITY_FLAT",
    "MAGIC_SHIELD_CAPACITY_PERCENT",
    "MANA_GAIN",
    "MANA_LEECH_AMOUNT",
    "MANA_LEECH_CHANCE",
    "MANA_TICKS",
    "PERFECT_SHOT_DAMAGE",
    "PERFECT_SHOT_RANGE",
    "REFLECT_DAMAGE",
    "SKILL_AXE",
    "SKILL_CLUB",
    "SKILL_DISTANCE",
    "SKILL_FIST",
    "SKILL_SHIELD",
    "SKILL_SWORD",
    "SPEED",
    "SUPPRESS_DROWN",
    "SUPPRESS_DRUNK",
    "REFLECT_PERCENT_ALL",
)
POINT_BOOSTS = {
    **{
        f"{skill} fighting": f"SKILL_{skill.upper()}"
        for skill in ("axe", "club", "distance", "fist", "sword")
    },
    **{
        label: f"{label.upper().replace(' ', '_')}_POINTS"
        for label in (
            "magic level",
            *(
                f"{element} magic level"
                for element in (
                    "death",
                    "earth",
                    "energy",
                    "fire",
                    "healing",
                    "holy",
                    "ice",
                )
            ),
        )
    },
    "shielding": "SKILL_SHIELD",
    "speed": "SPEED",
}
PERCENT_BOOSTS = {
    "critical hit chance": "CRITICAL_HIT_CHANCE",
    "critical extra damage": "CRITICAL_HIT_DAMAGE",
    "life leech": "LIFE_LEECH_AMOUNT",
    "mana leech": "MANA_LEECH_AMOUNT",
}


def item_id(identity):
    key = identity["key"].removeprefix(ITEM_PREFIX)
    return int(key) if key.isdigit() else None


def known(value):
    return {"state": "KNOWN", "value": value}


def group(semantics, name, fields):
    existing = semantics.get(name, {})
    if existing.get("state") != "KNOWN":
        existing = semantics[name] = known(dict.fromkeys(fields.split(), UNKNOWN))
    return existing["value"]


def percent(value):
    return {"numerator": int(value), "denominator": 1}


def attack_fields(text):
    if match := re.fullmatch(
        r"(\d+)(?: physical \+(\d+))?(?: (death|earth|energy|fire|ice))?", text
    ):
        first, extra, element = match.groups()
        attack = int(first) if extra or not element else 0
        # The typed weapon group stores the physical and elemental portions separately.
        result = {"attack": attack}
        if element:
            result["elemental"] = [
                {"element": element.upper(), "points": known(int(extra or first))}
            ]
        return result
    launcher_fields(text)
    return {}


def launcher_fields(text):
    """Decode the client launcher line as flat attack, hit bonus and range."""
    match = re.fullmatch(
        r"(?:attack ([+-]\d+), )?(?:chance to hit ([+-]\d+)%, )?(\d+) fields", text
    )
    if match is None:
        raise ValueError(f"Unrecognized Cyclopedia Attack: {text}")
    bonus, hit, distance = match.groups()
    return {
        "attack": int(bonus or 0),
        "hit_chance": int(hit or 0),
        "range_cells": int(distance),
    }


def skill_boosts(text, existing, counts):
    by_kind = {row["kind"]: row for row in existing}
    captured_kinds = (
        set(POINT_BOOSTS.values()) | set(PERCENT_BOOSTS.values()) | {"SUPPRESS_DRUNK"}
    )
    result = {kind: row for kind, row in by_kind.items() if kind not in captured_kinds}
    for entry in text.split(", "):
        if entry == "faster regeneration":
            counts["unquantified_regeneration"] += 1
            continue
        if entry == "hard drinking":
            kind, parameter = "SUPPRESS_DRUNK", {"kind": "BOOLEAN", "value": True}
        else:
            match = re.fullmatch(r"(.+) ([+-]?\d+)(%?)", entry)
            if match is None:
                raise ValueError(f"Unrecognized Cyclopedia Skill Boost: {entry}")
            label, value, suffix = match.groups()
            kind, parameter = (
                (
                    PERCENT_BOOSTS[label],
                    {"kind": "RATIONAL_PERCENT", "value": percent(value)},
                )
                if suffix
                else (
                    POINT_BOOSTS[label],
                    {"kind": "SIGNED_POINTS", "value": int(value)},
                )
            )
        result[kind] = {
            **by_kind.get(
                kind,
                dict.fromkeys(
                    ("target_domain", "evaluation_phase", "priority"), UNKNOWN
                ),
            ),
            "kind": kind,
            "parameter": known(parameter),
        }
    return sorted(result.values(), key=lambda row: SKILL_KINDS.index(row["kind"]))


def apply_stats(definition, basic, counts):
    semantics = definition.get("semantics")
    if semantics is None:
        counts["matched_items_without_semantics"] += 1
        return

    def set_field(destination, field, value, name):
        desired = known(value)
        counts[
            name + ("_matched" if destination.get(field) == desired else "_changed")
        ] += 1
        destination[field] = desired

    if "Attack" in basic or "Defence" in basic:
        fields = attack_fields(basic["Attack"]) if "Attack" in basic else {}
        if "Defence" in basic:
            defense = basic["Defence"].split()
            fields["defense"] = int(defense[0])
            if len(defense) == 2:
                fields["extra_defense"] = int(defense[1])
        if fields:
            weapon = group(
                semantics,
                "weapon",
                "weapon_type attack defense extra_defense range hit_chance max_hit_chance ammunition elemental",
            )
            for field, value in fields.items():
                set_field(weapon, field, value, field)
            # A displayed plain value has no secondary attack or defence component.
            for field, present in (
                ("elemental", "attack"),
                ("extra_defense", "defense"),
            ):
                if (
                    present in fields
                    and field not in fields
                    and weapon[field].get("state") == "KNOWN"
                    and weapon[field]["value"]
                ):
                    set_field(weapon, field, [] if field == "elemental" else 0, field)

    if "Armor" in basic or "Protection" in basic:
        protection = group(semantics, "protection", "armor resistances")
        if "Armor" in basic:
            set_field(protection, "armor", int(basic["Armor"]), "armor")
        if "Protection" in basic:
            entries = [part.rsplit(" ", 1) for part in basic["Protection"].split(", ")]
            entries.sort(key=lambda pair: tuple(RESISTANCES).index(pair[0]))
            set_field(
                protection,
                "resistances",
                [
                    {
                        "kind": RESISTANCES[label],
                        "percent": known(percent(value.removesuffix("%"))),
                    }
                    for label, value in entries
                ],
                "protections",
            )

    if "Imbuement Slots" in basic:
        imbuement = group(
            semantics, "imbuement", "slot_count allowed_family_tiers excluded_families"
        )
        set_field(
            imbuement, "slot_count", int(basic["Imbuement Slots"]), "imbuement_slots"
        )

    if "Skill Boost" in basic:
        previous = (
            semantics.get("skill_modifiers", {})
            .get("value", {})
            .get("modifiers", {})
            .get("value", [])
        )
        boosts = skill_boosts(basic["Skill Boost"], previous, counts)
        if boosts or previous:
            set_field(
                group(semantics, "skill_modifiers", "modifiers"),
                "modifiers",
                boosts,
                "skill_boosts",
            )
            # Native profiles retain a raw speed attribute alongside the typed boost.
            if (attributes := definition.get("attributes")) is not None:
                speed = next(
                    (
                        row["parameter"]["value"]["value"]
                        for row in boosts
                        if row["kind"] == "SPEED"
                    ),
                    None,
                )
                if speed is not None or any(row["kind"] == "SPEED" for row in previous):
                    attributes["speed_bonus"] = speed if speed is not None else 0

    requirements = {}
    if "Vocations" in basic:
        vocations = {VOCATIONS[name] for name in basic["Vocations"].split(", ")}
        requirements["vocations"] = [
            name for name in VOCATIONS.values() if name in vocations
        ]
    if "Required Level" in basic:
        requirements["level"] = int(basic["Required Level"])
    if not requirements:
        return

    position = basic.get("Body Position")
    if position in (None, "extra slot"):
        use = group(
            semantics, "use_requirements", "min_level min_magic_level vocations"
        )
        use["enforcement_mode"] = "ON_USE"
        for name, value in requirements.items():
            set_field(use, "min_level" if name == "level" else name, value, name)
        return

    equipment = group(semantics, "equipment", "patterns")
    patterns = equipment["patterns"].get("value")
    if not patterns:
        patterns = [
            {
                "pattern_id": 1,
                "primary_slot": known(SLOTS[position]),
                "additional_reserved_slots": known(
                    ["SHIELD"] if position == "both hands" else []
                ),
                "mutually_exclusive_groups": UNKNOWN,
                "compatibility_rule": UNKNOWN,
                "level": UNKNOWN,
                "vocations": UNKNOWN,
            }
        ]
        equipment["patterns"] = known(patterns)
    for pattern in patterns:
        for name, value in requirements.items():
            set_field(pattern, name, value, name)


# Preserve native WorldProject wire field order in the declaration document.
OFFER_FIELDS = (
    "item",
    "direction",
    "unit_price",
    "currency",
    "count",
    "sub_type",
    "parity_pending",
)
TRADE_PREFIX = "oteryn:service.trade."
TRADE_DIRECTIONS = {"npc_buy_from": "SellToPlayer", "npc_sell_to": "BuyFromPlayer"}
FLUID_CONTAINERS = {2874, 2875, 2877, 2880, 2901, 5552, 9232}
EXERCISE_WRAPS = {50293, 50294, 50295}


def qualified_wraps(service, rows):
    counts = {}
    for field in service.get("fields", []):
        if not field["field_path"].startswith("oteryn:source.npc.held_offer_"):
            continue
        hold = json.loads(field["value"]["value"])
        offer = hold.get("old_native_offer", {})
        if not offer:
            continue
        identifier = item_id(offer["item"])
        if (
            identifier in EXERCISE_WRAPS
            and identifier in rows
            and hold.get("reason") == "VARIANT_SAME_TUPLE_TWO_WIKI_NOT_PROVEN"
            and hold.get("quote_item_variant_qualification")
            == "CURRENT_NATIVE_ITEM_KNOWN_CHARGES_EXACT_AND_TWO_WIKI_PRICE_CHARGE_COMPONENTS"
            and hold.get("service") == service["identity"]["key"]
            and offer.get("sub_type") == int(rows[identifier]["basic"]["Total Charges"])
        ):
            counts[identifier, offer["direction"]] = offer["sub_type"]
    return counts


def npc_name(name):
    # Canonical NPC slugs differ in punctuation, spacing and case from the client labels.
    return "".join(
        char
        for char in unicodedata.normalize("NFKD", name.casefold())
        if char.isascii() and char.isalnum()
    )


def trade_identity(offer):
    return (
        offer["item"]["key"],
        offer["item"]["revision"],
        offer["direction"],
        offer["unit_price"],
        (offer.get("currency") or {}).get("key", ""),
        offer.get("count", 0),
        offer.get("sub_type", -1),
        offer.get("parity_pending", False),
    )


def apply_trades(declarations, rows, identities, counts):
    services = {
        d["identity"]["key"]: d
        for d in declarations
        if d["identity"]["key"].startswith(TRADE_PREFIX) and d.get("offers")
    }
    shops = defaultdict(set)
    for npc in declarations:
        if npc["kind"] == "NPC":
            name = npc_name(npc["identity"]["key"].removeprefix("oteryn:npc."))
            shops[name].update(
                ref["key"] for ref in npc.get("services", []) if ref["key"] in services
            )

    captures = defaultdict(lambda: defaultdict(Counter))
    for identifier, row in rows.items():
        for capture_field, direction in TRADE_DIRECTIONS.items():
            for offer in row.get(capture_field, []):
                currency = offer.get("currency_object_type_id")
                candidates = shops[npc_name(offer["npc"])]
                if "currency" in offer and currency is None:
                    counts["unmodeled_named_currency"] += 1
                elif identifier not in identities or (
                    currency is not None and currency not in identities
                ):
                    counts["trade_item_or_currency_missing"] += 1
                elif len(candidates) != 1:
                    counts["trade_npc_without_unique_active_service"] += 1
                else:
                    shop = next(iter(candidates))
                    captures[shop][identifier, direction][currency, offer["price"]] += 1

    for shop, service in services.items():
        captured_offers = captures[shop]
        wrap_subtypes = qualified_wraps(service, rows)
        by_row = defaultdict(list)
        for offer in service["offers"]:
            by_row[item_id(offer["item"]), offer["direction"]].append(offer)
        changed = False
        for (identifier, direction), captured in captured_offers.items():
            if identifier in FLUID_CONTAINERS:
                # The client trade rows supply Item identity and price without
                # variant evidence. Keep the independently qualified offers.
                counts["trade_fluid_quotes_without_subtype"] += sum(captured.values())
                continue
            wrap_subtype = wrap_subtypes.get((identifier, direction))
            if identifier in EXERCISE_WRAPS and wrap_subtype is None:
                continue
            by_currency = defaultdict(list)
            for (currency, price), occurrences in captured.items():
                by_currency[currency].append((price, occurrences))
            current = by_row[identifier, direction]
            if wrap_subtype is not None:
                current = [
                    offer
                    for offer in current
                    if offer.get("sub_type", offer.get("count")) in (None, wrap_subtype)
                ]
            for currency, prices in by_currency.items():
                if len(prices) != 1:
                    counts["trade_conflicting_location_prices"] += sum(
                        n for _, n in prices
                    )
                    continue
                price, occurrences = prices[0]
                ref = identities.get(currency)
                matching = [offer for offer in current if offer.get("currency") == ref]
                # One existing row and one unambiguous capture can also correct the currency.
                if not matching and len(current) == len(by_currency) == 1:
                    matching = current
                if not matching:
                    matching = [
                        {"item": identities[identifier], "direction": direction}
                    ]
                    service["offers"].extend(matching)
                    counts["trade_offers_added"] += 1
                for offer in matching:
                    before = dict(offer)
                    offer["unit_price"] = price
                    if wrap_subtype is not None:
                        offer.update(count=1, sub_type=wrap_subtype)
                    offer.pop("parity_pending", None)
                    if ref:
                        offer["currency"] = ref
                    else:
                        offer.pop("currency", None)
                    if before == offer:
                        counts["trade_offers_matched"] += 1
                    else:
                        changed = True
                        counts["trade_offers_changed"] += int("unit_price" in before)
                counts["trade_capture_rows_matched_to_service"] += occurrences
                if currency is not None:
                    counts["trade_item_currency_capture_rows"] += occurrences
        if changed:
            service["offers"] = [
                {field: offer[field] for field in OFFER_FIELDS if field in offer}
                for offer in sorted(service["offers"], key=trade_identity)
            ]


def forge_tiers(root):
    from sys import path as module_path

    module_path.insert(0, str(root / "tools/content-schema/item-authoring"))
    try:
        from source_field_catalogs import FORGE_CLASSIFICATION_MAX_TIER
    finally:
        module_path.pop(0)
    return FORGE_CLASSIFICATION_MAX_TIER


def captured_authoring_fields(basic, tiers):
    fields = {}
    if "Classification" in basic:
        classification = int(basic["Classification"])
        fields["forge"] = {
            "classification": classification,
            "max_tier": tiers[classification],
        }
    attack = basic.get("Attack")
    if attack and not attack_fields(attack):
        fields["weapon_attack_modifier_points"] = launcher_fields(attack)["attack"]
    return fields


def project_item_authoring(root, definitions, legacy_authoring, *, rows=None):
    """Overlay the capture on the sealed legacy authoring rows without mutating them."""
    rows = load_capture(root) if rows is None else rows
    tiers = forge_tiers(root)
    effective = {key: dict(row) for key, row in legacy_authoring.items()}
    for key, definition in definitions.items():
        identity = definition["identity"]
        captured = rows.get(item_id(identity))
        if captured and (fields := captured_authoring_fields(captured["basic"], tiers)):
            profile = effective.setdefault(key, {"item": identity})
            profile.update(fields)
    return effective


def apply_trade_source(package_root, rows, reference, counts):
    identities = {
        item_id(record["identity"]): record["identity"]
        for record in reference["records"]
        if record["identity"]["family"] == "Item"
        and item_id(record["identity"]) is not None
    }
    path = package_root / "definitions/declarations.json"
    raw = path.read_bytes()
    document = json.loads(raw)
    apply_trades(document["records"], rows, identities, counts)
    updated = (
        json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n"
    ).encode()
    if updated != raw:
        path.write_bytes(updated)
    return document


def apply_admissions(item_records, root, *, rows=None):
    """Replay verified client semantics during legacy-to-tree generation."""
    rows = load_capture(root) if rows is None else rows
    weights = capture_weights(rows)
    for definition in item_records:
        apply_weight(definition, weights)
        if row := rows.get(item_id(definition["identity"])):
            apply_stats(definition, row["basic"], Counter())


def materialize_details(root, reference, declarations):
    """Regenerate the affected tree projections and preserve other registry owners."""
    import questpc7_world_seal
    import world_project_v2_to_tree as tree
    from regenerate_content import repin_rust_inventory

    if root.resolve() != tree.ROOT:
        raise ValueError("Details regeneration requires its repository checkout")
    questpc7_world_seal.refresh(root)
    repin_rust_inventory()
    tree.materialize_item_tree(
        reference,
        declarations,
        tree.load(tree.LEGACY / "editor/author.json"),
        tree.load(tree.LEGACY / "provenance/sources.json"),
    )

    trades = {
        row["identity"]["key"]: row
        for row in declarations["records"]
        if row["kind"] == "Service" and row["identity"]["key"].startswith(TRADE_PREFIX)
    }
    trade_index = tree.load(root / "content/services/trade/index.json")
    for relative in trade_index["shards"]:
        shard = tree.load(root / relative)
        for row in shard["records"]:
            key = row["declaration"]["identity"]["key"]
            if key.startswith(TRADE_PREFIX):
                row["declaration"] = trades.pop(key)
        tree.write(relative, shard)
    if trades:
        raise ValueError("Global trade import changed the service identity inventory")

    source = "content/world/definitions/declarations.json"
    blob = tree.git_blob_sha(root / source)
    for path in sorted((root / "content").rglob("index.json")):
        index = tree.load(path)
        legacy = index.get("legacy_source", {})
        if legacy.get("path") == source and legacy["git_blob_sha"] != blob:
            legacy["git_blob_sha"] = blob
            tree.write(path.relative_to(root).as_posix(), index)
    lock_path = "content/content.lock.json"
    lock = tree.load(root / lock_path)
    lock["legacy_blobs"]["declarations"] = blob
    tree.write_registry(lock_path, lock)


def refresh_profile_input_pins(root, base, rows):
    """Rebind active native source profiles only across verified client detail edits."""
    from functools import cache

    from grizzly_shop_profiles import encode

    tiers = forge_tiers(root)

    @cache
    def previous(path):
        return subprocess.check_output(
            ["git", "-C", str(root), "show", f"{base}:{path}"]
        )

    def definitions(document):
        return {
            row["definition"]["identity"]["key"]: row["definition"]
            for row in json.loads(document)["records"]
        }

    @cache
    def verified_shard(path):
        if not re.fullmatch(r"content/items/definitions/items-\d{5}-\d{5}\.json", path):
            raise ValueError(f"Unexpected Cyclopedia input pin path: {path}")
        raw = (root / path).read_bytes()
        expected = json.loads(previous(path))
        for record in expected["records"]:
            definition = record["definition"]
            if row := rows.get(item_id(definition["identity"])):
                apply_stats(definition, row["basic"], Counter())
                if authoring := captured_authoring_fields(row["basic"], tiers):
                    record.setdefault("authoring", {}).update(authoring)
        if expected != json.loads(raw):
            raise ValueError(f"Canonical Item changes exceed capture replay: {path}")
        return raw, definitions(raw)

    def digest(value):
        return hashlib.sha256(value).hexdigest()

    def definition_bytes(definition):
        return json.dumps(
            definition, ensure_ascii=False, sort_keys=True, separators=(",", ":")
        ).encode()

    changed = []
    directory = "tools/content-schema/native-gameplay/"
    for name in (
        "condition-potion",
        "grizzly-shop",
        "monk-reward",
        "quests-2-emperor-item",
        "task-board-item",
        "quest-batch3-item",
        "qnpcfx-item",
    ):
        relative = f"{directory}{name}-profiles.json"
        baseline = json.loads(previous(relative))
        packet = json.loads((root / relative).read_bytes())
        if {
            **packet,
            "canonical_inputs": baseline["canonical_inputs"],
        } != baseline or len(packet["canonical_inputs"]) != len(
            baseline["canonical_inputs"]
        ):
            raise ValueError(f"Native source profile changed: {relative}")
        for original, current in zip(
            baseline["canonical_inputs"], packet["canonical_inputs"], strict=True
        ):
            path = original["path"]
            field = "definition_sha256" if "key" in original else "sha256"
            if {**current, field: original[field]} != original:
                raise ValueError(f"Native source profile input changed: {relative}")
            raw = (root / path).read_bytes()
            if raw == previous(path):
                continue
            raw, after = verified_shard(path)
            old_raw = previous(path)
            if "key" in original:
                key = original["key"]
                old_raw = definition_bytes(definitions(old_raw)[key])
                raw = definition_bytes(after[key])
            if digest(old_raw) != original[field] or current[field] not in (
                original[field],
                digest(raw),
            ):
                raise ValueError(
                    f"Native source profile pin mismatch: {relative} {path}"
                )
            current[field] = digest(raw)
        encoded = encode(packet)
        if encoded != (root / relative).read_bytes():
            (root / relative).write_bytes(encoded)
            changed.append(relative)
    return changed


def refresh_reward_proof(root, base, rows):
    """Run the reward proof generator with an exact capture replay as qualifier."""
    from unittest.mock import patch

    import refresh_reward_stack_weight_proof as proof

    def qualified_digest(before, after, pinned, _weight):
        if proof.semantic_digest(before) != pinned:
            raise ValueError("REWARD_STACK_BASE_DEFINITION_DRIFT")
        identifier = item_id(before["identity"])
        apply_stats(before, rows[identifier]["basic"], Counter())
        if before != after:
            raise ValueError("REWARD_STACK_NON_CYCLOPEDIA_CHANGE")
        return proof.semantic_digest(after)

    with patch.object(proof, "qualified_digest", qualified_digest):
        return proof.refresh(root, base)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument(
        "--details-only",
        action="store_true",
        help="import stats and NPC trades beyond weight",
    )
    parser.add_argument(
        "--refresh-detail-pins-from",
        metavar="BASE",
        help="requalify derived pins after tree regeneration against this Git base",
    )
    args = parser.parse_args()
    root = args.root
    if args.refresh_detail_pins_from:
        rows = load_capture(root)
        base = args.refresh_detail_pins_from
        print(
            json.dumps(
                {
                    "reward_stack_proofs": refresh_reward_proof(root, base, rows),
                    "native_profile_pins": refresh_profile_input_pins(root, base, rows),
                },
                indent=2,
            )
        )
        return
    if args.details_only:
        rows = load_capture(root)
        counts = Counter()
        package_root = root / "content/world"
        reference = json.loads(
            (package_root / "definitions/reference.json").read_bytes()
        )
        declarations = apply_trade_source(package_root, rows, reference, counts)
        materialize_details(root, reference, declarations)
        print(json.dumps(counts, sort_keys=True, indent=2))
        return
    weights = load_weights(root)
    overlays = refresh_overlay_weights(root, weights)
    counts = Counter()
    constructors = []

    def write(path, document):
        previous = path.read_text()
        # Preserve the canonical compact shard representation.
        compact = "\n" not in previous.rstrip("\n")
        options = (
            {"separators": (",", ":"), "sort_keys": True} if compact else {"indent": 2}
        )
        path.write_text(json.dumps(document, ensure_ascii=False, **options) + "\n")

    paths = sorted((root / "content/items/definitions").glob("items-*.json"))
    paths.append(root / "content/craft/runtime-item-profiles.json")
    for path in paths:
        document = json.loads(path.read_bytes())
        changed = False
        for record in document["records"]:
            if apply_weight(record.get("definition", record), weights):
                counts[str(path.relative_to(root))] += 1
                changed = True
        if changed:
            write(path, document)
    path = (
        root
        / "tools/content-schema/native-gameplay/authored-static-item-constructors.json"
    )
    document = json.loads(path.read_bytes())
    for declaration in document["constructors"]:
        identifier = int(declaration["item"].removeprefix("oteryn:item.tibia.i"))
        if identifier in weights and declaration["weight"] != weights[identifier]:
            constructors.append(
                [identifier, declaration["weight"], weights[identifier]]
            )
            declaration["weight"] = weights[identifier]
    if constructors:
        write(path, document)
    print(
        json.dumps(
            {
                "weight_changes_by_file": counts,
                "constructor_weights": constructors,
                "overlay_weights": overlays,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
