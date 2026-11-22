"""Install Buddel's admitted paid passages into the staged WorldProject."""

import json
from pathlib import Path

from questpc7_world_seal import refresh


def install_buddel(root: Path, package: Path) -> None:
    source = json.loads((root / "content/services/travel/buddel.authoring.json").read_bytes())
    path = package / "definitions/declarations.json"
    document = json.loads(path.read_bytes())
    records = document["records"]
    npc = next(row for row in records if row["identity"]["key"] == source["npc"])
    service = source["service"]
    identity = {"key": service, "revision": npc["identity"]["revision"]}
    declaration = {
        "kind": "Service",
        "identity": identity,
        "fields": [{
            "field_path": "oteryn:source.npc.travel.donor",
            "value": {"type": "Text", "value": source["donor"]},
        }],
        "routes": source["routes"],
    }

    npc["services"] = [ref for ref in npc["services"] if ref["key"] != service]
    npc["services"].append({"family": "Service", **identity})
    npc["services"].sort(key=lambda ref: (ref["family"], ref["key"], ref["revision"]))
    npc["fields"] = [
        field for field in npc["fields"]
        if field["field_path"] != "oteryn:source.npc.travel.unadmitted_routes"
    ]
    document["records"] = [row for row in records if row["identity"]["key"] != service]
    index = next(
        i for i, row in enumerate(document["records"])
        if row["kind"] == "Service" and row["identity"]["key"] > service
    )
    document["records"].insert(index, declaration)
    path.write_text(
        json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    refresh(root, package)
