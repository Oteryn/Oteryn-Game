"""Capture six declared Rotten native death vectors from the complete fixed source.

The Crystal overlay declares an unloaded RottenBloodBossDeath name. Missing
handlers may be omitted only after checking every pinned Lua constructor and
the exact C++ loaded-event lookup; they are never imported from Canary.

These are the monster arrays' source-specific callbacks. The shared BossDeath
registered by Monster.setReward(true) in onAppear belongs to the existing
generic personal-reward owner; an empty copied vector does not mean that the
source monster has no dynamically registered generic death event.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

from build_srx3_rotten_blood_witness import PIN, ROOT
from build_srx3_rotten_blood_witness import build as witness_build
from death_event_source import monster_registrations, registered_callbacks

OUTPUT = ROOT / "tools/content-schema/native-gameplay/srx3-rotten-death-events.json"
FROZEN_SOURCE_SHA256 = "fd21849dd13decf03b015c7d64692ae6380e50b45f5c61e59e00060c45c7309d"
KEYS = ("bakragore", "chagorz", "ichgahal", "murcion", "mushroom", "vemiath")
PARENTS = "data-global/monster/quests/rotten_blood_quest/"
SCRIPTS = "data-global/scripts/quests/rotten_blood_quest/"
HANDLERS = {
    "BakragoreMechanicsDeath": SCRIPTS + "bakragore_mechanics_death.lua",
    "ChagorzMechanicsDeath": SCRIPTS + "chagorz_mechanics_death.lua",
    "VemiathMechanicsDeath": SCRIPTS + "vemiath_mechanics_death.lua",
}
TEXT_SUFFIXES = {".lua", ".cpp", ".hpp", ".h", ".xml"}


def canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def source_digest(value: dict) -> str:
    return hashlib.sha256(canonical({k: v for k, v in value.items()
                                     if k != "source_definitions_sha256"})).hexdigest()


def qualify_capture(value: dict) -> None:
    if source_digest(value) != FROZEN_SOURCE_SHA256:
        raise ValueError("Rotten death source capture changed; recapture and review the fixed source")


def refresh_native_pin() -> dict:
    result = json.loads(OUTPUT.read_bytes())
    qualify_capture(result)
    result["source_definitions_sha256"] = witness_build()["source_definitions_sha256"]
    return result


def resolve_parent(raw: bytes, definitions: dict[str, tuple[str, bytes]], absent: set[str]) -> dict:
    registrations = monster_registrations(raw.decode())
    events = []
    seen = set()
    for registration in registrations:
        name = registration["name"]
        if name in seen:
            continue
        seen.add(name)
        if name in absent:
            if name in definitions:
                raise ValueError("a supposedly absent callback is actually defined")
            continue
        if name not in definitions:
            raise ValueError("unproved source event: " + name)
        path, body = definitions[name]
        if "onDeath" in registered_callbacks(body.decode(), name):
            events.append({**registration, "source_path": path,
                           "source_sha256": hashlib.sha256(body).hexdigest()})
    return {"monster_sha256": hashlib.sha256(raw).hexdigest(),
            "registered_events": registrations, "on_death": events}


def build(source_root: Path, tree: dict) -> dict:
    if tree.get("sha") != PIN or tree.get("truncated") is not False:
        raise ValueError("the complete exact Crystal201189f Git tree is required")
    blobs = {row["path"]: row for row in tree["tree"] if row["type"] == "blob"}
    bodies = {}
    files = []
    constructors = []
    noncalls = []
    for path, blob in sorted(blobs.items()):
        if Path(path).suffix not in TEXT_SUFFIXES:
            continue
        raw = (source_root / path).read_bytes()
        git_blob = hashlib.sha1(f"blob {len(raw)}\0".encode() + raw).hexdigest()
        if git_blob != blob["sha"]:
            raise ValueError("source bytes disagree with the fixed Git tree: " + path)
        files.append({"path": path, "git_blob": git_blob,
                      "sha256": hashlib.sha256(raw).hexdigest()})
        bodies[path] = raw
        text = raw.decode("utf-8")
        if not path.endswith(".lua"):
            if "RottenBloodBossDeath" in text:
                raise ValueError("unexpected non-Lua registration of RottenBloodBossDeath")
            continue
        for mention in re.finditer(r"\bCreatureEvent\b", text):
            tail = text[mention.end():]
            if not re.match(r"\s*\(", tail):
                noncalls.append({"path": path, "line": text.count("\n", 0, mention.start()) + 1})
                continue
            call = re.match(r'''\s*\(\s*(["'])([^\n]*?)\1\s*\)''', tail)
            if not call or call[1] in call[2]:
                raise ValueError("dynamic/unparsed CreatureEvent constructor: " + path)
            constructors.append({"path": path, "name": call[2],
                                 "line": text.count("\n", 0, mention.start()) + 1})
    # These three references are the reviewed class wrapper, metatable hook and
    # comment. Neither helper invents names; all actual constructor arguments
    # above are closed literal strings. Any changed reference needs review.
    expected_noncalls = [
        {"path": "data/libs/functions/revscriptsys.lua", "line": 121},
        {"path": "data/libs/functions/revscriptsys.lua", "line": 175},
        {"path": "data/scripts/lib/helper_constructors.lua", "line": 2},
    ]
    if noncalls != expected_noncalls or len(constructors) != 335:
        raise ValueError("the complete constructor/reference corpus changed")
    if any(row["name"] == "RottenBloodBossDeath" for row in constructors):
        raise ValueError("RottenBloodBossDeath is no longer an unloaded declaration")
    for name, path in HANDLERS.items():
        if [row["path"] for row in constructors if row["name"] == name] != [path]:
            raise ValueError("missing or ambiguous loaded death handler: " + name)
    definitions = {name: (path, bodies[path]) for name, path in HANDLERS.items()}
    rows = []
    for key in KEYS:
        path = PARENTS + key + ".lua"
        rows.append({"creature_key": "oteryn:creature." + key,
                     "creature_revision": "definition-r1", "parent_path": path,
                     "parent_git_blob": blobs[path]["sha"],
                     **resolve_parent(bodies[path], definitions, {"RottenBloodBossDeath"})})
    source_paths = sorted({PARENTS + key + ".lua" for key in KEYS} | set(HANDLERS.values()))
    witness = witness_build()
    return {
        "schema": "SRX3_ROTTEN_ENTRY_ORDERED_DEATH_EVENTS/v1",
        "source_repository": "zimbadev/crystalserver", "source_revision": PIN,
        "source_definitions_sha256": witness["source_definitions_sha256"],
        "registry_proof": {
            "complete_tree": PIN, "text_files": len(files),
            "lua_files": sum(row["path"].endswith(".lua") for row in files),
            "source_set_sha256": hashlib.sha256(canonical(files)).hexdigest(),
            "constructors": len(constructors),
            "constructors_sha256": hashlib.sha256(canonical(constructors)).hexdigest(),
            "noncall_references": noncalls,
            "absent_loaded_names": ["RottenBloodBossDeath"],
            "lookup_source": "src/creatures/creature.cpp",
            "lookup_git_blob": blobs["src/creatures/creature.cpp"]["sha"],
            "loaded_lookup_source": "src/lua/creature/creatureevent.cpp",
            "loaded_lookup_git_blob": blobs["src/lua/creature/creatureevent.cpp"]["sha"],
        },
        "sources": [{"path": path, "git_blob": blobs[path]["sha"],
                     "sha256": hashlib.sha256(bodies[path]).hexdigest()} for path in source_paths],
        "records": rows,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--tree", type=Path)
    parser.add_argument("--refresh-native-pin", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.refresh_native_pin:
        result = refresh_native_pin()
    else:
        if not args.source_root or not args.tree:
            parser.error("--source-root and --tree are required for exact source recapture")
        result = build(args.source_root, json.loads(args.tree.read_bytes()))
        qualify_capture(result)
    raw = (json.dumps(result, sort_keys=True, indent=2) + "\n").encode()
    if args.check:
        if OUTPUT.read_bytes() != raw:
            raise SystemExit("stale six-entry Rotten death source capture")
    else:
        OUTPUT.write_bytes(raw)
    print("6 exact source parents, 3 loaded declared handlers, 3 empty declared vectors")


if __name__ == "__main__":
    main()
