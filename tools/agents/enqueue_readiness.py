#!/usr/bin/env python3
"""Read-only enqueue-readiness evaluation of one Merge Queue candidate.

The control plane (docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md, "Qualification-completion
resume") runs this on a snapshot of live GitHub state whenever a qualification event completes. It
only classifies the snapshot. It never calls GitHub, enables auto-merge, enqueues or retries; the
existing control-plane enqueue path (D247) stays the only mutation.

Exit code: 0 ENQUEUE_ELIGIBLE, 3 BLOCKED, 4 NOOP (already queued or merged), 2 invalid input.
"""
from __future__ import annotations

import json
import sys

# Fixed evaluation order. Each blocker names its owner and the single event that clears it.
ORDER = (
    "CANDIDATE_STATE",
    "CHECKS",
    "REVIEW",
    "REVIEW_THREADS",
    "EVIDENCE",
    "DEPENDENCIES",
    "PRIOR_MQ_FAILURE",
    "AUTHORIZATION",
)


def _blocker(reason: str, owner: str, event: str, detail: str = "") -> dict:
    return {"reason": reason, "owner": owner, "unblocking_event": event, "detail": detail}


def _text(value) -> str:
    return value if isinstance(value, str) else ""


def evaluate(snapshot) -> dict:
    """Return {"verdict", "blockers", "ready_at"}; any malformed input fails closed."""
    try:
        return _evaluate(snapshot)
    except (AttributeError, KeyError, TypeError, ValueError) as exc:
        blocker = _blocker("INPUT_INCOMPLETE", "control_plane", "complete live readback", repr(exc))
        return {"verdict": "BLOCKED", "blockers": [blocker], "ready_at": None}


def _evaluate(s: dict) -> dict:
    cand = s["candidate"]
    head = _text(cand["head_sha"])
    if len(head) != 40:
        raise ValueError("candidate.head_sha must be a full 40-character SHA")
    pr = cand["pr"]
    if cand.get("merged") or cand.get("queued"):
        return {"verdict": "NOOP", "blockers": [], "ready_at": None}

    found: dict[str, dict] = {}
    stamps: list[str] = []

    def stamp(item) -> None:
        at = _text(item.get("at"))
        if at:
            stamps.append(at)

    if cand.get("state") != "open" or cand.get("draft") is not False or cand.get("base_ref") != "main":
        found["CANDIDATE_STATE"] = _blocker(
            "CANDIDATE_NOT_OPEN_READY", "writer", f"PR {pr} open, ready for review, base main")

    runs = s["checks"]["runs"]
    for name in s["checks"]["required"]:
        hit = [r for r in runs if r["name"] == name and r["head_sha"] == head]
        if not hit or any(r["status"] != "completed" for r in hit):
            found["CHECKS"] = _blocker("CHECKS_PENDING", "ci", f"check_suite.completed {name} @ {head[:12]}")
            break
        if not any(r["conclusion"] == "success" for r in hit):
            found["CHECKS"] = _blocker(
                "CHECKS_FAILED", "writer", f"new head with {name} success", f"{name} did not succeed @ {head[:12]}")
            break
        stamp(max(hit, key=lambda r: _text(r.get("at"))))

    review = s["review"]
    if review["required"]:
        exact = [r for r in review["reviews"] if r["head_sha"] == head]
        if not exact:
            found["REVIEW"] = _blocker("REVIEW_NOT_ON_EXACT_HEAD", "control_plane", f"review for head {head[:12]} requested")
        elif any(r["state"] != "completed" for r in exact):
            found["REVIEW"] = _blocker("REVIEW_PENDING", "reviewer", f"review for head {head[:12]} completed")
        elif any(r["verdict"] != "clean" for r in exact):
            found["REVIEW"] = _blocker("REVIEW_BLOCKING_FINDINGS", "writer", "fix pushed as a new head, then new exact-head review")
        else:
            for r in exact:
                stamp(r)

    if int(s["threads"]["unresolved"]) != 0:
        found["REVIEW_THREADS"] = _blocker(
            "REVIEW_THREADS_UNRESOLVED", "writer", "every review thread resolved", f"{s['threads']['unresolved']} open")

    for item in s["evidence"]["required"]:
        if not item["present"] or item["head_sha"] != head:
            found["EVIDENCE"] = _blocker("EVIDENCE_MISSING", "writer", f"evidence {item['name']} bound to head {head[:12]}")
            break
        stamp(item)

    for dep in s["dependencies"]:
        if not dep["merged"]:
            found["DEPENDENCIES"] = _blocker("DEPENDENCY_NOT_MERGED", "control_plane", f"PR {dep['pr']} merged to main")
            break
        stamp(dep)

    for failure in s["queue"]["prior_failures"]:
        if failure["head_sha"] == head and not failure["cause_addressed"]:
            found["PRIOR_MQ_FAILURE"] = _blocker(
                "PRIOR_MQ_FAILURE_UNRESOLVED", "writer",
                "new head, or a recorded diagnosis showing the cause is outside the candidate",
                f"removed from queue at {failure.get('at', 'unknown time')}: {failure.get('cause', 'undiagnosed')}")
            break

    auth = s["authorization"]
    if (auth["head_sha"] != head or "enqueue" not in auth["scope"] or auth["revoked"]
            or not _text(auth.get("granted_by"))):
        found["AUTHORIZATION"] = _blocker(
            "AUTHORIZATION_MISSING", "owner", f"enqueue authorization recorded for head {head[:12]}")
    else:
        stamp(auth)

    blockers = [found[key] for key in ORDER if key in found]
    if blockers:
        return {"verdict": "BLOCKED", "blockers": blockers, "ready_at": None}
    return {"verdict": "ENQUEUE_ELIGIBLE", "blockers": [], "ready_at": max(stamps) if stamps else None}


def main(argv: list[str]) -> int:
    source = open(argv[1], encoding="utf-8") if len(argv) > 1 else sys.stdin
    try:
        snapshot = json.load(source)
    except json.JSONDecodeError as exc:
        print(f"invalid JSON: {exc}", file=sys.stderr)
        return 2
    result = evaluate(snapshot)
    print(json.dumps(result, indent=2, sort_keys=True))
    return {"ENQUEUE_ELIGIBLE": 0, "BLOCKED": 3, "NOOP": 4}[result["verdict"]]


if __name__ == "__main__":
    sys.exit(main(sys.argv))
