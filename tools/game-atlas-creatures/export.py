#!/usr/bin/env python3
"""Game-owned deterministic static NPC and monster/spawn Atlas projection.

Legacy Crystal XML/Lua is accepted only as migration evidence at this importer
boundary. Consumers receive normalized public-safe records and never legacy
paths as runtime authority.
"""
from __future__ import annotations

from dataclasses import asdict, dataclass
import hashlib
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET

CONTRACT_ID = "oteryn-game-atlas-export-v1"
CAPABILITY = "static-creatures-v1"
LEGACY_EVIDENCE_SHA = "e417c5e7c22986bf4acef0495eb47f7b72c97cce"

NPC_NAME = re.compile(r'local\s+internalNpcName\s*=\s*["\']([^"\']+)["\']')
NPC_OUTFIT = re.compile(r'npcConfig\.outfit\s*=\s*\{(.*?)\}', re.DOTALL)
MONSTER_NAME = re.compile(r'''Game\.createMonsterType\(\s*(?:"([^"]+)"|'([^']+)')\s*\)''')
MONSTER_OUTFIT = re.compile(r'\bmonster\.outfit\s*=\s*\{(.*?)\}', re.DOTALL)
VALUE = re.compile(r'\b(lookType|lookHead|lookBody|lookLegs|lookFeet|lookAddons)\s*=\s*(\d+)')


class ExportError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class Outfit:
    name: str
    look_type: int
    head: int = 0
    body: int = 0
    legs: int = 0
    feet: int = 0
    addons: int = 0

    @property
    def key(self) -> str:
        return f"{self.look_type}-{self.head}-{self.body}-{self.legs}-{self.feet}-{self.addons}"


def stable_id(kind: str, *parts: object) -> str:
    payload = "\0".join((kind, *(str(p) for p in parts))).encode("utf-8")
    return f"{kind}:{hashlib.sha256(payload).hexdigest()[:32]}"


def _parse_definition(path: Path, kind: str) -> tuple[str, Outfit] | None:
    text = path.read_text(encoding="utf-8")
    name_match = (NPC_NAME if kind == "npc" else MONSTER_NAME).search(text)
    if name_match is None:
        return None
    name = name_match.group(1) if kind == "npc" else (name_match.group(1) or name_match.group(2))
    block = (NPC_OUTFIT if kind == "npc" else MONSTER_OUTFIT).search(text)
    if block is None:
        return name, Outfit(name, 0)
    values = {key: int(value) for key, value in VALUE.findall(block.group(1))}
    return name, Outfit(name, values.get("lookType", 0), values.get("lookHead", 0), values.get("lookBody", 0), values.get("lookLegs", 0), values.get("lookFeet", 0), values.get("lookAddons", 0))


def definition_index(root: Path, kind: str) -> tuple[dict[str, Outfit], set[str]]:
    candidates: dict[str, list[Outfit]] = {}
    for path in sorted(root.rglob("*.lua"), key=lambda p: p.relative_to(root).as_posix()):
        parsed = _parse_definition(path, kind)
        if parsed is None:
            continue
        name, outfit = parsed
        candidates.setdefault(name.casefold(), []).append(outfit)
    resolved: dict[str, Outfit] = {}
    ambiguous: set[str] = set()
    for key, values in candidates.items():
        unique = {(v.look_type, v.head, v.body, v.legs, v.feet, v.addons): v for v in values}
        if len(unique) == 1:
            resolved[key] = next(iter(unique.values()))
        else:
            ambiguous.add(key)
    return resolved, ambiguous


def _origin(path: Path, world_root: Path) -> str:
    rel = path.relative_to(world_root).as_posix()
    if "/" not in rel:
        return "base-map"
    prefix = rel.split("/", 1)[0]
    return {"custom": "conditional-custom-map", "world_changes": "runtime-world-change", "annual_events": "annual-event-map", "quest": "quest-map"}.get(prefix, "UNKNOWN")


def _spawn_files(world_root: Path, kind: str) -> list[Path]:
    return sorted(world_root.rglob(f"*-{kind}.xml"), key=lambda p: p.relative_to(world_root).as_posix())


