"""Project selected paid-travel corrections into canonical Service.Travel records."""

import json
from pathlib import Path

from questpc7_world_seal import refresh


def project_routes(root: Path, package: Path, *, check: bool = False) -> None:
    corrections = json.loads(
        (root / "crates/gameplay/content/src/content/npc_catalogue/travel_routes.json").read_bytes()
    )
    path = package / "definitions/declarations.json"
    document = json.loads(path.read_bytes())
    routes = {
        f"{record['identity']['key']}/{route['key']}": route
        for record in document["records"]
        if record["kind"] == "Service"
        for route in record.get("routes", [])
    }
    changed = False
    for correction in corrections:
        if "project_required_quest_tracks" not in correction:
            continue
        tracks = correction["project_required_quest_tracks"]
        if not isinstance(tracks, list):
            raise TypeError(f"Expected a quest requirement list: {correction['key']}")
        route = routes[correction["key"]]
        expected = dict(route)
        expected["destination"] = {
            **route["destination"],
            **{axis: correction[axis] for axis in ("x", "y", "floor")},
        }
        expected["price"] = correction["price"]
        expected["premium"] = correction["premium"]
        expected.pop("min_level", None)
        expected.pop("required_quest_tracks", None)
        if tracks:
            expected["required_quest_tracks"] = tracks
        if route != expected:
            if check:
                raise SystemExit(f"Canonical travel route differs from correction: {correction['key']}")
            route.clear()
            route.update(expected)
            changed = True
    if changed:
        path.write_text(
            json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n",
            encoding="utf-8",
        )
        refresh(root, package)
