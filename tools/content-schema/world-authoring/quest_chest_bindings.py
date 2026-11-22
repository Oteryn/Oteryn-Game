"""Restore pinned Crystal startup chest UIDs and their required level-door actions.

Selections live in separate source captures so quest lanes can add their own
cohort. The canonical RewardClaim must still agree with every captured selector.
Run --apply [--check]; convert_world_base also applies the same overlay.
"""

import argparse
import hashlib
import json
from pathlib import Path

import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
CAPTURES = "tools/content-schema/world-authoring/samples/quest-chest-bindings"
INDEX = "content/world/placements/index.json"
SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
TABLE_SHA256 = "7c3ac1280196923e0bf5ca6f2ea932c19065bb9ac22d1559150a184f3cf06df4"
CURRENT_REVISION = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
TABLE_PINS = {
    REVISION: TABLE_SHA256,
    CURRENT_REVISION: "0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca",
}
STARTUP_PINS = {
    CURRENT_REVISION: "897c4ea5572c6adef1d2b300fd75c141910d608a093ca15c3d3bf65fcf56826c"
}
TELEPORT_SHA256 = "9e8f807264ec6ab85e263976136666ff38c195f1921cc30e73fd4431370fe87e"
ROUTE_DOORS = {
    (33265, 32278, 7): (5293, 1040),
    (33266, 32278, 7): (5293, 1040),
    (32483, 31722, 15): (5111, 1060),
}
FIBULA_RETURNS = (38001, 38002)
STARTUP_TELEPORT_NAMESPACE = "crystalserver/startup-teleport-unique"
LEVEL_SHA256 = "d8868f675aae7c541ddc3e76b3648b6a886e0ae31a4a614747d17e1784d234fc"
ITEM_SHA256 = "7f551a398529e18ccad1ba56d740a4961874bbddd4736dd2df00272ddf6ea6ad"
ITEM_ACCESS = {
    "oteryn:reward-claim.kv.naginata": ((32356, 32074, 10), 1791, 40031),
}
# Restoring a reward must retain its pinned Crystal expertise door.
LEVEL_ACCESS = {
    **{
        f"oteryn:reward-claim.kv.behemothquest{i}": ((33297, 31670, 14), 1648, 1080)
        for i in range(1, 5)
    },
    "oteryn:reward-claim.kv.blackknight1": ((32874, 31974, 12), 5102, 1050),
    "oteryn:reward-claim.kv.blackknight2": ((32875, 31974, 12), 5102, 1050),
    **{
        f"oteryn:reward-claim.kv.desert{i}": ((32673, 32100, 8), 5102, 1020)
        for i in (1, 2)
    },
    # Pinned door_level.lua:173-178; the raw B3 omits startup attributes.
    "oteryn:reward-claim.quest.u7_24.the_annihilator.reward": (
        (33214, 31671, 13),
        5111,
        1100,
    ),
    **{
        f"oteryn:reward-claim.kv.orcfortress{i}": ((32981, 31760, 9), 6264, 1040)
        for i in range(1, 4)
    },
    "oteryn:reward-claim.kv.naginata": ((32353, 32073, 11), 1646, 1040),
    "oteryn:reward-claim.kv.noblearmor1": ((32448, 32042, 8), 6264, 1035),
    "oteryn:reward-claim.kv.noblearmor2": ((32448, 32042, 8), 6264, 1035),
}


def valid_capture_pin(document):
    revision = document.get("revision")
    return (
        document.get("schema") == 1
        and revision in TABLE_PINS
        and document.get("table_sha256") == TABLE_PINS[revision]
        and document.get("level_source_sha256") == LEVEL_SHA256
        and (
            revision not in STARTUP_PINS
            or document.get("startup_sha256") == STARTUP_PINS[revision]
        )
    )


def canonical(value):
    return (
        json.dumps(value, separators=(",", ":"), sort_keys=True, ensure_ascii=False)
        + "\n"
    ).encode()


def captured_level_access(raw, selected, root=ROOT):
    """Requalify level access independently of an unchanged chest capture."""
    import sys

    sys.path.insert(0, str(root / "tools/content-schema/quest-authoring"))
    import lua_tables

    if hashlib.sha256(raw).hexdigest() != LEVEL_SHA256:
        raise ValueError("QUEST_CHEST_LEVEL_SOURCE_PIN")
    fields = lua_tables.assignments(raw.decode(), {"LevelDoorAction"})[
        "LevelDoorAction"
    ]["fields"]
    doors = {}
    for key in sorted(set(selected) & LEVEL_ACCESS.keys()):
        position, appearance, action = LEVEL_ACCESS[key]
        field = next(f for f in fields if f["key"] == action)
        value = lua_tables.as_python(field["value"])
        groups = value if isinstance(value, list) else [value]
        if not any(
            p == dict(zip("xyz", position)) and group["itemId"] in (False, appearance)
            for group in groups
            if isinstance(group, dict)
            for p in (
                group["itemPos"]
                if isinstance(group["itemPos"], list)
                else [group["itemPos"]]
            )
        ):
            raise ValueError("QUEST_CHEST_LEVEL_ACCESS_SOURCE_DRIFT")
        row = doors.setdefault(
            position,
            {
                "position": list(position),
                "appearance": appearance,
                "action": action,
                "source_line": field["line"],
                "claims": [],
            },
        )
        row["claims"].append(key)
    return [doors[p] for p in sorted(doors)]


def claim_definitions(root):
    claims = {}
    for path in sorted(
        (root / "content/interactions/reward_claims").glob("reward-claim*.json")
    ):
        for record in json.loads(path.read_bytes())["records"]:
            definition = record["definition"]
            claims[definition["identity"]["key"]] = definition
    return claims


def supported_claim(definition):
    """Match the existing native container/text gate; retain all source holds."""
    native = definition.get("native_contents", {})
    return (
        definition["readiness"] == "ready"
            number = numbered_key_claim(definition)
            if number is not None and (
