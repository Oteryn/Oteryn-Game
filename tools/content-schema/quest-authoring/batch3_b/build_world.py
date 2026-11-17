"""Materialize reviewed batch-three B callbacks into the existing world schema.

The input preserves Crystal selectors and positions. No map cell, creature,
instance owner or new command is created by this generator.
"""

import argparse
import json

from build_dialogues import HERE, QUESTS, REPO, SOURCE


def materialize(quest, check=False):
    source = HERE / "world" / f"{quest}.json"
    target = REPO / "content/quests/world-callbacks" / f"batch3-b-{quest}.json"
    document = json.loads(source.read_text())
    if document.get("schema") != 1 or document.get("quest") != QUESTS[quest]:
        raise ValueError(f"Wrong quest callback owner: {source}")
    keys = set()
    for callback in document["callbacks"]:
        if callback["quest"] != QUESTS[quest] or callback["key"] in keys:
            raise ValueError(f"Duplicate callback or foreign quest: {callback['key']}")
        keys.add(callback["key"])
        if callback["trigger"] not in {"use", "step_in"}:
            raise ValueError(f"Unknown callback trigger: {callback['key']}")
        if not callback["selector"] or not any(v is not None for v in callback["selector"].values()):
            raise ValueError(f"Unbound callback: {callback['key']}")
        if not callback["sources"] or any(not s.startswith(SOURCE) for s in callback["sources"]):
            raise ValueError(f"Unpinned callback: {callback['key']}")
    payload = json.dumps({"schema": 1, "callbacks": document["callbacks"]},
                         ensure_ascii=False, indent=2) + "\n"
    if check:
        if not target.exists() or target.read_text() != payload:
            raise ValueError(f"Stale generated world callbacks: {target}")
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(payload)
    held_outputs = (
        ("presentation", "quest-source", f"batch3-b-{quest}-presentation.json"),
        ("world", "world-callbacks-held", f"batch3-b-{quest}.json"),
    )
    for suffix, directory, name in held_outputs:
        held_source = HERE / "held" / f"{quest}-{suffix}.json"
        if not held_source.exists():
            continue
        held = json.loads(held_source.read_text())
        if held.get("quest") != QUESTS[quest] or held.get("runtime_enabled") is not False:
            raise ValueError(f"Unqualified source cannot be activated: {held_source}")
        held_target = REPO / "content/quests" / directory / name
        held_payload = json.dumps(held, ensure_ascii=False, indent=2) + "\n"
        if check:
            if not held_target.exists() or held_target.read_text() != held_payload:
                raise ValueError(f"Stale retained source: {held_target}")
        else:
            held_target.parent.mkdir(parents=True, exist_ok=True)
            held_target.write_text(held_payload)
    return len(document["callbacks"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--quest", choices=sorted(QUESTS))
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    quests = [args.quest] if args.quest else [
        quest for quest in sorted(QUESTS) if (HERE / "world" / f"{quest}.json").exists()]
    for quest in quests:
        print(f"{quest}: {materialize(quest, args.check)} world callbacks")


if __name__ == "__main__":
    main()
