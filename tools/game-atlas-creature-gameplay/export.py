#!/usr/bin/env python3
"""Deterministic, static-only Game -> Atlas creature gameplay profile extraction."""
from __future__ import annotations

import copy
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


def _lua_long_bracket_end(text: str, start: int) -> int | None:
    if start >= len(text) or text[start] != "[":
        return None
    cursor = start + 1
    while cursor < len(text) and text[cursor] == "=":
        cursor += 1
    if cursor >= len(text) or text[cursor] != "[":
        return None
    equals = text[start + 1 : cursor]
    closer = "]" + equals + "]"
    end = text.find(closer, cursor + 1)
    return len(text) if end < 0 else end + len(closer)


def _strip_line_comments(text: str) -> str:
    """Remove Lua comments and neutralize unsupported long strings without executing source."""
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
            long_end = _lua_long_bracket_end(text, i + 2)
            if long_end is not None:
                skipped = text[i:long_end]
                out.append("\n" * skipped.count("\n"))
                i = long_end
                continue
            i += 2
            while i < len(text) and text[i] not in "\r\n":
                i += 1
            continue
        long_end = _lua_long_bracket_end(text, i)
        if long_end is not None:
            skipped = text[i:long_end]
            out.append(" LONG_STRING_UNSUPPORTED ")
            out.append("\n" * skipped.count("\n"))
            i = long_end
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


LIMIT_PROFILE = "creature-gameplay-profiles-v1-e417-census-v1"
LIMITS = {
    "max_manifest_bytes": 256 * 1024,
    "max_shard_bytes": 512 * 1024,
    "max_profiles_per_shard": 32,
    "max_npc_profiles": 2048,
    "max_monster_profiles": 4096,
    "max_referenced_items": 4096,
    "max_shards": 513,
    "max_shop_sells_per_profile": 256,
    "max_shop_buys_per_profile": 2048,
    "max_shop_rows_per_profile": 2304,
    "max_loot_rows_per_profile": 128,
    "max_travel_destinations_per_profile": 16,
    "max_resistance_elements_per_profile": 16,
    "max_immunities_per_profile": 16,
    "max_string_bytes": 256,
    "max_nesting_depth": 12,
    "max_price": 100_000_000,
    "max_loot_count": 1024,
    "max_abs_resistance_percent": 2048,
}
STATES = {"COMPLETE", "PARTIAL", "UNRESOLVED", "AMBIGUOUS", "UNKNOWN", "NOT_APPLICABLE"}
ENTITY_ID_RE = re.compile(r"^(npc|monster)-entity:[0-9a-f]{32}$")
ITEM_REF_RE = re.compile(r"^oteryn:item\.[a-z0-9][a-z0-9._-]{0,127}$")
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True).encode("utf-8")


def _check_tree_limits(value: Any, path: str = "root", depth: int = 0) -> None:
    if depth > LIMITS["max_nesting_depth"]:
        raise ExportError(f"nesting limit exceeded at {path}")
    if isinstance(value, str):
        if len(value.encode("utf-8")) > LIMITS["max_string_bytes"]:
            raise ExportError(f"string limit exceeded at {path}")
        return
    if isinstance(value, dict):
        for key, child in value.items():
            if not isinstance(key, str):
                raise ExportError(f"non-string object key at {path}")
            _check_tree_limits(key, f"{path}.<key>", depth + 1)
            _check_tree_limits(child, f"{path}.{key}", depth + 1)
        return
    if isinstance(value, list):
        for index, child in enumerate(value):
            _check_tree_limits(child, f"{path}[{index}]", depth + 1)
        return
    if value is None or isinstance(value, (bool, int)):
        return
    raise ExportError(f"unsupported JSON value at {path}: {type(value).__name__}")


