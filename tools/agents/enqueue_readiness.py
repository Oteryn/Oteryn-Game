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
from datetime import datetime, timezone

CANONICAL_CHECK = "game-gate"  # .github/repository-policy.json required status
# Logins whose authorization record is accepted: the owner (code_owner in .github/repository-policy.json) and
# the control plane, which records the standing D247 grant under the owner's account. Pinned here, never taken
# from the snapshot: a name the record merely states (`granted_by`) proves nothing.
TRUSTED_GRANT_AUTHORS = frozenset({"blakinio"})
# Platform-reported logins whose exact-head review qualifies (D245): Codex, and the owner's account, under which
# the control plane records the Claude fallback review. Any other reviewer's record is ignored, never trusted.
TRUSTED_REVIEWERS = frozenset({"chatgpt-codex-connector[bot]", "blakinio"})
# The owner's account also authors ordinary comments, so its login alone never proves a Claude fallback review
# (AGENTS.md: independent review). Such a record qualifies only with this explicit readback provider and a
# non-empty `fallback_evidence` reference (the recorded Claude review) set by the control plane's readback.
FALLBACK_PROVIDER = "claude-fallback"
FALLBACK_LOGINS = frozenset({"blakinio"})
# Readback and platform clocks may differ slightly; an event this far past `observed_at` is still accepted.
CLOCK_SKEW_SECONDS = 120

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


def _pr(value) -> int:
    """A PR number: a real positive JSON integer; without one the record names no PR."""
    if isinstance(value, bool) or not isinstance(value, int) or value < 1:
        raise ValueError(f"expected a positive PR number, got {value!r}")
    return value


def _list(value, field: str) -> list:
    """A gate collection must be a real JSON array; an object or string iterates as empty or per character."""
    if not isinstance(value, list):
        raise ValueError(f"{field} must be a JSON array, got {value!r}")
    return value


def evaluate(snapshot, now: datetime | None = None) -> dict:
    """Return {"verdict", "blockers", "ready_at"}; any malformed input fails closed.

    `now` is the evaluator's clock (injectable for tests); `observed_at` may not lie beyond it by more than the skew.
    """
    try:
        return _evaluate(snapshot, now or datetime.now(timezone.utc))
    except (AttributeError, KeyError, TypeError, ValueError) as exc:
        blocker = _blocker("INPUT_INCOMPLETE", "control_plane", "complete live readback", repr(exc))
        return {"verdict": "BLOCKED", "blockers": [blocker], "ready_at": None}


def _evaluate(s: dict, now: datetime) -> dict:
    cand = s["candidate"]
    head = _sha(cand["head_sha"])
    pr = _pr(cand["pr"])
    if _flag(cand["merged"]) or _flag(cand["queued"]):
        return {"verdict": "NOOP", "blockers": [], "ready_at": None}

    found: dict[str, dict] = {}
    stamps: list[tuple[datetime, str]] = []
    # The time the readback was taken: no qualifying event can lie after it, so ready_at is never in the future.
    observed = _parse_at(s["observed_at"])
    if (observed - now).total_seconds() > CLOCK_SKEW_SECONDS:
        raise ValueError(f"observed_at {s['observed_at']!r} is later than the evaluator clock")

    def stamp(item, key: str = "at") -> None:
        at = item[key]
        parsed = _parse_at(at)
        if (parsed - observed).total_seconds() > CLOCK_SKEW_SECONDS:
            raise ValueError(f"event time {at!r} is later than observed_at {s['observed_at']!r}")
        stamps.append((parsed, at))

    state, base_ref = cand["state"], cand["base_ref"]
    if not isinstance(state, str) or not isinstance(base_ref, str):
        raise ValueError("candidate state and base_ref must be strings")
    if state != "open" or _flag(cand["draft"]) or base_ref != "main":
        found["CANDIDATE_STATE"] = _blocker(
            "CANDIDATE_NOT_OPEN_READY", "writer", f"PR {pr} open, ready for review, base main")
    else:
        # When the PR last became open, ready for review and based on main (opened, reopened, marked ready or
        # retargeted): that transition can be the event that unblocks it, so its time is required.
        stamp(cand, "state_at")

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
        def qualifies(r) -> bool:
            login = r["reviewer"].lower()
            if login in FALLBACK_LOGINS and r.get("provider") == FALLBACK_PROVIDER and _text(r.get("fallback_evidence")):
                return True
            return login in TRUSTED_REVIEWERS - FALLBACK_LOGINS

        exact = [r for r in reviews if qualifies(r) and _sha(r["head_sha"]) == head]
        if not exact:
            found["REVIEW"] = _blocker("REVIEW_NOT_ON_EXACT_HEAD", "control_plane", f"review for head {head[:12]} requested")
        else:
            # Re-reviews: only the newest qualifying exact-head review is authoritative, as for check reruns.
            ats = [_parse_at(r.get("at")) for r in exact]
            latest = [r for r, a in zip(exact, ats) if a == max(ats)]
            if len({(r["state"], r["verdict"]) for r in latest}) > 1:
                raise ValueError("conflicting exact-head reviews at the same latest time")
            r = latest[0]
            if r["state"] != "completed":
                found["REVIEW"] = _blocker("REVIEW_PENDING", "reviewer", f"review for head {head[:12]} completed")
            elif r["verdict"] != "clean":
                found["REVIEW"] = _blocker("REVIEW_BLOCKING_FINDINGS", "writer", "fix pushed as a new head, then new exact-head review")
            else:
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
        dep_pr = _pr(dep["pr"])
        dep_base = dep["base_ref"]  # platform-reported base the dependency PR merged into
        if not isinstance(dep_base, str):
            raise ValueError("dependencies[].base_ref must be a string")
        if not _flag(dep["merged"]) or dep_base != "main":
            found["DEPENDENCIES"] = _blocker("DEPENDENCY_NOT_MERGED", "control_plane", f"PR {dep_pr} merged to main")
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

    # The snapshot carries the whole authorization history; the newest record by parsed `at` governs, so an older
    # grant can never mask a later narrowing or revocation.
    history = _list(s["authorization"], "authorization")
    if not history:
        raise ValueError("authorization history is empty")
    records = []
    for auth in history:
        scope = auth["scope"]
        if not isinstance(scope, list) or not all(isinstance(x, str) for x in scope):
            raise ValueError("authorization.scope must be a list of strings")
        author = auth["author"]  # platform-reported login of the record, set by the readback, not by the grantee
        if not isinstance(author, str):
            raise ValueError("authorization.author must be a string login")
        records.append((_parse_at(auth["at"]), auth, _sha(auth["head_sha"]), _flag(auth["revoked"])))
    # Only records by a trusted login count; an untrusted record can neither grant nor revoke.
    trusted = [r for r in records if r[1]["author"].lower() in TRUSTED_GRANT_AUTHORS]
    newest = [r for r in trusted if r[0] == max(x[0] for x in trusted)] if trusted else []
    if len({(r[2], r[3], tuple(r[1]["scope"])) for r in newest}) > 1:
        raise ValueError("conflicting authorization records at the same newest time")
    if (not newest or newest[0][2] != head or "enqueue" not in newest[0][1]["scope"] or newest[0][3]
            or not _text(newest[0][1].get("granted_by"))):
        found["AUTHORIZATION"] = _blocker(
            "AUTHORIZATION_MISSING", "owner", f"enqueue authorization recorded for head {head[:12]}")
    else:
        stamp(newest[0][1])

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
