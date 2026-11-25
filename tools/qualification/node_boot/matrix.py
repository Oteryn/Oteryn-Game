#!/usr/bin/env python3
"""Plan, run-check and summarise node boot shards on hosted runners.

One node boot machine runs the basic seam, every journey group and only the
first row batch. This tool splits that work into shards. Each shard is one
hosted machine running ``tools/qualification/node_boot/run.sh`` (or the
separately pinned ``game_platform`` lane) with harness knobs the harness
already reads:

* ``NODE_BOOT_WORLD=0``: the basic seam only;
* ``NODE_BOOT_SKIP_BASIC=1``: the World pass without the basic pass;
* ``NODE_BOOT_E2E_ONLY``: the journey groups in ``verdict.rs`` ``GROUPS``;
* ``OTERYN_E2E_ROW_BATCH=start:len``: the row journeys in ``world.rs``;
* ``NODE_BOOT_SPELLS=1``: the spell scenarios.

Group names and row indices come from the sources, so a new group or row is
planned without editing this file, and a misspelt knob fails the plan
instead of running zero scenarios.

Subcommands:
  plan       print the shard list (JSON) for a set
  export-env print KEY=VALUE lines for $GITHUB_ENV from one shard
  extract    turn one shard's raw log into a closed-vocabulary result.json
  redact     drop secret-bearing lines from a log before upload
  summarize  aggregate result.json files into a Markdown table
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[3]
NODE_BOOT_RUN = Path("tools/qualification/node_boot/run.sh")
GAME_PLATFORM_RUN = Path("tools/qualification/game_platform/run.sh")
VERDICT = Path("apps/game-server/src/qualification/client_phases/verdict.rs")
CLIENT_PHASES = Path("apps/game-server/src/qualification/client_phases")

SETS = ("reference", "pr", "full", "custom")
# Rows the default batch (`e2e_row_batch.rs`) runs on one machine today.
PR_ROW_BATCHES = ((0, 5), (5, 5), (10, 5))
# The last batch is open-ended so rows added after planning still run.
OPEN_END = 1_000_000
MAX_SHARDS = 64
SHARD_NAME = re.compile(r"^[a-z0-9][a-z0-9-]{0,62}$")
SHA = re.compile(r"^[0-9a-f]{40}$")
ENTRIES = ("node_boot", "game_platform")
# Knobs a shard may set, with the values each accepts. Every key must be read
# by the harness (checked by test_matrix.py).
ROW_BATCH = re.compile(r"^[0-9]{1,7}:[0-9]{1,7}$")
SENSITIVE = re.compile(r"password|postgres://|mysql://|secret|token|(?<!hot)key|grant", re.IGNORECASE)
SCENARIO = re.compile(
    r"SEAM_EVIDENCE e2e_all scenario=([A-Za-z0-9_.-]{1,64}) status=([A-Z0-9_-]{1,32}) "
    r"seconds=([0-9]{1,7}(?:\.[0-9]{1,6})?)(?:\s|$)"
)
NODE_BOOT_RESULT = re.compile(r"(?:^|\s)NODE_BOOT_RESULT=([A-Z_]{1,40}(?: mode=unprivileged)?)(?:\s|$)")
GAME_PLATFORM_RESULT = re.compile(r"(?:^|\s)GAME_PLATFORM_RESULT=(PASS|FAIL|BLOCKED)(?:\s|$)")
INCOMPLETE = re.compile(r"SEAM_EVIDENCE E2EF1_INCOMPLETE(?:\s|$)")


class PlanError(ValueError):
    pass


def read(root: Path, relative: Path) -> str:
    return (root / relative).read_text(encoding="utf-8")


def journey_groups(root: Path = ROOT) -> list[str]:
    source = read(root, VERDICT)
    block = re.search(r"const GROUPS: &\[\(&str, &\[&str\]\)\] = &\[(.*?)\n\];", source, re.DOTALL)
    if block is None:
        raise PlanError("GROUPS registry not found in verdict.rs")
    names = re.findall(r'^\s*\("([a-z0-9]+)",', block[1], re.MULTILINE)
    if not names or "rows" not in names or len(set(names)) != len(names):
        raise PlanError("GROUPS registry is empty, has duplicates or lacks rows")
    return names


def row_indices(root: Path = ROOT) -> list[int]:
    indices: list[int] = []
    for path in sorted((root / CLIENT_PHASES).glob("*.rs")):
        indices += [int(n) for n in re.findall(r"\be2e_row_selected\((\d+)\)", path.read_text(encoding="utf-8"))]
    if not indices:
        raise PlanError("no e2e_row_selected(N) call sites found")
    if len(set(indices)) != len(indices):
        raise PlanError("a row index is selected by two call sites")
    return sorted(indices)


def pinned_sha(root: Path, relative: Path, variable: str) -> str:
    match = re.search(rf"^readonly {variable}=([0-9a-f]{{40}})$", read(root, relative), re.MULTILINE)
    if match is None:
        raise PlanError(f"{variable} pin not found in {relative}")
    return match[1]


def group_partition(groups: list[str]) -> list[list[str]]:
    """Core-like groups | f4..f6 | f7 and later, in registry order.

    These are the combinations the single-machine run already executes
    together; a group gets its own shard only through the custom set.
    """
    journey = [g for g in groups if g not in {"rows", "c19a"}]  # F6 already includes C19a.
    numbered = {g: int(m[1]) for g in journey if (m := re.fullmatch(r"f(\d+)[a-z]*", g))}
    parts = [
        [g for g in journey if g not in numbered],
        [g for g in journey if g in numbered and numbered[g] < 7],
        [g for g in journey if g in numbered and numbered[g] >= 7],
    ]
    return [part for part in parts if part]


def row_batches(indices: list[int], start: int, end: int | None, size: int) -> list[tuple[int, int]]:
    """Contiguous batches over [start, end); with end None the last is open-ended."""
    if size < 1:
        raise PlanError("batch size must be at least 1")
    stop = (max(indices) + 1) if end is None else end
    batches = []
    for first in range(start, stop, size):
        length = min(size, stop - first)
        if any(first <= i < first + length for i in indices):
            batches.append((first, length))
    if not batches:
        raise PlanError("row range selects no row journey")
    if end is None:
        first, _ = batches[-1]
        batches[-1] = (first, OPEN_END)
    return batches


def shard(name: str, entry: str, platform_ref: str, world: bool, env: dict[str, str]) -> dict:
    return {"name": name, "entry": entry, "platform_ref": platform_ref, "world": world, "env": env}


def plan(
    set_name: str,
    *,
    groups: str = "",
    rows: str = "",
    batch: str = "10",
    basic: bool = False,
    spells: bool = False,
    game_platform: bool = False,
    root: Path = ROOT,
) -> list[dict]:
    if set_name not in SETS:
        raise PlanError(f"unknown set {set_name!r}")
    if set_name != "custom" and (groups.strip() or rows.strip()):
        raise PlanError("groups and rows inputs apply only to the custom set")
    if not re.fullmatch(r"[0-9]{1,3}", batch or ""):
        raise PlanError("batch must be a positive integer")
    size = int(batch)
    registry = journey_groups(root)
    indices = row_indices(root)
    platform = pinned_sha(root, NODE_BOOT_RUN, "PLATFORM_SHA")
    portal = pinned_sha(root, GAME_PLATFORM_RUN, "GAME_PLATFORM_SHA")
    world = {"NODE_BOOT_SKIP_BASIC": "1"}

    def group_shard(names: list[str]) -> dict:
        return shard("g-" + "-".join(names), "node_boot", platform, True,
                     {**world, "NODE_BOOT_E2E_ONLY": ",".join(names)})

    def row_shard(first: int, length: int) -> dict:
        suffix = "open" if length >= OPEN_END else f"{first + length:03d}"
        return shard(f"rows-{first:03d}-{suffix}", "node_boot", platform, True,
                     {**world, "NODE_BOOT_E2E_ONLY": "rows", "OTERYN_E2E_ROW_BATCH": f"{first}:{length}"})

    basic_shard = shard("basic", "node_boot", platform, False, {"NODE_BOOT_WORLD": "0"})
    spells_shard = shard("spells", "node_boot", platform, False, {"NODE_BOOT_SPELLS": "1"})
    portal_shard = shard("game-platform", "game_platform", portal, False, {})

    if set_name == "reference":
        shards = [shard("reference", "node_boot", platform, True, {})]
    elif set_name == "pr":
        shards = [basic_shard, *map(group_shard, group_partition(registry)),
                  *(row_shard(first, length) for first, length in PR_ROW_BATCHES)]
    elif set_name == "full":
        shards = [basic_shard, *map(group_shard, group_partition(registry)),
                  *(row_shard(f, n) for f, n in row_batches(indices, 0, None, size)),
                  spells_shard, portal_shard]
    else:
        shards = [basic_shard] if basic else []
        known = set(registry) - {"rows"}
        seen: set[str] = set()
        for chunk in filter(None, (part.strip() for part in groups.split(";"))):
            names = [name.strip() for name in chunk.split(",") if name.strip()]
            unknown = [name for name in names if name not in known]
            if unknown:
                raise PlanError(f"unknown journey group(s) {unknown}; rows are selected with the rows input")
            if seen & set(names) or len(set(names)) != len(names):
                raise PlanError("a journey group appears in two shards")
            seen |= set(names)
            shards.append(group_shard(names))
        text = rows.strip()
        if text:
            if text == "all":
                ranges = row_batches(indices, 0, None, size)
            elif match := re.fullmatch(r"([0-9]{1,4}):([0-9]{1,4})", text):
                start, end = int(match[1]), int(match[2])
                if end <= start:
                    raise PlanError("rows input is start:end with end greater than start")
                ranges = row_batches(indices, start, end, size)
            else:
                raise PlanError("rows input is 'all' or start:end")
            shards += [row_shard(first, length) for first, length in ranges]
        if spells:
            shards.append(spells_shard)
        if game_platform:
            shards.append(portal_shard)
    for item in shards:
        validate_shard(item, registry)
    if not shards:
        raise PlanError("the set selects no shard")
    if len(shards) > MAX_SHARDS:
        raise PlanError(f"{len(shards)} shards exceed the limit of {MAX_SHARDS}")
    names = [item["name"] for item in shards]
    if len(set(names)) != len(names):
        raise PlanError("duplicate shard name")
    return shards


def validate_shard(item: dict, registry: list[str] | None = None) -> None:
    if set(item) != {"name", "entry", "platform_ref", "world", "env"}:
        raise PlanError("shard has unexpected fields")
    if not isinstance(item["name"], str) or not SHARD_NAME.fullmatch(item["name"]):
        raise PlanError("invalid shard name")
    if item["entry"] not in ENTRIES:
        raise PlanError("invalid shard entry")
    if not isinstance(item["platform_ref"], str) or not SHA.fullmatch(item["platform_ref"]):
        raise PlanError("invalid Platform ref")
    if not isinstance(item["world"], bool) or not isinstance(item["env"], dict):
        raise PlanError("invalid shard fields")
    if item["entry"] == "game_platform" and item["env"]:
        raise PlanError("the game_platform lane sets its own knobs")
    groups = set(registry if registry is not None else journey_groups())
    for key, value in item["env"].items():
        if not isinstance(value, str):
            raise PlanError(f"{key} must be a string")
        if key == "NODE_BOOT_WORLD":
            ok = value in ("0", "1")
        elif key in ("NODE_BOOT_SKIP_BASIC", "NODE_BOOT_SPELLS"):
            ok = value == "1"
        elif key == "NODE_BOOT_E2E_ONLY":
            names = value.split(",")
            ok = bool(value) and all(name in groups for name in names) and len(set(names)) == len(names)
        elif key == "OTERYN_E2E_ROW_BATCH":
            ok = bool(ROW_BATCH.fullmatch(value)) and int(value.split(":")[1]) > 0
        else:
            raise PlanError(f"knob {key} is not allowed")
        if not ok:
            raise PlanError(f"invalid value for {key}")


def export_env(item: dict) -> str:
    validate_shard(item)
    lines = [f"SHARD_NAME={item['name']}", f"SHARD_ENTRY={item['entry']}",
             f"SHARD_WORLD={'1' if item['world'] else '0'}"]
    lines += [f"{key}={value}" for key, value in sorted(item["env"].items())]
    return "\n".join(lines) + "\n"


def extract(log: Path, *, name: str, entry: str, exit_code: int, seconds: int) -> dict:
    if not SHARD_NAME.fullmatch(name) or entry not in ENTRIES:
        raise PlanError("invalid shard identity")
    text = log.read_text(encoding="utf-8", errors="replace") if log.is_file() else ""
    scenarios = [{"id": m[1], "status": m[2], "seconds": float(m[3])} for m in SCENARIO.finditer(text)]
    pattern = GAME_PLATFORM_RESULT if entry == "game_platform" else NODE_BOOT_RESULT
    results = pattern.findall(text)
    return {
        "shard": name,
        "entry": entry,
        "exit_code": exit_code,
        "seconds": seconds,
        "result": results[-1] if results else "NONE",
        "incomplete": bool(INCOMPLETE.search(text)),
        "scenarios": scenarios,
    }


def shard_passed(result: dict) -> bool:
    expected = "PASS" if result["entry"] == "game_platform" else "NODE_BOOT_PASS"
    return result["exit_code"] == 0 and result["result"] == expected


def redact(source: Path, destination: Path) -> None:
    with source.open(encoding="utf-8", errors="replace") as input_file:
        with destination.open("w", encoding="utf-8") as output_file:
            for line in input_file:
                if not SENSITIVE.search(line):
                    output_file.write(line)


def summarize(shards: list[dict], directory: Path, prefix: str, *, missing_fatal: bool = True) -> tuple[str, list[str]]:
    """Markdown table plus the problems that fail the summary.

    Fatal: a planned shard without a readable result, a World shard that ran
    no scenario, and any MISSING* scenario status unless `missing_fatal` is
    false (the PR heavy and Merge Queue gates, whose single-machine run never
    failed on MISSING rows). Scenario FAIL rows and E2EF1_INCOMPLETE are
    reported; each shard job already fails on its own NODE_BOOT_RESULT.
    """
    problems: list[str] = []
    rows = ["| shard | result | exit | minutes | scenarios | PASS | FAIL | MISSING | other |",
            "|---|---|---|---|---|---|---|---|---|"]
    details: list[str] = []
    owners: dict[str, list[str]] = {}
    for item in shards:
        validate_shard(item)
        path = directory / f"{prefix}{item['name']}" / "result.json"
        if len(shards) == 1 and not path.exists():
            # download-artifact extracts a lone matching artifact straight into its path.
            path = directory / "result.json"
        try:
            result = json.loads(path.read_text(encoding="utf-8"))
            if result.get("shard") != item["name"]:
                raise ValueError("shard mismatch")
        except (OSError, ValueError):
            problems.append(f"{item['name']}: no result evidence")
            rows.append(f"| {item['name']} | NO EVIDENCE | - | - | - | - | - | - | - |")
            continue
        counts = {"PASS": 0, "FAIL": 0, "MISSING": 0, "other": 0}
        for scenario in result["scenarios"]:
            status = scenario["status"]
            key = status if status in ("PASS", "FAIL") else "MISSING" if status.startswith("MISSING") else "other"
            counts[key] += 1
            owners.setdefault(scenario["id"], []).append(item["name"])
            if status != "PASS":
                details.append(f"- `{item['name']}` {scenario['id']} {status}")
        if counts["MISSING"] and missing_fatal:
            problems.append(f"{item['name']}: {counts['MISSING']} MISSING scenario(s)")
        if item["world"] and not result["scenarios"]:
            problems.append(f"{item['name']}: World shard ran no scenario")
        if result["incomplete"]:
            details.append(f"- `{item['name']}` E2EF1_INCOMPLETE")
        verdict = result["result"] + ("" if shard_passed(result) else " (failed)")
        rows.append(
            f"| {item['name']} | {verdict} | {result['exit_code']} | {result['seconds'] / 60:.1f} | "
            f"{len(result['scenarios'])} | {counts['PASS']} | {counts['FAIL']} | {counts['MISSING']} | {counts['other']} |"
        )
    overlaps = sorted(f"{sid} ({', '.join(names)})" for sid, names in owners.items() if len(names) > 1)
    out = ["## Node boot shards", "", *rows, ""]
    if details:
        out += ["### Scenarios not passing", "", *details, ""]
    if overlaps:
        out += ["### Scenarios run by more than one shard", "", *(f"- {entry}" for entry in overlaps), ""]
    if problems:
        out += ["### Summary failures", "", *(f"- {problem}" for problem in problems), ""]
    return "\n".join(out), problems


def truthy(value: str) -> bool:
    if value not in ("", "true", "false"):
        raise PlanError("boolean inputs are true or false")
    return value == "true"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("plan")
    p.add_argument("--set", default="reference")
    p.add_argument("--groups", default="")
    p.add_argument("--rows", default="")
    p.add_argument("--batch", default="10")
    p.add_argument("--basic", default="")
    p.add_argument("--spells", default="")
    p.add_argument("--game-platform", default="")
    e = sub.add_parser("export-env")
    e.add_argument("shard_json")
    x = sub.add_parser("extract")
    x.add_argument("log", type=Path)
    x.add_argument("destination", type=Path)
    x.add_argument("--shard", required=True)
    x.add_argument("--entry", required=True)
    x.add_argument("--exit-code", type=int, required=True)
    x.add_argument("--seconds", type=int, required=True)
    r = sub.add_parser("redact")
    r.add_argument("source", type=Path)
    r.add_argument("destination", type=Path)
    s = sub.add_parser("summarize")
    s.add_argument("plan", type=Path)
    s.add_argument("directory", type=Path)
    s.add_argument("--prefix", required=True)
    s.add_argument("--missing-informational", action="store_true",
                   help="report MISSING* scenarios without failing the summary")
    args = parser.parse_args(argv)
    try:
        if args.command == "plan":
            shards = plan(args.set, groups=args.groups, rows=args.rows, batch=args.batch,
                          basic=truthy(args.basic), spells=truthy(args.spells),
                          game_platform=truthy(args.game_platform))
            print(json.dumps(shards, separators=(",", ":")))
        elif args.command == "export-env":
            sys.stdout.write(export_env(json.loads(args.shard_json)))
        elif args.command == "extract":
            result = extract(args.log, name=args.shard, entry=args.entry,
                             exit_code=args.exit_code, seconds=args.seconds)
            args.destination.write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8")
            print(f"NODE_BOOT_SHARD shard={result['shard']} result={result['result']} "
                  f"exit={result['exit_code']} scenarios={len(result['scenarios'])}")
            return 0 if shard_passed(result) else 1
        elif args.command == "redact":
            redact(args.source, args.destination)
        else:
            markdown, problems = summarize(json.loads(args.plan.read_text(encoding="utf-8")),
                                           args.directory, args.prefix,
                                           missing_fatal=not args.missing_informational)
            print(markdown)
            return 1 if problems else 0
    except PlanError as error:
        print(f"NODE_BOOT_MATRIX=BLOCKED reason={error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
