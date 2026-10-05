"""Compact own-Source event preparation; fullcohort replay requires closed prerequisites."""

import hashlib
import json
import re
from functools import lru_cache
from pathlib import Path

BASE = Path("/dev/shm/oteryn-source-ability-catalog")
CPP = Path("/tmp/native-primary-classification-audit/source-code")
VARIANTS = json.loads((BASE / "proposal.json").read_bytes())["finite_event_variants"]
HANDLERS = json.loads((BASE / "own-handler-catalog.json").read_bytes())
FUNCTIONS = {
    "parseInvisible": "ASSIGN_INVISIBLE_BOOL",
    "parseSpeed": "ASSIGN_SPEED_I32",
    "parseSkills": "ASSIGN_SKILL_I32",
    "parseCriticalHit": "ASSIGN_SKILL_I32",
    "parseLifeAndManaLeech": "ASSIGN_SKILL_I32",
    "parseFieldAbsorbPercent": "ACCUMULATE_FIELD_ABSORB_I16",
    "parseAbsorbPercent": "ACCUMULATE_GENERAL_ABSORB_I16",
    "parseSupressDrunk": "CONDITIONAL_SUPPRESSION_OWN_ENUM",
    "parseElement": "ASSIGN_ELEMENT_U16_AND_OWN_TYPE",
    "parseSpecializedMagicLevelPoint": "ACCUMULATE_SPECIAL_MAGIC_I32_AND_ASSIGN_OWN_TYPE",
    "parseMagicShieldCapacity": "ACCUMULATE_MAGICSHIELD_I32",
    "parseCleavePercent": "ACCUMULATE_CLEAVE_I32",
    "parseMantra": "CRYSTAL_MANTRA_ACCUMULATE_I16_FOUR_OWN_ELEMENTS",
    "parseElementalBond": "CRYSTAL_ELEMENTAL_BOND_ASSIGN_OWN_TYPE",
}


def variant(handler, key):
    if handler == "parseHealthAndMana":
        return (
            "ASSIGN_MANASHIELD_BOOL"
            if key == "manashield"
            else "ASSIGN_REGEN_RAW_U32_AND_ENABLE"
        )
    if handler in ("parseMaxHitAndManaPoints", "parseMagicLevelPoint"):
        return (
            "ASSIGN_STAT_PERCENT_I32" if key.endswith("percent") else "ASSIGN_STAT_I32"
        )
    if handler == "parsePerfecShot":
        return (
            "ASSIGN_PERFECT_RANGE_I16_TO_U8"
            if key.endswith("range")
            else "ASSIGN_PERFECT_DAMAGE_I32"
        )
    if handler == "parseReflectDamage":
        return (
            "ACCUMULATE_REFLECT_ALL_I32_TO_I16"
            if key == "reflectpercentall"
            else "ACCUMULATE_REFLECT_FLAT_I32"
        )
    return FUNCTIONS[handler]


def integer(lexeme, bits, signed):
    # Only successful full-token from_chars paths. Error/logError paths held.
    if not re.fullmatch(r"-?[0-9]+" if signed else r"[0-9]+", lexeme):
        raise ValueError("HOLD non-full-token integer path")
    value = int(lexeme)
    low = -(1 << (bits - 1)) if signed else 0
    high = (1 << (bits - (1 if signed else 0))) - 1
    if not low <= value <= high:
        raise ValueError("HOLD out-of-range/logError path")
    return value


def boolean(lexeme):
    return bool(lexeme) and lexeme[0] in "1tTyY"


def parsed(kind, lexeme):
    if kind == "CRYSTAL_ELEMENTAL_BOND_ASSIGN_OWN_TYPE":
        return lexeme
    if kind.endswith("_BOOL") or kind == "CONDITIONAL_SUPPRESSION_OWN_ENUM":
        return boolean(lexeme)
    if kind in ("ASSIGN_REGEN_RAW_U32_AND_ENABLE", "ASSIGN_ELEMENT_U16_AND_OWN_TYPE"):
        return integer(lexeme, 32 if "REGEN" in kind else 16, False)
    return integer(
        lexeme,
        16 if ("I16" in kind and "I32_TO_I16" not in kind) or "MANTRA" in kind else 32,
        True,
    )


def source_body(text, name):
    start = re.search(r"void ItemParse::" + name + r"\([^\n]*\)\s*\{", text).end()
    cursor, depth = start, 1
    while depth:
        depth += (text[cursor] == "{") - (text[cursor] == "}")
        cursor += 1
    return text[start : cursor - 1]


