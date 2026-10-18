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


def load(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def validate(directory):
    errors = []

    def need(condition, message):
        if not condition:
            errors.append(message)

    index = load(directory / "index.json")
    climates = load(directory / "climates.json")
    areas = load(directory / "areas.json")
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


def main(argv):
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
