"""Fail closed on absent scenario evidence; legacy skip messages are not verdicts."""

import argparse
import math
from pathlib import Path
import re


RESULT = re.compile(
    r"^SEAM_EVIDENCE e2e_all scenario=([A-Z]\d\d[a-z]?\.[a-z0-9-]+) "
    r"status=(PASS|FAIL|SKIPPED|MISSING-SERVER|MISSING-CLIENT) seconds=([0-9]+(?:\.[0-9]+)?)"
    r"(?: evidence=(\S+))?$"
)
ROW = re.compile(r"^\| ([A-Z]\d\d[a-z]?) \| ([^|]+) \|")
MAP_CHECK = re.compile(
    r"^(?:SEAM_EVIDENCE )?MAP_CHECK ([a-z][a-z0-9_]*) "
    r"(\d+,\d+,\d+) (PASS|FAIL|SKIPPED) (\S.*)$"
)
MAP_SUMMARY = re.compile(
    r"^(?:SEAM_EVIDENCE )?MAP_SUMMARY kind=([a-z][a-z0-9_]*) "
    r"pass=(\d+) fail=(\d+) skipped=(\d+)$"
)


def map_results(text):
    checks, totals, declared = [], {}, {}
    seen = set()
    for line in text.splitlines():
        if not re.match(r"^(?:SEAM_EVIDENCE )?MAP_(?:CHECK|SUMMARY)\b", line):
            continue
        check, aggregate = MAP_CHECK.fullmatch(line), MAP_SUMMARY.fullmatch(line)
        if check:
            kind, position, status, reason = check.groups()
            coordinates = tuple(map(int, position.split(",")))
            if coordinates[0] > 65535 or coordinates[1] > 65535 or coordinates[2] > 15:
                raise ValueError("map position outside bundle coordinate bounds")
            key = kind, coordinates
            if key in seen or kind in declared:
                raise ValueError("duplicate or late MAP_CHECK")
            seen.add(key)
            counts = totals.setdefault(kind, [0, 0, 0])
            counts[("PASS", "FAIL", "SKIPPED").index(status)] += 1
            checks.append((kind, position, status, reason))
        elif aggregate:
            kind, *counts = aggregate.groups()
            if kind in declared:
                raise ValueError("duplicate MAP_SUMMARY")
            declared[kind] = list(map(int, counts))
        else:
            raise ValueError("malformed map result")
    if totals != declared:
        raise ValueError("MAP_SUMMARY does not match MAP_CHECK counts")
    return checks, declared

COMBAT_CHECK = re.compile(r"^COMBAT_CHECK ([A-Z]\d\d[a-z]?) ([a-z0-9-]+) (PASS|FAIL|SKIPPED) (\S.*)$")
COMBAT_SUMMARY = re.compile(r"^COMBAT_SUMMARY row=([A-Z]\d\d[a-z]?) pass=(\d+) fail=(\d+) skipped=(\d+)$")
# A summary cannot qualify a row by omitting the difficult assertions.
COMBAT_CASES = {
    "C01": {
        "offensive-stand", "offensive-chase", "balanced-stand", "balanced-chase",
        "defensive-stand", "defensive-chase", "armor-defence", "kill-xp",
        "corpse-loot", "relog-xp-loot", "monster-target-follow-disengage",
    },
    "C02": {"bow-ammo", "wand-mana-element", "rod-mana-element"},
    "C04": {"words-mana", "spell-cooldown", "group-cooldown", "pz-aggression"},
    "C07": {"attack-rune", "field-step-expiry"},
    "C09": {"food-regeneration"},
    "C16a": {"wall-block-expiry"},
    "C17a": {"haste-speed"},
    "C17b": {"invisible-targeting"},
    "C18a": {"directional-hit"},
}


def combat_results(text):
    checks, totals = {}, {}
    for line in text.splitlines():
        if not (line.startswith("COMBAT_CHECK") or line.startswith("COMBAT_SUMMARY")):
            continue
        check, total = COMBAT_CHECK.fullmatch(line), COMBAT_SUMMARY.fullmatch(line)
        if check:
            row, case, status, reason = check.groups()
            if row not in COMBAT_CASES or case not in COMBAT_CASES[row]:
                raise ValueError(f"unknown combat check: {row}.{case}")
            if row in totals or case in checks.get(row, {}):
                raise ValueError(f"duplicate or late combat check: {row}.{case}")
            if status == "SKIPPED" and not re.fullmatch(r"content:[A-Za-z0-9:_./-]+", reason):
                raise ValueError(f"missing exact combat content id: {row}.{case}")
            checks.setdefault(row, {})[case] = (status, reason)
        elif total:
            row, passed, failed, skipped = total.groups()
            if row in totals or row not in checks:
                raise ValueError(f"duplicate or orphan combat summary: {row}")
            expected = tuple(sum(status == wanted for status, _ in checks[row].values())
                             for wanted in ("PASS", "FAIL", "SKIPPED"))
            if expected != (int(passed), int(failed), int(skipped)):
                raise ValueError(f"combat summary count mismatch: {row}")
            totals[row] = expected
        else:
            raise ValueError("malformed combat evidence")
    if checks.keys() != totals.keys():
        raise ValueError("missing combat summary")
    found = {}
    for row, cases in checks.items():
        missing = COMBAT_CASES[row] - cases.keys()
        gaps = [f"{case}:{status}:{reason}" for case, (status, reason) in sorted(cases.items())
                if status != "PASS"]
        if missing:
            gaps.append("missing-checks:" + ",".join(sorted(missing)))
        status = ("FAIL" if missing or totals[row][1] else
                  "SKIPPED" if totals[row][2] else "PASS")
        found[f"{row}.full"] = (status, 0.0, "; ".join(gaps) or "combat checks complete")
    return found


