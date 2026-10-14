import copy
import sys
import unittest
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import enqueue_readiness  # noqa: E402

NOW = datetime(2026, 10, 14, 12, 0, 30, tzinfo=timezone.utc)


def evaluate(snapshot, now=None):
    return enqueue_readiness.evaluate(snapshot, now=now or NOW)


HEAD = "a" * 40
OTHER = "b" * 40


def ready() -> dict:
    return {
        "observed_at": "2026-10-14T12:00:00Z",
        "candidate": {"pr": 1, "head_sha": HEAD, "state": "open", "draft": False, "base_ref": "main",
                      "merged": False, "queued": False, "state_at": "2026-10-14T07:00:00Z"},
        "checks": {
            "required": ["game-gate"],
            "runs": [{"name": "game-gate", "head_sha": HEAD, "status": "completed",
                      "conclusion": "success", "at": "2026-10-14T10:00:00Z"}],
        },
        "review": {"required": True, "reviews": [
            {"head_sha": HEAD, "state": "completed", "verdict": "clean", "reviewer": "chatgpt-codex-connector[bot]",
             "at": "2026-10-14T10:05:00Z"}]},
        "threads": {"total": 0, "unresolved": 0},
        "evidence": {"required": [{"name": "task-record", "head_sha": HEAD, "present": True,
                                   "at": "2026-10-14T09:00:00Z"}]},
        "dependencies": [{"pr": 2, "merged": True, "base_ref": "main", "at": "2026-10-14T08:00:00Z"}],
        "queue": {"prior_failures": []},
        "authorization": {"head_sha": HEAD, "scope": ["enqueue"], "revoked": False,
                          "granted_by": "owner", "author": "blakinio", "at": "2026-10-14T09:30:00Z"},
    }


def reasons(snapshot) -> list[str]:
    return [b["reason"] for b in evaluate(snapshot)["blockers"]]


