"""Register schema-preserving spell collections without changing legacy identities."""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import import_spell_families as spell_import

ROOT = Path(__file__).resolve().parents[2]
FAMILY_INDEXES = {
    "Ability": "content/abilities/definitions/index.json",
    "Effect": "content/abilities/effects/index.json",
    "Formula": "content/abilities/formulas/index.json",
    "Creature": "content/creatures/definitions/index.json",
    "Presentation": "content/presentations/definitions/index.json",
    "Item": "content/items/index.json",
}


def encoded(value: object, *, registry: bool = False) -> bytes:
    options = {"indent": 2} if registry else {"separators": (",", ":")}
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, **options) + "\n").encode()


def outputs(root: Path = ROOT) -> dict[str, bytes]:
    """Prepare the entire registration before writing any file.

    Legacy shard counts describe legacy definitions only. Imported collections have
    their own schema, identity and counts; they are not silently coerced into v2.
    """
    generated = spell_import.outputs(root)
    descriptors = spell_import.descriptors(root, generated)
    for family, path in FAMILY_INDEXES.items():
        index = json.loads((root / path).read_bytes())
        index["spell_imports"] = descriptors.get(family, [])
        # Registration is semantic metadata; retain the existing index's explicit
        # pretty/compact convention instead of reformatting unchanged family data.
        generated[path] = encoded(index, registry=b'\n  "' in (root / path).read_bytes())
    for path, prefix in (
        ("content/presentations/bindings/index.json", "content/presentations/bindings/"),
        ("rulesets/progression/wheel-of-destiny/index.json", "rulesets/progression/wheel-of-destiny/"),
    ):
        index = json.loads((root / path).read_bytes())
        index["spell_imports"] = [
            entry for entries in descriptors.values() for entry in entries
            if entry["path"].startswith(prefix)
        ]
        generated[path] = encoded(index, registry=b'\n  "' in (root / path).read_bytes())
    registration = {
        "native_manifest": "content/spells.manifest.json",
        "collections": descriptors,
        "legacy_family_counts_unchanged": True,
        "runtime_activation": False,
    }
    manifest = json.loads((root / "content/manifest.json").read_bytes())
    managed = {row["path"] for row in manifest["managed_files"]} | set(generated)
    manifest["managed_files"] = [{"path": path} for path in sorted(managed)]
    manifest["spell_import"] = registration
    generated["content/manifest.json"] = encoded(manifest, registry=True)
    lock = json.loads((root / "content/content.lock.json").read_bytes())
    lock["spell_import"] = registration
    generated["content/content.lock.json"] = encoded(lock, registry=True)
    return generated


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = outputs()
    if args.check:
        drift = [path for path, data in generated.items()
                 if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != data]
        if drift:
            raise SystemExit("Spell import drift: " + ", ".join(drift))
    else:
        for path, data in generated.items():
            destination = ROOT / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(data)
    print(f"PASS registered spell import files={len(generated)} activation=false")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
