"""S06 bounded census: permanent cosmetic grants versus current production NPC placement.

Reads raw NPC files fetched at the pinned Crystal tree, not executable Lua. Only
normalized file identities and grant line anchors are emitted, never donor source.
"""

import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = Path(__file__).with_name("srv7-npc-reachability.json")


def placed(root):
    result = set()
    for path in (root / "content/world/npc-placements").glob("placements-*.json"):
        result.update(
            row["declaration"]["npc"]
            for row in json.loads(path.read_bytes())["records"]
        )
    source = (root / "apps/game-server/src/node/serve/bundle_npcs.rs").read_text()
    table = source[source.index("const TIMED_NPCS:") :].split("];", 1)[0]
    result.update(re.findall(r'"(oteryn:npc\.[a-z_]+)"', table))
    return result


def capture(tree, sources):
    rows = [
        row
        for row in json.loads(tree.read_bytes())["tree"]
        if row["type"] == "blob"
        and row["path"].startswith("data-global/npc/")
        and row["path"].endswith(".lua")
    ]
    grants = []
    for row in rows:
        raw = (sources / Path(row["path"]).name).read_bytes()
        if (
            hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
            != row["sha"]
        ):
            raise ValueError(f"NPC source is not pinned: {row['path']}")
        lines = [
            i
            for i, line in enumerate(raw.decode().splitlines(), 1)
            if ":addOutfit" in line or ":addMount" in line
        ]
        if lines:
            grants.append(
                {
                    "npc": "oteryn:npc." + Path(row["path"]).stem,
                    "path": row["path"],
                    "sha256": hashlib.sha256(raw).hexdigest(),
                    "grant_lines": lines,
                }
            )
    actual = placed(ROOT)
    return {
        "schema": 1,
        "classification": "OTS_HYPOTHESIS_ONLY",
        "repository": "zimbadev/crystalserver",
        "revision": "201189f871d23597b82caf88fd611454a2ddefb8",
        "npc_files_checked": len(rows),
        "grant_files": len(grants),
        "reachable_grant_npcs": [row for row in grants if row["npc"] in actual],
        "unplaced_grant_npcs": [row for row in grants if row["npc"] not in actual],
        "exclusions": [
            "Testserver Assistant is an OT-only donor source and is never admitted."
        ],
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tree", type=Path, required=True)
    parser.add_argument("--sources", type=Path, required=True)
    args = parser.parse_args()
    OUTPUT.write_text(json.dumps(capture(args.tree, args.sources), indent=2) + "\n")
