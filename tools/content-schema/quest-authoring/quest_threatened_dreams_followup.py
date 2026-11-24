"""Update the chosen Threatened Dreams projection from Crystal 201189f8."""

from quest_completion_authoring import validate_journey

QUEST = "oteryn:quest.threatened_dreams_quest"
TITLE = "Threatened Dreams Quest"
MAELYRRA = "https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/maelyrra.lua#L84-L161"
BASIS = "CHOSEN_OTERYN_APPROXIMATION"


def apply(records):
    """Keep the historical Source record and the existing chosen stage identities."""
    record, = (row["definition"] for row in records if row["definition"]["identity"]["key"] == QUEST)
    chosen = record["oteryn_recipe"]
    recipe = chosen["payload"]["recipe"]
    if chosen["runtime_enabled"] is not False or recipe["wiki_title"] != TITLE:
        raise ValueError("Threatened Dreams baseline recipe differs")
    start, book, hunt, boss, rescue, report, finish = recipe["stages"]
    if (hunt["key"], hunt["kind"], hunt["count"]) != ("s3", "kill", 20):
        raise ValueError("Threatened Dreams baseline hunt differs")

    start["objective"] = "Accept Alkestios' request for help against the poachers."
    book["objective"] = (
        "Recover the book, pay Ahmet 5000 gold to amend it, and use it on the poachers' table. "
        "Help the wolf and swan to receive Alkestios' seal for Feyrist access."
    )
    hunt["count"] = 200
    hunt["objective"] = "Defeat two hundred Enfeebled Silencers or Weakened Frazzlemaws in total for Maelyrra."
    boss["objective"] = "Enter Kroazur's lair, defeat him and report the hunt to Maelyrra."
    rescue["objective"] = "Recover the Dark Moon Mirror from the stump and release all five captive fairies."
    report["objective"] = (
        "Report the rescue and keep the mirror. Help Aurita and Taegen, then obtain the vial and "
        "sun catcher. Gather moonlight and starlight at night and sunlight by day."
    )
    finish["objective"] = (
        "Show Maelyrra the three charged vessels, infuse five sculptures, five trees and five mosaics, "
        "then report the restored barrier to receive a Blossom Bag."
    )
    recipe["summary"] = "Earn access to Feyrist and restore its barrier in Maelyrra's service."
    recipe["reward_intents"] = [
        {"kind": "access", "name": "Feyrist", "count": 1, "basis": BASIS},
        {"kind": "item", "name": "Blossom Bag", "count": 1, "basis": BASIS},
    ]
    recipe["adaptations"] = [
        "Select the Crystal main journey through the restored barrier. The rescued mirror is kept "
        "for the light missions. Maelyrra awards the Blossom Bag after all fifteen targets.",
    ]
    recipe["source_notes"].append(MAELYRRA)
    validate_journey(recipe)


def align_historical_associations(event, recipe):
    """Carry the new chosen count and reward alongside the pinned identity evidence."""
    event = dict(event)
    event["stages"] = [
        dict(stage, count=200) if stage["stage_key"] == "s3" else stage
        for stage in event["stages"]
    ]
    bag, = (reward for reward in recipe["reward_intents"] if reward["name"] == "Blossom Bag")
    _, mirror = event["reward_intents"]
    if mirror["mapping"]["target"] != "Moon Mirror" or bag["item"]["key"] != "oteryn:item.tibia.i25780":
        raise ValueError("Threatened Dreams reward identity differs")
    event["reward_intents"] = [
        event["reward_intents"][0],
        {
            "reward_index": 1, "kind": "item", "count": bag["count"], "basis": bag["basis"],
            "mapping": {"target": bag["name"], "status": "CHOSEN_CANONICAL_ITEM", "refs": [bag["item"]]},
            "unresolved": ["NATIVE_REWARD_DELIVERY_CONSUMER_BINDING_PENDING"],
            "runtime_admitted": False,
        },
    ]
    return event
