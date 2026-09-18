#!/usr/bin/env python3
from __future__ import annotations

from dataclasses import replace
import hashlib
import importlib.util
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace

import producer


class Tile:
    def __init__(self, x: int, y: int, z: int, items: tuple[object, ...]):
        self.position = SimpleNamespace(x=x, y=y, z=z)
        self.ground = None
        self.items = items


class FakeBounded:
    def __init__(self) -> None:
        self.delegated = 0

    def _tile_record(self, tile, *, appearances, sheets, sheet_for_sprite):
        self.delegated += 1
        return {"delegated": True}, {"presentation_count": 1, "primitive_count": 1, "appearance_ids": {1}, "sprite_ids": {99}}

    @staticmethod
    def _stable_id(domain: str, *parts: object) -> str:
        payload = "\0".join([domain, *(str(part) for part in parts)]).encode()
        return f"{domain}:{hashlib.sha256(payload).hexdigest()[:32]}"


def runtime(appearances):
    bounded = FakeBounded()
    legacy = SimpleNamespace(Tile=Tile)
    return producer.Runtime(None, None, None, None, bounded, legacy, appearances, [], None), bounded


def _git_blob_id(data: bytes) -> str:
    digest = hashlib.sha1(usedforsecurity=False)
    digest.update(f"blob {len(data)}\0".encode("ascii"))
    digest.update(data)
    return digest.hexdigest()


