"""Capture Tereban's complete Father quest graph, including absent storage -1.

The eight temporary donor targetMessage values are encoded as session topics
7201..7208. No new persistent progress is introduced for that conversation state.
"""

import argparse

from capture import Capture, inventory, topic
from lua_tables import assignments


def fields(table):
    return {field["key"]: field["value"] for field in table["fields"]}


def messages(value):
    if isinstance(value, str):
        return [value]
    return [field["value"] for field in value["fields"]]


def capture(donor):
    c = Capture(donor, "fathers_burden", "quest/u8_6/afathers_burden")
    npc = "tereban_functions"
    source = c.read(npc)
    config = assignments(source, {"TerebanConfig"})["TerebanConfig"]
    g, e, r = c.guard, c.effect, c.rule
    r(npc, "mission-pending", 132, ["mission"], [g("Status", "eq", 1), g("Progress", "ne", 8)], say=135, topics=[0], exclude=["cloak"])
    r(npc, "mission-complete", 139, ["mission"], [g("Status", "eq", 1), g("Progress", "eq", 8)], [e("Status", 2), inventory("give_item", 11701), {"kind": "experience", "amount": 8000}], say=142, topics=[0], exclude=["cloak"])
    r(npc, "mission-already-complete", 146, ["mission"], [g("Status", "eq", 2)], say=147, topics=[0], exclude=["cloak"])
    r(npc, "mission-offer", 150, ["mission"], [g("Status", "ne", 1), g("Status", "ne", 2)], [topic(1)], say=150, topics=[0], exclude=["cloak"])

    material_tracks = []
    for index, entry in enumerate(config["fields"], 1):
        keyword = entry["key"]
        values = fields(entry["value"])
        name = values["storage"]["expr"].rsplit(".", 1)[1]
        material_tracks.append(name)
        reply = fields(values["messages"])
        label = keyword.replace(" ", "-")
        pending = 7200 + index
        r(npc, f"{label}-done", 163, [keyword], [g(name, "eq", 2)], replies=messages(reply["done"]), topics=[0], exclude=["cloak", "mission"], keyword_mode="exact")
        r(npc, f"{label}-offer", 168, [keyword], [g(name, "ne", 2)], [topic(pending)], replies=messages(reply["deliever"]), topics=[0], exclude=["cloak", "mission"], keyword_mode="exact")
        r(npc, f"{label}-deliver", 187, ["yes"], [inventory("item", values["itemId"])], [inventory("take_item", values["itemId"]), e(name, 2), e("Progress", 1, "add"), {"kind": "experience", "amount": 2500}, topic(0)], replies=messages(reply["success"]), topics=[pending])
        # The donor returns before its trailing topic reset when removeItem fails.
        r(npc, f"{label}-missing", 187, ["yes"], replies=messages(reply["failure"]), topics=[pending])
        r(npc, f"{label}-no", 197, ["no"], effects=[topic(0)], replies=messages(reply["no"]), topics=[pending], exclude=["yes"])
        r(npc, f"{label}-other", 200, effects=[topic(0)], topics=[pending], exclude=["yes", "no"], any_message=True)
        for rule in c.rules[-6:]:
            rule["sources"].append(c.ref(npc, entry["line"]))
    # These eleven writes must be committed as one transition. Initial flags are
    # deliberately -1, not pre-set to 1 to fit the older eight-effect limit.
    accept = [e("QuestLog", 1), e("Progress", 0), e("Status", 1)]
    accept.extend(e(name, 1) for name in material_tracks)
    accept.append(topic(0))
    r(npc, "mission-accept", 173, ["yes"], effects=accept, say=174, topics=[1])
    r(npc, "mission-decline", 182, ["no"], effects=[topic(0)], say=182, topics=[1], exclude=["yes"])
    r(npc, "mission-other", 184, effects=[topic(0)], topics=[1], exclude=["yes", "no"], any_message=True)
    for rule in c.rules:
        rule["npc"] = "oteryn:npc.tereban"
    c.save()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("donor")
    capture(parser.parse_args().donor)