@lru_cache(maxsize=3)
def table(cut):
    text = (CPP / cut / "item_parse.cpp").read_text()
    body = source_body(text, "initParse")
    order = re.findall(r"ItemParse::(\w+)\(", body)
    rows = {}
    for descriptor in HANDLERS[cut]["abilities_handlers"]:
        name = descriptor["handler"]
        actual = source_body(text, name)
        assert (
            hashlib.sha256(actual.encode()).hexdigest() == descriptor["own_body_sha256"]
        )
        rows[name] = {**descriptor, "order": order.index(name)}
    return rows


def emit(cut, assignments, normal_return=False):
    """Fixture/raw-row emitter; caller must separately qualify own eligibility/writer phase."""
    if not normal_return:
        raise ValueError(
            "HOLD full initParse normal-return/interprocedural writer closure"
        )
    handlers = table(cut)
    registered = set(HANDLERS[cut]["all_recognized_dispatch_keys"])
    allocation, events, previous = None, [], -1
    for assignment in assignments:
        if set(assignment) != {
            "attribute_ordinal",
            "key",
            "value_lexeme",
            "raw_attribute_sha256",
        }:
            raise ValueError("closed direct-attribute evidence required")
        ordinal, key, lexeme = (
            assignment["attribute_ordinal"],
            assignment["key"],
            assignment["value_lexeme"],
        )
        if (
            type(ordinal) is not int
            or not 0 <= ordinal <= 65535
            or not re.fullmatch(r"[0-9a-f]{64}", assignment["raw_attribute_sha256"])
        ):
            raise ValueError("own ordinal/rawattribute hash malformed")
        if ordinal <= previous:
            raise ValueError("ordered direct attributes required")
        previous = ordinal
        if key not in registered:
            continue
        for name, descriptor in sorted(
            handlers.items(), key=lambda pair: pair[1]["order"]
        ):
            matched = key in descriptor["reachable_keys"]
            truthy_suppression = name == "parseSupressDrunk" and boolean(lexeme)
            allocates = (
                descriptor["unconditional_lazy_allocation_before_key_match"]
                or matched
                or truthy_suppression
            )
            # A false suppression branch does not allocate; later handlers do.
            if name == "parseSupressDrunk":
                allocates = truthy_suppression
            if allocates and allocation is None:
                allocation = {
                    "attribute_ordinal": ordinal,
                    "handler": name,
                    "handler_order": descriptor["order"],
                    "key": key,
                    "raw_attribute_sha256": assignment["raw_attribute_sha256"],
                }
            if matched or truthy_suppression:
                kind = variant(name, key)
                value = parsed(kind, lexeme)
                events.append(
                    {
                        **assignment,
                        "kind": kind,
                        "handler": name,
                        "handler_order": descriptor["order"],
                        "parsed_own_value": value,
                        "own_body_sha256": descriptor["own_body_sha256"],
                        "suppression_default_NONE": truthy_suppression and not matched,
                    }
                )
        if key in HANDLERS[cut]["aliases"]:
            events.append(
                {
                    **assignment,
                    "kind": "RECOGNIZED_ALIAS_NOOP_ALLOCATION_EVENT",
                    "handler": "DISPATCH_ALIAS_NO_SETTER",
                    "handler_order": 65535,
                    "parsed_own_value": None,
                    "own_body_sha256": None,
                    "suppression_default_NONE": False,
                }
            )
    return {
        "source_cut": cut,
        "default_catalog_sha256": hashlib.sha256(
            (BASE / (cut + "-default-catalog.json")).read_bytes()
        ).hexdigest(),
        "allocation": {"state": "ALLOCATED", "first_getter": allocation}
        if allocation
        else {"state": "ABSENT_STORED_OBJECT"},
        "ordered_events": events,
        "final_sparse_members": {"state": "UNKNOWN"},
        "runtime_getter_rates": {"state": "UNKNOWN"},
        "Lua_instance_effects": {"state": "UNKNOWN"},
    }


def require_full_replay(receipt, own_receipt_sha256, heavy_release=False):
    """Checks happen BEFORE opening a cohort input. No bypass switches for Source laws."""
    if not heavy_release:
        raise ValueError("HOLD Root exclusive heavy-slot release required")
    raw = receipt.read_bytes()
    if hashlib.sha256(raw).hexdigest() != own_receipt_sha256:
        raise ValueError("unsealed own Source closure")
    proof = json.loads(raw)
    required = (
        "all_source_phase_writers_closed",
        "all_initParse_normal_returns_closed",
        "loaded_eligibility45721_exact",
        "all_nested_callbacks_closed",
        "all_arithmetic_paths_qualified",
        "all_event_maxima_and_budget_censused",
    )
    if not all(proof.get(key) is True for key in required):
        raise ValueError("HOLD missing complete Source prerequisites")
    return proof
