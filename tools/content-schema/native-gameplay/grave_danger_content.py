#!/usr/bin/env python3
"""Capture Jack Springer's Crystal201189f text and closed QuestState changes.

The start and outfit final remain source-only artifacts: PR2844 ANSWER6589521017
point5 holds the thirteen-effect start; Q20 holds the outfit final. No partial
start or completed-without-outfit transition is emitted.
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
SOURCE = "data-global/npc/jack_springer.lua"
QUEST = "oteryn:quest.grave_danger_quest"
PREFIX = "oteryn:quest-progress/quest/u12_20/grave_danger/"
QUESTLINE = PREFIX + "questline"
DOOR = "oteryn:quest-progress/crystalserver/quest/u12_20/grave_danger/bosses/king_zelos_door"
OUTFIT = "oteryn:quest-progress/crystalserver/quest/u12_20/grave_danger/bosses/inquisition_outfit_received"
START = "oteryn:quest-transition/grave-danger/start"
ACCESS = "oteryn:quest-transition/grave-danger/zelos-access"
FINISH = "oteryn:quest-transition/grave-danger/final-report"
FINISH_HELD = "oteryn:quest-transition/grave-danger/final-report-outfit-held"
STRING = re.compile(r'"(?:[^"\\]|\\.)*"')
SPEECHES = (("final_report", 82, "report"), ("report_ready", 93, "report"),
            ("report_good_luck", 96, "report"), ("report_missing", 99, "report"),
            ("ultimate", 103, "ultimate"), ("threat", 110, "threat"), ("start", 128, "late"))
GRAVES = ("edron", "dark_cathedral", "ghostlands", "cormaya", "femor_hills", "ankrahmun",
          "kilmaresh", "vengoth", "darashia", "thais", "orclands", "ice_islands")


def literal(expression, line):
    if not STRING.findall(expression) or STRING.sub("", expression).strip("{}, \t\r\n"):
        raise ValueError(f"Nonliteral Jack Springer text at {SOURCE}:{line}")
    return [json.loads(value) for value in STRING.findall(expression)]


def build(crystal):
    raw = subprocess.run(["git", "-C", str(crystal), "show", f"{PIN}:{SOURCE}"],
                         check=True, capture_output=True).stdout
    text = raw.decode()
    lines = text.splitlines()
    keywords = []
    witnesses = []
    for key, line, trigger in SPEECHES:
        match = re.match(r'\s*npcHandler:say\(("(?:[^"\\]|\\.)*"|\{.*?\}), npc, creature\)',
                         "\n".join(lines[line - 1:]), re.DOTALL)
        if not match:
            raise ValueError(f"Pinned Jack Springer speech moved: {line}")
        keywords.append({"key": key, "triggers": [trigger], "reply": literal(match[1], line)})
        witnesses.append({"key": key, "line": line, "kind": "default_callback"})
    basic = re.compile(
        r'keywordHandler:addKeyword\(\s*\{\s*("(?:[^"\\]|\\.)*")\s*\},\s*StdModule.say,\s*'
        r'\{\s*npcHandler\s*=\s*npcHandler,\s*text\s*=\s*("(?:[^"\\]|\\.)*"|\{.*?\})\s*,?\s*\}\s*\)',
        re.DOTALL,
    )
    for match in basic.finditer(text):
        keyword = json.loads(match[1])
        line = text[:match.start()].count("\n") + 1
        keywords.append({"key": "basic_" + keyword, "triggers": [keyword], "reply": literal(match[2], line)})
        witnesses.append({"key": "basic_" + keyword, "line": line, "kind": "keyword_handler"})
    if len(keywords) != 33:
        raise ValueError(f"Pinned Jack Springer reply closure changed: {len(keywords)}")
    greetings = []
    for line in (65, 67):
        match = re.fullmatch(r'\s*npcHandler:setMessage\(MESSAGE_GREET, ("(?:[^"\\]|\\.)*")\)', lines[line - 1])
        if not match:
            raise ValueError(f"Pinned Jack Springer greeting moved: {line}")
        greetings.append(json.loads(match[1]))
    source = {"repository": "zimbadev/crystalserver", "revision": PIN, "path": SOURCE,
              "sha256": hashlib.sha256(raw).hexdigest(), "speech": witnesses,
              "greeting_lines": [65, 67],
              "keyword_precedence": "data/npclib/npc_system/npc_handler.lua:448-451",
              "keyword_matcher": "data/npclib/npc_system/keyword_handler.lua:58-67"}
    dialogue = {"schema": "OTERYN_DIALOGUE_AUTHORING_SHARD/v1", "family": "Dialogue",
                "shard": {"index": 0, "start": 0, "end": 0, "count": 1}, "source_legacy_role": SOURCE,
                "records": [{"declaration": {
                    "kind": "Dialogue", "identity": {"key": "oteryn:dialogue.npc.jack_springer", "revision": "definition-r1"},
                    "greet": [greetings[0]], "keywords": keywords,
                    "fields": [
                        {"field_path": "oteryn:source.quest.grave_danger.dialogue",
                         "value": {"type": "Text", "value": json.dumps(source, ensure_ascii=False, sort_keys=True)}},
                        {"field_path": "oteryn:source.quest.grave_danger.started_greeting",
                         "value": {"type": "Text", "value": greetings[1]}},
                    ],
                }}]}

    def track(key, initial, low, maximum, storage):
        return {"key": key, "quest": QUEST, "initial": initial, "min": low, "max": maximum,
                "bounds_basis": "source_closed_values_with_canonical_initial",
                "source_key": f"storage:Quest.U12_20.GraveDanger.{storage}"}

    def effect(track, comparison, value, result, kind="SET"):
        guard = {"op": comparison}
        if value is not None:
            guard["value"] = value
        return {"track": track, "from": guard, "from_exact": True,
                "effect": {"kind": kind, "value": result}}

    def transition(key, keyword, topics, line, effects):
        return {"key": key, "quest": QUEST, "completes": False, "effects": effects,
                "requested_by": {"npc": "crystal:npc/jack_springer", "keywords": [keyword], "topics": topics},
                "source": {"repository": "zimbadev/crystalserver", "revision": PIN, "path": SOURCE, "line": line}}

    state = {"schema": "OTERYN_QUEST_STATE_LOWERING/v1", "classification": "OTS_HYPOTHESIS_ONLY",
             "family": "Quest",
             "contract": "docs/architecture/reviews/OTERYN_GAME_QUEST_STATE0_QUEST_PROGRESS_STORE_DECISION_2026-09-30.md",
             "authoring_sources": [source], "counts": {"quests": 1, "tracks": 3, "transitions": 1},
             "quests": [{"quest": QUEST, "source_quest": "crystal:quest/grave_danger",
                         "completion": "NOT_LOWERED_MULTI_TRACK",
                         "tracks": [track(QUESTLINE, 0, 0, 14, "QuestLine"),
                                    track(DOOR, 0, -1, 1, "Bosses.KingZelosDoor"),
                                    track(OUTFIT, -1, -1, 1, "Bosses.InquisitionOutfitReceived")],
                         "transitions": [
                             transition(ACCESS, "threat", [2], 119,
                                        [effect(QUESTLINE, "EQ", 2, 3), effect(DOOR, "ANY", None, 1)]),
                         ]}]}
    start = {"source": {**source, "line": 127}, "runtime_status": "LEFT_OUT_COORDINATOR_OWNER",
             "answer": "https://github.com/Oteryn/Oteryn-Game/pull/2844#issuecomment-6589521017",
             "reason": "Point5 reserves the existing8-effect bound; atomic start must not be split",
             "transition": transition(START, "late", [], 127,
                                      [effect(QUESTLINE, "LT", 1, 1)] +
                                      [effect(PREFIX + "graves/" + name, "ANY", None, 1) for name in GRAVES])}
    final = {"source": {**source, "line": 80}, "runtime_status": "LEFT_OUT_APPEARANCE_MAPPING",
             "question": "https://github.com/Oteryn/Oteryn-Game/pull/2844#issuecomment-6589624980",
             "reason": "No admitted selectable mapping for both source outfits; no completion flag without reward",
             "source_outfits": [{"look_type": 1244, "addon": 0, "line": 85},
                                {"look_type": 1243, "addon": 0, "line": 86}],
             "achievement": {"name": "Inquisition's Hand", "line": 91},
             "transitions": [transition(FINISH, "report", [], 80,
                                         [effect(QUESTLINE, "EQ", 4, 5), effect(OUTFIT, "LT", 1, 1)]),
                             transition(FINISH_HELD, "report", [], 80,
                                         [effect(QUESTLINE, "EQ", 4, 5), effect(OUTFIT, "GE", 1, 0, "ADD")])]}
    return {ROOT / "content/dialogues/grave-danger.json": dialogue,
            ROOT / "content/quests/grave-danger-npc-state.json": state,
            ROOT / "content/quests/grave-danger-start-source.json": start,
            ROOT / "content/quests/grave-danger-final-source.json": final}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crystal", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for path, data in build(args.crystal).items():
        expected = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
        if args.check:
            if path.read_text() != expected:
                raise SystemExit(f"Stale Grave Danger content: {path.relative_to(ROOT)}")
        else:
            path.write_text(expected)


if __name__ == "__main__":
    main()
