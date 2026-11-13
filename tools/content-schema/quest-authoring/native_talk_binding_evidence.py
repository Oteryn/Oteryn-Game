"""Offline evidence join for the server's closed native TALK binding registry."""
import hashlib
import json
from pathlib import Path, PurePosixPath
from functools import lru_cache

from jsonschema import Draft202012Validator, ValidationError


def require(condition, message):
    if not condition:
        raise ValueError(message)


@lru_cache(maxsize=1)
def callback_schema():
    schema = json.loads(Path(__file__).with_name("shared-reward-callbacks.schema.json").read_text())
    return Draft202012Validator(schema)


def verify_talk_binding(root, definition, stage_key, reference):
    """Verify an exact registered callback; a runtime flag grants no authority."""
    require(isinstance(reference, dict) and set(reference) in (
            {"kind", "document", "stage", "callback"},
            {"kind", "document", "stage", "callbacks"}), "TALK reference shape")
    require(reference["kind"] == "npc_dialogue" and reference["stage"] == stage_key,
            "TALK reference stage or kind")
    require(isinstance(reference["document"], str), "TALK document path type")
    path = PurePosixPath(reference["document"])
    require(len(path.parts) == 4 and path.parts[:3] ==
            ("content", "quests", "authored-bindings") and path.suffix == ".json"
            and all(c.isascii() and (c.islower() or c.isdigit() or c in "-_.")
                    for c in path.name), "TALK document is outside the compiled registry")
    file = root / str(path)
    require(file.is_file() and not file.is_symlink(), "TALK registry document is missing")
    document = json.loads(file.read_text(encoding="utf-8"))
    require(document.get("schema") == 1, "TALK registry schema")
    owner = definition["identity"]["key"]
    selected = definition.get("oteryn_recipe", {})
    if definition.get("definition_profile") == "oteryn_authored_v1":
        recipe = definition.get("recipe")
        complete = definition.get("data_complete")
    else:
        require(selected.get("profile") == "chosen_source_completion_v1",
                "TALK recipe profile")
        payload = selected.get("payload", {})
        require(payload.get("identity") == definition["identity"], "TALK selected recipe owner")
        recipe = payload.get("recipe")
        complete = selected.get("chosen_data_complete")
    require(complete is True and isinstance(recipe, dict), "TALK recipe is incomplete")
    allowed_holds = {"quest_native_lowering_missing", "authored_trigger_and_delivery_bindings_missing"}
    if definition.get("definition_profile") != "oteryn_authored_v1":
        allowed_holds.add("reported_source_gap")
    missing = definition.get("missing_data")
    require(isinstance(missing, list) and all(isinstance(m, dict) and m.get("code") in allowed_holds
                                            for m in missing), "TALK recipe has unresolved DATA holds")
    digest = hashlib.sha256(json.dumps(recipe, sort_keys=True, ensure_ascii=False,
                                      separators=(",", ":")).encode()).hexdigest()
    quests = [q for q in document.get("quests", []) if q.get("quest") == owner]
    require(len(quests) == 1 and quests[0].get("recipe_sha256") == digest,
            "TALK registry owner or recipe digest")
    chosen_stages = [s for s in recipe.get("stages", []) if s.get("key") == stage_key]
    require(len(chosen_stages) == 1 and chosen_stages[0].get("kind") == "talk",
            "TALK stage is absent from the selected recipe")
    stages = [s for s in quests[0].get("stages", []) if s.get("key") == stage_key]
    require(len(stages) == 1, "TALK registry stage is missing or duplicated")
    require(stages[0].get("completes") is False, "TALK stage cannot certify whole-quest completion")
    trigger = stages[0].get("trigger", {})
    callbacks = [trigger.get("callback", {})] + trigger.get("alternatives", [])
    expected = reference.get("callbacks", [reference.get("callback")])
    require(isinstance(expected, list) and 1 <= len(expected) <= 32
            and all(isinstance(key, str) and key for key in expected)
            and len(set(expected)) == len(expected), "TALK callback group reference")
    require(trigger.get("kind") == "npc_dialogue" and len(callbacks) == len(expected)
            and sorted(callback.get("key", "") for callback in callbacks) == sorted(expected),
            "TALK registry callback group differs")
    declared = quests[0].get("tracks", [])
    require(isinstance(declared, list) and declared, "TALK binding has no track declarations")
    tracks = {track["key"]: track for track in declared}
    require(len(tracks) == len(declared), "TALK track declarations are duplicated")
    for callback in callbacks:
        try:
            callback_schema().validate({"schema": 1, "callbacks": [callback]})
        except ValidationError as error:
            raise ValueError(f"TALK callback schema: {error.message}") from error
        for storage in callback["storage"]:
            track = tracks.get(storage["track"])
            require(track is not None and track["minimum"] <= storage["minimum"] <= storage["maximum"] <= track["maximum"]
                    and track["minimum"] <= storage["set"] <= track["maximum"], "TALK storage is foreign or unbounded")
        for operation in callback.get("state_ops", []):
            require(operation["track"] in tracks, "TALK state operation is foreign")
        require(callback.get("quest") == owner and callback.get("carried_item") is None,
                "TALK registry callback owner or trigger differs")
        require(isinstance(callback.get("npc"), str) and callback["npc"].startswith("oteryn:npc.")
                and callback.get("keywords") and callback.get("storage"),
                "TALK callback has no exact NPC, dialogue or state binding")
    return callbacks


def registered_talk_bindings(root):
    """Use the same immediate JSON registry as the Rust build-time consumer."""
    result = {}
    owners = set()
    for path in sorted((root / "content/quests/authored-bindings").glob("*.json")):
        require(path.is_file() and not path.is_symlink(), "Native binding registry file type")
        document = json.loads(path.read_text(encoding="utf-8"))
        require(document.get("schema") == 1, "Native binding registry schema")
        for quest in document["quests"]:
            require(quest["quest"] not in owners, "Duplicate native binding quest owner")
            owners.add(quest["quest"])
            for stage in quest["stages"]:
                trigger = stage["trigger"]
                if trigger["kind"] != "npc_dialogue":
                    continue
                key = (quest["quest"], stage["key"])
                require(key not in result, "Duplicate registered TALK stage")
                callbacks = [trigger["callback"]["key"]] + [c["key"] for c in trigger.get("alternatives", [])]
                reference = {"kind": "npc_dialogue", "document": path.relative_to(root).as_posix(),
                             "stage": stage["key"]}
                reference["callback" if len(callbacks) == 1 else "callbacks"] = callbacks[0] if len(callbacks) == 1 else callbacks
                result[key] = reference
    return result


def canonical_definitions(root):
    index = json.loads((root / "content/quests/definitions/index.json").read_text())
    return {row["definition"]["identity"]["key"]: row["definition"]
            for path in index["shards"]
            for row in json.loads((root / path).read_text())["records"]}


def apply_native_talk_bindings(root, plan, definitions):
    stages = {(record["quest"], stage["stage_key"]): stage
              for record in plan["records"] for stage in record["stages"]}
    paths = set()
    for (quest, key), reference in registered_talk_bindings(root).items():
        stage = stages.get((quest, key))
        require(stage is not None and quest in definitions, "Registered TALK stage has no canonical plan owner")
        if stage["event_identity_associations"]["kind"] != "talk":
            continue  # Completion/other trigger consumers retain their own policy.
        verify_talk_binding(root, definitions[quest], key, reference)
        stage["native_dispatch_binding"] = reference
        stage["runtime_enabled"] = True
        paths.add(reference["document"])
    return sorted(paths)
