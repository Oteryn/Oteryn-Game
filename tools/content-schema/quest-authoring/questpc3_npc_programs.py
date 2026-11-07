#!/usr/bin/env python3
"""Author questpc3's source NPC programs; no donor code is executed or copied."""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SOURCE = "https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/santiago.lua"
BASE = "oteryn:quest-progress/quest/u8_2/the_beginning_quest/"
GREET = BASE + "santiago_npc_greet_storage"
LOG = BASE + "santiago_quest_log"


def storage(track, low, high, value):
    return {"track": track, "minimum": low, "maximum": high, "set": value, "preserve": False}


def callback(name, keywords, topic, next_topic, before, after, old_log, new_log, line, reply):
    return {
        "key": "oteryn:quest-reward/questpc3/beginning/" + name,
        "quest": "oteryn:quest.the_beginning_quest", "npc": "oteryn:npc.santiago",
        "keywords": keywords, "topic": topic, "next_topic": next_topic,
        "storage": [storage(GREET, before[0], before[1], after), storage(LOG, old_log[0], old_log[1], new_log)],
        "take": [], "give": None, "outfit": None, "achievement": None,
        "reply": [reply], "sources": [f"{SOURCE}#L{line}"],
    }


def documents():
    callbacks = [
        callback("greet", ["hi", "hello", "greetings"], 0, 0, (-1, 0), 1, (-1, 0), 1, 68,
                 "Welcome to Rookgaard, |PLAYERNAME|. Could you help me? Say yes if you can."),
        callback("house-confirm", ["yes", "right", "ok"], 1, 0, (1, 1), 2, (1, 1), 2, 139,
                 "Your quest log records your progress. Visit the chest upstairs in my house and return wearing its coat."),
        callback("level-explained", ["yes", "right", "ok"], 4, 5, (7, 7), 8, (5, 5), 6, 177,
                 "Advancing improves your health, carrying capacity and speed. Did the cockroaches hurt you?"),
        callback("hurt-explained", ["yes", "right", "ok"], 5, 6, (8, 8), 9, (6, 6), 7, 183,
                 "There are more dangerous monsters beyond this tutorial. May I demonstrate why you should watch your health?"),
        callback("food-explained", ["yes", "right", "ok"], 8, 9, (11, 11), 12, (9, 9), 10, 210,
                 "You can find food on creatures in the forest. Have you seen Zirella nearby?"),
        callback("refer-zirella", ["yes", "right", "ok"], 9, 0, (12, 12), 13, (10, 10), 11, 216,
                 "Zirella lives east of here, down the mountain. Please visit her and ask whether she needs help."),
    ]
    offers = [{"npc": "oteryn:npc.santiago", "keywords": ["yes", "right", "ok"],
               "topic": 0, "next_topic": 1, "storage": [storage(GREET, 1, 1, 1)],
               "reply": ["Visit my house to the south and look in the upstairs chest. May I first explain your quest log?"]}]
    for stage, topic, reply in [
        (3, 2, "You found my chest. You put on that coat, yes?"),
        (4, 2, "I want to give you a weapon. Say yes."),
        (7, 4, "You saw your level advance, right?"),
        (8, 5, "Did those cockroaches hurt you?"),
        (9, 6, "May I demonstrate some damage?"),
        (10, 7, "Shall I show you how to regain health?"),
        (11, 8, "I gave you a fish to eat. Shall we continue?"),
        (12, 9, "Have you seen Zirella nearby?"),
    ]:
        offers.append({"npc": "oteryn:npc.santiago", "keywords": ["hi", "hello", "greetings"],
                       "topic": 0, "next_topic": topic,
                       "storage": [storage(GREET, stage, stage, stage)], "reply": [reply]})
    greetings = {"schema": 1, "npcs": [{"npc": "oteryn:npc.santiago",
                  "greet": "Welcome to Rookgaard, |PLAYERNAME|. Can you help me?",
                  "farewell": "Take care, |PLAYERNAME|.",
                  "walkaway": "Enjoy your stay on Rookgaard."}]}
    return {"content/quests/shared-reward-callbacks.json": {"schema": 1, "callbacks": callbacks},
            "content/quests/shared-reward-offers.json": {"schema": 1, "offers": offers},
            "content/dialogues/quests/shared-reward-greetings.json": greetings}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    drift = []
    for relative, document in documents().items():
        target = ROOT / relative
        # This shared registry may receive independently authored lane records.
        # Replace only our NPC programs and retain all foreign entries.
        field = "callbacks" if "callbacks" in document else "offers" if "offers" in document else "npcs"
        if target.exists():
            existing = json.loads(target.read_text())
            foreign = [record for record in existing[field]
                       if (not record["key"].startswith("oteryn:quest-reward/questpc3/beginning/")
                           if field == "callbacks" else record["npc"] != "oteryn:npc.santiago")]
            document[field].extend(foreign)
        if field == "callbacks":
            document[field].sort(key=lambda record: record["key"])
        text = json.dumps(document, indent=2) + "\n"
        target.parent.mkdir(parents=True, exist_ok=True)
        if not target.exists() or target.read_text() != text:
            if args.check:
                drift.append(relative)
            else:
                target.write_text(text)
    if drift:
        raise SystemExit("NPC program drift: " + ", ".join(drift))


if __name__ == "__main__":
    main()
