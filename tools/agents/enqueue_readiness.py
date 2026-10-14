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
import re
import sys
from datetime import datetime

CANONICAL_CHECK = "game-gate"  # .github/repository-policy.json required status
# Logins whose authorization record is accepted: the owner (code_owner in .github/repository-policy.json) and
# the control plane, which records the standing D247 grant under the owner's account. Pinned here, never taken
# from the snapshot: a name the record merely states (`granted_by`) proves nothing.
TRUSTED_GRANT_AUTHORS = frozenset({"blakinio"})
# Platform-reported logins whose exact-head review qualifies (D245): Codex, and the owner's account, under which
# the control plane records the Claude fallback review. Any other reviewer's record is ignored, never trusted.
TRUSTED_REVIEWERS = frozenset({"chatgpt-codex-connector[bot]", "blakinio"})

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


def _flag(value) -> bool:
    """Gate fields must be real JSON booleans; a string such as "false" is truthy and fails open."""
    if not isinstance(value, bool):
        raise ValueError(f"expected a JSON boolean, got {value!r}")
    return value


def _parse_at(value) -> datetime:
    """Parse an ISO-8601 timestamp with an explicit offset; naive or malformed values are invalid."""
    if not isinstance(value, str) or not value:
        raise ValueError(f"expected an ISO-8601 timestamp, got {value!r}")
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        raise ValueError(f"timestamp must carry a UTC offset, got {value!r}")
    return parsed


def _sha(value) -> str:
    """A git object id: exactly 40 hex digits (case-insensitive), normalised to lowercase."""
    if not isinstance(value, str) or not re.fullmatch(r"[0-9a-fA-F]{40}", value):
        raise ValueError(f"expected a 40-character hex SHA, got {value!r}")
    return value.lower()


def _count(value) -> int:
    """A count must be a real non-negative JSON integer (not bool, float or string)."""
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise ValueError(f"expected a non-negative JSON integer, got {value!r}")
    return value


def _list(value, field: str) -> list:
    """A gate collection must be a real JSON array; an object or string iterates as empty or per character."""
    if not isinstance(value, list):
        raise ValueError(f"{field} must be a JSON array, got {value!r}")
    return value


def evaluate(snapshot) -> dict:
    """Return {"verdict", "blockers", "ready_at"}; any malformed input fails closed."""
    try:
        return _evaluate(snapshot)
    except (AttributeError, KeyError, TypeError, ValueError) as exc:
        blocker = _blocker("INPUT_INCOMPLETE", "control_plane", "complete live readback", repr(exc))
        return {"verdict": "BLOCKED", "blockers": [blocker], "ready_at": None}


