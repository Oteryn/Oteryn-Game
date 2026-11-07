#!/usr/bin/env python3
"""Game-owned offline full-world Atlas source projection API.

This module broadens only the spatial selection of the already qualified
DYN-ATLAS-001 producer.  It does not define an Atlas publication format and it
does not move legacy authority into Atlas.  Pinned OTBM/Tibia inputs remain
inside the Game/import boundary; callers receive only the semantic projection
records owned by Game.
"""
from __future__ import annotations

from dataclasses import dataclass
import hashlib
import importlib.util
from pathlib import Path
import subprocess
import sys
from typing import Any, Iterator

PRODUCER_API = "oteryn-game-atlas-fullworld-source-v0"
BOUNDED_EXPORT_REL = Path("tools/game-atlas-thais-fixture/export.py")
QUALIFIED_BOUNDED_MODULE_NAME = "oteryn_game_atlas_qualified_dyn_producer"
FRESH_SOURCE_GENERATION_PROFILE_ID = "oteryn-crystalserver-fresh-source-generation-v2"


@dataclass(frozen=True, slots=True)
class SourceGenerationProfile:
    profile_id: str
    revision: int
    source_repository: str
    source_repository_sha: str
    world_otbm_sha256: str
    world_otbm_git_blob: str
    world_otbm_bytes: int
    asset_zip_sha256: str
    asset_catalog_sha256: str
    asset_appearance_sha256: str
    parser_repository: str
    parser_repository_sha: str
    parser_blobs: tuple[tuple[str, str], ...]


class ProducerError(RuntimeError):
    pass


FRESH_SOURCE_GENERATION_PROFILE = SourceGenerationProfile(
    profile_id=FRESH_SOURCE_GENERATION_PROFILE_ID,
    revision=2,
    source_repository="zimbadev/crystalserver",
    source_repository_sha="ff7ede593c69d4c658b382c97443e8155926924a",
    world_otbm_sha256="09cce62af6c86644b5579fba460c674585261eb987ca5aa1f52baef9e91f8bbb",
    world_otbm_git_blob="e95e8f7c7a95d1b634b49a5dea5a5dc76021406b",
    world_otbm_bytes=52_267_895,
    asset_zip_sha256="1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f",
    asset_catalog_sha256="35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85",
    asset_appearance_sha256="dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075",
    parser_repository="blakinio/Otheryn",
    parser_repository_sha="e417c5e7c22986bf4acef0495eb47f7b72c97cce",
    parser_blobs=(
        ("tools/otbm_atlas/__init__.py", "047d1274022e1d2a71d6aa23d6efcf420a24535d"),
        ("tools/otbm_atlas/assets.py", "25ed2400813bb3ccdc54482967ed05197eb1a850"),
        ("tools/otbm_atlas/semantic.py", "a11343a472145aee4d9cf65c6ce28b3e4a71a2b3"),
        ("tools/otbm_atlas/nodefile.py", "bed6f7a803d9de485c1f03cbdca4be0cb1521d30"),
    ),
)
_SOURCE_GENERATION_PROFILES = {
    FRESH_SOURCE_GENERATION_PROFILE.profile_id: FRESH_SOURCE_GENERATION_PROFILE,
}


@dataclass(frozen=True, slots=True)
class Runtime:
    legacy_root: Path
    map_path: Path
    asset_zip: Path
    assets_dir: Path
    bounded: Any
    legacy_semantic: Any
    appearances: dict[int, Any]
    sheets: list[Any]
    sheet_for_sprite: Any
    source_generation_profile_id: str | None = None
    source_generation_profile_revision: int | None = None


