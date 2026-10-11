"""Apply the reviewed Soul War reconstruction to the chosen Quest projection only.

Original SOURCE evidence and runtime holds stay intact. This module changes only the
chosen Oteryn recipe projection and does not enable Native quest execution.
"""
import copy
import hashlib
import json

PATH = "tools/content-schema/quest-authoring/samples/soul-war-reconstruction/reconstruction.json"
PACKET_SHA256 = "__SET_AFTER_READBACK__"
KEY = "oteryn:quest.soul_war_quest"


def digest(value):
    raw = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(raw.encode("utf-8")).hexdigest()


def apply(root, records):
    raw = (root / PATH).read_bytes()
    if PACKET_SHA256 != "__SET_AFTER_READBACK__" and hashlib.sha256(raw).hexdigest() != PACKET_SHA256:
        raise ValueError("Soul War reconstruction packet differs")
    packet = json.loads(raw)
    if (
        packet["schema"] != "OTERYN_SOUL_WAR_RECONSTRUCTION/v1"
        or packet["canonical_quest"]["key"] != KEY
        or packet["source_holds_preserved"] is not True
        or packet["runtime_enabled"] is not False
    ):
        raise ValueError("Soul War reconstruction scope differs")

    result = copy.deepcopy(records)
    selected = [row for row in result if row["definition"]["identity"]["key"] == KEY]
    if len(selected) != 1:
        raise ValueError("Soul War canonical Quest identity differs")

    definition = selected[0]["definition"]
    supplement = definition.get("oteryn_recipe")
    if not supplement or supplement.get("runtime_enabled") is not False:
        raise ValueError("Soul War chosen recipe supplement missing or unexpectedly runtime-enabled")
    payload = supplement["payload"]
    if digest(payload["recipe"]) != packet["baseline_recipe_sha256"]:
        raise ValueError("Soul War chosen recipe baseline differs")

    recipe = copy.deepcopy(packet["recipe"])
    recipe["source_refs"] = copy.deepcopy(payload["recipe"]["source_refs"])
    from quest_completion_authoring import validate_journey
    validate_journey(recipe)
    payload["recipe"] = recipe
    payload["title_stage_keys"] = copy.deepcopy(packet["title_stage_keys"])
    return result, {"path": PATH, "sha256": hashlib.sha256(raw).hexdigest()}
