"""Capture the complete fifteen-dish menu and the separate native cook operation."""

import argparse
import hashlib
import json

from build_dialogues import HERE, PIN, REPO
from capture import Capture, inventory
from lower_dialogue import Lowering
from lua_tables import assignments


def capture(donor):
    c = Capture(donor, "hot_cuisine", "quest/u8_5/hot_cuisine_quest")
    npc = "jean_pierre"
    source = c.read(npc)
    lower = Lowering(c, npc, "Storage.Quest.U8_5.HotCuisineQuest")
    lower.lower(190, 320, "menu")
    c.rule(npc, "apprentice-accept", 322, ["yes"], effects=[c.effect("QuestStart", 1), c.effect("CurrentDish", 1), c.effect("QuestLog", 1)], say=323, topics=[3], exclude=["cook", "recipe", "menu", "apprentice"])
    lower.lines[704] = lower.lines[704].replace("elseif", "if", 1)
    before = len(c.rules)
    lower.lower(705, 799, "dish-choice")
    for rule in c.rules[before:]:
        rule["exclude_keywords"] = ["cook", "recipe", "menu", "apprentice", "yes"]
    c.rule(npc, "greet-august", 171, guards=[{"kind": "calendar_month", "value": 8}],
           replies=c.strings(npc, 172, 172), trigger="greet")
    c.rule(npc, "greet-outside-august", 174, effects=[{"kind": "end_conversation", "delay_ms": 1000}], trigger="greet")

    # Runtime code selects a row by the donor confirmation topic; ingredient
    # selection still reads CurrentDish, exactly as playerHasIngredients does.
    ingredients = assignments(source, {"ingredients"})["ingredients"]["fields"]
    success_lines = [330, 353, 376, 400, 424, 448, 472, 497, 521, 545, 573, 599, 625, 650, 675]
    reward_ids = [9079, 9080, 9081, 9082, 9083, 9084, 9085, 9086, 9088, 9087, 11584, 11586, 11587, 11588, 50334]
    reward_lines = [340, 363, 387, 411, 435, 459, 484, 508, 532, 560, 586, 612, 637, 662, 693]
    log_values = [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14, 15, 16, 17]
    recipes = []
    for index, ingredient_row in enumerate(ingredients, 1):
        needs = []
        for entry in ingredient_row["value"]["fields"]:
            values = [field["value"] for field in entry["value"]["fields"]]
            needs.append(inventory("take_item", values[0], values[1], values[2] if len(values) == 3 else None))
        dish_number = index + 1
        last_key = c.track(f"LastInteractionDate{dish_number}") if dish_number < 16 else c.track("Kv.lastinteractiondate16", line=673, npc=npc)
        count_key = c.track(f"Kv.cookcount-{dish_number}", line=123, npc=npc)
        bonus_key = c.track(f"Kv.bonusused-{dish_number}", maximum=1, line=115, npc=npc)
        for key in (count_key, bonus_key):
            c.tracks[key]["initial"] = 0
            c.tracks[key]["min"] = 0
        c.tracks[last_key]["max"] = 9223372036854775807
        recipes.append({
            "index": index, "dish_number": dish_number, "topic": 2 * index + 3,
            "ingredients": needs, "reward": inventory("give_item", reward_ids[index - 1]),
            "last_track": last_key, "cook_count_track": count_key, "bonus_used_track": bonus_key,
            "next_dish": min(index + 1, 15), "quest_log": log_values[index - 1],
            "opens_cookbook_door": index == 10, "completes_menu": index == 15,
            "reply": c.say(npc, success_lines[index - 1]),
            "sources": [c.ref(npc, ingredient_row["line"]), c.ref(npc, success_lines[index - 1]), c.ref(npc, reward_lines[index - 1])],
        })
    bonus_year = c.track("Kv.bonusyear", line=686, npc=npc)
    c.track("CookbookDoor", maximum=1)
    # Coordinator ANSWER #6589517205 point 10: donor os.date is UTC.
    native = {"schema": 1, "quest": c.key, "calendar": "UTC", "recipes": recipes,
              "quest_start_track": c.track("QuestStart"), "current_dish_track": c.track("CurrentDish"),
              "quest_log_track": c.track("QuestLog"), "cookbook_door_track": c.track("CookbookDoor"),
              "bonus_year_track": bonus_year,
              "missing_ingredients_reply": c.say(npc, 343), "already_cooked_reply": c.say(npc, 347),
              "sources": [c.ref(npc, 66), c.ref(npc, 107), c.ref(npc, 139), c.ref(npc, 672),
                          "https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589517205"]}
    native["books"] = []
    for number, source_id in [(1, 9093), (2, 11541)]:
        path = f"data-global/scripts/quests/hot_cuisine/actions_cookbook{number}.lua"
        data = (c.donor / path).read_bytes()
        rows = assignments(data.decode(), {"config"})["config"]["fields"]
        native["books"].append({"item": f"oteryn:item.tibia.i{source_id}", "text": "".join(row["value"] for row in rows),
                                "sha256": hashlib.sha256(data).hexdigest(), "sources": [f"{PIN}{path}:1"]})
    for path in [HERE / "native" / "hot_cuisine.json", REPO / "content/quests/hot-cuisine-batch3.json"]:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(native, ensure_ascii=False, indent=2) + "\n")
    c.save()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("donor")
    capture(parser.parse_args().donor)
