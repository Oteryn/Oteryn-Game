"""Select the town fixture and map regions needed by a key-sorted transition batch."""
import json
from pathlib import Path
import re
import sys


def regions(root: Path, batch: str, plan: Path | None = None) -> str:
    if not re.fullmatch(r"[0-9]+:[0-9]+", batch):
        raise ValueError("expected start:len")
    start, length = map(int, batch.split(":"))
    if plan:
        entries = json.loads(plan.read_text())
        if any(left[0] >= right[0] for left, right in zip(entries, entries[1:])):
            raise ValueError("full-bundle plan is not in unique key order")
    else:
        index = json.loads((root / "content/world/transitions/index.json").read_text())
        declarations = (
            record["declaration"] for shard in index["shards"]
            for record in json.loads((root / shard).read_text())["records"]
        )
        entries = sorted(
            (entry["identity"]["key"], [entry["from"]["x"], entry["from"]["y"]],
             [entry["to"]["x"], entry["to"]["y"]])
            for entry in declarations if entry["transition_kind"] == "teleport"
        )
    if length < 1 or start + length > len(entries):
        raise ValueError("batch outside the plan; native placements require a full-bundle plan")
    town = root / "tools/qualification/node_boot/town-regions.txt"
    selected = {tuple(map(int, region.split(","))) for region in town.read_text().strip().split(";")}
    for _, (x, y), target in entries[start:start + length]:
        # Include both sides of region boundaries, plus native offset landings.
        for dx in (-2, 0, 2):
            for dy in (-2, 0, 2):
                if 0 <= x + dx <= 65535 and 0 <= y + dy <= 65535:
                    selected.add(((x + dx) // 256, (y + dy) // 256))
        if target:
            selected.add((target[0] // 256, target[1] // 256))
    return ";".join(f"{x},{y}" for x, y in sorted(selected))


if __name__ == "__main__":
    try:
        print(regions(Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3]) if len(sys.argv) == 4 else None))
    except (ValueError, KeyError, OSError) as error:
        sys.exit(f"transition regions: {error}")
