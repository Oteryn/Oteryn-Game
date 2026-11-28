"""Admit existing Global receipt definitions as native readable items."""

from copy import deepcopy
import json

DEFINITIONS = "content/items/definitions/items-11000-11499.json"
KEYS = {
    "oteryn:item.tibia.i21932",  # Fandom archive, Receipt (Success), 2023-07-15
    "oteryn:item.tibia.i21933",  # Fandom archive, Receipt (Fail), 2023-07-15
}


def admit(root, generated, profile_path, order, canonical):
    shards = json.loads(
        generated[DEFINITIONS] if DEFINITIONS in generated else (root / DEFINITIONS).read_bytes()
    )
    source = {
        row["definition"]["identity"]["key"]: row["definition"]
        for row in shards["records"]
        if row["definition"]["identity"]["key"] in KEYS
    }
    if source.keys() != KEYS:
        raise ValueError("Guild Bank receipt definitions are incomplete")
    # Reuse the repository's exact Item identity bindings. Behaviour and
    # physical facts come from the Global archive and its canonical definitions.
    bindings = {
        binding["target"]["key"]: binding
        for binding in json.loads((root / "imports/crystalserver/bindings/items.json").read_bytes())[
            "bindings"
        ]
        if binding["target"]["key"] in KEYS and binding["disposition"] == "EXACT"
    }
    profiles = json.loads(generated[profile_path])
    if any(row["production_definition"]["production_key"] in KEYS for row in profiles["records"]):
        raise ValueError("Guild Bank receipt already has a native Item profile")
    for key in sorted(KEYS):
        definition = source[key]
        identity = definition["identity"]
        binding = bindings[key]
        if binding["target"] != identity:
            raise ValueError("Guild Bank receipt Item identity drift")
        semantics = deepcopy(definition["semantics"])
        semantics["readable_writeable"] = {
            "state": "KNOWN",
            "value": {
                "readable": {"state": "KNOWN", "value": True},
                "writeable": {"state": "UNKNOWN"},
                "distance_read": {"state": "UNKNOWN"},
                "max_text_length": {"state": "UNKNOWN"},
                "write_once_target": {"state": "UNKNOWN"},
            },
        }
        profiles["records"].append(
            {
                "admission": {
                    "legal_destinations": ["CharacterInventory", "Ground"],
                    "materializable": True,
                    "stack_class": "NonStackable",
                },
                "attributes": {},
                "authoring": {"item": identity},
                "production_binding": binding,
                "production_definition": {
                    "family": "Item",
                    "production_key": key,
                    "revision_ref": identity["revision"],
                },
                "semantics": semantics,
            }
        )
    profiles["records"].sort(key=order)
    generated[profile_path] = canonical(profiles)
