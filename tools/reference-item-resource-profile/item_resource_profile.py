#!/usr/bin/env python3
"""Executable typed Item body codec candidate for D6-M1 measurement only.

No donor value is promoted. Every admitted v1 field has a closed codec; fields whose
typed unit or grammar is not accepted are rejected and listed as explicit unsupported.
"""

from __future__ import annotations

import hashlib
import argparse
import json
import math
import struct
import sys
import xml.etree.ElementTree as ET
from collections import Counter
from dataclasses import dataclass, fields, replace
from enum import IntEnum
from pathlib import Path
from typing import Any, Callable


RESISTANCE_KEYS = (
    "modifier.absorbpercentdeath", "modifier.absorbpercentdrown", "modifier.absorbpercentearth",
    "modifier.absorbpercentenergy", "modifier.absorbpercentfire", "modifier.absorbpercentholy",
    "modifier.absorbpercentice", "modifier.absorbpercentlifedrain", "modifier.absorbpercentmanadrain",
    "modifier.absorbpercentphysical", "modifier.absorbpercentpoison", "modifier.fieldabsorbpercentfire",
)
MODIFIER_KEYS = (
    "elemental_bond", "invisibility", "mana_shield", "mantra", "modifier.cleavepercent",
    "modifier.criticalhitchance", "modifier.criticalhitdamage", "modifier.deathmagiclevelpoints",
    "modifier.earthmagiclevelpoints", "modifier.elementdeath", "modifier.elementearth",
    "modifier.elementenergy", "modifier.elementfire", "modifier.elementice",
    "modifier.energymagiclevelpoints", "modifier.firemagiclevelpoints",
    "modifier.healingmagiclevelpoints", "modifier.healthgain", "modifier.healthticks",
    "modifier.holymagiclevelpoints", "modifier.icemagiclevelpoints", "modifier.lifeleechamount",
    "modifier.lifeleechchance", "modifier.magiclevelpoints", "modifier.magicshieldcapacityflat",
    "modifier.magicshieldcapacitypercent", "modifier.managain", "modifier.manaleechamount",
    "modifier.manaleechchance", "modifier.manaticks", "modifier.perfectshotdamage",
    "modifier.perfectshotrange", "modifier.reflectdamage", "modifier.skillaxe", "modifier.skillclub",
    "modifier.skilldist", "modifier.skillfist", "modifier.skillshield", "modifier.skillsword",
    "modifier.speed", "suppress_drown", "suppress_drunk",
)
assert len(RESISTANCE_KEYS) == 12 and len(MODIFIER_KEYS) == 42
WEAPON_ELEMENT_KEYS = (
    "modifier.elementdeath", "modifier.elementearth", "modifier.elementenergy",
    "modifier.elementfire", "modifier.elementice",
)
SKILL_MODIFIER_KEYS = tuple(key for key in MODIFIER_KEYS if key not in WEAPON_ELEMENT_KEYS)
MODIFIER_TARGET_CODEC_KEYS = tuple(f"modifier-target-slot-{index:02d}" for index in range(1, 38))
MODIFIER_PHASE_CODEC_KEYS = tuple(f"modifier-phase-slot-{index:02d}" for index in range(1, 38))
BOOLEAN_MODIFIER_KEYS = ("invisibility", "mana_shield", "suppress_drown", "suppress_drunk")
assert len(WEAPON_ELEMENT_KEYS) == 5 and len(SKILL_MODIFIER_KEYS) == 37
CLASSIFICATION_KEYS = (
    "weapon", "armor", "helmet", "legs", "boots", "shield", "ammo", "rune", "container",
    "consumable", "currency", "material", "loot", "decoration", "key", "book_readable", "fluid",
    "usable", "transformable", "stackable", "charge_based", "imbueable", "presentation_only", "other",
)
ITEM_TYPE_CANDIDATE_KEYS = (
    "bed", "carpet", "container", "depot", "door", "dummy", "key", "ladder", "magicfield",
    "mailbox", "rewardchest", "rune", "teleport", "trashholder",
)
WEAPON_TYPE_CANDIDATE_KEYS = ("ammunition", "axe", "club", "distance", "fist", "shield", "spellbook", "sword", "wand")
AMMO_TYPE_CANDIDATE_KEYS = ("arrow", "bolt")
FLUID_TYPE_CANDIDATE_KEYS = ("beer", "blood", "lemonade", "mud", "rum", "slime", "water", "wine")
IMBUEMENT_FAMILY_CANDIDATE_KEYS = (
    "critical hit", "elemental damage", "elemental protection death", "elemental protection earth",
    "elemental protection energy", "elemental protection fire", "elemental protection holy",
    "elemental protection ice", "increase capacity", "increase speed", "life leech", "mana leech",
    "paralysis removal", "skillboost axe", "skillboost club", "skillboost distance", "skillboost fist",
    "skillboost magic level", "skillboost shielding", "skillboost sword",
)
EQUIPMENT_SLOT_KEYS = ("HEAD", "TORSO", "LEGS", "FEET", "WEAPON", "SHIELD", "AMULET", "RING", "CONTAINER", "EXTRA")
BASE_VOCATION_KEYS = ("DRUID", "KNIGHT", "MONK", "PALADIN", "SORCERER")
assert len(CLASSIFICATION_KEYS) == 24 and len(IMBUEMENT_FAMILY_CANDIDATE_KEYS) == 20

RATIONAL_PERCENT_MODIFIERS = frozenset({
    "modifier.cleavepercent", "modifier.criticalhitchance", "modifier.criticalhitdamage",
    "modifier.lifeleechamount", "modifier.lifeleechchance",
    "modifier.magicshieldcapacitypercent", "modifier.manaleechamount",
    "modifier.manaleechchance",
})
MILLISECOND_MODIFIERS = frozenset({"modifier.healthticks", "modifier.manaticks"})
CELL_MODIFIERS = frozenset({"modifier.perfectshotrange"})
ELEMENT_MODIFIERS = frozenset({"elemental_bond"})


def modifier_parameter_shape(key: str) -> str:
    if key in BOOLEAN_MODIFIER_KEYS:
        return "BOOLEAN"
    if key in RATIONAL_PERCENT_MODIFIERS:
        return "EXACT_RATIONAL_PERCENT"
    if key in MILLISECOND_MODIFIERS:
        return "MILLISECONDS"
    if key in CELL_MODIFIERS:
        return "CELLS"
    if key in ELEMENT_MODIFIERS:
        return "ELEMENT_KIND"
    return "SIGNED_POINTS"


class Tag(IntEnum):
    UNKNOWN = 0
    NOT_APPLICABLE = 1
    CONFLICT = 2
    KNOWN = 3


@dataclass(frozen=True)
class State:
    tag: Tag
    value: Any = None

    def __post_init__(self) -> None:
        if (self.tag == Tag.KNOWN) != (self.value is not None):
            raise ValueError("KNOWN requires value; other states forbid value")


U = State(Tag.UNKNOWN)
NA = State(Tag.NOT_APPLICABLE)
C = State(Tag.CONFLICT)


def K(value: Any) -> State:
    return State(Tag.KNOWN, value)


@dataclass(frozen=True)
class Presentation:
    name: State
    description: State
    # appearance binding, aliases and tags are EXPLICIT_UNSUPPORTED_IN_V1.


@dataclass(frozen=True)
class Classification:
    item_class: State
    capabilities: State


@dataclass(frozen=True)
class Physical:
    weight: State
    movable: State
    pickupable: State


@dataclass(frozen=True)
class Stack:
    stackable: State
    stack_max: State


@dataclass(frozen=True)
class Equipment:
    patterns: State


@dataclass(frozen=True)
class EquipmentPattern:
    pattern_id: int
    primary_slot: State
    additional_reserved_slots: State
    mutually_exclusive_groups: State
    vocations: State
    level: State
    compatibility_rule: State


@dataclass(frozen=True)
class Weapon:
    weapon_type: State
    attack: State
    defense: State
    extra_defense: State
    range_cells: State
    hit_chance: State
    max_hit_chance: State
    ammunition: State
    elemental: State


@dataclass(frozen=True)
class Protection:
    armor: State
    resistances: State


@dataclass(frozen=True)
class SkillModifiers:
    modifiers: State


@dataclass(frozen=True)
class BoolFlag:
    value: bool


@dataclass(frozen=True)
class SignedPoints:
    value: int


@dataclass(frozen=True)
class Cells:
    value: int


@dataclass(frozen=True)
class Milliseconds:
    value: int


@dataclass(frozen=True)
class RationalPercent:
    numerator: int
    denominator: int


def validate_rational_percent(value: RationalPercent) -> None:
    if not isinstance(value, RationalPercent):
        raise ValueError("rational percent")
    if (not isinstance(value.numerator, int) or isinstance(value.numerator, bool)
            or not -(1 << 63) <= value.numerator < (1 << 63)):
        raise ValueError("rational numerator")
    if (not isinstance(value.denominator, int) or isinstance(value.denominator, bool)
            or not 1 <= value.denominator <= (1 << 64) - 1):
        raise ValueError("rational denominator")
    if math.gcd(abs(value.numerator), value.denominator) != 1:
        raise ValueError("noncanonical rational")


@dataclass(frozen=True)
class ElementKind:
    value: int


@dataclass(frozen=True)
class ModifierBinding:
    kind: int
    target_domain: State
    evaluation_phase: State
    priority: State
    parameter: State


@dataclass(frozen=True)
class Charges:
    count: State


@dataclass(frozen=True)
class Temporal:
    consumption_mode: State
    duration_ms: State
    stop_duration: State
    decay_target_ordinal: State


@dataclass(frozen=True)
class Container:
    capacity: State


@dataclass(frozen=True)
class Imbuement:
    slot_count: State
    allowed_family_tiers: State
    excluded_families: State


@dataclass(frozen=True)
class UseTransform:
    # Ten fixed kinds: rotate, wrap, use, equip, deequip, male, female,
    # destroy, decay and write-once. Each value is State<u32 ordinal>.
    targets: tuple[State, ...]


@dataclass(frozen=True)
class TradeRestrictions:
    tradeable: State
    marketable: State
    vocations: State
    account_binding_policy: State
    character_binding_policy: State


@dataclass(frozen=True)
class Fluid:
    fluid_type: State


@dataclass(frozen=True)
class ReadableWriteable:
    readable: State
    writeable: State
    distance_read: State
    max_text_length: State
    write_once_target_ordinal: State


@dataclass(frozen=True)
class RetainedItemCore:
    # Existing protected ReferenceItemDefinition semantics. Numeric values mirror
    # the v1-v3 codec: physical 1/unknown 2; stack nonstack 1/stack 2/unknown 3.
    physical_class: int
    materializable: State
    stack_class: int
    legal_destinations: State


