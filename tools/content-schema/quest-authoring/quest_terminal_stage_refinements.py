"""Normalize the nine chosen recipes whose terminal count represents pre-completion work.

This overlay is candidate-only. It preserves the immutable completion242 packet,
existing recipe refinements, SOURCE holds, identities and reward intents.
"""
import copy
import hashlib
import json
from pathlib import Path

DIRECTORY = "tools/content-schema/quest-authoring/samples/terminal-normalization242/"
PACKET = DIRECTORY + "normalizations.json"
APPROVED_SHA256 = "234fa874bf5652339147e5b224c3bb0da2fb992afefebb834b0c3e28d69d82d0"


def digest(value):
    raw = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(raw.encode()).hexdigest()


def stage_by_key(recipe, key):
    matches = [stage for stage in recipe["stages"] if stage["key"] == key]
    if len(matches) != 1:
        raise ValueError("terminal normalization stage key differs")
    return matches[0]


def apply_entry(row, entry):
    if digest(row) != entry["before_sha256"]:
        raise ValueError("terminal normalization input differs: " + entry["canonical_key"])
    result = copy.deepcopy(row)
    recipe = result["recipe"]
    terminal = recipe["stages"][-1]
    if terminal != entry["old_terminal"]:
        raise ValueError("terminal normalization old terminal differs")
    for override in entry.get("stage_overrides", []):
        stage = stage_by_key(recipe, override["stage_key"])
        if stage[override["field"]] != override["old_value"]:
            raise ValueError("terminal normalization override fence differs")
        stage[override["field"]] = copy.deepcopy(override["new_value"])
    terminal = recipe["stages"][-1]
    terminal["kind"] = entry["terminal_action_kind"]
    terminal["count"] = entry["terminal_action_count"]
    terminal["next"] = [entry["completion_stage"]["key"]]
    recipe["stages"].append(copy.deepcopy(entry["completion_stage"]))
    for title, route in result["title_stage_keys"].items():
        if not route or route[-1] != entry["old_terminal"]["key"]:
            raise ValueError("terminal normalization route does not end at old terminal")
        route.append(entry["completion_stage"]["key"])
    from quest_completion_authoring import validate_journey
    validate_journey(recipe)
    if digest(result) != entry["after_sha256"]:
        raise ValueError("terminal normalization output differs: " + entry["canonical_key"])
    return result


def load_packet(root):
    root = Path(root)
    path = root / PACKET
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != APPROVED_SHA256:
        raise ValueError("unapproved terminal normalization packet")
    packet = json.loads(raw)
    if (
        packet["schema"] != "OTERYN_QUEST_TERMINAL_NORMALIZATION/v1"
        or packet["runtime_enabled"] is not False
        or packet["source_holds_preserved"] is not True
        or len(packet["records"]) != 9
    ):
        raise ValueError("terminal normalization packet contract differs")
    return packet


def normalize_payload(root, row):
    packet = load_packet(root)
    key = row["identity"]["key"]
    entry = next((entry for entry in packet["records"] if entry["canonical_key"] == key), None)
    if entry is None:
        return copy.deepcopy(row), None
    normalized = apply_entry(row, entry)
    return normalized, {
        "path": PACKET,
        "sha256": APPROVED_SHA256,
        "canonical_key": key,
        "runtime_enabled": False,
        "source_holds_preserved": True,
    }


def apply_terminal_normalizations(root, rows):
    packet = load_packet(root)
    result = copy.deepcopy(rows)
    indexed = {row["identity"]["key"]: row for row in result}
    if len(indexed) != len(result):
        raise ValueError("duplicate chosen recipe identity")
    seen = set()
    for entry in packet["records"]:
        key = entry["canonical_key"]
        if key in seen or key not in indexed:
            raise ValueError("unknown or duplicate terminal normalization target")
        seen.add(key)
        updated = apply_entry(indexed[key], entry)
        index = next(i for i, row in enumerate(result) if row["identity"]["key"] == key)
        result[index] = updated
        indexed[key] = updated
    if seen != set(packet["canonical_keys"]):
        raise ValueError("terminal normalization key set differs")
    return result, {
        "path": PACKET,
        "sha256": APPROVED_SHA256,
        "runtime_enabled": False,
        "source_holds_preserved": True,
        "normalized_records": len(seen),
        "canonical_keys": list(packet["canonical_keys"]),
    }
