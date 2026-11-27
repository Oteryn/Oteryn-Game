"""Refresh the deployment pin from the canonical project and staged catalogue."""

import argparse
import json
from pathlib import Path

from stage_npc_data import (
    PIN_SCHEMA,
    catalogue_digest,
    collect_catalogue,
    project_files,
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "repository_root", nargs="?", type=Path, default=Path(__file__).resolve().parents[2]
    )
    args = parser.parse_args()

    content = args.repository_root / "content"
    revision, project = project_files(content / "world")
    catalogue = collect_catalogue(content)
    if not catalogue:
        raise ValueError("no catalogue files")
    pin = {
        "schema": PIN_SCHEMA,
        "project_revision": revision,
        "project_sha256": catalogue_digest(project.items()),
        "catalogue_sha256": catalogue_digest(catalogue),
    }
    path = args.repository_root / "deploy/synology-game/npc-data.pin.json"
    path.write_text(json.dumps(pin, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
