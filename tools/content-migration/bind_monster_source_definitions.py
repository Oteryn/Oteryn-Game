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

def bind_definitions(root: Path, generated: dict[str, bytes]):
    result = dict(generated)
    creatures = json.loads(result[CREATURES])
    creatures["source_definitions_sha256"] = definition_digest(root)
    result[CREATURES] = canonical(creatures) + b"\n"
    manifest = json.loads(result[MANIFEST])
    manifest["creature_profiles"]["sha256"] = hashlib.sha256(result[CREATURES]).hexdigest()
    result[MANIFEST] = canonical(manifest) + b"\n"
    return result
