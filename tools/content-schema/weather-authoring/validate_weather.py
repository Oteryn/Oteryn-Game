#!/usr/bin/env python3
"""Validate content/weather (CP D1448, OTERYN_DESIGN values).

Mirrors the loader in crates/gameplay/weather/src/lib.rs: a file that fails here would make
the server fail closed and never offer capability 26. Standard library only.

Usage: validate_weather.py [content/weather]
"""
import json
import sys
from pathlib import Path

KINDS = {"clear", "cloudy", "rain", "storm", "fog", "snow", "blizzard", "heat_haze", "sandstorm"}
ROOT = Path(__file__).resolve().parents[3]

# Exact fields of the serde structs in crates/gameplay/weather/src/lib.rs (all deny_unknown_fields): name -> Rust
# type. Option<T> fields may be absent, every other field is required. Types: "str" (String),
# "u8"/"u32"/"u64"/"i32" (integers, never bool), "f64" (finite number, never bool), None
# (a nested record or collection, checked by its own spec below).
INT_BOUNDS = {"u8": (0, 2**8 - 1), "u32": (0, 2**32 - 1), "u64": (0, 2**64 - 1),
              "i32": (-(2**31), 2**31 - 1)}
INDEX_KEYS = ({"schema": "str", "family": "str", "kind": "str", "owner": "str",
               "population_state": "str", "design_status": "str", "revision": "str",
               "slot_ms": "u64", "files": None, "notes": "str"}, set())
CLIMATES_KEYS = ({"schema": "str", "design_status": "str", "notes": "str", "duration_slots": None,
                  "intensity": None, "climates": None}, set())
AREAS_KEYS = ({"schema": "str", "design_status": "str", "notes": "str", "areas": None}, set())
RANGE_KEYS = ({"min": "u8", "max": "u8"}, set())
AREA_KEYS = ({"key": "str", "kind": "str", "climate": "str", "anchor": None, "real_world": None},
             {"anchor", "real_world"})
ANCHOR_KEYS = ({"x": "i32", "y": "i32", "z": "i32"}, set())
REAL_WORLD_KEYS = ({"latitude": "f64", "longitude": "f64"}, set())

def load(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def scalar_ok(value, rust_type):
    """True when a JSON value deserializes into the Rust type (serde_json rules)."""
    if rust_type is None:
        return True
    if rust_type == "str":
        return isinstance(value, str)
    if rust_type == "f64":
        return (isinstance(value, (int, float)) and not isinstance(value, bool)
                and value == value and abs(value) != float("inf"))
    low, high = INT_BOUNDS[rust_type]
    return isinstance(value, int) and not isinstance(value, bool) and low <= value <= high


def check_keys(value, spec, where, errors):
    """Reject unknown, missing and wrongly typed fields like serde; False if not an object."""
    if not isinstance(value, dict):
        errors.append(f"{where}: must be an object")
        return False
    fields, optional = spec
    required = set(fields) - optional
    for key in sorted(set(value) - set(fields)):
        errors.append(f"{where}: unknown key {key}")
    for key in sorted(required - set(value)):
        errors.append(f"{where}: missing key {key}")
    for key, rust_type in fields.items():
        if key in value and not scalar_ok(value[key], rust_type):
            errors.append(f"{where}.{key}: must be {rust_type}")
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
    files = index["files"]
    if not isinstance(files, dict) or not all(
            isinstance(k, str) and isinstance(v, str) for k, v in files.items()):
        return errors + ["index.files: must be an object of strings"]
    table_doc = climates["climates"]
    if not isinstance(table_doc, dict) or not all(
            isinstance(w, dict) and all(scalar_ok(v, "u32") for v in w.values())
            for w in table_doc.values()):
        return errors + ["climates.climates: must map climate -> kind -> u32 weight"]
    for a in areas["areas"]:
        key = a.get("key")
        if a.get("anchor") is not None and not check_keys(a["anchor"], ANCHOR_KEYS, f"{key}.anchor", errors):
            return errors
        if a.get("real_world") is not None and not check_keys(
                a["real_world"], REAL_WORLD_KEYS, f"{key}.real_world", errors):
            return errors
    if errors:
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
    need(1 <= duration["min"] <= duration["max"] <= 12, "duration_slots")
    intensity = climates.get("intensity", {})
    need(1 <= intensity["min"] <= intensity["max"] <= 3, "intensity range")
    table = climates.get("climates", {})
    need(bool(table), "climates must not be empty")
    for name, weights in table.items():
        need(bool(weights), f"climate {name} has no kinds")
        for kind, weight in weights.items():
            need(kind in KINDS, f"climate {name}: unknown kind {kind}")
            need(weight > 0, f"climate {name}.{kind}: weight")

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
        real = area.get("real_world")
        if real is not None:
            need(-90 <= real["latitude"] <= 90, f"{key}: latitude")
            need(-180 <= real["longitude"] <= 180, f"{key}: longitude")
    need(anchored > 0, "at least one area needs an anchor")
    return errors


def self_test():
    """Negative cases: unknown/missing keys, wrong scalar types and loader range checks."""
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
    first_climate = lambda d: next(iter(d["climates"]["climates"].values()))
    def real_world(d):
        area = d["areas"]["areas"][0]
        area["real_world"] = {"latitude": 0.0, "longitude": 0.0}
        return area["real_world"]

    anchored = lambda d: next(a for a in d["areas"]["areas"] if a.get("anchor"))
    cases.update({
        "bool weight": lambda d: first_climate(d).update({next(iter(first_climate(d))): True}),
        "weight above u32": lambda d: first_climate(d).update({next(iter(first_climate(d))): 2**32}),
        "zero weight": lambda d: first_climate(d).update({next(iter(first_climate(d))): 0}),
        "negative weight": lambda d: first_climate(d).update({next(iter(first_climate(d))): -1}),
        "float weight": lambda d: first_climate(d).update({next(iter(first_climate(d))): 1.5}),
        "string weight": lambda d: first_climate(d).update({next(iter(first_climate(d))): "5"}),
        "bool range": lambda d: d["climates"]["intensity"].update(min=True),
        "range above u8": lambda d: d["climates"]["duration_slots"].update(max=256),
        "negative range": lambda d: d["climates"]["duration_slots"].update(min=-1),
        "range min above max": lambda d: d["climates"]["intensity"].update(min=3, max=2),
        "bool slot_ms": lambda d: d["index"].update(slot_ms=True),
        "wrong slot_ms": lambda d: d["index"].update(slot_ms=1000),
        "numeric string field": lambda d: d["index"].update(notes=5),
        "null string field": lambda d: d["index"].update(revision=None),
        "empty revision": lambda d: d["index"].update(revision=""),
        "anchor above i32": lambda d: anchored(d)["anchor"].update(x=2**31),
        "anchor bool": lambda d: anchored(d)["anchor"].update(y=True),
        "anchor float": lambda d: anchored(d)["anchor"].update(z=1.5),
        "latitude out of range": lambda d: real_world(d).update(latitude=91),
        "latitude bool": lambda d: real_world(d).update(latitude=True),
        "latitude string": lambda d: real_world(d).update(longitude="1"),
        "area key not a string": lambda d: d["areas"]["areas"][0].update(key=1),
        "empty area key": lambda d: d["areas"]["areas"][0].update(key=""),
        "files value not a string": lambda d: d["index"]["files"].update(climates=1),
    })
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
