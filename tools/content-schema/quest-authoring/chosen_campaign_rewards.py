"""Generate itinerary completion without adding to source Grizzly task rewards."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def generate():
    recipes = []
    for path in sorted((ROOT / "content/quests/definitions").glob("quests-*.json")):
        for record in json.loads(path.read_text())["records"]:
            definition = record["definition"]
            if definition["identity"]["key"] == "oteryn:quest.killing_in_the_name_of_quest":
                recipes.append(definition["oteryn_recipe"]["payload"]["recipe"])
    if len(recipes) != 1:
        raise ValueError("campaign canonical identity must resolve once")
    recipe = recipes[0]
    kills = [(stage["count"], stage["targets"]) for stage in recipe["stages"] if stage["kind"] == "kill"]
    if len(recipe["stages"]) != 11 or kills != [
        (300, ["Crocodile"]), (1, ["The Snapper"]),
        (150, ["Carniphila"]), (1, ["Deathbine"]),
        (650, ["Hydra"]), (1, ["The Many"]),
    ]:
        raise ValueError("campaign rule amendment required for changed chosen stages")
    # The historical recipe labels its extra XP and gold as balancing choices.
    # Keep its source facts, but do not issue those authored bonuses at runtime.
    callback = {
        "key": "oteryn:quest-reward/chosen/hunting-campaign",
        "quest": "oteryn:quest.killing_in_the_name_of_quest",
        "npc": "oteryn:npc.grizzly_adams",
        "keywords": ["campaign"], "topic": 0, "next_topic": 0,
        "completes": True,
        "storage": [{"track": "oteryn:quest-progress/chosen/killing_in_the_name_of/phase",
                     "minimum": 10, "maximum": 10, "set": 11}],
        "required_receipts": [{'transition': 'oteryn:quest-transition/grizzly/report-boss-1', 'track': 'oteryn:quest-progress/storage/34100', 'value': 3}, {'transition': 'oteryn:quest-transition/grizzly/report-boss-3', 'track': 'oteryn:quest-progress/storage/34102', 'value': 3}, {'transition': 'oteryn:quest-transition/grizzly/report-boss-19', 'track': 'oteryn:quest-progress/storage/34118', 'value': 3}],
        "take": [], "give": None,
        "outfit": None, "achievement": None,
        "reply": ["Your hunting campaign is complete."],
        "sources": [
            "OTERYN_AUTHORED_APPROXIMATION: chosen Killing in the Name of... Quest itinerary, stages s1-s11; completion adds no XP or items to the source task and boss rewards.",
            "content/quests/definitions/quests-00100-00199.json#/records/64/definition/oteryn_recipe/payload/recipe",
            "https://tibia.fandom.com/index.php?curid=36695&oldid=843864",
        ]
    }
    authored = json.loads((Path(__file__).with_name("chosen-route-rewards.json")).read_text())
    if (authored.get("schema") != 1 or set(authored) != {"schema", "callbacks", "offers", "greetings", "tracks"}
            or len(authored["callbacks"]) > 128 or len(authored["offers"]) > 128
            or len(authored["greetings"]) > 64 or len(authored["tracks"]) > 64):
        raise ValueError("chosen route reward authoring bounds")
    keys = [row["key"] for row in authored["callbacks"]]
    if len(keys) != len(set(keys)) or callback["key"] in keys:
        raise ValueError("duplicate chosen reward callback")
    canonical = {record["definition"]["identity"]["key"]
                 for path in (ROOT / "content/quests/definitions").glob("quests-*.json")
                 for record in json.loads(path.read_text())["records"]}
    if any(row["quest"] not in canonical for row in authored["callbacks"]):
        raise ValueError("chosen route reward needs a canonical quest")
    return {
        "content/quests/chosen-reward-callbacks.json": {"schema": 1, "tracks": authored["tracks"], "callbacks": [callback, *authored["callbacks"]]},
        "content/quests/chosen-reward-offers.json": {"schema": 1, "offers": authored["offers"]},
        "content/dialogues/quests/chosen-reward-greetings.json": {"schema": 1, "npcs": authored["greetings"]},
    }


if __name__ == "__main__":
    import sys
    for name, data in generate().items():
        raw = json.dumps(data, indent=2) + "\n"
        path = ROOT / name
        if "--check" in sys.argv:
            if path.read_text() != raw:
                raise SystemExit("stale chosen reward output: " + name)
        else:
            path.write_text(raw)
