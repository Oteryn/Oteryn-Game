#!/usr/bin/env python3
"""Gate node boot's selected client rows on their fresh Rust replays."""

import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
PHASES = ROOT / "apps/game-server/src/qualification/client_phases"
SCENARIO = r"[A-Za-z0-9_.-]+\.(?:full|client)"
ROW_SELECTOR = re.compile(r"\be2e_row_selected\s*\(\s*(\d+)\s*\)")
ROW_REPORTER = re.compile(
    rf'\s*\{{\s*(?:verdict::indexed\s*\(\s*"({SCENARIO})"\s*,'
    r'|(\w+::\w+)\s*\(\s*clients\s*[,\)])'
)
# These older indexed call sites invoke their reporters directly.
LEGACY_REPORTERS = {
    "full_e2ef19::run_packet": "S04b.full",
    "full_e2ef18::run_packet": "C14.full",
    "full_e2ef20::run_packet": "E02.full",
    "full_e2ef20::greet_ten": "NPC-GREET-10.full",
    "client_row_c07::run_client_packet": "C07.client",
}
SERVER_ROW = re.compile(
    rf"^SEAM_EVIDENCE (e2e_all|e2e_capture) scenario=({SCENARIO}) "
    r"status=(PASS|FAIL|SKIPPED|MISSING-SERVER|MISSING-CLIENT|CAPTURED)(?:\s|$)"
)
PACKAGES = (
    "apps/client", "crates/session", "crates/client-ui", "crates/client-runtime",
    "crates/client-domain", "tools/dev-client",
)


def selected_rows(batch):
    if batch is None:
        source = (PHASES / "e2e_row_batch.rs").read_text()
        default = re.search(r"^const E2E_ROW_BATCH_DEFAULT: \(usize, usize\) = \((\d+), (\d+)\);$",
                            source, re.MULTILINE)
        if default is None:
            raise ValueError("missing E2E_ROW_BATCH_DEFAULT")
        batch = ":".join(default.groups())
    if batch == "all":
        selected = range(2**64)
    elif re.fullmatch(r"\+?[0-9]+:\+?[0-9]+", batch):
        start, length = map(int, batch.split(":"))
        if max(start, length) >= 2**64:
            raise ValueError("invalid OTERYN_E2E_ROW_BATCH")
        selected = range(start, start + length)
    else:
        raise ValueError("invalid OTERYN_E2E_ROW_BATCH")
    source = (PHASES / "world.rs").read_text()
    rows = {}
    for selector in ROW_SELECTOR.finditer(source):
        index = int(selector[1])
        match = ROW_REPORTER.match(source, selector.end())
        if match is None:
            raise RuntimeError(f"unmapped row selector {index}")
        scenario, reporter = match.groups()
        if reporter is not None:
            scenario = LEGACY_REPORTERS.get(reporter)
            if scenario is None:
                raise RuntimeError(f"unmapped row selector {index}: {reporter}")
        if index in rows:
            raise RuntimeError(f"duplicate row selector {index}")
        rows[index] = scenario
    return {scenario for index, scenario in rows.items() if index in selected}, set(rows.values())


def read_verdicts(log):
    verdicts = {}
    for line in log.read_text(errors="replace").splitlines():
        if match := SERVER_ROW.match(line):
            kind, scenario, status = match.groups()
            if kind == "e2e_capture" and (not scenario.endswith(".client") or status != "CAPTURED"):
                continue
            if kind == "e2e_all" and status == "CAPTURED":
                continue
            verdicts.setdefault(scenario, []).append((kind, status))
    return verdicts


def batch_failure(cause, scenario=None):
    detail = f" scenario={scenario}" if scenario else ""
    print(f"NODE_BOOT_ROW_BATCH=FAIL cause={cause}{detail}")
    return 1


def validate_rows(selected, registered, verdicts):
    failed = 0
    for scenario in sorted(selected):
        count = len(verdicts.get(scenario, []))
        if count != 1:
            failed = batch_failure("missing-verdict" if count == 0 else "duplicate-verdict", scenario)
    # A rows-only run has no other journey groups supplying indexed verdicts.
    if os.environ.get("NODE_BOOT_E2E_ONLY", "").strip() == "rows":
        for scenario in sorted((verdicts.keys() & registered) - selected):
            failed = batch_failure("unexpected-verdict", scenario)
    return failed


def capture_names(row, source):
    return set(re.findall(rf'"({row}[-_.][\w.-]+\.(?:bin|txt|json))"', source))


