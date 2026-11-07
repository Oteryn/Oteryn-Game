"""Project official ground while preserving an existing exact canonical Item binding."""

import json

import summer_ground

ROOT = summer_ground.ROOT
PROFILE = "OTERYN_OFFICIAL_CLIENT_BOUND_ITEM_GROUND/v1"


def record(root=ROOT, *, identifier, expected_speed):
    """No binding, canonical Item, custody fact or generated file is changed."""
    bindings = json.loads((root / "imports/crystalserver/bindings/items.json").read_bytes())
    matches = [r for r in bindings["bindings"] if r["external_id"] == str(identifier)]
    target = {"family": "Item", "key": f"oteryn:item.tibia.i{identifier}",
              "revision": "definition-r1"}
    if (len(matches) != 1 or matches[0]["disposition"] != "EXACT"
            or matches[0]["target"] != target):
        raise ValueError("bound ground requires one exact Item binding")
    definitions = [r for path in sorted((root / "content/items/definitions").glob("items-*.json"))
                   for r in json.loads(path.read_bytes())["records"]
                   if r["definition"]["identity"]["key"] == target["key"]]
    if len(definitions) != 1 or definitions[0]["definition"]["identity"] != target:
        raise ValueError("bound ground canonical Item identity differs")
    result = summer_ground.official_ground_record(root, identifier, expected_speed)
    result["provenance"]["source"]["profile"] = PROFILE
    result["provenance"]["item_pointer"] = target
    result["provenance"]["route"]["reason"] = "official_ground_preserving_item_binding"
    return result


def validate_record(observed):
    try:
        expected = record(identifier=observed["provenance"]["source_item_id"],
                          expected_speed=observed["ground_speed"]["value"])
    except (KeyError, ValueError, StopIteration) as error:
        return [f"bound ground source admission: {error}"]
    return [] if observed == expected else ["bound ground differs from its exact official source"]
