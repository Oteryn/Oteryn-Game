"""Render the final local evidence packet from captured commands."""
import json
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parent
records = [json.loads(line) for line in (root / "commands.jsonl").read_text().splitlines()]
latest = {record["label"]: record for record in records}
required = ["check-final", "clippy-final", "fmt-final", "client-tests", "ui-tests", "protocol-tests", "session-tests", "synthetic-tests", "protocol-generated", "session-closure", "architecture", "pin-refresh", "pin-check", "diff-check", "registry-roundtrip", "final-numbers"]
assert all(label in latest and latest[label]["exit_code"] == 0 for label in required)
pg = [record for label, record in latest.items() if label.startswith("pg-") and label != "pg-list"]
assert len(pg) == 12 and all(record["exit_code"] == 0 for record in pg), "twelve executed PostgreSQL cases required"


def git(*args):
    return subprocess.check_output(["git", *args], text=True).strip()


out = ["# CL6 + CL7 package validation", "", f"Base/current main verified: `{git('rev-parse', 'origin/main')}`.", f"CL6 HEAD: `{git('rev-parse', 'HEAD')}`; incoming CL7: `{git('rev-parse', 'MERGE_HEAD')}`.", "", "## Final numbers", "", "| Slice | Capability | Command | Mechanic | Offer |", "|---|---:|---:|---:|---:|", "| CL6 mechanics status | 384 | 669 | 642 | 639 |", "| CL7 proficiency browse | 195 (existing) | 641 | 638 | none added |", "", "CL7 also enables existing creature-push capability 82 / command 98 and carried-hotkey capability 180 under existing owner 179 / command 348. No number clashes or renumbering. Capability 384 and existing command 384 are separate namespaces. The client advertises 100 sorted capabilities, within the server's selected capacity of 112.", "", "## Repairs and generated files", "", "- Resolved the diff3 conflict in `crates/session/src/session_tests_chat.rs` by retaining both sides' capability assertions: 82, 180, 195 and 384. No tests removed or weakened.", "- Corrected existing command-fragment filenames 383/384 through `registry_fragments.split`; their names were not the canonical names emitted by the round-trip generator. IDs, JSON bytes and assembled registry bytes are identical. The registry round-trip test was red before this repair and green afterward.", "- Restricted the new test-only mechanics-status snapshot accessor to test builds within the play module; all remaining new public production functions have callers.", "- Regenerated client registration fixtures and capability goldens with the repository scripts; regenerated and checked World pins with the compiler. Build scripts regenerate dispatch mounts, capability tables and client registrations from fragments.", "", "## Self-review", "", "Reviewed the complete package diff against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and the repository checklist. Searched all constructor, match and implementation sites under `apps/`, `crates/` and `tools/` for added structs, fields, variants and trait methods (local `symbol-sites.log`, `proficiency-sites.log`, `caller-sites.log`). Checked bounded reads, semantic deadlines, capability negotiation, current-session/scope fencing, reply preservation, production callers and real-dispatch coverage. No remaining finding. Existing durable owners remain authoritative; there are no migrations, new durable writes, or changed wire IDs.", "", "## Command results", "", "Wall durations include queued-wrapper waiting and compilation. Every Cargo command uses the absolute queued wrapper with the repository toolchain and locked dependencies; dependency downloads were disabled with `CARGO_NET_OFFLINE=true`. Historical failures are retained below; their repaired reruns pass.", "", "| Result | Wall duration | Command | Log |", "|---|---:|---|---|"]
for record in records:
    result = "PASS" if record["exit_code"] == 0 else "FAIL (repaired)"
    out.append(f"| {result} | {record['duration_seconds']:.3f}s | `{record['command']}` | `{record['log']}` |")
out.extend(["", "The first test-plan discovery attempt found duplicate short test names in different client modules. The runner was corrected to use fully qualified exact names; the actual Cargo test runs below execute the complete selected set.", "", "## Exact test filters and execution", ""])
for label in ["client-tests", "ui-tests", "protocol-tests", "session-tests", "synthetic-tests", "protocol-generated", "session-closure"]:
    record = latest[label]
    log = (root / record["log"]).read_text()
    summary, = re.findall(r"test result: ok\.[^\n]+", log)
    out.extend([f"### {label}: PASS", "", f"`{summary}`", "", "```sh", record["command"], "```", ""])
out.extend(["### PostgreSQL: twelve exact cases, PASS", "", "No unfiltered PostgreSQL suite was run. Each case uses `--exact --nocapture --test-threads=1` through the absolute wrapper. No absent-configuration early return counts as qualification.", "", "| Case | Test duration | Execution evidence |", "|---|---:|---|"])
for record in pg:
    log = (root / record["log"]).read_text()
    elapsed, = re.findall(r"test result: ok\. 1 passed;.*finished in ([\d.]+)s", log)
    assert float(elapsed) > 0 and "is not configured" not in log
    markers = [line for line in log.splitlines() if re.search(r"POSTGRES_CASE_COMPLETED|POSTGRES_EXECUTED|CL6_TIMER|CL6_PARTY|PG_MARKER", line)]
    name = record["label"][3:]
    if not markers:
        assert name == "creature_push_real_dispatch_moves_and_refuses_on_postgres"
        markers = ["Real-dispatch test completed with non-zero runtime; this existing test emits no marker."]
    evidence = "; ".join(dict.fromkeys(markers)).replace("|", "\\|")
    out.append(f"| `{name}` | {elapsed}s | {evidence} |")
out.extend(["", "Exact commands:", ""])
for record in pg:
    out.extend(["```sh", record["command"], "```", ""])
out.extend(["## Changed files", "", "Complete package relative to current main (including incoming merge files):", ""])
changes = [line.split("\t", 1) for line in git("diff", "--name-status", "origin/main").splitlines()]
for status, path in changes:
    out.append(f"- `{status}` `{path}`")
for path in git("ls-files", "--others", "--exclude-standard").splitlines():
    if not path.startswith(".cl6/"):
        out.append(f"- `A` `{path}`")
out.extend(["- `A` `.cl6/REPORT.md`", "- `A` `.cl6/.gitignore` (keeps local logs, helpers and compiler output out of accidental staging)", "", "The deleted `0383-cyclopedia-bestiary-tracker.json` and `0384-cyclopedia-bosstiary-tracker.json` files are byte-identical renames to `0383-bestiary-tracker.json` and `0384-bosstiary-tracker.json`.", "", "## Unfinished", "", "None for implementation or requested local qualification. The index intentionally remains unmerged for `crates/session/src/session_tests_chat.rs` until the owner stages the resolution. No index writes, commits, pushes, resets, checkouts, stashes, credential changes, Docker operations or dependency downloads were performed. Publication and full CI are outside this local task.", "", "ALL PASS"])
(root / "REPORT.md").write_text("\n".join(out) + "\n")
print(f"Wrote {root / 'REPORT.md'}; {len(records)} captured validation commands, {len(pg)} executed PostgreSQL cases")