def _evaluate(s: dict) -> dict:
    cand = s["candidate"]
    head = _sha(cand["head_sha"])
    pr = cand["pr"]
    if _flag(cand["merged"]) or _flag(cand["queued"]):
        return {"verdict": "NOOP", "blockers": [], "ready_at": None}

    found: dict[str, dict] = {}
    stamps: list[tuple[datetime, str]] = []

    def stamp(item, key: str = "at") -> None:
        at = item[key]
        stamps.append((_parse_at(at), at))

    state, base_ref = cand["state"], cand["base_ref"]
    if not isinstance(state, str) or not isinstance(base_ref, str):
        raise ValueError("candidate state and base_ref must be strings")
    if state != "open" or _flag(cand["draft"]) or base_ref != "main":
        found["CANDIDATE_STATE"] = _blocker(
            "CANDIDATE_NOT_OPEN_READY", "writer", f"PR {pr} open, ready for review, base main")

    runs = _list(s["checks"]["runs"], "checks.runs")
    required = _list(s["checks"]["required"], "checks.required")
    # Check contexts are strings: a fabricated non-string name could otherwise match itself.
    if not all(_text(name) for name in required) or not all(_text(r["name"]) for r in runs):
        raise ValueError("check names must be non-empty strings")
    if CANONICAL_CHECK not in required:
        raise ValueError(f"checks.required must include {CANONICAL_CHECK}")
    for name in required:
        hit = [r for r in runs if r["name"] == name and _sha(r["head_sha"]) == head]
        if len(hit) > 1:
            # Reruns: only the newest run is authoritative; an unordered or tied history is ambiguous.
            ats = [_parse_at(r.get("at")) for r in hit]
            if ats.count(max(ats)) != 1:
                raise ValueError(f"ambiguous duplicate runs for required check {name}")
            hit = [hit[ats.index(max(ats))]]
        if not hit or any(r["status"] != "completed" for r in hit):
            found["CHECKS"] = _blocker("CHECKS_PENDING", "ci", f"check_suite.completed {name} @ {head[:12]}")
            break
        if hit[0]["conclusion"] != "success":
            found["CHECKS"] = _blocker(
                "CHECKS_FAILED", "writer", f"new head with {name} success", f"{name} did not succeed @ {head[:12]}")
            break
        stamp(hit[0])

    review = s["review"]
    if _flag(review["required"]):
        reviews = _list(review["reviews"], "review.reviews")
        if not all(_text(r["reviewer"]) for r in reviews):
            raise ValueError("review.reviews[].reviewer must be the platform-reported login")
        exact = [r for r in reviews
                 if r["reviewer"].lower() in TRUSTED_REVIEWERS and _sha(r["head_sha"]) == head]
        if not exact:
            found["REVIEW"] = _blocker("REVIEW_NOT_ON_EXACT_HEAD", "control_plane", f"review for head {head[:12]} requested")
        elif any(r["state"] != "completed" for r in exact):
            found["REVIEW"] = _blocker("REVIEW_PENDING", "reviewer", f"review for head {head[:12]} completed")
        elif any(r["verdict"] != "clean" for r in exact):
            found["REVIEW"] = _blocker("REVIEW_BLOCKING_FINDINGS", "writer", "fix pushed as a new head, then new exact-head review")
        else:
            for r in exact:
                stamp(r)

    threads = s["threads"]
    total, unresolved = _count(threads["total"]), _count(threads["unresolved"])
    if unresolved > total:
        raise ValueError("threads.unresolved exceeds threads.total")
    if unresolved != 0:
        found["REVIEW_THREADS"] = _blocker(
            "REVIEW_THREADS_UNRESOLVED", "writer", "every review thread resolved", f"{unresolved} open")
    elif total > 0:
        # A PR that had threads was unblocked when the last one was resolved; that time is required.
        stamp(threads, "resolved_at")

    for item in _list(s["evidence"]["required"], "evidence.required"):
        if not _text(item["name"]):
            raise ValueError("evidence.required[].name must be a non-empty string")
        if not _flag(item["present"]) or _sha(item["head_sha"]) != head:
            found["EVIDENCE"] = _blocker("EVIDENCE_MISSING", "writer", f"evidence {item['name']} bound to head {head[:12]}")
            break
        stamp(item)

    for dep in _list(s["dependencies"], "dependencies"):
        if not _flag(dep["merged"]):
            found["DEPENDENCIES"] = _blocker("DEPENDENCY_NOT_MERGED", "control_plane", f"PR {dep['pr']} merged to main")
            break
        stamp(dep)

    for failure in _list(s["queue"]["prior_failures"], "queue.prior_failures"):
        if _sha(failure["head_sha"]) != head:
            continue
        if _flag(failure["cause_addressed"]):
            # Required: when the diagnosis showing the cause is outside the candidate was recorded.
            stamp(failure, "addressed_at")
        else:
            found["PRIOR_MQ_FAILURE"] = _blocker(
                "PRIOR_MQ_FAILURE_UNRESOLVED", "writer",
                "new head, or a recorded diagnosis showing the cause is outside the candidate",
                f"removed from queue at {failure.get('at', 'unknown time')}: {failure.get('cause', 'undiagnosed')}")
            break

    auth = s["authorization"]
    scope = auth["scope"]
    if not isinstance(scope, list) or not all(isinstance(x, str) for x in scope):
        raise ValueError("authorization.scope must be a list of strings")
    author = auth["author"]  # platform-reported login of the comment/record, set by the readback, not by the grantee
    if not isinstance(author, str):
        raise ValueError("authorization.author must be a string login")
    if (author.lower() not in TRUSTED_GRANT_AUTHORS or _sha(auth["head_sha"]) != head or "enqueue" not in auth["scope"] or _flag(auth["revoked"])
            or not _text(auth.get("granted_by"))):
        found["AUTHORIZATION"] = _blocker(
            "AUTHORIZATION_MISSING", "owner", f"enqueue authorization recorded for head {head[:12]}")
    else:
        stamp(auth)

    blockers = [found[key] for key in ORDER if key in found]
    if blockers:
        return {"verdict": "BLOCKED", "blockers": blockers, "ready_at": None}
    return {"verdict": "ENQUEUE_ELIGIBLE", "blockers": [], "ready_at": max(stamps)[1] if stamps else None}


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
