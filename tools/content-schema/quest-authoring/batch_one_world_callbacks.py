"""Validate private batch-one inputs; register them only at final integration.

Schema: crates/gameplay/quests/src/quest/world_callbacks.rs at
Oteryn/Oteryn-Game@39f104c39e844e523c43158e0c46370fb40fbf5f:13-219.
No runtime parser extension. AUTHORING: PR2845 comments6590727985/6591052733.
"""

import argparse
import json
import re
import subprocess
from pathlib import Path, PurePosixPath

PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
INPUTS = Path("tools/content-schema/quest-authoring/batch-one-world-callbacks")
OUTPUT = Path("content/quests/world-callbacks.json")
OWNERS = {
    "chayenne_realm": ("oteryn:quest.chayenne_realm", "chayenne_realm", 1),
    "deeper_fibula": ("oteryn:quest.deeper_fibula_quest", "deeper_fibula", 1),
    "ferumbras_ascension": (
        "oteryn:quest.ferumbras_ascension_quest",
        "ferumbras_ascension",
        1,
    ),
    "others": ("oteryn:quest.serpentine_tower_quest", "others", 1),
    "spike_tasks": ("oteryn:quest.spike_task", "the_spike_tasks", 17),
    "targuna": ("oteryn:quest.targuna_quest", "targuna", 5),
    "the_isle_of_evil_quest": (
        "oteryn:quest.the_isle_of_evil_quest",
        "the_isle_of_evil_quest",
        1,
    ),
    "white_pearl": ("oteryn:quest.white_pearl", "white_pearl", 1),
}
CALLBACK_FIELDS = {
    "key",
    "quest",
    "trigger",
    "selector",
    "lever",
    "reversible",
    "passages",
    "teleport",
    "storage",
    "minimum_level",
    "experience",
    "monk_reward",
    "offerings",
    "item_on_target",
    "return_on_failure",
    "failure_message",
    "sources",
}
CALLBACK_REQUIRED = {
    "key",
    "quest",
    "trigger",
    "selector",
    "reversible",
    "passages",
    "storage",
    "offerings",
    "return_on_failure",
    "sources",
}
SOURCE = re.compile(
    rf"(?:crystal:{PIN}|zimbadev/crystalserver@{PIN}):"
    r"([^:]+):([1-9][0-9]*(?:-[1-9][0-9]*)?(?:,[1-9][0-9]*(?:-[1-9][0-9]*)?)*)"
)


class _SourceValidationError(ValueError):
    """Decoded JSON violates the schema; this is not Python caller misuse."""


def unique_object(pairs):
    result = {}
    for name, value in pairs:
        if name in result:
            raise ValueError(f"duplicate JSON key: {name}")
        result[name] = value
    return result


def decode(text):
    return json.loads(text, object_pairs_hook=unique_object)


def fields(value, allowed, required, label):
    if not isinstance(value, dict):
        raise _SourceValidationError(f"{label}: expected object")
    if set(value) - set(allowed):
        raise ValueError(f"{label}: unknown field {sorted(set(value) - set(allowed))}")
    if set(required) - set(value):
        raise ValueError(f"{label}: missing field {sorted(set(required) - set(value))}")


def integer(value, low, high, label):
    if type(value) is not int or not low <= value <= high:
        raise ValueError(f"{label}: expected integer in {low}..{high}")


def key(value):
    if (
        not isinstance(value, str)
        or len(value) > 128
        or not re.fullmatch(r"oteryn:[A-Za-z0-9._:/-]+", value)
    ):
        raise ValueError("invalid Oteryn key")


def position(value):
    fields(value, ("x", "y", "floor"), ("x", "y", "floor"), "position")
    integer(value["x"], 0, 65535, "x")
    integer(value["y"], 0, 65535, "y")
    integer(value["floor"], -15, 0, "floor")
    return value["x"], value["y"], value["floor"]