@dataclass(frozen=True)
class Item:
    core: RetainedItemCore
    presentation: State = U
    classification: State = U
    physical: State = U
    stack: State = U
    equipment: State = U
    weapon: State = U
    protection: State = U
    skill_modifiers: State = U
    charges: State = U
    temporal: State = U
    container: State = U
    imbuement: State = U
    use_transform: State = U
    trade_restrictions: State = U
    fluid: State = U
    readable_writeable: State = U


@dataclass(frozen=True)
class Bounds:
    name_bytes: int = 46
    description_bytes: int = 200
    capabilities: int = 24
    equipment_patterns: int = 2  # max(corpus normalized max 1, minimum non-singleton 2)
    equipment_slots: int = 10
    equipment_additional_slots: int = 9
    equipment_exclusive_groups: int = 2  # max(explicit corpus max 0, minimum plural witness 2)
    equipment_group_key_bytes: int = 512
    equipment_vocations: int = 5
    weapon_elements: int = 5  # full pinned elemental field vocabulary
    resistances: int = 12  # full pinned absorb/field-absorb vocabulary
    resistance_kind_domain: int = 12
    modifiers: int = 37
    modifier_kind_domain: int = 37
    imbuement_families: int = 20
    imbuement_entries: int = 20
    imbuement_slot_count: int = 3
    trade_vocations: int = 5
    item_count: int = 38_157


class Reader:
    def __init__(self, data: bytes):
        self.data = data
        self.pos = 0

    def take(self, n: int) -> bytes:
        if n < 0 or self.pos + n > len(self.data):
            raise ValueError("truncated")
        out = self.data[self.pos:self.pos + n]
        self.pos += n
        return out

    def u8(self) -> int:
        return self.take(1)[0]

    def u16(self) -> int:
        return struct.unpack(">H", self.take(2))[0]

    def finish(self) -> None:
        if self.pos != len(self.data):
            raise ValueError("trailing bytes")


def enc_state(state: State, encoder: Callable[[Any], bytes]) -> bytes:
    if state.tag != Tag.KNOWN:
        return bytes([state.tag])
    return bytes([Tag.KNOWN]) + encoder(state.value)


def dec_state(reader: Reader, decoder: Callable[[Reader], Any]) -> State:
    try:
        tag = Tag(reader.u8())
    except ValueError as exc:
        raise ValueError("invalid state tag") from exc
    return K(decoder(reader)) if tag == Tag.KNOWN else State(tag)


def fixed(fmt: str, minimum: int, maximum: int) -> tuple[Callable[[int], bytes], Callable[[Reader], int]]:
    size = struct.calcsize(fmt)
    def enc(value: int) -> bytes:
        if not isinstance(value, int) or isinstance(value, bool) or not minimum <= value <= maximum:
            raise ValueError("integer domain")
        return struct.pack(fmt, value)
    def dec(reader: Reader) -> int:
        value = struct.unpack(fmt, reader.take(size))[0]
        if not minimum <= value <= maximum:
            raise ValueError("integer domain")
        return value
    return enc, dec


EU8, DU8 = fixed(">B", 0, 255)
EU16, DU16 = fixed(">H", 0, 65_535)
EI16, DI16 = fixed(">h", -32_768, 32_767)
EU32, DU32 = fixed(">I", 0, 0xFFFFFFFF)
EI32, DI32 = fixed(">i", -0x80000000, 0x7FFFFFFF)
EU64, DU64 = fixed(">Q", 0, 0xFFFFFFFFFFFFFFFF)


def ebool(value: bool) -> bytes:
    if not isinstance(value, bool):
        raise ValueError("bool domain")
    return bytes([int(value)])


def dbool(reader: Reader) -> bool:
    value = reader.u8()
    if value not in (0, 1):
        raise ValueError("bool domain")
    return bool(value)


def enum_codec(maximum: int):
    def enc(value: int) -> bytes:
        if not isinstance(value, int) or isinstance(value, bool) or not 1 <= value <= maximum:
            raise ValueError("enum domain")
        return bytes([value])
    def dec(reader: Reader) -> int:
        value = reader.u8()
        if not 1 <= value <= maximum:
            raise ValueError("enum domain")
        return value
    return enc, dec


def enum_values_codec(values: tuple[int, ...]):
    admitted = frozenset(values)
    def enc(value: int) -> bytes:
        if not isinstance(value, int) or isinstance(value, bool) or value not in admitted:
            raise ValueError("sparse enum domain")
        return bytes([value])
    def dec(reader: Reader) -> int:
        value = reader.u8()
        if value not in admitted:
            raise ValueError("sparse enum domain")
        return value
    return enc, dec


def unsupported_codec(label: str):
    def enc(_: Any) -> bytes:
        raise ValueError(f"{label} explicit unsupported")
    def dec(_: Reader) -> Any:
        raise ValueError(f"{label} explicit unsupported")
    return enc, dec


def text_codec(max_bytes: int):
    def enc(value: str) -> bytes:
        if not isinstance(value, str):
            raise ValueError("text type")
        raw = value.encode("utf-8")
        if len(raw) > max_bytes:
            raise ValueError("text max+1")
        return struct.pack(">H", len(raw)) + raw
    def dec(reader: Reader) -> str:
        size = reader.u16()
        if size > max_bytes:  # fail before payload allocation/copy
            raise ValueError("text max+1")
        return reader.take(size).decode("utf-8")
    return enc, dec


def production_key_codec(max_bytes: int):
    def valid(value: str) -> bool:
        lower = value.lower()
        segments = lower.replace(":", ".").replace("/", ".").replace("_", ".").replace("-", ".").split(".")
        nonproduction = "fixture" in lower or "synthetic" in lower or "evidence" in lower or "test-only" in lower or "test" in segments
        namespaced = ":" in value and all(value.split(":", 1))
        return (
            isinstance(value, str) and namespaced and value.isascii() and 0 < len(value.encode()) <= max_bytes
            and all(char.isalnum() or char in ":._-/" for char in value) and not nonproduction
        )
    def enc(value: str) -> bytes:
        if not valid(value): raise ValueError("production key")
        raw = value.encode("ascii")
        return struct.pack(">H", len(raw)) + raw
    def dec(reader: Reader) -> str:
        size = reader.u16()
        if size > max_bytes: raise ValueError("production key max+1")
        value = reader.take(size).decode("ascii")
        if not valid(value): raise ValueError("production key")
        return value
    return enc, dec


def tuple_codec(max_count: int, enc_entry, dec_entry, key=lambda value: value):
    def validate(values):
        if not isinstance(values, tuple) or len(values) > max_count:
            raise ValueError("vector max+1")
        keys = [key(value) for value in values]
        if keys != sorted(keys) or len(set(keys)) != len(keys):
            raise ValueError("vector duplicate/order")
    def enc(values: tuple) -> bytes:
        validate(values)  # before output allocation
        return bytes([len(values)]) + b"".join(enc_entry(value) for value in values)
    def dec(reader: Reader) -> tuple:
        count = reader.u8()
        if count > max_count:  # before entry allocation
            raise ValueError("vector max+1")
        values = tuple(dec_entry(reader) for _ in range(count))
        validate(values)
        return values
    return enc, dec


def pair_entry(enc_key, dec_key, enc_value, dec_value):
    return (
        lambda pair: enc_key(pair[0]) + enc_state(pair[1], enc_value),
        lambda reader: (dec_key(reader), dec_state(reader, dec_value)),
    )


