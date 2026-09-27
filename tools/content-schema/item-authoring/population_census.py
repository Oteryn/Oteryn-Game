"""Census the outcome of converting one pinned engine's items.xml universe.

Runs `engine_items.convert_item` over every item id in the engine's own pinned
`items.xml`, validates every structurally-produced bundle with `validate_item.validate`,
and writes one deterministic, size-bounded JSON summary (`samples/population-<engine>-
<short-revision>.json`). This is a census, not a corpus: no per-item rows are committed,
only counters, top blockers/validator errors (each capped at 5 example keys) and per-raw-
field coverage.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

import validate_item
from engine_items import (
    ENGINES,
    IMPLEMENTED_FIELDS,
    ROUTED_STATUSES,
    convert_item,
    load_engine_sources,
)

ROOT = Path(__file__).resolve().parent
SELF_CHECK_IDS = (2854, 2874, 3155, 3288, 3388, 3585)


def normalize_message(message):
    """Strip item-specific keys/numbers so validator errors bucket by shape."""
    text = re.sub(r"'oteryn:[^']*'", "'<key>'", message)
    text = re.sub(r"\d+", "<n>", text)
    return text


def canonical_bytes(value):
    return json.dumps(
        value, sort_keys=True, ensure_ascii=False, separators=(",", ":")
    ).encode("utf-8")


def field_coverage_table(disposition, field_items):
    table = {}
    for field, row in disposition.items():
        status = row["status"]
        if status == "mapped":
            converter = "implemented" if field in IMPLEMENTED_FIELDS else "missing"
        elif status in ROUTED_STATUSES:
            converter = "routed"
        else:
            converter = "missing"
        table[field] = {
            "items": field_items.get(field, 0),
            "disposition": status,
            "converter": converter,
        }
    return table


def self_check(sources, engine):
    if engine != "crystal":
        print(
            json.dumps(
                {"self_check": "skipped", "reason": "assertions are Crystal-specific"}
            )
        )
        return
    identity_key, _ = sources["identity_index"][3288]
    assert identity_key == "oteryn:item.registry.i00003167", identity_key

    item, dependencies, report = convert_item(sources, 3288)
    assert item["physical"]["weight"] == {"value": "42.00", "unit": "oz"}
    assert item["weapon"]["attack"] == 48
    assert item["weapon"]["defense"] == 35
    slots = {p["slot"] for p in item["equipment"]["patterns"]}
    assert slots == {"right_hand", "left_hand"}, slots

    for item_id in SELF_CHECK_IDS:
        item, dependencies, report = convert_item(sources, item_id)
        assert item is not None, (item_id, report)
        errors, _warnings = validate_item.validate(item, dependencies)
        assert not errors, (item_id, errors)
    print(json.dumps({"self_check": "ok", "ids": list(SELF_CHECK_IDS)}))


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--engine", choices=sorted(ENGINES), required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--bundles", type=Path, help="write every item bundle here")
    parser.add_argument("--self-check", action="store_true")
    args = parser.parse_args()

    sources = load_engine_sources(args.engine, args.source)
    default_names = {
        "crystal": "population-crystal-ff7ede5.json",
        "canary": "population-canary-47dfd51f.json",
    }
    out_path = args.out or ROOT / "samples" / default_names[args.engine]

    outcome = Counter()
    by_profile = Counter()
    identity_basis_counts = Counter()
    delivery_counts = Counter()
    blocker_counts = Counter()
    blocker_examples = defaultdict(list)
    validator_error_counts = Counter()
    validator_error_examples = defaultdict(list)
    field_items = Counter()
    bundles = {}

    for item_id in sorted(sources["items"]):
        item, dependencies, report = convert_item(sources, item_id)
        outcome["items"] += 1
        for field in report.get("field_status", {}):
            field_items[field] += 1
        by_profile[report.get("family_profile") or "unresolved"] += 1
        basis = report.get("identity_basis")
        if basis:
            identity_basis_counts[basis] += 1

        if not report["converted"]:
            outcome["not_converted"] += 1
            for blocker in report["blockers"]:
                blocker_counts[blocker] += 1
                if len(blocker_examples[blocker]) < 5 and report.get("key"):
                    blocker_examples[blocker].append(report["key"])
            continue

        key = report["key"]
        delivery_counts[report["delivery_task_basis"]] += 1
        errors, _warnings = validate_item.validate(item, dependencies)
        if errors:
            outcome["structure_invalid"] += 1
            for error in errors:
                bucket = normalize_message(error)
                validator_error_counts[bucket] += 1
                if len(validator_error_examples[bucket]) < 5:
                    validator_error_examples[bucket].append(key)
        else:
            residual = [
                b for b in report["blockers"] if b != "sprite_atlas_not_admitted"
            ]
            outcome["blocked" if residual else "fully_resolved"] += 1
        for blocker in report["blockers"]:
            blocker_counts[blocker] += 1
            if len(blocker_examples[blocker]) < 5:
                blocker_examples[blocker].append(key)

        bundles[key] = {"item": item, "dependencies": dependencies}
        if args.bundles:
            target_dir = args.bundles / args.engine
            target_dir.mkdir(parents=True, exist_ok=True)
            (target_dir / f"{item_id}.json").write_text(
                json.dumps(
                    {"item": item, "dependencies": dependencies, "report": report},
                    indent=2,
                    sort_keys=True,
                    ensure_ascii=False,
                )
                + "\n",
                encoding="utf-8",
            )

    bundle_digest = hashlib.sha256(
        canonical_bytes({key: bundles[key] for key in sorted(bundles)})
    ).hexdigest()

    result = {
        "source": {
            "engine": args.engine,
            "profile": sources["profile"],
            "repository": sources["repository"],
            "revision": sources["revision"],
            "artifact_digests": sources["artifact_digests"],
        },
        "scope": (
            "Every item id in this engine's pinned data/items/items.xml "
            "(fromid/toid ranges expanded); not a placed map object or instance."
        ),
        "outcome": dict(sorted(outcome.items())),
        "by_profile": dict(sorted(by_profile.items())),
        "identity_basis": dict(sorted(identity_basis_counts.items())),
        "delivery_task_eligible": dict(sorted(delivery_counts.items())),
        "blockers_by_item_count": [
            {"blocker": blocker, "items": count, "examples": blocker_examples[blocker]}
            for blocker, count in sorted(
                blocker_counts.items(), key=lambda row: (-row[1], row[0])
            )
        ],
        "validator_errors_by_count": [
            {
                "error": bucket,
                "items": count,
                "examples": validator_error_examples[bucket],
            }
            for bucket, count in sorted(
                validator_error_counts.items(), key=lambda row: (-row[1], row[0])
            )
        ],
        "field_coverage": field_coverage_table(sources["disposition"], field_items),
        "bundle_digest": bundle_digest,
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(
        json.dumps(result, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(json.dumps({"outcome": dict(result["outcome"]), "out": str(out_path)}))

    if args.self_check:
        self_check(sources, args.engine)


if __name__ == "__main__":
    main()
