#!/usr/bin/env python3
"""Game-owned deterministic Atlas appearance animation metadata producer.

This module is the authoritative conversion boundary for the exact 15.32
appearance source selected by Oteryn-Game.  It accepts proprietary/legacy
bytes only offline, validates their immutable identity, and publishes a
normalized metadata product that Atlas can consume without parsing DAT/SPR or
reimplementing Tibia animation/outfit heuristics.
"""
from __future__ import annotations

from dataclasses import dataclass
from collections import Counter
import argparse
import hashlib
import json
from pathlib import Path
import zipfile
from typing import Any, Iterator

CAPABILITY = "animated-appearances-v1"
CONTRACT_ID = "oteryn-game-atlas-animated-appearances-v1"
SEMANTIC_REVISION = 1
SOURCE_LABEL = "15.32"
SOURCE_PROFILE_ID = "oteryn-atlas-15-32-appearance-spatial-v1"
SOURCE_DRIVE_FILE_ID = "1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv"
SOURCE_ZIP_SHA256 = "1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f"
SOURCE_CATALOG_SHA256 = "35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85"
SOURCE_APPEARANCE_SHA256 = "dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075"
SOURCE_APPEARANCE_NAME = f"assets/appearances-{SOURCE_APPEARANCE_SHA256}.dat"
SOURCE_CATALOG_NAME = "assets/catalog-content.json"
MIGRATION_EVIDENCE_REPOSITORY = "blakinio/Otheryn"
MIGRATION_EVIDENCE_SHA = "e417c5e7c22986bf4acef0495eb47f7b72c97cce"

CATEGORY_FIELDS = {"object": 1, "outfit": 2, "effect": 3, "missile": 4}
FRAME_GROUP_SEMANTICS = {0: "outfit-idle", 1: "outfit-moving", 2: "object-initial"}
LOOP_TYPES = {-1: "pingpong", 0: "infinite", 1: "counted"}
UINT64_MAX = (1 << 64) - 1
MAX_APPEARANCES = 100_000
MAX_FRAME_GROUPS_PER_APPEARANCE = 16
MAX_PATTERN_DIMENSION = 64
MAX_LAYERS = 16
MAX_PHASES = 1024
MAX_SPRITE_REFS_PER_GROUP = 2_000_000
MAX_CATALOG_ENTRIES = 20_000


class ProductError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class Phase:
    duration_min_ms: int
    duration_max_ms: int


@dataclass(frozen=True, slots=True)
class FrameGroup:
    fixed_frame_group: int
    group_id: int
    pattern_width: int
    pattern_height: int
    pattern_depth: int
    layers: int
    sprite_ids: tuple[int, ...]
    phases: tuple[Phase, ...]
    default_start_phase: int
    synchronized: bool
    random_start_phase: bool
    loop_type: int
    loop_count: int

    @property
    def phase_count(self) -> int:
        return len(self.phases) if self.phases else 1


@dataclass(frozen=True, slots=True)
class Appearance:
    appearance_id: int
    frame_groups: tuple[FrameGroup, ...]


def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def _varint(data: bytes, offset: int) -> tuple[int, int]:
    value = 0
    shift = 0
    start = offset
    while offset < len(data) and shift < 70:
        byte = data[offset]
        offset += 1
        value |= (byte & 0x7F) << shift
        if byte < 0x80:
            return value, offset
        shift += 7
    raise ProductError(f"invalid protobuf varint at offset {start}")