def codec(bounds: Bounds):
    EName, DName = text_codec(bounds.name_bytes)
    EDesc, DDesc = text_codec(bounds.description_bytes)
    EClass, DClass = enum_codec(len(ITEM_TYPE_CANDIDATE_KEYS))
    ESlot, DSlot = enum_codec(bounds.equipment_slots)
    EGroupKey, DGroupKey = production_key_codec(bounds.equipment_group_key_bytes)
    ECompatibility, DCompatibility = unsupported_codec("equipment compatibility rule grammar")
    EAccountBinding, DAccountBinding = unsupported_codec("immutable account binding policy")
    ECharacterBinding, DCharacterBinding = unsupported_codec("immutable character binding policy")
    EWeapon, DWeapon = enum_codec(len(WEAPON_TYPE_CANDIDATE_KEYS))
    EAmmo, DAmmo = enum_codec(len(AMMO_TYPE_CANDIDATE_KEYS))
    EFluid, DFluid = enum_codec(len(FLUID_TYPE_CANDIDATE_KEYS))
    ETemporalMode, DTemporalMode = enum_codec(2)
    EPhysicalClass, DPhysicalClass = enum_codec(2)
    EStackClass, DStackClass = enum_codec(3)
    EDestination, DDestination = enum_codec(1)
    EVocation, DVocation = enum_codec(len(BASE_VOCATION_KEYS))
    ECapability, DCapability = enum_codec(bounds.capabilities)
    EElement, DElement = enum_codec(bounds.weapon_elements)
    EResistance, DResistance = enum_codec(bounds.resistance_kind_domain)
    EModifier, DModifier = enum_codec(bounds.modifier_kind_domain)
    EImbue, DImbue = enum_codec(bounds.imbuement_families)

    def e_signed_points(value: SignedPoints) -> bytes:
        if not isinstance(value, SignedPoints): raise ValueError("signed points")
        return EI32(value.value)

    def d_signed_points(reader: Reader) -> SignedPoints:
        return SignedPoints(DI32(reader))

    def e_cells(value: Cells) -> bytes:
        if not isinstance(value, Cells): raise ValueError("cells")
        return EU16(value.value)

    def d_cells(reader: Reader) -> Cells:
        return Cells(DU16(reader))

    def e_rational_percent(value: RationalPercent) -> bytes:
        validate_rational_percent(value)
        return struct.pack(">qQ", value.numerator, value.denominator)

    def d_rational_percent(reader: Reader) -> RationalPercent:
        numerator, denominator = struct.unpack(">qQ", reader.take(16))
        value = RationalPercent(numerator, denominator)
        validate_rational_percent(value)
        return value

    def EOrdinal(value: int) -> bytes:
        if not isinstance(value, int) or not 0 <= value < bounds.item_count:
            raise ValueError("Item registry ordinal")
        return EU32(value)

    def DOrdinal(reader: Reader) -> int:
        value = DU32(reader)
        if value >= bounds.item_count:
            raise ValueError("Item registry ordinal")
        return value

    def e_capabilities(values: tuple[State, ...]) -> bytes:
        if not isinstance(values, tuple) or len(values) != bounds.capabilities:
            raise ValueError("exact capability truth-state vector required")
        return b"".join(enc_state(value, ebool) for value in values)

    def d_capabilities(reader: Reader) -> tuple[State, ...]:
        return tuple(dec_state(reader, dbool) for _ in range(bounds.capabilities))

    caps = (e_capabilities, d_capabilities)
    vocs = tuple_codec(bounds.equipment_vocations, EVocation, DVocation)
    reserved_slots = tuple_codec(bounds.equipment_additional_slots, ESlot, DSlot)
    exclusive_groups = tuple_codec(bounds.equipment_exclusive_groups, EGroupKey, DGroupKey)
    trade_vocs = tuple_codec(bounds.trade_vocations, EVocation, DVocation)
    destinations = tuple_codec(1, EDestination, DDestination)
    e_pair, d_pair = pair_entry(EElement, DElement, e_signed_points, d_signed_points)
    elements = tuple_codec(bounds.weapon_elements, e_pair, d_pair, key=lambda p: p[0])
    r_pair, rd_pair = pair_entry(EResistance, DResistance, e_rational_percent, d_rational_percent)
    resistances = tuple_codec(bounds.resistances, r_pair, rd_pair, key=lambda p: p[0])
    ETarget, DTarget = enum_codec(bounds.modifier_kind_domain)
    EPhase, DPhase = enum_codec(bounds.modifier_kind_domain)
    EElementKind, DElementKind = enum_codec(6)

    def e_modifier_parameter(kind: int, value: Any) -> bytes:
        shape = modifier_parameter_shape(SKILL_MODIFIER_KEYS[kind - 1])
        if shape == "BOOLEAN" and isinstance(value, BoolFlag):
            return ebool(value.value)
        if shape == "SIGNED_POINTS" and isinstance(value, SignedPoints):
            return EI32(value.value)
        if shape == "CELLS" and isinstance(value, Cells):
            return EU16(value.value)
        if shape == "MILLISECONDS" and isinstance(value, Milliseconds):
            return EU64(value.value)
        if shape == "EXACT_RATIONAL_PERCENT" and isinstance(value, RationalPercent):
            if value.denominator == 0:
                raise ValueError("rational denominator")
            return e_rational_percent(value)
        if shape == "ELEMENT_KIND" and isinstance(value, ElementKind):
            return EElementKind(value.value)
        raise ValueError("modifier parameter variant")

    def d_modifier_parameter(kind: int, reader: Reader) -> Any:
        shape = modifier_parameter_shape(SKILL_MODIFIER_KEYS[kind - 1])
        if shape == "BOOLEAN": return BoolFlag(dbool(reader))
        if shape == "SIGNED_POINTS": return SignedPoints(DI32(reader))
        if shape == "CELLS": return Cells(DU16(reader))
        if shape == "MILLISECONDS": return Milliseconds(DU64(reader))
        if shape == "EXACT_RATIONAL_PERCENT":
            return d_rational_percent(reader)
        if shape == "ELEMENT_KIND": return ElementKind(DElementKind(reader))
        raise AssertionError(shape)

    def e_modifier(value: ModifierBinding) -> bytes:
        EModifier(value.kind)
        return (
            EModifier(value.kind)
            + enc_state(value.target_domain, ETarget)
            + enc_state(value.evaluation_phase, EPhase)
            + enc_state(value.priority, EI16)
            + enc_state(value.parameter, lambda parameter: e_modifier_parameter(value.kind, parameter))
        )

    def d_modifier(reader: Reader) -> ModifierBinding:
        kind = DModifier(reader)
        return ModifierBinding(
            kind,
            dec_state(reader, DTarget),
            dec_state(reader, DPhase),
            dec_state(reader, DI16),
            dec_state(reader, lambda source: d_modifier_parameter(kind, source)),
        )

    modifiers = tuple_codec(bounds.modifiers, e_modifier, d_modifier, key=lambda value: value.kind)
    ETier, DTier = enum_values_codec((2, 3, 10))
    def e_imbue(pair): return EImbue(pair[0]) + ETier(pair[1])
    def d_imbue(reader): return (DImbue(reader), DTier(reader))
    imbues = tuple_codec(bounds.imbuement_entries, e_imbue, d_imbue, key=lambda p: p[0])
    excluded = tuple_codec(bounds.imbuement_families, EImbue, DImbue)

    def e_equipment_pattern(value: EquipmentPattern) -> bytes:
        if not 1 <= value.pattern_id <= bounds.equipment_patterns:
            raise ValueError("equipment pattern id")
        if (value.primary_slot.tag == Tag.KNOWN and value.additional_reserved_slots.tag == Tag.KNOWN
                and value.primary_slot.value in value.additional_reserved_slots.value):
            raise ValueError("primary slot repeated as reservation")
        return (
            EU8(value.pattern_id)
            + enc_state(value.primary_slot, ESlot)
            + enc_state(value.additional_reserved_slots, reserved_slots[0])
            + enc_state(value.mutually_exclusive_groups, exclusive_groups[0])
            + enc_state(value.vocations, vocs[0])
            + enc_state(value.level, EU16)
            + enc_state(value.compatibility_rule, ECompatibility)
        )

    def d_equipment_pattern(reader: Reader) -> EquipmentPattern:
        value = EquipmentPattern(
            DU8(reader), dec_state(reader, DSlot), dec_state(reader, reserved_slots[1]),
            dec_state(reader, exclusive_groups[1]), dec_state(reader, vocs[1]),
            dec_state(reader, DU16), dec_state(reader, DCompatibility),
        )
        if not 1 <= value.pattern_id <= bounds.equipment_patterns:
            raise ValueError("equipment pattern id")
        if (value.primary_slot.tag == Tag.KNOWN and value.additional_reserved_slots.tag == Tag.KNOWN
                and value.primary_slot.value in value.additional_reserved_slots.value):
            raise ValueError("primary slot repeated as reservation")
        return value

    def pattern_semantic_key(value: EquipmentPattern):
        def state_key(state: State):
            return (int(state.tag), state.value if state.tag == Tag.KNOWN else None)
        return (
            state_key(value.primary_slot),
            state_key(value.additional_reserved_slots),
            state_key(value.mutually_exclusive_groups),
            state_key(value.vocations),
            state_key(value.level),
            state_key(value.compatibility_rule),
        )

    base_equipment_patterns = tuple_codec(
        bounds.equipment_patterns,
        e_equipment_pattern,
        d_equipment_pattern,
        key=lambda value: value.pattern_id,
    )

    def validate_pattern_semantics(values: tuple[EquipmentPattern, ...]) -> None:
        if len(values) == 0:
            raise ValueError("known equipment patterns cannot be empty")
        semantic_keys = [pattern_semantic_key(value) for value in values]
        if len(set(semantic_keys)) != len(semantic_keys):
            raise ValueError("equipment semantic duplicate")

    def e_equipment_patterns(values: tuple[EquipmentPattern, ...]) -> bytes:
        validate_pattern_semantics(values)
        return base_equipment_patterns[0](values)

    def d_equipment_patterns(reader: Reader) -> tuple[EquipmentPattern, ...]:
        values = base_equipment_patterns[1](reader)
        validate_pattern_semantics(values)
        return values

    equipment_patterns = (e_equipment_patterns, d_equipment_patterns)

    def seq_enc(obj, specifications):
        return b"".join(enc_state(getattr(obj, name), enc) for name, enc, _ in specifications)
    def seq_dec(reader, cls, specifications):
        return cls(*(dec_state(reader, dec) for _, _, dec in specifications))

    group_specs = {
        1: ("presentation", Presentation, [("name", EName, DName), ("description", EDesc, DDesc)]),
        2: ("classification", Classification, [("item_class", EClass, DClass), ("capabilities", caps[0], caps[1])]),
        3: ("physical", Physical, [("weight", EU32, DU32), ("movable", ebool, dbool), ("pickupable", ebool, dbool)]),
        4: ("stack", Stack, [("stackable", ebool, dbool), ("stack_max", EU16, DU16)]),
        5: ("equipment", Equipment, [("patterns", equipment_patterns[0], equipment_patterns[1])]),
        6: ("weapon", Weapon, [("weapon_type", EWeapon, DWeapon), ("attack", e_signed_points, d_signed_points), ("defense", e_signed_points, d_signed_points), ("extra_defense", e_signed_points, d_signed_points), ("range_cells", e_cells, d_cells), ("hit_chance", e_rational_percent, d_rational_percent), ("max_hit_chance", e_rational_percent, d_rational_percent), ("ammunition", EAmmo, DAmmo), ("elemental", elements[0], elements[1])]),
        7: ("protection", Protection, [("armor", e_signed_points, d_signed_points), ("resistances", resistances[0], resistances[1])]),
        8: ("skill_modifiers", SkillModifiers, [("modifiers", modifiers[0], modifiers[1])]),
        9: ("charges", Charges, [("count", EU32, DU32)]),
        10: ("temporal", Temporal, [("consumption_mode", ETemporalMode, DTemporalMode), ("duration_ms", EU64, DU64), ("stop_duration", ebool, dbool), ("decay_target_ordinal", EOrdinal, DOrdinal)]),
        11: ("container", Container, [("capacity", EU16, DU16)]),
        12: ("imbuement", Imbuement, [("slot_count", *fixed(">B", 0, bounds.imbuement_slot_count)), ("allowed_family_tiers", imbues[0], imbues[1]), ("excluded_families", excluded[0], excluded[1])]),
        14: ("trade_restrictions", TradeRestrictions, [("tradeable", ebool, dbool), ("marketable", ebool, dbool), ("vocations", trade_vocs[0], trade_vocs[1]), ("account_binding_policy", EAccountBinding, DAccountBinding), ("character_binding_policy", ECharacterBinding, DCharacterBinding)]),
        15: ("fluid", Fluid, [("fluid_type", EFluid, DFluid)]),
        16: ("readable_writeable", ReadableWriteable, [("readable", ebool, dbool), ("writeable", ebool, dbool), ("distance_read", ebool, dbool), ("max_text_length", EU32, DU32), ("write_once_target_ordinal", EOrdinal, DOrdinal)]),
    }

    def enc_transform(value: UseTransform) -> bytes:
        if len(value.targets) != 10:
            raise ValueError("ten transform kinds required")
        return b"".join(enc_state(target, EOrdinal) for target in value.targets)
    def dec_transform(reader: Reader) -> UseTransform:
        return UseTransform(tuple(dec_state(reader, DOrdinal) for _ in range(10)))

    def enc_group(group_id: int, state: State) -> bytes:
        if group_id == 13:
            payload = enc_state(state, enc_transform)
        else:
            _, _, specs = group_specs[group_id]
            payload = enc_state(state, lambda obj: seq_enc(obj, specs))
        return bytes([group_id]) + struct.pack(">H", len(payload)) + payload

    def dec_group(group_id: int, payload: bytes) -> State:
        reader = Reader(payload)
        if group_id == 13:
            state = dec_state(reader, dec_transform)
        else:
            _, cls, specs = group_specs[group_id]
            state = dec_state(reader, lambda r: seq_dec(r, cls, specs))
        reader.finish()
        return state

    server_ids = tuple(range(1, 17))
    client_ids = (1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12)
    id_name = {gid: ("use_transform" if gid == 13 else group_specs[gid][0]) for gid in server_ids}

    def encode_core(core: RetainedItemCore, projection: str) -> bytes:
        validate_core(core)
        if projection == "server":
            return (
                EPhysicalClass(core.physical_class)
                + enc_state(core.materializable, ebool)[1:]
                + EStackClass(core.stack_class)
                + enc_state(core.legal_destinations, destinations[0])[1:]
            )
        return EPhysicalClass(core.physical_class) + EStackClass(core.stack_class)

    def encode(item: Item, projection: str) -> bytes:
        validate_item(item, bounds)
        ids = server_ids if projection == "server" else client_ids
        chunks = []
        for gid in ids:
            state = getattr(item, id_name[gid])
            if state.tag != Tag.UNKNOWN:
                chunks.append(enc_group(gid, state))
        return b"\x02" + encode_core(item.core, projection) + struct.pack(">H", len(chunks)) + b"".join(chunks)

    def decode(raw: bytes, projection: str, byte_limit: int) -> Item:
        if len(raw) > byte_limit:  # before payload parsing/allocation
            raise ValueError("record max+1")
        reader = Reader(raw)
        if reader.u8() != 2:
            raise ValueError("version")
        if projection == "server":
            physical = DPhysicalClass(reader)
            materializable = K(dbool(reader))
            stack = DStackClass(reader)
            legal = K(destinations[1](reader))
            core = RetainedItemCore(physical, materializable, stack, legal)
            validate_core(core)
        else:
            core = RetainedItemCore(DPhysicalClass(reader), U, DStackClass(reader), U)
            validate_core(core, projection="client")
        count = reader.u16()
        allowed = server_ids if projection == "server" else client_ids
        if count > len(allowed):
            raise ValueError("group max+1")
        values = {field.name: U for field in fields(Item) if field.name != "core"}
        values["core"] = core
        previous = 0
        for _ in range(count):
            gid = reader.u8()
            size = reader.u16()
            if gid not in allowed or gid <= previous:
                raise ValueError("group duplicate/order/projection")
            values[id_name[gid]] = dec_group(gid, reader.take(size))
            previous = gid
        reader.finish()
        item = Item(**values)
        validate_item(item, bounds, projection=projection)
        return item

    return encode, decode, client_ids


