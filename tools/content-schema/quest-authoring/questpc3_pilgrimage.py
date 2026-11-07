#!/usr/bin/env python3
"""Author the chosen Pilgrimage route and canonical blessing Services offline."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PREFIX = "oteryn:quest-reward/questpc3/pilgrimage/"
QUEST = "oteryn:quest.pilgrimage_of_ashes"
STAGE = "oteryn:quest-progress/questpc3/pilgrimage_of_ashes/stage"
GUIDES = ("alexena", "behil", "davina", "edna", "elena", "jonathan",
          "kunibert", "luke", "meruka", "rahlkora", "thelandil", "tiko")
VISITS = (("norf", "the_spiritual_shielding"), ("humphrey", "the_embrace_of_tibia"),
          ("edala", "the_fire_of_the_suns"), ("kawill", None),
          ("pydar", "the_spark_of_the_phoenix"), ("eremo", "the_wisdom_of_solitude"))


def encoded(value):
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def documents():
    recipe = "content/quests/definitions/quests-00100-00199.json:1#/records/85/definition/oteryn_recipe/payload/recipe"
    blessings_path = ROOT / "rulesets/character/blessings/catalogue.json"
    blessings = json.loads(blessings_path.read_text())["blessings"]
    source = [recipe + "@5e72a3f0b", "rulesets/character/blessings/catalogue.json:1 sha256=" + hashlib.sha256(blessings_path.read_bytes()).hexdigest(),
              "owner:questpc3:2026-11-07:actual-blessings-with-chosen-route-and-20-platinum"]
    callbacks = []

    def add(name, npc, low, high, value, blessing=None, final=False):
        callbacks.append({"key": PREFIX + name, "quest": QUEST, "npc": "oteryn:npc." + npc,
                          "keywords": ["pilgrimage", "mission"], "topic": 0, "next_topic": 0,
                          "storage": [{"track": STAGE, "minimum": low, "maximum": high,
                                       "set": value, "preserve": False}], "take": [],
                          "give": {"item": "oteryn:item.tibia.i3035", "count": 20} if final else None,
                          "outfit": None, "achievement": None, "blessing": blessing,
                          "premium": True, "completes": final,
                          "reply": ["Your pilgrimage is complete. Here are twenty platinum coins." if final
                                    else "Your visit is recorded. Continue your pilgrimage."], "sources": source})

    for guide in GUIDES:
        add("guide_" + guide + "-start", "guide_" + guide, -1, 0, 1)
        add("guide_" + guide + "-finish", "guide_" + guide, 7, 7, 8, final=True)
    for stage, (npc, blessing) in enumerate(VISITS, 1):
        add(npc, npc, stage, stage, stage + 1,
            "oteryn:service.blessing." + blessing if blessing else None)
    path = "content/quests/shared-reward-callbacks.json"
    document = json.loads((ROOT / path).read_text())
    document["callbacks"] = [c for c in document["callbacks"] if not c["key"].startswith(PREFIX)] + callbacks
    document["callbacks"].sort(key=lambda c: c["key"])
    result = {path: document}
    path = "content/dialogues/quests/shared-reward-greetings.json"
    document = json.loads((ROOT / path).read_text())
    names = {c["npc"] for c in callbacks}
    document["npcs"] = [n for n in document["npcs"] if n["npc"] not in names]
    document["npcs"] += [{"npc": npc, "greet": "Welcome, |PLAYERNAME|. Ask me about your pilgrimage.",
                          "farewell": "Safe travels, |PLAYERNAME|.", "walkaway": "Safe travels."} for npc in sorted(names)]
    document["npcs"].sort(key=lambda row: row["npc"])
    result[path] = document
    index_path = "content/services/blessings/index.json"
    index = json.loads((ROOT / index_path).read_text())
    records = {}
    template = None
    for shard in index["shards"]:
        data = json.loads((ROOT / shard).read_text())
        template = template or data
        for record in data["records"]:
            records[record["declaration"]["identity"]["key"]] = record
    wanted = {"oteryn:service.blessing." + blessing for _, blessing in VISITS if blessing}
    for blessing in blessings:
        if blessing["service"] not in wanted:
            continue
        records.setdefault(blessing["service"], {"declaration": {"fields": [],
                           "identity": {"key": blessing["service"], "revision": "definition-r1"}, "kind": "Service"}})
    data = copy.deepcopy(template)
    data["records"] = [records[key] for key in sorted(records)]
    count = len(records)
    data["shard"] = {"count": count, "end": count - 1, "index": 0, "start": 0}
    shard = f"content/services/blessings/blessing-00000-{count - 1:05}.json"
    result[shard] = data
    index["shards"] = [shard]
    index["record_count"] = count
    result[index_path] = index
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    outputs = documents()
    old = json.loads((ROOT / "content/services/blessings/index.json").read_text())["shards"]
    drift = []
    for relative, document in outputs.items():
        path = ROOT / relative
        text = (json.dumps(document, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"
                if relative.startswith("content/services/blessings/") else encoded(document))
        if not path.exists() or path.read_text() != text:
            if args.check:
                drift.append(relative)
            else:
                path.write_text(text)
    if args.check:
        if drift:
            raise SystemExit("Pilgrimage authoring drift: " + ", ".join(drift))
    else:
        for shard in old:
            if shard not in outputs:
                (ROOT / shard).unlink()


if __name__ == "__main__":
    main()