class EnqueueReadinessTest(unittest.TestCase):
    def test_all_requirements_met_is_eligible_with_ready_at(self):
        result = evaluate(ready())
        self.assertEqual(result["verdict"], "ENQUEUE_ELIGIBLE")
        self.assertEqual(result["ready_at"], "2026-10-14T10:05:00Z")

    def test_every_blocker_names_owner_and_unblocking_event(self):
        snap = ready()
        snap["checks"]["runs"][0]["status"] = "in_progress"
        blocker = evaluate(snap)["blockers"][0]
        self.assertEqual(blocker["reason"], "CHECKS_PENDING")
        self.assertTrue(blocker["owner"] and blocker["unblocking_event"])

    def test_check_on_other_head_does_not_count(self):
        snap = ready()
        snap["checks"]["runs"][0]["head_sha"] = OTHER
        self.assertEqual(reasons(snap), ["CHECKS_PENDING"])

    def test_failed_check_blocks(self):
        snap = ready()
        snap["checks"]["runs"][0]["conclusion"] = "failure"
        self.assertEqual(reasons(snap), ["CHECKS_FAILED"])

    def test_skipped_required_check_blocks(self):
        snap = ready()
        snap["checks"]["runs"][0]["conclusion"] = "skipped"
        self.assertEqual(reasons(snap), ["CHECKS_FAILED"])

    def test_review_on_other_head_blocks(self):
        snap = ready()
        snap["review"]["reviews"][0]["head_sha"] = OTHER
        self.assertEqual(reasons(snap), ["REVIEW_NOT_ON_EXACT_HEAD"])

    def test_pending_and_blocking_review(self):
        snap = ready()
        snap["review"]["reviews"][0]["state"] = "running"
        self.assertEqual(reasons(snap), ["REVIEW_PENDING"])
        snap = ready()
        snap["review"]["reviews"][0]["verdict"] = "blocking"
        self.assertEqual(reasons(snap), ["REVIEW_BLOCKING_FINDINGS"])

    def test_review_from_untrusted_reviewer_does_not_qualify(self):
        snap = ready()
        snap["review"]["reviews"][0]["reviewer"] = "someone-else"
        self.assertEqual(reasons(snap), ["REVIEW_NOT_ON_EXACT_HEAD"])
        for bad in ("", None, 7):
            snap = ready()
            snap["review"]["reviews"][0]["reviewer"] = bad
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap = ready()
        del snap["review"]["reviews"][0]["reviewer"]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_owner_login_review_needs_fallback_provenance(self):
        snap = ready()
        snap["review"]["reviews"][0]["reviewer"] = "blakinio"
        self.assertEqual(reasons(snap), ["REVIEW_NOT_ON_EXACT_HEAD"])
        snap["review"]["reviews"][0]["provider"] = "claude-fallback"
        self.assertEqual(reasons(snap), ["REVIEW_NOT_ON_EXACT_HEAD"])
        snap["review"]["reviews"][0]["fallback_evidence"] = "https://claude.ai/code/session_x"
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")
        snap["review"]["reviews"][0]["provider"] = "codex"
        self.assertEqual(reasons(snap), ["REVIEW_NOT_ON_EXACT_HEAD"])

    def test_observed_at_beyond_evaluator_clock_is_rejected(self):
        snap = ready()
        snap["observed_at"] = "2026-10-14T12:02:00Z"  # 90 s ahead of NOW: within skew
        snap["checks"]["runs"][0]["at"] = "2026-10-14T12:02:00Z"
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")
        snap["observed_at"] = snap["checks"]["runs"][0]["at"] = "2026-10-14T12:03:00Z"  # 150 s ahead
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_latest_review_is_authoritative(self):
        snap = ready()
        base = snap["review"]["reviews"][0]
        old = dict(base, state="running", at="2026-10-14T09:00:00Z")
        snap["review"]["reviews"] = [base, old]
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")
        newer = dict(base, state="running", at="2026-10-14T11:00:00Z")
        snap["review"]["reviews"] = [base, newer]
        self.assertEqual(reasons(snap), ["REVIEW_PENDING"])
        older_block = dict(base, verdict="blocking", at="2026-10-14T09:00:00Z")
        snap["review"]["reviews"] = [older_block, base]
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")
        tie = dict(base, state="running")
        snap["review"]["reviews"] = [base, tie]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap["review"]["reviews"] = [base, dict(base)]
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")
        del base["at"]
        snap["review"]["reviews"] = [base]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_event_after_observed_at_is_rejected(self):
        snap = ready()
        snap["checks"]["runs"][0]["at"] = "2026-10-14T12:05:00Z"
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap["checks"]["runs"][0]["at"] = "2026-10-14T12:01:00Z"  # within the documented skew
        result = evaluate(snap)
        self.assertEqual((result["verdict"], result["ready_at"]), ("ENQUEUE_ELIGIBLE", "2026-10-14T12:01:00Z"))

    def test_observed_at_is_required_and_valid(self):
        for bad in (None, "", "2026-10-14T12:00:00", "nope", 5):
            snap = ready()
            snap["observed_at"] = bad
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap = ready()
        del snap["observed_at"]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_evidence_must_name_its_artifact(self):
        for bad in ("", None, 3):
            snap = ready()
            snap["evidence"]["required"][0]["name"] = bad
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap = ready()
        del snap["evidence"]["required"][0]["name"]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_review_not_required_is_not_demanded(self):
        snap = ready()
        snap["review"] = {"required": False, "reviews": []}
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")

    def test_unresolved_threads_block(self):
        snap = ready()
        snap["threads"].update(total=2, unresolved=2)
        self.assertEqual(reasons(snap), ["REVIEW_THREADS_UNRESOLVED"])

    def test_evidence_must_be_present_and_bound_to_head(self):
        snap = ready()
        snap["evidence"]["required"][0]["head_sha"] = OTHER
        self.assertEqual(reasons(snap), ["EVIDENCE_MISSING"])

    def test_unmerged_dependency_blocks(self):
        snap = ready()
        snap["dependencies"][0]["merged"] = False
        self.assertEqual(reasons(snap), ["DEPENDENCY_NOT_MERGED"])

    def test_dependency_merged_elsewhere_blocks(self):
        snap = ready()
        snap["dependencies"][0]["base_ref"] = "release"
        self.assertEqual(reasons(snap), ["DEPENDENCY_NOT_MERGED"])
        for bad in (None, 1):
            snap = ready()
            if bad is None:
                del snap["dependencies"][0]["base_ref"]
            else:
                snap["dependencies"][0]["base_ref"] = bad
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"], bad)

    def test_pr_numbers_must_be_positive_integers(self):
        for where in ("candidate", "dependency"):
            for bad in (None, 0, -1, True, "2", 2.0):
                snap = ready()
                target = snap["candidate"] if where == "candidate" else snap["dependencies"][0]
                if bad is None:
                    del target["pr"]
                else:
                    target["pr"] = bad
                self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"], (where, bad))

    def test_candidate_becoming_ready_can_be_the_latest_event(self):
        snap = ready()
        snap["candidate"]["state_at"] = "2026-10-14T11:00:00Z"
        self.assertEqual(evaluate(snap)["ready_at"], "2026-10-14T11:00:00Z")

    def test_prior_mq_failure_blocks_same_head_until_addressed(self):
        snap = ready()
        snap["queue"]["prior_failures"] = [
            {"head_sha": HEAD, "cause": "lint", "cause_addressed": False, "at": "2026-10-14T10:20:00Z"}]
        self.assertEqual(reasons(snap), ["PRIOR_MQ_FAILURE_UNRESOLVED"])
        snap["queue"]["prior_failures"][0]["cause_addressed"] = True
        snap["queue"]["prior_failures"][0]["addressed_at"] = "2026-10-14T10:30:00Z"
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")

    def test_failure_of_older_head_does_not_block_new_head(self):
        snap = ready()
        snap["queue"]["prior_failures"] = [{"head_sha": OTHER, "cause_addressed": False}]
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")

    def test_authorization_must_match_head_scope_and_not_be_revoked(self):
        for change in ({"head_sha": OTHER}, {"scope": ["review"]}, {"revoked": True}, {"granted_by": ""}):
            snap = ready()
            snap["authorization"].update(change)
            self.assertEqual(reasons(snap), ["AUTHORIZATION_MISSING"], change)

    def test_draft_candidate_blocks(self):
        snap = ready()
        snap["candidate"]["draft"] = True
        self.assertEqual(reasons(snap), ["CANDIDATE_NOT_OPEN_READY"])

    def test_queued_or_merged_candidate_is_noop_never_retried(self):
        for flag in ("queued", "merged"):
            snap = ready()
            snap["candidate"][flag] = True
            snap["checks"]["runs"] = []
            self.assertEqual(evaluate(snap)["verdict"], "NOOP")

    def test_all_blockers_listed_in_fixed_order(self):
        snap = ready()
        snap["authorization"]["revoked"] = True
        snap["threads"].update(total=1, unresolved=1)
        snap["checks"]["runs"] = []
        self.assertEqual(reasons(snap), ["CHECKS_PENDING", "REVIEW_THREADS_UNRESOLVED", "AUTHORIZATION_MISSING"])

    def test_malformed_input_fails_closed(self):
        snap = ready()
        del snap["threads"]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        self.assertEqual(reasons({}), ["INPUT_INCOMPLETE"])
        snap = copy.deepcopy(ready())
        snap["candidate"]["head_sha"] = "abc"
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_non_boolean_gate_fields_fail_closed(self):
        for path in (("evidence", "required", 0, "present"), ("dependencies", 0, "merged"),
                     ("authorization", "revoked"), ("review", "required")):
            snap = ready()
            node = snap
            for key in path[:-1]:
                node = node[key]
            node[path[-1]] = "false"
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"], path)
        snap = ready()
        snap["queue"]["prior_failures"] = [{"head_sha": HEAD, "cause_addressed": "false"}]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_newest_rerun_is_authoritative(self):
        snap = ready()
        snap["checks"]["runs"].append({"name": "game-gate", "head_sha": HEAD, "status": "completed",
                                       "conclusion": "failure", "at": "2026-10-14T10:10:00Z"})
        self.assertEqual(reasons(snap), ["CHECKS_FAILED"])
        snap = ready()
        snap["checks"]["runs"][0]["conclusion"] = "failure"
        snap["checks"]["runs"].append({"name": "game-gate", "head_sha": HEAD, "status": "completed",
                                       "conclusion": "success", "at": "2026-10-14T10:10:00Z"})
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")

    def test_ambiguous_duplicate_runs_fail_closed(self):
        snap = ready()
        snap["checks"]["runs"].append(dict(snap["checks"]["runs"][0], conclusion="failure"))
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_final_unblock_events_extend_ready_at(self):
        snap = ready()
        snap["threads"].update(total=1, resolved_at="2026-10-14T11:00:00Z")
        self.assertEqual(evaluate(snap)["ready_at"], "2026-10-14T11:00:00Z")
        snap["queue"]["prior_failures"] = [{"head_sha": HEAD, "cause_addressed": True,
                                            "addressed_at": "2026-10-14T12:00:00Z"}]
        self.assertEqual(evaluate(snap)["ready_at"], "2026-10-14T12:00:00Z")

    def test_canonical_check_required(self):
        for required in ([], ["other"]):
            snap = ready()
            snap["checks"]["required"] = required
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"], required)

    def test_addressed_failure_requires_timestamp(self):
        for extra in ({}, {"addressed_at": "nope"}):
            snap = ready()
            snap["queue"]["prior_failures"] = [dict({"head_sha": HEAD, "cause_addressed": True}, **extra)]
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"], extra)

    def test_scalar_scope_rejected(self):
        snap = ready()
        snap["authorization"]["scope"] = "no-enqueue"
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_gate_collections_must_be_arrays(self):
        paths = (("checks", "runs"), ("review", "reviews"), ("evidence", "required"),
                 ("dependencies",), ("queue", "prior_failures"))
        for path in paths:
            for bad in ({}, "", "x"):
                snap = ready()
                parent = snap
                for key in path[:-1]:
                    parent = parent[key]
                parent[path[-1]] = bad
                self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"], (path, bad))

    def test_check_names_must_be_strings(self):
        for bad in (7, {}, None, ""):
            snap = ready()
            snap["checks"]["required"].append(bad)
            snap["checks"]["runs"].append(dict(snap["checks"]["runs"][0], name=bad))
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"], bad)

    def test_candidate_flags_must_be_present(self):
        for flag in ("queued", "merged"):
            snap = ready()
            del snap["candidate"][flag]
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"], flag)