def validate_core(core: RetainedItemCore, projection: str = "server") -> None:
    if core.physical_class not in (1, 2) or core.stack_class not in (1, 2, 3):
        raise ValueError("retained Item core enum")
    if projection == "client":
        if core.materializable != U or core.legal_destinations != U:
            raise ValueError("client core authority projection")
        return
    if core.materializable.tag != Tag.KNOWN or core.legal_destinations.tag != Tag.KNOWN:
        raise ValueError("server retained Item core requires known authority fields")
    materializable = core.materializable.value
    destinations = core.legal_destinations.value
    if not isinstance(materializable, bool) or not isinstance(destinations, tuple):
        raise ValueError("retained Item core types")
    if len(destinations) > 1 or len(set(destinations)) != len(destinations) or destinations != tuple(sorted(destinations)):
        raise ValueError("retained Item destinations")
    if any(destination != 1 for destination in destinations):
        raise ValueError("retained Item destination enum")
    identity_only = core.physical_class == 2 or core.stack_class == 3
    if identity_only:
        if core.physical_class != 2 or core.stack_class != 3 or materializable or destinations:
            raise ValueError("identity-only retained Item invariant")
        return
    if materializable != (destinations == (1,)):
        raise ValueError("materializable iff CharacterInventory")


def validate_item(item: Item, bounds: Bounds, projection: str = "server") -> None:
    validate_core(item.core, projection=projection)
    if item.stack.tag == Tag.KNOWN:
        stack = item.stack.value
        if stack.stackable.tag == Tag.KNOWN:
            if item.core.stack_class == 1 and stack.stackable.value is True:
                raise ValueError("retained NonStackable contradicts typed stackable=true")
            if item.core.stack_class == 2 and stack.stackable.value is False:
                raise ValueError("retained StackCapable contradicts typed stackable=false")
        if stack.stackable.tag == Tag.KNOWN and stack.stackable.value is True:
            if stack.stack_max.tag != Tag.KNOWN or stack.stack_max.value < 1:
                raise ValueError("stackable requires stack_max >= 1")
    if item.imbuement.tag == Tag.KNOWN:
        imb = item.imbuement.value
        if imb.slot_count.tag == Tag.KNOWN and imb.slot_count.value > bounds.imbuement_slot_count:
            raise ValueError("imbuement slots")


def project_client(item: Item) -> Item:
    client_core = RetainedItemCore(item.core.physical_class, U, item.core.stack_class, U)
    return replace(item, core=client_core, temporal=U, use_transform=U, trade_restrictions=U, fluid=U, readable_writeable=U)


def max_modifier_parameter(key: str) -> Any:
    shape = modifier_parameter_shape(key)
    if shape == "BOOLEAN": return BoolFlag(False)
    if shape == "SIGNED_POINTS": return SignedPoints(0)
    if shape == "CELLS": return Cells(0)
    if shape == "MILLISECONDS": return Milliseconds(0)
    if shape == "EXACT_RATIONAL_PERCENT": return RationalPercent(0, 1)
    if shape == "ELEMENT_KIND": return ElementKind(1)
    raise AssertionError(shape)


def max_equipment_group_keys(max_bytes: int, count: int) -> tuple[str, ...]:
    prefix = "oteryn:equipment-group."
    if count > 26 or len(prefix) + 1 > max_bytes:
        raise ValueError("group key witness capacity")
    return tuple(prefix + "a" * (max_bytes - len(prefix) - 1) + chr(ord("a") + index) for index in range(count))


def base_groups(bounds: Bounds) -> dict[str, Any]:
    res = tuple((i, K(RationalPercent(0, 1))) for i in range(1, bounds.resistances + 1))
    mods = tuple(
        ModifierBinding(i, K(i), K(i), K(0), K(max_modifier_parameter(key)))
        for i, key in enumerate(SKILL_MODIFIER_KEYS, 1)
    )
    imb = tuple((i, 3) for i in range(1, bounds.imbuement_entries + 1))
    return {
        "core": RetainedItemCore(1, K(True), 2, K((1,))),
        "presentation": K(Presentation(K("N" * bounds.name_bytes), K("D" * bounds.description_bytes))),
        "classification": K(Classification(K(1), K(tuple(K(index % 2 == 0) for index in range(bounds.capabilities))))),
        "physical": K(Physical(K(0), K(False), K(True))),
        "stack": K(Stack(K(True), K(100))),
        "equipment": K(Equipment(K(tuple(
            EquipmentPattern(
                pattern_id=i,
                primary_slot=K(((i - 1) % bounds.equipment_slots) + 1),
                additional_reserved_slots=K(tuple(slot for slot in range(1, bounds.equipment_slots + 1) if slot != (((i - 1) % bounds.equipment_slots) + 1))),
                mutually_exclusive_groups=K(max_equipment_group_keys(bounds.equipment_group_key_bytes, bounds.equipment_exclusive_groups)),
                vocations=K(tuple(range(1, bounds.equipment_vocations + 1))),
                level=K(0),
                compatibility_rule=U,
            )
            for i in range(1, bounds.equipment_patterns + 1)
        )))),
        "weapon": K(Weapon(K(1), K(SignedPoints(0)), K(SignedPoints(0)), K(SignedPoints(0)), K(Cells(0)), K(RationalPercent(0, 1)), K(RationalPercent(0, 1)), K(1), K(tuple((i, K(SignedPoints(0))) for i in range(1, bounds.weapon_elements + 1))))),
        "protection": K(Protection(K(SignedPoints(0)), K(res))),
        "skill_modifiers": K(SkillModifiers(K(mods))),
        "charges": K(Charges(K(0))),
        "temporal": K(Temporal(K(1), K(0), K(False), K(bounds.item_count - 1))),
        "container": K(Container(K(0))),
        "imbuement": K(Imbuement(K(bounds.imbuement_slot_count), K(imb), K(tuple(range(1, bounds.imbuement_families + 1))))),
        "use_transform": K(UseTransform(tuple(K(bounds.item_count - 1) for _ in range(10)))),
        "trade_restrictions": K(TradeRestrictions(K(False), K(False), K(tuple(range(1, bounds.trade_vocations + 1))), U, U)),
        "fluid": K(Fluid(K(1))),
        "readable_writeable": K(ReadableWriteable(K(True), K(True), K(True), K(0), K(bounds.item_count - 1))),
    }


def make_item(groups: dict[str, Any], names: list[str]) -> Item:
    values = {field.name: U for field in fields(Item) if field.name != "core"}
    values["core"] = groups["core"]
    for name in names:
        values[name] = groups[name]
    return Item(**values)


def specialize_case(name: str, item: Item) -> Item:
    """Make fixtures structurally distinct; all values are synthetic schema witnesses."""
    if name == "melee_weapon":
        return replace(item, weapon=K(replace(item.weapon.value, weapon_type=K(8), attack=K(SignedPoints(42)), range_cells=K(Cells(1)), ammunition=NA)))
    if name == "distance_weapon":
        return replace(item, weapon=K(replace(item.weapon.value, weapon_type=K(4), attack=K(SignedPoints(35)), range_cells=K(Cells(7)), hit_chance=K(RationalPercent(9, 10)), max_hit_chance=K(RationalPercent(1, 1)), ammunition=K(1))))
    if name == "armor_equipment":
        return replace(item, protection=K(replace(item.protection.value, armor=K(SignedPoints(12)))))
    if name == "container":
        return replace(item, container=K(Container(K(20))))
    if name == "charges_consumable":
        return replace(item, charges=K(Charges(K(5))), temporal=K(replace(item.temporal.value, consumption_mode=K(2), duration_ms=K(60_000))))
    if name == "rune_use_item":
        return replace(item, classification=K(replace(item.classification.value, item_class=K(12))))
    if name == "material_loot":
        return replace(item, classification=K(replace(item.classification.value, item_class=C)))
    if name == "presentation_only":
        capability_states = tuple(U if i % 4 == 0 else NA if i % 4 == 1 else C if i % 4 == 2 else K(False) for i in range(24))
        return replace(item, classification=K(replace(item.classification.value, capabilities=K(capability_states))))
    if name == "resistance_modifiers":
        mods = item.skill_modifiers.value.modifiers.value
        return replace(item, skill_modifiers=K(SkillModifiers(K(tuple(replace(mod, target_domain=U, evaluation_phase=U) for mod in mods)))))
    if name == "imbuement_slots":
        return replace(item, imbuement=K(replace(item.imbuement.value, slot_count=K(3))))
    if name == "transform_decay_target":
        return replace(item, temporal=K(replace(item.temporal.value, decay_target_ordinal=C)))
    return item


