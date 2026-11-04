#!/usr/bin/env python3
"""Capture Liquid Black's exact Crystal201189f NPC text and two state changes.

The existing QuestState schema owns writes. The native NPC caller supplies the
source topic, live NPC identity and current session-generation fence. This tool
does not execute Lua or activate the other quests sharing Spectulus. Additional
static source speech is retained in a separate SOURCE_ONLY evidence packet.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
QUEST = "oteryn:quest.liquid_black_quest"
PREFIX = "oteryn:quest-progress/quest/u9_4/liquid_black_quest/"
QUESTLINE = PREFIX + "questline"
VISITOR = PREFIX + "visitor"
START = "oteryn:quest-transition/liquid-black/visitor-start"
ACCESS = "oteryn:quest-transition/liquid-black/machine-access"
STRING = re.compile(r'"(?:[^"\\]|\\.)*"')


def literal_speech(lines, line, path):
    match = re.match(
        r'\s*npcHandler:say\(("(?:[^"\\]|\\.)*"|\{.*?\})'
        r', npc, creature(?:, (\d+))?\)', "\n".join(lines[line - 1:]), re.DOTALL,
    )
    if not match or STRING.sub("", match[1]).strip("{}, \t\r\n"):
        raise ValueError(f"Pinned Liquid Black speech moved or became computed: {path}:{line}")
    return [json.loads(value) for value in STRING.findall(match[1])]


def message(lines, line, kind):
    match = re.fullmatch(r'\s*npcHandler:setMessage\(' + kind + r', ("(?:[^"\\]|\\.)*")\)', lines[line - 1])
    if not match:
        raise ValueError(f"Pinned Liquid Black {kind} moved: {line}")
    return json.loads(match[1])


def source_keyword(lines, registration, text_line, end_line, source):
    header = re.search(
        r':(addKeyword|addChildKeyword)\((\{[^}]*\}), StdModule\.say, \{',
        lines[registration - 1],
    )
    text = re.search(
        r'\btext\s*=\s*("(?:[^"\\]|\\.)*")'
        r'(?=\s*(?:,\s*reset\s*=\s*true\s*}\)|,\s*$))',
        lines[text_line - 1],
    )
    if not header or not text or not lines[end_line - 1].rstrip().endswith('})'):
        raise ValueError(f"Pinned Liquid Black source keyword moved or became computed: {registration}")
    if STRING.sub('', header[2]).strip('{}, \t\r\n'):
        raise ValueError("Pinned Liquid Black source keyword is not literal")
    return {
        "native_binding": "SOURCE_ONLY",
        "source": {**source, "line": registration, "text_line": text_line, "end_line": end_line},
        "registration": header[1],
        "keyword_expression": header[2],
        "source_expression": text[1],
        "literal_messages": [json.loads(text[1])],
    }


def build(crystal):
    records = []
    sources = []
    source_keywords = []
    configurations = (
        ("scrutinon", 101, (("scrutinon_quirefang", 57, "quirefang"),
                            ("scrutinon_visitor", 65, "visitor"))),
        ("spectulus", 492, (("spectulus_machine", 380, "machine"),
                            ("spectulus_yes_21", 389, "yes"),
                            ("spectulus_yes_22", 401, "yes"),
                            ("spectulus_yes_23", 411, "yes"))),
    )
    for npc, greet_line, speeches in configurations:
        path = f"data-global/npc/{npc}.lua"
        raw = subprocess.run(["git", "-C", str(crystal), "show", f"{PIN}:{path}"],
                             check=True, capture_output=True).stdout
        lines = raw.decode().splitlines()
        source = {"repository": "zimbadev/crystalserver", "revision": PIN,
                  "path": path, "sha256": hashlib.sha256(raw).hexdigest()}
        locations = [(83, 83, 83)] if npc == "scrutinon" else [
            (line, line + 2, line + 3) for line in (463, 467, 471, 475, 479, 483, 487)]
        source_keywords.extend(source_keyword(lines, registration, text_line, end_line, source)
                               for registration, text_line, end_line in locations)
        sources.append(source)
        keywords = [{"key": key, "triggers": [trigger], "reply": literal_speech(lines, line, path)}
                    for key, line, trigger in speeches]
        if npc == "scrutinon":
            name_expression = "\n".join(lines[94:97])
            if STRING.sub("", name_expression).strip(", \t\r\n"):
                raise ValueError("Pinned Scrutinon name is no longer literal")
            keywords.append({"key": "scrutinon_name", "triggers": ["name"],
                             "reply": [json.loads(value) for value in STRING.findall(name_expression)]})
        else:
            keywords.append({"key": "spectulus_returned", "triggers": ["hi", "hello"],
                             "reply": [message(lines, 70, "MESSAGE_GREET")]})
        source["speech"] = [{"key": key, "line": line} for key, line, _ in speeches]
        source["greeting_line"] = greet_line
        source["additional_lines"] = [95, 96, 97] if npc == "scrutinon" else [70, 493, 494]
        declaration = {
            "kind": "Dialogue", "identity": {"key": f"oteryn:dialogue.npc.{npc}", "revision": "definition-r1"},
            "greet": [message(lines, greet_line, "MESSAGE_GREET")], "keywords": keywords,
            "fields": [{"field_path": "oteryn:source.quest.liquid_black.dialogue",
                        "value": {"type": "Text", "value": json.dumps(source, ensure_ascii=False, sort_keys=True)}}],
        }
        if npc == "spectulus":
            declaration["farewell"] = [message(lines, 493, "MESSAGE_FAREWELL")]
        records.append({"declaration": declaration})
    dialogue = {"schema": "OTERYN_DIALOGUE_AUTHORING_SHARD/v1", "family": "Dialogue",
                "shard": {"index": 0, "start": 0, "end": 1, "count": 2},
                "source_legacy_role": "Crystal201189f Liquid Black NPC branches", "records": records}

    def track(key, initial, maximum, storage):
        return {"key": key, "quest": QUEST, "initial": initial, "min": initial, "max": maximum,
                "bounds_basis": "pinned_source_and_existing_canonical_track",
                "source_key": f"storage:Quest.U9_4.LiquidBlackQuest.{storage}"}

    def effect(key, operation, value, set_value):
        guard = {"op": operation}
        if value is not None:
            guard["value"] = value
        return {"track": key, "from": guard, "from_exact": True,
                "effect": {"kind": "SET", "value": set_value}}

    def transition(key, npc, topic, keyword, line, effects):
        return {"key": key, "quest": QUEST, "completes": False, "effects": effects,
                "requested_by": {"npc": f"crystal:npc/{npc}", "keywords": [keyword], "topics": [topic]},
                "source": {"repository": "zimbadev/crystalserver", "revision": PIN,
                           "path": f"data-global/npc/{npc}.lua", "line": line}}

    state = {"schema": "OTERYN_QUEST_STATE_LOWERING/v1", "classification": "OTS_HYPOTHESIS_ONLY",
             "family": "Quest",
             "contract": "docs/architecture/reviews/OTERYN_GAME_QUEST_STATE0_QUEST_PROGRESS_STORE_DECISION_2026-09-30.md",
             "authoring_sources": sources, "counts": {"quests": 1, "tracks": 2, "transitions": 2},
             "quests": [{"quest": QUEST, "source_quest": "crystal:quest/liquid_black",
                         "completion": "NOT_LOWERED_MULTI_TRACK",
                         "tracks": [track(QUESTLINE, -1, 1, "Questline"), track(VISITOR, 0, 5, "Visitor")],
                         "transitions": [
                             transition(START, "scrutinon", 1, "visitor", 69,
                                        [effect(QUESTLINE, "LT", 1, 1), effect(VISITOR, "ANY", None, 1)]),
                             transition(ACCESS, "spectulus", 23, "yes", 409,
                                        [effect(VISITOR, "EQ", 3, 4)]),
                         ]}]}
    keywords_capture = {
        "schema": "OTERYN_QUESTS_2_LIQUID_BLACK_SOURCE_KEYWORDS/v1",
        "classification": "OTS_HYPOTHESIS_ONLY", "source_revision": PIN,
        "native_binding": "SOURCE_ONLY",
        "scope": "Additional static speech in the shared Scrutinon/Spectulus source files. No NPC admission, selector order, travel or foreign quest transition is authorized by this packet.",
        "records": source_keywords,
    }
    return {ROOT / "content/dialogues/liquid-black.json": dialogue,
            ROOT / "content/quests/liquid-black-npc-state.json": state,
            ROOT / "content/dialogues/quests-2/liquid-black-source-keywords.json": keywords_capture}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crystal", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for path, data in build(args.crystal).items():
        expected = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
        if args.check:
            if path.read_text() != expected:
                raise SystemExit(f"Stale Liquid Black content: {path.relative_to(ROOT)}")
        else:
            path.write_text(expected)


if __name__ == "__main__":
    main()
