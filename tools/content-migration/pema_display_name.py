"""Preserve Enpa-Deia Pema's source display name in the staged World Project."""

import json
from pathlib import Path


def install_pema_display_name(world: Path) -> None:
    """Preserve the donor spelling when the World Project is rebuilt from its predecessor."""
    path = world / "definitions/declarations.json"
    document = json.loads(path.read_bytes())
    npc, = (
        row for row in document["records"]
        if row["kind"] == "NPC" and row["identity"]["key"] == "oteryn:npc.enpa_deia_pema"
    )
    field_path = "oteryn:source.npc.display_name"
    fields = [field for field in npc["fields"] if field["field_path"] != field_path]
    # Crystal 201189f8, data-global/npc/enpa_deia_pema.lua, internalNpcName.
    fields.append({"field_path": field_path, "value": {"type": "Text", "value": "Enpa-Deia Pema"}})
    npc["fields"] = sorted(fields, key=lambda field: field["field_path"])
    path.write_text(json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n")


if __name__ == "__main__":
    from questpc7_world_seal import refresh

    root = Path(__file__).resolve().parents[2]
    world = root / "content/world"
    install_pema_display_name(world)
    refresh(root, world)
