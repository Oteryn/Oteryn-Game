"""Apply the bounded Soul War faithful-reimplementation recipe followup.

This changes only the chosen Oteryn recipe projection. Original SOURCE data, holds,
Quest identity and native non-admission remain unchanged.
"""
from __future__ import annotations

import copy
import hashlib
import json

from jsonschema import Draft202012Validator

PATH = "tools/content-schema/quest-authoring/samples/soul-war-reconstruction/recipe-followup.json"
SHA256 = "c5a144bbe284020ff3e5c1e0722bfa582df1a9d663edbfbe6eb567de65a8d0a3"
RECONSTRUCTION_SCHEMA = "tools/content-schema/quest-authoring/soul_war_reconstruction.schema.json"


def _canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def digest(value):
    return hashlib.sha256(_canonical(value).encode()).hexdigest()


def _read_checked(root, path, expected_sha256):
    raw = (root / path).read_bytes()
    if hashlib.sha256(raw).hexdigest() != expected_sha256:
        raise ValueError(f"Soul War input differs: {path}")
    return json.loads(raw)


def _validate_packet(packet):
    if packet.get("schema") != "OTERYN_SOUL_WAR_RECIPE_FOLLOWUP/v1":
        raise ValueError("Soul War followup schema differs")
    if packet.get("canonical_key") != "oteryn:quest.soul_war_quest":
        raise ValueError("Soul War followup owner differs")
    if packet.get("runtime_enabled") is not False or packet.get("source_holds_preserved") is not True:
        raise ValueError("Soul War followup scope differs")
    replacement = packet["replacement"]
    stages = replacement["stages"]
    if [stage["key"] for stage in stages] != ["s1", "s2", "s3", "s4", "s5"]:
        raise ValueError("Soul War followup stage keys differ")
    if stages[1]["kind"] != "kill" or stages[1]["count"] != 5:
        raise ValueError("Soul War five-boss projection differs")
    expected_bosses = {
        "Goshnar's Malice",
        "Goshnar's Greed",
        "Goshnar's Spite",
        "Goshnar's Cruelty",
        "Goshnar's Hatred",
    }
    if set(stages[1]["targets"]) != expected_bosses:
        raise ValueError("Soul War mini-boss set differs")
    rewards = replacement["reward_intents"]
    if len(rewards) != 1 or rewards[0]["kind"] != "item" or rewards[0]["count"] != 1:
        raise ValueError("Soul War final reward projection differs")
    pool = packet["reward_pool"]
    if len(pool) != 18 or {row["id"] for row in pool} != set(range(34082, 34100)):
        raise ValueError("Soul War final reward pool ids differ")
    if len({row["name"] for row in pool}) != len(pool):
        raise ValueError("Soul War final reward pool names differ")


def apply(root, records):
    raw = (root / PATH).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SHA256:
        raise ValueError("Soul War followup packet differs")
    packet = json.loads(raw)
    _validate_packet(packet)

    reconstruction_ref = packet["reconstruction"]
    reconstruction = _read_checked(root, reconstruction_ref["path"], reconstruction_ref["sha256"])
    schema = _read_checked(root, reconstruction_ref["schema_path"], reconstruction_ref["schema_sha256"])
    Draft202012Validator(schema).validate(reconstruction)
    if reconstruction["runtime_activation"] is not False:
        raise ValueError("Soul War reconstruction unexpectedly activates runtime")
    if reconstruction["quest"]["key"] != packet["canonical_key"]:
        raise ValueError("Soul War reconstruction owner differs")
    if reconstruction["source_files"] != packet["donor_source_refs"]:
        raise ValueError("Soul War donor references differ from reconstruction")

    result = copy.deepcopy(records)
    selected = [
        row for row in result
        if row["definition"]["identity"]["key"] == packet["canonical_key"]
    ]
    if len(selected) != 1:
        raise ValueError("Soul War canonical definition cardinality differs")
    definition = selected[0]["definition"]
    supplement = definition.get("oteryn_recipe")
    if not supplement or supplement.get("runtime_enabled") is not False:
        raise ValueError("Soul War chosen recipe is missing or runtime-enabled")
    if definition.get("native_lowering", {}).get("state") != "WAITING_IMPLEMENTATION":
        raise ValueError("Soul War native-lowering hold differs")

    payload = supplement["payload"]
    recipe = payload["recipe"]
    if digest(recipe) != packet["baseline_recipe_sha256"]:
        raise ValueError("Soul War followup recipe fence differs")
    if payload["donor_source_refs"]:
        raise ValueError("Soul War baseline already has donor refs; reconcile instead of replacing")

    replacement = packet["replacement"]
    recipe["summary"] = replacement["summary"]
    recipe["stages"] = copy.deepcopy(replacement["stages"])
    recipe["reward_intents"] = copy.deepcopy(replacement["reward_intents"])
    recipe["adaptations"] = copy.deepcopy(replacement["adaptations"])
    if replacement["source_note"] not in recipe["source_notes"]:
        recipe["source_notes"].append(replacement["source_note"])
    payload["donor_source_refs"] = copy.deepcopy(packet["donor_source_refs"])
    payload["title_stage_keys"] = {
        "Soul War Quest": [stage["key"] for stage in recipe["stages"]]
    }

    from quest_completion_authoring import validate_journey
    validate_journey(recipe)
    return result, {"path": PATH, "sha256": SHA256}
