"""Project chosen source-derived Quest recipes into an unactivated QuestState candidate.

Chosen Oteryn approximation only: no donor equivalence and no runtime activation.
Existing Source owners may receive additive chosen progress only when Source completion
is explicitly NOT_LOWERED; Source tracks/transitions are preserved unchanged.
"""
import argparse
import copy
import hashlib
import json
import pathlib
import re
import sys

TOOL_DIR = pathlib.Path(__file__).resolve().parents[3]
if str(TOOL_DIR) not in sys.path:
    sys.path.insert(0, str(TOOL_DIR))
import quest_terminal_stage_refinements as terminal_refinements

BASIS = "CHOSEN_OTERYN_APPROXIMATION"
PROFILE = "chosen_source_completion_v1"
SOURCE_STATE = "content/quests/missions/quest-state.json"
NEW_STATE = "CHOSEN_SOURCE_TYPED_PROGRESS_ONLY"
OVERLAY_STATE = "SOURCE_PLUS_CHOSEN_TYPED_PROGRESS_ONLY"
OVERLAYABLE_SOURCE_STATES = {"NOT_LOWERED_MULTI_TRACK", "NOT_LOWERED_NO_MISSIONS"}
ALLOWED = {"talk", "kill", "use", "collect", "explore", "complete"}


