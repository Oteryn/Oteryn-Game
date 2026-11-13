"""Add the pinned registered Spite GlobalEvent loop to its real Encounter/v2."""
import argparse
import copy
import json
from pathlib import Path

KEY = ("Encounter", "oteryn:encounter.soul_war_taint_zones", "definition-r1")
RULE = "spite_registered_global_fire"


def apply(profiles, root):
    capture = json.loads((root / "tools/content-schema/native-gameplay/srx3-spite-global-event.json").read_bytes())
    if capture["revision"] != "201189f871d23597b82caf88fd611454a2ddefb8" or len(capture["events"]) != 1:
        raise ValueError("bounded Spite GlobalEvent source proof")
    event = capture["events"][0]
    data = copy.deepcopy(profiles[KEY])
    if data["kind"] != "Encounter":
        raise ValueError("Spite GlobalEvent requires its real Encounter")
    details = data["profile"]["details"]
    timer = {"name": "spite_global_fire", "duration_ms": {"min": event["interval_ms"], "max": event["interval_ms"]}, "repeat": True}
    anchors = [{"key": f"spite_global_fire_{index}", "location": {"kind": "point", "x": at[0], "y": at[1], "floor": at[2]}}
               for index, at in enumerate(event["positions"])]
    rule = {"key": RULE, "trigger": {"kind": "timer_elapsed", "timer": timer["name"]},
            "actions": [{"kind": "one_of", "branches": [
                {"weight": 1, "actions": [{"kind": "map_item", "operation": "create", "item": event["item"],
                  "anchor": anchor["key"], "interaction": "crystal:global-event/CreateGoshnarSpiteFire"}]}
                for anchor in anchors]}]}
    for collection, item, name in [(details.setdefault("state", {}).setdefault("timers", []), timer, "name"),
                                   *[(details["anchors"], anchor, "key") for anchor in anchors],
                                   (details["rules"], rule, "key")]:
        previous = [entry for entry in collection if entry[name] == item[name]]
        if previous and previous != [item]:
            raise ValueError("conflicting registered Spite loop DATA")
        if not previous:
            collection.append(item)
    profiles[KEY] = data


def tree_output(root):
    path = root / "content/encounters/definitions/encounters-00000-00107.json"
    document = json.loads(path.read_bytes())
    legacy = json.loads((root / "content/world/definitions/declarations.json").read_bytes())
    profiles = {(row["target"]["family"], row["target"]["key"], row["target"]["revision"]): row["data"]
                for row in legacy["authoring_profiles"]}
    apply(profiles, root)
    found = 0
    for row in document["records"]:
        identity = row["declaration"]["identity"]
        if ("Encounter", identity["key"], identity["revision"]) == KEY:
            row["authoring"] = profiles[KEY]
            found += 1
    if found != 1:
        raise ValueError("one real Spite-containing Encounter row required")
    return path, (json.dumps(document, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    path, raw = tree_output(root)
    if args.check:
        if path.read_bytes() != raw:
            raise SystemExit("registered Spite GlobalEvent Encounter drift")
    else:
        path.write_bytes(raw)
