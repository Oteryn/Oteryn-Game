"""Assemble bounded native-goal cohorts, checking every captured spawn origin."""

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
COHORTS = ROOT / "tools/content-schema/quest-authoring/samples/native-kill-goal-cohorts"
OUTPUT = ROOT / "tools/content-schema/quest-authoring/samples/native-kill-goals.json"


SPAWNS = "content/world/spawns/spawns-34000-35999.json"
RECIPE = "tools/content-schema/quest-authoring/samples/completion242/recipes.json"
CENTRES = (
    (32278, 32467),
    (32282, 32469),
    (32282, 32473),
    (32235, 32474),
    (32273, 32476),
    (32255, 32477),
    (32238, 32479),
    (32278, 32479),
    (32266, 32480),
    (32233, 32482),
    (32270, 32485),
    (32242, 32487),
    (32235, 32488),
    (32247, 32490),
    (32256, 32490),
    (32266, 32493),
    (32278, 32494),
    (32237, 32495),
    (32248, 32495),
    (32256, 32498),
    (32273, 32498),
)


def fibula_document():
    raw = (ROOT / SPAWNS).read_bytes()
    records = json.loads(raw)["records"]
    # Explicit source declarations select the final cavern; no rectangle is invented.
    wanted = {f"oteryn:spawn.x{x}_y{y}_z10" for x, y in CENTRES}
    origins = []
    found = set()
    for index, row in enumerate(records):
        declaration = row["declaration"]
        key = declaration["identity"]["key"]
        if key not in wanted:
            continue
        found.add(key)
        for ordinal, point in enumerate(declaration["points"]):
            if point["creature"] not in (
                "oteryn:creature.dragon",
                "oteryn:creature.dragon_lord",
            ):
                raise ValueError("chosen cavern species drift")
            cell = point["cell"]
            origins.append(
                {
                    "creature": point["creature"],
                    "x": cell["x"],
                    "y": cell["y"],
                    "floor": cell["floor"],
                    "source_pointer": f"/records/{index}/declaration/points/{ordinal}",
                }
            )
    if found != wanted:
        raise ValueError(
            "chosen native spawn declarations missing: " + str(sorted(wanted - found))
        )
    recipe_raw = (ROOT / RECIPE).read_bytes()
    (recipe,) = [
        row["recipe"]
        for row in json.loads(recipe_raw)["records"]
        if row["identity"]["key"] == "oteryn:quest.deeper_fibula_quest"
    ]
    (stage,) = [stage for stage in recipe["stages"] if stage["key"] == "s4"]
    if stage["targets"] != ["Dragon", "Dragon Lord"] or stage["count"] != 8:
        raise ValueError("chosen Fibula goal drift")
    return {
        "schema": 1,
        "goals": [
            {
                "key": "oteryn:quest-transition/native-kill/fibula-cavern",
                "quest": "oteryn:quest.deeper_fibula_quest",
                "phase": "oteryn:quest-progress/fibula/supplement",
                "counter": "oteryn:quest-progress/fibula/cavern_kills",
                "activation": "oteryn:quest-reward/fibula-key-first",
                "count": stage["count"],
                "origins": sorted(
                    origins, key=lambda row: (row["x"], row["y"], row["creature"])
                ),
                "sources": [
                    {"path": SPAWNS, "sha256": hashlib.sha256(raw).hexdigest()},
                    {
                        "path": RECIPE,
                        "sha256": hashlib.sha256(recipe_raw).hexdigest(),
                        "lines": "6863-7011",
                    },
                ],
            }
        ],
    }


def document(*, refresh_spawn_source_pins=False):
    goals, keys = [], set()
    updates = {}
    for path in sorted(COHORTS.glob("*.json")):
        cohort = json.loads(path.read_bytes())
        if cohort.get("schema") != 1:
            raise ValueError("NATIVE_GOAL_COHORT_SCHEMA")
        for goal in cohort["goals"]:
            if goal["key"] in keys:
                raise ValueError("NATIVE_GOAL_DUPLICATE")
            keys.add(goal["key"])
            spawns = []
            for source in goal["sources"]:
                relative = Path(source["path"])
                if (
                    relative.is_absolute()
                    or ".." in relative.parts
                    or not source["path"].startswith(
                        (
                            "content/world/spawns/",
                            "tools/content-schema/quest-authoring/samples/",
                        )
                    )
                ):
                    raise ValueError("NATIVE_GOAL_SOURCE_SCOPE")
                raw = (ROOT / relative).read_bytes()
                is_spawn = source["path"].startswith("content/world/spawns/spawns-")
                digest = hashlib.sha256(raw).hexdigest()
                if digest != source["sha256"]:
                    if not refresh_spawn_source_pins or not is_spawn:
                        raise ValueError("NATIVE_GOAL_SOURCE_DRIFT")
                    source["sha256"] = digest
                    updates[path] = cohort
                if is_spawn:
                    spawns.append(json.loads(raw))
            for origin in goal["origins"]:
                matches = 0
                for source in spawns:
                    value = source
                    try:
                        for field in origin["source_pointer"].strip("/").split("/"):
                            value = (
                                value[int(field)]
                                if isinstance(value, list)
                                else value[field]
                            )
                    except (KeyError, IndexError, ValueError):
                        continue
                    if value.get("creature") == origin["creature"] and value.get(
                        "cell"
                    ) == {"x": origin["x"], "y": origin["y"], "floor": origin["floor"]}:
                        matches += 1
                if matches != 1:
                    raise ValueError("NATIVE_GOAL_EXACT_ORIGIN")
            goals.append(goal)
    fibula_goals = fibula_document()["goals"]
    if not goals or len(goals) + len(fibula_goals) > 16:
        raise ValueError("NATIVE_GOAL_BOUND")
    goals.extend(fibula_goals)
    for path, cohort in updates.items():
        path.write_text(json.dumps(cohort, indent=2) + "\n")
    return {"schema": 1, "goals": sorted(goals, key=lambda g: g["key"])}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--refresh-spawn-source-pins", action="store_true")
    args = parser.parse_args()
    raw = (
        json.dumps(
            document(refresh_spawn_source_pins=args.refresh_spawn_source_pins), indent=2
        )
        + "\n"
    )
    if args.check:
        if OUTPUT.read_text() != raw:
            raise SystemExit("NATIVE_KILL_GOAL_DRIFT")
    else:
        OUTPUT.write_text(raw)