def enc(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def digest(value):
    return hashlib.sha256(enc(value).encode()).hexdigest()


def req(ok, message):
    if not ok:
        raise ValueError(message)


def valid(key):
    return (
        isinstance(key, str)
        and len(key.encode()) <= 128
        and bool(re.fullmatch(r"oteryn:[A-Za-z0-9._:/-]+", key))
    )


def completion_state(quest):
    value = quest["completion"]
    return value if isinstance(value, str) else value["state"]


def recipe_of(root, quest):
    wrapper = quest.get("oteryn_recipe") or {}
    if wrapper.get("profile") != PROFILE or not wrapper.get("chosen_data_complete"):
        return None, None
    payload = wrapper.get("payload") or {}
    normalized, provenance = terminal_refinements.normalize_payload(root, payload)
    recipe = normalized.get("recipe")
    return (recipe if isinstance(recipe, dict) else None), provenance


def validate_recipe(quest, recipe):
    req(valid(quest["identity"]["key"]), "Invalid Quest owner")
    stages = recipe.get("stages") or []
    req(stages, "Empty chosen source recipe")
    ids = [stage.get("key") for stage in stages]
    req(len(ids) == len(set(ids)), "Duplicate stage keys")
    for index, stage in enumerate(stages):
        req(bool(re.fullmatch(r"s[1-9][0-9]*", str(stage.get("key", "")))), "Stage key")
        req(
            type(stage.get("count")) is int and 1 <= stage["count"] <= 2**63 - 1,
            "Stage count",
        )
        req(
            stage.get("next") == ([ids[index + 1]] if index + 1 < len(ids) else []),
            "Unsupported graph/order",
        )
        req(stage.get("kind") in ALLOWED, "Unknown event intent")
        req(
            (stage.get("kind") == "complete") == (index == len(stages) - 1),
            "Completion only terminal",
        )
    req(stages[-1]["count"] == 1, "Terminal completion count must be one")
    req((recipe.get("repeat") or {}).get("kind") in {"once", "daily"}, "Unsupported repeat")


def project(root, quest, canonical_ref, recipe, normalization, state=NEW_STATE, source_completion_state=None):
    req(recipe is not None, "Not a chosen source recipe")
    validate_recipe(quest, recipe)
    owner = quest["identity"]["key"]
    tail = owner.removeprefix("oteryn:quest.")
    req(tail != owner, "Chosen source namespace")
    tracks = []
    transitions = []
    for index, stage in enumerate(recipe["stages"]):
        track_key = f"oteryn:quest-progress/chosen-source/{tail}/{stage['key']}"
        transition_key = f"oteryn:quest-transition/chosen-source/{tail}/{stage['key']}"
        req(valid(track_key) and valid(transition_key), "Track/transition key budget")
        tracks.append(
            {
                "key": track_key,
                "quest": owner,
                "initial": 0,
                "min": 0,
                "max": stage["count"],
                "bounds_basis": "EXPLICIT_CHOSEN_STAGE_OCCURRENCE_COUNT",
                "source_key": f"oteryn:chosen-source-stage/{tail}/{stage['key']}",
            }
        )
        effects = []
        if index:
            previous = tracks[-2]
            effects.append(
                {
                    "track": previous["key"],
                    "from": {"op": "EQ", "value": previous["max"]},
                    "from_exact": True,
                    "effect": {"kind": "SET", "value": previous["max"]},
                }
            )
        effects.append(
            {
                "track": track_key,
                "from": {"op": "LT", "value": stage["count"]},
                "from_exact": True,
                "effect": {"kind": "ADD", "value": 1},
            }
        )
        transitions.append(
            {
                "key": transition_key,
                "quest": owner,
                "completes": index == len(recipe["stages"]) - 1,
                "effects": effects,
                "requested_by": None,
                "source": {
                    "basis": BASIS,
                    "chosen_stage": copy.deepcopy(stage),
                    "canonical_ref": dict(
                        canonical_ref, definition_sha256=digest(quest)
                    ),
                    "native_dispatch_binding": None,
                    "event_intent_only": True,
                    "runtime_admission": False,
                    "source_equivalence": False,
                },
            }
        )
    repeat = recipe["repeat"]["kind"]
    completion = {
        "basis": BASIS,
        "state": state,
        "native_admission": False,
        "runtime_enabled": False,
        "source_equivalence": False,
        "event_dispatch_binding": None,
        "NPC_dialogue_binding": None,
        "reward_delivery_binding": None,
        "repeat_lowering": (
            "FIRST_CYCLE_ONLY"
            if repeat == "once"
            else "HELD_NO_CYCLE_RESET_BINDING"
        ),
        "canonical_ref": dict(canonical_ref, definition_sha256=digest(quest)),
        "recipe_metadata": copy.deepcopy(
            {key: value for key, value in recipe.items() if key != "stages"}
        ),
        "counter_assumption": (
            "Explicit chosen occurrence counters only; native dispatch/dialogue/reward "
            "bindings remain unresolved."
        ),
    }
    if normalization is not None:
        completion["terminal_stage_normalization"] = copy.deepcopy(normalization)
    if source_completion_state is not None:
        completion.update(
            {
                "source_completion_state": source_completion_state,
                "source_progress_preserved": True,
            }
        )
    return {
        "quest": owner,
        "source_quest": owner,
        "completion": completion,
        "tracks": tracks,
        "transitions": transitions,
    }


def build(root):
    root = pathlib.Path(root)
    source = json.loads((root / SOURCE_STATE).read_text(encoding="utf-8"))
    source_by_owner = {quest["quest"]: quest for quest in source["quests"]}
    quests = []
    overlays = []
    skipped_source_lowered = []
    authoring_sources = []
    held = []

    for path in sorted((root / "content/quests/definitions").glob("quests-*.json")):
        raw = path.read_bytes()
        packet_sha = hashlib.sha256(raw).hexdigest()
        relative = path.relative_to(root).as_posix()
        authoring_sources.append({"path": relative, "sha256": packet_sha})
        for index, row in enumerate(json.loads(raw)["records"]):
            quest = row["definition"]
            recipe, normalization = recipe_of(root, quest)
            if recipe is None:
                continue
            ref = {
                "path": relative,
                "packet_sha256": packet_sha,
                "json_pointer": f"/records/{index}/definition",
            }
            owner = quest["identity"]["key"]
            existing = source_by_owner.get(owner)
            source_state = completion_state(existing) if existing is not None else None
            try:
                if existing is None:
                    quests.append(
                        project(root, quest, ref, recipe, normalization)
                    )
                elif source_state == "LOWERED":
                    validate_recipe(quest, recipe)
                    skipped_source_lowered.append(
                        {"quest": owner, "source_completion_state": source_state, "canonical_ref": ref}
                    )
                elif source_state in OVERLAYABLE_SOURCE_STATES:
                    overlays.append(
                        project(
                            root,
                            quest,
                            ref,
                            recipe,
                            normalization,
                            state=OVERLAY_STATE,
                            source_completion_state=source_state,
                        )
                    )
                else:
                    raise ValueError("Unexpected existing Source completion state")
            except ValueError as error:
                held.append(
                    {
                        "quest": owner,
                        "reason": str(error),
                        "source_completion_state": source_state,
                        "canonical_ref": ref,
                    }
                )

    quests.sort(key=lambda quest: quest["quest"])
    overlays.sort(key=lambda quest: quest["quest"])
    skipped_source_lowered.sort(key=lambda row: row["quest"])
    held.sort(key=lambda row: row["quest"])

    req(len(quests) == 146, "Qualified new chosen source recipe count changed")
    req(len(overlays) == 90, "Qualified chosen source overlay count changed")
    req(
        len(skipped_source_lowered) == 6
        and all(row["source_completion_state"] == "LOWERED" for row in skipped_source_lowered),
        "Source-lowered skip set changed",
    )
    req(len(held) == 0, "Chosen source hold set changed")
    projected = quests + overlays
    return {
        "schema": "OTERYN_CHOSEN_SOURCE_QUEST_PROGRESS_IMPORT/v3",
        "basis": BASIS,
        "native_admission": False,
        "runtime_enabled": False,
        "authoring_sources": authoring_sources,
        "summary": {
            "quests": len(projected),
            "new_quests": len(quests),
            "overlay_quests": len(overlays),
            "source_lowered_skipped": len(skipped_source_lowered),
            "tracks": sum(len(quest["tracks"]) for quest in projected),
            "transitions": sum(len(quest["transitions"]) for quest in projected),
            "completion_transitions": len(projected),
            "repeat_cycles_held": sum(
                quest["completion"]["repeat_lowering"]
                == "HELD_NO_CYCLE_RESET_BINDING"
                for quest in projected
            ),
            "held_quests": len(held),
            "NPC_bindings": 0,
            "event_dispatch_bindings": 0,
            "reward_delivery_bindings": 0,
        },
        "held": held,
        "skipped_source_lowered": skipped_source_lowered,
        "quests": quests,
        "overlays": overlays,
    }


def validate_projected(quest, expected_state):
    req(valid(quest["quest"]), "Quest key")
    completion = quest["completion"]
    req(
        completion["state"] == expected_state
        and completion["runtime_enabled"] is False
        and completion["source_equivalence"] is False,
        "No promotion",
    )
    if expected_state == OVERLAY_STATE:
        req(
            completion.get("source_completion_state") in OVERLAYABLE_SOURCE_STATES
            and completion.get("source_progress_preserved") is True,
            "Overlay Source provenance",
        )
    tracks = {track["key"]: track for track in quest["tracks"]}
    req(len(tracks) == len(quest["tracks"]), "Unique tracks")
    for transition in quest["transitions"]:
        req(
            transition["quest"] == quest["quest"]
            and transition["requested_by"] is None,
            "Owned transition",
        )
        req(
            transition["source"]["native_dispatch_binding"] is None
            and transition["source"]["runtime_admission"] is False
            and transition["source"]["source_equivalence"] is False,
            "No binding promotion",
        )
        req(1 <= len(transition["effects"]) <= 8, "Effect budget")
        for effect in transition["effects"]:
            req(
                effect["track"] in tracks and effect["from_exact"] is True,
                "Closed exact effect",
            )


def validate_packet(packet):
    req(
        packet.get("schema") == "OTERYN_CHOSEN_SOURCE_QUEST_PROGRESS_IMPORT/v3",
        "Packet schema",
    )
    req(
        packet.get("basis") == BASIS
        and packet.get("native_admission") is False
        and packet.get("runtime_enabled") is False,
        "Packet admission",
    )
    quests = packet.get("quests") or []
    overlays = packet.get("overlays") or []
    held = packet.get("held") or []
    skipped = packet.get("skipped_source_lowered") or []
    req(
        len(quests) == 146 and len({quest["quest"] for quest in quests}) == 146,
        "All146 unique new owners",
    )
    req(
        len(overlays) == 90
        and len({quest["quest"] for quest in overlays}) == 90
        and not ({quest["quest"] for quest in quests} & {quest["quest"] for quest in overlays}),
        "All90 unique overlay owners",
    )
    req(
        len(skipped) == 6
        and all(row["source_completion_state"] == "LOWERED" for row in skipped),
        "All6 Source-lowered owners retained",
    )
    req(len(held) == 0, "No terminal-count holds remain")
    for quest in quests:
        validate_projected(quest, NEW_STATE)
    for quest in overlays:
        validate_projected(quest, OVERLAY_STATE)


def merge(source, packet):
    validate_packet(packet)
    result = copy.deepcopy(source)
    by_owner = {quest["quest"]: quest for quest in result["quests"]}
    incoming = {quest["quest"] for quest in packet["quests"]}
    req(not (set(by_owner) & incoming), "Duplicate new owner")

    result["quests"].extend(copy.deepcopy(packet["quests"]))
    by_owner = {quest["quest"]: quest for quest in result["quests"]}
    for overlay in packet["overlays"]:
        target = by_owner.get(overlay["quest"])
        req(target is not None, "Overlay owner missing")
        source_state = completion_state(target)
        req(
            source_state == overlay["completion"]["source_completion_state"],
            "Overlay Source completion drift",
        )
        source_track_keys = {track["key"] for track in target["tracks"]}
        source_transition_keys = {transition["key"] for transition in target["transitions"]}
        overlay_track_keys = {track["key"] for track in overlay["tracks"]}
        overlay_transition_keys = {transition["key"] for transition in overlay["transitions"]}
        req(not (source_track_keys & overlay_track_keys), "Overlay track collision")
        req(
            not (source_transition_keys & overlay_transition_keys),
            "Overlay transition collision",
        )
        target["tracks"].extend(copy.deepcopy(overlay["tracks"]))
        target["transitions"].extend(copy.deepcopy(overlay["transitions"]))
        target["completion"] = copy.deepcopy(overlay["completion"])

    result["quests"] = sorted(result["quests"], key=lambda quest: quest["quest"])
    transitions = [
        transition for quest in result["quests"] for transition in quest["transitions"]
    ]
    effects = [
        effect["effect"]["kind"]
        for transition in transitions
        for effect in transition["effects"]
    ]
    result["counts"] = {
        "quests": len(result["quests"]),
        "tracks": sum(len(quest["tracks"]) for quest in result["quests"]),
        "transitions": len(transitions),
        "effects": {
            kind: effects.count(kind) for kind in sorted(set(effects))
        },
        "inexact_from": sum(
            any(not effect["from_exact"] for effect in transition["effects"])
            for transition in transitions
        ),
        "requested_by": sum(
            transition["requested_by"] is not None for transition in transitions
        ),
        "completes": sum(transition["completes"] for transition in transitions),
        "completion": {},
    }
    for quest in result["quests"]:
        state = completion_state(quest)
        result["counts"]["completion"][state] = (
            result["counts"]["completion"].get(state, 0) + 1
        )
    result["authoring_sources"] = copy.deepcopy(
        source.get("authoring_sources", [])
    ) + copy.deepcopy(packet["authoring_sources"])
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=pathlib.Path, required=True)
    parser.add_argument("--out", type=pathlib.Path, required=True)
    parser.add_argument("--source-state", type=pathlib.Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    packet = build(args.repo_root)
    value = (
        merge(
            json.loads(args.source_state.read_text(encoding="utf-8")),
            packet,
        )
        if args.source_state
        else packet
    )
    encoded = (
        json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    ).encode("utf-8")
    if args.check:
        req(
            args.out.exists() and args.out.read_bytes() == encoded,
            "Generated candidate differs",
        )
    else:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_bytes(encoded)
    print(value.get("summary", value.get("counts")))


if __name__ == "__main__":
    main()
