#!/usr/bin/env python3
"""Author finite Threatened Dreams exchanges from retained source facts."""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PREFIX = "oteryn:quest-reward/questpc3/threatened/"
QUEST = "oteryn:quest.threatened_dreams_quest"
COUPLE = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission03_1"
BASE = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission03/"
CLOAK = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission05/swan_feather_cloak"


def storage(track, low, high, value, preserve=False):
    return {"track": track, "minimum": low, "maximum": high, "set": value, "preserve": preserve}


def documents():
    archive = "tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz"
    rows = []
    def add(name, npc, keywords, topic, track, low, high, value, take, item, line, blob, digest):
        rows.append({"key": PREFIX + name, "quest": QUEST, "npc": "oteryn:npc." + npc,
                     "keywords": keywords, "topic": topic, "next_topic": 0,
                     "storage": [storage(track, low, high, value), storage(COUPLE, 4, 4, 4, True)],
                     "take": [{"item": "oteryn:item.tibia.i" + str(ident), "count": count} for ident, count in take],
                     "give": {"item": "oteryn:item.tibia.i" + str(item), "count": 1},
                     "outfit": None, "achievement": None,
                     "reply": ["Here is the item you need. Thank you for helping Feyrist."],
                     "sources": [archive + "#trees/crystalserver-9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.json",
                                 f"data-global/npc/{npc}.lua:{line} git-blob={blob} sha256={digest}"]})
    add("aurita-vial", "aurita", ["starlight vial", "empty starlight vial"], 0,
        BASE + "empty_starlight_vial", -1, 0, 1, [], 25731, "81-88",
        "ebaf7b39b5d59d1642d60a9dbe763219dda00724", "d8a1fe20f4ec11245498191b0269ca76ca5949c92db529e640faacc87a57ea35")
    add("taegen-catcher", "taegen", ["yes"], 3, BASE + "dark_sun_catcher", -1, 0, 1,
        [(675,2),(676,2),(677,2),(678,2)], 25733, "174-188",
        "bf435e06539adfba12820ba042310d76da4c630e", "cd1bd27585798ae5a732a30c5a6f96f4ec6e8400ad4e2854d1dda945b040e74f")
    add("valindara-cloak", "valindara", ["yes"], 1, CLOAK, 1, 1, 2, [(26181,100)], 25779, "75-90",
        "572697b6e855387cefc8e6336ea55dd6ee304678", "8cd64f9b2e7d678e91a07485789f4b08c168b7f52d1c741ecb77cfca3fbeb4fa")
    path = "content/quests/shared-reward-callbacks.json"
    document = json.loads((ROOT / path).read_text())
    document["callbacks"] = [c for c in document["callbacks"] if not c["key"].startswith(PREFIX)] + rows
    document["callbacks"].sort(key=lambda c: c["key"])
    result = {path: document}
    path = "content/dialogues/quests/shared-reward-greetings.json"
    document = json.loads((ROOT / path).read_text())
    names = {c["npc"] for c in rows}
    document["npcs"] = [row for row in document["npcs"] if row["npc"] not in names]
    document["npcs"] += [{"npc": npc, "greet": "Welcome, |PLAYERNAME|. How can I help you?",
                          "farewell": "Safe travels, |PLAYERNAME|.", "walkaway": "Safe travels."} for npc in sorted(names)]
    document["npcs"].sort(key=lambda row: row["npc"])
    result[path] = document
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    drift = []
    for relative, document in documents().items():
        path = ROOT / relative
        text = json.dumps(document, indent=2, ensure_ascii=False) + "\n"
        if path.read_text() != text:
            if args.check:
                drift.append(relative)
            else:
                path.write_text(text)
    if drift:
        raise SystemExit("Threatened inventory authoring drift: " + ", ".join(drift))


if __name__ == "__main__":
    main()