def parse_spawns(world_root: Path, kind: str) -> list[dict[str, object]]:
    records: list[dict[str, object]] = []
    expected = "monsters" if kind == "monster" else "npcs"
    for path in _spawn_files(world_root, kind):
        root = ET.parse(path).getroot()
        if root.tag != expected:
            raise ExportError(f"{path}: expected <{expected}>")
        for group in root:
            if group.tag != kind:
                raise ExportError(f"{path}: unexpected <{group.tag}>")
            required_group = {"centerx", "centery", "centerz", "radius"}
            if not required_group <= set(group.attrib):
                raise ExportError(f"{path}: incomplete {kind} group")
            cx, cy, cz, radius = (int(group.attrib[k]) for k in ("centerx", "centery", "centerz", "radius"))
            for ordinal, entry in enumerate(group):
                if entry.tag != kind or not {"name", "x", "y", "z", "spawntime"} <= set(entry.attrib):
                    raise ExportError(f"{path}: malformed {kind} spawn")
                x, y, z = cx + int(entry.attrib["x"]), cy + int(entry.attrib["y"]), int(entry.attrib["z"])
                name = entry.attrib["name"]
                records.append({"kind": kind, "name": name, "position": {"x": x, "y": y, "floor": -z}, "spawn_area": {"center": {"x": cx, "y": cy, "floor": -cz}, "radius": radius}, "spawn_time_seconds": int(entry.attrib["spawntime"]), "direction": int(entry.attrib["direction"]) if "direction" in entry.attrib else None, "weight": int(entry.attrib["weight"]) if "weight" in entry.attrib else None, "origin": _origin(path, world_root), "record_id": stable_id(kind, path.relative_to(world_root).as_posix(), ordinal, name, x, y, z)})
    return records


def export_creatures(world_root: Path, npc_root: Path, monster_root: Path) -> dict[str, object]:
    npc_defs, npc_ambiguous = definition_index(npc_root, "npc")
    monster_defs, monster_ambiguous = definition_index(monster_root, "monster")
    groups: dict[str, list[dict[str, object]]] = {"npcs": parse_spawns(world_root, "npc"), "monster_spawns": parse_spawns(world_root, "monster")}
    unresolved = 0
    ambiguous = 0
    for kind, key, defs, conflicts in (("npc", "npcs", npc_defs, npc_ambiguous), ("monster", "monster_spawns", monster_defs, monster_ambiguous)):
        for record in groups[key]:
            folded = str(record["name"]).casefold()
            outfit = defs.get(folded)
            if folded in conflicts:
                record["resolution_state"] = "AMBIGUOUS"
                ambiguous += 1
            elif outfit is None or outfit.look_type <= 0:
                record["resolution_state"] = "UNRESOLVED"
                unresolved += 1
            else:
                record["resolution_state"] = "RESOLVED"
                record["appearance"] = {"outfit_key": outfit.key, "look_type": outfit.look_type, "head": outfit.head, "body": outfit.body, "legs": outfit.legs, "feet": outfit.feet, "addons": outfit.addons}
                record["entity_id"] = stable_id(f"{kind}-entity", folded)
    for values in groups.values():
        values.sort(key=lambda r: (int(r["position"]["floor"]), int(r["position"]["y"]), int(r["position"]["x"]), str(r["name"]).casefold(), str(r["record_id"])))
    result = {"contract_id": CONTRACT_ID, "semantic_revision": 1, "capability": CAPABILITY, "coordinate_profile": "oteryn-native-floor-v1", "legacy_evidence": {"repository": "blakinio/Otheryn", "sha": LEGACY_EVIDENCE_SHA}, "npcs": groups["npcs"], "monster_spawns": groups["monster_spawns"], "statistics": {"npcs": len(groups["npcs"]), "monster_spawns": len(groups["monster_spawns"]), "unresolved": unresolved, "ambiguous": ambiguous}}
    canonical = json.dumps(result, ensure_ascii=False, separators=(",", ":"), sort_keys=True).encode("utf-8")
    result["semantic_digest"] = "sha256:" + hashlib.sha256(canonical).hexdigest()
    return result


def main() -> int:
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("world_root", type=Path); parser.add_argument("npc_root", type=Path); parser.add_argument("monster_root", type=Path); parser.add_argument("output", type=Path)
    args = parser.parse_args(); result = export_creatures(args.world_root, args.npc_root, args.monster_root)
    args.output.parent.mkdir(parents=True, exist_ok=True); args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"semantic_digest": result["semantic_digest"], **result["statistics"]}, sort_keys=True)); return 0


if __name__ == "__main__":
    raise SystemExit(main())
