#!/usr/bin/env python3
"""Regenerate ready-lane equipment count pins from independent admission inputs."""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TARGET = (
    ROOT
    / "crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs"
)
BEGIN = "    // BEGIN GENERATED QREADY PROFILE ADDITIONS\n"
END = "    // END GENERATED QREADY PROFILE ADDITIONS\n"


def expected():
    sys.path.insert(0, str(ROOT / "tools/content-migration"))
    import qready2_native_profiles

    source = "tools/content-schema/native-gameplay/qready1-reward-item-profiles.json"
    records = json.loads((ROOT / source).read_bytes())["records"]
    first = {row["production_definition"]["production_key"] for row in records}
    if len(first) != len(records) or any(
        row["attributes"]["speed_bonus"] != 0 for row in records
    ):
        raise ValueError(
            "QREADY1 admission proof must contain unique zero-speed profiles"
        )
    # Shared engine admissions have their own independently pinned count below.
    shared = json.loads(
        (
            ROOT
            / "tools/content-schema/native-gameplay/qengine-reward-item-profiles.json"
        ).read_bytes()
    )["records"]
    shared_keys = {row["production_definition"]["production_key"] for row in shared}
    first -= shared_keys
    second = qready2_native_profiles.keys(ROOT) - shared_keys
    if first & second:
        raise ValueError("ready-lane admission cohorts overlap")
    # M14 admits exactly one independent, source-qualified physical input.
    # Count its admission packet, never the generated runtime union.
    import srv7_targuna_item_profiles
    ram_source = srv7_targuna_item_profiles.PATH
    ram_records = json.loads((ROOT / ram_source).read_bytes())["records"]
    ram_keys = {row["production_definition"]["production_key"] for row in ram_records}
    if (ram_keys != {srv7_targuna_item_profiles.KEY} or len(ram_records) != 1
            or ram_keys & (shared_keys | first | second)
            or any(row["attributes"]["speed_bonus"] != 0 for row in ram_records)):
        raise ValueError("SRV7 M14 admission must be one distinct zero-speed turnip profile")
    block = (
        BEGIN
        + "    // Regenerate: python3 tools/contracts/update_equipment_profile_additions.py\n"
    )
    for name, keys, owner in (
        ("QREADY1", first, source),
        ("QREADY2", second, "tools/content-migration/qready2_native_profiles.py"),
        ("SRV7-M14", ram_keys, ram_source),
    ):
        ids = sorted(int(key.rsplit(".i", 1)[1]) for key in keys)
        block += f"    // {owner}\n    // Source-qualified zero speed: {', '.join(map(str, ids))}.\n"
        block += f'    ("{name}", {len(keys)}),\n'
    block += END
    text = TARGET.read_text()
    start = text.index("const PROFILE_ADDITIONS:")
    end = text.index("\n];", start)
    if BEGIN in text[start:end]:
        start = text.index(BEGIN, start)
        end = text.index(END, start) + len(END)
    else:
        start = end = end + 1
    return text[:start] + block + text[end:]


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = expected()
    if args.check:
        if TARGET.read_text() != output:
            raise SystemExit("STALE equipment profile admission pins")
        print("PASS equipment profile admission pins")
    else:
        TARGET.write_text(output)
        print("Regenerated equipment profile admission pins")
