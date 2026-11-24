"""Refresh the NPC data pin shared by Synology and Oracle game deploys."""

import json
from pathlib import Path

from stage_npc_data import (
    PIN_SCHEMA,
    catalogue_digest,
    collect_catalogue,
    project_files,
)


def main():
    content = Path(__file__).resolve().parents[2] / "content"
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
    Path(__file__).with_name("npc-data.pin.json").write_text(
        json.dumps(pin, indent=2) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