def matrix_rows(text):
    rows = []
    for line in text.splitlines():
        match = ROW.match(line)
        if match:
            row_id, name = match.groups()
            if f"`{row_id}.full`:" not in line:
                raise ValueError(f"missing complete scenario requirement: {row_id}")
            if any(row_id == existing[0] for existing in rows):
                raise ValueError(f"duplicate matrix row: {row_id}")
            rows.append((row_id, name.strip()))
    if not rows:
        raise ValueError("empty scenario matrix")
    return rows


def results(text):
    found = {}
    for line in text.splitlines():
        if "SEAM_EVIDENCE e2e_all " not in line:
            continue
        match = RESULT.fullmatch(line)
        if not match:
            raise ValueError("malformed E2E-ALL result")
        scenario, status, seconds, evidence = match.groups()
        seconds = float(seconds)
        if not math.isfinite(seconds):
            raise ValueError(f"non-finite duration: {scenario}")
        if scenario in found:
            raise ValueError(f"duplicate scenario result: {scenario}")
        # A skip needs an exact missing content id; absence of implementation
        # needs a source citation. Legacy skipped= strings satisfy neither.
        if status in {"SKIPPED", "MISSING-SERVER", "MISSING-CLIENT"} and not evidence:
            raise ValueError(f"missing source/content evidence: {scenario}")
        if status == "SKIPPED" and not re.fullmatch(r"content:[A-Za-z0-9:_./-]+", evidence):
            raise ValueError(f"missing exact content id: {scenario}")
        if status == "MISSING-SERVER" and not re.fullmatch(r"source:[A-Za-z0-9_./-]+:\d+", evidence):
            raise ValueError(f"missing file:line source evidence: {scenario}")
        found[scenario] = (status, seconds, evidence)
    for scenario, result in combat_results(text).items():
        if scenario in found:
            raise ValueError(f"duplicate scenario result: {scenario}")
        found[scenario] = result
    return found


def summary(rows, found, node_status=0, client_status=0, map_evidence=None):
    known = {key for key, _ in rows}
    if any(scenario.split(".")[0] not in known for scenario in found):
        raise ValueError("result is not a mechanic in the matrix")
    output = ["mechanic | status | seconds | evidence / gap"]
    complete = True
    for row_id, name in rows:
        status, seconds, evidence = found.get(
            f"{row_id}.full", ("FAIL", 0.0, "complete scenario unimplemented or not reached")
        )
        complete &= status == "PASS"
        output.append(f"{row_id} {name} | {status} | {seconds:.3f} | {evidence or ''}")
    for scenario, (status, seconds, evidence) in sorted(found.items()):
        if not scenario.endswith(".full"):
            output.append(f"slice {scenario} | {status} | {seconds:.3f} | {evidence or 'bounded assertion only'}")
            complete &= status == "PASS"
    for name, status in [("node_boot", node_status), ("client_tests", client_status)]:
        output.append(f"{name} | {'PASS' if status == 0 else 'FAIL'} | - | exit={status}")
    if map_evidence is not None:
        checks, totals = map_evidence
        for kind, position, status, reason in checks:
            output.append(f"map {kind} {position} | {status} | - | {reason}")
            complete &= status == "PASS"
        for kind, (passed, failed, skipped) in sorted(totals.items()):
            output.append(f"map summary {kind} | pass={passed} fail={failed} skipped={skipped}")
    return "\n".join(output), complete and node_status == 0 and client_status == 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", required=True, type=Path)
    parser.add_argument("--log", required=True, type=Path)
    parser.add_argument("--node-status", required=True, type=int)
    parser.add_argument("--client-status", required=True, type=int)
    args = parser.parse_args()
    try:
        log = args.log.read_text()
        output, complete = summary(
            matrix_rows(args.matrix.read_text()), results(log),
            args.node_status, args.client_status, map_results(log),
        )
    except (OSError, ValueError) as error:
        parser.exit(1, f"E2E-ALL FAIL: {error}\n")
    print(output)
    return 0 if complete else 1


if __name__ == "__main__":
    raise SystemExit(main())
