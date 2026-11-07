#!/usr/bin/env python3
"""Author questpc3's source NPC programs; no donor code is executed or copied."""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SOURCE = "https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/santiago.lua"
ZIRELLA_SOURCE = SOURCE.rsplit("/", 1)[0] + "/zirella.lua"
BASE = "oteryn:quest-progress/quest/u8_2/the_beginning_quest/"
GREET = BASE + "santiago_npc_greet_storage"
LOG = BASE + "santiago_quest_log"
ZIRELLA_GREET = BASE + "zirella_npc_greet_storage"
ZIRELLA_LOG = BASE + "zirella_quest_log"
OWNED_NPCS = {"oteryn:npc.santiago", "oteryn:npc.zirella"}


def storage(track, low, high, value):
    return {"track": track, "minimum": low, "maximum": high, "set": value, "preserve": False}


def callback(name, keywords, topic, next_topic, before, after, old_log, new_log, line, reply,
             *, npc="oteryn:npc.santiago", greet=GREET, log=LOG, source=SOURCE,
             experience=None, take=(), give=None):
    result = {
        "key": "oteryn:quest-reward/questpc3/beginning/" + name,
        "quest": "oteryn:quest.the_beginning_quest", "npc": npc,
        "keywords": keywords, "topic": topic, "next_topic": next_topic,
        "storage": [storage(greet, before[0], before[1], after), storage(log, old_log[0], old_log[1], new_log)],
        "take": list(take), "give": give, "outfit": None, "achievement": None,
        "reply": [reply], "sources": [f"{source}#L{line}"],
    }
    if experience is not None:
        result["experience"] = experience
    return result


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
        callback("legs-handin", ["hi", "hello", "greetings"], 0, 4, (6, 6), 7, (4, 4), 5, 88,
                 "You brought three cockroach legs. Here are 100 experience points. Did you see your level advance?",
                 take=[{"item": "oteryn:item.tibia.i7882", "count": 3}], experience=100),
        callback("food-grant", ["yes", "right", "ok"], 7, 8, (10, 10), 11, (8, 8), 9, 189,
                 "Take this fish. Use it from your inventory to eat and start recovering health. Easy, yes?",
                 give={"item": "oteryn:item.tibia.i3578", "count": 1}),
        callback("zirella-reward", ["yes", "quest", "ok"], 8, 0, (7, 7), 8, (7, 7), 8, 153,
                 "Thank you for the firewood. You earned 50 experience points, and may take what is in the chest beside my bed.",
                 npc="oteryn:npc.zirella", greet=ZIRELLA_GREET, log=ZIRELLA_LOG,
                 source=ZIRELLA_SOURCE, experience=50),
    ]
    for stage, line, reply in [
        (1, 107, "I need firewood for my oven. Will you help me?"),
        (2, 112, "Find a dead tree without leaves in the forest to the south."),
        (3, 117, "Use the dead tree to break a branch from it."),
        (4, 122, "Move the branch across the ground, then use it with my cart."),
        (5, 127, "Drag and drop the branch across the grass until it is near my cart."),
        (6, 139, "Use a dead tree, drag its branch across the map, and use that ground branch with my cart."),
    ]:
        before = (-1, 0) if stage == 1 else (stage - 1, stage - 1)
        callbacks.append(callback(
            f"zirella-lesson-{stage}", ["yes", "quest", "ok"], stage, stage + 1,
            before, stage, before, stage, line, reply,
            npc="oteryn:npc.zirella", greet=ZIRELLA_GREET, log=ZIRELLA_LOG,
            source=ZIRELLA_SOURCE))
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
    offers.append({"npc": "oteryn:npc.zirella", "keywords": ["hi", "hello", "greetings"],
                   "topic": 0, "next_topic": 8,
                   "storage": [storage(ZIRELLA_GREET, 7, 7, 7)],
                   "reply": ["That is enough firewood for my oven. Are you ready for your reward?"]})
    for stage in range(0, 7):
        low = -1 if stage == 0 else stage
        offers.append({"npc": "oteryn:npc.zirella", "keywords": ["hi", "hello", "greetings"],
                       "topic": 0, "next_topic": 0 if stage == 0 else stage + 1,
                       "storage": [storage(ZIRELLA_GREET, low, stage, stage)],
                       "reply": ["Could you help me with a quest?" if stage == 0 else
                                 "Welcome back. Shall I explain the firewood task?"]})
    offers.extend([
        {"npc": "oteryn:npc.zirella", "keywords": ["yes", "quest", "ok"],
         "topic": 0, "next_topic": 1,
         "storage": [storage(ZIRELLA_GREET, -1, 0, 0)],
         "reply": ["Quest is a keyword used to ask NPCs about tasks. Are you listening?"]},
        {"npc": "oteryn:npc.zirella", "keywords": ["yes", "quest", "ok", "explain"],
         "topic": 7, "next_topic": 0,
         "storage": [storage(ZIRELLA_GREET, 6, 6, 6)],
         "reply": ["Use a dead tree, drag the branch along the ground to my cart, then use it with the cart."]},
    ])
    greetings = {"schema": 1, "npcs": [{"npc": "oteryn:npc.santiago",
                  "greet": "Welcome to Rookgaard, |PLAYERNAME|. Can you help me?",
                  "farewell": "Take care, |PLAYERNAME|.",
                  "walkaway": "Enjoy your stay on Rookgaard."},
                 {"npc": "oteryn:npc.zirella",
                  "greet": "Hello, |PLAYERNAME|. Can you help me with a quest?",
                  "farewell": "Thank you, dear. Take care.",
                  "walkaway": "Come back if you need help with the firewood."}]}
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
        if target.exists():
            existing = json.loads(target.read_text())
            field = "callbacks" if "callbacks" in document else "offers" if "offers" in document else "npcs"
            foreign = [record for record in existing[field]
                       if (not record["key"].startswith("oteryn:quest-reward/questpc3/")
                           if field == "callbacks" else record["npc"] not in OWNED_NPCS)]
            document[field].extend(foreign)
            for key, value in existing.items():
                if key not in document:
                    document[key] = value
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
