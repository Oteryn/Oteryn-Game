"""Census the outcome of converting one pinned engine's items.xml universe.

Runs `engine_items.convert_item` over every item id in the engine's own pinned
`items.xml`, validates every structurally-produced bundle with `validate_item.validate`,
and writes one deterministic, size-bounded JSON summary (`samples/population-<engine>-
<short-revision>.json`). This is a census, not a corpus: no per-item rows are committed,
only counters, top blockers/validator errors (each capped at 5 example keys) and per-raw-
field coverage.

The owner-approved `ADOPT_CRYSTAL_DELIVERY_LIST@ff7ede5` rule (`engine_items.RULE_ID`)
decides `delivery_task_eligible` for every converted item with a CW2 B1 allocator key, so
the emitted bundle now carries the real field and is validated as-is (no throwaway copy).
A Canary run additionally requires `--rule-source <crystal checkout>` so the rule can read
the pinned Crystal delivery list; `engine_items.load_engine_sources` refuses to run
without it. Outcome counters: `blocked` covers any residual blocker other than
`sprite_atlas_not_admitted`/`delivery_task_decision_not_admitted`; `pending_author_decision`
covers items whose only residual blockers are those two (in practice this is now only an
item whose Delivery Task decision was for some reason not admitted, since the rule
otherwise decides every keyed item); `fully_resolved` covers items with no residual
blocker but `sprite_atlas_not_admitted`.

`--self-check` runs fixed, engine-specific assertions (both engines are required, never
skipped). `--check` regenerates the census in memory and diffs it against the committed
samples file instead of writing.
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
    RULE_ID,
    convert_item,
    load_engine_sources,
)

ROOT = Path(__file__).resolve().parent
DEFAULT_SAMPLE_NAMES = {
    "crystal": "population-crystal-ff7ede5.json",
    "canary": "population-canary-47dfd51f.json",
}
SELF_CHECK_IDS = (2854, 2874, 3155, 3288, 3388, 3585)
CRYSTAL_KNOWN_DELIVERY_MEMBER_ID = 811
CANARY_KNOWN_DELIVERY_MEMBER_ID = 3031
CANARY_KNOWN_DELIVERY_NON_MEMBER_ID = 3585
EXEMPT_BLOCKERS = ("sprite_atlas_not_admitted", "delivery_task_decision_not_admitted")


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


def assert_ids_convert_and_validate(sources, item_ids):
    """These ids are also the six committed real examples: the rule must decide the
    same way as those examples' own `false` / `crystal_list_non_member` decision."""
    for item_id in item_ids:
        item, dependencies, report = convert_item(sources, item_id)
        assert item is not None, (item_id, report)
        decision = report["delivery_task"]["decision"]
        assert item["delivery_task_eligible"] == decision["eligible"], item_id
        assert decision == {
            "rule": RULE_ID,
            "basis": "crystal_list_non_member",
            "eligible": False,
        }, (item_id, decision)
        errors, _warnings = validate_item.validate(item, dependencies)
        assert not errors, (item_id, errors)


