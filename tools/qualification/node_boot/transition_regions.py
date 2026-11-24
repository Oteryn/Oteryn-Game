"""Select the town fixture and map regions needed by an authored teleport batch."""
import json
from pathlib import Path
import re
import sys


def regions(root: Path, batch: str) -> str:
    if not re.fullmatch(r"[0-9]+:[0-9]+", batch):
        raise ValueError("expected start:len")
    start, length = map(int, batch.split(":"))
    index = json.loads((root / "content/world/transitions/index.json").read_text())
    entries = sorted(
        (record["declaration"] for shard in index["shards"]
         for record in json.loads((root / shard).read_text())["records"]),
        key=lambda entry: entry["identity"]["key"],
    )
    if (length < 1 or start + length > len(entries)
            or any(entry["transition_kind"] != "teleport" for entry in entries)):
        raise ValueError("regional mode requires a batch entirely inside the authored teleport prefix")
    town = root / "tools/qualification/node_boot/town-regions.txt"
    selected = {tuple(map(int, region.split(","))) for region in town.read_text().strip().split(";")}
    for entry in entries[start:start + length]:
        source = entry["from"]
        # The client can start on either side of a region boundary.
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                selected.add(((source["x"] + dx) // 256, (source["y"] + dy) // 256))
        target = entry["to"]
        selected.add((target["x"] // 256, target["y"] // 256))
    return ";".join(f"{x},{y}" for x, y in sorted(selected))


if __name__ == "__main__":
    try:
        print(regions(Path(sys.argv[1]), sys.argv[2]))
    except (ValueError, KeyError, OSError) as error:
        sys.exit(f"transition regions: {error}")