class FailClosedTimestampAndCountTest(unittest.TestCase):
    def assert_incomplete(self, snap):
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_unresolved_must_be_non_negative_integer(self):
        for bad in (0.0, False, "0", -1, None, 1.5):
            snap = ready()
            snap["threads"]["unresolved"] = bad
            self.assert_incomplete(snap)

    def test_unresolved_positive_integer_blocks(self):
        snap = ready()
        snap["threads"].update(total=2, unresolved=2)
        self.assertEqual(reasons(snap), ["REVIEW_THREADS_UNRESOLVED"])

    def test_missing_or_invalid_at_is_not_eligible(self):
        paths = (
            (lambda s: s["candidate"], "state_at"),
            (lambda s: s["checks"]["runs"][0], "at"),
            (lambda s: s["review"]["reviews"][0], "at"),
            (lambda s: s["evidence"]["required"][0], "at"),
            (lambda s: s["dependencies"][0], "at"),
            (lambda s: s["authorization"], "at"),
        )
        for get, key in paths:
            for mode in ("missing", "bad", "naive", "empty"):
                snap = ready()
                item = get(snap)
                if mode == "missing":
                    del item[key]
                else:
                    item[key] = {"bad": "not-a-time", "naive": "2026-10-14T10:00:00", "empty": ""}[mode]
                self.assert_incomplete(snap)

    def test_invalid_resolved_at_is_not_eligible(self):
        snap = ready()
        snap["threads"].update(total=1, resolved_at="2026-10-14T10:00:00")
        self.assert_incomplete(snap)

    def test_newest_check_run_is_chosen_by_parsed_datetime(self):
        snap = ready()
        snap["observed_at"] = "2026-10-14T16:00:00Z"
        global NOW
        self.addCleanup(lambda saved=NOW: globals().update(NOW=saved))
        NOW = datetime(2026, 10, 14, 16, 0, 0, tzinfo=timezone.utc)
        # String order disagrees with chronological order across offsets.
        old_fail = {"name": "game-gate", "head_sha": HEAD, "status": "completed",
                    "conclusion": "failure", "at": "2026-10-14T09:00:00-05:00"}  # 14:00Z
        new_pass = {"name": "game-gate", "head_sha": HEAD, "status": "completed",
                    "conclusion": "success", "at": "2026-10-14T15:00:00+02:00"}  # 13:00Z
        snap["checks"]["runs"] = [new_pass, old_fail]
        self.assertEqual(reasons(snap), ["CHECKS_FAILED"])  # old_fail is actually newer
        old_fail["at"] = "2026-10-14T07:00:00-05:00"  # 12:00Z, older than 13:00Z
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")

    def test_duplicate_runs_with_naive_time_are_invalid(self):
        snap = ready()
        snap["checks"]["runs"].append({"name": "game-gate", "head_sha": HEAD, "status": "completed",
                                       "conclusion": "success", "at": "2026-10-14T11:00:00"})
        self.assert_incomplete(snap)