def _repository_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _load_bounded_module() -> Any:
    path = _repository_root() / BOUNDED_EXPORT_REL
    if not path.is_file():
        raise ProducerError(f"missing qualified DYN producer implementation: {path}")
    name = QUALIFIED_BOUNDED_MODULE_NAME
    existing = sys.modules.get(name)
    if existing is not None:
        return existing
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ProducerError(f"unable to load qualified DYN producer implementation: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def _reject_preexisting_fresh_source_modules() -> None:
    """Require a fresh one-shot import context for pinned parser evidence."""
    contaminated = sorted(
        name
        for name in sys.modules
        if name == QUALIFIED_BOUNDED_MODULE_NAME
        or name == "tools"
        or name == "tools.otbm_atlas"
        or name.startswith("tools.otbm_atlas.")
    )
    if contaminated:
        raise ProducerError(
            "fresh source parser import context contains pre-existing module: "
            f"{contaminated[0]}"
        )


def source_generation_profile(profile_id: str) -> SourceGenerationProfile:
    """Resolve one explicitly admitted source generation; unknown IDs fail closed."""
    profile = _SOURCE_GENERATION_PROFILES.get(profile_id)
    if profile is None:
        raise ProducerError(f"unknown source generation profile: {profile_id}")
    return profile


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _git_blob_sha1_file(path: Path) -> str:
    """Return Git's SHA-1 blob identity; this is provenance, not a trust digest."""
    size = path.stat().st_size
    digest = hashlib.sha1(usedforsecurity=False)
    digest.update(f"blob {size}\0".encode("ascii"))
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _git_output(repository: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(repository), *args],
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip() or "git command failed"
        raise ProducerError(f"fresh source parser Git validation failed: {detail}")
    return completed.stdout.strip()


def _validate_parser_bytecode_state(legacy_root: Path) -> None:
    """Reject ignored executable caches that could bypass pinned source blobs."""
    parser_root = legacy_root / "tools" / "otbm_atlas"
    cache_dirs = sorted(path for path in parser_root.rglob("__pycache__") if path.is_dir())
    bytecode_files = sorted(
        path
        for suffix in ("*.pyc", "*.pyo")
        for path in parser_root.rglob(suffix)
        if path.is_file()
    )
    contaminated = [*cache_dirs, *bytecode_files]
    if contaminated:
        try:
            relative = contaminated[0].resolve().relative_to(legacy_root.resolve()).as_posix()
        except ValueError:
            relative = str(contaminated[0].resolve())
        raise ProducerError(f"fresh source parser bytecode cache present: {relative}")


def _load_fresh_parser_modules(bounded: Any, legacy_root: Path) -> tuple[Any, Any]:
    """Import the pinned parser from source without creating bytecode caches."""
    previous = sys.dont_write_bytecode
    sys.dont_write_bytecode = True
    try:
        importlib.invalidate_caches()
        return bounded._load_legacy_modules(legacy_root)
    finally:
        sys.dont_write_bytecode = previous


def _validate_parser_source(profile: SourceGenerationProfile, legacy_root: Path) -> None:
    expected_root = legacy_root.resolve()
    actual_root = Path(_git_output(legacy_root, "rev-parse", "--show-toplevel")).resolve()
    if actual_root != expected_root:
        raise ProducerError("fresh source parser repository root mismatch")
    if _git_output(legacy_root, "rev-parse", "HEAD") != profile.parser_repository_sha:
        raise ProducerError("fresh source parser repository revision mismatch")
    if _git_output(legacy_root, "status", "--porcelain=v1", "--untracked-files=all"):
        raise ProducerError("fresh source parser worktree is dirty")
    _validate_parser_bytecode_state(legacy_root)

    for relative_path, expected_blob in profile.parser_blobs:
        path = legacy_root / relative_path
        if not path.is_file():
            raise ProducerError(f"fresh source parser file missing: {relative_path}")
        _git_output(legacy_root, "ls-files", "--error-unmatch", "--", relative_path)
        pinned_blob = _git_output(
            legacy_root, "rev-parse", f"{profile.parser_repository_sha}:{relative_path}"
        )
        if pinned_blob != expected_blob:
            raise ProducerError(f"fresh source pinned parser blob mismatch: {relative_path}")
        actual_blob = _git_output(legacy_root, "hash-object", "--", relative_path)
        if actual_blob != expected_blob:
            raise ProducerError(f"fresh source parser blob mismatch: {relative_path}")


def _validate_source_generation_inputs(
    profile: SourceGenerationProfile,
    map_path: Path,
    asset_zip: Path,
    assets_dir: Path,
) -> Path:
    if map_path.stat().st_size != profile.world_otbm_bytes:
        raise ProducerError("fresh world.otbm byte-size mismatch")
    if _sha256_file(map_path) != profile.world_otbm_sha256:
        raise ProducerError("fresh world.otbm SHA-256 mismatch")
    if _git_blob_sha1_file(map_path) != profile.world_otbm_git_blob:
        raise ProducerError("fresh world.otbm Git blob mismatch")
    if _sha256_file(asset_zip) != profile.asset_zip_sha256:
        raise ProducerError("15.32 asset ZIP SHA-256 mismatch")

    catalog = assets_dir / "catalog-content.json"
    if _sha256_file(catalog) != profile.asset_catalog_sha256:
        raise ProducerError("15.32 asset catalog SHA-256 mismatch")

    appearances = sorted(assets_dir.glob("appearances-*.dat"))
    if len(appearances) != 1:
        raise ProducerError(f"expected exactly one appearances-*.dat, got {len(appearances)}")
    if _sha256_file(appearances[0]) != profile.asset_appearance_sha256:
        raise ProducerError("15.32 appearance SHA-256 mismatch")
    return appearances[0]


def _validate_loaded_parser_modules(legacy_root: Path, legacy_assets: Any, legacy_semantic: Any) -> None:
    tools_module = sys.modules.get("tools")
    tools_paths = getattr(tools_module, "__path__", None) if tools_module is not None else None
    expected_tools_root = (legacy_root / "tools").resolve()
    if tools_paths is None:
        raise ProducerError("fresh source parser package path mismatch: tools")
    resolved_tools_paths = tuple(Path(path).resolve() for path in tools_paths)
    if not resolved_tools_paths or any(path != expected_tools_root for path in resolved_tools_paths):
        raise ProducerError("fresh source parser package path mismatch: tools")

    expected_modules = {
        "tools.otbm_atlas": (sys.modules.get("tools.otbm_atlas"), "tools/otbm_atlas/__init__.py"),
        "tools.otbm_atlas.assets": (legacy_assets, "tools/otbm_atlas/assets.py"),
        "tools.otbm_atlas.semantic": (legacy_semantic, "tools/otbm_atlas/semantic.py"),
        "tools.otbm_atlas.nodefile": (
            sys.modules.get("tools.otbm_atlas.nodefile"),
            "tools/otbm_atlas/nodefile.py",
        ),
    }
    loaded_parser_modules = {
        name
        for name in sys.modules
        if name == "tools.otbm_atlas" or name.startswith("tools.otbm_atlas.")
    }
    unexpected = sorted(loaded_parser_modules - expected_modules.keys())
    if unexpected:
        raise ProducerError(f"fresh source unexpected parser module loaded: {unexpected[0]}")
    missing = sorted(expected_modules.keys() - loaded_parser_modules)
    if missing:
        raise ProducerError(f"fresh source required parser module missing: {missing[0]}")

    for name, (module, relative_path) in expected_modules.items():
        if sys.modules.get(name) is not module:
            raise ProducerError(f"fresh source parser module identity mismatch: {name}")
        module_file = getattr(module, "__file__", None) if module is not None else None
        expected_path = (legacy_root / relative_path).resolve()
        if module_file is None or Path(module_file).resolve() != expected_path:
            raise ProducerError(f"fresh source parser module path mismatch: {relative_path}")


def load_runtime(
    *,
    legacy_root: str | Path,
    map_path: str | Path,
    asset_zip: str | Path,
    assets_dir: str | Path,
    source_generation_profile_id: str | None = None,
) -> Runtime:
    """Validate one exact accepted source generation and prepare projection state.

    Omitting ``source_generation_profile_id`` preserves the qualified historical
    validation path exactly.  Any later source generation is explicit opt-in and
    exact-digest scoped.
    """
    legacy_root = Path(legacy_root).resolve()
    map_path = Path(map_path).resolve()
    asset_zip = Path(asset_zip).resolve()
    assets_dir = Path(assets_dir).resolve()
    profile = (
        None
        if source_generation_profile_id is None
        else source_generation_profile(source_generation_profile_id)
    )
    if profile is not None:
        _reject_preexisting_fresh_source_modules()
    bounded = _load_bounded_module()
    if profile is None:
        appearance_path = bounded._validate_inputs(map_path, asset_zip, assets_dir)
    else:
        appearance_path = _validate_source_generation_inputs(profile, map_path, asset_zip, assets_dir)
        _validate_parser_source(profile, legacy_root)

    if profile is None:
        legacy_assets, legacy_semantic = bounded._load_legacy_modules(legacy_root)
    else:
        legacy_assets, legacy_semantic = _load_fresh_parser_modules(bounded, legacy_root)
        _validate_parser_bytecode_state(legacy_root)
        _validate_loaded_parser_modules(legacy_root, legacy_assets, legacy_semantic)
    appearances = legacy_assets.load_object_appearances(appearance_path)
    sheets = legacy_assets.load_sprite_catalog(assets_dir)
    return Runtime(
        legacy_root=legacy_root,
        map_path=map_path,
        asset_zip=asset_zip,
        assets_dir=assets_dir,
        bounded=bounded,
        legacy_semantic=legacy_semantic,
        appearances=appearances,
        sheets=sheets,
        sheet_for_sprite=legacy_assets.sheet_for_sprite,
        source_generation_profile_id=profile.profile_id if profile is not None else None,
        source_generation_profile_revision=profile.revision if profile is not None else None,
    )


def iter_records(runtime: Runtime, *, strict: bool = True) -> Iterator[Any]:
    """Yield Game/import-side semantic records from the pinned source exactly once."""
    yield from runtime.legacy_semantic.iter_map_records(runtime.map_path, strict=strict)


def is_map_header(runtime: Runtime, record: Any) -> bool:
    return isinstance(record, runtime.legacy_semantic.MapHeader)


def is_tile(runtime: Runtime, record: Any) -> bool:
    return isinstance(record, runtime.legacy_semantic.Tile)


def is_town(runtime: Runtime, record: Any) -> bool:
    return isinstance(record, runtime.legacy_semantic.Town)


def is_waypoint(runtime: Runtime, record: Any) -> bool:
    return isinstance(record, runtime.legacy_semantic.Waypoint)


def native_floor(tile: Any) -> int:
    """Apply the accepted legacy-spatial import transform for a tile."""
    return -int(tile.position.z)


def project_tile(runtime: Runtime, tile: Any) -> tuple[dict[str, Any], dict[str, Any]]:
    """Resolve one tile while preserving explicitly unresolved appearances.

    For tiles whose visible server IDs all exist in the exact pinned appearance
    catalogue, this delegates byte-for-byte to the already-qualified DYN
    producer.  A visible server ID missing from that exact catalogue remains an
    explicit unresolved presentation with no resolved primitives.  No sprite or
    appearance is substituted or inferred.
    """
    if not is_tile(runtime, tile):
        raise ProducerError(f"project_tile requires a Tile, got {type(tile)!r}")

    visible: list[tuple[str, Any]] = []
    if tile.ground is not None:
        visible.append(("ground", tile.ground))
    visible.extend(("tile_item", item) for item in tile.items)
    missing_ids = {item.server_id for _role, item in visible if item.server_id not in runtime.appearances}
    if not missing_ids:
        return runtime.bounded._tile_record(
            tile,
            appearances=runtime.appearances,
            sheets=runtime.sheets,
            sheet_for_sprite=runtime.sheet_for_sprite,
        )

    x, y, z = tile.position.x, tile.position.y, tile.position.z
    floor = native_floor(tile)
    hook_south = False
    hook_east = False
    for _role, item in visible:
        appearance = runtime.appearances.get(item.server_id)
        if appearance is None:
            continue
        hook_south = hook_south or appearance.hook_direction == 1
        hook_east = hook_east or appearance.hook_direction == 2

    presentations: list[dict[str, Any]] = []
    appearance_ids: set[int] = set()
    sprite_ids: set[int] = set()
    primitive_count = 0
    unresolved_count = 0
    for order, (role, item) in enumerate(visible):
        appearance_ids.add(item.server_id)
        appearance = runtime.appearances.get(item.server_id)
        if appearance is None:
            unresolved_count += 1
            primitives: list[dict[str, Any]] = []
            resolution_state = "UNRESOLVED_APPEARANCE"
        else:
            primitives = runtime.bounded._resolved_primitives(
                item=item,
                appearance=appearance,
                x=x,
                y=y,
                z=z,
                hook_south=hook_south,
                hook_east=hook_east,
                sheet_for_sprite=runtime.sheet_for_sprite,
                sheets=runtime.sheets,
            )
            primitive_count += len(primitives)
            sprite_ids.update(primitive["sprite_source_id"] for primitive in primitives)
            resolution_state = "RESOLVED"
        presentation = {
            "canonical_entity_id": None,
            "entity_identity_state": "UNRESOLVED",
            "export_record_id": runtime.bounded._stable_id("presentation", x, y, floor, order, item.server_id),
            "appearance_source_id": item.server_id,
            "presentation_order": {"order": order, "plane": 0},
            "resolved_primitives": primitives,
            "source_role": role,
        }
        if resolution_state != "RESOLVED":
            presentation["presentation_resolution_state"] = resolution_state
        presentations.append(presentation)

    record = {
        "position": {"floor": floor, "x": x, "y": y},
        "presentation": presentations,
        "record_type": "tile",
        "source_position": {"legacy_x": x, "legacy_y": y, "legacy_z": z},
        "tile_record_id": runtime.bounded._stable_id("tile", x, y, floor),
    }
    return record, {
        "appearance_ids": appearance_ids,
        "primitive_count": primitive_count,
        "presentation_count": len(presentations),
        "sprite_ids": sprite_ids,
        "unresolved_appearance_ids": missing_ids,
        "unresolved_presentation_count": unresolved_count,
    }


def canonical_tile_bytes(runtime: Runtime, record: dict[str, Any]) -> bytes:
    """Encode one semantic tile identically to the qualified DYN producer."""
    data = runtime.bounded._canonical_json_bytes(record)
    if len(data) > int(runtime.bounded.MAX_TILE_LINE_BYTES):
        raise ProducerError(f"semantic tile record exceeds qualified per-tile cap: {len(data)} bytes")
    return data


def project_tile_bytes(runtime: Runtime, tile: Any) -> tuple[bytes, dict[str, Any]]:
    record, stats = project_tile(runtime, tile)
    return canonical_tile_bytes(runtime, record), stats


TYPED_DEFINITION_FAMILIES = frozenset(
    {
        "TERRAIN",
        "PRESENTATION",
        "LOCAL_OBJECT",
        "CREATURE",
        "ITEM",
        "ABILITY",
        "EFFECT",
        "FORMULA",
        "BEHAVIOR",
    }
)


@dataclass(frozen=True, slots=True)
class SourceIdentityBinding:
    """Explicit D1 identity binding for one producer presentation occurrence.

    The binding is intentionally external to the legacy numeric source identity.
    Neither coordinates, presentation order nor appearance_source_id may mint a
    canonical Content definition or PlacementKey.
    """

    export_record_id: str
    appearance_source_id: int
    definition_family: str
    production_key: str
    definition_revision: str
    placement_key: str


PRODUCTION_MAX_KEY_BYTES = 512
PRODUCTION_MAX_ATOM_BYTES = 512


def _has_nonproduction_marker(value: str) -> bool:
    """Mirror the existing ProductionKey/ProductionAtom release marker fence."""

    lower = value.lower()
    segments = lower
    for separator in ":./_-":
        segments = segments.replace(separator, " ")
    has_test_segment = "test" in segments.split()
    return (
        "fixture" in lower
        or "synthetic" in lower
        or "evidence" in lower
        or "test-only" in lower
        or has_test_segment
    )


def _valid_namespaced_key(value: str) -> bool:
    if (
        not value
        or not value.isascii()
        or len(value) > PRODUCTION_MAX_KEY_BYTES
        or ":" not in value
        or _has_nonproduction_marker(value)
    ):
        return False
    namespace, local = value.split(":", 1)
    if not namespace or not local:
        return False
    return all(char.isalnum() or char in ":._-/" for char in value)


def _valid_revision(value: str) -> bool:
    return (
        bool(value)
        and value.isascii()
        and len(value) <= PRODUCTION_MAX_ATOM_BYTES
        and all(0x21 <= ord(char) <= 0x7E for char in value)
        and not _has_nonproduction_marker(value)
    )


def validate_source_identity_binding(binding: SourceIdentityBinding) -> None:
    if not binding.export_record_id:
        raise ProducerError("source identity binding requires export_record_id")
    if binding.appearance_source_id <= 0:
        raise ProducerError("source identity binding requires positive appearance_source_id")
    if binding.definition_family not in TYPED_DEFINITION_FAMILIES:
        raise ProducerError(f"unsupported typed definition family: {binding.definition_family}")
    if not _valid_namespaced_key(binding.production_key):
        raise ProducerError("source identity binding has invalid production_key")
    if not _valid_revision(binding.definition_revision):
        raise ProducerError("source identity binding has invalid definition_revision")
    if not _valid_namespaced_key(binding.placement_key):
        raise ProducerError("source identity binding has invalid placement_key")


def adapt_presentation_source_identity(
    presentation: dict[str, Any],
    binding: SourceIdentityBinding | None,
) -> dict[str, Any]:
    """Return a typed Content-source candidate without rewriting Atlas identity.

    An unbound legacy occurrence remains explicitly unresolved.  A supplied
    binding must match the exact source occurrence and legacy appearance
    provenance before it can expose D1 typed definition and placement identity.
    """

    record_id = presentation.get("export_record_id")
    appearance_source_id = presentation.get("appearance_source_id")
    if not isinstance(record_id, str) or not record_id:
        raise ProducerError("presentation lacks stable export_record_id provenance")
    if not isinstance(appearance_source_id, int) or appearance_source_id <= 0:
        raise ProducerError("presentation lacks positive appearance_source_id provenance")

    candidate: dict[str, Any] = {
        "source_occurrence_ref": record_id,
        "appearance_source_id": appearance_source_id,
        "source_role": presentation.get("source_role"),
        "source_presentation_order": presentation.get("presentation_order"),
        "identity_disposition": "UNRESOLVED_SOURCE_IDENTITY",
        "typed_definition_ref": None,
        "placement_key": None,
    }
    if binding is None:
        return candidate

    validate_source_identity_binding(binding)
    if binding.export_record_id != record_id:
        raise ProducerError("source identity binding export_record_id mismatch")
    if binding.appearance_source_id != appearance_source_id:
        raise ProducerError("source identity binding appearance_source_id mismatch")

    candidate["identity_disposition"] = "EXPLICITLY_BOUND"
    candidate["typed_definition_ref"] = {
        "definition_family": binding.definition_family,
        "production_key": binding.production_key,
        "definition_revision": binding.definition_revision,
    }
    candidate["placement_key"] = binding.placement_key
    return candidate


def adapt_tile_source_identities(
    record: dict[str, Any],
    bindings: dict[str, SourceIdentityBinding],
) -> list[dict[str, Any]]:
    """Adapt one projected tile with exact occurrence-keyed bindings.

    Existing tile/presentation projection bytes are not modified.  Extra or
    duplicate bindings fail closed so stale mappings cannot silently attach to a
    different source occurrence.
    """

    presentations = record.get("presentation")
    if not isinstance(presentations, list):
        raise ProducerError("tile record lacks presentation list")

    seen_occurrences: set[str] = set()
    candidates: list[dict[str, Any]] = []
    for presentation in presentations:
        if not isinstance(presentation, dict):
            raise ProducerError("tile presentation must be an object")
        record_id = presentation.get("export_record_id")
        if not isinstance(record_id, str) or not record_id:
            raise ProducerError("tile presentation lacks export_record_id")
        if record_id in seen_occurrences:
            raise ProducerError(f"duplicate source occurrence: {record_id}")
        seen_occurrences.add(record_id)
        candidates.append(
            adapt_presentation_source_identity(presentation, bindings.get(record_id))
        )

    stale = sorted(set(bindings) - seen_occurrences)
    if stale:
        raise ProducerError(f"identity binding does not match tile occurrence: {stale[0]}")
    return candidates