def _fields(data: bytes) -> Iterator[tuple[int, int, int | bytes]]:
    offset = 0
    while offset < len(data):
        key, offset = _varint(data, offset)
        field, wire = key >> 3, key & 7
        if field <= 0:
            raise ProductError("invalid protobuf field zero")
        if wire == 0:
            value, offset = _varint(data, offset)
        elif wire == 1:
            if offset + 8 > len(data):
                raise ProductError("truncated fixed64 field")
            value = data[offset:offset + 8]
            offset += 8
        elif wire == 2:
            size, offset = _varint(data, offset)
            if size < 0 or offset + size > len(data):
                raise ProductError("truncated length-delimited field")
            value = data[offset:offset + size]
            offset += size
        elif wire == 5:
            if offset + 4 > len(data):
                raise ProductError("truncated fixed32 field")
            value = data[offset:offset + 4]
            offset += 4
        else:
            raise ProductError(f"unsupported protobuf wire type {wire}")
        yield field, wire, value


def _values(data: bytes) -> dict[int, list[int | bytes]]:
    result: dict[int, list[int | bytes]] = {}
    for field, _wire, value in _fields(data):
        result.setdefault(field, []).append(value)
    return result


def _first_int(values: dict[int, list[int | bytes]], field: int, default: int = 0) -> int:
    entries = values.get(field)
    if entries and isinstance(entries[0], int):
        return int(entries[0])
    return default


def _first_bytes(values: dict[int, list[int | bytes]], field: int) -> bytes | None:
    entries = values.get(field)
    if entries and isinstance(entries[0], bytes):
        return entries[0]
    return None


def _normalize_loop_type(raw: int) -> int:
    if raw == UINT64_MAX:
        return -1
    if raw in LOOP_TYPES:
        return raw
    raise ProductError(f"unsupported animation loop type {raw}")


def _decode_frame_group(payload: bytes) -> FrameGroup:
    values = _values(payload)
    sprite_payload = _first_bytes(values, 3)
    if sprite_payload is None:
        raise ProductError("frame group has no sprite_info")
    sprite = _values(sprite_payload)
    width = _first_int(sprite, 1, 1)
    height = _first_int(sprite, 2, 1)
    depth = _first_int(sprite, 3, 1)
    layers = _first_int(sprite, 4, 1)
    for label, value in (("pattern_width", width), ("pattern_height", height), ("pattern_depth", depth)):
        if value <= 0 or value > MAX_PATTERN_DIMENSION:
            raise ProductError(f"{label} out of bounds: {value}")
    if layers <= 0 or layers > MAX_LAYERS:
        raise ProductError(f"layers out of bounds: {layers}")
    sprite_ids = tuple(int(value) for value in sprite.get(5, ()) if isinstance(value, int))
    if len(sprite_ids) > MAX_SPRITE_REFS_PER_GROUP:
        raise ProductError("sprite reference cap exceeded")

    animation_payload = _first_bytes(sprite, 6)
    phases: tuple[Phase, ...]
    default_start = 0
    synchronized = False
    random_start = False
    loop_type = 0
    loop_count = 0
    if animation_payload is None:
        phases = ()
    else:
        animation = _values(animation_payload)
        phase_values = [value for value in animation.get(6, ()) if isinstance(value, bytes)]
        if len(phase_values) > MAX_PHASES:
            raise ProductError("animation phase cap exceeded")
        decoded_phases: list[Phase] = []
        for phase_payload in phase_values:
            phase = _values(phase_payload)
            low = _first_int(phase, 1, 0)
            high = _first_int(phase, 2, low)
            if low > high:
                raise ProductError(f"malformed animation timing {low}>{high}")
            decoded_phases.append(Phase(low, high))
        phases = tuple(decoded_phases)
        default_start = _first_int(animation, 1, 0)
        synchronized = bool(_first_int(animation, 2, 0))
        random_start = bool(_first_int(animation, 3, 0))
        loop_type = _normalize_loop_type(_first_int(animation, 4, 0))
        loop_count = _first_int(animation, 5, 0)

    phase_count = len(phases) if phases else 1
    if not 0 <= default_start < phase_count:
        raise ProductError(f"default start phase {default_start} outside phase count {phase_count}")
    expected = width * height * depth * layers * phase_count
    if len(sprite_ids) != expected:
        raise ProductError(f"sprite reference count mismatch: expected {expected}, got {len(sprite_ids)}")
    if loop_type != 1 and loop_count not in (0,):
        raise ProductError("loop_count is only valid for counted animations")
    if loop_type == 1 and phase_count > 1 and loop_count <= 0:
        raise ProductError("counted animation requires positive loop_count")

    return FrameGroup(
        fixed_frame_group=_first_int(values, 1, 0),
        group_id=_first_int(values, 2, 0),
        pattern_width=width,
        pattern_height=height,
        pattern_depth=depth,
        layers=layers,
        sprite_ids=sprite_ids,
        phases=phases,
        default_start_phase=default_start,
        synchronized=synchronized,
        random_start_phase=random_start,
        loop_type=loop_type,
        loop_count=loop_count,
    )


