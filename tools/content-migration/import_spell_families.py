#!/usr/bin/env python3
"""Import qualified r25 spell authoring data without changing legacy WorldProject records.

The complete catalog remains byte-identical for NativeGameplayInput. Dependency
collections retain candidate identities and authoring schemas; they are explicit
supplements, never legacy ProjectV2 shards or permission to activate a generation.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
PACK = "content/test-packs/spells/r25"
STATUS_SHA256 = "df83fe54bd8be56197493a0caa65213e292e9074690c1f9f8bdaf2f0a478d203"
MANIFEST_SHA256 = "8994bbe0fa04cf78bc6f2af400888dcee4c1f0c1f73eb57527b75c35ad7fcaf9"
CATALOG = "content/abilities/definitions/player-spells.json"
SELECTION = "content/abilities/definitions/player-spell-selection.json"
NATIVE_MANIFEST = "content/spells.manifest.json"
CUES_SOURCE = "tools/content-schema/native-gameplay/spell-cues.json"
CUES_DESTINATION = "content/presentations/bindings/spell-cues.json"
CUES_SHA256 = "3f3ca7f837e0865e04e75e42bff9317953dac0f09300439596847cdae4503362"
PROVIDER_DESTINATIONS = {
    "catalog": CATALOG,
    "source_selection": SELECTION,
    "creature_profiles": "content/creatures/definitions/spell-native-profiles.json",
    "presentation_profiles": "content/presentations/definitions/spell-native-profiles.json",
    "item_profiles": "content/items/definitions/spell-native-profiles.json",
    "spell_appearances": "content/presentations/bindings/spell-appearances.json",
    "build_training": "content/abilities/definitions/spell-build-training.json",
    "familiar_config": "content/creatures/definitions/spell-familiar-config.json",
    "familiar_defenses": "content/creatures/definitions/spell-familiar-defenses.json",
    "wheel_profile": "rulesets/progression/wheel-of-destiny/spell-profile.json",
    "source_world": "imports/spells/r25/source-world.json",
}
PROVIDER_FAMILIES = {
    "creature_profiles": "Creature", "presentation_profiles": "Presentation",
    "item_profiles": "Item", "spell_appearances": "Presentation",
    "build_training": "Ability", "familiar_config": "Creature",
    "familiar_defenses": "Creature", "wheel_profile": "Ability",
}
NATIVE_SCHEMA_CONTRACTS = {
    "creature_profiles": "apps/game-server/src/content/native_gameplay.rs#CreatureProfilesDocument",
    "presentation_profiles": "apps/game-server/src/content/native_gameplay.rs#PresentationProfilesDocument",
    "item_profiles": "apps/game-server/src/content/native_gameplay.rs#ItemProfilesDocument",
    "spell_appearances": "apps/game-server/src/content/native_spell_appearances.rs#SpellAppearancesDocument",
    "build_training": "apps/game-server/src/spell/mana_training.rs",
    "familiar_config": "apps/game-server/src/content/spell_familiar_config.rs",
    "familiar_defenses": "apps/game-server/src/content/spell_familiar_defenses.rs",
    "wheel_profile": "apps/game-server/src/content/spell_wheel_profile.rs",
}
DEPENDENCIES = {
    "Ability": ("abilities", "content/abilities/definitions/player-spell-abilities.json"),
    "Effect": ("effects", "content/abilities/effects/player-spell-effects.json"),
    "Formula": ("formulas", "content/abilities/formulas/player-spell-formulas.json"),
}
SCHEMA_ROOT = "tools/content-schema/spell-authoring/"
COLLECTION_SCHEMA = SCHEMA_ROOT + "spell-family-import.schema.json"
SCHEMA_REFS = {
    "bundle": SCHEMA_ROOT + "spell.schema.json",
    "dependencies": SCHEMA_ROOT + "spell-dependencies.schema.json",
    "manifest": "tools/content-schema/monster-authoring/monster-import-readiness.schema.json",
}


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def pinned_file(root: Path, path: str, expected: str) -> bytes:
    pack = (root / PACK).resolve()
    target = (pack / path).resolve()
    if not target.is_relative_to(pack):
        raise ValueError(f"INPUT_PATH_ESCAPE:{path}")
    data = target.read_bytes()
    if digest(data) != expected:
        raise ValueError(f"INPUT_DIGEST_MISMATCH:{path}")
    return data


def verified_inputs(root: Path = ROOT) -> tuple[dict[str, bytes], dict[str, bytes]]:
    status = json.loads(pinned_file(root, "import-status.json", STATUS_SHA256))
    manifest = json.loads(pinned_file(root, "manifest.json", MANIFEST_SHA256))
    runtime: dict[str, bytes] = {}
    for binding in status["runtime_inputs"]:
        key = binding["key"]
        if key in runtime or manifest.get(key, {}).get("path") != binding["path"] or \
                manifest[key]["sha256"] != binding["sha256"]:
            raise ValueError(f"RUNTIME_BINDING_MISMATCH:{key}")
        runtime[key] = pinned_file(root, binding["path"], binding["sha256"])
    if len(runtime) != 11 or {key for key, value in manifest.items() if isinstance(value, dict)} != set(runtime):
        raise ValueError("RUNTIME_INPUT_COVERAGE")
    sidecars: dict[str, bytes] = {}
    for binding in status["source_sidecars"]:
        path = binding["path"]
        if path in sidecars or not path.startswith("sidecars/"):
            raise ValueError(f"SIDECAR_BINDING_MISMATCH:{path}")
        sidecars[path] = pinned_file(root, path, binding["sha256"])
    if len(sidecars) != 7:
        raise ValueError("SIDECAR_INPUT_COVERAGE")
    return runtime, sidecars


def spell_validator(root: Path):
    spec = importlib.util.spec_from_file_location("canonical_import_spell_validator", root / SCHEMA_ROOT / "validate_spell.py")
    if spec is None or spec.loader is None:
        raise ValueError("SPELL_VALIDATOR_UNAVAILABLE")
    validator = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(validator)
    return validator


def validate_catalog(root: Path, data: bytes) -> dict[str, Any]:
    catalog = json.loads(data)
    if catalog.get("schema") != "OTERYN_EXECUTABLE_SPELL_CATALOG/v1" or len(catalog["bundles"]) != 246:
        raise ValueError("SPELL_CATALOG_COVERAGE")
    validator = spell_validator(root)
    identities = set()
    for entry in catalog["bundles"]:
        spell = entry["bundle"]["spell"]
        identity = (spell["identity"]["key"], spell["identity"]["revision"])
        if identity in identities:
            raise ValueError(f"DUPLICATE_SPELL_IDENTITY:{identity}")
        identities.add(identity)
        errors = validator.validate(entry["bundle"], entry["dependencies"], entry["catalog"], entry["manifest"])
        if errors:
            raise ValueError(f"INVALID_SPELL:{identity}:{errors}")
    return catalog


def outputs(root: Path = ROOT) -> dict[str, bytes]:
    """Return every planned byte only after all pinned inputs and bundles validate."""
    runtime, sidecars = verified_inputs(root)
    catalog = validate_catalog(root, runtime["catalog"])
    selection = json.loads(runtime["source_selection"])
    if selection["catalog_sha256"] != digest(runtime["catalog"]):
        raise ValueError("SOURCE_SELECTION_CATALOG_MISMATCH")
    result = {path: runtime[key] for key, path in PROVIDER_DESTINATIONS.items()}
    manifest = json.loads(pinned_file(root, "manifest.json", MANIFEST_SHA256))
    for key, path in PROVIDER_DESTINATIONS.items():
        manifest[key]["path"] = Path(os.path.relpath(path, "content")).as_posix()
    result[NATIVE_MANIFEST] = canonical_bytes(manifest)
    cues = (root / CUES_SOURCE).read_bytes()
    if digest(cues) != CUES_SHA256:
        raise ValueError("CUE_REGISTRY_DIGEST_MISMATCH")
    result[CUES_DESTINATION] = cues
    for family, (section, path) in DEPENDENCIES.items():
        by_identity: dict[tuple[str, str], dict[str, Any]] = {}
        for entry in catalog["bundles"]:
            for record in entry["dependencies"][section]:
                identity = (record["identity"]["key"], record["identity"]["revision"])
                if identity in by_identity and by_identity[identity] != record:
                    raise ValueError(f"CONFLICTING_DEPENDENCY:{family}:{identity}")
                by_identity[identity] = record
        records = [by_identity[key] for key in sorted(by_identity)]
        result[path] = canonical_bytes({
            "schema": "OTERYN_SPELL_DEPENDENCY_COLLECTION/v1",
            "family": family,
            "representation": "spell_authoring_supplement",
            "schema_refs": {"records": SCHEMA_REFS["dependencies"] + f"#/properties/{section}/items"},
            "source_catalog": {"path": CATALOG, "sha256": digest(runtime["catalog"])},
            "runtime_activation": False,
            "record_count": len(records),
            "records": records,
        })
    for path, data in sidecars.items():
        result["imports/spells/r25/" + Path(path).name] = data
    validator = spell_validator(root)
    schema = json.loads((root / COLLECTION_SCHEMA).read_bytes())
    registry = validator.REGISTRY.with_resource(schema["$id"], validator.Resource.from_contents(schema))
    collection_validator = validator.Draft202012Validator(schema, registry=registry)
    for path in [CATALOG, *(path for _, path in DEPENDENCIES.values())]:
        errors = list(collection_validator.iter_errors(json.loads(result[path])))
        if errors:
            raise ValueError(f"INVALID_FAMILY_COLLECTION:{path}:{errors[0].message}")
    return result


def descriptors(root: Path = ROOT, generated: dict[str, bytes] | None = None) -> dict[str, list[dict[str, Any]]]:
    """Family index supplements, separate from legacy shards and their counts."""
    data = outputs(root) if generated is None else generated
    result: dict[str, list[dict[str, Any]]] = {}
    for family, (_, path) in DEPENDENCIES.items():
        document = json.loads(data[path])
        result[family] = [{
            "path": path, "sha256": digest(data[path]),
            "record_count": document["record_count"],
            "schema_refs": {**document["schema_refs"], "collection": COLLECTION_SCHEMA},
            "representation": "spell_authoring_supplement", "runtime_activation": False,
        }]
    result["Ability"].insert(0, {
        "path": CATALOG, "sha256": digest(data[CATALOG]), "record_count": 246,
        "schema_refs": {**SCHEMA_REFS, "collection": COLLECTION_SCHEMA},
        "source_selection": {"path": SELECTION, "sha256": digest(data[SELECTION])},
        "representation": "complete_spell_catalog", "runtime_activation": False,
    })
    for key, family in PROVIDER_FAMILIES.items():
        path = PROVIDER_DESTINATIONS[key]
        document = json.loads(data[path])
        descriptor = {
            "path": path, "sha256": digest(data[path]),
            "schema": document["schema"],
            "schema_contract": {"kind": "rust_serde", "path": NATIVE_SCHEMA_CONTRACTS[key]},
            "representation": "native_profile_overlay",
            "creates_canonical_identities": False,
            "runtime_activation": False,
        }
        if isinstance(document.get("records"), list):
            descriptor["record_count"] = len(document["records"])
        result.setdefault(family, []).append(descriptor)
    result.setdefault("Presentation", []).append({
        "path": CUES_DESTINATION, "sha256": digest(data[CUES_DESTINATION]),
        "schema": "OTERYN_SOURCE_SPELL_CUES/v1",
        "schema_contract": {"kind": "source_validator", "path": "tools/content-schema/native-gameplay/test_spell_cues.py"},
        "record_count": len(json.loads(data[CUES_DESTINATION])["records"]),
        "representation": "source_cue_alias_bindings",
        "creates_canonical_identities": False, "runtime_activation": False,
    })
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["content"])
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    planned = outputs()
    if args.check:
        mismatches = [path for path, data in planned.items() if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != data]
        if mismatches:
            raise ValueError("CANONICAL_IMPORT_MISMATCH:" + ",".join(mismatches))
    else:
        for path, data in planned.items():
            target = ROOT / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
    print(json.dumps({"files": len(planned), "checked": args.check, "runtime_activation": False}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
