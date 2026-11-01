"""W09: admit the Cube and its complete twenty-item pool to the native Item provider.

Crystal 201189f8 data/scripts/actions/items/drome_cube.lua:2-23 names the pool.
Physical, stacking and presentation facts are reused from the existing canonical
Item definitions and exact Crystal bindings, as for admit_consumable_profiles.
This is not a second inventory owner or a runtime fallback for missing content.
"""
from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path

CUBE_ID = 36827
REWARD_IDS = tuple(range(36723, 36743))
ITEM_IDS = (*REWARD_IDS, CUBE_ID)
KEYS = frozenset(f"oteryn:item.tibia.i{item}" for item in ITEM_IDS)
WORLD_PATH = "content/world/definitions/reference.json"
BINDINGS_PATH = "imports/crystalserver/bindings/items.json"


def _fail(key: str, reason: str) -> None:
    raise ValueError(f"DROME_ITEM_PROFILE_{reason}: {key}")


def _known(value: object, key: str, field: str) -> object:
    if not isinstance(value, dict) or value.get("state") != "KNOWN" or "value" not in value:
        _fail(key, f"UNKNOWN_{field}")
    return value["value"]


def _selected(rows: list[dict], field: str) -> dict[str, dict]:
    selected = {}
    for row in rows:
        key = row.get(field, {}).get("key")
        if key not in KEYS:
            continue
        if key in selected:
            _fail(key, f"DUPLICATE_{field.upper()}")
        selected[key] = row
    missing = KEYS - selected.keys()
    if missing:
        _fail(",".join(sorted(missing)), f"MISSING_{field.upper()}")
    return selected


def profiles(world: dict, bindings: dict) -> list[dict]:
    """Build only the closed W09 set; unknown or conflicting facts fail closed."""
    sources = _selected(world["records"], "identity")
    bound = _selected(bindings["bindings"], "target")
    result = []
    for item in ITEM_IDS:
        key = f"oteryn:item.tibia.i{item}"
        source, binding = sources[key], bound[key]
        identity = source["identity"]
        if (
            identity.get("family") != "Item"
            or not isinstance(identity.get("revision"), str)
            or not identity["revision"]
            or identity != binding["target"]
            or binding.get("disposition") != "EXACT"
            or source.get("materializable") is not True
        ):
            _fail(key, "IDENTITY_OR_BINDING")
        semantics = source["semantics"]
        physical = _known(semantics.get("physical"), key, "PHYSICAL")
        if not isinstance(physical, dict):
            _fail(key, "PHYSICAL_SHAPE")
        for field in ("movable", "pickupable"):
            if _known(physical.get(field), key, field.upper()) is not True:
                _fail(key, "NOT_PORTABLE")
        weight = _known(physical.get("weight"), key, "WEIGHT")
        if type(weight) is not int or not 0 <= weight <= 0xFFFF_FFFF:
            _fail(key, "WEIGHT_BOUND")
        if semantics.get("container") != {"state": "NOT_APPLICABLE"}:
            _fail(key, "NOT_A_LEAF")
        stack_class = source.get("stack_class")
        stack = _known(semantics.get("stack"), key, "STACK")
        if not isinstance(stack, dict):
            _fail(key, "STACK_SHAPE")
        stackable = _known(stack.get("stackable"), key, "STACKABLE")
        if stack_class == "StackCapable":
            maximum = _known(stack.get("stack_max"), key, "STACK_MAXIMUM")
            if stackable is not True or type(maximum) is not int or not 1 <= maximum <= 0xFFFF_FFFF:
                _fail(key, "STACK_BOUND")
        elif stack_class != "NonStackable" or stackable is not False:
            _fail(key, "STACK_CLASS")
        presentation = _known(semantics.get("presentation"), key, "PRESENTATION")
        if not isinstance(presentation, dict):
            _fail(key, "PRESENTATION_SHAPE")
        name = _known(presentation.get("name"), key, "NAME")
        if not isinstance(name, str) or not name or len(f"You have received a {name}!".encode()) > 255:
            _fail(key, "NAME_BOUND")
        result.append({
            "authoring": {"item": copy.deepcopy(identity)},
            "semantics": copy.deepcopy(semantics),
            "attributes": {},
            "admission": {
                "materializable": True,
                "stack_class": stack_class,
                "legal_destinations": ["CharacterInventory", "Ground"],
            },
            "production_definition": {
                "family": "Item", "production_key": key, "revision_ref": identity["revision"],
            },
            "production_binding": copy.deepcopy(binding),
            "production_binding_qualification": None,
        })
    return result


def admit(root: Path, generated: dict[str, bytes], profile_path: str,
          manifest_path: str, order, canonical) -> None:
    """Join the existing registration pipeline, preserving an agreeing richer profile."""
    world = json.loads((root / WORLD_PATH).read_bytes())
    bindings = json.loads((root / BINDINGS_PATH).read_bytes())
    incoming = profiles(world, bindings)
    document = json.loads(generated[profile_path])
    present = {}
    for record in document["records"]:
        key = record["production_definition"]["production_key"]
        if key in KEYS:
            if key in present:
                _fail(key, "DUPLICATE_NATIVE")
            present[key] = record
    additions = []
    for record in incoming:
        key = record["production_definition"]["production_key"]
        existing = present.get(key)
        if existing is None:
            additions.append(record)
            continue
        # An equipment/consumption overlay may already own this identity. Do not
        # replace its richer attributes or weaken its existing custody facts.
        if (
            any(existing.get(field) != record[field] for field in
                ("production_definition", "production_binding", "admission"))
            or any(existing["semantics"].get(field) != record["semantics"][field]
                   for field in ("physical", "stack", "container", "presentation"))
        ):
            _fail(key, "NATIVE_CONFLICT")
    document["records"] = sorted(document["records"] + additions, key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(raw).hexdigest()
    manifest_raw = canonical(manifest)
    # Neither generated value is modified until the complete closed set validates.
    generated[profile_path] = raw
    generated[manifest_path] = manifest_raw