def decode_category(appearance_bytes: bytes, category: str) -> tuple[Appearance, ...]:
    try:
        category_field = CATEGORY_FIELDS[category]
    except KeyError as exc:
        raise ProductError(f"unknown category {category}") from exc
    appearances: list[Appearance] = []
    seen: set[int] = set()
    for field, wire, value in _fields(appearance_bytes):
        if field != category_field or wire != 2 or not isinstance(value, bytes):
            continue
        values = _values(value)
        appearance_id = _first_int(values, 1, 0)
        if appearance_id <= 0:
            raise ProductError(f"invalid {category} appearance id {appearance_id}")
        if appearance_id in seen:
            raise ProductError(f"duplicate {category} appearance id {appearance_id}")
        groups = tuple(_decode_frame_group(group) for group in values.get(2, ()) if isinstance(group, bytes))
        if not groups:
            raise ProductError(f"{category} appearance {appearance_id} has no frame groups")
        if len(groups) > MAX_FRAME_GROUPS_PER_APPEARANCE:
            raise ProductError(f"{category} appearance {appearance_id} frame-group cap exceeded")
        appearances.append(Appearance(appearance_id, groups))
        seen.add(appearance_id)
        if len(appearances) > MAX_APPEARANCES:
            raise ProductError(f"{category} appearance cap exceeded")
    return tuple(appearances)


def _catalog_sprite_ranges(catalog_bytes: bytes) -> tuple[tuple[int, int, int, str], ...]:
    try:
        catalog = json.loads(catalog_bytes)
    except json.JSONDecodeError as exc:
        raise ProductError(f"invalid source catalog JSON: {exc}") from exc
    if not isinstance(catalog, list) or len(catalog) > MAX_CATALOG_ENTRIES:
        raise ProductError("invalid source catalog shape/count")
    result: list[tuple[int, int, int, str]] = []
    for entry in catalog:
        if not isinstance(entry, dict) or entry.get("type") != "sprite":
            continue
        try:
            first = int(entry["firstspriteid"])
            last = int(entry["lastspriteid"])
            sprite_type = int(entry["spritetype"])
            file_name = str(entry["file"])
        except (KeyError, TypeError, ValueError) as exc:
            raise ProductError("malformed sprite catalog entry") from exc
        if first < 0 or last < first or sprite_type not in (0, 1, 2, 3) or not file_name:
            raise ProductError("invalid sprite catalog range")
        result.append((first, last, sprite_type, file_name))
    result.sort(key=lambda row: (row[0], row[1], row[3]))
    previous_last = -1
    for first, last, _sprite_type, _file in result:
        if first <= previous_last:
            raise ProductError("overlapping sprite catalog ranges")
        previous_last = last
    if not result:
        raise ProductError("source catalog contains no sprite ranges")
    return tuple(result)


def _sprite_geometry_index(ranges: tuple[tuple[int, int, int, str], ...]) -> dict[int, tuple[int, int]]:
    sizes = ((32, 32), (32, 64), (64, 32), (64, 64))
    index: dict[int, tuple[int, int]] = {}
    for first, last, sprite_type, _file in ranges:
        geometry = sizes[sprite_type]
        for sprite_id in range(first, last + 1):
            index[sprite_id] = geometry
    return index


