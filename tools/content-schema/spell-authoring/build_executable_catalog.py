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

C18B_SOURCE = {
    "repository": "zimbadev/crystalserver",
    "revision": "201189f871d23597b82caf88fd611454a2ddefb8",
}
C18B_HEADERS = {
    "divine_barrage": (
        (
            ("targeting", "needs_target"),
            True,
            "needTarget",
            32,
            "supersedes S21's Canary false",
        ),
        (
            ("targeting", "cast_at_position"),
            True,
            "needPosition",
            31,
            "retains position input with held-target precedence",
        ),
    ),
    "ethereal_barrage": (
        (
            ("targeting", "needs_target"),
            True,
            "needTarget",
            34,
            "supersedes S21's Canary false",
        ),
        (
            ("targeting", "cast_at_position"),
            True,
            "needPosition",
            33,
            "retains position input with held-target precedence",
        ),
    ),
}


def _constant(value):
    return {"const": str(value)}


def _operation(name, *args):
    return {"op": name, "args": list(args)}


def _crystal_damage_bounds(slug):
    level = {"fn": "level_base_damage_healing", "args": [{"var": "level"}]}
    magic = {"var": "magic_level"}
    power = {"var": "base_power"}
    if slug == "death_echo":
        # Death Echo's delayed owner halves these same bounds before truncation.
        return tuple(
            _operation("add", level, _operation("mul", magic, _constant(scale)))
            for scale in ("2.4", "3.6")
        )
    if slug == "divine_barrage":
        scaled = _operation(
            "mul", magic, _operation("sqrt", _operation("mul", power, _constant("0.4")))
        )
        rounding = (("floor", "0.88"), ("floor", "1.12"))
    else:
        scaled = _operation(
            "mul",
            _operation(
                "mul",
                _operation("div", power, _constant(1000)),
                {"var": "attack_skill"},
            ),
            {"var": "attack_value"},
        )
        rounding = (("floor", "0.9"), ("ceil", "1.1"))
    average = _operation(
        "add", _operation("add", level, scaled), _operation("div", power, _constant(6))
    )
    return tuple(
        _operation(mode, _operation("mul", average, _constant(scale)))
        for mode, scale in rounding
    )


def apply_crystal_grenade_parameters(parameters):
    # Crystal 201189f8: divine_grenade.lua:6-24,26-42,110 and
    # player_wheel.cpp:1408-1411,2315-2323.
    if parameters["value_timing"] != "at_expiry":
        parameters["expiry_inputs"][:0] = ["level", "magic_level", "wheel_stage"]
    parameters.pop("snapshot_inputs", None)
    parameters.update(
        value_timing="at_expiry",
        cooldown_ms_by_stage=[26000, 20000, 16000],
        base_damage_bonus_percent_by_stage=[30, 60, 100],
    )


def apply_c18b_spell_policy(spell):
    key = spell["identity"]["key"]
    slug = key.removeprefix("candidate:spell/")
    for (section, field), value, *_ in C18B_HEADERS.get(slug, ()):
        spell[section][field] = value
    if slug == "death_echo":
        # Official news 8872 and the spell library govern base power and mana.
        apply_c18b_death_echo_formula(
            spell["execution"]["native_behavior"]["parameters"]
        )
    elif slug == "divine_grenade":
        # Keep the S22 Wheel unlock and the Global four-tile range.
        apply_crystal_grenade_parameters(
            spell["execution"]["native_behavior"]["parameters"]
        )


def apply_c18b_death_echo_formula(parameters):
    minimum, maximum = _crystal_damage_bounds("death_echo")
    parameters["formula"].update(
        minimum=minimum, maximum=maximum, selection_rule="C18b"
    )


def _apply_c18b_entry(entry):
    spell = entry["bundle"]["spell"]
    slug = spell["identity"]["key"].removeprefix("candidate:spell/")
    if slug not in C18B_HEADERS and slug not in {"death_echo", "divine_grenade"}:
        return
    apply_c18b_spell_policy(spell)
    if slug == "divine_grenade":
        _bind_manifest(entry)
        return
    manifest = entry["manifest"]
    if C18B_SOURCE not in manifest["sources"]:
        manifest["sources"].append(C18B_SOURCE.copy())
    source_index = manifest["sources"].index(C18B_SOURCE)
    path = f"data/scripts/spells/attack/{slug}.lua"
    for pointer, _, field, line, decision in C18B_HEADERS.get(slug, ()):
        destination = "/spell/spell/" + "/".join(pointer)
        manifest["entries"] = [
            row
            for row in manifest["entries"]
            if row.get("destination") != destination
            and not (
                row["source_field"] == field and row["status"] == "approved_omission"
            )
        ]
        manifest["entries"].append(
            {
                "destination": destination,
                "kind": "field",
                "source_field": field,
                "source_file": path,
                "source_index": source_index,
                "source_line": line,
                "status": "mapped",
                "resolution": f"C18b: pinned Crystal registration {decision}.",
            }
        )

    _record_crystal_formula(entry, slug, path, source_index)
    _bind_manifest(entry)


