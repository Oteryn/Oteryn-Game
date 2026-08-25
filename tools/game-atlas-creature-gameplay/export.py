#!/usr/bin/env python3
"""Deterministic, static-only Game -> Atlas creature gameplay profile extraction."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import sys
from typing import Any

HERE = Path(__file__).resolve().parent
CREATURE_TOOLS = HERE.parent / "game-atlas-creatures"
if str(CREATURE_TOOLS) not in sys.path:
    sys.path.insert(0, str(CREATURE_TOOLS))
from identity import stable_creature_entity_id

CONTRACT_ID = "oteryn-game-atlas-export-v1"
CAPABILITY = "creature-gameplay-profiles-v1"
PROFILE_SCHEMA_VERSION = 1
LEGACY_EVIDENCE_SHA = "e417c5e7c22986bf4acef0495eb47f7b72c97cce"
NPC_NAME = re.compile(r'local\s+internalNpcName\s*=\s*["\']([^"\']+)["\']')
MONSTER_NAME = re.compile(r'''Game\.createMonsterType\(\s*(?:"([^"]+)"|'([^']+)')\s*\)''')
SERVICE_ORDER = ("bank", "blessing", "trainer", "shop", "travel", "quest")
ELEMENT_TYPES = {
    "COMBAT_PHYSICALDAMAGE": "physical",
    "COMBAT_ENERGYDAMAGE": "energy",
    "COMBAT_EARTHDAMAGE": "earth",
    "COMBAT_FIREDAMAGE": "fire",
    "COMBAT_LIFEDRAIN": "life_drain",
    "COMBAT_MANADRAIN": "mana_drain",
    "COMBAT_DROWNDAMAGE": "drown",
    "COMBAT_ICEDAMAGE": "ice",
    "COMBAT_HOLYDAMAGE": "holy",
    "COMBAT_DEATHDAMAGE": "death",
}


class ExportError(RuntimeError):
    pass


def _strip_line_comments(text: str) -> str:
    out: list[str] = []
    i = 0
    quote: str | None = None
    escaped = False
    while i < len(text):
        ch = text[i]
        if quote is not None:
            out.append(ch)
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == quote:
                quote = None
            i += 1
            continue
        if ch in {'"', "'"}:
            quote = ch
            out.append(ch)
            i += 1
            continue
        if ch == "-" and i + 1 < len(text) and text[i + 1] == "-":
            i += 2
            while i < len(text) and text[i] not in "\r\n":
                i += 1
            continue
        out.append(ch)
        i += 1
    return "".join(out)


def _matching(text: str, start: int, opener: str, closer: str) -> int:
    if start >= len(text) or text[start] != opener:
        raise ExportError(f"expected {opener!r} at offset {start}")
    depth = 0
    quote: str | None = None
    escaped = False
    i = start
    while i < len(text):
        ch = text[i]
        if quote is not None:
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == quote:
                quote = None
            i += 1
            continue
        if ch in {'"', "'"}:
            quote = ch
        elif ch == opener:
            depth += 1
        elif ch == closer:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    raise ExportError(f"unterminated {opener}{closer} block")


def _table_assignments(text: str, lhs: str) -> list[str | None]:
    pattern = re.compile(rf"\b{re.escape(lhs)}\s*=\s*")
    result: list[str | None] = []
    for match in pattern.finditer(text):
        pos = match.end()
        if pos < len(text) and text[pos] == "{":
            end = _matching(text, pos, "{", "}")
            result.append(text[pos : end + 1])
        else:
            result.append(None)
    return result


def _split_top_level(text: str) -> list[str]:
    parts: list[str] = []
    start = 0
    braces = parens = brackets = 0
    quote: str | None = None
    escaped = False
    for i, ch in enumerate(text):
        if quote is not None:
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == quote:
                quote = None
            continue
        if ch in {'"', "'"}:
            quote = ch
        elif ch == "{":
            braces += 1
        elif ch == "}":
            braces -= 1
        elif ch == "(":
            parens += 1
        elif ch == ")":
            parens -= 1
        elif ch == "[":
            brackets += 1
        elif ch == "]":
            brackets -= 1
        elif ch == "," and braces == 0 and parens == 0 and brackets == 0:
            token = text[start:i].strip()
            if token:
                parts.append(token)
            start = i + 1
    token = text[start:].strip()
    if token:
        parts.append(token)
    return parts


def _table_entries(block: str) -> tuple[list[str], bool]:
    inner = block[1:-1].strip()
    if not inner:
        return [], True
    entries: list[str] = []
    ok = True
    for token in _split_top_level(inner):
        token = token.strip()
        if token.startswith("{"):
            try:
                end = _matching(token, 0, "{", "}")
            except ExportError:
                ok = False
                continue
            if token[end + 1 :].strip():
                ok = False
            entries.append(token[: end + 1])
        else:
            ok = False
    return entries, ok


def _field_tokens(block: str) -> tuple[dict[str, str], bool]:
    inner = block[1:-1].strip()
    fields: dict[str, str] = {}
    ok = True
    if not inner:
        return fields, True
    for token in _split_top_level(inner):
        match = re.fullmatch(r"([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.+)", token.strip(), re.DOTALL)
        if match is None:
            ok = False
            continue
        key, raw = match.group(1), match.group(2).strip()
        if key in fields:
            ok = False
            continue
        fields[key] = raw
    return fields, ok


def _literal_string(raw: str | None) -> str | None:
    if raw is None or len(raw) < 2 or raw[0] not in {'"', "'"} or raw[-1] != raw[0]:
        return None
    body = raw[1:-1]
    if "\\" in body:
        try:
            return bytes(body, "utf-8").decode("unicode_escape")
        except UnicodeDecodeError:
            return None
    return body


def _literal_int(raw: str | None) -> int | None:
    if raw is None or re.fullmatch(r"-?\d+", raw) is None:
        return None
    return int(raw)


def _literal_bool(raw: str | None) -> bool | None:
    if raw == "true":
        return True
    if raw == "false":
        return False
    return None


def _literal_position(raw: str | None) -> dict[str, int] | None:
    if raw is None:
        return None
    match = re.fullmatch(r"Position\(\s*(-?\d+)\s*,\s*(-?\d+)\s*,\s*(-?\d+)\s*\)", raw)
    if match is None:
        return None
    x, y, z = (int(value) for value in match.groups())
    return {"x": x, "y": y, "floor": -z}


def _reasoned(state: str, **payload: Any) -> dict[str, Any]:
    result = {"state": state, **payload}
    result.setdefault("reason_codes", [])
    return result


def _shop_profile(text: str, items: dict[str, dict[str, Any]]) -> dict[str, Any]:
    assignments = _table_assignments(text, "npcConfig.shop")
    if not assignments:
        return _reasoned("UNKNOWN", sells=[], buys=[], reason_codes=["NO_STATIC_SHOP_EVIDENCE"])
    if len(assignments) != 1 or assignments[0] is None:
        return _reasoned("PARTIAL", sells=[], buys=[], reason_codes=["DYNAMIC_SHOP_ASSIGNMENT_UNSUPPORTED"])
    entries, structural_ok = _table_entries(assignments[0])
    sells: list[dict[str, Any]] = []
    buys: list[dict[str, Any]] = []
    reasons: set[str] = set()
    if not structural_ok:
        reasons.add("UNSUPPORTED_SHOP_ROW")
    for entry in entries:
        fields, fields_ok = _field_tokens(entry)
        if not fields_ok:
            reasons.add("UNSUPPORTED_SHOP_ROW")
            continue
        name = _literal_string(fields.get("itemName"))
        client_id = _literal_int(fields.get("clientId"))
        if name is None:
            reasons.add("UNSUPPORTED_SHOP_ROW")
            continue
        ref = None
        resolution = "UNRESOLVED"
        buy_price = _literal_int(fields.get("buy")) if "buy" in fields else None
        sell_price = _literal_int(fields.get("sell")) if "sell" in fields else None
        if "buy" in fields and buy_price is None:
            reasons.add("DYNAMIC_SHOP_PRICE_UNSUPPORTED")
        if "sell" in fields and sell_price is None:
            reasons.add("DYNAMIC_SHOP_PRICE_UNSUPPORTED")
        for price in (buy_price, sell_price):
            if price is not None and price < 0:
                raise ExportError("shop price must be non-negative")
        base = {"item_ref": ref, "item_name": name, "item_resolution_state": resolution, "currency": "gold"}
        if buy_price is not None:
            sells.append({**base, "unit_price": buy_price})
        if sell_price is not None:
            buys.append({**base, "unit_price": sell_price})
        if buy_price is None and sell_price is None:
            reasons.add("UNSUPPORTED_SHOP_ROW")
        unsupported = set(fields) - {"itemName", "clientId", "buy", "sell", "count", "subType"}
        if unsupported:
            reasons.add("CONDITIONAL_OR_EXTENDED_SHOP_ROW_UNSUPPORTED")
    if re.search(r"table\.insert\s*\(\s*npcConfig\.shop\b", text) or re.search(r"npcConfig\.shop\s*\[", text):
        reasons.add("DYNAMIC_SHOP_MUTATION_UNSUPPORTED")
    state = "COMPLETE" if not reasons else "PARTIAL"
    return _reasoned(state, sells=sells, buys=buys, reason_codes=sorted(reasons))


def _keyword_labels(text: str) -> dict[str, str]:
    labels: dict[str, str] = {}
    pattern = re.compile(r'''(?:local\s+)?([A-Za-z_]\w*)\s*=\s*[A-Za-z_]\w*\s*:\s*addKeyword\s*\(\s*\{\s*(?:"([^"]+)"|'([^']+)')''')
    for match in pattern.finditer(text):
        labels[match.group(1)] = match.group(2) or match.group(3)
    return labels


def _travel_profile(text: str) -> dict[str, Any]:
    occurrences = list(re.finditer(r"\bStdModule\.travel\b", text))
    if not occurrences:
        return _reasoned("UNKNOWN", destinations=[], reason_codes=["NO_STATIC_TRAVEL_EVIDENCE"])
    labels = _keyword_labels(text)
    destinations: list[dict[str, Any]] = []
    reasons: set[str] = set()
    handled_offsets: set[int] = set()
    call_pattern = re.compile(r"\b([A-Za-z_]\w*)\s*:\s*addChildKeyword\s*\(")
    for call in call_pattern.finditer(text):
        open_pos = text.find("(", call.start())
        if open_pos < 0:
            continue
        try:
            end_pos = _matching(text, open_pos, "(", ")")
        except ExportError:
            reasons.add("DYNAMIC_TRAVEL_UNSUPPORTED")
            continue
        body = text[open_pos + 1 : end_pos]
        travel_match = re.search(r"\bStdModule\.travel\b", body)
        if travel_match is None:
            continue
        absolute_travel = open_pos + 1 + travel_match.start()
        handled_offsets.add(absolute_travel)
        options_pos = body.find("{", travel_match.end())
        label = labels.get(call.group(1))
        if options_pos < 0 or label is None:
            reasons.add("DYNAMIC_TRAVEL_UNSUPPORTED")
            continue
        absolute_options = open_pos + 1 + options_pos
        try:
            options_end = _matching(text, absolute_options, "{", "}")
        except ExportError:
            reasons.add("DYNAMIC_TRAVEL_UNSUPPORTED")
            continue
        fields, fields_ok = _field_tokens(text[absolute_options : options_end + 1])
        position = _literal_position(fields.get("destination"))
        cost = _literal_int(fields.get("cost")) if "cost" in fields else 0
        if not fields_ok or position is None or cost is None or cost < 0:
            reasons.add("DYNAMIC_TRAVEL_UNSUPPORTED")
            continue
        if "discount" in fields or "condition" in fields:
            reasons.add("CONDITIONAL_TRAVEL_UNSUPPORTED")
        destinations.append({"label": label, "position": position, "price": cost, "currency": "gold"})
    if len(handled_offsets) != len(occurrences):
        reasons.add("DYNAMIC_TRAVEL_UNSUPPORTED")
    state = "COMPLETE" if not reasons else "PARTIAL"
    return _reasoned(state, destinations=destinations, reason_codes=sorted(reasons))


def _services_profile(text: str, shop: dict[str, Any], travel: dict[str, Any]) -> dict[str, Any]:
    values: set[str] = set()
    if "npc:parseBank(" in text or "NpcBankGreetCallback" in text:
        values.add("bank")
    if shop.get("sells") or shop.get("buys"):
        values.add("shop")
    if travel.get("destinations"):
        values.add("travel")
    if "StdModule.bless" in text or re.search(r"\bplayer:addBlessing\s*\(", text):
        values.add("blessing")
    if "StdModule.learnSpell" in text:
        values.add("trainer")
    ordered = [value for value in SERVICE_ORDER if value in values]
    if not ordered:
        return {"state": "UNKNOWN", "values": [], "reason_codes": ["NO_EXHAUSTIVE_STATIC_SERVICE_EVIDENCE"]}
    return {"state": "PARTIAL", "values": ordered, "reason_codes": ["SERVICE_TAXONOMY_NOT_EXHAUSTIVE"]}


def _loot_profile(text: str, items: dict[str, dict[str, Any]]) -> dict[str, Any]:
    assignments = _table_assignments(text, "monster.loot")
    if not assignments:
        return _reasoned("UNKNOWN", entries=[], reason_codes=["NO_STATIC_LOOT_EVIDENCE"])
    if len(assignments) != 1 or assignments[0] is None:
        return _reasoned("PARTIAL", entries=[], reason_codes=["DYNAMIC_LOOT_ASSIGNMENT_UNSUPPORTED"])
    rows, structural_ok = _table_entries(assignments[0])
    result: list[dict[str, Any]] = []
    reasons: set[str] = set()
    if not structural_ok:
        reasons.add("UNSUPPORTED_LOOT_ROW")
    for row in rows:
        fields, fields_ok = _field_tokens(row)
        if not fields_ok:
            reasons.add("UNSUPPORTED_LOOT_ROW")
            continue
        name = _literal_string(fields.get("name"))
        client_id = _literal_int(fields.get("id"))
        if client_id is None:
            client_id = _literal_int(fields.get("clientId"))
        if name is None and client_id is None:
            reasons.add("UNSUPPORTED_LOOT_ROW")
            continue
        chance = _literal_int(fields.get("chance"))
        if chance is None:
            reasons.add("DYNAMIC_LOOT_CHANCE_UNSUPPORTED")
            continue
        if not 0 <= chance <= 100000:
            reasons.add("INVALID_LOOT_CHANCE")
            continue
        min_count = _literal_int(fields.get("minCount")) if "minCount" in fields else 1
        max_count = _literal_int(fields.get("maxCount")) if "maxCount" in fields else 1
        if min_count is None or max_count is None or min_count < 0 or max_count < min_count:
            reasons.add("INVALID_LOOT_COUNT")
            continue
        ref = None
        display_name = name if name is not None else f"Item #{client_id}"
        result.append({
            "item_ref": ref,
            "item_name": display_name,
            "item_resolution_state": "RESOLVED" if ref is not None else "UNRESOLVED",
            "chance_ppm": chance * 10,
            "min_count": min_count,
            "max_count": max_count,
        })
        unsupported = set(fields) - {"name", "id", "clientId", "chance", "minCount", "maxCount"}
        if unsupported:
            reasons.add("UNSUPPORTED_LOOT_ROW_FIELDS")
    if re.search(r"table\.insert\s*\(\s*monster\.loot\b", text) or re.search(r"monster\.loot\s*\[", text):
        reasons.add("DYNAMIC_LOOT_MUTATION_UNSUPPORTED")
    state = "COMPLETE" if not reasons else ("PARTIAL" if result else "UNRESOLVED")
    return _reasoned(state, entries=result, reason_codes=sorted(reasons))


def _scalar_int(text: str, lhs: str) -> int | None:
    match = re.search(rf"\b{re.escape(lhs)}\s*=\s*(-?\d+)\b", text)
    return int(match.group(1)) if match else None


def _stats_profile(text: str) -> dict[str, Any]:
    health = _scalar_int(text, "monster.health")
    experience = _scalar_int(text, "monster.experience")
    speed = _scalar_int(text, "monster.speed")
    defense = armor = None
    defense_tables = _table_assignments(text, "monster.defenses")
    reasons: set[str] = set()
    if len(defense_tables) == 1 and defense_tables[0] is not None:
        fields, ok = _field_tokens(defense_tables[0])
        defense = _literal_int(fields.get("defense"))
        armor = _literal_int(fields.get("armor"))
        if not ok or defense is None or armor is None:
            reasons.add("INCOMPLETE_STATIC_STATS")
    elif defense_tables:
        reasons.add("INCOMPLETE_STATIC_STATS")
    if health is None and experience is None and speed is None and not defense_tables:
        return _reasoned("UNKNOWN", health=None, experience=None, armor=None, defense=None, speed=None, reason_codes=["NO_STATIC_STATS_EVIDENCE"])
    values = {"health": health, "experience": experience, "armor": armor, "defense": defense, "speed": speed}
    if any(value is None for value in values.values()):
        reasons.add("INCOMPLETE_STATIC_STATS")
    for key, value in values.items():
        if value is not None and value < 0:
            raise ExportError(f"{key} must be non-negative")
    result = {"state": "COMPLETE" if not reasons else "PARTIAL", **values}
    if reasons:
        result["reason_codes"] = sorted(reasons)
    return result


def _resistances_profile(text: str) -> dict[str, Any]:
    element_tables = _table_assignments(text, "monster.elements")
    immunity_tables = _table_assignments(text, "monster.immunities")
    if not element_tables and not immunity_tables:
        return _reasoned("UNKNOWN", elements=[], immunities=[], reason_codes=["NO_STATIC_RESISTANCE_EVIDENCE"])
    reasons: set[str] = set()
    elements: list[dict[str, Any]] = []
    immunities: list[str] = []
    if len(element_tables) != 1 or element_tables[0] is None:
        reasons.add("UNSUPPORTED_ELEMENTS_TABLE")
    else:
        rows, ok = _table_entries(element_tables[0])
        if not ok:
            reasons.add("UNSUPPORTED_ELEMENTS_TABLE")
        for row in rows:
            fields, fields_ok = _field_tokens(row)
            raw_type = fields.get("type")
            kind = ELEMENT_TYPES.get(raw_type or "")
            percent = _literal_int(fields.get("percent"))
            if not fields_ok or kind is None or percent is None:
                reasons.add("UNSUPPORTED_ELEMENT_ROW")
                continue
            elements.append({"type": kind, "percent": percent})
    if len(immunity_tables) != 1 or immunity_tables[0] is None:
        reasons.add("UNSUPPORTED_IMMUNITIES_TABLE")
    else:
        rows, ok = _table_entries(immunity_tables[0])
        if not ok:
            reasons.add("UNSUPPORTED_IMMUNITIES_TABLE")
        for row in rows:
            fields, fields_ok = _field_tokens(row)
            kind = _literal_string(fields.get("type"))
            condition = _literal_bool(fields.get("condition"))
            if not fields_ok or kind is None or condition is None:
                reasons.add("UNSUPPORTED_IMMUNITY_ROW")
                continue
            if condition:
                immunities.append(kind)
    if not element_tables or not immunity_tables:
        reasons.add("INCOMPLETE_RESISTANCE_TABLES")
    result = {"state": "COMPLETE" if not reasons else "PARTIAL", "elements": elements, "immunities": immunities}
    if reasons:
        result["reason_codes"] = sorted(reasons)
    return result


def _npc_profile(path: Path, items: dict[str, dict[str, Any]]) -> dict[str, Any] | None:
    raw = path.read_text(encoding="utf-8")
    text = _strip_line_comments(raw)
    match = NPC_NAME.search(text)
    if match is None:
        return None
    name = match.group(1)
    shop = _shop_profile(text, items)
    travel = _travel_profile(text)
    services = _services_profile(text, shop, travel)
    return {
        "entity_id": stable_creature_entity_id("npc", name.casefold()),
        "kind": "npc",
        "name": name,
        "shop": shop,
        "services": services,
        "travel": travel,
    }


def _monster_profile(path: Path, items: dict[str, dict[str, Any]]) -> dict[str, Any] | None:
    raw = path.read_text(encoding="utf-8")
    text = _strip_line_comments(raw)
    match = MONSTER_NAME.search(text)
    if match is None:
        return None
    name = match.group(1) or match.group(2)
    return {
        "entity_id": stable_creature_entity_id("monster", name.casefold()),
        "kind": "monster",
        "name": name,
        "loot": _loot_profile(text, items),
        "stats": _stats_profile(text),
        "resistances": _resistances_profile(text),
    }


def _merge_duplicate_profile(existing: dict[str, Any], candidate: dict[str, Any]) -> dict[str, Any]:
    if existing == candidate:
        return existing
    if existing["entity_id"] != candidate["entity_id"] or existing["kind"] != candidate["kind"]:
        raise ExportError("attempted to merge unrelated creature profiles")
    reason_codes = ["DUPLICATE_PROFILE_CONFLICT"]
    name = min((str(existing["name"]), str(candidate["name"])), key=lambda value: (value.casefold(), value))
    if existing["kind"] == "npc":
        return {
            "entity_id": existing["entity_id"],
            "kind": "npc",
            "name": name,
            "shop": {"state": "AMBIGUOUS", "sells": [], "buys": [], "reason_codes": reason_codes},
            "services": {"state": "AMBIGUOUS", "values": [], "reason_codes": reason_codes},
            "travel": {"state": "AMBIGUOUS", "destinations": [], "reason_codes": reason_codes},
        }
    return {
        "entity_id": existing["entity_id"],
        "kind": "monster",
        "name": name,
        "loot": {"state": "AMBIGUOUS", "entries": [], "reason_codes": reason_codes},
        "stats": {"state": "AMBIGUOUS", "health": None, "experience": None, "armor": None, "defense": None, "speed": None, "reason_codes": reason_codes},
        "resistances": {"state": "AMBIGUOUS", "elements": [], "immunities": [], "reason_codes": reason_codes},
    }


def export_gameplay_profiles(npc_root: Path, monster_root: Path) -> dict[str, Any]:
    npc_root = Path(npc_root)
    monster_root = Path(monster_root)
    items: dict[str, dict[str, Any]] = {}
    npc_profiles: dict[str, dict[str, Any]] = {}
    monster_profiles: dict[str, dict[str, Any]] = {}
    for path in sorted(npc_root.rglob("*.lua"), key=lambda value: value.relative_to(npc_root).as_posix()):
        profile = _npc_profile(path, items)
        if profile is None:
            continue
        entity_id = str(profile["entity_id"])
        current = npc_profiles.get(entity_id)
        npc_profiles[entity_id] = profile if current is None else _merge_duplicate_profile(current, profile)
    for path in sorted(monster_root.rglob("*.lua"), key=lambda value: value.relative_to(monster_root).as_posix()):
        profile = _monster_profile(path, items)
        if profile is None:
            continue
        entity_id = str(profile["entity_id"])
        current = monster_profiles.get(entity_id)
        monster_profiles[entity_id] = profile if current is None else _merge_duplicate_profile(current, profile)
    npcs = [npc_profiles[key] for key in sorted(npc_profiles)]
    monsters = [monster_profiles[key] for key in sorted(monster_profiles)]
    referenced_items = [items[key] for key in sorted(items)]
    return {
        "contract_id": CONTRACT_ID,
        "semantic_revision": 1,
        "capability": CAPABILITY,
        "profile_schema_version": PROFILE_SCHEMA_VERSION,
        "source_evidence": {"repository": "blakinio/Otheryn", "sha": LEGACY_EVIDENCE_SHA},
        "npcs": npcs,
        "monsters": monsters,
        "referenced_items": referenced_items,
        "statistics": {
            "npc_profiles": len(npcs),
            "monster_profiles": len(monsters),
            "referenced_items": len(referenced_items),
        },
    }


def main() -> int:
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("npc_root", type=Path)
    parser.add_argument("monster_root", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    product = export_gameplay_profiles(args.npc_root, args.monster_root)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(product, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(product["statistics"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())