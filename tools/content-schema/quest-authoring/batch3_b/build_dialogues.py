"""Materialize the batch-three B NPC rules; input order is Lua branch order.

The pure planner validates these typed source rules. Their live NPC caller is
Left out under PR2843#issuecomment-6589900671; no Lua text is evaluated and no
other quest's branches are rewritten.
"""

import argparse
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
QUESTS = {
    "a_pirates_tail_quest": "oteryn:quest.a_pirates_tail",
    "hero_of_rathleton": "oteryn:quest.hero_of_rathleton_quest",
    "marapur": "oteryn:quest.within_the_tides_quest",
    "oramond": "oteryn:quest.oramond_quest",
    "the_shattered_isles_quest": "oteryn:quest.the_shattered_isles_quest",
}
SOURCE = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:"


def materialize(quest, check=False):
    source = HERE / "npc" / f"{quest}.json"
    target = REPO / "content/dialogues/quest-rules" / f"quest-batch3-b-{quest}.json"
    legacy_target = REPO / "content/dialogues/definitions" / target.name
    document = json.loads(source.read_text())
    if document.get("schema") != 1 or document.get("quest") != QUESTS[quest]:
        raise ValueError(f"Wrong quest document: {source}")
    keys = set()
    for track in document["tracks"]:
        if track["quest"] != QUESTS[quest] or not track["min"] <= track["initial"] <= track["max"]:
            raise ValueError(f"Wrong track owner or bounds: {track['key']}")
    for rule in document["rules"]:
        if rule["key"] in keys or rule["quest"] != QUESTS[quest]:
            raise ValueError(f"Duplicate rule or foreign quest: {rule['key']}")
        keys.add(rule["key"])
        if rule["trigger"] not in {"say", "greet"}:
            raise ValueError(f"Wrong trigger: {rule['key']}")
        if not rule["npc"].startswith("oteryn:npc."):
            raise ValueError(f"Noncanonical NPC: {rule['key']}")
        if not rule["sources"] or any(not s.startswith(SOURCE) for s in rule["sources"]):
            raise ValueError(f"Unpinned source: {rule['key']}")
        if rule["trigger"] == "say" and not rule["keywords"]:
            raise ValueError(f"Missing utterance selector: {rule['key']}")
        if any(not isinstance(reply, str) or not reply for reply in rule["reply"]):
            raise ValueError(f"Empty reply: {rule['key']}")
    payload = json.dumps(document, ensure_ascii=False, indent=2) + "\n"
    if check:
        if not target.exists() or target.read_text() != payload:
            raise ValueError(f"Stale generated dialogue: {target}")
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(payload)
        # The custom quest-rule schema is not a general dialogue definition shard.
        # Remove only this generator's obsolete output when migrating its location.
        if legacy_target.exists():
            legacy_target.unlink()
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
