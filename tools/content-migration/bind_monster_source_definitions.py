"""Bind the canonical monster definition closure into the existing pinned native input.

The adopted overlay stays immutable. This derived successor changes no source
values: its SHA is independently retained in the loader-decoded artifact.
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
    explicit_reference_levels(creatures)
    creatures["source_definitions_sha256"] = definition_digest(root)
    result[CREATURES] = canonical(creatures) + b"\n"
    manifest = json.loads(result[MANIFEST])
    manifest["creature_profiles"]["sha256"] = hashlib.sha256(result[CREATURES]).hexdigest()
    result[MANIFEST] = canonical(manifest) + b"\n"
    return result
