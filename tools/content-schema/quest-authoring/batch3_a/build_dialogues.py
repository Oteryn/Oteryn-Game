"""Emit pinned batch-three A NPC rules without evaluating donor Lua.

The reviewed source documents live beside this generator. Rule order is a
semantic part of Lua if/elseif lowering and is deliberately never sorted.
"""

import argparse
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
PIN = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:"
QUESTS = {
    "djinn_war_efreet": "oteryn:quest.the_djinn_war_efreet_faction",
    "djinn_war_marid": "oteryn:quest.the_djinn_war_marid_faction",
    "white_raven_monastery": "oteryn:quest.the_white_raven_monastery_quest",
    "bigfoot_burden": "oteryn:quest.bigfoot_s_burden_quest",
    "fathers_burden": "oteryn:quest.a_father_s_burden_quest",
    "hot_cuisine": "oteryn:quest.hot_cuisine_quest",
    "in_service_of_yalahar": "oteryn:quest.in_service_of_yalahar_quest",
    "the_hidden_city_of_beregar": "oteryn:quest.the_hidden_city_of_beregar_quest",
    "the_cursed_crystal": "oteryn:quest.tibia_tales",
    "the_way_of_the_monk": "oteryn:quest.the_way_of_the_monk_quest",
}


def validate_document(document):
    quest = document.get("quest")
    if document.get("schema") != 1 or quest not in QUESTS.values():
        raise ValueError("Wrong quest document")
    keys = set()
    for track in document["tracks"]:
        if track["quest"] != quest or not track["min"] <= track["initial"] <= track["max"]:
            raise ValueError(f"Wrong track owner or bounds: {track['key']}")
    for rule in document["rules"]:
        if rule["key"] in keys:
            raise ValueError(f"Duplicate rule: {rule['key']}")
        keys.add(rule["key"])
        if rule["quest"] != quest or not rule["npc"].startswith("oteryn:npc."):
            raise ValueError(f"Wrong rule owner: {rule['key']}")
        if rule["trigger"] not in {"say", "greet"}:
            raise ValueError(f"Wrong trigger: {rule['key']}")
        if not rule["sources"] or any(not source.startswith(PIN) for source in rule["sources"]):
            raise ValueError(f"Unpinned source: {rule['key']}")
        if rule["trigger"] == "say" and not rule["keywords"] and not rule.get("any_message", False):
            raise ValueError(f"Missing utterance selector: {rule['key']}")
        if rule.get("keyword_mode", "msg_contains") not in {"msg_contains", "substring", "exact"}:
            raise ValueError(f"Unknown utterance match: {rule['key']}")
        if any(not isinstance(reply, str) or not reply for reply in rule["reply"]):
            raise ValueError(f"Empty reply: {rule['key']}")


def materialize(quest, check=False):
    source = HERE / "npc" / f"{quest}.json"
    target = REPO / "content/dialogues/quest-rules" / f"quest-batch3-a-{quest}.json"
    document = json.loads(source.read_text())
    if document["quest"] != QUESTS[quest]:
        raise ValueError(f"Wrong quest source: {source}")
    validate_document(document)
    payload = json.dumps(document, ensure_ascii=False, indent=2) + "\n"
    if check:
        if not target.exists() or target.read_text() != payload:
            raise ValueError(f"Stale generated dialogue: {target}")
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(payload)
        legacy = REPO / "content/dialogues/definitions" / target.name
        if legacy.exists():
            # Only migrate an identical output from this generator, never an
            # ordinary authored dialogue shard that happens to share a name.
            legacy_document = json.loads(legacy.read_text())
            if legacy_document.get("schema") != 1 or legacy_document.get("quest") != QUESTS[quest] or "rules" not in legacy_document:
                raise ValueError(f"Refusing to migrate unrelated file: {legacy}")
            legacy.unlink()
    return len(document["rules"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--quest", choices=sorted(QUESTS))
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    quests = [args.quest] if args.quest else [q for q in sorted(QUESTS) if (HERE / "npc" / f"{q}.json").exists()]
    for quest in quests:
        print(f"{quest}: {materialize(quest, args.check)} rules")


if __name__ == "__main__":
    main()