class ShaValidationTest(unittest.TestCase):
    def test_non_hex_or_short_sha_fails_closed_everywhere(self):
        spots = (
            lambda s: s["candidate"], lambda s: s["checks"]["runs"][0], lambda s: s["review"]["reviews"][0],
            lambda s: s["evidence"]["required"][0], lambda s: s["authorization"],
        )
        for bad in ("g" * 40, "a" * 39, "", None, 5):
            for get in spots:
                snap = ready()
                get(snap)["head_sha"] = bad
                self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"], (bad, get(snap)))

    def test_non_hex_sha_in_prior_failure_fails_closed(self):
        snap = ready()
        snap["queue"]["prior_failures"] = [{"head_sha": "g" * 40, "cause_addressed": False}]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_uppercase_hex_matches_same_head(self):
        snap = ready()
        snap["candidate"]["head_sha"] = HEAD.upper()
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")


class AuthorizationAuthorTest(unittest.TestCase):
    def test_untrusted_author_with_forged_granted_by_is_not_eligible(self):
        snap = ready()
        snap["authorization"]["author"] = "random-user"
        snap["authorization"]["granted_by"] = "blakinio"
        result = evaluate(snap)
        self.assertEqual(result["verdict"], "BLOCKED")
        self.assertEqual(reasons(snap), ["AUTHORIZATION_MISSING"])

    def test_missing_or_non_string_author_fails_closed(self):
        for bad in (None, 5, ["blakinio"]):
            snap = ready()
            snap["authorization"]["author"] = bad
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap = ready()
        del snap["authorization"]["author"]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_trusted_author_is_case_insensitive(self):
        snap = ready()
        snap["authorization"]["author"] = "BlakinIO"
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")


