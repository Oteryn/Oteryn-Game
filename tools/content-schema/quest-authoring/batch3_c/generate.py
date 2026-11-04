"""Generate the batch-three C source dialogue and native callback projections."""
import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    authoring = Path(__file__).resolve().parent
    root = authoring.parents[3]
    outputs = []
    for source in sorted((authoring / "npc").glob("*.json")):
        outputs.append((source, root / "content/dialogues/quest-rules" / ("quest-batch3-c-" + source.stem + ".json")))
    source = authoring / "heart_of_destruction_deaths.json"
    if source.is_file():
        outputs.append((source, root / "content/quests/heart-of-destruction-deaths.json"))
    source = authoring / "stag_death.json"
    if source.is_file():
        outputs.append((source, root / "content/quests/order-of-the-stag-death.json"))
    source = authoring / "gnomadness_native.json"
    if source.is_file():
        outputs.append((source, root / "content/dialogues/quest-native/gnomadness.json"))
    for source in sorted((authoring / "deaths").glob("*.json")):
        outputs.append((source, root / "content/quests/native-death-bindings" / ("quest-batch3-c-" + source.stem + ".json")))
    for source, target in outputs:
        document = json.loads(source.read_text())
        canonical = json.dumps(document, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"
        if args.check:
            if not target.is_file() or target.read_text() != canonical:
                raise SystemExit("stale generated batch-three quest data: " + str(target.relative_to(root)))
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(canonical)
    print(str(len(outputs)) + " batch-three C source projections " + ("verified" if args.check else "generated"))


if __name__ == "__main__":
    main()
