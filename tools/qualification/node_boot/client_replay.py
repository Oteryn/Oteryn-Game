#!/usr/bin/env python3
"""Gate the cloud wrapper's selected client rows on their fresh Rust replays."""

import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
PHASES = ROOT / "apps/game-server/src/qualification/client_phases"
CLIENT_ROW = re.compile(
    r'e2e_row_selected\((\d+)\)\s*\{\s*verdict::indexed\("([A-Za-z0-9]+)\.client"'
)
SERVER_ROW = re.compile(
    r"^SEAM_EVIDENCE (e2e_all|e2e_capture) scenario=([A-Za-z0-9]+)\.client status=([A-Z-]+)\b"
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
    return [
        row for index, row in CLIENT_ROW.findall((PHASES / "world.rs").read_text())
        if int(index) in selected
    ]


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


def replay_cause(row, package, test, names, captures, server, log):
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
             "--lib", test, "--", "--nocapture"],
            cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
        )
    except OSError:
        return "replay-not-run"
    log.with_name(f"{log.stem}-{row}.replay.log").write_text(result.stdout)
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
    if any(verdict not in {("e2e_all", "PASS"), ("e2e_capture", "CAPTURED")} for verdict in server):
        return "server-failed"
    return None


def main(log, batch):
    rows = selected_rows(batch)
    captures = Path(os.environ["E2E_ALL_CAPTURE_DIR"])
    server_rows = {}
    for line in log.read_text(errors="replace").splitlines():
        if match := SERVER_ROW.match(line):
            kind, row, status = match.groups()
            server_rows.setdefault(row, []).append((kind, status))

    if not rows:
        return 0
    targets = replay_targets()
    failed = False
    for row in rows:
        module = f"e2e_{row.lower()}"
        matches = targets.get(module, [])
        package, test, names = matches[0] if len(matches) == 1 else (None, module, set())
        cause = replay_cause(row, package, test, names, captures, server_rows.get(row), log)
        verdict = f"FAIL cause={cause}" if cause else "PASS"
        print(f"SEAM_EVIDENCE e2e_all scenario={row}.client replay={test} status={verdict}")
        failed |= cause is not None
    return int(failed)


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1]), os.environ.get("OTERYN_E2E_ROW_BATCH")))