class ThreadAndCandidateStateTest(unittest.TestCase):
    def test_resolved_threads_require_resolved_at(self):
        snap = ready()
        snap["threads"]["total"] = 3
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap["threads"]["resolved_at"] = "2026-10-14T12:00:00Z"
        self.assertEqual(evaluate(snap)["ready_at"], "2026-10-14T12:00:00Z")

    def test_thread_total_is_required_and_bounds_unresolved(self):
        snap = ready()
        del snap["threads"]["total"]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap = ready()
        snap["threads"]["unresolved"] = 1
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_missing_or_malformed_candidate_state_is_incomplete(self):
        for field, bad in (("state", None), ("draft", None), ("base_ref", None),
                           ("state", 1), ("draft", "false"), ("base_ref", ["main"])):
            snap = ready()
            if bad is None:
                del snap["candidate"][field]
            else:
                snap["candidate"][field] = bad
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"], (field, bad))

    def test_closed_or_draft_candidate_still_blocks(self):
        for field, value in (("state", "closed"), ("draft", True), ("base_ref", "dev")):
            snap = ready()
            snap["candidate"][field] = value
            self.assertEqual(reasons(snap), ["CANDIDATE_NOT_OPEN_READY"], field)


if __name__ == "__main__":
    unittest.main()
