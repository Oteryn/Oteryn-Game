#!/usr/bin/env python3
"""Prepare r25 inputs using the existing pinned-provider materializer."""
import argparse
import json
import runpy
from pathlib import Path


def build(output):
    root = Path(__file__).resolve().parent
    recipe_path = root / "manifest-recipe.json"
    recipe = json.loads(recipe_path.read_bytes())
    required = {
        "catalog", "source_selection", "creature_profiles", "presentation_profiles",
        "item_profiles", "spell_appearances", "build_training", "familiar_config",
        "familiar_defenses", "wheel_profile", "source_world",
    }
    if (recipe.get("schema") != "OTERYN_R25_CANDIDATE_MANIFEST_RECIPE/v1"
            or set(recipe["providers"]) != required):
        raise ValueError("Complete r25 provider recipe required")
    existing = runpy.run_path(str(root.parent / "r23-candidate/materialize_manifest.py"))
    return existing["build"](recipe_path, output)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps({"manifest_sha256": build(args.out), "runtime_activation": False}))
