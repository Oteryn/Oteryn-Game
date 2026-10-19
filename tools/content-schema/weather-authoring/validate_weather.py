#!/usr/bin/env python3
"""Validate content/weather (CP D1448, OTERYN_DESIGN values).

Mirrors the loader in apps/game-server/src/world_weather.rs: a file that fails here would make
the server fail closed and never offer capability 26. Standard library only.

Usage: validate_weather.py [content/weather]
"""
import json
import sys
from pathlib import Path

KINDS = {"clear", "cloudy", "rain", "storm", "fog", "snow", "blizzard", "heat_haze", "sandstorm"}
ROOT = Path(__file__).resolve().parents[3]

# Exact key sets of the serde structs in world_weather.rs (all deny_unknown_fields).
# (required, optional): an Option<T> field may be absent, every other field is required.
INDEX_KEYS = ({"schema", "family", "kind", "owner", "population_state", "design_status",
               "revision", "slot_ms", "files", "notes"}, set())
CLIMATES_KEYS = ({"schema", "design_status", "notes", "duration_slots", "intensity", "climates"}, set())
AREAS_KEYS = ({"schema", "design_status", "notes", "areas"}, set())
RANGE_KEYS = ({"min", "max"}, set())
AREA_KEYS = ({"key", "kind", "climate"}, {"anchor", "real_world"})
ANCHOR_KEYS = ({"x", "y", "z"}, set())
REAL_WORLD_KEYS = ({"latitude", "longitude"}, set())


def load(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def check_keys(value, spec, where, errors):
    """Reject unknown and missing keys like serde deny_unknown_fields; False if not an object."""
    if not isinstance(value, dict):
        errors.append(f"{where}: must be an object")
        return False
    required, optional = spec
    for key in sorted(set(value) - required - optional):
        errors.append(f"{where}: unknown key {key}")
    for key in sorted(required - set(value)):
        errors.append(f"{where}: missing key {key}")
    return True


def validate(directory):
    errors = []

    def need(condition, message):
        if not condition:
            errors.append(message)

    index = load(directory / "index.json")
    climates = load(directory / "climates.json")
    areas = load(directory / "areas.json")
    for name, doc, spec in (("index", index, INDEX_KEYS), ("climates", climates, CLIMATES_KEYS),
                            ("areas", areas, AREAS_KEYS)):
        if not check_keys(doc, spec, name, errors):
            return errors
    for name in ("duration_slots", "intensity"):
        if not check_keys(climates.get(name), RANGE_KEYS, f"climates.{name}", errors):
            return errors
    if not isinstance(areas.get("areas"), list) or not all(
            check_keys(a, AREA_KEYS, f"areas[{i}]", errors) for i, a in enumerate(areas["areas"])):
        return errors or ["areas must be a list"]
    for a in areas["areas"]:
        key = a.get("key")
        if a.get("anchor") is not None and not check_keys(a["anchor"], ANCHOR_KEYS, f"{key}.anchor", errors):
            return errors
        if a.get("real_world") is not None and not check_keys(
                a["real_world"], REAL_WORLD_KEYS, f"{key}.real_world", errors):
            return errors
    need(index.get("schema") == "OTERYN_WEATHER_INDEX/v1", "index schema")
    need(climates.get("schema") == "OTERYN_WEATHER_CLIMATES/v1", "climates schema")
    need(areas.get("schema") == "OTERYN_WEATHER_AREAS/v1", "areas schema")
    for name, doc in (("index", index), ("climates", climates), ("areas", areas)):
        need(doc.get("design_status") == "OTERYN_DESIGN", f"{name} design_status")
    need(index.get("slot_ms") == 900000, "slot_ms must be 900000")
    need(bool(index.get("revision")), "revision")
    need(index.get("files") == {"climates": "climates.json", "areas": "areas.json"}, "files")

    duration = climates.get("duration_slots", {})
    need(1 <= duration.get("min", 0) <= duration.get("max", 0) <= 12, "duration_slots")
    intensity = climates.get("intensity", {})
    need(1 <= intensity.get("min", 0) <= intensity.get("max", 0) <= 3, "intensity range")
    table = climates.get("climates", {})
    need(bool(table), "climates must not be empty")
    for name, weights in table.items():
        need(bool(weights), f"climate {name} has no kinds")
        for kind, weight in weights.items():
            need(kind in KINDS, f"climate {name}: unknown kind {kind}")
            need(isinstance(weight, int) and weight > 0, f"climate {name}.{kind}: weight")

    keys = set()
    anchored = 0
    known = set()
    for sub in ("cities", "regions"):
        for path in (ROOT / "content/world/areas" / sub).glob("areas-*.json"):
            known.update(a["identity"]["key"] for a in load(path)["areas"])
    for area in areas.get("areas", []):
        key = area.get("key")
        need(bool(key) and key not in keys, f"area key {key}")
        keys.add(key)
        need(area.get("kind") in ("region", "city"), f"{key}: kind")
        need(area.get("climate") in table, f"{key}: unknown climate")
        if known:
            need(key in known, f"{key}: not an area in content/world/areas")
        anchor = area.get("anchor")
        if anchor is not None:
            anchored += 1
            need(all(isinstance(anchor.get(c), int) for c in "xyz"), f"{key}: anchor")
        real = area.get("real_world")
        if real is not None:
            need(-90 <= real.get("latitude", 999) <= 90, f"{key}: latitude")
            need(-180 <= real.get("longitude", 999) <= 180, f"{key}: longitude")
    need(anchored > 0, "at least one area needs an anchor")
    return errors


def self_test():
    """Negative cases: an unknown key and a missing required key must be rejected."""
    import copy
    import tempfile

    src = ROOT / "content/weather"
    docs = {n: load(src / f"{n}.json") for n in ("index", "climates", "areas")}
    cases = {
        "unknown top-level key": lambda d: d["index"].update(extra=1),
        "missing top-level key": lambda d: d["climates"].pop("notes"),
        "unknown nested range key": lambda d: d["climates"]["intensity"].update(step=1),
        "missing nested range key": lambda d: d["climates"]["duration_slots"].pop("max"),
        "unknown area key": lambda d: d["areas"]["areas"][0].update(extra=1),
        "missing area key": lambda d: d["areas"]["areas"][0].pop("climate"),
        "unknown anchor key": lambda d: next(
            a for a in d["areas"]["areas"] if a.get("anchor"))["anchor"].update(w=0),
    }
    failures = []
    for label, mutate in cases.items():
        doc = copy.deepcopy(docs)
        mutate(doc)
        with tempfile.TemporaryDirectory() as tmp:
            for name, value in doc.items():
                Path(tmp, f"{name}.json").write_text(json.dumps(value), encoding="utf-8")
            if not validate(Path(tmp)):
                failures.append(label)
    for label in failures:
        print(f"weather self-test: not rejected: {label}", file=sys.stderr)
    if failures:
        return 1
    print(f"weather self-test: ok ({len(cases)} negative cases)")
    return 0


def main(argv):
    if len(argv) > 1 and argv[1] == "--self-test":
        return self_test()
    directory = Path(argv[1]) if len(argv) > 1 else ROOT / "content/weather"
    errors = validate(directory)
    for error in errors:
        print(f"weather: {error}", file=sys.stderr)
    if errors:
        return 1
    print(f"weather: ok ({directory})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