def _require_exact_keys(value: Any, required: set[str], optional: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ExportError(f"{label} must be an object")
    keys = set(value)
    if not required <= keys or not keys <= required | optional:
        raise ExportError(f"invalid keys for {label}: {sorted(keys)}")
    return value


def _require_state(section: dict[str, Any], label: str) -> str:
    state = section.get("state")
    if state not in STATES:
        raise ExportError(f"invalid completeness state for {label}: {state!r}")
    return str(state)


def _validate_reason_codes(section: dict[str, Any], label: str) -> None:
    reasons = section.get("reason_codes", [])
    if not isinstance(reasons, list) or any(not isinstance(value, str) or not value for value in reasons):
        raise ExportError(f"invalid reason_codes for {label}")
    if len(reasons) != len(set(reasons)):
        raise ExportError(f"duplicate reason code for {label}")


def _validate_item_relation(row: dict[str, Any], label: str, item_refs: set[str]) -> None:
    item_ref = row.get("item_ref")
    resolution = row.get("item_resolution_state")
    name = row.get("item_name")
    if not isinstance(name, str) or not name:
        raise ExportError(f"missing item name for {label}")
    if item_ref is None:
        if resolution == "RESOLVED":
            raise ExportError(f"resolved item row lacks stable item_ref for {label}")
    else:
        if not isinstance(item_ref, str) or ITEM_REF_RE.fullmatch(item_ref) is None:
            raise ExportError(f"invalid stable item_ref for {label}")
        if resolution != "RESOLVED" or item_ref not in item_refs:
            raise ExportError(f"unproven stable item_ref for {label}")
    if resolution not in {"RESOLVED", "UNRESOLVED", "AMBIGUOUS", "UNKNOWN"}:
        raise ExportError(f"invalid item resolution for {label}")


def _validate_npc_profile(profile: Any, item_refs: set[str]) -> None:
    p = _require_exact_keys(profile, {"entity_id", "kind", "name", "shop", "services", "travel"}, set(), "npc profile")
    if p.get("kind") != "npc" or not isinstance(p.get("name"), str) or not p["name"]:
        raise ExportError("invalid npc profile identity fields")
    entity_id = p.get("entity_id")
    if not isinstance(entity_id, str) or ENTITY_ID_RE.fullmatch(entity_id) is None or not entity_id.startswith("npc-entity:"):
        raise ExportError("invalid npc entity_id")

    shop = _require_exact_keys(p["shop"], {"state", "sells", "buys", "reason_codes"}, set(), "npc shop")
    _require_state(shop, "npc shop"); _validate_reason_codes(shop, "npc shop")
    sells, buys = shop["sells"], shop["buys"]
    if not isinstance(sells, list) or not isinstance(buys, list):
        raise ExportError("npc shop rows must be arrays")
    if len(sells) > LIMITS["max_shop_sells_per_profile"] or len(buys) > LIMITS["max_shop_buys_per_profile"] or len(sells) + len(buys) > LIMITS["max_shop_rows_per_profile"]:
        raise ExportError("npc shop row limit exceeded")
    for direction, rows in (("sells", sells), ("buys", buys)):
        for index, raw in enumerate(rows):
            row = _require_exact_keys(raw, {"item_ref", "item_name", "item_resolution_state", "unit_price", "currency"}, {"amount"}, f"npc {direction}[{index}]")
            _validate_item_relation(row, f"npc {direction}[{index}]", item_refs)
            price = row["unit_price"]
            if not isinstance(price, int) or isinstance(price, bool) or not 0 <= price <= LIMITS["max_price"]:
                raise ExportError("invalid npc shop price")
            if row["currency"] != "gold":
                raise ExportError("unsupported npc shop currency")
            if "amount" in row and (not isinstance(row["amount"], int) or isinstance(row["amount"], bool) or row["amount"] <= 0):
                raise ExportError("invalid npc shop amount")

    services = _require_exact_keys(p["services"], {"state", "values"}, {"reason_codes"}, "npc services")
    _require_state(services, "npc services"); _validate_reason_codes(services, "npc services")
    values = services["values"]
    if not isinstance(values, list) or any(value not in SERVICE_ORDER for value in values) or len(values) != len(set(values)):
        raise ExportError("invalid npc service values")

    travel = _require_exact_keys(p["travel"], {"state", "destinations", "reason_codes"}, set(), "npc travel")
    _require_state(travel, "npc travel"); _validate_reason_codes(travel, "npc travel")
    destinations = travel["destinations"]
    if not isinstance(destinations, list) or len(destinations) > LIMITS["max_travel_destinations_per_profile"]:
        raise ExportError("npc travel limit exceeded")
    for index, raw in enumerate(destinations):
        row = _require_exact_keys(raw, {"label"}, {"position", "price", "currency"}, f"travel[{index}]")
        if not isinstance(row["label"], str) or not row["label"]:
            raise ExportError("invalid travel label")
        if "position" in row:
            pos = _require_exact_keys(row["position"], {"x", "y", "floor"}, set(), "travel position")
            if any(not isinstance(pos[key], int) or isinstance(pos[key], bool) for key in ("x", "y", "floor")):
                raise ExportError("invalid travel position")
        if "price" in row and (not isinstance(row["price"], int) or isinstance(row["price"], bool) or not 0 <= row["price"] <= LIMITS["max_price"]):
            raise ExportError("invalid travel price")
        if "currency" in row and row["currency"] != "gold":
            raise ExportError("unsupported travel currency")


def _validate_monster_profile(profile: Any, item_refs: set[str]) -> None:
    p = _require_exact_keys(profile, {"entity_id", "kind", "name", "loot", "stats", "resistances"}, set(), "monster profile")
    if p.get("kind") != "monster" or not isinstance(p.get("name"), str) or not p["name"]:
        raise ExportError("invalid monster profile identity fields")
    entity_id = p.get("entity_id")
    if not isinstance(entity_id, str) or ENTITY_ID_RE.fullmatch(entity_id) is None or not entity_id.startswith("monster-entity:"):
        raise ExportError("invalid monster entity_id")

    loot = _require_exact_keys(p["loot"], {"state", "entries", "reason_codes"}, set(), "monster loot")
    _require_state(loot, "monster loot"); _validate_reason_codes(loot, "monster loot")
    entries = loot["entries"]
    if not isinstance(entries, list) or len(entries) > LIMITS["max_loot_rows_per_profile"]:
        raise ExportError("monster loot row limit exceeded")
    for index, raw in enumerate(entries):
        row = _require_exact_keys(raw, {"item_ref", "item_name", "item_resolution_state", "chance_ppm", "min_count", "max_count"}, set(), f"loot[{index}]")
        _validate_item_relation(row, f"loot[{index}]", item_refs)
        chance, minimum, maximum = row["chance_ppm"], row["min_count"], row["max_count"]
        if not isinstance(chance, int) or isinstance(chance, bool) or not 0 <= chance <= 1_000_000:
            raise ExportError("invalid loot chance_ppm")
        if any(not isinstance(value, int) or isinstance(value, bool) for value in (minimum, maximum)) or minimum < 0 or maximum < minimum or maximum > LIMITS["max_loot_count"]:
            raise ExportError("invalid loot count")

    stats = _require_exact_keys(p["stats"], {"state", "health", "experience", "armor", "defense", "speed"}, {"reason_codes"}, "monster stats")
    state = _require_state(stats, "monster stats"); _validate_reason_codes(stats, "monster stats")
    for key in ("health", "experience", "armor", "defense", "speed"):
        value = stats[key]
        if value is not None and (not isinstance(value, int) or isinstance(value, bool) or value < 0 or value > LIMITS["max_price"]):
            raise ExportError(f"invalid monster stat: {key}")
    if state == "COMPLETE" and any(stats[key] is None for key in ("health", "experience", "armor", "defense", "speed")):
        raise ExportError("complete monster stats contain null")

    resistances = _require_exact_keys(p["resistances"], {"state", "elements", "immunities"}, {"reason_codes"}, "monster resistances")
    _require_state(resistances, "monster resistances"); _validate_reason_codes(resistances, "monster resistances")
    elements, immunities = resistances["elements"], resistances["immunities"]
    if not isinstance(elements, list) or len(elements) > LIMITS["max_resistance_elements_per_profile"] or not isinstance(immunities, list) or len(immunities) > LIMITS["max_immunities_per_profile"]:
        raise ExportError("resistance/immunity row limit exceeded")
    for index, raw in enumerate(elements):
        row = _require_exact_keys(raw, {"type", "percent"}, set(), f"element[{index}]")
        if not isinstance(row["type"], str) or not row["type"] or not isinstance(row["percent"], int) or isinstance(row["percent"], bool) or abs(row["percent"]) > LIMITS["max_abs_resistance_percent"]:
            raise ExportError("invalid resistance element")
    if any(not isinstance(value, str) or not value for value in immunities) or len(immunities) != len(set(immunities)):
        raise ExportError("invalid immunity values")


def validate_normalized_product(product: Any) -> None:
    _check_tree_limits(product)
    p = _require_exact_keys(product, {"contract_id", "semantic_revision", "capability", "profile_schema_version", "source_evidence", "npcs", "monsters", "referenced_items", "statistics"}, set(), "normalized product")
    if p["contract_id"] != CONTRACT_ID or p["semantic_revision"] != 1 or p["capability"] != CAPABILITY or p["profile_schema_version"] != PROFILE_SCHEMA_VERSION:
        raise ExportError("normalized product contract mismatch")
    evidence = _require_exact_keys(p["source_evidence"], {"repository", "sha"}, set(), "source evidence")
    if evidence != {"repository": "blakinio/Otheryn", "sha": LEGACY_EVIDENCE_SHA}:
        raise ExportError("source evidence mismatch")
    npcs, monsters, items = p["npcs"], p["monsters"], p["referenced_items"]
    if not isinstance(npcs, list) or len(npcs) > LIMITS["max_npc_profiles"] or not isinstance(monsters, list) or len(monsters) > LIMITS["max_monster_profiles"] or not isinstance(items, list) or len(items) > LIMITS["max_referenced_items"]:
        raise ExportError("product record limit exceeded")
    item_refs: set[str] = set()
    for index, raw in enumerate(items):
        row = _require_exact_keys(raw, {"item_ref", "name", "resolution_state", "appearance_ref"}, {"reason_codes"}, f"referenced item[{index}]")
        ref = row["item_ref"]
        if not isinstance(ref, str) or ITEM_REF_RE.fullmatch(ref) is None or ref in item_refs:
            raise ExportError("invalid or duplicate referenced item identity")
        if row["resolution_state"] != "RESOLVED" or not isinstance(row["name"], str) or not row["name"]:
            raise ExportError("invalid referenced item record")
        item_refs.add(ref)
    entity_ids: set[str] = set()
    for row in npcs:
        _validate_npc_profile(row, item_refs)
        if row["entity_id"] in entity_ids:
            raise ExportError("duplicate creature entity profile")
        entity_ids.add(row["entity_id"])
    for row in monsters:
        _validate_monster_profile(row, item_refs)
        if row["entity_id"] in entity_ids:
            raise ExportError("duplicate creature entity profile")
        entity_ids.add(row["entity_id"])
    stats = _require_exact_keys(p["statistics"], {"npc_profiles", "monster_profiles", "referenced_items"}, set(), "statistics")
    expected = {"npc_profiles": len(npcs), "monster_profiles": len(monsters), "referenced_items": len(items)}
    if stats != expected:
        raise ExportError("statistics/count mismatch")


def _safe_shard_path(value: Any) -> str:
    if not isinstance(value, str) or not value.startswith("shards/") or "\\" in value or value.startswith("/") or "//" in value:
        raise ExportError("unsafe shard path")
    parts = value.split("/")
    if any(part in {"", ".", ".."} for part in parts) or not value.endswith(".json"):
        raise ExportError("unsafe shard path")
    return value


def _digest_bytes(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def compute_semantic_digest(manifest: dict[str, Any]) -> str:
    unsigned = copy.deepcopy(manifest)
    unsigned.pop("semantic_digest", None)
    return _digest_bytes(canonical_json_bytes(unsigned))


def _write_shard(root: Path, kind: str, key: str, payload: dict[str, Any], records: int) -> dict[str, Any]:
    path = f"shards/{kind}-{key}.json"
    data = canonical_json_bytes(payload)
    if len(data) > LIMITS["max_shard_bytes"]:
        raise ExportError(f"shard byte limit exceeded: {path}")
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
    return {"kind": kind, "key": key, "path": path, "bytes": len(data), "digest": _digest_bytes(data), "records": records}


def write_product(product: dict[str, Any], output_root: Path, producer_sha: str) -> dict[str, Any]:
    validate_normalized_product(product)
    if SHA_RE.fullmatch(producer_sha) is None:
        raise ExportError("producer SHA must be exact lowercase 40-hex")
    root = Path(output_root)
    if root.exists() and any(root.iterdir()):
        raise ExportError("output directory must be empty")
    root.mkdir(parents=True, exist_ok=True)
    shards: list[dict[str, Any]] = []
    for kind, records in (("npc", product["npcs"]), ("monster", product["monsters"])):
        groups: dict[str, list[dict[str, Any]]] = {}
        for record in sorted(records, key=lambda row: row["entity_id"]):
            key = str(record["entity_id"]).split(":", 1)[1][:2]
            groups.setdefault(key, []).append(copy.deepcopy(record))
        for key in sorted(groups):
            rows = groups[key]
            if len(rows) > LIMITS["max_profiles_per_shard"]:
                raise ExportError("profiles-per-shard limit exceeded")
            shards.append(_write_shard(root, kind, key, {"kind": kind, "key": key, "profiles": rows}, len(rows)))
    items = sorted((copy.deepcopy(row) for row in product["referenced_items"]), key=lambda row: row["item_ref"])
    if items:
        shards.append(_write_shard(root, "referenced-items", "all", {"kind": "referenced-items", "key": "all", "items": items}, len(items)))
    if len(shards) > LIMITS["max_shards"]:
        raise ExportError("shard count limit exceeded")
    shards.sort(key=lambda row: (row["kind"], row["key"]))
    manifest: dict[str, Any] = {
        "contract_id": CONTRACT_ID,
        "semantic_revision": 1,
        "capability": CAPABILITY,
        "profile_schema_version": PROFILE_SCHEMA_VERSION,
        "producer_repository_sha": producer_sha,
        "source_evidence": copy.deepcopy(product["source_evidence"]),
        "shard_key_rule": "entity-hash-prefix-2",
        "limit_profile": LIMIT_PROFILE,
        "limits": copy.deepcopy(LIMITS),
        "counts": {"npc_profiles": len(product["npcs"]), "monster_profiles": len(product["monsters"]), "referenced_items": len(items)},
        "shards": shards,
    }
    manifest["semantic_digest"] = compute_semantic_digest(manifest)
    manifest_bytes = canonical_json_bytes(manifest)
    if len(manifest_bytes) > LIMITS["max_manifest_bytes"]:
        raise ExportError("manifest byte limit exceeded")
    (root / "manifest.json").write_bytes(manifest_bytes)
    return manifest


def verify_product(output_root: Path) -> dict[str, Any]:
    root = Path(output_root)
    manifest_path = root / "manifest.json"
    if not manifest_path.is_file() or manifest_path.stat().st_size > LIMITS["max_manifest_bytes"]:
        raise ExportError("manifest missing or oversized")
    manifest_bytes = manifest_path.read_bytes()
    try:
        manifest = json.loads(manifest_bytes.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ExportError("manifest is not valid UTF-8 JSON") from exc
    if canonical_json_bytes(manifest) != manifest_bytes:
        raise ExportError("manifest is not canonical JSON")
    expected_keys = {"contract_id", "semantic_revision", "capability", "profile_schema_version", "producer_repository_sha", "source_evidence", "shard_key_rule", "limit_profile", "limits", "counts", "shards", "semantic_digest"}
    _require_exact_keys(manifest, expected_keys, set(), "manifest")
    if manifest["contract_id"] != CONTRACT_ID or manifest["semantic_revision"] != 1 or manifest["capability"] != CAPABILITY or manifest["profile_schema_version"] != PROFILE_SCHEMA_VERSION:
        raise ExportError("manifest contract mismatch")
    if not isinstance(manifest["producer_repository_sha"], str) or SHA_RE.fullmatch(manifest["producer_repository_sha"]) is None:
        raise ExportError("invalid producer SHA")
    if manifest["source_evidence"] != {"repository": "blakinio/Otheryn", "sha": LEGACY_EVIDENCE_SHA}:
        raise ExportError("manifest source evidence mismatch")
    if manifest["shard_key_rule"] != "entity-hash-prefix-2" or manifest["limit_profile"] != LIMIT_PROFILE or manifest["limits"] != LIMITS:
        raise ExportError("manifest producer profile mismatch")
    digest = manifest["semantic_digest"]
    if not isinstance(digest, str) or DIGEST_RE.fullmatch(digest) is None or digest != compute_semantic_digest(manifest):
        raise ExportError("manifest semantic digest mismatch")
    descriptors = manifest["shards"]
    if not isinstance(descriptors, list) or len(descriptors) > LIMITS["max_shards"]:
        raise ExportError("invalid shard descriptors")
    npcs: list[dict[str, Any]] = []
    monsters: list[dict[str, Any]] = []
    items: list[dict[str, Any]] = []
    seen_paths: set[str] = set()
    seen_slots: set[tuple[str, str]] = set()
    for descriptor in descriptors:
        desc = _require_exact_keys(descriptor, {"kind", "key", "path", "bytes", "digest", "records"}, set(), "shard descriptor")
        kind, key, path = desc["kind"], desc["key"], _safe_shard_path(desc["path"])
        if kind not in {"npc", "monster", "referenced-items"} or not isinstance(key, str) or not key:
            raise ExportError("invalid shard kind/key")
        if path in seen_paths or (kind, key) in seen_slots:
            raise ExportError("duplicate shard descriptor")
        seen_paths.add(path); seen_slots.add((kind, key))
        target = root / path
        if not target.is_file():
            raise ExportError("shard file missing")
        data = target.read_bytes()
        if not isinstance(desc["bytes"], int) or isinstance(desc["bytes"], bool) or desc["bytes"] != len(data) or len(data) > LIMITS["max_shard_bytes"]:
            raise ExportError("shard byte/count mismatch")
        if not isinstance(desc["digest"], str) or DIGEST_RE.fullmatch(desc["digest"]) is None or desc["digest"] != _digest_bytes(data):
            raise ExportError("shard digest mismatch")
        try:
            payload = json.loads(data.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise ExportError("shard is not valid UTF-8 JSON") from exc
        if canonical_json_bytes(payload) != data:
            raise ExportError("shard is not canonical JSON")
        if kind in {"npc", "monster"}:
            body = _require_exact_keys(payload, {"kind", "key", "profiles"}, set(), "profile shard")
            rows = body["profiles"]
            if body["kind"] != kind or body["key"] != key or not isinstance(rows, list) or len(rows) > LIMITS["max_profiles_per_shard"] or desc["records"] != len(rows):
                raise ExportError("profile shard metadata mismatch")
            for row in rows:
                entity_id = row.get("entity_id") if isinstance(row, dict) else None
                if not isinstance(entity_id, str) or entity_id.split(":", 1)[-1][:2] != key:
                    raise ExportError("profile stored in wrong shard")
            (npcs if kind == "npc" else monsters).extend(rows)
        else:
            body = _require_exact_keys(payload, {"kind", "key", "items"}, set(), "referenced item shard")
            rows = body["items"]
            if body["kind"] != "referenced-items" or key != "all" or body["key"] != "all" or not isinstance(rows, list) or desc["records"] != len(rows):
                raise ExportError("referenced item shard metadata mismatch")
            items.extend(rows)
    counts = manifest["counts"]
    _require_exact_keys(counts, {"npc_profiles", "monster_profiles", "referenced_items"}, set(), "manifest counts")
    product = {
        "contract_id": CONTRACT_ID,
        "semantic_revision": 1,
        "capability": CAPABILITY,
        "profile_schema_version": PROFILE_SCHEMA_VERSION,
        "source_evidence": copy.deepcopy(manifest["source_evidence"]),
        "npcs": sorted(npcs, key=lambda row: row["entity_id"]),
        "monsters": sorted(monsters, key=lambda row: row["entity_id"]),
        "referenced_items": sorted(items, key=lambda row: row["item_ref"]),
        "statistics": {"npc_profiles": len(npcs), "monster_profiles": len(monsters), "referenced_items": len(items)},
    }
    validate_normalized_product(product)
    if counts != product["statistics"]:
        raise ExportError("manifest count mismatch")
    return manifest

def main() -> int:
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("npc_root", type=Path)
    parser.add_argument("monster_root", type=Path)
    parser.add_argument("output_root", type=Path)
    parser.add_argument("--producer-sha", required=True)
    args = parser.parse_args()
    product = export_gameplay_profiles(args.npc_root, args.monster_root)
    manifest = write_product(product, args.output_root, args.producer_sha)
    verify_product(args.output_root)
    print(json.dumps({"semantic_digest": manifest["semantic_digest"], "limit_profile": manifest["limit_profile"], **manifest["counts"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())