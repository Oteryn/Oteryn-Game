"""Generate the Synology and Oracle NPC staging pin from the admitted trees."""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEPLOY = ROOT / "deploy/synology-game"
sys.path.insert(0, str(DEPLOY))

from stage_npc_data import (
    PIN_SCHEMA,
    catalogue_digest,
    collect_catalogue,
    project_files,
)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    content = ROOT / "content"
    revision, documents = project_files(content / "world")
    catalogue = collect_catalogue(content)
    if not catalogue:
        raise ValueError("no NPC catalogue files")
    path = DEPLOY / "npc-data.pin.json"
    pin = {
        "schema": PIN_SCHEMA,
        "project_revision": revision,
        "project_sha256": catalogue_digest(documents.items()),
        "catalogue_sha256": catalogue_digest(catalogue),
    }
    expected = json.dumps(pin, indent=2) + "\n"
    if args.check:
        if path.read_text(encoding="utf-8") != expected:
            raise SystemExit(f"NPC staging pin drift: {path.relative_to(ROOT)}")
    else:
        path.write_text(expected, encoding="utf-8")


if __name__ == "__main__":
    main()
