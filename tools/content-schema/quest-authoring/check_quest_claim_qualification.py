"""Build and validate the auditable quest reward-claim qualification ledger."""

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
COVERAGE = ROOT / "docs/repository/QUEST_CALLBACK_COVERAGE.json"
LEDGER = ROOT / "docs/agents/evidence/quest-claim-qualification.json"
# W15b supports 308 claims; eight have source-selector dispatch PG grant
# coverage. Every other supported claim keeps its per-World qualification hold.
EXPECTED_DENOMINATOR = 300
BLOCKERS = {
    "WORLD_BINDING_AND_E2E_EVIDENCE_MISSING": "QUEST-QUAL-1",
    "LEVER_STEP_IN_OR_OFFERING_CALLBACK_MISSING": "QUEST-CALLBACKS-2",
    "DOOR_ITEM_TARGET_OR_STORAGE_CALLBACK_MISSING": "QUEST-CALLBACKS-3",
    "NATIVE_ITEM_ADMISSION_MISSING": "ITEM-1",
}


class DuplicateKey(ValueError):
    pass


def _object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateKey(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def read_json(path):
    return json.loads(Path(path).read_text(), object_pairs_hook=_object)


def inventory(coverage_path=COVERAGE, root=ROOT):
    coverage = read_json(coverage_path)
    rows = {}
    for quest in coverage.get("quests", []):
        for chest in quest.get("chests", []):
            if chest.get("status") != "supported_unqualified":
                continue
            key = chest.get("claim")
            if key in rows:
                raise ValueError(f"duplicate coverage claim: {key}")
            rows[key] = {"quest": quest.get("quest"), "coverage": chest}

    definitions = {}
    claim_dir = root / "content/interactions/reward_claims"
    for path in sorted(claim_dir.glob("reward-claim*.json")):
        for record in read_json(path).get("records", []):
            definition = record["definition"]
            key = definition["identity"]["key"]
            if key in rows:
                if key in definitions:
                    raise ValueError(f"duplicate reward definition: {key}")
                definitions[key] = definition
    missing = sorted(set(rows) - set(definitions))
    if missing:
        raise ValueError("missing reward definitions: " + ", ".join(missing))
    return coverage, rows, definitions


def expected_record(key, row, definition):
    placements = []
    for authored, covered in zip(definition["placements"], row["coverage"]["placements"]):
        selector = {
            "position": authored["source_binding"]["project_position"],
            "appearance_tibia_id": authored["appearance_tibia_id"],
            "source_unique_ids": authored["source_binding"]["legacy_unique_ids"],
        }
        if "action_id" in authored["source_binding"]:
            selector["action_id"] = authored["source_binding"]["action_id"]
        placements.append({
            "selector": selector,
            "source_witnesses": covered["sources"],
            "expected_reward": authored["reward"],
        })
    return {
        "quest": row["quest"],
        "placements": placements,
        "current_inventory_status": "supported_unqualified",
    }


def generated_ledger(coverage_path=COVERAGE, root=ROOT):
    coverage, rows, definitions = inventory(coverage_path, root)
    claims = {}
    for key in sorted(rows):
        record = expected_record(key, rows[key], definitions[key])
        record.update({
            "disposition": "blocked",
            "dependency": "QUEST-QUAL-1",
            "evidence": {
                "kind": "blocked",
                "blocker_type": "WORLD_BINDING_AND_E2E_EVIDENCE_MISSING",
                "blocked_on": "QUEST-QUAL-1",
                "missing_prerequisite": (
                    "No named production Use-route PostgreSQL test or retained scenario proves this "
                    "exact World selector, native reward mint, durable once-only receipt, reconnect "
                    "replay, and a second new-command refusal."
                ),
            },
        })
        claims[key] = record
    return {
        "schema_version": 1,
        "task": "QUEST-QUAL-1",
        "source_revisions": coverage["source_revisions"],
        "source_sha256": coverage["source_sha256"],
        "denominator": EXPECTED_DENOMINATOR,
        "disposition_totals": {
            "test": 0,
            "scenario": 0,
            "blocked": {"WORLD_BINDING_AND_E2E_EVIDENCE_MISSING": EXPECTED_DENOMINATOR},
        },
        "claims": claims,
    }


def _validate_locator(key, record, root):
    disposition = record.get("disposition")
    evidence = record.get("evidence")
    if not isinstance(evidence, dict) or evidence.get("kind") != disposition:
        return "evidence kind does not equal disposition"
    if disposition == "blocked":
        blocker = evidence.get("blocker_type")
        owner = evidence.get("blocked_on")
        if blocker not in BLOCKERS or BLOCKERS.get(blocker) != owner:
            return "invalid blocker type/task ownership"
        if record.get("dependency") != owner or not evidence.get("missing_prerequisite"):
            return "blocked record lacks matching dependency or missing prerequisite"
        return None
    if disposition == "test":
        path = evidence.get("path")
        name = evidence.get("fully_qualified_name")
        scenario = evidence.get("asserted_scenario")
        target_name = evidence.get("crate_or_target")
        observed = evidence.get("observed_result")
        if not all(isinstance(value, str) and value for value in
                   (path, name, scenario, target_name, observed)):
            return "malformed test locator"
        target = root / path
        leaf = re.escape(name.rsplit("::", 1)[-1])
        declaration = rf"\b(?:fn|def)\s+{leaf}\b"
        if not target.is_file() or not re.search(declaration, target.read_text()):
            return "invented or unresolved test locator"
        return None
    if disposition == "scenario":
        path = evidence.get("fixture")
        expected_hash = evidence.get("artifact_sha256")
        if not all(evidence.get(field) for field in ("scenario_id", "runner_command", "expected_receipt")):
            return "malformed scenario locator"
        target = root / str(path)
        if not target.is_file() or hashlib.sha256(target.read_bytes()).hexdigest() != expected_hash:
            return "unresolved scenario artifact or hash"
        return None
    return "unknown disposition"


def validate(ledger_path=LEDGER, coverage_path=COVERAGE, root=ROOT):
    errors = []
    try:
        ledger = read_json(ledger_path)
        coverage, rows, definitions = inventory(coverage_path, root)
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as exc:
        return [str(exc)], Counter()
    if coverage.get("counts", {}).get("supported_unqualified") != EXPECTED_DENOMINATOR:
        errors.append(f"coverage denominator drift (expected {EXPECTED_DENOMINATOR} supported_unqualified)")
    if ledger.get("denominator") != EXPECTED_DENOMINATOR:
        errors.append("ledger denominator drift")
    if ledger.get("source_revisions") != coverage.get("source_revisions") or ledger.get("source_sha256") != coverage.get("source_sha256"):
        errors.append("stale pinned source witness")
    claims = ledger.get("claims")
    if not isinstance(claims, dict):
        return errors + ["claims must be an object keyed by canonical claim key"], Counter()
    missing, stale = sorted(set(rows) - set(claims)), sorted(set(claims) - set(rows))
    errors.extend(f"missing claim: {key}" for key in missing)
    errors.extend(f"stale claim: {key}" for key in stale)
    counts = Counter()
    for key in sorted(set(rows) & set(claims)):
        record = claims[key]
        expected = expected_record(key, rows[key], definitions[key])
        for field, value in expected.items():
            if record.get(field) != value:
                errors.append(f"{key}: source/reward/placement drift in {field}")
        issue = _validate_locator(key, record, root)
        if issue:
            errors.append(f"{key}: {issue}")
        disposition = record.get("disposition", "malformed")
        if disposition == "blocked" and isinstance(record.get("evidence"), dict):
            counts[f"blocked:{record['evidence'].get('blocker_type', 'malformed')}"] += 1
        else:
            counts[disposition] += 1
    reported = {
        "test": counts.get("test", 0),
        "scenario": counts.get("scenario", 0),
        "blocked": {
            name.removeprefix("blocked:"): count
            for name, count in sorted(counts.items()) if name.startswith("blocked:")
        },
    }
    if ledger.get("disposition_totals") != reported:
        errors.append("ledger disposition summary drift")
    return errors, counts


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="validate the checked-in ledger")
    args = parser.parse_args(argv)
    if not args.check:
        LEDGER.write_text(json.dumps(generated_ledger(), indent=2, sort_keys=True) + "\n")
        return 0
    errors, counts = validate()
    print("quest claim qualification totals:")
    for disposition, count in sorted(counts.items()):
        print(f"  {disposition}: {count}")
    if errors:
        print("qualification errors:", file=sys.stderr)
        for error in errors:
            print(f"  {error}", file=sys.stderr)
        return 1
    print(f"validated {sum(counts.values())} supported_unqualified claim keys")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