def _effective_duration_ranges(frame: FrameGroup) -> tuple[tuple[int, int], ...]:
    if frame.phase_count <= 1:
        return ((0, 0),)
    raw = tuple((phase.duration_min_ms, phase.duration_max_ms) for phase in frame.phases)
    fallback = next((pair for pair in raw if pair != (0, 0)), (1, 1))
    return tuple(fallback if pair == (0, 0) else pair for pair in raw)


def _presentation_durations(frame: FrameGroup) -> tuple[int, ...]:
    if frame.phase_count <= 1:
        return (0,)
    return tuple(max(1, (low + high) // 2) for low, high in _effective_duration_ranges(frame))


def _frame_program(category: str, appearance_id: int, frame: FrameGroup, sprite_geometry: dict[int, tuple[int, int]]) -> dict[str, Any]:
    for sprite_id in frame.sprite_ids:
        if sprite_id not in sprite_geometry:
            raise ProductError(f"{category} appearance {appearance_id} references missing sprite {sprite_id}")
    raw_ranges = [[phase.duration_min_ms, phase.duration_max_ms] for phase in frame.phases]
    effective = [list(pair) for pair in _effective_duration_ranges(frame)]
    semantic = FRAME_GROUP_SEMANTICS.get(frame.fixed_frame_group, "unknown")
    if category == "object" and frame.fixed_frame_group != 2:
        raise ProductError(f"object appearance {appearance_id} uses unsupported frame group {frame.fixed_frame_group}")
    if category == "outfit" and frame.fixed_frame_group not in (0, 1):
        raise ProductError(f"outfit appearance {appearance_id} uses unsupported frame group {frame.fixed_frame_group}")
    if category == "outfit" and frame.layers not in (1, 2):
        raise ProductError(f"outfit appearance {appearance_id} has unsupported layer count {frame.layers}")
    core: dict[str, Any] = {
        "appearance_source_id": appearance_id,
        "category": category,
        "frame_group": {"id": frame.group_id, "semantic": semantic, "type": frame.fixed_frame_group},
        "index_order": ["phase", "pattern_z", "pattern_y", "pattern_x", "layer"],
        "layers": frame.layers,
        "patterns": {"depth": frame.pattern_depth, "height": frame.pattern_height, "width": frame.pattern_width},
        "phase_count": frame.phase_count,
        "source_profile_id": SOURCE_PROFILE_ID,
        "sprite_source_ids": list(frame.sprite_ids),
    }
    if frame.phase_count > 1:
        core["animation"] = {
            "default_start_phase": frame.default_start_phase,
            "duration_ranges_ms": raw_ranges,
            "effective_duration_ranges_ms": effective,
            "loop_count": frame.loop_count,
            "loop_type": LOOP_TYPES[frame.loop_type],
            "presentation_durations_ms": list(_presentation_durations(frame)),
            "random_start_phase": frame.random_start_phase,
            "synchronized": frame.synchronized,
            "timing_policy": "source-range-first-nonzero-fallback+deterministic-midpoint-v1",
        }
    else:
        core["animation"] = None
    program_digest = _sha256_bytes(canonical_bytes(core))
    core["program_id"] = f"animation-program:sha256:{program_digest}"
    return core


def _census_category(category: str, appearances: tuple[Appearance, ...]) -> dict[str, Any]:
    frame_groups = [frame for appearance in appearances for frame in appearance.frame_groups]
    animated_groups = [frame for frame in frame_groups if frame.phase_count > 1]
    animated_appearances = sum(any(frame.phase_count > 1 for frame in appearance.frame_groups) for appearance in appearances)
    sprite_ids = [sprite for frame in frame_groups for sprite in frame.sprite_ids]
    raw_pairs = [(phase.duration_min_ms, phase.duration_max_ms) for frame in animated_groups for phase in frame.phases]
    return {
        "appearances": len(appearances),
        "frame_groups": len(frame_groups),
        "animated_appearances": animated_appearances,
        "animated_frame_groups": len(animated_groups),
        "appearance_frame_group_counts": dict(sorted(Counter(len(a.frame_groups) for a in appearances).items())),
        "frame_group_types": dict(sorted(Counter(frame.fixed_frame_group for frame in frame_groups).items())),
        "frame_group_ids": dict(sorted(Counter(frame.group_id for frame in frame_groups).items())),
        "phase_counts": dict(sorted(Counter(frame.phase_count for frame in frame_groups).items())),
        "pattern_width": dict(sorted(Counter(frame.pattern_width for frame in frame_groups).items())),
        "pattern_height": dict(sorted(Counter(frame.pattern_height for frame in frame_groups).items())),
        "pattern_depth": dict(sorted(Counter(frame.pattern_depth for frame in frame_groups).items())),
        "layers": dict(sorted(Counter(frame.layers for frame in frame_groups).items())),
        "animated_default_start_phase": dict(sorted(Counter(frame.default_start_phase for frame in animated_groups).items())),
        "animated_synchronized": {str(key).lower(): value for key, value in sorted(Counter(frame.synchronized for frame in animated_groups).items())},
        "animated_random_start_phase": {str(key).lower(): value for key, value in sorted(Counter(frame.random_start_phase for frame in animated_groups).items())},
        "animated_loop_types": dict(sorted(Counter(LOOP_TYPES[frame.loop_type] for frame in animated_groups).items())),
        "animated_loop_counts": dict(sorted(Counter(frame.loop_count for frame in animated_groups).items())),
        "sprite_refs": len(sprite_ids),
        "unique_sprite_ids": len(set(sprite_ids)),
        "sprite_id_min": min(sprite_ids) if sprite_ids else None,
        "sprite_id_max": max(sprite_ids) if sprite_ids else None,
        "animated_duration_pairs": len(raw_pairs),
        "animated_zero_duration_pairs": sum(pair == (0, 0) for pair in raw_pairs),
        "animated_all_zero_duration_groups": sum(frame.phase_count > 1 and all((p.duration_min_ms, p.duration_max_ms) == (0, 0) for p in frame.phases) for frame in frame_groups),
        "duration_min_ms": min((low for low, _ in raw_pairs), default=None),
        "duration_max_ms": max((high for _, high in raw_pairs), default=None),
    }


def _source_identity() -> dict[str, str]:
    return {
        "appearance_sha256": SOURCE_APPEARANCE_SHA256,
        "catalog_sha256": SOURCE_CATALOG_SHA256,
        "drive_file_id": SOURCE_DRIVE_FILE_ID,
        "label": SOURCE_LABEL,
        "zip_sha256": SOURCE_ZIP_SHA256,
    }


def _outfit_semantics() -> dict[str, Any]:
    return {
        "addons": {"pattern_y_policy": "base pattern y=0 always; each y>=1 is enabled iff addons bit (1 << (y-1)) is set"},
        "colors": {
            "algorithm": "tibia-hsi-19x7-v1",
            "input_domain": [0, 132],
            "mask_rgba_roles": {
                "0,0,255,255": "feet",
                "0,255,0,255": "legs",
                "255,0,0,255": "body",
                "255,255,0,255": "head",
            },
            "composition": "layer 0 base RGBA alpha-composed; when layer 1 exists its exact mask pixels multiply the already composed base RGB by the resolved role color / 255; enabled addon y-patterns are composed in ascending y",
        },
        "directions": {
            "pattern_width_1": {"south": 0},
            "pattern_width_at_least_4": {"east": 1, "north": 0, "south": 2, "west": 3},
            "pattern_width_2_or_3": "UNSUPPORTED_DIRECTION_SEMANTICS",
            "static_default": "south",
        },
        "frame_groups": {"idle": 0, "moving": 1},
        "mounted_pattern_z": "UNSUPPORTED_FOR_CURRENT_STATIC_CREATURE_PLACEMENTS",
        "static_pattern_z": 0,
    }


def build_product_from_bytes(catalog_bytes: bytes, appearance_bytes: bytes) -> dict[str, Any]:
    ranges = _catalog_sprite_ranges(catalog_bytes)
    sprite_geometry = _sprite_geometry_index(ranges)
    decoded = {category: decode_category(appearance_bytes, category) for category in CATEGORY_FIELDS}

    object_programs = [
        _frame_program("object", appearance.appearance_id, frame, sprite_geometry)
        for appearance in decoded["object"] for frame in appearance.frame_groups if frame.phase_count > 1
    ]
    outfit_programs = [
        _frame_program("outfit", appearance.appearance_id, frame, sprite_geometry)
        for appearance in decoded["outfit"] for frame in appearance.frame_groups
    ]
    object_programs.sort(key=lambda row: (row["appearance_source_id"], row["frame_group"]["type"], row["frame_group"]["id"]))
    outfit_programs.sort(key=lambda row: (row["appearance_source_id"], row["frame_group"]["type"], row["frame_group"]["id"]))

    census = {category: _census_category(category, decoded[category]) for category in CATEGORY_FIELDS}
    census["catalog"] = {
        "entries": len(json.loads(catalog_bytes)),
        "sprite_sheets": len(ranges),
        "sprite_id_min": ranges[0][0],
        "sprite_id_max": ranges[-1][1],
    }
    product_core = {
        "capability": CAPABILITY,
        "contract_id": CONTRACT_ID,
        "semantic_revision": SEMANTIC_REVISION,
        "source": _source_identity(),
        "source_profile_id": SOURCE_PROFILE_ID,
        "migration_evidence": {"repository": MIGRATION_EVIDENCE_REPOSITORY, "sha": MIGRATION_EVIDENCE_SHA},
        "selection_semantics": {
            "object_pattern_resolution": "Game placement producer must export concrete x/y/z; Atlas must not derive coordinate/stack/hook/fluid patterns",
            "sprite_index_order": ["phase", "pattern_z", "pattern_y", "pattern_x", "layer"],
        },
        "outfit_semantics": _outfit_semantics(),
        "statistics": {
            "animated_object_appearances": census["object"]["animated_appearances"],
            "animated_object_programs": len(object_programs),
            "outfit_appearances": census["outfit"]["appearances"],
            "outfit_programs": len(outfit_programs),
            "animated_outfit_appearances": census["outfit"]["animated_appearances"],
            "animated_outfit_programs": census["outfit"]["animated_frame_groups"],
            "effect_appearances_census_only": census["effect"]["appearances"],
            "missile_appearances_census_only": census["missile"]["appearances"],
        },
    }
    root_payload = b"OTERYN-ANIMATED-APPEARANCES-V1\0" + canonical_bytes(product_core)
    for program in object_programs:
        root_payload += canonical_bytes(program)
    for program in outfit_programs:
        root_payload += canonical_bytes(program)
    product_root = "sha256:" + _sha256_bytes(root_payload)
    return {"manifest": {**product_core, "product_root": product_root}, "census": census, "object_programs": object_programs, "outfit_programs": outfit_programs}


def read_exact_source_zip(path: Path, *, verify_zip_digest: bool = True) -> tuple[bytes, bytes]:
    if verify_zip_digest and _sha256_file(path) != SOURCE_ZIP_SHA256:
        raise ProductError("15.32 asset ZIP SHA-256 mismatch")
    try:
        with zipfile.ZipFile(path) as archive:
            names = set(archive.namelist())
            if SOURCE_CATALOG_NAME not in names or SOURCE_APPEARANCE_NAME not in names:
                raise ProductError("exact 15.32 catalog/appearance members are missing")
            catalog = archive.read(SOURCE_CATALOG_NAME)
            appearance = archive.read(SOURCE_APPEARANCE_NAME)
    except zipfile.BadZipFile as exc:
        raise ProductError("invalid 15.32 asset ZIP") from exc
    if _sha256_bytes(catalog) != SOURCE_CATALOG_SHA256:
        raise ProductError("15.32 asset catalog SHA-256 mismatch")
    if _sha256_bytes(appearance) != SOURCE_APPEARANCE_SHA256:
        raise ProductError("15.32 appearance SHA-256 mismatch")
    return catalog, appearance


def write_product(product: dict[str, Any], output: Path) -> dict[str, Any]:
    output.mkdir(parents=True, exist_ok=True)
    files: dict[str, bytes] = {
        "manifest.json": canonical_bytes(product["manifest"]),
        "census.json": canonical_bytes(product["census"]),
        "object-programs.jsonl": b"".join(canonical_bytes(row) for row in product["object_programs"]),
        "outfit-programs.jsonl": b"".join(canonical_bytes(row) for row in product["outfit_programs"]),
    }
    file_meta: dict[str, dict[str, Any]] = {}
    for name, payload in files.items():
        (output / name).write_bytes(payload)
        file_meta[name] = {"bytes": len(payload), "sha256": _sha256_bytes(payload)}
    envelope = {"capability": CAPABILITY, "files": file_meta, "product_root": product["manifest"]["product_root"], "source": _source_identity()}
    envelope_bytes = canonical_bytes(envelope)
    (output / "product.json").write_bytes(envelope_bytes)
    return {**envelope, "product_file_sha256": _sha256_bytes(envelope_bytes)}


def load_program_indexes(product_dir: Path) -> tuple[dict[int, dict[str, Any]], dict[int, list[dict[str, Any]]]]:
    manifest = json.loads((product_dir / "manifest.json").read_text(encoding="utf-8"))
    if manifest.get("contract_id") != CONTRACT_ID or manifest.get("capability") != CAPABILITY:
        raise ProductError("unsupported appearance product")
    if manifest.get("source") != _source_identity():
        raise ProductError("appearance product source identity mismatch")
    object_index: dict[int, dict[str, Any]] = {}
    for line in (product_dir / "object-programs.jsonl").read_text(encoding="utf-8").splitlines():
        row = json.loads(line)
        appearance_id = int(row["appearance_source_id"])
        if appearance_id in object_index:
            raise ProductError(f"duplicate object animation program for {appearance_id}")
        object_index[appearance_id] = row
    outfit_index: dict[int, list[dict[str, Any]]] = {}
    for line in (product_dir / "outfit-programs.jsonl").read_text(encoding="utf-8").splitlines():
        row = json.loads(line)
        outfit_index.setdefault(int(row["appearance_source_id"]), []).append(row)
    return object_index, outfit_index


def resolve_object_animation_ref(product_dir: Path, appearance_source_id: int, pattern: dict[str, int]) -> dict[str, Any] | None:
    object_index, _ = load_program_indexes(product_dir)
    program = object_index.get(int(appearance_source_id))
    if program is None:
        return None
    patterns = program["patterns"]
    x, y, z = int(pattern["x"]), int(pattern["y"]), int(pattern["z"])
    if not (0 <= x < patterns["width"] and 0 <= y < patterns["height"] and 0 <= z < patterns["depth"]):
        raise ProductError("resolved object pattern is outside animation program dimensions")
    variant_core = {"animation_program_id": program["program_id"], "pattern": {"x": x, "y": y, "z": z}}
    return {**variant_core, "variant_id": "animation-variant:sha256:" + _sha256_bytes(canonical_bytes(variant_core))}


def outfit_color(value: int) -> tuple[int, int, int]:
    H_STEPS, SI_VALUES = 19, 7
    if not 0 <= value < H_STEPS * SI_VALUES:
        raise ProductError(f"outfit color {value} outside supported 0..132 domain")
    if value % H_STEPS == 0:
        gray = round((1 - value / (H_STEPS * SI_VALUES)) * 255)
        return gray, gray, gray
    hue = (value % H_STEPS) / 18
    saturation, intensity = ((.25, 1), (.25, .75), (.5, .75), (.667, .75), (1, 1), (1, .75), (1, .5))[value // H_STEPS]
    minimum = intensity * (1 - saturation)
    if hue < 1 / 6:
        channels = (intensity, minimum + (intensity - minimum) * 6 * hue, minimum)
    elif hue < 2 / 6:
        channels = (intensity - (intensity - minimum) * (6 * hue - 1), intensity, minimum)
    elif hue < 3 / 6:
        channels = (minimum, intensity, minimum + (intensity - minimum) * (6 * hue - 2))
    elif hue < 4 / 6:
        channels = (minimum, intensity - (intensity - minimum) * (6 * hue - 3), intensity)
    elif hue < 5 / 6:
        channels = (minimum + (intensity - minimum) * (6 * hue - 4), minimum, intensity)
    else:
        channels = (intensity, minimum, intensity - (intensity - minimum) * (6 * hue - 5))
    return tuple(round(channel * 255) for channel in channels)  # type: ignore[return-value]


def resolve_outfit_presentation(product_dir: Path, *, look_type: int, head: int, body: int, legs: int, feet: int, addons: int) -> dict[str, Any]:
    _, outfit_index = load_program_indexes(product_dir)
    programs = outfit_index.get(int(look_type))
    if not programs:
        raise ProductError(f"unknown lookType {look_type}")
    if addons < 0:
        raise ProductError("addons must be non-negative")
    colors = {"head": outfit_color(head), "body": outfit_color(body), "legs": outfit_color(legs), "feet": outfit_color(feet)}
    resolved_groups: list[dict[str, Any]] = []
    seen_semantics: set[str] = set()
    for program in programs:
        semantic = str(program["frame_group"]["semantic"])
        if semantic not in ("outfit-idle", "outfit-moving"):
            continue
        if semantic in seen_semantics:
            raise ProductError(f"ambiguous {semantic} frame group for lookType {look_type}")
        seen_semantics.add(semantic)
        width = int(program["patterns"]["width"])
        height = int(program["patterns"]["height"])
        if width == 1:
            directions = {"south": 0}
        elif width >= 4:
            directions = {"north": 0, "east": 1, "south": 2, "west": 3}
        else:
            raise ProductError(f"lookType {look_type} has unsupported direction pattern width {width}")
        enabled_y = [0] + [value for value in range(1, height) if addons & (1 << (value - 1))]
        resolved_groups.append({
            "animation_program_id": program["program_id"],
            "directions": directions,
            "enabled_addon_pattern_y": enabled_y,
            "frame_group": program["frame_group"],
            "pattern_z": 0,
            "phase_count": program["phase_count"],
            "animation": program["animation"],
        })
    if not resolved_groups:
        raise ProductError(f"lookType {look_type} has no supported outfit frame groups")
    core = {
        "addons": addons,
        "colors_rgb": {key: list(value) for key, value in colors.items()},
        "groups": sorted(resolved_groups, key=lambda row: row["frame_group"]["type"]),
        "look_type": look_type,
        "mask_policy": "tibia-outfit-mask-layer1-v1",
    }
    return {**core, "outfit_presentation_id": "outfit-presentation:sha256:" + _sha256_bytes(canonical_bytes(core))}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("asset_zip", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    try:
        catalog, appearance = read_exact_source_zip(args.asset_zip)
        product = build_product_from_bytes(catalog, appearance)
        result = write_product(product, args.output)
    except ProductError as exc:
        raise SystemExit(f"ERROR: {exc}") from exc
    print(json.dumps({"product_root": result["product_root"], **product["manifest"]["statistics"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