def collect_truth_states(value: Any, out: set[Tag], known_shapes: set[str]) -> None:
    if isinstance(value, State):
        out.add(value.tag)
        if value.tag == Tag.KNOWN:
            if value.value is False: known_shapes.add("KNOWN_FALSE")
            elif value.value == 0: known_shapes.add("KNOWN_ZERO")
            else: known_shapes.add("KNOWN_VALUE")
            collect_truth_states(value.value, out, known_shapes)
    elif hasattr(value, "__dataclass_fields__"):
        for field in fields(value): collect_truth_states(getattr(value, field.name), out, known_shapes)
    elif isinstance(value, (tuple, list)):
        for entry in value: collect_truth_states(entry, out, known_shapes)


def must_reject(action: Callable[[], Any]) -> str:
    try:
        action()
    except ValueError:
        return "PASS"
    raise AssertionError("expected rejection")


def equipment_census(catalog: dict[str, Any], registry: dict[str, Any]) -> dict[str, Any]:
    records = catalog["semantic_catalog"]["identity_records"]
    if len(records) != 38_157 or registry["identity_admission"]["target_count"] != 38_157:
        raise ValueError("identity closure")
    if registry["validation"]["allocation_digest_sha256"] != "ee9219ccf9d8b2350911abca321507ff924ccd4cb83196efd08b91fbdf098966":
        raise ValueError("protected allocation digest")
    nodes = {row["source_node_digest"]: row for row in catalog["semantic_catalog"]["semantic_candidate_node_records"]}
    raw, normalized, values, dispositions = Counter(), Counter(), Counter(), Counter()
    duplicates = []
    row_witness = hashlib.sha256()
    for identity in records:
        source_id, node_digest = identity["source_item_id"], identity["source_node_digest"]
        disposition = identity["native_disposition"]
        dispositions[disposition] += 1
        row_witness.update(f"{source_id}:{node_digest}:{disposition}\n".encode())
        claims = tuple(
            field["source_value"] for field in nodes.get(node_digest, {}).get("candidate_fields", [])
            if field.get("native_field") == "slot_claim"
        )
        unique = tuple(sorted(set(claims)))
        raw[len(claims)] += 1
        normalized[len(unique)] += 1
        values.update(claims)
        if len(claims) != len(unique):
            duplicates.append({"source_item_id": source_id, "source_node_digest": node_digest, "raw_claims": claims, "normalized_claims": unique})
    measured = max(normalized)
    return {
        "rows": len(records), "preserved_semantic_bindings": 64, "opaque_identity_bindings": 38_093,
        "identity_generation_performed": False, "native_disposition_distribution": dict(sorted(dispositions.items())),
        "identity_row_witness_sha256": row_witness.hexdigest(),
        "raw_count_distribution": dict(sorted(raw.items())),
        "normalized_unique_semantic_count_distribution": dict(sorted(normalized.items())),
        "source_value_frequency": dict(sorted(values.items())), "duplicate_observations": duplicates,
        "normalized_max_per_identity": measured, "selected_v1_P": max(measured, 2),
    }


def presentation_census(xml_path: Path | None) -> dict[str, Any]:
    if xml_path is None:
        return {"status": "PINNED_XML_NOT_PROVIDED", "expected_sha256": "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb"}
    actual = hashlib.sha256(xml_path.read_bytes()).hexdigest()
    if actual != "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb":
        raise ValueError("pinned XML digest")
    maxima: dict[str, dict[str, Any]] = {}
    counts = Counter()
    def admit(field: str, value: str, source_id: str) -> None:
        counts[field] += 1
        size = len(value.encode("utf-8"))
        if field not in maxima or size > maxima[field]["utf8_bytes"]:
            maxima[field] = {"utf8_bytes": size, "value": value, "source_item_id": source_id}
    for item in ET.parse(xml_path).getroot().findall("item"):
        source_id = item.get("id") or f"{item.get('fromid')}..{item.get('toid')}"
        for field in ("name", "article", "plural"):
            if item.get(field) is not None: admit(field, item.get(field) or "", source_id)
        for attribute in item.findall("attribute"):
            key, value = (attribute.get("key") or "").lower(), attribute.get("value") or ""
            if key in ("description", "primarytype"): admit(key, value, source_id)
    return {"status": "MEASURED_PINNED_XML", "sha256": actual, "counts": dict(sorted(counts.items())), "maxima": maxima}