def self_check(sources, engine):
    """Engine-specific assertions for both engines; an engine without any is a bug."""
    identity_key, _ = sources["identity_index"][3288]
    assert identity_key == "oteryn:item.tibia.i3288", identity_key

    item, _dependencies, _report = convert_item(sources, 3288)
    assert item["physical"]["weight"] == {"value": "42.00", "unit": "oz"}
    assert item["weapon"]["attack"] == 48
    assert item["weapon"]["defense"] == 35
    slots = {p["slot"] for p in item["equipment"]["patterns"]}
    assert slots == {"right_hand", "left_hand"}, slots

    if engine == "canary":
        _, _, member_report = convert_item(sources, CANARY_KNOWN_DELIVERY_MEMBER_ID)
        observation = member_report["delivery_task"]["observation"]
        assert observation["member"] is True, member_report
        # The Canary Task Board pool is observation only: it never decides. This id is
        # a Canary weeklyItems member but not a Crystal delivery-list member, so the
        # rule's decision must still be `false` / `crystal_list_non_member`.
        assert member_report["delivery_task"]["decision"] == {
            "rule": RULE_ID,
            "basis": "crystal_list_non_member",
            "eligible": False,
        }, member_report
        _, _, non_member_report = convert_item(
            sources, CANARY_KNOWN_DELIVERY_NON_MEMBER_ID
        )
        assert non_member_report["delivery_task"]["observation"]["member"] is False, (
            non_member_report
        )
        assert_ids_convert_and_validate(sources, SELF_CHECK_IDS)
        print(
            json.dumps(
                {
                    "self_check": "ok",
                    "engine": engine,
                    "ids": [
                        3288,
                        CANARY_KNOWN_DELIVERY_MEMBER_ID,
                        CANARY_KNOWN_DELIVERY_NON_MEMBER_ID,
                        *SELF_CHECK_IDS,
                    ],
                }
            )
        )
        return

    if engine == "crystal":
        assert CRYSTAL_KNOWN_DELIVERY_MEMBER_ID in sources["delivery_member_ids"]
        _, _, member_report = convert_item(sources, CRYSTAL_KNOWN_DELIVERY_MEMBER_ID)
        assert member_report["delivery_task"]["decision"] == {
            "rule": RULE_ID,
            "basis": "crystal_list_member",
            "eligible": True,
        }, member_report
        assert_ids_convert_and_validate(sources, SELF_CHECK_IDS)
        print(
            json.dumps(
                {
                    "self_check": "ok",
                    "engine": engine,
                    "ids": [CRYSTAL_KNOWN_DELIVERY_MEMBER_ID, *SELF_CHECK_IDS],
                }
            )
        )
        return

    raise SystemExit(f"no self-check assertions defined for engine {engine!r}")


def build_census(sources, engine, bundles_dir=None):
    """Return (result, bundles): the deterministic census dict and its {key: bundle} map."""
    outcome = Counter()
    by_profile = Counter()
    family_profile_basis_counts = Counter()
    identity_basis_counts = Counter()
    delivery_observation_counts = Counter()
    delivery_decision_counts = Counter()
    delivery_override_items = 0
    crystal_list_in_items_xml_not_converted = 0
    blocker_counts = Counter()
    blocker_examples = defaultdict(list)
    validator_error_counts = Counter()
    validator_error_examples = defaultdict(list)
    field_items = Counter()
    routed_non_item_counts = Counter()
    routed_non_item_examples = defaultdict(list)
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

        routed_non_item = report.get("routed_non_item")
        if routed_non_item:
            # A non-Item owner decision the catalogs already made (WorldObject/Terrain
            # corpse behavior, a sprite-sheet placeholder slot, a Terrain primarytype):
            # never a converter failure, so it is counted apart from `not_converted`.
            outcome["routed_non_item"] += 1
            owner_key = f"{routed_non_item['owner']}:{routed_non_item['reason']}"
            routed_non_item_counts[owner_key] += 1
            if len(routed_non_item_examples[owner_key]) < 5 and report.get("key"):
                routed_non_item_examples[owner_key].append(report["key"])
            continue

        if not report["converted"]:
            outcome["not_converted"] += 1
            if item_id in sources["crystal_list_ids"]:
                crystal_list_in_items_xml_not_converted += 1
            for blocker in report["blockers"]:
                blocker_counts[blocker] += 1
                if len(blocker_examples[blocker]) < 5 and report.get("key"):
                    blocker_examples[blocker].append(report["key"])
            continue

        key = report["key"]
        family_profile_basis_counts[
            report.get("family_profile_basis") or "engine_attribute"
        ] += 1
        delivery_task = report["delivery_task"]
        delivery_observation_counts[delivery_task["observation"]["member"]] += 1
        delivery_decision_counts[delivery_task["decision"]["eligible"]] += 1
        if delivery_task["decision"]["basis"] == "override":
            delivery_override_items += 1
        errors, _warnings = validate_item.validate(item, dependencies)
        if errors:
            outcome["structure_invalid"] += 1
            for error in errors:
                bucket = normalize_message(error)
                validator_error_counts[bucket] += 1
                if len(validator_error_examples[bucket]) < 5:
                    validator_error_examples[bucket].append(key)
        else:
            residual_other = [b for b in report["blockers"] if b not in EXEMPT_BLOCKERS]
            if residual_other:
                outcome["blocked"] += 1
            elif "delivery_task_decision_not_admitted" in report["blockers"]:
                outcome["pending_author_decision"] += 1
            else:
                outcome["fully_resolved"] += 1
        for blocker in report["blockers"]:
            blocker_counts[blocker] += 1
            if len(blocker_examples[blocker]) < 5:
                blocker_examples[blocker].append(key)

        bundles[key] = {"item": item, "dependencies": dependencies}
        if bundles_dir:
            target_dir = bundles_dir / engine
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
            "engine": engine,
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
        "family_profile_basis": dict(sorted(family_profile_basis_counts.items())),
        "identity_basis": dict(sorted(identity_basis_counts.items())),
        "routed_non_item": [
            {
                "owner_reason": owner_key,
                "items": count,
                "examples": routed_non_item_examples[owner_key],
            }
            for owner_key, count in sorted(
                routed_non_item_counts.items(), key=lambda row: (-row[1], row[0])
            )
        ],
        "delivery_task": {
            "observation": {
                "source": sources["delivery_source"],
                "member": delivery_observation_counts[True],
                "not_member": delivery_observation_counts[False],
            },
            "decision": {
                "rule": RULE_ID,
                "eligible_true": delivery_decision_counts[True],
                "eligible_false": delivery_decision_counts[False],
                "override": delivery_override_items,
            },
            "crystal_list": {
                "entries": sources["crystal_list_entries"],
                "unique": len(sources["crystal_list_ids"]),
                "not_in_items_xml": sorted(
                    sources["crystal_list_ids"] - set(sources["items"])
                ),
                "in_items_xml_not_converted": crystal_list_in_items_xml_not_converted,
            },
        },
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
    return result, bundles


