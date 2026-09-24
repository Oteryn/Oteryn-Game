#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import unittest

MODULE_PATH = Path(__file__).resolve().with_name("prepare-thais-scene.py")
SPEC = importlib.util.spec_from_file_location("prepare_thais_scene", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot load prepare-thais-scene.py")
M = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = M
SPEC.loader.exec_module(M)


class PrepareThaisSceneTests(unittest.TestCase):
    def test_fixture_payload_recomputes_artifact(self) -> None:
        tiles = b'{"position":{"floor":-7,"x":32400,"y":32239}}\n'
        diagnostics = b'{"diagnostics":[]}\n'
        core = {
            "asset_catalog_revision": {"zip_sha256": M.SOURCE_ZIP_SHA256},
            "counts": {
                "tiles": M.EXPECTED_TILES,
                "presentation_records": M.EXPECTED_PRESENTATIONS,
                "resolved_primitives": M.EXPECTED_PRIMITIVES,
                "unique_sprite_source_ids": M.EXPECTED_UNIQUE_SPRITES,
            },
            "files": {
                "tiles.jsonl": {"bytes": len(tiles), "sha256": M.sha256_bytes(tiles)},
                "diagnostics.json": {
                    "bytes": len(diagnostics),
                    "sha256": M.sha256_bytes(diagnostics),
                },
            },
        }
        digest = M.compute_artifact_digest(core, tiles, diagnostics)
        manifest = dict(core)
        manifest["artifact_digest"] = digest
        M.verify_fixture_payload(
            manifest, tiles, diagnostics, expected_artifact=digest
        )
        with self.assertRaises(M.SceneError):
            M.verify_fixture_payload(
                manifest, tiles + b"x", diagnostics, expected_artifact=digest
            )

    def test_dense_projection_preserves_order_and_geometry(self) -> None:
        row = {
            "position": {"floor": -7, "x": 32400, "y": 32239},
            "presentation": [
                {
                    "appearance_source_id": 100,
                    "presentation_order": {"order": 1, "plane": 0},
                    "source_role": "tile_item",
                    "resolved_primitives": [primitive(22, 1)],
                },
                {
                    "appearance_source_id": 99,
                    "presentation_order": {"order": 0, "plane": 0},
                    "source_role": "ground",
                    "resolved_primitives": [primitive(11, 0)],
                },
            ],
        }
        scene = M.select_dense_scene([row], strict=False)
        self.assertEqual(scene["counts"]["tile_records"], 1)
        self.assertEqual(scene["counts"]["primitives"], 2)
        self.assertEqual(
            [entry["sprite_source_id"] for entry in scene["primitives"]], [11, 22]
        )
        self.assertEqual(scene["primitives"][0]["source_geometry"], [64, 32])
        self.assertEqual(
            scene["primitives"][0]["displacement"],
            {"dx_units": -3, "dy_units": -4},
        )
        self.assertFalse(scene["proprietary_pixels_embedded"])

    def test_dense_projection_rejects_unknown_geometry(self) -> None:
        bad = primitive(11, 0)
        bad["width_units"] = 96
        row = {
            "position": {"floor": -7, "x": 32400, "y": 32239},
            "presentation": [
                {
                    "appearance_source_id": 99,
                    "presentation_order": {"order": 0, "plane": 0},
                    "source_role": "ground",
                    "resolved_primitives": [bad],
                }
            ],
        }
        with self.assertRaises(M.SceneError):
            M.select_dense_scene([row], strict=False)


def primitive(sprite_id: int, layer: int) -> dict[str, object]:
    return {
        "sprite_source_id": sprite_id,
        "width_units": 64,
        "height_units": 32,
        "displacement": {"dx_units": -3, "dy_units": -4},
        "visual_coverage_offsets": [
            {"dx_tiles": -1, "dy_tiles": 0},
            {"dx_tiles": 0, "dy_tiles": 0},
        ],
        "layer_index": layer,
        "phase": 0,
        "frame_group_id": 1,
        "frame_group_type": 0,
    }


if __name__ == "__main__":
    unittest.main()
