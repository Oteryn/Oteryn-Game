"""Bind the canonical monster definition closure into the existing pinned native input.

The adopted overlay stays immutable. This derived successor carries official
balance overrides from the canonical definitions; its SHA is independently retained in the loader-decoded artifact.
World/channel authority is still checked separately at runtime.
"""
from __future__ import annotations
import hashlib
import json
from pathlib import Path

CREATURES = "content/creatures/definitions/spell-native-profiles.json"
MANIFEST = "content/spells.manifest.json"

def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()

def definition_digest(root: Path):
    reference = json.loads((root / "content/world/definitions/reference.json").read_bytes())
    declarations = json.loads((root / "content/world/definitions/declarations.json").read_bytes())
    provenance = json.loads((root / "content/world/provenance/sources.json").read_bytes())
    closure = {
        "schema": "OTERYN_NATIVE_MONSTER_DEFINITIONS/v1",
        "records": reference["records"],
        "authoring_profiles": declarations["authoring_profiles"],
        "declarations": declarations["records"],
        "sources": provenance["sources"],
        "source_identity_bindings": provenance["source_identity_bindings"],
    }
    return hashlib.sha256(canonical(closure)).hexdigest()

def explicit_reference_levels(value):
    """Materialize the nullable MRL default without changing any other source fact."""
    if isinstance(value, dict):
        if value.get("kind") == "Creature" and "profile" in value:
            value["profile"]["details"].setdefault("reference_level", None)
        for child in value.values():
            explicit_reference_levels(child)
    elif isinstance(value, list):
        for child in value:
            explicit_reference_levels(child)


def bind_definitions(root: Path, generated: dict[str, bytes]):
    result = dict(generated)
    creatures = json.loads(result[CREATURES])
    ledger = root / "content/creatures/official-balance.json"
    if ledger.is_file():
        declarations = json.loads((root / "content/world/definitions/declarations.json").read_bytes())
        canonical_profiles = {row["target"]["key"]: row["data"]["profile"]
                              for row in declarations["authoring_profiles"] if row["target"]["family"] == "Creature"}
        del declarations
        native_profiles = {row["profile"]["target"]["key"]: row["profile"]["data"]["profile"]
                           for row in creatures["records"]}
        for override in json.loads(ledger.read_bytes())["overrides"]:
            key = "oteryn:creature." + override["creature"]
            source, target = canonical_profiles[key], native_profiles[key]
            if override["field"] == "experience":
                target["experience"] = source["experience"]
            else:
                element = override["field"].removesuffix("_sensitivity")
                resistance = next(row for row in source["resistances"] if row["damage_type"] == element)
                native = next(row for row in target["resistances"] if row["damage_type"] == element)
                native["percent"] = resistance["percent"]
        for override in json.loads(ledger.read_bytes()).get("reference_overrides", []):
            key = "oteryn:creature." + override["creature"]
            native_profiles[key][override["field"]] = canonical_profiles[key][override["field"]]
    explicit_reference_levels(creatures)
    creatures["source_definitions_sha256"] = definition_digest(root)
    result[CREATURES] = canonical(creatures) + b"\n"
    manifest = json.loads(result[MANIFEST])
    manifest["creature_profiles"]["sha256"] = hashlib.sha256(result[CREATURES]).hexdigest()
    result[MANIFEST] = canonical(manifest) + b"\n"
    return result
