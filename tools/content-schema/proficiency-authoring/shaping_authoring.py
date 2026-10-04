#!/usr/bin/env python3
"""Weapon Proficiency shaping authoring: validate shaping catalogues and derive operation admission.

PROFICIENCY-1B section 3. A `ProficiencyShaping` definition holds the modification pool with its
draw weights, each entry's values at ranks 1-10, and the dust and orb costs. Every value is a cell:
UNKNOWN, or KNOWN with an evidence class. `validate` runs the schema and the semantic rules;
`admitted` says whether an operation may run, which is true only when every cell it reads is KNOWN
(PROFICIENCY-1B section 3.3). This module writes nothing under content/.
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

import proficiency_authoring as pa

HERE = Path(__file__).resolve().parent
SCHEMA = HERE / "shaping.schema.json"
SAMPLE = HERE / "samples" / "shaping-candidate.json"
RANKS = 10
MAX_POOL = 64  # PROF1B-RL-04
OFFER_SIZE = 3  # PROF1B-RL-02
OPERATIONS = (
    "MODIFY",
    "RANK_UP",
    "ORB_RANK",
    "RESHAPE_OFFER",
    "RESHAPE_CHOOSE",
    "CLEAR",
)


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def value_fields(kind: str) -> set[str]:
    """The numeric fields a perk kind carries per rank."""
    return {"probability", "multiplier"} if kind in pa.NO_VALUE_KINDS else {"value"}


def perk_validator() -> Draft202012Validator:
    schema = copy.deepcopy(pa.load_json(pa.SCHEMA))
    root = {
        "$schema": schema["$schema"],
        "$ref": "#/$defs/perk",
        "$defs": schema["$defs"],
    }
    return Draft202012Validator(root, format_checker=FormatChecker())


def known(cell: dict) -> bool:
    return cell["state"] == "KNOWN"


def entry_errors(where: str, entry: dict, perks: Draft202012Validator) -> list[str]:
    errors: list[str] = []
    identity = entry["identity"]
    kind = identity["kind"]
    fields = value_fields(kind)
    if fields & set(identity):
        errors.append(
            f"{where}: identity carries value fields {sorted(fields & set(identity))}"
        )
        return errors
    cooldown = kind == "spell_augment" and identity.get("augment") == "cooldown"
    placeholder = {name: 1 for name in fields}
    if cooldown:
        placeholder["value"] = -1
    if kind in pa.NO_VALUE_KINDS:
        placeholder["probability"] = 0.5
    if list(perks.iter_errors(dict(identity) | placeholder)):
        errors.append(f"{where}: identity is not a perk of the Proficiency schema")
        return errors
    for rank, cell in enumerate(entry["rank_values"], start=1):
        if not known(cell):
            continue
        values = cell["values"]
        if set(values) != fields:
            errors.append(
                f"{where} rank {rank}: values must be exactly {sorted(fields)}"
            )
            continue
        # The actual values must form a valid perk, with the catalogue's sign rules.
        if list(perks.iter_errors(dict(identity) | values)):
            errors.append(f"{where} rank {rank}: values are not a valid perk")
        elif "value" in values and (
            values["value"] >= 0 if cooldown else values["value"] <= 0
        ):
            sign = "negative" if cooldown else "positive"
            errors.append(f"{where} rank {rank}: {kind} value must be {sign}")
    return errors


def semantic_errors(
    catalogue: dict, proficiencies: dict[str, dict] | None = None
) -> list[str]:
    errors: list[str] = []
    perks = perk_validator()
    seen: set[str] = set()
    for shaping in catalogue["shapings"]:
        key = shaping["identity"]["key"]
        if key in seen:
            errors.append(f"duplicate key {key}")
        seen.add(key)
        target = shaping["proficiency"]["key"]
        if key.rsplit(".p", 1)[1] != target.rsplit(".p", 1)[1]:
            errors.append(f"{key}: does not shape {target}")
        if proficiencies is not None:
            definition = proficiencies.get(target)
            if definition is None:
                errors.append(f"{key}: unknown Proficiency {target}")
            elif (
                definition["identity"]["revision"] != shaping["proficiency"]["revision"]
            ):
                errors.append(f"{key}: Proficiency revision differs from the catalogue")
        pool = shaping["pool"]
        if known(pool):
            identities: list[str] = []
            for index, entry in enumerate(pool["entries"]):
                errors.extend(entry_errors(f"{key} entry {index}", entry, perks))
                identities.append(json.dumps(entry["identity"], sort_keys=True))
            if len(set(identities)) != len(identities):
                errors.append(f"{key}: pool entries must have distinct identities")
    return errors


def validate(
    catalogue: dict, proficiencies: dict[str, dict] | None = None
) -> list[str]:
    validator = Draft202012Validator(load_json(SCHEMA), format_checker=FormatChecker())
    errors = [
        f"{'/'.join(map(str, e.absolute_path)) or '<root>'}: {e.message}"
        for e in sorted(
            validator.iter_errors(catalogue), key=lambda e: list(e.absolute_path)
        )
    ]
    return errors or semantic_errors(catalogue, proficiencies)


def _pool_drawable(shaping: dict) -> bool:
    pool = shaping["pool"]
    return known(pool) and all(known(e["weight"]) for e in pool["entries"])


def _values_known(shaping: dict, rank: int, entry: int | None = None) -> bool:
    entries = shaping["pool"]["entries"]
    chosen = entries if entry is None else [entries[entry]]
    return all(known(e["rank_values"][rank - 1]) for e in chosen)


def admitted(
    shaping: dict,
    operation: str,
    *,
    slot: int | None = None,
    rank: int | None = None,
    entry: int | None = None,
) -> bool:
    """PROFICIENCY-1B section 3.3: an operation is admitted only when every cell it reads is KNOWN.

    `slot` is MODIFY's slot; `rank` is the row's current rank; `entry` is the row's entry index.
    Admission reads cells only. Runtime preconditions (POOL_TOO_SMALL, RANK_MAX, a pending offer)
    are section 5's checks and never turn into NOT_ADMITTED here.
    RESHAPE_CHOOSE reads no cell: it follows the pending offer, which was paid under its own
    revision, so it is admitted whenever an offer is pending (a runtime check, not content).
    """
    if operation not in OPERATIONS:
        raise ValueError(f"unknown operation {operation}")
    costs = shaping["costs"]
    if operation == "RESHAPE_CHOOSE":
        return True
    if operation == "CLEAR":
        return known(costs["clear"])
    if operation == "MODIFY":
        if slot not in (1, 2):
            raise ValueError("MODIFY needs slot 1 or 2")
        return (
            known(costs["modify"][f"slot_{slot}"])
            and _pool_drawable(shaping)
            and _values_known(shaping, 1)
        )
    if rank is None or not 1 <= rank <= RANKS:
        raise ValueError(f"{operation} needs the row's rank 1..{RANKS}")
    if operation == "RANK_UP" and rank == RANKS:
        return True  # reads no cell; RANK_MAX is a runtime check (section 5)
    if not known(shaping["pool"]):
        return False
    if operation == "RESHAPE_OFFER":
        # A pool of OFFER_SIZE or fewer is POOL_TOO_SMALL, a runtime check (section 5).
        return (
            known(costs["reshape_offer"])
            and _pool_drawable(shaping)
            and _values_known(shaping, rank)
        )
    if entry is None or not 0 <= entry < len(shaping["pool"]["entries"]):
        raise ValueError(f"{operation} needs the row's entry index")
    if operation == "RANK_UP":
        return known(costs["rank_steps"][rank - 1]) and _values_known(
            shaping, rank + 1, entry
        )
    return known(costs["orb_rank"]) and _values_known(shaping, RANKS, entry)


def committed_proficiencies() -> dict[str, dict]:
    catalogue = pa.load_json(pa.CATALOGUE)
    return {p["identity"]["key"]: p for p in catalogue["proficiencies"]}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("path", type=Path, nargs="?", default=SAMPLE)
    args = parser.parse_args()
    errors = validate(load_json(args.path), committed_proficiencies())
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        return 1
    print(f"{args.path}: valid")
    return 0


if __name__ == "__main__":
    sys.exit(main())
