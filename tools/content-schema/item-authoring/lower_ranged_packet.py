"""Lower the RANGED-CONTENT-1 weapon rows (RANGED-0 section 7) from TibiaWiki and Canary into one packet.

Rule: RANGED-0 section 7. Source policy: tibia.com, then TibiaWiki; Canary `items.xml` (D384 pin,
OTS_HYPOTHESIS_ONLY) is the fallback. Per field, the pinned TibiaWiki stat snapshot wins when every
page that lists the Item agrees on one value that parses; a disagreeing or unparseable wiki value
is reported under `conflicts` and the Canary value is used. A field only Canary has is Canary's.
Where both sources have a value and the two differ, the field is listed in the row's
`parity_pending` (RANGED-0: every value is `PARITY_PENDING` until RANGED-PARITY-1).

Per Item with a distance, throwing, ammunition or wand weapon: `weapon_kind` (`DISTANCE`,
`THROWING`, `AMMUNITION`, `WAND`), `ammo_type`, `range_tiles` (1-7), `hit_chance` (the relative bonus, Canary `hitchance`),
`max_hit_chance` (the absolute cap, the wiki hit chance first, then Canary `maxhitchance`), `break_chance_pct`, `on_use`, `mana_per_shot`, wand `damage_min` / `damage_max`
and `element`, `missile` (the Canary `shoottype` name; the client missile id is the presentation
owner's), `requirements` (level, magic level, vocations) and the `quiver` flag on quivers.
A throwing weapon is a distance weapon without an `ammotype` (RANGED-0 section 3.2), and an Item
whose Canary row has no weapon registration (`script` with a nested `weaponType`) is held: the
engine cannot fire it. `on_use`: ammunition `CONSUME`; a throwing weapon `CONSUME` when its
registration says `action = removecount`, else `DROP`; a distance weapon or wand `NONE`.

An Item is held, gets no row and is listed under `holds` by reason when its kind cannot be told,
when a kind-required field has no usable source, or when it has no content definition. The packet
is applied by `item_ranged_promotion.rs`; `--check` rebuilds it in memory and fails on any byte
difference.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path

from lower_equip_abilities_packet import CANARY, canary_pin
from lower_wiki_stats_packet import ROOT, SNAPSHOT, content_item_ids

OUTPUT = (
    ROOT / "docs" / "agents" / "evidence" / "OTV2-20261013-ranged-weapon-facts-v1.json"
)
WEAPON_METADATA_PROOF = "docs/agents/evidence/OTV2-20261002-item-weapon-metadata-source-qualification-v2.json"
WEAPON_METADATA_PROOF_SHA256 = (
    "44d6a859c38a0a8235c501ce15478f4e0edb829fba049a7587ea642f261dc6dc"
)
WEAPON_METADATA_PROOF_SCHEMA = "OTERYN_ITEM_WEAPON_METADATA_SOURCE_QUALIFICATION/v2"
SCHEMA = "OTERYN_ITEM_RANGED_PROMOTION/v1"
TASK_ID = "OTV2-20261013-ranged-content-1"
COMPILER = "tools/content-schema/item-authoring/lower_ranged_packet.py"

WIKI_PRIMARY_KINDS = {
    "Ammunition": "AMMUNITION",
    "Distance Weapons": "DISTANCE",
    "Rods": "WAND",
    "Wands": "WAND",
}
CANARY_KINDS = {"distance": "DISTANCE", "ammunition": "AMMUNITION", "wand": "WAND"}
AMMO_TYPES = {"arrow": "ARROW", "bolt": "BOLT"}
ELEMENTS = {
    "death": "DEATH",
    "earth": "EARTH",
    "energy": "ENERGY",
    "fire": "FIRE",
    "ice": "ICE",
}
VOCATIONS = {
    "knight": "KNIGHT",
    "paladin": "PALADIN",
    "sorcerer": "SORCERER",
    "druid": "DRUID",
    "none": "NONE",
    "without": "NONE",
}
VOCATION_ORDER = ("KNIGHT", "PALADIN", "SORCERER", "DRUID", "NONE")
MISSILE = re.compile(r"^[a-z0-9_]{1,40}$")
UINT = re.compile(r"^[0-9]+$")
SINT = re.compile(r"^-?[0-9]+$")


def _key(item_id):
    return f"oteryn:item.tibia.i{item_id}"


def _sint(raw, low=-100, high=100):
    if raw is None or not SINT.match(raw):
        return None
    value = int(raw)
    return value if low <= value <= high else None


def _uint(raw, low=0, high=65535):
    if raw is None or not UINT.match(raw):
        return None
    value = int(raw)
    return value if low <= value <= high else None


def load_canary_weapons():
    """Canary Item id -> `{"top": {...}, "nested": {...}}`, keys lower-cased; ranges expand.

    `nested` is the attribute map of the `script` attribute (the weapon registration).
    """
    out = {}
    for item in ET.parse(CANARY).getroot().iter("item"):
        top, nested = {}, None
        for node in item.findall("attribute"):
            key = node.get("key").lower()
            top[key] = node.get("value")
            if key == "script":
                nested = {
                    child.get("key").lower(): child.get("value")
                    for child in node.findall("attribute")
                }
        if item.get("id"):
            ids = [int(item.get("id"))]
        else:
            ids = range(int(item.get("fromid")), int(item.get("toid")) + 1)
        for item_id in ids:
            out[item_id] = {"top": top, "nested": nested}
    return out


def wiki_fields(snapshot):
    """Item id -> (field -> raw value | CONFLICT sentinel set); the pages listing the Item."""
    out = {}
    for record in snapshot["records"].values():
        merged, pages = {}, []
        for observation in record["observations"]:
            pages.append(observation["page_id"])
            for name, raw in observation["fields"].items():
                merged.setdefault(name, set()).add(raw)
        out[record["item_id"]] = (merged, sorted(set(pages)))
    return out


def _wiki_value(merged, name, parse, conflicts, item_id):
    """The one parsed wiki value, or None (absent, disagreeing or unparseable; the last two are
    listed under `conflicts`)."""
    raws = merged.get(name)
    if not raws:
        return None
    if len(raws) > 1:
        conflicts.append(
            {"item_key": _key(item_id), "field": name, "reason": "wiki_pages_disagree"}
        )
        return None
    (raw,) = raws
    value = parse(raw)
    if value is None:
        conflicts.append(
            {"item_key": _key(item_id), "field": name, "reason": "wiki_unparseable"}
        )
    return value


def _parse_range(raw):
    return _uint(raw, 1, 7)


def _parse_mana(raw):
    """Wiki `manacost`: a bare number, or the decorated `+N` / `+N mana per attack` form."""
    match = re.fullmatch(r"\+?([0-9]+)(?:\s+mana per attack)?", raw.strip())
    return int(match[1]) if match and int(match[1]) <= 65535 else None


def _parse_percent(raw):
    match = re.fullmatch(r"([0-9]{1,3})%", raw)
    return int(match[1]) if match and 0 <= int(match[1]) <= 100 else None


def _parse_damage(raw):
    # Wiki writes `min-max` or `average (min-max)`; the average is derived, only the range counts.
    match = re.fullmatch(
        r"(?:[0-9]+\s*\(\s*([0-9]+)\s*-\s*([0-9]+)\s*\)|([0-9]+)-([0-9]+))", raw.strip()
    )
    if not match:
        return None
    low, high = int(match[1] or match[3]), int(match[2] or match[4])
    return (low, high) if 0 <= low <= high <= 65535 else None


def _parse_element(raw):
    return ELEMENTS.get(raw.strip().lower())


def _parse_vocations(raw):
    """Wiki `vocrequired`: base vocations joined by commas or `and`; `without` is `NONE`."""
    parts = re.split(r",\s*|\s+and\s+", raw.strip().lower())
    values = [
        VOCATIONS.get(p[:-1] if p.endswith("s") and p != "without" else p)
        for p in parts
    ]
    if None in values or len(set(values)) != len(values):
        return None
    return sorted(values, key=VOCATION_ORDER.index)


def _canary_vocations(raw):
    """Canary `Sorcerer;true, Master Sorcerer`: the entries with `;true` are the base
    vocations (the rest are their promotions)."""
    if raw is None:
        return None
    values = []
    for part in raw.split(","):
        name, _, flag = part.strip().partition(";")
        if flag != "true":
            continue
        value = VOCATIONS.get(name.strip().lower())
        if value is None:
            return None
        values.append(value)
    if not values or len(set(values)) != len(values):
        return None
    return sorted(values, key=VOCATION_ORDER.index)


def closed_weapon_owners():
    """Item keys whose weapon leaf is pinned by the closed weapon-metadata packet (its native_weapon_guard)."""
    raw = (ROOT / WEAPON_METADATA_PROOF).read_bytes()
    if hashlib.sha256(raw).hexdigest() != WEAPON_METADATA_PROOF_SHA256:
        raise ValueError("weapon-metadata proof digest drift")
    proof = json.loads(raw)
    if proof.get("schema") != WEAPON_METADATA_PROOF_SCHEMA:
        raise ValueError("weapon-metadata proof schema drift")
    return {r["target"]["key"] for r in proof["records"]}


def _requirements(merged, nested, local_conflicts, item_id, parity, sources):
    """Level, magic level and vocations: wiki first, Canary fallback, with provenance and parity."""
    requirements = {}

    def pick_requirement(name, wiki_value, canary_value):
        if wiki_value is not None:
            if canary_value is not None and canary_value != wiki_value:
                parity.append(name)
            value, source = wiki_value, "WIKI"
        elif canary_value is not None:
            value, source = canary_value, "CANARY"
        else:
            return
        sources[name] = source
        if value:
            requirements[name] = value

    pick_requirement(
        "min_level",
        _wiki_value(merged, "levelrequired", _uint, local_conflicts, item_id),
        _uint(nested.get("level")),
    )
    pick_requirement(
        "min_magic_level",
        _wiki_value(merged, "mlrequired", _uint, local_conflicts, item_id),
        None,
    )
    pick_requirement(
        "vocations",
        _wiki_value(merged, "vocrequired", _parse_vocations, local_conflicts, item_id),
        _canary_vocations(nested.get("vocation")),
    )
    return requirements


def build(snapshot, canary, content_ids, owned=frozenset()):
    """(promotions, holds, conflicts, counts)."""
    wiki = wiki_fields(snapshot)
    promotions, holds, conflicts = [], [], []
    candidates = set()
    for item_id, (merged, _) in wiki.items():
        primary = merged.get("primarytype", set())
        if primary & (set(WIKI_PRIMARY_KINDS) | {"Quivers"}):
            candidates.add(item_id)
    for item_id, row in canary.items():
        top = row["top"]
        if top.get("weapontype") in CANARY_KINDS or top.get("primarytype") == "quivers":
            candidates.add(item_id)
    for item_id in sorted(candidates):
        merged, pages = wiki.get(item_id, ({}, []))
        c = canary.get(item_id, {"top": {}, "nested": None})
        top, nested = c["top"], c["nested"]
        reasons, local_conflicts = [], []
        if item_id not in content_ids:
            holds.append(
                {"item_key": _key(item_id), "reasons": ["no_content_definition"]}
            )
            continue
        wiki_primary = merged.get("primarytype", set())
        is_quiver = "Quivers" in wiki_primary or top.get("primarytype") == "quivers"
        wiki_kinds = {
            WIKI_PRIMARY_KINDS[p] for p in wiki_primary if p in WIKI_PRIMARY_KINDS
        }
        canary_kind = CANARY_KINDS.get(top.get("weapontype"))
        if len(wiki_kinds) > 1:
            holds.append(
                {"item_key": _key(item_id), "reasons": ["wiki_kind_ambiguous"]}
            )
            continue
        if not is_quiver and _key(item_id) in owned:
            holds.append(
                {"item_key": _key(item_id), "reasons": ["owned_by_weapon_metadata_103"]}
            )
            continue
        ammo = AMMO_TYPES.get(top.get("ammotype"))
        # Canary decides what the engine can fire; the wiki primary type is the cross-check.
        kind = canary_kind or next(iter(wiki_kinds), None)
        if kind is None and is_quiver:
            promotions.append(
                _quiver_row(
                    item_id,
                    top,
                    nested,
                    merged,
                    pages,
                    "WIKI" if "Quivers" in wiki_primary else "CANARY",
                    local_conflicts,
                )
            )
            conflicts.extend(local_conflicts)
            continue
        if is_quiver:
            holds.append({"item_key": _key(item_id), "reasons": ["quiver_and_weapon"]})
            continue
        if kind is None:
            holds.append({"item_key": _key(item_id), "reasons": ["kind_unknown"]})
            continue
        if kind == "DISTANCE" and ammo is None:
            kind = "THROWING"  # a distance weapon without an ammotype (RANGED-0 section 3.2)
        parity = []
        wiki_equiv = {
            "DISTANCE": {"DISTANCE", "AMMUNITION"},
            "THROWING": {"DISTANCE", "AMMUNITION"},
        }
        if wiki_kinds and not wiki_kinds & wiki_equiv.get(kind, {kind}):
            local_conflicts.append(
                {
                    "item_key": _key(item_id),
                    "field": "weapon_kind",
                    "reason": "wiki_" + "_".join(sorted(wiki_kinds)),
                }
            )
            parity.append("weapon_kind")
        sources = {"weapon_kind": "CANARY" if canary_kind else "WIKI"}
        row = {"item_key": _key(item_id), "weapon_kind": kind}

        def pick(
            field, wiki_value, canary_value, parity=parity, row=row, sources=sources
        ):
            """Wiki first, Canary fallback; both present and different is parity-pending."""
            if wiki_value is not None:
                if canary_value is not None and canary_value != wiki_value:
                    parity.append(field)
                row[field] = wiki_value
                sources[field] = "WIKI"
            elif canary_value is not None:
                row[field] = canary_value
                sources[field] = "CANARY"

        if kind in ("DISTANCE", "AMMUNITION"):
            if ammo is None:
                reasons.append("ammo_type_unknown")
            else:
                row["ammo_type"] = ammo
                sources["ammo_type"] = "CANARY"
        registered = nested is not None and "weapontype" in nested
        if not registered:
            reasons.append("no_weapon_registration")

        range_tiles = _wiki_value(
            merged, "range", _parse_range, local_conflicts, item_id
        )
        pick("range_tiles", range_tiles, _uint(top.get("range"), 1, 7))
        if kind != "AMMUNITION" and "range_tiles" not in row:
            reasons.append("range_tiles_unknown")
        # Canary `hitchance` is the relative bonus a bow or ammunition adds (the tree `hit_chance`);
        # the wiki "Hit chance" and Canary `maxhitchance` are the absolute cap (`max_hit_chance`).
        pick("hit_chance", None, _sint(top.get("hitchance")))
        if kind in ("DISTANCE", "THROWING", "AMMUNITION"):
            cap = _wiki_value(
                merged, "hit_chance", _parse_percent, local_conflicts, item_id
            )
            pick("max_hit_chance", cap, _uint(top.get("maxhitchance"), 0, 100))
        if kind in ("DISTANCE", "THROWING", "AMMUNITION", "WAND"):
            shoot = top.get("shoottype")
            if shoot is not None and MISSILE.match(shoot.lower()):
                row["missile"] = shoot.lower()
                sources["missile"] = "CANARY"
            elif kind != "DISTANCE":  # a bow or crossbow shows its ammunition's missile
                reasons.append("missile_unknown")
        nested = nested or {}
        if kind == "THROWING":
            raw_break = nested.get("breakchance")
            row["break_chance_pct"] = _uint(raw_break, 0, 100) if raw_break else 0
            if row["break_chance_pct"] is None:
                reasons.append("break_chance_malformed")
            sources["break_chance_pct"] = "CANARY" if raw_break else "DERIVED"
            row["on_use"] = (
                "CONSUME" if nested.get("action") == "removecount" else "DROP"
            )
            sources["on_use"] = "CANARY" if nested.get("action") else "DERIVED"
        elif kind == "AMMUNITION":
            if nested.get("action") != "removecount":
                reasons.append("ammunition_action_unknown")
            row["break_chance_pct"] = 0
            sources["break_chance_pct"] = "DERIVED"
            row["on_use"] = "CONSUME"
            sources["on_use"] = "CANARY"
        else:
            row["on_use"] = "NONE"
            sources["on_use"] = "DERIVED"
        if kind == "WAND":
            mana = _wiki_value(
                merged,
                "manacost",
                _parse_mana,
                local_conflicts,
                item_id,
            )
            pick("mana_per_shot", mana, _uint(nested.get("mana"), 0, 65535))
            if "mana_per_shot" not in row:
                reasons.append("mana_per_shot_unknown")
            damage = _wiki_value(
                merged, "damagerange", _parse_damage, local_conflicts, item_id
            )
            low, high = _uint(nested.get("fromdamage")), _uint(nested.get("todamage"))
            canary_damage = (
                (low, high)
                if low is not None and high is not None and 0 <= low <= high
                else None
            )
            if damage is not None:
                if canary_damage is not None and canary_damage != damage:
                    parity.append("damage")
                row["damage_min"], row["damage_max"] = damage
                sources["damage"] = "WIKI"
            elif canary_damage is not None:
                row["damage_min"], row["damage_max"] = canary_damage
                sources["damage"] = "CANARY"
            else:
                reasons.append("damage_unknown")
            element = _wiki_value(
                merged, "damagetype", _parse_element, local_conflicts, item_id
            )
            pick(
                "element", element, ELEMENTS.get((nested.get("wandtype") or "").lower())
            )
            if "element" not in row:
                reasons.append("element_unknown")
        requirements = _requirements(
            merged, nested, local_conflicts, item_id, parity, sources
        )
        if requirements:
            row["requirements"] = requirements
        if reasons:
            holds.append({"item_key": _key(item_id), "reasons": sorted(set(reasons))})
            conflicts.extend(local_conflicts)
            continue
        row["sources"] = dict(sorted(sources.items()))
        if pages:
            row["wiki_pages"] = pages
        if parity:
            row["parity_pending"] = sorted(set(parity))
        promotions.append(row)
        conflicts.extend(local_conflicts)
    counts = Counter(
        {
            "items": len(promotions),
            "holds": len(holds),
            "conflicts": len(conflicts),
            "quivers": sum(1 for p in promotions if p.get("quiver")),
            "parity_pending_items": sum(
                1 for p in promotions if p.get("parity_pending")
            ),
        }
    )
    for kind in ("DISTANCE", "THROWING", "AMMUNITION", "WAND"):
        counts[f"kind_{kind}"] = sum(
            1 for p in promotions if p.get("weapon_kind") == kind
        )
    return promotions, holds, conflicts, dict(sorted(counts.items()))


def _quiver_row(item_id, top, nested, merged, pages, quiver_source, local_conflicts):
    row = {
        "item_key": _key(item_id),
        "quiver": True,
    }
    sources = {"quiver": quiver_source}
    size = _uint(top.get("containersize"), 1, 255)
    if size is not None:
        row["container_size"] = size
        sources["container_size"] = "CANARY"
    parity = []
    requirements = _requirements(
        merged, nested or {}, local_conflicts, item_id, parity, sources
    )
    if requirements:
        row["requirements"] = requirements
    row["sources"] = dict(sorted(sources.items()))
    if pages:
        row["wiki_pages"] = pages
    if parity:
        row["parity_pending"] = sorted(set(parity))
    return row


def packet_bytes(snapshot, canary, content_ids, pin):
    promotions, holds, conflicts, counts = build(
        snapshot, canary, content_ids, closed_weapon_owners()
    )
    packet = {
        "schema": SCHEMA,
        "decision": "RANGED-0 section 7",
        "task_id": TASK_ID,
        "compiler": COMPILER,
        "source": {
            "canary": pin,
            "wiki_snapshot": str(SNAPSHOT.relative_to(ROOT)),
            "weapon_metadata_proof": {
                "path": WEAPON_METADATA_PROOF,
                "schema": WEAPON_METADATA_PROOF_SCHEMA,
                "sha256": WEAPON_METADATA_PROOF_SHA256,
            },
            "policy": "TibiaWiki first, Canary fallback; differing values are parity_pending",
        },
        "counts": counts,
        "promotions": promotions,
        "holds": holds,
        "conflicts": conflicts,
    }
    return (json.dumps(packet, indent=2, sort_keys=True) + "\n").encode("utf-8")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args(argv)
    snapshot = json.loads(SNAPSHOT.read_text(encoding="utf-8"))
    data = packet_bytes(
        snapshot, load_canary_weapons(), content_item_ids(), canary_pin()
    )
    if args.check:
        if args.output.read_bytes() != data:
            print(f"packet drift against {args.output}", file=sys.stderr)
            return 1
        print(json.dumps({"check": "PASS", "bytes": len(data)}))
        return 0
    args.output.write_bytes(data)
    print(json.dumps({"counts": json.loads(data)["counts"], "bytes": len(data)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