def read_source(root, path):
    result = subprocess.run(
        ["git", "-C", str(root), "show", f"{PIN}:{path}"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if result.returncode:
        raise ValueError(f"source unavailable at pinned commit: {path}")
    return result.stdout


def validate_sources(sources, owner, source_lines):
    if not isinstance(sources, list) or not sources:
        raise ValueError("source list is empty")
    if not any(isinstance(s, str) and s.startswith(f"crystal:{PIN}:") for s in sources):
        raise ValueError("source requires the existing runtime Crystal prefix")
    folder = OWNERS[owner][1]
    for reference in sources:
        match = SOURCE.fullmatch(reference) if isinstance(reference, str) else None
        if match is None:
            raise ValueError(
                "source must contain the exact Crystal pin, path and positive line span"
            )
        path, span = match.groups()
        if (
            str(PurePosixPath(path)) != path
            or ".." in PurePosixPath(path).parts
            or not path.startswith(f"data-global/scripts/quests/{folder}/")
            or not path.endswith(".lua")
            or owner == "others"
            and path
            != "data-global/scripts/quests/others/actions_serpentine_tower_torch.lua"
        ):
            raise ValueError(f"foreign or invalid source path: {path}")
        count = source_lines(path)
        for section in span.split(","):
            limits = [int(v) for v in section.split("-")]
            if limits[0] > limits[-1] or limits[-1] > count:
                raise ValueError(
                    f"source line span outside pinned file: {path}:{section}"
                )


def validate_callback(row, owner, source_lines):
    fields(row, CALLBACK_FIELDS, CALLBACK_REQUIRED, "callback")
    if row["quest"] != OWNERS[owner][0]:
        raise ValueError(f"foreign quest in {owner}: {row['quest']}")
    key(row["key"])
    if row["trigger"] not in ("use", "step_in"):
        raise ValueError("unsupported trigger")
    for name in ("reversible", "return_on_failure"):
        if type(row[name]) is not bool:
            raise ValueError(f"{name}: expected boolean")
    selector = row["selector"]
    fields(selector, ("position", "unique", "action", "appearance"), (), "selector")
    if all(selector.get(name) is None for name in ("position", "unique", "action")):
        raise ValueError("selector requires position, unique or action")
    for name in ("unique", "action", "appearance"):
        if selector.get(name) is not None:
            integer(selector[name], 0, 65535, name)
    if selector.get("position") is not None:
        position(selector["position"])
    if row.get("teleport") is not None:
        position(row["teleport"])
    lever = row.get("lever")
    if lever is not None:
        if (
            not isinstance(lever, list)
            or len(lever) != 2
            or lever[0] == lever[1]
            or row["trigger"] != "use"
        ):
            raise ValueError("invalid lever")
        for value in lever:
            integer(value, 0, 65535, "lever")
    shapes = {
        "passages": (
            {"position", "appearance", "replacement"},
            {"position", "appearance"},
        ),
        "storage": (
            {"track", "minimum", "maximum", "set"},
            {"track", "minimum", "maximum", "set"},
        ),
        "offerings": ({"position", "item", "count"}, {"position", "item", "count"}),
    }
    for name, (allowed, required) in shapes.items():
        entries = row[name]
        if not isinstance(entries, list) or len(entries) > 8:
            raise ValueError(f"{name}: expected at most eight effects")
        seen = set()
        for entry in entries:
            fields(entry, allowed, required, name)
            identity = (
                entry["track"] if name == "storage" else position(entry["position"])
            )
            if name == "storage":
                key(identity)
            if identity in seen:
                raise ValueError(f"duplicate {name} target")
            seen.add(identity)
            if name == "storage":
                for field in ("minimum", "maximum", "set"):
                    integer(entry[field], -(1 << 63), (1 << 63) - 1, field)
                if entry["minimum"] > entry["maximum"]:
                    raise ValueError("inverted storage bounds")
            elif name == "passages":
                integer(entry["appearance"], 0, 65535, "appearance")
                if entry.get("replacement") is not None:
                    integer(entry["replacement"], 0, 65535, "replacement")
            else:
                key(entry["item"])
                integer(entry["count"], 1, 100, "count")
    if row["offerings"] and not row["storage"]:
        raise ValueError("offerings require storage")
    if row.get("minimum_level") is not None:
        integer(row["minimum_level"], 1, (1 << 32) - 1, "minimum_level")
        if not row["storage"]:
            raise ValueError("minimum_level requires storage")
    if row.get("experience") is not None:
        integer(row["experience"], 1, 100_000_000, "experience")
    if row.get("monk_reward") is not None:
        raise ValueError("foreign quest monk_reward is not part of batch one")
    item = row.get("item_on_target")
    if item is not None:
        fields(
            item,
            ("item", "count", "replacement", "target_replacement"),
            ("item", "count"),
            "item_on_target",
        )
        key(item["item"])
        integer(item["count"], 1, 100, "count")
        if row["trigger"] != "use" or not row["storage"]:
            raise ValueError("item_on_target requires use and storage")
        if item.get("replacement") is not None:
            key(item["replacement"])
            if item["count"] != 1:
                raise ValueError("replacement requires count one")
        if item.get("target_replacement") is not None:
            integer(item["target_replacement"], 1, 65535, "target_replacement")
    message = row.get("failure_message")
    if message is not None and (
        not isinstance(message, str)
        or not 1 <= len(message.encode()) <= 256
        or any(c in message for c in "\n\r\0")
    ):
        raise ValueError("invalid failure_message")
    validate_sources(row["sources"], owner, source_lines)


def load_inputs(root, crystal_root, *, complete=False, quest=None):
    paths = sorted((root / INPUTS).glob("*.json"))
    names = {p.stem for p in paths}
    if names - OWNERS.keys():
        raise ValueError(f"foreign input: {sorted(names - OWNERS.keys())}")
    if complete and names != OWNERS.keys():
        raise ValueError("final generation requires all eight private inputs")
    if quest is not None and (quest not in OWNERS or quest not in names):
        raise ValueError("unknown or missing quest input")
    if not paths:
        raise ValueError("no private inputs")
    rows, keys, line_counts = [], set(), {}

    def source_lines(path):
        if path not in line_counts:
            line_counts[path] = len(read_source(crystal_root, path).splitlines())
        return line_counts[path]

    for path in paths:
        if quest is not None and path.stem != quest:
            continue
        document = decode(path.read_text(encoding="utf-8"))
        fields(
            document,
            ("schema", "quest", "callbacks"),
            ("schema", "quest", "callbacks"),
            "input",
        )
        integer(document["schema"], 1, 1, "schema")
        if document["quest"] != OWNERS[path.stem][0]:
            raise ValueError(f"foreign quest in {path.name}")
        if not isinstance(document["callbacks"], list):
            raise _SourceValidationError("callbacks must be an array")
        for row in document["callbacks"]:
            fields(row, CALLBACK_FIELDS, CALLBACK_REQUIRED, "callback")
            key(row["key"])
            if row["key"] in keys:
                raise ValueError(f"duplicate callback key: {row['key']}")
            keys.add(row["key"])
            validate_callback(row, path.stem, source_lines)
            rows.append(row)
        if len(document["callbacks"]) != OWNERS[path.stem][2]:
            raise ValueError(f"unexpected approved callback count: {path.name}")
    return sorted(rows, key=lambda row: row["key"])


def merge_callbacks(text, incoming):
    document = decode(text)
    fields(
        document, ("schema", "callbacks"), ("schema", "callbacks"), "runtime document"
    )
    integer(document["schema"], 1, 1, "schema")
    if not isinstance(document["callbacks"], list):
        raise _SourceValidationError("callbacks must be an array")
    # Existing format: a literal top-level callbacks key. Preserve every foreign
    # object as its original text, including nulls, escapes and field order.
    match = re.search(r'"callbacks"\s*:\s*\[', text)
    if match is None:
        raise ValueError("missing literal callbacks array")
    cursor = match.end()
    decoder = json.JSONDecoder(object_pairs_hook=unique_object)
    existing, keys = [], set()
    for row in document["callbacks"]:
        cursor += len(text[cursor:]) - len(text[cursor:].lstrip())
        value, end = decoder.raw_decode(text, cursor)
        if not isinstance(row, dict) or not isinstance(row.get("quest"), str):
            raise _SourceValidationError("invalid existing callback identity")
        key(row.get("key"))
        if row["key"] in keys:
            raise ValueError(f"duplicate callback key: {row['key']}")
        keys.add(row["key"])
        if value != row:
            raise ValueError("callback array span mismatch")
        existing.append((row, text[cursor:end]))
        cursor = end + len(text[end:]) - len(text[end:].lstrip())
        if text[cursor : cursor + 1] == ",":
            cursor += 1
    cursor += len(text[cursor:]) - len(text[cursor:].lstrip())
    if text[cursor : cursor + 1] != "]":
        raise ValueError("callback array boundary mismatch")
    updates = {}
    for row in incoming:
        if row["key"] in updates:
            raise ValueError(f"duplicate callback key: {row['key']}")
        updates[row["key"]] = row
    merged, changed = [], False
    for old, raw in existing:
        new = updates.pop(old["key"], None)
        if new is not None and new["quest"] != old["quest"]:
            raise ValueError(f"foreign quest key collision: {old['key']}")
        if new is not None and old != new:
            raw = json.dumps(new, ensure_ascii=False, indent=2).replace("\n", "\n    ")
            changed = True
        merged.append((new if new is not None else old, raw))
    for name, row in sorted(updates.items()):
        raw = json.dumps(row, ensure_ascii=False, indent=2).replace("\n", "\n    ")
        index = next(
            (i for i, (old, _) in enumerate(merged) if old["key"] > name), len(merged)
        )
        merged.insert(index, (row, raw))
        changed = True
    # load() reserves one of MAX_CALLBACKS=128 for the existing Rotten Blood altar.
    if len(merged) >= 128:
        raise ValueError("callback bounds including Rotten Blood altar")
    if not changed:
        return text
    body = "\n    " + ",\n    ".join(raw for _, raw in merged) + "\n  "
    return text[: match.end()] + body + text[cursor:]


def run(root, crystal_root, *, check=False, inputs_only=False, quest=None):
    # A cherry-picked quest can regenerate its own block without the other seven
    # inputs. The unfiltered final run still requires the complete batch.
    rows = load_inputs(
        root, crystal_root, complete=not inputs_only and quest is None, quest=quest
    )
    if inputs_only:
        return len(rows)
    path = root / OUTPUT
    before = path.read_bytes().decode("utf-8")
    after = merge_callbacks(before, rows)
    if check and before != after:
        raise ValueError(f"out of date: {OUTPUT}")
    if not check and before != after:
        path.write_bytes(after.encode("utf-8"))
    return len(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[3]
    )
    parser.add_argument("--crystal-root", type=Path, required=True)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--check-inputs-only", action="store_true")
    parser.add_argument("--quest", choices=sorted(OWNERS))
    args = parser.parse_args()
    try:
        count = run(
            args.root,
            args.crystal_root,
            check=args.check,
            inputs_only=args.check_inputs_only,
            quest=args.quest,
        )
    except (ValueError, OSError) as error:
        parser.exit(1, f"batch-one callbacks: {error}\n")
    label = (
        "AUTHORING inputs only; runtime untouched"
        if args.check_inputs_only
        else "final callback registry"
    )
    print(f"{label}: {count} approved callbacks")


if __name__ == "__main__":
    main()
