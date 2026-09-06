#!/usr/bin/env python3
"""Read-only consistency checks for this audit publication, not product qualification.

No network, installs, project execution, Git writes or merge authority. Counts below
check the declared report scope; they do not prove universal semantic correctness.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

SOURCE = "7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3"


def load(root: Path, name: str) -> dict:
    value = json.loads((root / name).read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"expected JSON object: {name}")
    return value


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def verify(root: Path) -> dict:
    manifest = load(root, "publication-manifest.json")
    for name, expected in manifest["sha256"].items():
        path = Path(name)
        require(not path.is_absolute() and ".." not in path.parts, "unsafe manifest path")
        digest = hashlib.sha256((root / path).read_bytes()).hexdigest()
        require(digest == expected, f"digest mismatch: {name}")
    text = (root / "README.md").read_text(encoding="utf-8")
    sections = [int(n) for n in re.findall(r"^## (\d+)\.", text, re.M)]
    require(sections == list(range(1, 15)), "report must contain ordered sections 1..14")
    assessment = load(root, "assessment.json")
    controls = load(root, "controls.json")
    trace = load(root, "prompt-traceability.json")
    scope = load(root, "review-scope.json")
    evidence = load(root, "closeout-evidence.json")
    require(assessment["source_commit"] == evidence["source_commit"] == scope["source_commit"] == controls["audited_commit"] == SOURCE, "source identity mismatch")
    require(assessment["programme_audit"] == controls["programme_audit"] == "FAIL", "negative programme finding must not be relabeled PASS")
    base_findings = {x["id"]: x for x in load(root, assessment["inherits"])["findings"]}
    findings = [dict(base_findings.get(x["id"], {}), **x) for x in assessment["findings"]]
    require({x["id"] for x in findings} == {f"F{x:02d}" for x in range(1, 21)} and len(findings) == 20, "missing or duplicate finding")
    for row in findings:
        require(row["phase"] in {"CURRENT_GATE", "NEXT_GATE", "FUTURE_CONSTRAINT", "FUTURE_ONLY"}, "invalid finding phase")
        require(bool(row["evidence"]) and bool(row["acceptance"]) and bool(row["source_paths"]), "finding lacks evidence or acceptance")
        require(row["repair_performed"] is False, "audit did not repair product")
    require(next(x for x in findings if x["id"] == "F20")["priority"] == "P1", "open bridge P1 must remain visible")
    ids = {x["id"] for x in controls["controls"]}
    require(ids == {f"C{x:02d}" for x in range(1, 33)} and len(controls["controls"]) == 32, "missing or duplicate control")
    require(all(x["remaining"] and x["evidence"] and x["audit_assessment"] for x in controls["controls"]), "control is silently unassessed")
    require([x["section"] for x in trace["prompt_sections"]] == list(range(35)), "prompt chapter missing")
    require({x["id"] for x in trace["user_requirements"]} == {f"U{x:02d}" for x in range(1, 14)}, "user requirement missing")
    for item in trace["prompt_sections"]:
        require(set(item["controls"]) <= ids and set(item["report_sections"]) <= set(sections), "trace points outside report")
    require(scope["identity_and_static_files"] == 803 and scope["historical_focused_paths"] == 97, "historical coverage rewritten")
    combined = set(load(root, "coverage-register.json")["focused_paths"]) | {x["path"] for x in scope["reads"]}
    require(scope["combined_unique_focused_paths"] == len(combined), "invalid focused count")
    require(scope["all_line_semantic_review"] is False and assessment["all_line_review_claim"] is False, "unsupported universal review claim")
    require(evidence["windows"]["observed"]["exited_after_minimize"] is True and evidence["windows"]["observed"]["observed_exit_code"] == 0, "native window finding changed")
    require(all(x["warnings"] == "" for x in evidence["coverage"]["isolated"]), "isolated coverage warning not resolved")
    q = evidence["rust_codeql"]
    require(q["fixture_locations"] == 54 and q["unresolved_locations"] == 1 and q["total_results"] == 55, "CodeQL triage overstated")
    require(bool(q["extractor_diagnostics"]), "extractor diagnostic lost")
    require(evidence["live_correction"]["status"] == "P1_OPEN_UNACTIVATED_REPAIR_PAUSED_BY_OWNER", "bridge closeout hold lost")
    return {"publication_integrity": "PASS", "report_sections": 14, "prompt_sections_addressed": 35, "controls_addressed": 32, "findings": 20, "programme_audit": "FAIL", "all_line_review_claim": False, "source": SOURCE}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, default=Path(__file__).resolve().parent)
    args = parser.parse_args()
    try:
        print(json.dumps(verify(args.directory), indent=2))
        return 0
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(f"Audit publication verification failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