def census_document_bytes(result):
    return (
        json.dumps(result, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    ).encode("utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--engine", choices=sorted(ENGINES), required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--bundles", type=Path, help="write every item bundle here")
    parser.add_argument("--self-check", action="store_true")
    parser.add_argument(
        "--check",
        action="store_true",
        help=(
            "regenerate the census in memory and diff it against the committed "
            "samples file; exits 1 on drift and never writes"
        ),
    )
    parser.add_argument(
        "--rule-source",
        type=Path,
        help=(
            "pinned Crystal checkout the Delivery Task rule reads its delivery list "
            "from; required for --engine canary, ignored for --engine crystal"
        ),
    )
    parser.add_argument(
        "--overrides",
        type=Path,
        help="delivery-task-overrides.json to use instead of the committed one",
    )
    parser.add_argument(
        "--wiki-fallback",
        type=Path,
        help="items-family-fallback.json snapshot to use instead of the committed one",
    )
    args = parser.parse_args()

    sources = load_engine_sources(
        args.engine,
        args.source,
        rule_source=args.rule_source,
        overrides_path=args.overrides,
        wiki_fallback_path=args.wiki_fallback,
    )
    out_path = args.out or ROOT / "samples" / DEFAULT_SAMPLE_NAMES[args.engine]

    result, _bundles = build_census(sources, args.engine, bundles_dir=args.bundles)
    candidate_bytes = census_document_bytes(result)

    if args.check:
        if not out_path.is_file():
            raise SystemExit(f"no committed census at {out_path} to check against")
        committed_bytes = out_path.read_bytes()
        if candidate_bytes != committed_bytes:
            raise SystemExit(
                f"census drift detected for {args.engine}: regenerated census differs "
                f"from committed {out_path}"
            )
        print(json.dumps({"check": "ok", "engine": args.engine, "out": str(out_path)}))
    else:
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_bytes(candidate_bytes)
        print(json.dumps({"outcome": dict(result["outcome"]), "out": str(out_path)}))

    if args.self_check:
        self_check(sources, args.engine)


if __name__ == "__main__":
    main()
