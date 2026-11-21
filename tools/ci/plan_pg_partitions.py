#!/usr/bin/env python3
"""Assign the PostgreSQL nextest inventory to disjoint, duration-balanced partitions."""

import argparse
import json
from pathlib import Path

ARCHIVES = ("pg-e2e", "pg-lib", "pg-lib-1", "pg-lib-2", "pg-lib-3")
DISPATCH_MOUNT = "gameplay_transport::monk_save::postgres_tests::"
RUNE_CASE_PREFIX = f"oteryn-gameplay-dispatch {DISPATCH_MOUNT}rune_use_bridge::rune_use_bridge_postgres_"
OLD_RUNE_CASE = f"{RUNE_CASE_PREFIX}real_dispatch_casts_once_and_reloads_one_debit"
EXIVA_CASE_PREFIX = f"oteryn-gameplay-dispatch {DISPATCH_MOUNT}exiva_world_types::p17_exiva_rules_"
OLD_EXIVA_CASES = tuple(f"{EXIVA_CASE_PREFIX}{world}_postgres"
                       for world in ("optional_pvp", "open_pvp", "enforced_pvp"))


def estimate(case, durations):
    key = " ".join(case)
    if key in durations:
        return durations[key]
    # The canonical dedicated target has the same named cases as the library
    # harness. Reuse its measurements until this target has its own observations;
    # filters and inventory identity always retain the real binary ID.
    if case[0] == "oteryn-gameplay-dispatch::oteryn-gameplay-dispatch":
        key = f"oteryn-gameplay-dispatch {case[1]}"
        if key in durations:
            return durations[key]
    if key.startswith(RUNE_CASE_PREFIX) and OLD_RUNE_CASE in durations:
        # Ten isolated cases replaced the previous serial test. Use an equal
        # share of its observed runtime until each case has its own measurement.
        return (durations[OLD_RUNE_CASE] + 9) // 10
    for old_case in OLD_EXIVA_CASES:
        if key.startswith(old_case + "_") and old_case in durations:
            return (durations[old_case] + 8) // 9
    return 5 if "postgres" in case[1] else 1


def read_cases(plan_dir, optional):
    cases_by_archive = {}
    seen = set()
    for archive in ARCHIVES:
        path = plan_dir / f"{archive}.json"
        if optional and not path.exists():
            cases_by_archive[archive] = []
            continue
        listing = json.loads(path.read_text())
        cases = [
            (suite["binary-id"], name)
            for suite in listing["rust-suites"].values()
            for name, case in suite["testcases"].items()
            if case["filter-match"]["status"] == "matches"
            if archive not in ("pg-lib-2", "pg-lib-3") or name.startswith(DISPATCH_MOUNT)
        ]
        if not cases:
            raise ValueError(f"{archive}: no runnable tests in archive")
        if len(set(cases)) != len(cases) or seen.intersection(cases):
            raise ValueError(f"{archive}: duplicate test cases")
        seen.update(cases)
        cases_by_archive[archive] = cases
    if not seen:
        raise ValueError("no runnable PostgreSQL test archives")
    return cases_by_archive


def write_partition(plan_dir, index, total, durations, optional):
    if not 0 <= index < total:
        raise ValueError(f"partition index {index} is invalid for {total} partitions")
    cases_by_archive = read_cases(plan_dir, optional)
    shares = [{archive: [] for archive in ARCHIVES} for _ in range(total)]
    load = [0] * total
    # Place the longest cases first across all archives, including the library
    # suites, since every worker executes its share of those archives in sequence.
    ordered = sorted(
        (-estimate(case, durations), archive, case)
        for archive, cases in cases_by_archive.items()
        for case in cases
    )
    for negative_duration, archive, case in ordered:
        share = min(range(total), key=lambda i: (load[i], i))
        shares[share][archive].append(case)
        load[share] -= negative_duration

    overrides = []
    for archive, cases in shares[index].items():
        assigned = sorted(case for share in shares for case in share[archive])
        if assigned != sorted(cases_by_archive[archive]):
            raise ValueError(f"{archive}: the plan does not run every listed test exactly once")
        filters = []
        for binary, name in sorted(cases):
            filterset = f"binary_id(={binary}) & test(={name})"
            filters.append(f"--filterset={filterset}\n")
            duration = estimate((binary, name), durations)
            if duration >= 30:
                overrides.append(
                    f"[[profile.default.overrides]]\nfilter = '{filterset}'\n"
                    f"priority = {min(100, duration // 5)}\n"
                )
        (plan_dir / f"{archive}.filters").write_text("".join(filters))
        print(f"{archive}: {len(cases)} of {len(assigned)} tests")
    (plan_dir / "priority.toml").write_text("".join(overrides))
    print(f"Estimated partition load: {load[index]}s")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("plan_dir", type=Path)
    parser.add_argument("index", type=int)
    parser.add_argument("total", type=int)
    parser.add_argument("durations", type=Path)
    parser.add_argument("--optional-archives", action="store_true")
    args = parser.parse_args()
    write_partition(args.plan_dir, args.index, args.total,
                    json.loads(args.durations.read_text()), args.optional_archives)


if __name__ == "__main__":
    main()
