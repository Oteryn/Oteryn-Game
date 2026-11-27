"""Admit the official ground appearances already placed under the Crystal spawn delta."""

import argparse
import json
from functools import lru_cache

from activity_static import compose_terrain_notes
from summer_ground import ROOT, outputs_for

GROUND_SPEEDS = {
    45317: 100,
    **dict.fromkeys(range(45470, 45477), 150),
    **dict.fromkeys(range(48568, 48574), 150),
    50815: 100,
}
SHARD = "content/world/terrain/terrain-08751-08765.json"
MARKER_NOTE = (
    " Additionally 14 appearance-only ground records at Crystal 201189f8 spawn cells."
)


@lru_cache(maxsize=2)
def outputs(root=ROOT):
    from scoped_spawn_ground import records as scoped_records

    ground_speeds = dict(GROUND_SPEEDS)
    # The scoped reconciliation publishes this appearance at both reviewed cells.
    shared = scoped_records(root)[50815]
    if shared["ground_speed"]["value"] != ground_speeds.pop(50815):
        raise ValueError("Crystal spawn ground 50815 differs from the scoped admission")
    result = outputs_for(root, ground_speeds, SHARD, MARKER_NOTE)
    marker_path = "content/world/terrain/index.json"
    marker = json.loads(result[marker_path])
    marker["notes"] = compose_terrain_notes(marker["notes"])
    result[marker_path] = (
        json.dumps(marker, indent=2, ensure_ascii=False) + "\n"
    ).encode()
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    files = outputs()
    if args.check:
        from world_objects import validate_record

        for record in json.loads(files[SHARD])["records"]:
            if errors := validate_record(record):
                raise SystemExit(
                    f"invalid Crystal spawn ground {record['identity']}: {errors}"
                )
    for path, raw in files.items():
        target = ROOT / path
        if args.check:
            if target.read_bytes() != raw:
                raise SystemExit(f"stale Crystal spawn ground: {path}")
        else:
            target.write_bytes(raw)