def _bind_manifest(entry):
    # The runtime checks that every source identity binds the selected manifest.
    manifest = entry["manifest"]
    digest = hashlib.sha256(canonical_bytes(manifest)).hexdigest()
    for identity in entry["source_identities"]:
        identity["sources"] = manifest["sources"]
        identity["manifest_sha256"] = digest


def _record_crystal_formula(entry, slug, path, source_index):
    manifest = entry["manifest"]
    if slug == "death_echo":
        destination = "/spell/spell/execution/native_behavior/parameters/formula"
        callback, line = "onGetDeathEchoValues", 21
        resolution = "Callback lines 21–31 use calculateBaseDamageHealing (register_spells.lua:602–605); the echo halves the bounds."
    else:
        destination = "/dependencies/formulas"
        (formula,) = entry["dependencies"]["formulas"]
        minimum, maximum = _crystal_damage_bounds(slug)
        formula.update(minimum=minimum, maximum=maximum)
        callback = "onGetFormulaValues"
        line = 7 if slug == "divine_barrage" else 8
        resolution = (
            "calculateMagicSpellDamage (register_spells.lua:636–644) floors both bounds."
            if slug == "divine_barrage"
            else "spellSkillDamage (register_spells.lua:653–657) uses weapon attack; the callback floors minimum and ceils maximum."
        )
    manifest["entries"] = [
        row for row in manifest["entries"] if row.get("destination") != destination
    ]
    manifest["entries"].append(
        {
            "destination": destination,
            "kind": "script",
            "source_field": callback,
            "source_file": path,
            "source_index": source_index,
            "source_line": line,
            "status": "resolved_native_behavior",
            "resolution": f"C18b: pinned Crystal callback. {resolution}",
        }
    )


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

    for entry in catalog["bundles"]:
        _apply_c18b_entry(entry)
    apply_monk_attack_parity(catalog)
    return catalog


def canonical_bytes(document):
    return (
        json.dumps(document, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode()


def build(root):
    return apply_owner_decisions(collect_source_bundles(root))


def collect_source_bundles(root):
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
    return {
        "schema": SCHEMA,
        "revision": REVISION,
        "bundles": bundles,
        "removed": removed,
    }


def refresh_native_manifest(path):
    """Refresh the V1 sample's spell pins from its packaged inputs."""
    manifest = json.loads(path.read_bytes())
    if manifest["schema"] != "OTERYN_NATIVE_GAMEPLAY_MANIFEST/v1":
        raise ValueError("Expected a V1 native gameplay manifest")
    catalog_pin = manifest["catalog"]
    selection_pin = manifest["source_selection"]
    catalog_raw = (path.parent / catalog_pin["path"]).read_bytes()
    selection_raw = (path.parent / selection_pin["path"]).read_bytes()
    catalog_sha256 = hashlib.sha256(catalog_raw).hexdigest()
    selection = json.loads(selection_raw)
    if (
        selection["catalog_sha256"] != catalog_sha256
        or selection["revision"] != json.loads(catalog_raw)["revision"]
    ):
        raise ValueError("Executable catalogue/source selection mismatch")
    catalog_pin["sha256"] = catalog_sha256
    selection_pin["sha256"] = hashlib.sha256(selection_raw).hexdigest()
    path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=pathlib.Path, nargs="?")
    parser.add_argument("output", type=pathlib.Path, nargs="?")
    parser.add_argument("--selection-output", type=pathlib.Path)
    parser.add_argument(
        "--sources", type=pathlib.Path, default=pathlib.Path("/workspace/spell-sources")
    )
    parser.add_argument("--refresh-native-manifest", type=pathlib.Path, metavar="PATH")
    args = parser.parse_args()
    if args.refresh_native_manifest is not None:
        if (
            args.source is not None
            or args.output is not None
            or args.selection_output is not None
        ):
            parser.error("--refresh-native-manifest is a standalone mode")
        refresh_native_manifest(args.refresh_native_manifest)
        raise SystemExit(0)
    if args.source is None or args.output is None:
        parser.error("source and output are required when building a catalogue")
    catalog = build(args.source)
    data = canonical_bytes(catalog)
    args.output.write_bytes(data)
    if args.selection_output:
        selection = build_selection(
            catalog, hashlib.sha256(data).hexdigest(), args.sources
        )
        selected_bytes = canonical_bytes(selection)
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
