#!/usr/bin/env python3
"""ITEM-PROF-1: join the 15.30 client weapon proficiency staging to Item keys.

Inputs are the digest-pinned client files (`content/assets/files/proficiencies-7fea90ec...json`
and `appearances-2dfa943b...dat`, read through their committed staging under
`imports/cipsoft-staticdata/{proficiencies,weapon-proficiency-bindings}/`, produced by
`tools/content-census/stage_proficiencies.py`). Every staged shard is checked against its
manifest SHA-256 and the manifests against the pins below before any record is read.

Output (`samples/item-weapon-proficiency-15-30-7fea90ec.json`) is a normalized staging
artifact: the 443 client profiles with their raw level/perk records, the 666 weapon bindings
keyed `oteryn:item.tibia.i<client id>`, and the threshold class of each profile.
Perk `Type`, `SkillId`, `AugmentType`, `ElementId` and `DamageType` stay raw enum values; the
client file does not define them and no mapping is invented here. Nothing is an Oteryn
gameplay definition. `--check` diffs an in-memory regeneration against the committed file.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
STAGING = ROOT / "imports" / "cipsoft-staticdata"
ITEM_DEFINITIONS = ROOT / "content" / "items" / "definitions"
DEFAULT_OUTPUT = (
    Path(__file__).resolve().parent
    / "samples"
    / ("item-weapon-proficiency-15-30-7fea90ec.json")
)

SCHEMA = "OTERYN_ITEM_WEAPON_PROFICIENCY_STAGING/v1"
PROFICIENCIES_SHA256 = (
    "7fea90ec1cfd472f4b5978f4456b430d3271598b4e545b7919d692641411e015"
)
APPEARANCES_SHA256 = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
EXPECTED_PROFILES = 443
EXPECTED_BINDINGS = 666

# Owner-accepted evidence: #162 comments 5905884086 (table) and 5905899852 (decision).
THRESHOLD_SOURCE = {
    "url": "https://tibia.fandom.com/wiki/Weapon_Proficiency",
    "revid": 1192598,
    "fetched": "2026-09-30",
    "acceptance": "owner decision #162 5905899852",
}
# Progress needed for levels 1-7 and Mastery levels 8-9.
THRESHOLDS = {
    "standard": [
        1750,
        25000,
        100000,
        400000,
        2000000,
        8000000,
        30000000,
        60000000,
        90000000,
    ],
    "knight": [
        1250,
        20000,
        80000,
        300000,
        1500000,
        6000000,
        20000000,
        40000000,
        60000000,
    ],
    "crossbow": [
        600,
        8000,
        30000,
        150000,
        650000,
        2500000,
        10000000,
        20000000,
        30000000,
    ],
}
MASTERY_OFFSET = 2  # Mastery is reached at the weapon's top level + 2.
KNIGHT_WORDS = {"Sword", "Axe", "Club"}
CROSSBOW_WORD = "Crossbow"


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_family(directory: Path, family: str, expected: int) -> tuple[dict, list[dict]]:
    """Load one staged family, verifying the manifest and every shard digest first."""
    manifest_path = directory / "manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("family") != family:
        raise SystemExit(
            f"{manifest_path}: family {manifest.get('family')!r} != {family!r}"
        )
    records: list[dict] = []
    for entry in manifest["files"]:
        raw = (directory / entry["path"]).read_bytes()
        if sha256_bytes(raw) != entry["sha256"]:
            raise SystemExit(
                f"{directory / entry['path']}: digest differs from the manifest"
            )
        shard = json.loads(raw)
        if len(shard["records"]) != entry["records"]:
            raise SystemExit(f"{entry['path']}: record count differs from the manifest")
        records.extend(shard["records"])
    if len(records) != expected or manifest["record_count"] != expected:
        raise SystemExit(f"{family}: expected {expected} records, found {len(records)}")
    return manifest, records


def verify_pins(proficiency_manifest: dict, binding_manifest: dict) -> None:
    pins = {
        "proficiencies": PROFICIENCIES_SHA256,
        "appearances": APPEARANCES_SHA256,
    }
    for manifest in (proficiency_manifest, binding_manifest):
        for name, pin in pins.items():
            declared = manifest.get("inputs", {}).get(name, {}).get("sha256")
            if declared is not None and declared != pin:
                raise SystemExit(
                    f"manifest {name} digest {declared} is not the pin {pin}"
                )
    if binding_manifest["inputs"]["proficiencies"]["sha256"] != PROFICIENCIES_SHA256:
        raise SystemExit("binding manifest is not pinned to the proficiency file")
    # The client files are committed under content/assets/files; verify them when present.
    for manifest in (binding_manifest,):
        for name in ("appearances", "proficiencies"):
            path = ROOT / manifest["inputs"][name]["file"]
            if (
                path.is_file()
                and sha256_bytes(path.read_bytes())
                != manifest["inputs"][name]["sha256"]
            ):
                raise SystemExit(f"{path}: digest differs from its pin")


def threshold_class(profile_name: str) -> str:
    """Classify by the weapon word in the client profile name (never by an invented table)."""
    words = set(profile_name.replace("-", " ").split())
    if CROSSBOW_WORD in words:
        return "crossbow"
    if words & KNIGHT_WORDS:
        return "knight"
    return "standard"


def item_keys() -> set[str]:
    keys: set[str] = set()
    for path in sorted(ITEM_DEFINITIONS.glob("items-*.json")):
        for record in json.loads(path.read_text(encoding="utf-8"))["records"]:
            keys.add(record["definition"]["identity"]["key"])
    return keys


def perk_enum_inventory(profiles: list[dict]) -> dict:
    """Raw enum values observed in the perks, with counts; no meaning is attached."""
    counters = {
        name: Counter()
        for name in (
            "Type",
            "SkillId",
            "AugmentType",
            "ElementId",
            "DamageType",
            "MissileId",
        )
    }
    for profile in profiles:
        for level in profile["levels"]:
            for perk in level["Perks"]:
                for name, counter in counters.items():
                    if name in perk:
                        counter[perk[name]] += 1
    return {
        name: [
            {"value": value, "perks": count} for value, count in sorted(counter.items())
        ]
        for name, counter in counters.items()
    }


def build() -> dict:
    proficiency_manifest, definitions = load_family(
        STAGING / "proficiencies", "Proficiency", EXPECTED_PROFILES
    )
    binding_manifest, bindings = load_family(
        STAGING / "weapon-proficiency-bindings",
        "WeaponProficiencyBinding",
        EXPECTED_BINDINGS,
    )
    verify_pins(proficiency_manifest, binding_manifest)
    known_items = item_keys()

    profiles = []
    by_id = {}
    for definition in definitions:
        level_count = len(definition["levels"])
        profile = {
            "proficiency_id": definition["source_id"],
            "name": definition["name"],
            "version": definition["version"],
            "threshold_class": threshold_class(definition["name"]),
            "top_level": level_count,
            "mastery_level": level_count + MASTERY_OFFSET,
            "levels": definition["levels"],
        }
        if profile["proficiency_id"] in by_id:
            raise SystemExit(f"duplicate proficiency id {profile['proficiency_id']}")
        by_id[profile["proficiency_id"]] = profile
        profiles.append(profile)
    profiles.sort(key=lambda p: p["proficiency_id"])

    out_bindings = []
    seen = set()
    for binding in bindings:
        key = f"oteryn:item.tibia.i{binding['source_id']}"
        if key in seen:
            raise SystemExit(f"duplicate item key {key}")
        seen.add(key)
        profile = by_id.get(binding["proficiency_id"])
        if profile is None:
            raise SystemExit(
                f"{key}: dangling proficiency id {binding['proficiency_id']}"
            )
        out_bindings.append(
            {
                "item_key": key,
                "client_object_id": binding["source_id"],
                "client_name": binding["name"],
                "proficiency_id": binding["proficiency_id"],
                "threshold_class": profile["threshold_class"],
                "item_defined": key in known_items,
            }
        )
    out_bindings.sort(key=lambda b: b["client_object_id"])

    return {
        "schema": SCHEMA,
        "authority": "SOURCE_OBSERVATIONS_ONLY; no gameplay promotion; perk enum meanings undefined",
        "client_version": "15.30",
        "inputs": {
            "proficiencies_sha256": PROFICIENCIES_SHA256,
            "appearances_sha256": APPEARANCES_SHA256,
            "staging": [
                "imports/cipsoft-staticdata/proficiencies",
                "imports/cipsoft-staticdata/weapon-proficiency-bindings",
            ],
        },
        "threshold_source": THRESHOLD_SOURCE,
        "thresholds": THRESHOLDS,
        "mastery_offset": MASTERY_OFFSET,
        "counts": {
            "profiles": len(profiles),
            "bindings": len(out_bindings),
            "bindings_without_item_definition": sum(
                not b["item_defined"] for b in out_bindings
            ),
            "referenced_profiles": len({b["proficiency_id"] for b in out_bindings}),
            "bindings_by_threshold_class": dict(
                sorted(Counter(b["threshold_class"] for b in out_bindings).items())
            ),
            "profiles_by_threshold_class": dict(
                sorted(Counter(p["threshold_class"] for p in profiles).items())
            ),
        },
        "perk_raw_enums": perk_enum_inventory(profiles),
        "bindings": out_bindings,
        "profiles": profiles,
    }


def render(document: dict) -> str:
    """One record per line keeps the diff reviewable; keys sorted, LF endings."""
    head = {k: v for k, v in document.items() if k not in ("bindings", "profiles")}
    lines = ["{"]
    for key in sorted(head):
        lines.append(f"  {json.dumps(key)}: {json.dumps(head[key], sort_keys=True)},")
    for key in ("bindings", "profiles"):
        lines.append(f'  "{key}": [')
        rows = [
            json.dumps(row, sort_keys=True, separators=(", ", ": "))
            for row in document[key]
        ]
        lines.append(",\n".join("    " + row for row in rows))
        lines.append("  ]" + ("," if key == "bindings" else ""))
    lines.append("}")
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true", help="diff instead of writing")
    args = parser.parse_args(argv)
    text = render(build())
    json.loads(text)  # the rendering must stay valid JSON
    if args.check:
        current = (
            args.output.read_text(encoding="utf-8") if args.output.is_file() else None
        )
        if current != text:
            print(f"{args.output}: differs from a fresh regeneration", file=sys.stderr)
            return 1
        print(f"{args.output}: up to date")
        return 0
    args.output.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
