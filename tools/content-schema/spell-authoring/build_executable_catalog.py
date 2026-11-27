#!/usr/bin/env python3
"""Package every qualified r20 bundle for the normal content compiler.

This produces compiler input, not an activation receipt. In particular, aliases
and unsupported runtime owners remain visible to the Rust compiler.
"""

import argparse
import hashlib
import json
import pathlib
import re
import subprocess

SCHEMA = "OTERYN_EXECUTABLE_SPELL_CATALOG/v1"
REVISION = "spell-p2-r20"
REMOVED = {
    "expose weakness": "S24",
    "sap strength": "S24",
    "light stone shower rune": "S25",
    "lightest missile rune": "S25",
    "practise fire wave": "S25",
    "practise healing": "S25",
}
SOURCE_PINS = {
    "canary": ("opentibiabr/canary", "99902524e052f37574194466c2949c576e4ab269"),
    "crystal": ("zimbadev/crystalserver", "ff7ede593c69d4c658b382c97443e8155926924a"),
}


def build_selection(catalog, catalog_digest, sources):
    """Explicit S21 policy choice; alternatives remain complete source records."""
    for source, (_, revision) in SOURCE_PINS.items():
        head = subprocess.run(
            ["git", "-C", str(sources / source), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        if head != revision:
            raise ValueError(f"unexpected {source} source HEAD")
    rows = []
    for vocation, suffix in (
        ("druid", "dru"),
        ("knight", "eq"),
        ("paladin", "sac"),
        ("sorcerer", "ven"),
    ):
        selected_key = f"candidate:spell/{vocation}_familiar"
        alternative_key = f"candidate:spell/summon_{vocation}_familiar"
        entries = {
            x["bundle"]["spell"]["identity"]["key"]: x for x in catalog["bundles"]
        }
        chosen, other = entries[selected_key], entries[alternative_key]
        selected, alternative = chosen["bundle"]["spell"], other["bundle"]["spell"]
        words = f"utevo gran res {suffix}"
        if selected["words"] != words or alternative["words"] != words:
            raise ValueError("familiar incantation changed")
        if (
            selected["requirements"]["vocations"]
            != alternative["requirements"]["vocations"]
        ):
            raise ValueError("familiar vocation mismatch")
        proofs = []
        for source in ("canary", "crystal"):
            repository, revision = SOURCE_PINS[source]
            for path in (
                f"data/scripts/spells/familiar/{vocation}_familiar.lua",
                "data/libs/systems/familiar.lua",
                "data/scripts/creaturescripts/familiar/on_login.lua",
            ):
                raw = (sources / source / path).read_bytes()
                committed = subprocess.run(
                    ["git", "-C", str(sources / source), "show", f"{revision}:{path}"],
                    check=True,
                    capture_output=True,
                ).stdout
                if raw != committed:
                    raise ValueError(f"modified pinned source: {source}/{path}")
                proofs.append(
                    {
                        "repository": repository,
                        "revision": revision,
                        "path": path,
                        "sha256": hashlib.sha256(raw).hexdigest(),
                        "url": f"https://github.com/{repository}/blob/{revision}/{path}",
                    }
                )
        rows.append(
            {
                "words": words,
                "selected": selected["identity"],
                "alternatives": [alternative["identity"]],
                "policy": "S21",
                "source_proofs": proofs,
                "differences": {
                    "login": {
                        "selected": selected["execution"]["native_behavior"][
                            "parameters"
                        ]["login"],
                        "alternative": alternative["execution"]["native_behavior"][
                            "parameters"
                        ]["login"],
                    },
                    "party_protection_registration": {
                        "selected": selected["execution"]["native_behavior"][
                            "parameters"
                        ]["party_protection_registration"],
                        "alternative": alternative["execution"]["native_behavior"][
                            "parameters"
                        ]["party_protection_registration"],
                    },
                },
                "notes": "S14 pins Canary15.30; S3 applies wiki values. S21 chooses Canary for helper semantics absent from the wiki. Crystal lifetime and party registration remain attributed alternatives, never rewritten.",
            }
        )
    return {
        "schema": "OTERYN_SPELL_SOURCE_SELECTION/v1",
        "revision": REVISION,
        "catalog_sha256": catalog_digest,
        "selections": rows,
    }


def apply_monk_attack_parity(catalog):
    def op(name, *args):
        return {"op": name, "args": list(args)}

    def constant(value):
        return {"const": value}

    def variable(name):
        return {"var": name}

    # Crystal 201189f8 register_spells.lua:604-620 and the corresponding spell
    # callbacks. Global spell headers and the published Sweeping falloff win.
    level = {"fn": "level_base_damage_healing", "args": [variable("level")]}
    weapon = op("floor", op("mul", constant("1.2"), variable("attack_value")))
    skill = op(
        "div", op("add", variable("attack_skill"), constant("4")), constant("28")
    )
    attack = op("add", level, op("mul", weapon, skill))
    # The seven published builder centers are independently qualified against
    # TibiaTools eeed345c. Use the donor only for the remaining coefficients.
    factors = {
        "devastating_knockout": "1.0",
        "greater_tiger_clash": "0.9",
        "swift_jab": "0.7",
        "tiger_clash": "0.7",
    }
    unmitigated = {
        "devastating_knockout",
        "double_jab",
        "forceful_uppercut",
        "greater_flurry_of_blows",
        "greater_tiger_clash",
        "mystic_repulse",
        "swift_jab",
        "thousand_fist_blows",
        "tiger_clash",
    }
    for entry in catalog["bundles"]:
        spell = entry["bundle"]["spell"]
        name = spell["identity"]["key"].removeprefix("candidate:spell/")
        if name not in factors and name not in unmitigated:
            continue
        dependencies = entry["dependencies"]
        if name in factors:
            formulas = dependencies["formulas"]
            total = op(
                "mul",
                attack,
                op(
                    "add",
                    op("div", variable("base_power"), constant("100")),
                    constant(factors[name]),
                ),
            )
            for formula in formulas:
                formula.update(
                    minimum=op("mul", total, constant("0.9")),
                    maximum=op("mul", total, constant("1.1")),
                )
        if name in unmitigated:
            for effect in dependencies["effects"]:
                if effect.get("operation") != "damage" or "mitigated_by" not in effect:
                    continue
                mitigations = [
                    kind for kind in effect["mitigated_by"] if kind != "armor"
                ]
                if mitigations:
                    effect["mitigated_by"] = mitigations
                else:
                    del effect["mitigated_by"]


def apply_owner_decisions(catalog):
    # PKG1R2, owner 2026-10-30; Global revs 1189762 / 1182654.
    # Keep donor provenance, with the accepted viewport/LOS rule in compiler input.
    entry = next(
        entry
        for entry in catalog["bundles"]
        if entry["bundle"]["spell"]["identity"]["key"]
        == "candidate:spell/rune/destroy_field_rune"
    )
    spell = entry["bundle"]["spell"]
    if spell["targeting"].get("range_tiles") not in (None, 5):
        raise ValueError("unexpected Destroy Field donor range")
    spell["targeting"].pop("range_tiles", None)
    resolution = {
        "destination": "/spell/spell/targeting/range_tiles",
        "kind": "field",
        "source_field": "range",
        "source_file": "data/scripts/runes/destroy_field_rune.lua",
        "source_index": 0,
        "source_line": 41,
        "status": "resolved_native_behavior",
        "resolution": "owner 2026-10-30 PKG1R2: viewport and line of sight replace the donor five-tile range; Global revs 1189762 / 1182654",
    }
    if resolution not in entry["manifest"]["entries"]:
        entry["manifest"]["entries"].append(resolution)
    apply_monk_attack_parity(catalog)
    return catalog


def build(root):
    readiness = json.loads((root / "readiness.json").read_text())
    if readiness["revision"] != REVISION or len(readiness["spells"]) != 252:
        raise ValueError("unexpected source census")
    bundles, removed, identities = [], [], set()
    for row in readiness["spells"]:
        name, carrier = row["name"], row["spell_type"]
        if carrier not in ("instant", "rune") or row["status"] not in (
            "ready",
            "blocked",
        ):
            raise ValueError(f"unexpected census carrier or status: {name}")
        slug = re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")
        bundle_id = f"{carrier}-{slug}"
        directory = root / "bundles" / bundle_id
        documents = {
            key: json.loads((directory / f"{filename}.json").read_text())
            for key, filename in (
                ("bundle", "spell"),
                ("dependencies", "dependencies"),
                ("catalog", "catalog"),
                ("manifest", "manifest"),
            )
        }
        identity = documents["bundle"]["spell"]["identity"]
        spell = documents["bundle"]["spell"]
        expected_key = f"candidate:spell/{'rune/' if carrier == 'rune' else ''}{slug}"
        if (
            spell["name"].lower() != name
            or spell["carrier"] != carrier
            or identity != {"key": expected_key, "revision": REVISION}
        ):
            raise ValueError(f"census/bundle identity mismatch: {bundle_id}")
        binding = (identity["key"], identity["revision"])
        if binding in identities:
            raise ValueError(f"duplicate identity: {binding}")
        identities.add(binding)
        provenance = {
            "bundle_id": bundle_id,
            "identity": identity,
            "manifest_sha256": hashlib.sha256(
                (directory / "manifest.json").read_bytes()
            ).hexdigest(),
            "sources": documents["manifest"]["sources"],
        }
        if row["status"] == "ready":
            if (
                (carrier == "instant" and name in REMOVED)
                or row["errors"]
                or row["blockers"]
            ):
                raise ValueError(f"inconsistent ready status: {name}")
            bundles.append({**documents, "source_identities": [provenance]})
        else:
            policy = REMOVED.get(name)
            if carrier != "instant" or policy is None:
                raise ValueError(f"unexpected exclusion: {name}")
            proof = [x for x in row["blockers"] if f"{policy}" in x]
            if len(proof) != 1 or "https://" not in proof[0]:
                raise ValueError(f"missing removal evidence: {name}")
            removed.append(
                {
                    "name": name,
                    "carrier": carrier,
                    "policy": policy,
                    "evidence": proof[0],
                    "source_identity": provenance,
                }
            )
    if len(bundles) != 246 or len(removed) != 6:
        raise ValueError("incomplete coverage")
    return apply_owner_decisions(
        {"schema": SCHEMA, "revision": REVISION, "bundles": bundles, "removed": removed}
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=pathlib.Path)
    parser.add_argument("output", type=pathlib.Path)
    parser.add_argument("--selection-output", type=pathlib.Path)
    parser.add_argument(
        "--sources", type=pathlib.Path, default=pathlib.Path("/workspace/spell-sources")
    )
    args = parser.parse_args()
    catalog = build(args.source)
    data = (
        json.dumps(catalog, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode()
    args.output.write_bytes(data)
    if args.selection_output:
        selection = build_selection(
            catalog, hashlib.sha256(data).hexdigest(), args.sources
        )
        selected_bytes = (
            json.dumps(
                selection, ensure_ascii=False, sort_keys=True, separators=(",", ":")
            )
            + "\n"
        ).encode()
        args.selection_output.write_bytes(selected_bytes)
        print(
            json.dumps(
                {
                    "selection_sha256": hashlib.sha256(selected_bytes).hexdigest(),
                    "selections": 4,
                }
            )
        )
    print(
        json.dumps(
            {
                "sha256": hashlib.sha256(data).hexdigest(),
                "bytes": len(data),
                "bundles": 246,
                "removed": 6,
            }
        )
    )
