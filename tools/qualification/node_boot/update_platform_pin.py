#!/usr/bin/env python3
"""Pin creation qualification producers to a checkout with the v2/v4 contracts."""
import argparse
import json
from pathlib import Path
import re
import subprocess


def update(root: Path, platform: Path) -> str:
    revision = subprocess.check_output(["git", "-C", str(platform), "rev-parse", "HEAD"], text=True).strip()
    for directory, version in (("character-create-v2", 2), ("character-bootstrap-intent-v4", 4)):
        fixtures = platform / "docs/contracts/fixtures" / directory
        if not fixtures.is_dir():
            raise ValueError(f"missing producer fixtures: {directory}")
        if not any(json.loads(path.read_text()).get("contract_version") == version for path in fixtures.glob("*.json")):
            raise ValueError(f"missing current producer version: {directory}")
    for relative, name in (
        ("tools/qualification/node_boot/run.sh", "PLATFORM_SHA"),
        ("tools/qualification/game_platform/lifecycle.sh", "GAME_PLATFORM_SHA"),
        ("tools/qualification/game_platform/run.sh", "GAME_PLATFORM_SHA"),
    ):
        path = root / relative
        original = path.read_text()
        updated, count = re.subn(rf"(?m)^readonly {name}=[0-9a-f]{{40}}$", f"readonly {name}={revision}", original)
        if count != 1:
            raise ValueError(f"expected one producer pin in {relative}")
        path.write_text(updated)
    readme = root / "tools/qualification/game_platform/README.md"
    readme.write_text(re.sub(r"`[0-9a-f]{40}`", f"`{revision}`", readme.read_text(), count=1))
    return revision


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("platform", type=Path)
    args = parser.parse_args()
    print(update(Path(__file__).resolve().parents[3], args.platform.resolve()))
