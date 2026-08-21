#!/usr/bin/env python3
"""Build the Game-owned public semantic search-source snapshot for Atlas.

Legacy OTBM/Crystal inputs are accepted only at this offline Game/import boundary.
Atlas consumes the normalized output and must never reopen those inputs.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import sys
from typing import Any, Iterable

CONTRACT_ID = "oteryn-game-atlas-export-v1"
SEMANTIC_REVISION = 1
CAPABILITY = "semantic-search-source-v1"
PROFILE_ID = "oteryn-game-atlas-semantic-search-v1"
COORDINATE_PROFILE = "oteryn-world-spatial-v1"
LEGACY_IMPORT_PROFILE = "oteryn-crystalserver-legacy-spatial-import-v1"
LEGACY_REPOSITORY = "blakinio/Otheryn"
LEGACY_REPOSITORY_SHA = "e417c5e7c22986bf4acef0495eb47f7b72c97cce"
MAP_SHA256 = "3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034"
MAX_RECORDS = 250_000
ALLOWED_KINDS = {"npc", "monster", "town", "waypoint", "poi", "teleport", "house", "quest_area", "mechanic"}
FLOOR_ALIASES = {str(z): -z for z in range(16)}
NPC_NAME = re.compile(r'local\s+internalNpcName\s*=\s*["\']([^"\']+)["\']')


class ExportError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def stable_id(kind: str, *parts: object) -> str:
    payload = "\0".join((kind, *(str(part) for part in parts))).encode("utf-8")
    return f"{kind}:{hashlib.sha256(payload).hexdigest()[:32]}"


def _services_from_text(text: str) -> tuple[str, ...]:
    services: set[str] = set()
    if re.search(r"\bnpcConfig\.shop\s*=\s*\{", text):
        services.add("shop")
    if any(marker in text for marker in ("parseBank(", "parseBankMessages(", "NpcBankGreetCallback")):
        services.add("bank")
    if "parseGuildBank(" in text:
        services.add("guildBank")
    if "StdModule.travel" in text:
        services.add("travel")
    return tuple(sorted(services))


def service_index(npc_root: Path) -> dict[str, dict[str, Any]]:
    candidates: dict[str, list[tuple[str, ...]]] = {}
    for path in sorted(npc_root.rglob("*.lua"), key=lambda item: item.relative_to(npc_root).as_posix()):
        text = path.read_text(encoding="utf-8")
        match = NPC_NAME.search(text)
        if match is None:
            continue
        candidates.setdefault(match.group(1).casefold(), []).append(_services_from_text(text))
    result: dict[str, dict[str, Any]] = {}
    for key, values in candidates.items():
        unique = {value for value in values}
        if len(unique) == 1:
            result[key] = {"state": "RESOLVED", "services": list(next(iter(unique)))}
        else:
            result[key] = {"state": "AMBIGUOUS", "services": []}
    return result


def _load_fullworld_producer() -> Any:
    path = Path(__file__).resolve().parents[1] / "game-atlas-fullworld-source" / "producer.py"
    spec = importlib.util.spec_from_file_location("oteryn_game_atlas_fullworld_source_for_search", path)
    if spec is None or spec.loader is None:
        raise ExportError(f"unable to load Game full-world producer: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def extract_navigation(legacy_root: Path, map_path: Path) -> list[dict[str, Any]]:
    if sha256_file(map_path) != MAP_SHA256:
        raise ExportError("canonical world.otbm SHA-256 mismatch")
    fullworld = _load_fullworld_producer()
    bounded = fullworld._load_bounded_module()
    _legacy_assets, semantic = bounded._load_legacy_modules(legacy_root)
    records: list[dict[str, Any]] = []
    for source in semantic.iter_map_records(map_path, strict=True):
        if isinstance(source, semantic.Town):
            records.append({
                "kind": "town",
                "label": source.name,
                "position": {"x": int(source.temple.x), "y": int(source.temple.y), "floor": -int(source.temple.z)},
                "source_family": "town",
            })
        elif isinstance(source, semantic.Waypoint):
            records.append({
                "kind": "waypoint",
                "label": source.name,
                "position": {"x": int(source.position.x), "y": int(source.position.y), "floor": -int(source.position.z)},
                "source_family": "waypoint",
            })
    return records


def _base_record(kind: str, record_id: str, label: str, position: dict[str, int], *, capabilities: Iterable[str], provenance: dict[str, Any]) -> dict[str, Any]:
    if kind not in ALLOWED_KINDS:
        raise ExportError(f"unsupported semantic kind: {kind}")
    if not record_id or len(record_id) > 128 or not label or len(label) > 256:
        raise ExportError("semantic identity/label invalid")
    if set(position) != {"x", "y", "floor"} or not all(isinstance(position[key], int) for key in position):
        raise ExportError("semantic position invalid")
    return {
        "kind": kind,
        "id": record_id,
        "label": label,
        "aliases": [],
        "position": position,
        "bounds": None,
        "provenance": provenance,
        "capabilities": sorted(set(str(value) for value in capabilities)),
    }


def build_source(creature_source: dict[str, Any], npc_root: Path, navigation_records: Iterable[dict[str, Any]]) -> dict[str, Any]:
    if creature_source.get("contract_id") != CONTRACT_ID or creature_source.get("capability") != "static-creatures-v1":
        raise ExportError("unsupported Game static-creatures source")
    services = service_index(npc_root)
    records: list[dict[str, Any]] = []
    for family, kind in (("npcs", "npc"), ("monster_spawns", "monster")):
        values = creature_source.get(family)
        if not isinstance(values, list):
            raise ExportError(f"static-creatures source missing {family}")
        for source in values:
            record_id = source.get("record_id")
            label = source.get("name")
            position = source.get("position")
            if not isinstance(record_id, str) or not isinstance(label, str) or not isinstance(position, dict):
                raise ExportError("malformed static-creatures record")
            capability_values = ["static-placement"]
            service_state = None
            if kind == "npc":
                service = services.get(label.casefold())
                if service is not None:
                    service_state = service["state"]
                    if service_state == "RESOLVED":
                        capability_values.extend(service["services"])
            records.append(_base_record(
                kind,
                record_id,
                label,
                {"x": int(position["x"]), "y": int(position["y"]), "floor": int(position["floor"])},
                capabilities=capability_values,
                provenance={
                    "authority": "Oteryn/Oteryn-Game",
                    "source_capability": "static-creatures-v1",
                    "source_semantic_digest": creature_source.get("semantic_digest"),
                    "resolution_state": source.get("resolution_state", "UNKNOWN"),
                    "service_resolution_state": service_state,
                    "origin": source.get("origin"),
                },
            ))
    for source in navigation_records:
        kind = str(source.get("kind", ""))
        label = str(source.get("label", ""))
        position = source.get("position")
        if kind not in {"town", "waypoint"} or not isinstance(position, dict):
            raise ExportError("malformed navigation record")
        native = {"x": int(position["x"]), "y": int(position["y"]), "floor": int(position["floor"])}
        records.append(_base_record(
            kind,
            stable_id("semantic-record", kind, label.casefold(), native["x"], native["y"], native["floor"]),
            label,
            native,
            capabilities=("navigation", "overlay-point"),
            provenance={
                "authority": "Oteryn/Oteryn-Game",
                "identity_state": "UNRESOLVED",
                "source_family": source.get("source_family", kind),
                "legacy_repository": LEGACY_REPOSITORY,
                "legacy_repository_sha": LEGACY_REPOSITORY_SHA,
                "world_otbm_sha256": MAP_SHA256,
            },
        ))
    if len(records) > MAX_RECORDS:
        raise ExportError("semantic search source exceeds record cap")
    records.sort(key=lambda value: (value["label"].casefold(), value["kind"], value["position"]["floor"], value["position"]["y"], value["position"]["x"], value["id"]))
    ids = [record["id"] for record in records]
    if len(ids) != len(set(ids)):
        raise ExportError("duplicate semantic record id")
    output: dict[str, Any] = {
        "schema_version": 1,
        "contract_id": CONTRACT_ID,
        "semantic_revision": SEMANTIC_REVISION,
        "capability": CAPABILITY,
        "profile_id": PROFILE_ID,
        "coordinate_profile": COORDINATE_PROFILE,
        "legacy_import_profile": LEGACY_IMPORT_PROFILE,
        "input_floor_aliases": FLOOR_ALIASES,
        "records": records,
        "counts": {"records": len(records), "kinds": {kind: sum(record["kind"] == kind for record in records) for kind in sorted(ALLOWED_KINDS) if any(record["kind"] == kind for record in records)}},
    }
    output["semantic_digest"] = "sha256:" + hashlib.sha256(canonical_bytes(output)).hexdigest()
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--creatures", type=Path, required=True, help="Game static-creatures-v1 JSON")
    parser.add_argument("--npc-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--navigation-json", type=Path)
    parser.add_argument("--legacy-root", type=Path)
    parser.add_argument("--map-path", type=Path)
    args = parser.parse_args()
    creature_source = json.loads(args.creatures.read_text(encoding="utf-8"))
    if args.navigation_json is not None:
        navigation = json.loads(args.navigation_json.read_text(encoding="utf-8"))
        if not isinstance(navigation, list):
            raise ExportError("navigation JSON must be a list")
    else:
        if args.legacy_root is None or args.map_path is None:
            raise ExportError("provide --navigation-json or both --legacy-root and --map-path")
        navigation = extract_navigation(args.legacy_root, args.map_path)
    output = build_source(creature_source, args.npc_root, navigation)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"semantic_digest": output["semantic_digest"], **output["counts"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
