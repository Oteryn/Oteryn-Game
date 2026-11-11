"""Generate the chosen campaign reward over the reused atomic NPC reward engine.

The native coin owner represents 5,000 gold as 50 platinum coins (100 gold each).
This preserves exact currency value without inventing an oversized coin stack.
"""
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
    experience = next(reward["count"] for reward in recipe["reward_intents"] if reward["kind"] == "experience")
    gold = next(reward["count"] for reward in recipe["reward_intents"] if reward["name"] == "Gold Coin")
    if experience != 10000 or gold != 5000:
        raise ValueError("campaign reward amendment required for changed chosen values")
    if gold % 100 or not 0 < gold // 100 <= 100:
        raise ValueError("campaign currency requires a supported exact denomination plan")
    callback = {
        "key": "oteryn:quest-reward/chosen/hunting-campaign",
        "quest": "oteryn:quest.killing_in_the_name_of_quest",
        "npc": "oteryn:npc.grizzly_adams",
        "keywords": ["campaign"], "topic": 0, "next_topic": 0,
        "completes": True, "experience": experience,
        "storage": [{"track": "oteryn:quest-progress/chosen/killing_in_the_name_of/phase",
                     "minimum": 10, "maximum": 10, "set": 11}],
        "required_receipts": [{'transition': 'oteryn:quest-transition/grizzly/report-boss-1', 'track': 'oteryn:quest-progress/storage/34100', 'value': 3}, {'transition': 'oteryn:quest-transition/grizzly/report-boss-3', 'track': 'oteryn:quest-progress/storage/34102', 'value': 3}, {'transition': 'oteryn:quest-transition/grizzly/report-boss-19', 'track': 'oteryn:quest-progress/storage/34118', 'value': 3}],
        "take": [], "give": {"item": "oteryn:item.tibia.i3035", "count": gold // 100, "contents": []},
        "outfit": None, "achievement": None,
        "reply": ["Your hunting campaign is complete. You receive 10,000 experience and 5,000 gold."],
        "sources": [
            "OTERYN_AUTHORED_APPROXIMATION: chosen Killing in the Name of... Quest recipe, stages s1-s11; exact currency value normalized by the native coin owner.",
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
