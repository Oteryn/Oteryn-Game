"""Derive explicit test content without rewriting historical spell evidence."""
import argparse
import hashlib
import json
from pathlib import Path

PINS = (
    "catalog", "source_selection", "creature_profiles", "presentation_profiles",
    "item_profiles", "spell_appearances", "build_training", "familiar_config",
    "familiar_defenses", "wheel_profile", "source_world",
)


def prepare(source, output, policy, revision, creature_profiles=None, presentation_profiles=None, spell_appearances=None):
    source = source.resolve(strict=True)
    raw = source.read_bytes()
    if len(raw) > 16 * 1024:
        raise ValueError("manifest exceeds provisioning bound")
    manifest = json.loads(raw)
    if manifest.get("schema") != "OTERYN_NATIVE_GAMEPLAY_MANIFEST/v5":
        raise ValueError("full v5 candidate required")
    payloads = {}
    for key in PINS:
        pin = manifest.get(key)
        if pin is None:
            continue
        relative = Path(pin["path"])
        if relative.is_absolute() or ".." in relative.parts:
            raise ValueError("pin must stay inside source directory")
        path = (source.parent / relative).resolve(strict=True)
        if not path.is_relative_to(source.parent):
            raise ValueError("pin symlink escapes source directory")
        data = path.read_bytes()
        if len(data) > 8 * 1024 * 1024:
            raise ValueError("pin exceeds native input bound")
        if hashlib.sha256(data).hexdigest() != pin["sha256"]:
            raise ValueError("source pin digest mismatch")
        target = key + ".json"
        payloads[target] = data
        pin["path"] = target
    for key, replacement_path, schema in (
        ("creature_profiles", creature_profiles, "OTERYN_NATIVE_CREATURE_PROFILES/v1"),
        ("presentation_profiles", presentation_profiles, "OTERYN_NATIVE_PRESENTATION_PROFILES/v1"),
        ("spell_appearances", spell_appearances, "OTERYN_NATIVE_SPELL_APPEARANCES/v1"),
    ):
        if replacement_path is None:
            continue
        replacement = replacement_path.resolve(strict=True).read_bytes()
        if len(replacement) > 8 * 1024 * 1024:
            raise ValueError("native overlay exceeds input bound")
        if json.loads(replacement).get("schema") != schema:
            raise ValueError("native overlay schema mismatch")
        payloads[key + ".json"] = replacement
        manifest[key]["sha256"] = hashlib.sha256(replacement).hexdigest()
    training = manifest.get("build_training")
    if training is None:
        raise ValueError("qualified training input required")
    if revision is not None:
        if not revision or len(revision) > 128 or not revision.isascii():
            raise ValueError("invalid explicit training revision")
        training["content_revision"] = revision
    if policy == "baseline_test":
        training["magnitude_policy"] = policy
    else:
        training.pop("magnitude_policy", None)
    encoded = (json.dumps(manifest, indent=2) + "\n").encode()
    if len(encoded) > 16 * 1024:
        raise ValueError("derived manifest exceeds provisioning bound")
    # Exclusive directory creation prevents replacement of a serving generation.
    output.mkdir(parents=True, exist_ok=False)
    for name, data in payloads.items():
        (output / name).write_bytes(data)
    (output / "manifest.json").write_bytes(encoded)
    return {
        "source_manifest_sha256": hashlib.sha256(raw).hexdigest(),
        "manifest_sha256": hashlib.sha256(encoded).hexdigest(),
        "magnitude_policy": policy,
        "training_revision": training["content_revision"],
        "pinned_payloads": len(payloads),
        "manifest": str(output / "manifest.json"),
        "runtime_activation": False,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--magnitude-policy", choices=("strict", "baseline_test"), default="strict")
    parser.add_argument("--training-revision")
    parser.add_argument("--creature-profiles", type=Path)
    parser.add_argument("--presentation-profiles", type=Path)
    parser.add_argument("--spell-appearances", type=Path)
    args = parser.parse_args()
    print(json.dumps(prepare(args.source, args.output, args.magnitude_policy, args.training_revision, args.creature_profiles, args.presentation_profiles, args.spell_appearances)))


if __name__ == "__main__":
    main()