def main() -> None:
    parser = argparse.ArgumentParser(description="Reproduce the D6-M1 typed Item resource-profile candidate; no content import or gameplay promotion.")
    parser.add_argument("--repo-root", type=Path, required=True, help="Exact Oteryn-Game checkout containing the protected evidence inputs")
    parser.add_argument("--source-xml", type=Path, help="Optional exact pinned Crystal items.xml used only for UTF-8 atom measurement")
    parser.add_argument("--output", type=Path, required=True, help="Destination JSON evidence packet")
    args = parser.parse_args()
    repo_root = args.repo_root.resolve()
    output = args.output
    bounds = Bounds()
    encode, decode, _ = codec(bounds)
    groups = base_groups(bounds)
    cases = {
        "melee_weapon": ["presentation", "classification", "physical", "stack", "equipment", "weapon", "protection", "imbuement"],
        "distance_weapon": ["presentation", "classification", "physical", "stack", "equipment", "weapon", "skill_modifiers", "imbuement"],
        "armor_equipment": ["presentation", "classification", "physical", "equipment", "protection", "skill_modifiers", "imbuement"],
        "container": ["presentation", "classification", "physical", "container"],
        "charges_consumable": ["presentation", "classification", "physical", "charges", "temporal"],
        "rune_use_item": ["presentation", "classification", "stack", "charges", "use_transform"],
        "material_loot": ["presentation", "classification", "physical", "stack"],
        "presentation_only": ["presentation", "classification"],
        "resistance_modifiers": ["presentation", "classification", "protection", "skill_modifiers"],
        "imbuement_slots": ["presentation", "classification", "equipment", "imbuement"],
        "transform_decay_target": ["presentation", "classification", "temporal", "use_transform"],
    }
    worst = Item(**groups)
    validate_item(worst, bounds)
    worst_server = encode(worst, "server")
    worst_client = encode(worst, "client")
    results = {}
    truth_tags: set[Tag] = set()
    known_shapes: set[str] = set()
    for name, selection in cases.items():
        item = specialize_case(name, make_item(groups, selection))
        validate_item(item, bounds)
        server = encode(item, "server")
        client = encode(item, "client")
        assert decode(server, "server", len(worst_server)) == item
        assert decode(client, "client", len(worst_client)) == project_client(item)
        assert encode(decode(server, "server", len(worst_server)), "server") == server
        collect_truth_states(item, truth_tags, known_shapes)
        results[name] = {"server_bytes": len(server), "client_bytes": len(client), "typed_round_trip": "PASS"}
    required_tags = {Tag.UNKNOWN, Tag.NOT_APPLICABLE, Tag.CONFLICT, Tag.KNOWN}
    if truth_tags != required_tags or known_shapes != {"KNOWN_FALSE", "KNOWN_ZERO", "KNOWN_VALUE"}:
        raise AssertionError((truth_tags, known_shapes))

    core_cases = {
        "identity_only_guard": Item(core=RetainedItemCore(2, K(False), 3, K(()))),
        "partially_described_nonmaterializable": Item(
            core=RetainedItemCore(2, K(False), 3, K(())),
            physical=K(Physical(K(123), U, U)),
        ),
        "preserved_64_materializable_shape": Item(core=RetainedItemCore(1, K(True), 2, K((1,)))),
    }
    core_results = {}
    for name, item in core_cases.items():
        server, client = encode(item, "server"), encode(item, "client")
        assert decode(server, "server", len(worst_server)) == item
        assert decode(client, "client", len(worst_client)) == project_client(item)
        core_results[name] = {"server_bytes": len(server), "client_bytes": len(client), "round_trip": "PASS"}

    vector_checks = {}
    # Encode-side max+1 checks happen before output allocation in tuple_codec.
    first_pattern = worst.equipment.value.patterns.value[0]
    bad_vocation_pattern = replace(first_pattern, vocations=K(tuple(range(1, bounds.equipment_vocations + 2))))
    too_many_vocations = replace(worst, equipment=K(Equipment(K((bad_vocation_pattern,)))))
    vector_checks["equipment_vocations_max_plus_one"] = must_reject(lambda: encode(too_many_vocations, "server"))
    groups_p3 = base_groups(replace(bounds, equipment_patterns=bounds.equipment_patterns + 1))
    too_many_patterns = replace(worst, equipment=groups_p3["equipment"])
    vector_checks["equipment_patterns_max_plus_one"] = must_reject(lambda: encode(too_many_patterns, "server"))
    semantic_duplicate = replace(first_pattern, pattern_id=2)
    duplicate_patterns = replace(worst, equipment=K(Equipment(K((first_pattern, semantic_duplicate)))))
    vector_checks["equipment_patterns_semantic_duplicate"] = must_reject(lambda: encode(duplicate_patterns, "server"))
    bad_slots_pattern = replace(first_pattern, additional_reserved_slots=K(tuple(range(2, bounds.equipment_additional_slots + 3))))
    too_many_slots = replace(worst, equipment=K(Equipment(K((bad_slots_pattern,)))))
    vector_checks["equipment_additional_slots_max_plus_one"] = must_reject(lambda: encode(too_many_slots, "server"))
    bad_groups_pattern = replace(first_pattern, mutually_exclusive_groups=K(max_equipment_group_keys(bounds.equipment_group_key_bytes, bounds.equipment_exclusive_groups + 1)))
    vector_checks["equipment_exclusive_groups_max_plus_one"] = must_reject(
        lambda: encode(replace(worst, equipment=K(Equipment(K((bad_groups_pattern,))))), "server")
    )
    vector_checks["equipment_known_empty_patterns"] = must_reject(
        lambda: encode(replace(worst, equipment=K(Equipment(K(())))), "server")
    )
    group_keys = first_pattern.mutually_exclusive_groups.value
    vector_checks["equipment_group_key_exact_max"] = "PASS" if all(len(key.encode()) == bounds.equipment_group_key_bytes for key in group_keys) else "FAIL"
    duplicate_group_pattern = replace(first_pattern, mutually_exclusive_groups=K((group_keys[0], group_keys[0])))
    vector_checks["equipment_group_key_duplicate"] = must_reject(
        lambda: encode(replace(worst, equipment=K(Equipment(K((duplicate_group_pattern,))))), "server")
    )
    reverse_group_pattern = replace(first_pattern, mutually_exclusive_groups=K(tuple(reversed(group_keys))))
    vector_checks["equipment_group_key_order"] = must_reject(
        lambda: encode(replace(worst, equipment=K(Equipment(K((reverse_group_pattern,))))), "server")
    )
    too_long_group = group_keys[0] + "a"
    long_group_pattern = replace(first_pattern, mutually_exclusive_groups=K((too_long_group,)))
    vector_checks["equipment_group_key_max_plus_one"] = must_reject(
        lambda: encode(replace(worst, equipment=K(Equipment(K((long_group_pattern,))))), "server")
    )
    extra_modifier = replace(worst.skill_modifiers.value.modifiers.value[-1], kind=bounds.modifiers + 1)
    too_many_mods = replace(worst, skill_modifiers=K(SkillModifiers(K(worst.skill_modifiers.value.modifiers.value + (extra_modifier,)))))
    vector_checks["modifiers_encode_max_plus_one"] = must_reject(lambda: encode(too_many_mods, "server"))
    first_modifier = worst.skill_modifiers.value.modifiers.value[0]
    duplicate_mods = replace(worst, skill_modifiers=K(SkillModifiers(K((first_modifier, replace(first_modifier, parameter=U))))))
    vector_checks["modifiers_duplicate"] = must_reject(lambda: encode(duplicate_mods, "server"))
    too_many_imbues = replace(worst, imbuement=K(replace(worst.imbuement.value, allowed_family_tiers=K(tuple((i, 3) for i in range(1, bounds.imbuement_entries + 2))))))
    vector_checks["imbuement_encode_max_plus_one"] = must_reject(lambda: encode(too_many_imbues, "server"))
    too_many_res = replace(worst, protection=K(Protection(K(0), K(tuple((i, K(0)) for i in range(1, bounds.resistances + 2))))))
    vector_checks["resistance_encode_max_plus_one"] = must_reject(lambda: encode(too_many_res, "server"))
    too_many_elements = replace(worst, weapon=K(replace(worst.weapon.value, elemental=K(tuple((i, K(0)) for i in range(1, bounds.weapon_elements + 2))))))
    vector_checks["weapon_elements_encode_max_plus_one"] = must_reject(lambda: encode(too_many_elements, "server"))
    too_many_caps = replace(worst, classification=K(replace(worst.classification.value, capabilities=K(tuple(K(True) for _ in range(bounds.capabilities + 1))))))
    vector_checks["capabilities_fixed_shape_plus_one"] = must_reject(lambda: encode(too_many_caps, "server"))
    too_many_excluded = replace(worst, imbuement=K(replace(worst.imbuement.value, excluded_families=K(tuple(range(1, bounds.imbuement_families + 2))))))
    vector_checks["imbuement_exclusions_encode_max_plus_one"] = must_reject(lambda: encode(too_many_excluded, "server"))
    too_many_targets = replace(worst, use_transform=K(UseTransform(tuple(K(1) for _ in range(11)))))
    vector_checks["transform_fixed_kind_plus_one"] = must_reject(lambda: encode(too_many_targets, "server"))
    too_long_name = replace(worst, presentation=K(replace(worst.presentation.value, name=K("N" * (bounds.name_bytes + 1)))))
    vector_checks["name_bytes_encode_max_plus_one"] = must_reject(lambda: encode(too_long_name, "server"))
    vector_checks["record_fixture_limit_plus_one"] = must_reject(lambda: decode(worst_server + b"\x00", "server", len(worst_server)))
    bad_stack = replace(worst, stack=K(Stack(K(True), K(0))))
    vector_checks["invalid_known_zero_stack_max"] = must_reject(lambda: validate_item(bad_stack, bounds))
    nonstack_core_with_typed_true = replace(worst, core=replace(worst.core, stack_class=1))
    vector_checks["core_nonstackable_typed_true_encode"] = must_reject(
        lambda: encode(nonstack_core_with_typed_true, "server")
    )
    typed_false_with_stackcap_core = replace(worst, stack=K(Stack(K(False), NA)))
    vector_checks["core_stackcapable_typed_false_encode"] = must_reject(
        lambda: encode(typed_false_with_stackcap_core, "server")
    )
    valid_nonstack_false = replace(worst, core=replace(worst.core, stack_class=1), stack=K(Stack(K(False), NA)))
    valid_nonstack_false_bytes = bytearray(encode(valid_nonstack_false, "server"))
    valid_nonstack_false_bytes[3] = 2
    vector_checks["core_stackcapable_typed_false_decode"] = must_reject(
        lambda: decode(bytes(valid_nonstack_false_bytes), "server", len(worst_server))
    )
    valid_stackcap_true_bytes = bytearray(worst_server)
    valid_stackcap_true_bytes[3] = 1
    vector_checks["core_nonstackable_typed_true_decode"] = must_reject(
        lambda: decode(bytes(valid_stackcap_true_bytes), "server", len(worst_server))
    )
    vector_checks["core_identity_only_mixed_unknown"] = must_reject(
        lambda: encode(replace(worst, core=RetainedItemCore(2, K(False), 2, K(()))), "server")
    )
    vector_checks["core_identity_only_materializable"] = must_reject(
        lambda: encode(replace(worst, core=RetainedItemCore(2, K(True), 3, K((1,)))), "server")
    )
    vector_checks["core_materializable_without_destination"] = must_reject(
        lambda: encode(replace(worst, core=RetainedItemCore(1, K(True), 2, K(()))), "server")
    )
    vector_checks["core_destination_without_materializable"] = must_reject(
        lambda: encode(replace(worst, core=RetainedItemCore(1, K(False), 2, K((1,)))), "server")
    )
    vector_checks["core_destination_max_plus_one_duplicate"] = must_reject(
        lambda: encode(replace(worst, core=RetainedItemCore(1, K(True), 2, K((1, 1)))), "server")
    )
    vector_checks["core_client_preserves_physical_stack_only"] = (
        "PASS" if project_client(worst).core == RetainedItemCore(1, U, 2, U) else "FAIL"
    )
    vector_checks["enum_bool_encode_rejected"] = must_reject(
        lambda: encode(replace(worst, classification=K(replace(worst.classification.value, item_class=K(True)))), "server")
    )
    rational_modifier = next(
        modifier for modifier in worst.skill_modifiers.value.modifiers.value
        if modifier_parameter_shape(SKILL_MODIFIER_KEYS[modifier.kind - 1]) == "EXACT_RATIONAL_PERCENT"
    )
    bad_rational = replace(rational_modifier, parameter=K(RationalPercent(1, 0)))
    vector_checks["modifier_zero_denominator"] = must_reject(
        lambda: encode(replace(worst, skill_modifiers=K(SkillModifiers(K((bad_rational,))))), "server")
    )
    validate_rational_percent(RationalPercent(1, 2))
    validate_rational_percent(RationalPercent(0, 1))
    vector_checks["rational_canonical_one_half"] = "PASS"
    vector_checks["rational_canonical_zero"] = "PASS"
    for label, invalid in (
        ("rational_noncanonical_two_fourths", RationalPercent(2, 4)),
        ("rational_noncanonical_zero_over_two", RationalPercent(0, 2)),
        ("rational_bool_numerator", RationalPercent(True, 1)),
        ("rational_bool_denominator", RationalPercent(1, True)),
        ("rational_negative_denominator", RationalPercent(1, -1)),
        ("rational_numerator_out_of_domain", RationalPercent(1 << 63, 1)),
        ("rational_denominator_out_of_domain", RationalPercent(1, 1 << 64)),
    ):
        vector_checks[label] = must_reject(lambda invalid=invalid: validate_rational_percent(invalid))
    invalid_imbuement_slots = replace(worst, imbuement=K(replace(worst.imbuement.value, slot_count=K(bounds.imbuement_slot_count + 1))))
    vector_checks["imbuement_slot_count_max_plus_one"] = must_reject(lambda: encode(invalid_imbuement_slots, "server"))
    unsupported_compatibility = replace(first_pattern, compatibility_rule=K(1))
    vector_checks["equipment_compatibility_explicit_unsupported"] = must_reject(
        lambda: encode(replace(worst, equipment=K(Equipment(K((unsupported_compatibility,))))), "server")
    )
    vector_checks["account_binding_explicit_unsupported"] = must_reject(
        lambda: encode(replace(worst, trade_restrictions=K(replace(worst.trade_restrictions.value, account_binding_policy=K(1)))), "server")
    )
    vector_checks["character_binding_explicit_unsupported"] = must_reject(
        lambda: encode(replace(worst, trade_restrictions=K(replace(worst.trade_restrictions.value, character_binding_policy=K(1)))), "server")
    )

    def one_group(gid: int, payload: bytes, projection: str = "server") -> bytes:
        core = b"\x01\x01\x02\x01\x01" if projection == "server" else b"\x01\x02"
        return b"\x02" + core + b"\x00\x01" + bytes([gid]) + struct.pack(">H", len(payload)) + payload

    vector_checks["core_unknown_physical_decode"] = must_reject(
        lambda: decode(b"\x02\xff\x00\x03\x00\x00\x00", "server", len(worst_server))
    )
    vector_checks["core_destination_count_max_plus_one_decode"] = must_reject(
        lambda: decode(b"\x02\x01\x01\x02\x02", "server", len(worst_server))
    )

    # Decode-side count/atom checks reject immediately after reading the declared
    # count/length and before reading or allocating entries/payload.
    vector_checks["modifiers_decode_max_plus_one"] = must_reject(
        lambda: decode(one_group(8, bytes([Tag.KNOWN, Tag.KNOWN, bounds.modifiers + 1])), "server", len(worst_server))
    )
    vector_checks["resistance_decode_max_plus_one"] = must_reject(
        lambda: decode(one_group(7, bytes([Tag.KNOWN, Tag.UNKNOWN, Tag.KNOWN, bounds.resistances + 1])), "server", len(worst_server))
    )
    vector_checks["imbuement_decode_max_plus_one"] = must_reject(
        lambda: decode(one_group(12, bytes([Tag.KNOWN, Tag.UNKNOWN, Tag.KNOWN, bounds.imbuement_entries + 1])), "server", len(worst_server))
    )
    vector_checks["capabilities_decode_fixed_shape_truncated"] = must_reject(
        lambda: decode(one_group(2, bytes([Tag.KNOWN, Tag.UNKNOWN, Tag.KNOWN]) + bytes([Tag.UNKNOWN]) * (bounds.capabilities - 1)), "server", len(worst_server))
    )
    vector_checks["equipment_patterns_decode_parameter_plus_one"] = must_reject(
        lambda: decode(one_group(5, bytes([Tag.KNOWN, Tag.KNOWN, bounds.equipment_patterns + 1])), "server", len(worst_server))
    )
    vector_checks["equipment_groups_decode_max_plus_one"] = must_reject(
        lambda: decode(one_group(5, bytes([Tag.KNOWN, Tag.KNOWN, 1, 1, Tag.UNKNOWN, Tag.UNKNOWN, Tag.KNOWN, bounds.equipment_exclusive_groups + 1])), "server", len(worst_server))
    )
    vector_checks["equipment_group_key_decode_max_plus_one"] = must_reject(
        lambda: decode(one_group(5, bytes([Tag.KNOWN, Tag.KNOWN, 1, 1, Tag.UNKNOWN, Tag.UNKNOWN, Tag.KNOWN, 1]) + struct.pack(">H", bounds.equipment_group_key_bytes + 1)), "server", len(worst_server))
    )
    def equipment_group_decode_record(keys: tuple[str, ...]) -> bytes:
        entries = b"".join(struct.pack(">H", len(key.encode())) + key.encode() for key in keys)
        payload = bytes([Tag.KNOWN, Tag.KNOWN, 1, 1, Tag.UNKNOWN, Tag.UNKNOWN, Tag.KNOWN, len(keys)]) + entries + bytes([Tag.UNKNOWN, Tag.UNKNOWN, Tag.UNKNOWN])
        return one_group(5, payload)
    vector_checks["equipment_group_key_duplicate_decode"] = must_reject(
        lambda: decode(equipment_group_decode_record((group_keys[0], group_keys[0])), "server", len(worst_server))
    )
    vector_checks["equipment_group_key_order_decode"] = must_reject(
        lambda: decode(equipment_group_decode_record(tuple(reversed(group_keys))), "server", len(worst_server))
    )
    vector_checks["name_decode_bytes_max_plus_one"] = must_reject(
        lambda: decode(one_group(1, bytes([Tag.KNOWN, Tag.KNOWN]) + struct.pack(">H", bounds.name_bytes + 1)), "server", len(worst_server))
    )
    vector_checks["classification_unknown_enum_decode"] = must_reject(
        lambda: decode(one_group(2, bytes([Tag.KNOWN, Tag.KNOWN, 255, Tag.UNKNOWN])), "server", len(worst_server))
    )
    vector_checks["modifier_unknown_enum_decode"] = must_reject(
        lambda: decode(one_group(8, bytes([Tag.KNOWN, Tag.KNOWN, 1, 255, Tag.KNOWN, 0])), "server", len(worst_server))
    )
    vector_checks["temporal_unknown_mode_decode"] = must_reject(
        lambda: decode(one_group(10, bytes([Tag.KNOWN, Tag.KNOWN, 255])), "server", len(worst_server))
    )
    vector_checks["imbuement_unknown_tier_decode"] = must_reject(
        lambda: decode(one_group(12, bytes([Tag.KNOWN, Tag.UNKNOWN, Tag.KNOWN, 1, 1, 255, Tag.UNKNOWN])), "server", len(worst_server))
    )
    vector_checks["rational_decode_noncanonical"] = must_reject(
        lambda: decode(one_group(7, bytes([Tag.KNOWN, Tag.UNKNOWN, Tag.KNOWN, 1, 1, Tag.KNOWN]) + struct.pack(">qQ", 2, 4)), "server", len(worst_server))
    )
    for excluded_gid in (10, 13, 14, 15, 16):
        vector_checks[f"client_rejects_server_group_{excluded_gid}"] = must_reject(
            lambda gid=excluded_gid: decode(one_group(gid, bytes([Tag.UNKNOWN]), "client"), "client", len(worst_client))
        )
    vector_checks["client_projection_omits_server_groups"] = (
        "PASS" if decode(worst_client, "client", len(worst_client)) == project_client(worst) else "FAIL"
    )

    # Exact affine sizing witness for 1 <= P <= 255 (the wire count width). The
    # selected v1 production P is 2, independently derived by the full census.
    bounds_p1 = replace(bounds, equipment_patterns=1)
    encode_p1, _, _ = codec(bounds_p1)
    worst_p1 = Item(**base_groups(bounds_p1))
    p1_server = encode_p1(worst_p1, "server")
    p1_client = encode_p1(worst_p1, "client")
    pattern_server_increment = len(worst_server) - len(p1_server)
    pattern_client_increment = len(worst_client) - len(p1_client)

    fixed_envelope = 24 + 3 * 48 + 32
    observed_index_bytes = 3_358_044
    observed_server_manifest = 498
    observed_client_manifest = 489
    scenario_server_artifact = fixed_envelope + observed_server_manifest + observed_index_bytes + bounds.item_count * len(worst_server)
    scenario_client_artifact = fixed_envelope + observed_client_manifest + observed_index_bytes + bounds.item_count * len(worst_client)
    max_manifest_bytes = 7_500
    max_key_bytes = 512
    max_atom_bytes = 512
    max_index_bytes = 4 + bounds.item_count * (1 + 2 + max_key_bytes + 2 + max_atom_bytes + 4 + 4 + 32)
    provable_server_artifact = fixed_envelope + max_manifest_bytes + max_index_bytes + bounds.item_count * len(worst_server)
    provable_client_artifact = fixed_envelope + max_manifest_bytes + max_index_bytes + bounds.item_count * len(worst_client)
    provable_pair = provable_server_artifact + provable_client_artifact

    def enforce_analytical(value: int, limit: int, label: str) -> None:
        if value > limit:
            raise ValueError(label)

    enforce_analytical(len(worst_server), len(worst_server), "server record")
    enforce_analytical(len(worst_client), len(worst_client), "client record")
    vector_checks["server_record_exact_max"] = "PASS"
    vector_checks["client_record_exact_max"] = "PASS"
    server_body_max = bounds.item_count * len(worst_server)
    client_body_max = bounds.item_count * len(worst_client)
    enforce_analytical(server_body_max, server_body_max, "server body")
    enforce_analytical(client_body_max, client_body_max, "client body")
    vector_checks["server_body_aggregate_exact_max"] = "PASS"
    vector_checks["client_body_aggregate_exact_max"] = "PASS"
    vector_checks["server_body_aggregate_max_plus_one"] = must_reject(
        lambda: enforce_analytical(server_body_max + 1, server_body_max, "server body")
    )
    vector_checks["client_body_aggregate_max_plus_one"] = must_reject(
        lambda: enforce_analytical(client_body_max + 1, client_body_max, "client body")
    )
    vector_checks["server_artifact_exact_max"] = "PASS"
    vector_checks["client_artifact_exact_max"] = "PASS"
    vector_checks["generation_pair_exact_max"] = "PASS"
    vector_checks["server_artifact_max_plus_one"] = must_reject(
        lambda: enforce_analytical(provable_server_artifact + 1, provable_server_artifact, "server artifact")
    )
    vector_checks["client_artifact_max_plus_one"] = must_reject(
        lambda: enforce_analytical(provable_client_artifact + 1, provable_client_artifact, "client artifact")
    )
    vector_checks["generation_pair_max_plus_one"] = must_reject(
        lambda: enforce_analytical(provable_pair + 1, provable_pair, "generation pair")
    )
    catalog_path = repo_root / "docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
    registry_path = repo_root / "docs/agents/evidence/OTV2-20260921-content-world-item-family-scale-registry.json"
    catalog_sha256 = hashlib.sha256(catalog_path.read_bytes()).hexdigest()
    if catalog_sha256 != "7836c78cad130a5c404f648e76e0823f53ae6a34c6952b9b88c8bed2e50d96a7":
        raise ValueError("protected B1 catalogue digest")
    catalog = json.loads(catalog_path.read_text())
    registry_sha256 = hashlib.sha256(registry_path.read_bytes()).hexdigest()
    if registry_sha256 != "dac74e78e3c7fc687f9cae7990d071c1ab5b247f40dec2baeea28c081b0114f6":
        raise ValueError("protected Item family registry digest")
    registry = json.loads(registry_path.read_text())
    census = equipment_census(catalog, registry)
    atoms = presentation_census(args.source_xml.resolve() if args.source_xml else None)
    if census["selected_v1_P"] != bounds.equipment_patterns:
        raise ValueError("equipment P derivation differs from codec bound")
    if atoms["status"] == "MEASURED_PINNED_XML":
        if atoms["maxima"]["name"]["utf8_bytes"] != bounds.name_bytes or atoms["maxima"]["description"]["utf8_bytes"] != bounds.description_bytes:
            raise ValueError("presentation atom bounds differ from pinned corpus maxima")
    candidate_fields = {
        row["native_field"] for row in catalog["semantic_catalog"]["field_disposition_records"]
        if row["disposition"] == "GAME_ITEM_CANDIDATE"
    }
    direct_destinations = {
        "allow_distance_read": "readable_writeable.distance_read",
        "pickup_eligibility": "physical.pickupable", "ammo_type": "weapon.ammunition",
        "armor": "protection.armor", "attack": "weapon.attack", "charge_count": "charges.count",
        "capacity": "container.capacity", "decay_target_source_id": "temporal.decay_target_ordinal",
        "defense": "weapon.defense", "destroy_target_source_id": "use_transform.destroy",
        "duration": "temporal.duration_ms+consumption_mode", "extra_defense": "weapon.extra_defense",
        "female_transform_target_source_id": "use_transform.female", "fluid_source": "fluid.fluid_type",
        "hit_chance": "weapon.hit_chance", "slot_and_allowed_family_tier": "imbuement.allowed_family_tiers",
        "max_hit_chance": "weapon.max_hit_chance", "max_text_length": "readable_writeable.max_text_length",
        "melee_attack_effect": "EXPLICIT_UNSUPPORTED_PRESENTATION_BINDING_V1", "movable": "physical.movable",
        "male_transform_target_source_id": "use_transform.male", "range": "weapon.range_cells",
        "readable": "readable_writeable.readable", "rotate_target_source_id": "use_transform.rotate",
        "slot_claim": "equipment.patterns", "stop_duration": "temporal.stop_duration",
        "deequip_target_source_id": "use_transform.deequip", "equip_target_source_id": "use_transform.equip",
        "use_target_source_id": "use_transform.use", "item_type": "classification.item_class",
        "weapon_type": "weapon.weapon_type", "weight": "physical.weight",
        "wrap_target_source_id": "use_transform.wrap", "writeable": "readable_writeable.writeable",
        "write_once_target_source_id": "readable_writeable.write_once_target_ordinal",
        "augment_binding": "EXPLICIT_UNSUPPORTED_AUGMENT_V1",
    }
    destinations = dict(direct_destinations)
    destinations.update({key: "protection.resistances" for key in RESISTANCE_KEYS})
    destinations.update({key: "weapon.elemental" for key in WEAPON_ELEMENT_KEYS})
    destinations.update({key: "skill_modifiers.modifiers" for key in SKILL_MODIFIER_KEYS})
    if set(destinations) != candidate_fields:
        raise ValueError(f"B1 field coverage mismatch missing={candidate_fields-set(destinations)} extra={set(destinations)-candidate_fields}")
    evidence = {
        "status": "TYPED_CANDIDATE_MEASUREMENT_NOT_PRODUCTION_ACCEPTANCE",
        "pinned_inputs": {
            "b1_catalog_sha256": catalog_sha256,
            "item_family_registry_sha256": registry_sha256,
            "source_xml_sha256": atoms.get("sha256", atoms.get("expected_sha256")),
        },
        "retained_core_compatibility": {
            "candidate_body_version": 2,
            "legacy_v1_v2_v3_codec_mutation": "NONE",
            "single_record_layout": "version2 + retained ReferenceItemDefinition core + typed group extension",
            "server_core_max_bytes_after_shared_version": 5,
            "client_core_bytes_after_shared_version": 2,
            "server_fields": ["physical_class", "materializable", "stack_class", "legal_destinations"],
            "client_fields": ["physical_class", "stack_class"],
            "identity_only_guard": "Unknown legacy physical or stack requires both Unknown, materializable=false, destinations empty. It does not force verified typed fields to UNKNOWN; only a record with all typed groups UNKNOWN is identity-only.",
            "materialization_guard": "materializable iff CharacterInventory destination; destination maximum 1",
            "stack_cross_validation": "Known typed stackable must agree with retained NonStackable/StackCapable; retained Unknown permits truthful partial typed knowledge while remaining nonmaterializable",
        },
        "bounds": bounds.__dict__,
        "equipment_domain_status": "SOURCE-RESOLVED: 10 slots, 5 base vocations, up to 9 unique additional reservations per pattern. Complete 38,157-row normalized census derives v1 P=2; fixtures remain synthetic schema witnesses, not promoted Item facts.",
        "equipment_source_preflight": {
            "packet_id": "D6_M1_REFERENCE_ITEM_EQUIPMENT_SOURCE_PREFLIGHT/v1",
            "reviewed_scratch_packet_sha256": "e3055a714e3469c4a52bc5c1828a0a62e862d5c6daca826af728cadfb7166a98",
            "conclusions": {"semantic_slots": 10, "base_vocations": 5, "max_unique_additional_slots": 9},
        },
        "equipment_census": census,
        "presentation_atom_census": atoms,
        "vocation_scope_note": "The five-value domain covers base reference vocation families only. NoVocation, promotion/state predicates and unrestricted semantics remain distinct UNKNOWN/unaccepted dispositions; the synthetic worst shape is not a real-Item restriction claim.",
        "equipment_shape_note": "Each pattern carries truth-state primary slot, additional reservations, mutually-exclusive semantic group keys, base-vocation set, level requirement and compatibility rule. Group keys use the existing bounded canonical ProductionKey grammar; they are not slot IDs and create no parallel registry. The admitted corpus has zero explicit group keys (UNKNOWN, not known absence), so G=max(0, minimum plural schema witness 2)=2. Synthetic max-size keys are codec witnesses only.",
        "closed_vocabulary_derivation": "55 pinned modifier-family candidates = 1 explicit unsupported augment_binding + 12 typed protection entries + 5 typed Weapon elemental entries + 37 typed SkillModifier definitions. Every SkillModifier definition carries truth-state target domain, evaluation phase, priority and a closed typed parameter variant. Exact evaluation bindings/formulas remain UNKNOWN and no donor value is promoted.",
        "numeric_type_policy": "Percent-bearing fields use exact signed numerator/nonzero u64 denominator rationals; points, cells and milliseconds are distinct wrapper types. No global percentage scale or gameplay formula is inferred.",
        "typed_modifier_registry": {
            "resistance_ids": {str(index + 1): key for index, key in enumerate(RESISTANCE_KEYS)},
            "weapon_element_ids": {str(index + 1): key for index, key in enumerate(WEAPON_ELEMENT_KEYS)},
            "skill_modifier_ids": {
                str(index + 1): {"stable_key": key, "parameter_shape": modifier_parameter_shape(key)}
                for index, key in enumerate(SKILL_MODIFIER_KEYS)
            },
            "definition_shape": "stable closed kind key + FieldState<TargetDomainId> + FieldState<EvaluationPhaseId> + FieldState<i16 priority> + FieldState<closed typed parameter>",
            "execution_binding": "UNKNOWN_PENDING_RULESET; structural support does not promote donor arithmetic",
            "target_domain_capacity_ids": {str(index + 1): key for index, key in enumerate(MODIFIER_TARGET_CODEC_KEYS)},
            "evaluation_phase_capacity_ids": {str(index + 1): key for index, key in enumerate(MODIFIER_PHASE_CODEC_KEYS)},
            "capacity_id_note": "Profile-local closed capacity keys only; Reference target/phase bindings remain UNKNOWN until accepted by the owning ruleset.",
            "explicit_unsupported_modifier_candidates": ["augment_binding"],
            "imbuement_family_ids": {str(index + 1): key for index, key in enumerate(IMBUEMENT_FAMILY_CANDIDATE_KEYS)},
        },
        "closed_codec_registries": {
            "capability_ids": {str(index + 1): key for index, key in enumerate(CLASSIFICATION_KEYS)},
            "item_type_ids": {str(index + 1): key for index, key in enumerate(ITEM_TYPE_CANDIDATE_KEYS)},
            "weapon_type_ids": {str(index + 1): key for index, key in enumerate(WEAPON_TYPE_CANDIDATE_KEYS)},
            "ammo_type_ids": {str(index + 1): key for index, key in enumerate(AMMO_TYPE_CANDIDATE_KEYS)},
            "fluid_type_ids": {str(index + 1): key for index, key in enumerate(FLUID_TYPE_CANDIDATE_KEYS)},
            "equipment_slot_ids": {str(index + 1): key for index, key in enumerate(EQUIPMENT_SLOT_KEYS)},
            "base_vocation_ids": {str(index + 1): key for index, key in enumerate(BASE_VOCATION_KEYS)},
        },
        "atom_limits": {
            "presentation_name_utf8_bytes": bounds.name_bytes,
            "presentation_description_utf8_bytes": bounds.description_bytes,
            "presentation_aliases": "EXPLICIT_UNSUPPORTED_IN_V1_NO_SOURCE_VALUES",
            "presentation_tags": "EXPLICIT_UNSUPPORTED_IN_V1_NO_SOURCE_VALUES",
            "equipment_group_key_utf8_bytes": bounds.equipment_group_key_bytes,
            "carrier_production_key_utf8_bytes": max_key_bytes,
            "carrier_revision_and_manifest_atom_utf8_bytes": max_atom_bytes,
        },
        "client_projection_allowlist": [
            "presentation", "classification", "physical", "stack", "equipment", "weapon",
            "protection", "skill_modifiers", "charges", "container", "imbuement",
        ],
        "client_projection_denied": ["temporal", "use_transform", "trade_restrictions", "fluid", "readable_writeable"],
        "equipment_pattern_formula": {
            "valid_formula_range": "1 <= P <= 255 due one-byte count encoding; 255 is incidental wire capacity, not a production cap",
            "server_record_bytes": f"{len(p1_server)} + {pattern_server_increment}*(P-1)",
            "client_record_bytes": f"{len(p1_client)} + {pattern_client_increment}*(P-1)",
            "selected_v1_P": bounds.equipment_patterns,
            "selection_basis": "max(complete admitted corpus normalized maximum 1, minimum non-singleton schema witness 2)",
            "overflow": "fail closed before allocation; preserve evidence; no truncate, UNKNOWN coercion or heuristic merge; versioned successor profile",
        },
        "explicit_unsupported_v1": [
            "presentation.appearance_binding", "presentation.aliases", "presentation.tags",
            "equipment.compatibility_rule", "modifier.augment_binding",
            "trade_restrictions.account_binding_policy", "trade_restrictions.character_binding_policy",
        ],
        "temporal_modes": {"1": "DURABLE_ABSOLUTE_DEADLINE_SEMANTICS", "2": "AUTHORITATIVE_ACTIVE_TIME_BUDGET_SEMANTICS"},
        "temporal_scope": "Definition records encode time-consumption semantics and duration parameters only; mutable instance deadlines/budgets remain DUR/FND-owned.",
        "durability_scope": "Mutable durability and mutable binding state are ItemInstance/DUR-owned and absent from the current B1 immutable Item candidate family; no second mutable-state system is introduced. Immutable definition account/character binding policies are explicit unsupported in v1.",
        "b1_candidate_field_coverage": {
            "input_catalog_sha256": catalog_sha256,
            "candidate_fields": len(candidate_fields),
            "exactly_one_destination_each": "PASS",
            "destinations": dict(sorted(destinations.items())),
        },
        "worst_shape": {
            "server_record_bytes": len(worst_server), "client_record_bytes": len(worst_client),
            "server_sha256": hashlib.sha256(worst_server).hexdigest(), "client_sha256": hashlib.sha256(worst_client).hexdigest(),
            "server_body_section_bytes": server_body_max,
            "client_body_section_bytes": client_body_max,
            "provable_max_server_artifact_bytes": provable_server_artifact,
            "provable_max_client_artifact_bytes": provable_client_artifact,
            "provable_max_generation_pair_bytes": provable_pair,
        },
        "carrier_resource_arithmetic": {
            "fixed_envelope_bytes": fixed_envelope,
            "preserved_max_manifest_bytes": max_manifest_bytes,
            "preserved_max_key_bytes": max_key_bytes,
            "preserved_max_atom_bytes": max_atom_bytes,
            "provable_max_index_bytes": max_index_bytes,
            "observed_protected_index_bytes": observed_index_bytes,
            "observed_server_manifest_bytes": observed_server_manifest,
            "observed_client_manifest_bytes": observed_client_manifest,
            "observed_shape_scenario_server_artifact_bytes": scenario_server_artifact,
            "observed_shape_scenario_client_artifact_bytes": scenario_client_artifact,
            "observed_shape_scenario_pair_bytes": scenario_server_artifact + scenario_client_artifact,
            "scenario_note": "Observed index/manifest arithmetic is retained separately and is not a production maximum.",
        },
        "representative_typed_round_trips": results,
        "retained_core_round_trips": core_results,
        "fixture_truth_state_coverage": {
            "field_states": [tag.name for tag in sorted(truth_tags, key=int)],
            "known_value_shapes": sorted(known_shapes),
            "fixture_authority": "SYNTHETIC_SCHEMA_WITNESSES_NOT_SOURCE_OR_GAMEPLAY_FACTS",
        },
        "boundary_checks": vector_checks,
        "deterministic_repeat": "PASS" if encode(worst, "server") == worst_server else "FAIL",
        "remaining_blocker": "Production Rust implementation/review. Augment binding, presentation binding/aliases/tags and equipment compatibility grammar remain explicit unsupported in v1; every other B1 candidate field has one bounded typed destination.",
    }
    output.write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps({"output": str(output), **evidence["worst_shape"], "checks": vector_checks}, indent=2))


if __name__ == "__main__":
    main()