def replay_targets():
    targets = {}
    for package in PACKAGES:
        for path in (ROOT / package / "src").rglob("e2e_*.rs"):
            module = path.stem
            source = path.read_text()
            if "E2E_ALL_CAPTURE_DIR" not in source:
                continue
            tests = re.findall(r"#\[test\]\s*fn (\w+)\(", source)
            if len(tests) != 1:
                tests = [test for test in tests if test.startswith(f"{module}_")]
            names = capture_names(module.removeprefix("e2e_"), source)
            targets.setdefault(module, []).extend(
                (f"oteryn-{Path(package).name}", test, names) for test in tests
            )
    # The mail replay lives alongside the existing panel tests in play.rs.
    mail = ROOT / "apps/client/src/features/0115-mail/tests/play.rs"
    targets["e2e_e09"] = [(
        "oteryn-client", "e2e_e09_real_mail_outcomes_replay_into_mail_panel",
        capture_names("e09", mail.read_text()),
    )]
    return targets


def replay_cause(row, package, test, names, captures, server, log, suffix="", capture_only=False):
    # Some replays construct filenames from a prefix. Those replays check the
    # individual reads themselves, but still need at least one row-owned file.
    files = {path.name for path in captures.glob(f"{row.lower()}[-_.]*")
             if path.is_file() and path.stat().st_size}
    if not files or not names <= files:
        return "capture-missing"
    if not package or not server:
        return "replay-not-run"

    try:
        result = subprocess.run(
            [os.environ.get("CARGO", "cargo"), "test", "--locked", "-p", package,
             "--lib", test, "--", "--nocapture",
             *(["--ignored"] if capture_only else [])],
            cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
        )
    except OSError:
        return "replay-not-run"
    log.with_name(f"{log.stem}-{row}{suffix}.replay.log").write_text(result.stdout)
    if result.returncode:
        if re.search(r"^test result: FAILED\.", result.stdout, re.MULTILINE) and (
            "kind: NotFound" in result.stdout or "No such file or directory (os error 2)" in result.stdout
        ):
            return "capture-missing"
        return "replay-failed"

    counts = re.findall(r"^test result: ok\. (\d+) passed; 0 failed; (\d+) ignored\b",
                        result.stdout, re.MULTILINE)
    if not counts or all(passed == "0" for passed, _ in counts):
        return "replay-not-run"
    passed_test = re.search(rf"^test (?:\w+::)*{test} \.\.\. ok$", result.stdout, re.MULTILINE)
    if counts != [("1", "0")] or not passed_test:
        return "replay-failed"
    if len(server) != 1 or server[0] not in {("e2e_all", "PASS"), ("e2e_capture", "CAPTURED")}:
        return "server-failed"
    return None


def main(log, batch, validate=False):
    try:
        selected, registered = selected_rows(batch)
    except ValueError:
        return batch_failure("malformed-batch")
    if not selected:
        return batch_failure("empty-batch")
    server_rows = read_verdicts(log)
    if validate:
        return validate_rows(selected, registered, server_rows)

    rows = sorted(scenario for scenario in selected if scenario.endswith(".client"))
    if not rows:
        return 0
    captures = Path(os.environ["E2E_ALL_CAPTURE_DIR"])
    targets = replay_targets()
    failed = False
    for scenario in rows:
        row = scenario.removesuffix(".client")
        module = f"e2e_{row.lower()}"
        matches = targets.get(module, [])
        package, test, names = matches[0] if len(matches) == 1 else (None, module, set())
        cause = replay_cause(row, package, test, names, captures, server_rows.get(scenario), log)
        verdict = f"FAIL cause={cause}" if cause else "PASS"
        print(f"SEAM_EVIDENCE e2e_all scenario={scenario} replay={test} status={verdict}")
        failed |= cause is not None
        if row == "P19" and cause is None:
            test = "cl6_p19_live_capture_reaches_native_nameplates"
            cause = replay_cause(
                row, "oteryn-client", test, names, captures,
                server_rows.get(scenario), log, suffix="-nameplate", capture_only=True,
            )
            verdict = f"FAIL cause={cause}" if cause else "PASS"
            print(f"SEAM_EVIDENCE e2e_all scenario={scenario} nameplate={test} status={verdict}")
            failed |= cause is not None
    return int(failed)


if __name__ == "__main__":
    args = sys.argv[1:]
    validate = args[0] == "--validate"
    sys.exit(main(Path(args[-1]), os.environ.get("OTERYN_E2E_ROW_BATCH"), validate))
