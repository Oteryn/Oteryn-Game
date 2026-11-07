#!/usr/bin/env python3
"""Author Carlos's source tutorial lessons through reusable quest callbacks."""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PREFIX = "oteryn:quest-reward/questpc3/beginning-carlos/"
NPC = "oteryn:npc.carlos"
QUEST = "oteryn:quest.the_beginning_quest"
BASE = "oteryn:quest-progress/quest/u8_2/the_beginning_quest/"
GREET = BASE + "carlos_npc_greet_storage"
LOG = BASE + "carlos_quest_log"
SOURCE = "https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/carlos.lua"


def storage(track, low, high, value):
    return {"track": track, "minimum": low, "maximum": high, "set": value, "preserve": False}


def documents():
    rows = []
    lessons = [
        ("greet", 0, 1, -1, 0, 1, 85,
         "Welcome, |PLAYERNAME|. As a tailor and merchant, I can show you how to change your outfit. Shall I?"),
        ("outfit-lesson", 1, 0, 1, 1, 2, 145,
         "Choose an outfit and colours that suit you. You can open Set Outfit from your character's menu. Try it and talk to me again."),
        ("favor", 2, 3, 2, 2, 3, 154,
         "Standing here all day makes me hungry. Could you help me find some food?"),
        ("hunt", 3, 4, 3, 3, 4, 159,
         "Hunt rabbits or deer, loot them, and bring me a piece of meat or ham. I will pay you for it."),
        ("await-food", 4, 0, 4, 4, 5, 164,
         "I will await your return. Chase Opponent helps you follow fast creatures. Bring me meat or ham when you return."),
    ]
    for name, topic, next_topic, low, high, value, line, reply in lessons:
        rows.append({"key": PREFIX + name, "quest": QUEST, "npc": NPC,
                     "keywords": ["hi", "hello", "greetings"] if name == "greet" else ["yes", "help", "ok"],
                     "topic": topic, "next_topic": next_topic,
                     "storage": [storage(GREET, low, high, value), storage(LOG, low, high, value)],
                     "take": [], "give": None, "outfit": None, "achievement": None,
                     "reply": [reply], "sources": [f"{SOURCE}#L{line}"]})
    offers = []
    for stage, topic, reply in [
        (1, 1, "Shall I explain how to change your outfit?"),
        (2, 2, "Welcome back. Could you help me with a little favour?"),
        (3, 3, "Could you help me find some food?"),
        (4, 4, "Will you bring me meat or ham from a rabbit or deer?"),
        (5, 5, "Did your hunt succeed? Have you brought meat or ham?"),
    ]:
        offers.append({"npc": NPC, "keywords": ["hi", "hello", "greetings"],
                       "topic": 0, "next_topic": topic,
                       "storage": [storage(GREET, stage, stage, stage)],
                       "reply": [reply]})
    greetings = [{"npc": NPC, "greet": "Welcome, |PLAYERNAME|. Shall I explain how to change your outfit?",
                  "farewell": "Good luck with your hunt, |PLAYERNAME|.",
                  "walkaway": "Goodbye, |PLAYERNAME|."}]
    result = {}
    for path, field, own in [
        ("content/quests/shared-reward-callbacks.json", "callbacks", rows),
        ("content/quests/shared-reward-offers.json", "offers", offers),
        ("content/dialogues/quests/shared-reward-greetings.json", "npcs", greetings),
    ]:
        document = json.loads((ROOT / path).read_text())
        document[field] = [row for row in document[field]
                           if (not row["key"].startswith(PREFIX) if field == "callbacks" else row["npc"] != NPC)] + own
        if field in ("callbacks", "npcs"):
            document[field].sort(key=lambda row: row["key"] if field == "callbacks" else row["npc"])
        result[path] = document
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    drift = []
    for relative, document in documents().items():
        path = ROOT / relative
        text = json.dumps(document, indent=2) + "\n"
        if path.read_text() != text:
            if args.check:
                drift.append(relative)
            else:
                path.write_text(text)
    if drift:
        raise SystemExit("Carlos lesson drift: " + ", ".join(drift))


if __name__ == "__main__":
    main()
