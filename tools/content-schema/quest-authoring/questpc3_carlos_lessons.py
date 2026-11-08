#!/usr/bin/env python3
"""Author Carlos's source tutorial lessons through reusable quest callbacks."""
import argparse
import json
from pathlib import Path

from shared_reward_offers import replace_npc_offers

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
    rows.append({"key": PREFIX + "food-evidence", "quest": QUEST, "npc": NPC,
                 "keywords": ["yes", "help", "ok"], "topic": 5, "next_topic": 6,
                 "storage": [storage(GREET, 5, 5, 6), storage(LOG, 5, 5, 6),
                             storage(BASE + "carlos_npc_trade_storage", -1, 0, 1)],
                 "present_any": [{"item": "oteryn:item.tibia.i3577", "count": 1},
                                 {"item": "oteryn:item.tibia.i3582", "count": 1}],
                 "take": [], "give": None, "outfit": None, "achievement": None,
                 "reply": ["That smells delicious. Ask me for a trade and I will pay two gold pieces for each piece of meat or ham."],
                 "sources": [SOURCE + "#L168-L177"]})
    # The native trade leaves dialogue topic6 in place. The committed Greet7
    # milestone is the authoritative source post-sale readiness guard.
    for name, keywords, topic, line, reply in [
        ("report-ready", ["ready", "yes", "ok"], 6, "205-L218",
         "That is how trading works. You are ready to cross the bridge to Rookgaard. Follow the path to the northwest. Good luck, |PLAYERNAME|!"),
        ("report-greeting", ["hi", "hello", "greetings"], 0, "114-L119",
         "Welcome back, |PLAYERNAME|. That is how trading works. You are ready to cross the bridge to Rookgaard. Take care!"),
    ]:
        rows.append({"key": PREFIX + name, "quest": QUEST, "npc": NPC,
                     "keywords": keywords, "topic": topic, "next_topic": 0,
                     "storage": [storage(GREET, 7, 7, 8), storage(LOG, 6, 6, 7)],
                     "take": [], "give": None, "outfit": None, "achievement": None,
                     "reply": [reply], "sources": [SOURCE + "#L" + line]})
    offers = []
    for stage, topic, reply in [
        (1, 1, "Shall I explain how to change your outfit?"),
        (2, 2, "Welcome back. Could you help me with a little favour?"),
        (3, 3, "Could you help me find some food?"),
        (4, 4, "Will you bring me meat or ham from a rabbit or deer?"),
        (5, 5, "Did your hunt succeed? Have you brought meat or ham?"),
        (6, 6, "Do you still have meat or ham? Ask me for a trade to sell it."),
        (8, 0, "Hello again, |PLAYERNAME|. Head over the bridge to Rookgaard village now."),
    ]:
        offers.append({"npc": NPC, "keywords": ["hi", "hello", "greetings"],
                       "topic": 0, "next_topic": topic,
                       "storage": [storage(GREET, stage, stage, stage)],
                       "reply": [reply]})
    offers.append({"npc": NPC, "keywords": ["yes", "help", "ok"],
                   "topic": 6, "next_topic": 6,
                   "storage": [storage(GREET, 6, 6, 6), storage(LOG, 6, 6, 6),
                               storage(BASE + "carlos_npc_trade_storage", 1, 1, 1)],
                   "reply": ["Do you still have meat or ham? Ask me for a trade to sell it."]})
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
        if field == "offers":
            document[field] = replace_npc_offers(document[field], NPC, own)
        else:
            replacements = {row["key"] if field == "callbacks" else row["npc"]: row for row in own}
            retained = []
            for row in document[field]:
                key = row["key"] if field == "callbacks" else row["npc"]
                owned = key.startswith(PREFIX) if field == "callbacks" else key == NPC
                if not owned:
                    retained.append(row)
                elif key in replacements:
                    retained.append(replacements.pop(key))
            document[field] = retained + list(replacements.values())
        result[path] = document
    path = "content/quests/shared-trade-callbacks.json"
    target = ROOT / path
    document = json.loads(target.read_text()) if target.exists() else {"schema": 1, "callbacks": []}
    trade_key = "oteryn:quest-trade/questpc3/beginning/carlos-food-sale"
    document["callbacks"] = [row for row in document["callbacks"] if row["key"] != trade_key] + [{
        "key": trade_key, "quest": QUEST, "npc": NPC, "side": "sell",
        "items": ["oteryn:item.tibia.i3577", "oteryn:item.tibia.i3582"],
        "unit_price": 2, "offer_count": 1, "minimum_units": 1,
        "storage": [storage(GREET, 6, 6, 7),
                    {**storage(LOG, 6, 6, 6), "preserve": True},
                    {**storage(BASE + "carlos_npc_trade_storage", 1, 1, 1), "preserve": True}],
        "sources": [SOURCE + "#L46-L48",
                    "#owner-authorized-source-hook-repair: actual committed eligible food sale; no report-only progress"]}]
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
