#!/usr/bin/env python3
"""Stage the hash-bound gameplay manifest inputs under a destination directory.

usage: stage_gameplay.py <manifest> <repository-root> <destination>

Same selection rule as tools/qualification/node_boot/run.sh: only the manifest
and the inputs it pins by sha256, at their repository-relative paths.
"""
import hashlib
import json
import pathlib
import sys

FIELDS = (
    "catalog", "source_selection", "creature_profiles", "presentation_profiles",
    "item_profiles", "spell_appearances", "build_training", "familiar_config",
    "familiar_defenses", "wheel_profile", "source_world", "progression", "item_keys",
)


def main(argv):
    repo_root = pathlib.Path(argv[2]).resolve(strict=True)
    manifest = (repo_root / argv[1]).resolve(strict=True)
    staged = pathlib.Path(argv[3])
    manifest_bytes = manifest.read_bytes()
    document = json.loads(manifest_bytes)
    selected = {manifest.relative_to(repo_root).as_posix(): manifest_bytes}
    for field in FIELDS:
        pin = document.get(field)
        if pin is None:
            continue
        raw = pin["path"]
        locator = pathlib.PurePosixPath(raw)
        if locator.is_absolute() or "\\" in raw:
            raise ValueError("unsafe manifest locator")
        source = manifest.parent.joinpath(*locator.parts).resolve(strict=True)
        if not source.is_relative_to(repo_root):
            raise ValueError("manifest input escapes repository root")
        data = source.read_bytes()
        if hashlib.sha256(data).hexdigest() != pin["sha256"]:
            raise ValueError("manifest input digest mismatch")
        name = source.relative_to(repo_root).as_posix()
        if name in selected and selected[name] != data:
            raise ValueError("conflicting manifest input")
        selected[name] = data
    staged.mkdir(mode=0o755, parents=True)
    for name, data in selected.items():
        destination = staged / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(data)


if __name__ == "__main__":
    main(sys.argv)