def _run_git(repository: Path, *args: str) -> str:
    completed = producer.subprocess.run(
        ["git", "-C", str(repository), *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout.strip()


def _expect_producer_error(fragment: str, action) -> None:
    try:
        action()
    except producer.ProducerError as exc:
        assert fragment in str(exc), str(exc)
    else:
        raise AssertionError(f"expected ProducerError containing {fragment!r}")


def _profile_fixture(root: Path):
    map_path = root / "world.otbm"
    asset_zip = root / "15.32.zip"
    assets_dir = root / "assets"
    parser_root = root / "parser"
    parser_dir = parser_root / "tools" / "otbm_atlas"
    assets_dir.mkdir()
    parser_dir.mkdir(parents=True)

    map_path.write_bytes(b"map0")
    asset_zip.write_bytes(b"zip0")
    catalog = assets_dir / "catalog-content.json"
    appearance = assets_dir / "appearances-unit.dat"
    catalog.write_bytes(b"catalog0")
    appearance.write_bytes(b"appearance0")

    parser_bytes = {
        "tools/otbm_atlas/__init__.py": b"package0",
        "tools/otbm_atlas/assets.py": b"assets00",
        "tools/otbm_atlas/semantic.py": b"semantic",
        "tools/otbm_atlas/nodefile.py": b"nodefile",
    }
    for relative_path, data in parser_bytes.items():
        (parser_root / relative_path).write_bytes(data)

    _run_git(parser_root, "init", "-q")
    _run_git(parser_root, "config", "user.email", "unit@example.invalid")
    _run_git(parser_root, "config", "user.name", "unit")
    _run_git(parser_root, "add", ".")
    _run_git(parser_root, "commit", "-q", "-m", "unit parser")
    parser_repository_sha = _run_git(parser_root, "rev-parse", "HEAD")
    parser_blobs = tuple(
        (path, _run_git(parser_root, "hash-object", "--", path)) for path in parser_bytes
    )

    profile = producer.SourceGenerationProfile(
        profile_id="unit-source-generation-v1",
        revision=1,
        source_repository="example/source",
        source_repository_sha="1" * 40,
        world_otbm_sha256=hashlib.sha256(map_path.read_bytes()).hexdigest(),
        world_otbm_git_blob=_git_blob_id(map_path.read_bytes()),
        world_otbm_bytes=map_path.stat().st_size,
        asset_zip_sha256=hashlib.sha256(asset_zip.read_bytes()).hexdigest(),
        asset_catalog_sha256=hashlib.sha256(catalog.read_bytes()).hexdigest(),
        asset_appearance_sha256=hashlib.sha256(appearance.read_bytes()).hexdigest(),
        parser_repository="example/parser",
        parser_repository_sha=parser_repository_sha,
        parser_blobs=parser_blobs,
    )
    return profile, map_path, asset_zip, assets_dir, parser_root, appearance


def test_resolved_delegates() -> None:
    rt, bounded = runtime({1: SimpleNamespace(hook_direction=None)})
    tile = Tile(10, 20, 7, (SimpleNamespace(server_id=1),))
    record, stats = producer.project_tile(rt, tile)
    assert record == {"delegated": True}
    assert stats["primitive_count"] == 1
    assert bounded.delegated == 1


def test_missing_is_explicit() -> None:
    rt, bounded = runtime({})
    item = SimpleNamespace(server_id=2141)
    tile = Tile(33572, 32528, 14, (item,))
    record, stats = producer.project_tile(rt, tile)
    assert bounded.delegated == 0
    assert record["position"] == {"floor": -14, "x": 33572, "y": 32528}
    presentation = record["presentation"][0]
    assert presentation["appearance_source_id"] == 2141
    assert presentation["presentation_resolution_state"] == "UNRESOLVED_APPEARANCE"
    assert presentation["resolved_primitives"] == []
    assert stats["unresolved_appearance_ids"] == {2141}
    assert stats["unresolved_presentation_count"] == 1


def test_explicit_source_identity_binding() -> None:
    presentation = {
        "export_record_id": "presentation:abc",
        "appearance_source_id": 2141,
        "source_role": "tile_item",
        "presentation_order": {"plane": 0, "order": 3},
        "canonical_entity_id": None,
        "entity_identity_state": "UNRESOLVED",
    }
    binding = producer.SourceIdentityBinding(
        export_record_id="presentation:abc",
        appearance_source_id=2141,
        definition_family="LOCAL_OBJECT",
        production_key="oteryn:reference.object.local-door",
        definition_revision="definition-r1",
        placement_key="oteryn:reference.placement.local-door",
    )
    adapted = producer.adapt_presentation_source_identity(presentation, binding)
    assert adapted["identity_disposition"] == "EXPLICITLY_BOUND"
    assert adapted["typed_definition_ref"] == {
        "definition_family": "LOCAL_OBJECT",
        "production_key": "oteryn:reference.object.local-door",
        "definition_revision": "definition-r1",
    }
    assert adapted["placement_key"] == "oteryn:reference.placement.local-door"
    assert presentation["canonical_entity_id"] is None
    assert presentation["entity_identity_state"] == "UNRESOLVED"


def test_unbound_source_identity_stays_unresolved() -> None:
    presentation = {
        "export_record_id": "presentation:def",
        "appearance_source_id": 3687,
        "source_role": "tile_item",
        "presentation_order": {"plane": 0, "order": 1},
    }
    adapted = producer.adapt_presentation_source_identity(presentation, None)
    assert adapted["identity_disposition"] == "UNRESOLVED_SOURCE_IDENTITY"
    assert adapted["typed_definition_ref"] is None
    assert adapted["placement_key"] is None


def test_binding_mismatch_fails_closed() -> None:
    presentation = {
        "export_record_id": "presentation:abc",
        "appearance_source_id": 2141,
        "source_role": "tile_item",
        "presentation_order": {"plane": 0, "order": 0},
    }
    mismatched = producer.SourceIdentityBinding(
        export_record_id="presentation:other",
        appearance_source_id=2141,
        definition_family="LOCAL_OBJECT",
        production_key="oteryn:reference.object.local-door",
        definition_revision="definition-r1",
        placement_key="oteryn:reference.placement.local-door",
    )
    try:
        producer.adapt_presentation_source_identity(presentation, mismatched)
    except producer.ProducerError as exc:
        assert "export_record_id mismatch" in str(exc)
    else:
        raise AssertionError("mismatched source occurrence was accepted")


def test_binding_rejects_nonproduction_and_oversize_content_identity() -> None:
    base = dict(
        export_record_id="presentation:abc",
        appearance_source_id=2141,
        definition_family="LOCAL_OBJECT",
        production_key="oteryn:reference.object.local-door",
        definition_revision="definition-r1",
        placement_key="oteryn:reference.placement.local-door",
    )
    invalid_values = (
        ("production_key", "oteryn:reference.object.fixture-door"),
        ("production_key", "oteryn:reference.object.test-door"),
        ("production_key", "oteryn:" + "x" * 506),
        ("definition_revision", "evidence-r1"),
        ("definition_revision", "r" * 513),
        ("placement_key", "oteryn:reference.placement.synthetic-door"),
        ("placement_key", "oteryn:" + "p" * 506),
    )
    for field, value in invalid_values:
        values = dict(base)
        values[field] = value
        try:
            producer.validate_source_identity_binding(producer.SourceIdentityBinding(**values))
        except producer.ProducerError:
            pass
        else:
            raise AssertionError(f"invalid {field} unexpectedly accepted: {value[:64]!r}")


def test_tile_adapter_rejects_stale_binding() -> None:
    record = {
        "record_type": "tile",
        "presentation": [
            {
                "export_record_id": "presentation:abc",
                "appearance_source_id": 2141,
                "source_role": "tile_item",
                "presentation_order": {"plane": 0, "order": 0},
            }
        ],
    }
    stale = producer.SourceIdentityBinding(
        export_record_id="presentation:stale",
        appearance_source_id=2141,
        definition_family="LOCAL_OBJECT",
        production_key="oteryn:reference.object.local-door",
        definition_revision="definition-r1",
        placement_key="oteryn:reference.placement.local-door",
    )
    try:
        producer.adapt_tile_source_identities(record, {"presentation:stale": stale})
    except producer.ProducerError as exc:
        assert "does not match tile occurrence" in str(exc)
    else:
        raise AssertionError("stale source binding was accepted")


def test_fresh_source_generation_profile_is_exact_and_not_floating() -> None:
    profile = producer.source_generation_profile(producer.FRESH_SOURCE_GENERATION_PROFILE_ID)
    assert profile.revision == 2
    assert profile.source_repository == "zimbadev/crystalserver"
    assert profile.source_repository_sha == "ff7ede593c69d4c658b382c97443e8155926924a"
    assert profile.world_otbm_sha256 == "09cce62af6c86644b5579fba460c674585261eb987ca5aa1f52baef9e91f8bbb"
    assert profile.world_otbm_git_blob == "e95e8f7c7a95d1b634b49a5dea5a5dc76021406b"
    assert profile.world_otbm_bytes == 52_267_895
    for floating in ("main", "latest", "zimbadev/crystalserver@main"):
        _expect_producer_error(
            "unknown source generation profile",
            lambda profile_id=floating: producer.source_generation_profile(profile_id),
        )


def test_source_generation_inputs_and_parser_fail_closed() -> None:
    with TemporaryDirectory() as temp_dir:
        root = Path(temp_dir)
        profile, map_path, asset_zip, assets_dir, parser_root, appearance = _profile_fixture(root)
        assert producer._validate_source_generation_inputs(
            profile, map_path, asset_zip, assets_dir
        ) == appearance
        producer._validate_parser_source(profile, parser_root)
        _expect_producer_error(
            "repository root mismatch",
            lambda: producer._validate_parser_source(profile, parser_root / "tools"),
        )

        map_path.write_bytes(b"x")
        _expect_producer_error(
            "byte-size mismatch",
            lambda: producer._validate_source_generation_inputs(profile, map_path, asset_zip, assets_dir),
        )
        map_path.write_bytes(b"map0")
        map_path.write_bytes(b"MAP0")
        _expect_producer_error(
            "SHA-256 mismatch",
            lambda: producer._validate_source_generation_inputs(profile, map_path, asset_zip, assets_dir),
        )
        map_path.write_bytes(b"map0")
        _expect_producer_error(
            "Git blob mismatch",
            lambda: producer._validate_source_generation_inputs(
                replace(profile, world_otbm_git_blob="0" * 40), map_path, asset_zip, assets_dir
            ),
        )

        for path, replacement_bytes, fragment in (
            (asset_zip, b"ZIP0", "asset ZIP SHA-256 mismatch"),
            (assets_dir / "catalog-content.json", b"Catalog0", "asset catalog SHA-256 mismatch"),
            (appearance, b"Appearance0", "appearance SHA-256 mismatch"),
        ):
            original = path.read_bytes()
            path.write_bytes(replacement_bytes)
            _expect_producer_error(
                fragment,
                lambda: producer._validate_source_generation_inputs(
                    profile, map_path, asset_zip, assets_dir
                ),
            )
            path.write_bytes(original)

        bad_parser_blobs = tuple(
            (path, "0" * 40 if path.endswith("semantic.py") else blob)
            for path, blob in profile.parser_blobs
        )
        _expect_producer_error(
            "parser blob mismatch",
            lambda: producer._validate_parser_source(
                replace(profile, parser_blobs=bad_parser_blobs), parser_root
            ),
        )
        _expect_producer_error(
            "repository revision mismatch",
            lambda: producer._validate_parser_source(
                replace(profile, parser_repository_sha="0" * 40), parser_root
            ),
        )

        semantic = parser_root / "tools" / "otbm_atlas" / "semantic.py"
        semantic.write_bytes(b"Semantic")
        _expect_producer_error(
            "parser worktree is dirty",
            lambda: producer._validate_parser_source(profile, parser_root),
        )


def test_default_runtime_path_delegates_to_qualified_validator() -> None:
    class FakeLegacyAssets:
        @staticmethod
        def load_object_appearances(_path):
            return {}

        @staticmethod
        def load_sprite_catalog(_path):
            return []

        @staticmethod
        def sheet_for_sprite(_sheets, _sprite_id):
            return None

    class FakeLoadBounded:
        def __init__(self) -> None:
            self.validation_calls = 0

        def _validate_inputs(self, _map_path, _asset_zip, assets_dir):
            self.validation_calls += 1
            return assets_dir / "appearance.dat"

        @staticmethod
        def _load_legacy_modules(_legacy_root):
            return FakeLegacyAssets, SimpleNamespace()

    bounded = FakeLoadBounded()
    original_loader = producer._load_bounded_module
    module_name = "tools.otbm_atlas.semantic"
    sentinel = object()
    previous_module = producer.sys.modules.get(module_name, sentinel)
    producer._load_bounded_module = lambda: bounded
    producer.sys.modules[module_name] = SimpleNamespace(__file__="legacy/tools/otbm_atlas/semantic.py")
    try:
        runtime_state = producer.load_runtime(
            legacy_root="legacy",
            map_path="world.otbm",
            asset_zip="15.32.zip",
            assets_dir="assets",
        )
    finally:
        producer._load_bounded_module = original_loader
        if previous_module is sentinel:
            producer.sys.modules.pop(module_name, None)
        else:
            producer.sys.modules[module_name] = previous_module

    assert bounded.validation_calls == 1
    assert runtime_state.source_generation_profile_id is None
    assert runtime_state.source_generation_profile_revision is None


def test_fresh_runtime_rejects_stale_same_path_parser_cache() -> None:
    with TemporaryDirectory() as temp_dir:
        root = Path(temp_dir)
        profile, map_path, asset_zip, assets_dir, parser_root, _appearance = _profile_fixture(root)
        semantic_path = parser_root / "tools" / "otbm_atlas" / "semantic.py"
        original_semantic = semantic_path.read_bytes()
        module_name = "tools.otbm_atlas.semantic"
        sentinel = object()
        previous_module = producer.sys.modules.get(module_name, sentinel)
        previous_profile = producer._SOURCE_GENERATION_PROFILES.get(profile.profile_id, sentinel)

        semantic_path.write_text("SENTINEL = 'stale-parser-code'\n", encoding="utf-8")
        spec = importlib.util.spec_from_file_location(module_name, semantic_path)
        assert spec is not None and spec.loader is not None
        stale_module = importlib.util.module_from_spec(spec)
        producer.sys.modules[module_name] = stale_module
        spec.loader.exec_module(stale_module)
        assert stale_module.SENTINEL == "stale-parser-code"

        semantic_path.write_bytes(original_semantic)
        assert _run_git(parser_root, "status", "--porcelain=v1", "--untracked-files=all") == ""
        producer._validate_parser_source(profile, parser_root)
        producer._SOURCE_GENERATION_PROFILES[profile.profile_id] = profile
        try:
            _expect_producer_error(
                "pre-existing module",
                lambda: producer.load_runtime(
                    legacy_root=parser_root,
                    map_path=map_path,
                    asset_zip=asset_zip,
                    assets_dir=assets_dir,
                    source_generation_profile_id=profile.profile_id,
                ),
            )
        finally:
            if previous_module is sentinel:
                producer.sys.modules.pop(module_name, None)
            else:
                producer.sys.modules[module_name] = previous_module
            if previous_profile is sentinel:
                producer._SOURCE_GENERATION_PROFILES.pop(profile.profile_id, None)
            else:
                producer._SOURCE_GENERATION_PROFILES[profile.profile_id] = previous_profile


def test_loaded_parser_modules_are_root_bound() -> None:
    with TemporaryDirectory() as temp_dir:
        root = Path(temp_dir)
        expected = {
            "tools.otbm_atlas": "tools/otbm_atlas/__init__.py",
            "tools.otbm_atlas.nodefile": "tools/otbm_atlas/nodefile.py",
        }
        for relative_path in (
            "tools/otbm_atlas/__init__.py",
            "tools/otbm_atlas/assets.py",
            "tools/otbm_atlas/semantic.py",
            "tools/otbm_atlas/nodefile.py",
        ):
            path = root / relative_path
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("# unit\n", encoding="utf-8")

        sentinel = object()
        saved = {name: producer.sys.modules.get(name, sentinel) for name in expected}
        assets = SimpleNamespace(__file__=str(root / "tools/otbm_atlas/assets.py"))
        semantic = SimpleNamespace(__file__=str(root / "tools/otbm_atlas/semantic.py"))
        try:
            producer.sys.modules["tools.otbm_atlas"] = SimpleNamespace(
                __file__=str(root / expected["tools.otbm_atlas"])
            )
            producer.sys.modules["tools.otbm_atlas.nodefile"] = SimpleNamespace(
                __file__=str(root / expected["tools.otbm_atlas.nodefile"])
            )
            producer._validate_loaded_parser_modules(root, assets, semantic)
            semantic.__file__ = str(root / "wrong-semantic.py")
            _expect_producer_error(
                "parser module path mismatch",
                lambda: producer._validate_loaded_parser_modules(root, assets, semantic),
            )
        finally:
            for name, previous in saved.items():
                if previous is sentinel:
                    producer.sys.modules.pop(name, None)
                else:
                    producer.sys.modules[name] = previous


def main() -> int:
    test_resolved_delegates()
    test_missing_is_explicit()
    test_explicit_source_identity_binding()
    test_unbound_source_identity_stays_unresolved()
    test_binding_mismatch_fails_closed()
    test_binding_rejects_nonproduction_and_oversize_content_identity()
    test_tile_adapter_rejects_stale_binding()
    test_fresh_source_generation_profile_is_exact_and_not_floating()
    test_source_generation_inputs_and_parser_fail_closed()
    test_default_runtime_path_delegates_to_qualified_validator()
    test_fresh_runtime_rejects_stale_same_path_parser_cache()
    test_loaded_parser_modules_are_root_bound()
    print("game-atlas-fullworld-source self-test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
