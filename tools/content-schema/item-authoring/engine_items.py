"""Convert pinned Crystal/Canary engine item definitions into candidate v4 Item bundles.

Reads `items.xml` (expanding `fromid`/`toid` ranges the way the engine does) and decodes
the `appearances.dat` protobuf (`src/protobuf/appearances.proto` at the pinned Canary
revision; Crystal shares the same wire format). Every input artifact is digest-verified
before it is read (`read_verified_artifact`): text artifacts (`items.xml`, the two
Delivery Task pool files) are verified and parsed after CRLF->LF normalization, matching
Git's canonical text-blob bytes, so an autocrlf=true checkout digests identically to an
LF one; `appearances.dat` is always verified and read as exact raw bytes and is never
normalized. A missing pinned artifact is a hard error, never an empty result.

Identity reproduces the committed Content World B1 allocator: ascending `source_item_id`
from `docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json`,
a semantic id takes its native key from `NATIVE_ITEM_BATCH` in
`apps/game-server/src/content/cw2_b1_import.rs`, everything else gets the next opaque
`oteryn:item.registry.i%08d`. Field mapping follows the pinned
`crystal-field-dispositions.json` / `canary-field-dispositions.json` ledgers: a field whose
disposition is not `ITEM_TYPED`/mapped is counted as routed to its owner, never a blocker;
an `ITEM_TYPED` field this converter does not implement is `converter_missing:<field>`.

Each engine reads its own pinned Delivery Task pool as upstream observation only (Canary:
the `weeklyItems` table in `data/modules/scripts/taskboard/settings.lua`; Crystal: its
existing delivery list); pool membership never decides eligibility by itself. The final
`delivery_task_eligible` boolean is the owner-approved authoring rule
`ADOPT_CRYSTAL_DELIVERY_LIST@ff7ede5` (see `RULE_ID`): an Item is eligible iff the Crystal
id sharing its CW2 B1 allocator key is a member of the digest-pinned Crystal delivery list
at `ff7ede593c69d4c658b382c97443e8155926924a`, unless `delivery-task-overrides.json`
records an explicit per-Item exception. Crystal runs read that list as their own pool;
Canary runs require a separate `--rule-source <crystal checkout>` to read the same pinned
list, and refuse to run without it rather than silently deciding nothing is eligible. A
converted Item that resolves a decision carries `delivery_task_eligible`; the per-item
report keeps the upstream observation and the decision (`rule`, `basis`, `eligible`,
optional `reason`) separate. An item with no CW2 B1 allocator key gets no decision at all
and keeps the `delivery_task_decision_not_admitted` blocker.

This is evidence tooling: it proves what the pinned engine sources say, not Game truth.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import xml.etree.ElementTree as ET
from fractions import Fraction
from pathlib import Path

from source_field_catalogs import (
    CANARY_PROFILE,
    CRYSTAL_PROFILE,
    ENGINE_ARTIFACT_DIGESTS,
    TIBIAWIKI_ITEM_BINDINGS,
)

ROOT = Path(__file__).resolve().parent
REPO_ROOT = ROOT.parents[2]
IDENTITY_CATALOG_PATH = (
    REPO_ROOT
    / "docs/agents/evidence"
    / "OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
)
NATIVE_BATCH_RS_PATH = "apps/game-server/src/content/cw2_b1_import.rs"
DEFINITION_REVISION = "definition-r1"

ENGINES = {
    "crystal": {
        "profile": CRYSTAL_PROFILE,
        "repository": "zimbadev/crystalserver",
        "revision": "ff7ede593c69d4c658b382c97443e8155926924a",
    },
    "canary": {
        "profile": CANARY_PROFILE,
        "repository": "opentibiabr/canary",
        "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
    },
}
DELIVERY_LIST_PATH = "data/scripts/lib/task_board_delivery_items.lua"
DELIVERY_LIST_SHA256 = (
    "7b30362470893f6e3ce6bdc0237ac5d3f13d6e5288ce10591a93fde2013b688d"
)
CANARY_WEEKLY_ITEMS_PATH = "data/modules/scripts/taskboard/settings.lua"
CANARY_WEEKLY_ITEMS_SHA256 = (
    "9ae88e3a3ee6d0baecfc26adc806f5b59d54bf1491348f67e6f00e32894fe504"
)
DELIVERY_POOL_PATH = {"crystal": DELIVERY_LIST_PATH, "canary": CANARY_WEEKLY_ITEMS_PATH}
DELIVERY_SOURCE_NAME = {
    "crystal": "crystal_delivery_list",
    "canary": "canary_task_board_weekly_items",
}

# --- Delivery Task adoption rule (owner-approved authoring decision) ---------------
#
# `RULE_ID` is the accepted Oteryn authoring rule: an Item is `delivery_task_eligible`
# iff the Crystal id sharing its CW2 B1 allocator key is a member of the digest-pinned
# Crystal delivery list at the revision named in the rule id. Both engines already share
# one numeric id space through that allocator (see `build_identity_index`), so the same
# `item_id` used to resolve identity is also the id looked up in the Crystal list.
RULE_ID = "ADOPT_CRYSTAL_DELIVERY_LIST@ff7ede5"
DELIVERY_OVERRIDES_SCHEMA = "OTERYN_ITEM_DELIVERY_TASK_OVERRIDES/v1"
DELIVERY_OVERRIDES_PATH = ROOT / "delivery-task-overrides.json"

# Text artifacts are digested/parsed after CRLF->LF normalization (Git's canonical text
# blob bytes), so an autocrlf=true checkout digests identically to an LF one. Every other
# pinned artifact (appearances.dat) is digested/parsed as exact raw bytes and must never
# be normalized: a binary artifact may legitimately contain the byte sequence 0d 0a.
TEXT_ARTIFACTS = {"data/items/items.xml", DELIVERY_LIST_PATH, CANARY_WEEKLY_ITEMS_PATH}


def normalize_text_bytes(data):
    return data.replace(b"\r\n", b"\n")


def read_verified_artifact(source_root, relative_path, expected_digest):
    """Digest-verify one pinned source artifact; return (payload_bytes, digest_mode)."""
    path = source_root / relative_path
    if not path.is_file():
        raise SystemExit(f"missing required source artifact: {relative_path}")
    raw = path.read_bytes()
    if relative_path in TEXT_ARTIFACTS:
        payload, digest_mode = normalize_text_bytes(raw), "text_lf_normalized"
    else:
        payload, digest_mode = raw, "raw_bytes"
    actual = hashlib.sha256(payload).hexdigest()
    if actual != expected_digest:
        raise SystemExit(
            f"digest mismatch for {relative_path}: expected {expected_digest}, got {actual}"
        )
    return payload, digest_mode


# --- items.xml -----------------------------------------------------------------


def load_items_xml(text):
    """Expand fromid/toid ranges; split root vs. nested `script` sub-attributes."""
    items = {}
    root = ET.fromstring(text)
    for node in root.iter("item"):
        root_attrs = {}
        script_attrs = {}
        for attribute in node.findall("attribute"):
            key = attribute.get("key", "").lower()
            if key == "script":
                for child in attribute.findall("attribute"):
                    child_key = child.get("key", "").lower()
                    script_attrs.setdefault(child_key, child.get("value"))
                continue
            root_attrs.setdefault(key, attribute.get("value"))
        attrs = {**script_attrs, **root_attrs}
        record = {
            "name": node.get("name"),
            "article": node.get("article"),
            "plural": node.get("plural"),
            "attrs": attrs,
        }
        if node.get("id") is not None:
            ids = [int(node.get("id"))]
        else:
            low, high = int(node.get("fromid")), int(node.get("toid"))
            ids = list(range(low, high + 1)) if low <= high else []
        for item_id in ids:
            items[item_id] = record
    return items


# --- appearances.dat (protobuf, see appearances.proto) --------------------------

FLAG_BOOL_FIELDS = {
    2: "clip",
    3: "bottom",
    4: "top",
    5: "container",
    6: "cumulative",
    7: "usable",
    8: "forceuse",
    9: "multiuse",
    12: "liquidpool",
    13: "unpass",
    14: "unmove",
    15: "unsight",
    16: "avoid",
    17: "no_movement_animation",
    18: "take",
    19: "liquidcontainer",
    20: "hang",
    22: "rotate",
    24: "dont_hide",
    25: "translucent",
    28: "lying_object",
    29: "animate_always",
    32: "fullbank",
    33: "ignore_look",
    37: "wrap",
    38: "unwrap",
    39: "topeffect",
    42: "corpse",
    43: "player_corpse",
    45: "ammo",
    46: "show_off_socket",
    47: "reportable",
    49: "reverse_addons_east",
    50: "reverse_addons_west",
    51: "reverse_addons_south",
    52: "reverse_addons_north",
    53: "wearout",
    54: "clockexpire",
    55: "expire",
    56: "expirestop",
    57: "wrapkit",
    59: "dual_wielding",
}
# Each: dotted prefix, {inner field number: (suffix, kind)}. kind "u"=uint, "s"=bytes-as-str.
FLAG_SUBMESSAGE_FIELDS = {
    1: ("bank", {1: ("waypoints", "u")}),
    10: ("write", {1: ("max_text_length", "u")}),
    11: ("write_once", {1: ("max_text_length_once", "u")}),
    21: ("hook", {1: ("direction", "u")}),
    23: ("light", {1: ("brightness", "u"), 2: ("color", "u")}),
    26: ("shift", {1: ("x", "u"), 2: ("y", "u")}),
    27: ("height", {1: ("elevation", "u")}),
    30: ("automap", {1: ("color", "u")}),
    31: ("lenshelp", {1: ("id", "u")}),
    34: ("clothes", {1: ("slot", "u")}),
    35: ("default_action", {1: ("action", "u")}),
    36: (
        "market",
        {
            1: ("category", "u"),
            2: ("trade_as_object_id", "u"),
            3: ("show_as_object_id", "u"),
            5: ("restrict_to_profession", "u*"),
            6: ("minimum_level", "u"),
        },
    ),
    41: ("changedtoexpire", {1: ("former_object_typeid", "u")}),
    44: ("cyclopediaitem", {1: ("cyclopedia_type", "u")}),
    48: ("upgradeclassification", {1: ("upgrade_classification", "u")}),
    58: ("skillwheel_gem", {1: ("gem_quality_id", "u"), 2: ("vocation_id", "u")}),
    61: ("proficiency", {1: ("proficiency_id", "u")}),
}
FLAG_REPEATED_SUBMESSAGE = {
    40: (
        "npcsaledata",
        {
            1: ("name", "s"),
            2: ("location", "s"),
            3: ("sale_price", "u"),
            4: ("buy_price", "u"),
            5: ("currency_object_type_id", "u"),
            6: ("currency_quest_flag_display_name", "s"),
        },
    )
}
ITEM_CATEGORY_NAMES = {
    1: "armors",
    2: "amulets",
    3: "boots",
    4: "containers",
    5: "decoration",
    6: "food",
    7: "helmets_hats",
    8: "legs",
    9: "others",
    10: "potions",
    11: "rings",
    12: "runes",
    13: "shields",
    14: "tools",
    15: "valuables",
    16: "ammunition",
    17: "axes",
    18: "clubs",
    19: "distance_weapons",
    20: "swords",
    21: "wands_rods",
    22: "premium_scrolls",
    23: "tibia_coins",
    24: "creature_products",
    25: "quiver",
    26: "twohandweapon",
    27: "fist_weapons",
    28: "backpack",
    29: "onehandweapon",
    30: "arrow",
    31: "soulcores",
}
PLAYER_PROFESSION_NAMES = {
    -1: "any",
    0: "none",
    1: "knight",
    2: "paladin",
    3: "sorcerer",
    4: "druid",
    5: "monk",
    10: "promoted",
}
PLAYER_ACTION_NAMES = {
    0: "none",
    1: "look",
    2: "use",
    3: "open",
    4: "autowalk_highlight",
}


def varint(data, pos):
    shift = result = 0
    while True:
        byte = data[pos]
        pos += 1
        result |= (byte & 0x7F) << shift
        if byte < 0x80:
            return result, pos
        shift += 7


def zigzag_uint_to_signed(value):
    # Proto2 enums with negative values (e.g. PLAYER_PROFESSION_ANY = -1) are encoded
    # as plain varints, not zigzag; a small negative shows up as a huge uint64.
    if value >= 1 << 63:
        return value - (1 << 64)
    return value


def protobuf_fields(data):
    pos = 0
    while pos < len(data):
        key, pos = varint(data, pos)
        number, wire = key >> 3, key & 7
        if wire == 0:
            value, pos = varint(data, pos)
        elif wire == 2:
            size, pos = varint(data, pos)
            value, pos = data[pos : pos + size], pos + size
        elif wire == 5:
            value, pos = data[pos : pos + 4], pos + 4
        elif wire == 1:
            value, pos = data[pos : pos + 8], pos + 8
        else:
            raise ValueError(f"unsupported protobuf wire type {wire}")
        yield number, value


def decode_submessage(data, field_map):
    out = {}
    for number, value in protobuf_fields(data):
        entry = field_map.get(number)
        if entry is None:
            continue
        suffix, kind = entry
        if kind == "u":
            out[suffix] = zigzag_uint_to_signed(value)
        elif kind == "u*":
            out.setdefault(suffix, []).append(zigzag_uint_to_signed(value))
        elif kind == "s":
            out[suffix] = value.decode("utf-8", "replace")
    return out


def decode_flags(data):
    """Flatten AppearanceFlags into {dotted_field_name: value}; presence == key in dict."""
    flat = {}
    for number, value in protobuf_fields(data):
        if number in FLAG_BOOL_FIELDS:
            flat[f"flags.{FLAG_BOOL_FIELDS[number]}"] = bool(value)
            continue
        submessage = FLAG_SUBMESSAGE_FIELDS.get(number)
        if submessage:
            prefix, field_map = submessage
            flat[f"flags.{prefix}"] = True
            for suffix, inner_value in decode_submessage(value, field_map).items():
                flat[f"{prefix}.{suffix}"] = inner_value
            continue
        repeated = FLAG_REPEATED_SUBMESSAGE.get(number)
        if repeated:
            prefix, field_map = repeated
            flat.setdefault(f"_{prefix}_entries", []).append(
                decode_submessage(value, field_map)
            )
            for suffix in field_map.values():
                pass
    for prefix, field_map in FLAG_REPEATED_SUBMESSAGE.values():
        entries = flat.pop(f"_{prefix}_entries", [])
        if entries:
            for suffix, _ in field_map.values():
                if any(suffix in entry for entry in entries):
                    flat[f"{prefix}.{suffix}"] = [
                        entry.get(suffix) for entry in entries
                    ]
    return flat


def decode_frame_groups(appearance_bytes):
    groups = []
    for number, value in protobuf_fields(appearance_bytes):
        if number != 2:
            continue
        fixed = None
        sprite_info = None
        for fg_num, fg_value in protobuf_fields(value):
            if fg_num == 1:
                fixed = fg_value
            elif fg_num == 3:
                sprite_info = fg_value
        if sprite_info is None:
            continue
        width = height = depth = layers = 1
        sprite_ids = []
        is_opaque = False
        bounding_square = None
        phase_count = 1
        for si_num, si_value in protobuf_fields(sprite_info):
            if si_num == 1:
                width = si_value
            elif si_num == 2:
                height = si_value
            elif si_num == 3:
                depth = si_value
            elif si_num == 4:
                layers = si_value
            elif si_num == 5:
                sprite_ids.append(si_value)
            elif si_num == 7:
                bounding_square = si_value
            elif si_num == 8:
                is_opaque = bool(si_value)
            elif si_num == 6:
                phase_count = max(
                    1, sum(1 for n, _ in protobuf_fields(si_value) if n == 6)
                )
        groups.append(
            {
                "fixed_frame_group": fixed if fixed is not None else 0,
                "geometry": {
                    "pattern_width": width,
                    "pattern_height": height,
                    "pattern_depth": depth,
                    "layers": layers,
                    "phase_count": phase_count,
                    "is_opaque": is_opaque,
                    **(
                        {"bounding_square": bounding_square}
                        if bounding_square is not None
                        else {}
                    ),
                },
                "sprite_ids": sprite_ids,
            }
        )
    return groups


def decode_appearance_object(value):
    record = {"flags": {}, "frame_groups": []}
    for number, inner in protobuf_fields(value):
        if number == 1:
            record["id"] = inner
        elif number == 2:
            record["frame_groups"] = decode_frame_groups(value)
        elif number == 3:
            record["flags"] = decode_flags(inner)
        elif number == 4:
            record["name"] = inner.decode("utf-8", "replace")
        elif number == 5:
            record["description"] = inner.decode("utf-8", "replace")
    return record


def load_appearance_objects(data):
    """Decode the `object` (family 1) appearances into {id: decoded record}."""
    objects = {}
    for number, value in protobuf_fields(data):
        if number != 1:
            continue
        record = decode_appearance_object(value)
        if "id" in record:
            objects[record["id"]] = record
    return objects


# --- identity allocator ----------------------------------------------------------


def load_native_item_batch():
    text = None
    working_tree = REPO_ROOT / NATIVE_BATCH_RS_PATH
    if working_tree.is_file():
        text = working_tree.read_text(encoding="utf-8")
    if text is None or "NATIVE_ITEM_BATCH" not in text:
        text = subprocess.run(
            [
                "git",
                "-C",
                str(REPO_ROOT),
                "show",
                f"origin/main:{NATIVE_BATCH_RS_PATH}",
            ],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    start = text.index("const NATIVE_ITEM_BATCH")
    end = text.index("\n];", start)
    block = text[start:end]
    pattern = (
        r'source_item_id:\s*(\d+),\s*source_label:\s*"[^"]*",\s*'
        r'item_class:\s*"[^"]*",\s*native_key:\s*"([^"]*)"'
    )
    return {
        int(item_id): native_key for item_id, native_key in re.findall(pattern, block)
    }


def load_identity_ascending_ids():
    catalog = json.loads(IDENTITY_CATALOG_PATH.read_text(encoding="utf-8"))
    return [
        record["source_item_id"]
        for record in catalog["semantic_catalog"]["identity_records"]
    ]


def build_identity_index():
    """{source_item_id: (key, basis)} reproducing the committed B1 allocator exactly."""
    native = load_native_item_batch()
    tibiawiki_targets = set(TIBIAWIKI_ITEM_BINDINGS.values())
    index = {}
    opaque_sequence = 0
    for source_item_id in load_identity_ascending_ids():
        if source_item_id in native:
            key = native[source_item_id]
        else:
            opaque_sequence += 1
            key = f"oteryn:item.registry.i{opaque_sequence:08d}"
        basis = (
            "tibiawiki_exact_binding"
            if key in tibiawiki_targets
            else "cw2_b1_allocator_reproduced"
        )
        index[source_item_id] = (key, basis)
    return index


def item_ref(identity_index, item_id):
    entry = identity_index.get(item_id)
    if entry is None:
        return None
    return {"family": "Item", "key": entry[0], "revision": DEFINITION_REVISION}


# --- delivery task evidence (Canary Task Board / Crystal delivery list) ------------


def parse_crystal_delivery_ids(text):
    return {int(match) for match in re.findall(r"itemId\s*=\s*(\d+)", text)}


def parse_canary_weekly_item_ids(text):
    """Parse only the `weeklyItems = { ... }` table; ignore every other `id =` in the
    file (e.g. `shopOffers`), which is not the Delivery Task pool."""
    opening = re.search(r"weeklyItems\s*=\s*\{", text)
    if opening is None:
        raise SystemExit(f"weeklyItems table not found in {CANARY_WEEKLY_ITEMS_PATH}")
    depth = 0
    end = None
    for pos in range(opening.end() - 1, len(text)):
        if text[pos] == "{":
            depth += 1
        elif text[pos] == "}":
            depth -= 1
            if depth == 0:
                end = pos
                break
    if end is None:
        raise SystemExit(
            f"unterminated weeklyItems table in {CANARY_WEEKLY_ITEMS_PATH}"
        )
    block = text[opening.end() - 1 : end + 1]
    return {int(value) for value in re.findall(r"\bid\s*=\s*(\d+)", block)}


DELIVERY_POOL_PARSER = {
    "crystal": parse_crystal_delivery_ids,
    "canary": parse_canary_weekly_item_ids,
}


def load_delivery_overrides(path, valid_keys):
    """Strictly parse and validate the per-item Delivery Task override file.

    Returns `{item_key: {"eligible": bool, "reason": str}}`. Any unknown top-level or
    per-entry key, wrong type, empty reason, mismatched `schema`/`rule`, or override key
    that is not a known Item key (from the CW2 B1 allocator) is a hard error: an override
    file is never partially trusted.
    """
    if not path.is_file():
        raise SystemExit(f"missing delivery task overrides file: {path}")

    def unique_object(pairs):
        keys = [pair_key for pair_key, _value in pairs]
        duplicates = sorted({k for k in keys if keys.count(k) > 1})
        if duplicates:
            raise SystemExit(f"duplicate JSON key(s) in {path}: {duplicates}")
        return dict(pairs)

    try:
        payload = json.loads(
            path.read_text(encoding="utf-8"), object_pairs_hook=unique_object
        )
    except json.JSONDecodeError as exc:
        raise SystemExit(f"invalid JSON in delivery task overrides file {path}: {exc}")

    if not isinstance(payload, dict):
        raise SystemExit(f"delivery task overrides file {path} must be a JSON object")
    allowed_top_keys = {"schema", "rule", "overrides"}
    unknown_top = sorted(set(payload) - allowed_top_keys)
    if unknown_top:
        raise SystemExit(f"unknown top-level key(s) in {path}: {unknown_top}")
    missing_top = sorted(allowed_top_keys - set(payload))
    if missing_top:
        raise SystemExit(f"missing required key(s) in {path}: {missing_top}")
    if payload["schema"] != DELIVERY_OVERRIDES_SCHEMA:
        raise SystemExit(
            f"unexpected 'schema' in {path}: {payload['schema']!r} "
            f"(expected {DELIVERY_OVERRIDES_SCHEMA!r})"
        )
    if payload["rule"] != RULE_ID:
        raise SystemExit(
            f"unexpected 'rule' in {path}: {payload['rule']!r} (expected {RULE_ID!r})"
        )

    overrides_raw = payload["overrides"]
    if not isinstance(overrides_raw, dict):
        raise SystemExit(f"'overrides' in {path} must be a JSON object")

    allowed_entry_keys = {"eligible", "reason"}
    overrides = {}
    for key, entry in overrides_raw.items():
        if not isinstance(key, str) or not key:
            raise SystemExit(f"invalid override key in {path}: {key!r}")
        if key not in valid_keys:
            raise SystemExit(f"override in {path} targets unknown Item key: {key!r}")
        if not isinstance(entry, dict):
            raise SystemExit(f"override for {key!r} in {path} must be a JSON object")
        unknown_entry = sorted(set(entry) - allowed_entry_keys)
        if unknown_entry:
            raise SystemExit(
                f"unknown key(s) in override for {key!r} in {path}: {unknown_entry}"
            )
        missing_entry = sorted(allowed_entry_keys - set(entry))
        if missing_entry:
            raise SystemExit(
                f"missing key(s) in override for {key!r} in {path}: {missing_entry}"
            )
        eligible = entry["eligible"]
        if not isinstance(eligible, bool):
            raise SystemExit(
                f"override for {key!r} in {path}: 'eligible' must be a boolean"
            )
        reason = entry["reason"]
        if not isinstance(reason, str) or not reason.strip():
            raise SystemExit(
                f"override for {key!r} in {path}: 'reason' must be a non-empty string"
            )
        overrides[key] = {"eligible": eligible, "reason": reason}
    return overrides


# --- field dispositions (crystal-field-dispositions.json / canary-...) -------------

DISPOSITION_FILES = {
    CRYSTAL_PROFILE: "crystal-field-dispositions.json",
    CANARY_PROFILE: "canary-field-dispositions.json",
}


def load_disposition_catalog(profile):
    catalog = json.loads(
        (ROOT / DISPOSITION_FILES[profile]).read_text(encoding="utf-8")
    )
    return {row["source_field"]: row for row in catalog["fields"]}


# --- family_profile / taxonomy rule table ------------------------------------------
#
# Primary signal is the engine's own `primarytype` attribute, correlated against the
# navigation_families each profile lists in profile-catalog.json (BR client menu
# families). This is a heuristic crosswalk, not an authoritative per-value binding;
# a primarytype this table does not recognize, and every item lacking primarytype that
# no fallback rule below resolves, is left unresolved rather than guessed.

PRIMARYTYPE_PROFILE = {
    "helmets": "equipment_armor",
    "boots": "equipment_armor",
    "armors": "equipment_armor",
    "legs": "equipment_armor",
    "shields": "equipment_offhand",
    "spellbooks": "equipment_offhand",
    "amulets": "equipment_offhand",
    "amulets and necklaces": "equipment_offhand",
    "rings": "equipment_offhand",
    "quivers": "container_equipment",
    "axe weapons": "weapon_melee",
    "club weapons": "weapon_melee",
    "sword weapons": "weapon_melee",
    "fist weapons": "weapon_melee",
    "training weapons": "weapon_melee",
    "exercise weapons": "weapon_melee",
    "distance weapons": "weapon_distance",
    "ammunition": "weapon_distance",
    "wands": "weapon_magic",
    "rods": "weapon_magic",
    "attack runes": "rune",
    "support runes": "rune",
    "healing runes": "rune",
    "books": "document",
    "documents and papers": "document",
    "signs": "document",
    "containers": "container",
    "fluid containers": "container",
    "decoration": "decoration",
    "dolls and bears": "decoration",
    "trophies": "decoration",
    "musical instruments": "decoration",
    "floor decorations": "decoration",
    "furniture": "decoration",
    "statues": "decoration",
    "wall hangings": "decoration",
    "flags": "decoration",
    "painting equipment": "decoration",
    "closets": "decoration",
    "casks": "decoration",
    "tables": "decoration",
    "pillars": "decoration",
    "coffins": "decoration",
    "party items": "decoration",
    "contest prizes": "event_collectible",
    "fansite items": "event_collectible",
    "tournament rewards": "event_collectible",
    "game tokens": "event_collectible",
    "enchanted items": "transformation_item",
    "magical items": "transformation_item",
    "quest items": "quest_item",
    "quest objects": "quest_item",
    "valuables": "material_valuable",
    "creature products": "material_valuable",
    "natural products": "material_valuable",
    "metals": "material_valuable",
    "remains": "material_valuable",
    "flora and minerals": "material_valuable",
    "rocks": "material_valuable",
    "annelids": "material_valuable",
    "arachnids": "material_valuable",
    "animals": "material_valuable",
    "bats": "material_valuable",
    "bears": "material_valuable",
    "birds": "material_valuable",
    "blobs": "material_valuable",
    "canines": "material_valuable",
    "demons": "material_valuable",
    "dragons": "material_valuable",
    "dreamhaunters": "material_valuable",
    "event creatures": "material_valuable",
    "ghosts": "material_valuable",
    "glires": "material_valuable",
    "hive born": "material_valuable",
    "mollusks": "material_valuable",
    "outlaws": "material_valuable",
    "skeletons": "material_valuable",
    "undead humanoids": "material_valuable",
    "ungulates": "material_valuable",
    "refuse": "trash",
    "rubbish": "trash",
    "keys": "key",
    "light sources": "light_source",
    "illumination": "light_source",
    "tools": "tool",
    "kitchen tools": "tool",
    "taming items": "tool",
    "utilities": "tool",
    "food": "food",
    "liquids": "fluid",
    "plants": "plant",
    "plants and herbs": "plant",
    "bushes": "plant",
    "cactuses": "plant",
    "ferns": "plant",
    "flowers": "plant",
    "grass": "plant",
    "trees": "plant",
    "mushrooms": "plant",
}
PROFILE_ITEM_CLASS = {
    "equipment_armor": "equipment",
    "equipment_offhand": "equipment",
    "container_equipment": "container",
    "weapon_melee": "weapon",
    "weapon_distance": "weapon",
    "weapon_magic": "weapon",
    "rune": "rune",
    "document": "document",
    "container": "container",
    "decoration": "decoration",
    "event_collectible": "collectible",
    "progression_material": "material",
    "transformation_item": "transformation",
    "quest_item": "quest_item",
    "material_valuable": "material",
    "trash": "trash",
    "key": "key",
    "light_source": "light_source",
    "tool": "tool",
    "food": "food",
    "fluid": "fluid",
    "plant": "plant",
}
WEAPON_TYPE_MAP = {
    "sword": "sword",
    "axe": "axe",
    "club": "club",
    "fist": "fist",
    "distance": "distance_launcher",
    "ammunition": "ammunition",
    "ammo": "ammunition",
    "missile": "thrown_missile",
    "wand": "wand",
}
WEAPON_TYPE_PROFILE = {
    "sword": "weapon_melee",
    "axe": "weapon_melee",
    "club": "weapon_melee",
    "fist": "weapon_melee",
    "distance": "weapon_distance",
    "ammunition": "weapon_distance",
    "ammo": "weapon_distance",
    "missile": "weapon_distance",
    "wand": "weapon_magic",
}
RAW_SLOT_TO_EQUIP_SLOT = {
    "head": "head",
    "armor": "armor",
    "legs": "legs",
    "feet": "feet",
    "necklace": "neck",
    "ring": "ring",
    "ammo": "ammo",
    "backpack": "back",
}
# The OT/Tibia protocol's stable CLOTHSLOT_* wire values (client-visible, unrelated to
# any Oteryn schema string); 5/6 are the generic left/right hand slots and are left to
# the ambiguous-hand fallback rather than guessed as a specific hand.
CLOTHES_SLOT_MAP = {
    1: "head",
    2: "neck",
    3: "back",
    4: "armor",
    7: "legs",
    8: "feet",
    9: "ring",
    10: "ammo",
}


def classify_family_profile(attrs, primarytype):
    if primarytype in PRIMARYTYPE_PROFILE:
        return PRIMARYTYPE_PROFILE[primarytype]
    weapon_type = attrs.get("weapontype")
    if weapon_type in WEAPON_TYPE_PROFILE:
        return WEAPON_TYPE_PROFILE[weapon_type]
    if attrs.get("type") == "rune":
        return "rune"
    if attrs.get("type") == "key":
        return "key"
    if "containersize" in attrs:
        return "container_equipment" if "slottype" in attrs else "container"
    return None


AMBIGUOUS_HAND_PATTERNS = {
    "patterns": [
        {"pattern_id": 1, "slot": "right_hand", "hands": 1},
        {"pattern_id": 2, "slot": "left_hand", "hands": 1},
    ]
}


def build_equipment(attrs, clothes_slot=None):
    """Compact slot/hands form; raw `hand` with no slotType is genuinely ambiguous."""
    slot = attrs.get("slot")
    slot_type = attrs.get("slottype")
    if slot == "hand" or (slot is None and slot_type in ("two-handed", "right-hand")):
        if slot_type == "two-handed":
            return {"slot": "right_hand", "hands": 2, "reserved_slots": ["left_hand"]}
        if slot_type == "right-hand":
            return {"slot": "right_hand", "hands": 1}
        return AMBIGUOUS_HAND_PATTERNS
    if slot == "shield":
        return {"slot": "left_hand", "hands": 1}
    if slot in RAW_SLOT_TO_EQUIP_SLOT:
        return {"slot": RAW_SLOT_TO_EQUIP_SLOT[slot], "hands": 0}
    if slot is None and slot_type is None and clothes_slot is not None:
        if clothes_slot in CLOTHES_SLOT_MAP:
            return {"slot": CLOTHES_SLOT_MAP[clothes_slot], "hands": 0}
        if clothes_slot in (5, 6):
            return AMBIGUOUS_HAND_PATTERNS
    return None


# --- numeric helpers -----------------------------------------------------------


def ratio_value(numerator, denominator=1):
    fraction = Fraction(int(numerator), int(denominator))
    return {"numerator": fraction.numerator, "denominator": fraction.denominator}


def weight_payload(raw_centioz):
    """centi-ounces -> "NN.NN" oz; negative source values are a display override."""
    magnitude = abs(raw_centioz)
    text = f"{'-' if raw_centioz < 0 else ''}{magnitude // 100}.{magnitude % 100:02d}"
    if raw_centioz < 0:
        return "display_weight", {"value": text, "unit": "oz"}
    return "weight", {"value": text, "unit": "oz"}


def to_int(value, default=0):
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def truthy(value):
    return str(value).strip().lower() not in ("0", "false", "", "no")


VOCATION_NAMES = {"knight", "paladin", "sorcerer", "druid", "monk"}


def parse_vocation_attribute(raw):
    """ "Knight;true, Elite Knight" -> {"knight"}; trailing promoted display names drop."""
    names = set()
    for part in raw.split(","):
        part = part.strip()
        if ";" not in part:
            continue
        name = part.split(";", 1)[0].strip().lower()
        if name in VOCATION_NAMES:
            names.add(name)
    return names


# --- resistances / skill / magic-level / leech maps ------------------------------

RESIST_SUFFIX_MAP = {
    "death": "death",
    "drown": "drowning",
    "earth": "earth",
    "energy": "energy",
    "fire": "fire",
    "healing": "healing",
    "holy": "holy",
    "ice": "ice",
    "lifedrain": "life_drain",
    "manadrain": "mana_drain",
    "physical": "physical",
    "poison": "earth",
}
RESIST_ALL_GROUP = ["physical", "energy", "earth", "fire", "ice", "holy", "death"]
RESIST_ELEMENTS_GROUP = ["energy", "earth", "fire", "ice"]
SKILL_MAP = {
    "skillsword": "sword",
    "skillaxe": "axe",
    "skillclub": "club",
    "skilldist": "distance",
    "skillfist": "fist",
    "skillshield": "shield",
    "skillfish": "fishing",
}
MAGIC_LEVEL_SUFFIX_MAP = {
    "deathmagiclevelpoints": "death",
    "earthmagiclevelpoints": "earth",
    "energymagiclevelpoints": "energy",
    "firemagiclevelpoints": "fire",
    "healingmagiclevelpoints": "healing",
    "holymagiclevelpoints": "holy",
    "icemagiclevelpoints": "ice",
    "physicalmagiclevelpoints": "physical",
}
ELEMENTAL_ATTACK_SUFFIX_MAP = {
    "elementdeath": "death",
    "elementearth": "earth",
    "elementenergy": "energy",
    "elementfire": "fire",
    "elementholy": "holy",
    "elementice": "ice",
}
CONDITION_SUPPRESS_MAP = {
    "suppresscurse": "curse",
    "suppressdazzle": "dazzle",
    "suppressdrown": "drown",
    "suppressdrunk": "drunk",
    "suppressenergy": "energy",
    "suppressfire": "fire",
    "suppressfreeze": "freeze",
    "suppressphysical": "physical",
    "suppresspoison": "poison",
}
# Field names this converter implements (both engines share the same source_field names
# for the ITEM_TYPED rows it acts on). Every other ITEM_TYPED/mapped field present on an
# item becomes a `converter_missing:<field>` blocker; every non-ITEM_TYPED disposition is
# routed automatically and is never a blocker.
IMPLEMENTED_FIELDS = {
    "name",
    "article",
    "plural",
    "weight",
    "movable",
    "pickupable",
    "allowpickupable",
    "flags.take",
    "primarytype",
    "description",
    "appearance.description",
    "appearance.name",
    "appearance.id",
    "appearance.frame_group",
    # These render-only appearance flags have no destination but the appearance_binding
    # PresentationRef itself; the frame_group geometry this converter always attaches
    # already carries the underlying rendering data they describe.
    "flags.animate_always",
    "flags.bottom",
    "flags.clip",
    "flags.dont_hide",
    "flags.lying_object",
    "flags.no_movement_animation",
    "flags.reverse_addons_east",
    "flags.reverse_addons_north",
    "flags.reverse_addons_south",
    "flags.reverse_addons_west",
    "flags.shift",
    "flags.top",
    "flags.topeffect",
    "flags.translucent",
    "shift.x",
    "shift.y",
    "weapontype",
    "flags.ammo",
    "slot",
    "slottype",
    "flags.clothes",
    "clothes.slot",
    "armor",
    "attack",
    "defense",
    "extradef",
    "range",
    "ammotype",
    "hitchance",
    "maxhitchance",
    "breakchance",
    "fromdamage",
    "todamage",
    "wandtype",
    *("absorbpercent" + suffix for suffix in RESIST_SUFFIX_MAP),
    "absorbpercentall",
    "absorbpercentelements",
    "fieldabsorbpercentenergy",
    "fieldabsorbpercentfire",
    "fieldabsorbpercentpoison",
    *SKILL_MAP,
    *MAGIC_LEVEL_SUFFIX_MAP,
    "magiclevelpoints",
    "magicpoints",
    *ELEMENTAL_ATTACK_SUFFIX_MAP,
    *CONDITION_SUPPRESS_MAP,
    "lifeleechamount",
    "lifeleechchance",
    "manaleechamount",
    "manaleechchance",
    "healthgain",
    "healthticks",
    "managain",
    "manaticks",
    "criticalhitchance",
    "criticalhitdamage",
    "cleavepercent",
    "manashield",
    "magicshieldcapacityflat",
    "magicshieldcapacitypercent",
    "invisible",
    "perfectshotdamage",
    "perfectshotrange",
    "speed",
    "containersize",
    "charges",
    "showcharges",
    "decayto",
    "duration",
    "destroyto",
    "transformequipto",
    "transformdeequipto",
    "transformonuse",
    "wrapableto",
    "wrapcontainer",
    "flags.wrap",
    "flags.unwrap",
    "flags.wrapkit",
    "imbuementslot",
    "flags.light",
    "light.brightness",
    "light.color",
    "readable",
    "writeable",
    "allowdistread",
    "maxtextlen",
    "writeonceitemid",
    "flags.usable",
    "flags.multiuse",
    "flags.default_action",
    "default_action.action",
    "mana",
    "premium",
    "level",
    "vocation",
    "flags.market",
    "market.category",
    "market.minimum_level",
    "market.restrict_to_profession",
}


def build_resistances(attrs):
    entries = []
    for field in ("absorbpercentall", "absorbpercentelements"):
        if field not in attrs:
            continue
        group = (
            RESIST_ALL_GROUP if field == "absorbpercentall" else RESIST_ELEMENTS_GROUP
        )
        value = to_int(attrs[field])
        entries.extend(
            {
                "damage_type": kind,
                "reduction_percent": ratio_value(value),
                "scope": "direct",
            }
            for kind in group
        )
    for suffix, damage_type in RESIST_SUFFIX_MAP.items():
        field = "absorbpercent" + suffix
        if field in attrs:
            entries.append(
                {
                    "damage_type": damage_type,
                    "reduction_percent": ratio_value(to_int(attrs[field])),
                    "scope": "direct",
                }
            )
    for suffix in ("energy", "fire", "poison"):
        field = "fieldabsorbpercent" + suffix
        if field in attrs:
            entries.append(
                {
                    "damage_type": RESIST_SUFFIX_MAP.get(suffix, suffix),
                    "reduction_percent": ratio_value(to_int(attrs[field])),
                    "scope": "field",
                }
            )
    return entries


def build_modifiers(attrs):
    modifiers = {}
    skill_boost = [
        {"skill": skill, "amount": to_int(attrs[field])}
        for field, skill in SKILL_MAP.items()
        if field in attrs
    ]
    if skill_boost:
        modifiers["skill_boost"] = skill_boost
    magic_level = [
        {"combat_type": combat_type, "amount": to_int(attrs[field])}
        for field, combat_type in MAGIC_LEVEL_SUFFIX_MAP.items()
        if field in attrs
    ]
    for field in ("magiclevelpoints", "magicpoints"):
        if field in attrs:
            magic_level.append({"combat_type": "all", "amount": to_int(attrs[field])})
    if magic_level:
        modifiers["magic_level"] = magic_level
    # SKILL_*_LEECH_CHANCE/AMOUNT (item_parse.cpp parseLifeAndManaLeech) store raw
    # hundredths of a percent, e.g. 1800 -> 18.00%; no such division exists for
    # absorbpercent*/hitchance/breakchance, which item_parse.cpp casts unscaled.
    if "lifeleechamount" in attrs or "lifeleechchance" in attrs:
        modifiers.setdefault("leech", []).append(
            {
                "resource": "health",
                "chance_percent": ratio_value(
                    to_int(attrs.get("lifeleechchance", 0)), 100
                ),
                "amount_percent": ratio_value(
                    to_int(attrs.get("lifeleechamount", 0)), 100
                ),
            }
        )
    if "manaleechamount" in attrs or "manaleechchance" in attrs:
        modifiers.setdefault("leech", []).append(
            {
                "resource": "mana",
                "chance_percent": ratio_value(
                    to_int(attrs.get("manaleechchance", 0)), 100
                ),
                "amount_percent": ratio_value(
                    to_int(attrs.get("manaleechamount", 0)), 100
                ),
            }
        )
    if "healthgain" in attrs and "healthticks" in attrs:
        modifiers.setdefault("regeneration", []).append(
            {
                "resource": "health",
                "amount": to_int(attrs["healthgain"]),
                "interval_ms": to_int(attrs["healthticks"]) * 1000,
            }
        )
    if "managain" in attrs and "manaticks" in attrs:
        modifiers.setdefault("regeneration", []).append(
            {
                "resource": "mana",
                "amount": to_int(attrs["managain"]),
                "interval_ms": to_int(attrs["manaticks"]) * 1000,
            }
        )
    if "criticalhitchance" in attrs:
        modifiers["critical_chance_percent"] = ratio_value(
            to_int(attrs["criticalhitchance"]), 100
        )
    if "criticalhitdamage" in attrs:
        modifiers["critical_damage_percent"] = ratio_value(
            to_int(attrs["criticalhitdamage"]), 100
        )
    if "cleavepercent" in attrs:
        modifiers["cleave_percent"] = ratio_value(to_int(attrs["cleavepercent"]))
    if "manashield" in attrs:
        modifiers["mana_shield_enabled"] = truthy(attrs["manashield"])
    if "magicshieldcapacityflat" in attrs or "magicshieldcapacitypercent" in attrs:
        capacity = {}
        if "magicshieldcapacityflat" in attrs:
            capacity["flat"] = to_int(attrs["magicshieldcapacityflat"])
        if "magicshieldcapacitypercent" in attrs:
            capacity["percent"] = ratio_value(
                to_int(attrs["magicshieldcapacitypercent"])
            )
        modifiers["magic_shield_capacity"] = capacity
    if "invisible" in attrs:
        modifiers["invisibility_enabled"] = truthy(attrs["invisible"])
    if "perfectshotdamage" in attrs and "perfectshotrange" in attrs:
        modifiers["perfect_shot_bonus"] = [
            {
                "range_cells": to_int(attrs["perfectshotrange"]),
                "damage": to_int(attrs["perfectshotdamage"]),
            }
        ]
    if "speed" in attrs:
        modifiers["movement_speed"] = {
            "flat": to_int(attrs["speed"]),
            "unit": "speed_points",
        }
    return modifiers


def build_condition_suppressions(attrs):
    return [
        {"condition": condition, "scope": "all"}
        for field, condition in CONDITION_SUPPRESS_MAP.items()
        if field in attrs
    ]


# --- presentation / appearance binding ---------------------------------------------

ENGINE_APPEARANCE_SOURCE = {
    CRYSTAL_PROFILE: {
        "source_profile": CRYSTAL_PROFILE,
        "repository": "zimbadev/crystalserver",
        "revision": "ff7ede593c69d4c658b382c97443e8155926924a",
        "path": "data/items/appearances.dat",
        "digest_sha256": ENGINE_ARTIFACT_DIGESTS[CRYSTAL_PROFILE][
            "data/items/appearances.dat"
        ],
    },
    CANARY_PROFILE: {
        "source_profile": CANARY_PROFILE,
        "repository": "opentibiabr/canary",
        "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
        "path": "data/items/appearances.dat",
        "digest_sha256": ENGINE_ARTIFACT_DIGESTS[CANARY_PROFILE][
            "data/items/appearances.dat"
        ],
    },
}
ENGINE_SHORT_NAME = {CRYSTAL_PROFILE: "crystal", CANARY_PROFILE: "canary"}


def presentation_ref(profile, appearance_id):
    engine = ENGINE_SHORT_NAME[profile]
    revision = ENGINE_APPEARANCE_SOURCE[profile]["revision"]
    return {
        "family": "Presentation",
        "key": f"oteryn:presentation.{engine}.item.{appearance_id}",
        "revision": f"{revision}.appearance-v1",
    }


def build_presentation_dependency(profile, appearance_id, appearance_obj):
    frame_groups = [
        {
            "kind": "object_initial",
            "source_group_id": 2,
            "geometry": group["geometry"],
            "sprite_ids": group["sprite_ids"],
        }
        for group in appearance_obj["frame_groups"]
        if group["fixed_frame_group"] == 2 or len(appearance_obj["frame_groups"]) == 1
    ]
    return {
        "identity": presentation_ref(profile, appearance_id),
        "source": ENGINE_APPEARANCE_SOURCE[profile],
        "appearance_id": appearance_id,
        "frame_groups": frame_groups,
    }


# --- assembling one Item -----------------------------------------------------------


def default_artifact_digests(profile, engine):
    digests = dict(ENGINE_ARTIFACT_DIGESTS[profile])
    pool_path = DELIVERY_POOL_PATH[engine]
    digests[pool_path] = (
        DELIVERY_LIST_SHA256 if engine == "crystal" else CANARY_WEEKLY_ITEMS_SHA256
    )
    return digests


def load_engine_sources(
    engine,
    source_root,
    digests=None,
    rule_source=None,
    rule_source_digest=None,
    overrides_path=None,
):
    """digests overrides the pinned production SHA-256 map; used only by fixture tests.

    `rule_source` is the checkout the Delivery Task adoption rule reads its pinned
    Crystal delivery list from. Crystal runs already read that exact file as their own
    pool, so `rule_source` is ignored for `engine == "crystal"`. Canary runs have no
    Crystal checkout of their own: `rule_source` is required, and a missing one is a hard
    error rather than an empty (silently all-ineligible) rule. `rule_source_digest`
    overrides the pinned production SHA-256 for that list; used only by fixture tests.
    `overrides_path` overrides the committed `delivery-task-overrides.json`; used only by
    fixture tests.
    """
    config = ENGINES[engine]
    profile = config["profile"]
    artifact_digests = (
        digests if digests is not None else default_artifact_digests(profile, engine)
    )
    pool_path = DELIVERY_POOL_PATH[engine]

    items_bytes, items_mode = read_verified_artifact(
        source_root, "data/items/items.xml", artifact_digests["data/items/items.xml"]
    )
    appearances_bytes, appearances_mode = read_verified_artifact(
        source_root,
        "data/items/appearances.dat",
        artifact_digests["data/items/appearances.dat"],
    )
    pool_bytes, pool_mode = read_verified_artifact(
        source_root, pool_path, artifact_digests[pool_path]
    )
    delivery_member_ids = DELIVERY_POOL_PARSER[engine](pool_bytes.decode("utf-8"))

    if engine == "crystal":
        # Crystal's own pinned pool file *is* the rule's Crystal delivery list.
        crystal_list_text = pool_bytes.decode("utf-8")
        crystal_list_ids = delivery_member_ids
    else:
        if rule_source is None:
            raise SystemExit(
                "--rule-source <crystal checkout> is required for canary: the "
                f"{RULE_ID!r} Delivery Task rule is keyed by Crystal delivery-list "
                "membership, which canary has no checkout of its own to read."
            )
        expected_digest = (
            rule_source_digest
            if rule_source_digest is not None
            else DELIVERY_LIST_SHA256
        )
        rule_list_bytes, _rule_list_mode = read_verified_artifact(
            Path(rule_source), DELIVERY_LIST_PATH, expected_digest
        )
        crystal_list_text = rule_list_bytes.decode("utf-8")
        crystal_list_ids = parse_crystal_delivery_ids(crystal_list_text)
    crystal_list_entries = len(re.findall(r"itemId\s*=\s*(\d+)", crystal_list_text))

    identity_index = build_identity_index()
    valid_keys = {key for key, _basis in identity_index.values()}
    resolved_overrides_path = (
        overrides_path if overrides_path is not None else DELIVERY_OVERRIDES_PATH
    )
    delivery_overrides = load_delivery_overrides(resolved_overrides_path, valid_keys)

    return {
        "engine": engine,
        "profile": profile,
        "repository": config["repository"],
        "revision": config["revision"],
        "items": load_items_xml(items_bytes.decode("utf-8")),
        "appearances": load_appearance_objects(appearances_bytes),
        "identity_index": identity_index,
        "delivery_member_ids": delivery_member_ids,
        "delivery_source": DELIVERY_SOURCE_NAME[engine],
        "crystal_list_ids": crystal_list_ids,
        "crystal_list_entries": crystal_list_entries,
        "delivery_overrides": delivery_overrides,
        "disposition": load_disposition_catalog(profile),
        "artifact_digests": {
            "data/items/items.xml": {
                "sha256": artifact_digests["data/items/items.xml"],
                "digest_mode": items_mode,
            },
            "data/items/appearances.dat": {
                "sha256": artifact_digests["data/items/appearances.dat"],
                "digest_mode": appearances_mode,
            },
            pool_path: {
                "sha256": artifact_digests[pool_path],
                "digest_mode": pool_mode,
            },
        },
    }


# Dispositions whose status the census counts as automatically routed to another
# owner; never a converter blocker regardless of whether this converter acts on them.
ROUTED_STATUSES = {
    "external_domain",
    "reverse_relation",
    "provenance_only",
    "pinned_no_effect",
    "approved_omission",
}


def field_status_row(field, disposition_row):
    if disposition_row is None:
        return "unsupported"
    status = disposition_row["status"]
    if status == "mapped":
        return "mapped" if field in IMPLEMENTED_FIELDS else "converter_missing"
    if status == "unresolved_semantics":
        return "unresolved"
    if status in ROUTED_STATUSES:
        return "routed"
    return "unsupported"


def convert_item(sources, item_id):
    """Return (item, dependencies, report). item/dependencies are None when not_converted."""
    profile = sources["profile"]
    disposition = sources["disposition"]
    identity_index = sources["identity_index"]
    xml_record = sources["items"].get(item_id)
    appearance = sources["appearances"].get(item_id)
    blockers = []
    field_status = {}

    def note(field, present):
        if not present:
            return
        status = field_status_row(field, disposition.get(field))
        field_status[field] = status
        if status in ("converter_missing", "unresolved", "unsupported"):
            blockers.append(f"{status}:{field}")

    identity_entry = identity_index.get(item_id)
    if identity_entry is None:
        # No CW2 B1 allocator key means no Crystal id to check the Delivery Task rule
        # against: the decision stays not admitted rather than being guessed.
        blockers.append("identity_not_in_b1_catalog")
        blockers.append("delivery_task_decision_not_admitted")
        return (
            None,
            None,
            {
                "item_id": item_id,
                "converted": False,
                "blockers": blockers,
                "field_status": field_status,
            },
        )
    key, identity_basis = identity_entry

    attrs = dict(xml_record["attrs"]) if xml_record else {}
    flags = dict(appearance["flags"]) if appearance else {}
    root_fields = ("id", "fromid", "toid", "name", "article", "plural", "editorsuffix")
    for name in disposition:
        if name.startswith("flags.") or "." in name:
            note(name, name in flags)
        elif name not in root_fields:
            note(name, name in attrs)
    note("name", bool(xml_record and xml_record.get("name")))
    note("article", bool(xml_record and xml_record.get("article")))
    note("plural", bool(xml_record and xml_record.get("plural")))
    note("appearance.id", appearance is not None)
    note("appearance.name", bool(appearance and appearance.get("name")))
    note("appearance.description", bool(appearance and appearance.get("description")))
    note("appearance.frame_group", bool(appearance and appearance.get("frame_groups")))

    primarytype = attrs.get("primarytype")
    family_profile = classify_family_profile(attrs, primarytype)
    if family_profile is None:
        blockers.append("family_profile_unresolved")
        return (
            None,
            None,
            {
                "item_id": item_id,
                "key": key,
                "identity_basis": identity_basis,
                "converted": False,
                "blockers": blockers,
                "field_status": field_status,
            },
        )

    name = (
        (xml_record["name"] if xml_record else None)
        or (appearance.get("name") if appearance else None)
        or f"item {item_id}"
    )
    item = {
        "identity": {"key": key, "revision": DEFINITION_REVISION},
        "display_name": name,
        "family_profile": family_profile,
        "taxonomy": {
            "item_class": PROFILE_ITEM_CLASS[family_profile],
            "primary": primarytype or attrs.get("weapontype") or family_profile,
        },
    }
    # `RULE_ID`: eligible iff the Crystal id sharing this Item's allocator key is a
    # member of the pinned Crystal delivery list, unless a per-item override applies.
    # This engine's own pool membership is kept as observation only; it never decides.
    is_pool_member = item_id in sources["delivery_member_ids"]
    override = sources["delivery_overrides"].get(key)
    if override is not None:
        decision = {
            "rule": RULE_ID,
            "basis": "override",
            "eligible": override["eligible"],
            "reason": override["reason"],
        }
    else:
        crystal_list_member = item_id in sources["crystal_list_ids"]
        decision = {
            "rule": RULE_ID,
            "basis": (
                "crystal_list_member"
                if crystal_list_member
                else "crystal_list_non_member"
            ),
            "eligible": crystal_list_member,
        }
    delivery_task_report = {
        "observation": {"source": sources["delivery_source"], "member": is_pool_member},
        "decision": decision,
    }
    item["delivery_task_eligible"] = decision["eligible"]

    dependencies = {
        "definitions": [],
        "assets": [],
        "presentations": [],
        "proficiency_crosswalks": [],
    }
    definitions = {}

    def add_item_ref(target_id):
        ref = item_ref(identity_index, target_id)
        if ref is None or ref["key"] == key:
            # A self-reference (e.g. a decayto/transform cycle back to itself) is not
            # a valid dependency edge; the caller treats a None return as "no target".
            return None
        definitions[ref["key"]] = ref
        return ref

    # physical
    physical = {}
    if "weight" in attrs:
        destination, payload = weight_payload(to_int(attrs["weight"]))
        if destination == "weight":
            physical["weight"] = payload
        else:
            item.setdefault("presentation", {})["display_weight"] = payload
    if "movable" in attrs:
        physical["movable"] = truthy(attrs["movable"])
    if "pickupable" in attrs or "allowpickupable" in attrs or flags.get("flags.take"):
        physical["pickupable"] = truthy(
            attrs.get("pickupable", attrs.get("allowpickupable", "1"))
        ) or bool(flags.get("flags.take"))
    if physical:
        item["physical"] = physical

    # presentation
    presentation = item.get("presentation", {})
    grammar = {}
    if xml_record and xml_record.get("article"):
        grammar["article"] = xml_record["article"]
    if xml_record and xml_record.get("plural"):
        grammar["plural"] = xml_record["plural"]
    if grammar:
        presentation["grammar"] = grammar
    description = attrs.get("description") or (
        appearance.get("description") if appearance else None
    )
    if description:
        presentation["inspection_description"] = description
    if appearance is not None and appearance.get("frame_groups"):
        presentation["appearance_binding"] = presentation_ref(profile, item_id)
        dependencies["presentations"].append(
            build_presentation_dependency(profile, item_id, appearance)
        )
        blockers.append("sprite_atlas_not_admitted")
    if presentation:
        item["presentation"] = presentation

    # equipment
    equipment = build_equipment(attrs, flags.get("clothes.slot"))
    quiver_container = family_profile == "container_equipment"
    if equipment:
        item["equipment"] = equipment

    # weapon
    weapon_type_raw = attrs.get("weapontype")
    if family_profile in ("weapon_melee", "weapon_distance", "weapon_magic"):
        weapon = {"weapon_type": WEAPON_TYPE_MAP.get(weapon_type_raw, "sword")}
        for field, dest in (
            ("attack", "attack"),
            ("defense", "defense"),
            ("extradef", "extra_defense"),
            ("range", "range_cells"),
        ):
            if field in attrs:
                weapon[dest] = to_int(attrs[field])
        if "ammotype" in attrs:
            weapon["ammunition_kind"] = attrs["ammotype"]
        if "hitchance" in attrs:
            weapon["hit_chance_modifier_percent"] = ratio_value(
                to_int(attrs["hitchance"])
            )
        if "maxhitchance" in attrs:
            weapon["max_hit_chance_percent"] = ratio_value(
                to_int(attrs["maxhitchance"])
            )
        if "breakchance" in attrs:
            weapon["break_chance_percent"] = ratio_value(to_int(attrs["breakchance"]))
        if "fromdamage" in attrs and "todamage" in attrs:
            low, high = to_int(attrs["fromdamage"]), to_int(attrs["todamage"])
            weapon["damage_range"] = {
                "minimum": min(low, high),
                "maximum": max(low, high),
            }
        if attrs.get("wandtype") in (
            "physical",
            "energy",
            "earth",
            "fire",
            "life_drain",
            "mana_drain",
            "drowning",
            "ice",
            "holy",
            "death",
            "agony",
            "neutral",
            "healing",
        ):
            weapon["damage_type"] = attrs["wandtype"]
        elemental_attack = [
            {"damage_type": damage_type, "amount": to_int(attrs[field])}
            for field, damage_type in ELEMENTAL_ATTACK_SUFFIX_MAP.items()
            if field in attrs
        ]
        if elemental_attack:
            weapon["elemental_attack"] = elemental_attack
        item["weapon"] = weapon

    # protection
    protection = {}
    if "armor" in attrs:
        protection["armor"] = to_int(attrs["armor"])
    resistances = build_resistances(attrs)
    if resistances:
        protection["resistances"] = resistances
    suppressions = build_condition_suppressions(attrs)
    if suppressions:
        protection["condition_suppressions"] = suppressions
    if protection:
        item["protection"] = protection

    # container
    if "containersize" in attrs:
        content_kind = "items"
        if quiver_container:
            content_kind = "ammunition"
        elif primarytype == "fluid containers":
            content_kind = "fluids"
        item["container"] = {
            "capacity": to_int(attrs["containersize"]),
            "content_kind": content_kind,
        }

    # charges
    if "charges" in attrs:
        charges = {"count": max(1, to_int(attrs["charges"], 1))}
        if "showcharges" in attrs:
            charges["show_count"] = truthy(attrs["showcharges"])
        item["charges"] = charges

    # temporal + lifecycle transforms/wrapping
    decay_target_id = to_int(attrs.get("decayto", 0))
    if "decayto" in attrs and "duration" in attrs:
        temporal = {
            "duration_ms": max(1, to_int(attrs["duration"]) * 1000),
            "consumption_mode": "continuous",
        }
        if decay_target_id:
            ref = add_item_ref(decay_target_id)
            if ref:
                temporal["decay_target"] = ref
        item["temporal"] = temporal
    transforms = []
    for field, trigger in (
        ("destroyto", "destroy"),
        ("transformequipto", "equip"),
        ("transformdeequipto", "unequip"),
        ("transformonuse", "use"),
    ):
        target_id = to_int(attrs.get(field, 0))
        if field in attrs and target_id:
            ref = add_item_ref(target_id)
            if ref:
                transforms.append({"trigger": trigger, "target": ref})
    if decay_target_id and "decayto" in attrs:
        ref = add_item_ref(decay_target_id)
        if ref:
            transforms.append({"trigger": "decay", "target": ref})
    wrapping = {}
    if flags.get("flags.wrap"):
        wrapping["wrap_enabled"] = True
    if flags.get("flags.unwrap"):
        wrapping["unwrap_enabled"] = True
    if flags.get("flags.wrapkit"):
        wrapping["is_wrap_kit"] = True
    if "wrapableto" in attrs and to_int(attrs["wrapableto"]):
        ref = add_item_ref(to_int(attrs["wrapableto"]))
        if ref:
            wrapping["wrap_target"] = ref
    if "wrapcontainer" in attrs:
        wrapping["preserve_contents"] = truthy(attrs["wrapcontainer"])
    lifecycle = {}
    if transforms:
        lifecycle["transforms"] = transforms
    if wrapping:
        lifecycle["wrapping"] = wrapping
    if lifecycle:
        item["lifecycle"] = lifecycle

    # imbuement
    if "imbuementslot" in attrs:
        item["imbuement"] = {"slot_count": to_int(attrs["imbuementslot"])}

    # light
    if flags.get("flags.light"):
        brightness = to_int(flags.get("light.brightness", 0))
        if brightness:
            color = to_int(flags.get("light.color", 0))
            asset_key = f"{sources['engine']}.appearance:light-color/{color}"
            dependencies["assets"].append(asset_key)
            item["light"] = {
                "emits": True,
                "color_binding": asset_key,
                "intensity": brightness,
            }

    # requirements / trade
    requirements = {}
    min_level = to_int(attrs.get("level", flags.get("market.minimum_level", 0)))
    if min_level:
        requirements["min_level"] = min_level
    if "premium" in attrs:
        requirements["premium_only"] = truthy(attrs["premium"])
    professions = flags.get("market.restrict_to_profession")
    vocations = (
        sorted(
            {PLAYER_PROFESSION_NAMES.get(p, str(p)) for p in professions}
            - {"any", "none"}
        )
        if professions
        else []
    )
    if "vocation" in attrs:
        parsed = parse_vocation_attribute(attrs["vocation"])
        if parsed:
            vocations = sorted(parsed)
    on_equip_profiles = (
        "weapon_melee",
        "weapon_distance",
        "equipment_armor",
        "equipment_offhand",
        "container_equipment",
    )
    if requirements or vocations or family_profile in ("weapon_magic", "rune"):
        requirements["enforcement_mode"] = (
            "on_equip" if family_profile in on_equip_profiles else "on_use"
        )
        if vocations:
            requirements["vocations"] = vocations
        item["requirements"] = requirements
    trade = {}
    if flags.get("flags.market"):
        category = flags.get("market.category")
        if category is not None:
            trade["marketable"] = True
            if category in ITEM_CATEGORY_NAMES:
                trade["market_category"] = ITEM_CATEGORY_NAMES[category]
            if vocations:
                trade["vocations"] = vocations
    if trade:
        item["trade"] = trade

    # readable
    if "readable" in attrs or "writeable" in attrs:
        readable = {"readable": True}
        writable = truthy(attrs.get("writeable", "0"))
        readable["writable"] = writable
        if writable and to_int(attrs.get("maxtextlen", 0)) > 0:
            readable["write_policy"] = (
                "write_once" if "writeonceitemid" in attrs else "rewrite"
            )
            readable["max_characters"] = to_int(attrs["maxtextlen"])
            if "writeonceitemid" in attrs:
                ref = add_item_ref(to_int(attrs["writeonceitemid"]))
                if ref:
                    readable["write_once_target"] = ref
        elif writable:
            note("maxtextlen", False)
            writable = False
            readable["writable"] = False
            readable["write_policy"] = "none"
        else:
            readable["write_policy"] = "none"
        if "allowdistread" in attrs:
            readable["distance_readable"] = truthy(attrs["allowdistread"])
        item["readable"] = readable

    # use
    use = {}
    if flags.get("flags.usable") or flags.get("flags.default_action"):
        use["usable"] = True
    if flags.get("flags.multiuse"):
        use["use_with"] = True
    if use:
        use.setdefault("usable", False)
        use.setdefault("use_with", False)
        if "mana" in attrs:
            use["mana_cost"] = to_int(attrs["mana"])
        action = flags.get("default_action.action")
        if action in PLAYER_ACTION_NAMES and PLAYER_ACTION_NAMES[action] != "none":
            use["default_action"] = PLAYER_ACTION_NAMES[action]
        item["use"] = use

    # modifiers
    modifiers = build_modifiers(attrs)
    if modifiers:
        item["modifiers"] = modifiers

    if definitions:
        dependencies["definitions"] = [definitions[k] for k in sorted(definitions)]

    return (
        item,
        dependencies,
        {
            "item_id": item_id,
            "key": key,
            "identity_basis": identity_basis,
            "family_profile": family_profile,
            "delivery_task": delivery_task_report,
            "converted": True,
            "blockers": blockers,
            "field_status": field_status,
        },
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--engine", choices=sorted(ENGINES), required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--id", type=int, required=True)
    parser.add_argument(
        "--rule-source",
        type=Path,
        help=(
            "pinned Crystal checkout the Delivery Task rule reads its delivery list "
            "from; required for --engine canary, ignored for --engine crystal"
        ),
    )
    parser.add_argument(
        "--overrides",
        type=Path,
        help="delivery-task-overrides.json to use instead of the committed one",
    )
    cli_args = parser.parse_args()
    cli_sources = load_engine_sources(
        cli_args.engine,
        cli_args.source,
        rule_source=cli_args.rule_source,
        overrides_path=cli_args.overrides,
    )
    cli_item, cli_dependencies, cli_report = convert_item(cli_sources, cli_args.id)
    print(
        json.dumps(
            {"item": cli_item, "dependencies": cli_dependencies, "report": cli_report},
            indent=2,
            sort_keys=True,
        )
    )
