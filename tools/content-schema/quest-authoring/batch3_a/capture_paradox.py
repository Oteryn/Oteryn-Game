"""Capture the pinned Riddler callback with the existing closed literal reader.

Input is a separately fetched reference snapshot, never bundled donor source.
"""
import argparse
import hashlib
import json
from pathlib import Path

from capture import Capture
from lower_dialogue import Lowering

SOURCE_SHA256 = "198907507cb4bf3c40b7c00abdf75a5ebb0e260e6e1b18683a39b652b711dc8a"
QUEST = "the_paradox_tower"


class RiddlerCapture(Capture):
    def __init__(self, source):
        self.snapshot = Path(source).read_bytes()
        if hashlib.sha256(self.snapshot).hexdigest() != SOURCE_SHA256:
            raise ValueError("Riddler snapshot differs from pinned Crystal201189f source")
        self.quest = QUEST
        self.key = "oteryn:quest.the_paradox_tower_quest"
        self.prefix = "oteryn:quest-progress/quest/u7_24/the_paradox_tower"
        self.rules, self.tracks, self.files = [], {}, {}

    def read(self, npc):
        if npc != "riddler":
            raise ValueError("Riddler capture cannot read a different NPC")
        self.files["data-global/npc/riddler.lua"] = SOURCE_SHA256
        return self.snapshot.decode()

    def track(self, name, maximum=2147483647, line=None, npc=None):
        anchor = {"TheFearedHugo": 78, "FavoriteColour": 109, "Mathemagics": 129}[name]
        return super().track(name, maximum, line=anchor, npc="riddler")


def capture(source):
    result = RiddlerCapture(source)
    # KeywordHandler consumes these before CALLBACK_MESSAGE_DEFAULT, including aliases.
    for line, aliases in [(160, ["tower"]), (163, []), (164, []), (165, []), (166, []), (167, ["door"])]:
        literals = result.strings("riddler", line, line)
        result.rule("riddler", "keyword-" + literals[0], line,
                    keywords=[literals[0], *aliases], replies=[literals[-1]])
    lower = Lowering(result, "riddler", "storage")
    # The line-oriented block splitter must not treat words inside a multiline
    # literal as Lua keywords. Replies still come from the original literal decoder.
    for start, line in enumerate(lower.lines):
        if line.strip() != "npcHandler:say(":
            continue
        stop = start + 1
        while lower.lines[stop].strip() != ")":
            lower.lines[stop] = ""
            stop += 1
        lower.lines[stop] = ""
        reply = result.say("riddler", start + 1)[0]
        lower.lines[start] = "npcHandler:say(" + json.dumps(reply) + ", npc, creature)"
    # Reviewed literal table positions; the reader otherwise accepts Position(...).
    for line, xyz in [(139, (32478, 31905, -1)), (144, (32725, 31589, -12)),
                      (150, (32725, 31589, -12)), (154, (32725, 31589, -12))]:
        lower.statement_overrides[line] = [{"kind": "teleport", "position": dict(zip(("x", "y", "floor"), xyz))}]
    lower.lower(57, 156)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--riddler-source", type=Path, required=True)
    args = parser.parse_args()
    capture(args.riddler_source).save()


if __name__ == "__main__":
    main()
