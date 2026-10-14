import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from enqueue_readiness import evaluate  # noqa: E402

HEAD = "a" * 40
OTHER = "b" * 40


def ready() -> dict:
    return {
        "candidate": {"pr": 1, "head_sha": HEAD, "state": "open", "draft": False, "base_ref": "main"},
        "checks": {
            "required": ["game-gate"],
            "runs": [{"name": "game-gate", "head_sha": HEAD, "status": "completed",
                      "conclusion": "success", "at": "2026-10-14T10:00:00Z"}],
        },
        "review": {"required": True, "reviews": [
            {"head_sha": HEAD, "state": "completed", "verdict": "clean", "at": "2026-10-14T10:05:00Z"}]},
        "threads": {"unresolved": 0},
        "evidence": {"required": [{"name": "task-record", "head_sha": HEAD, "present": True,
                                   "at": "2026-10-14T09:00:00Z"}]},
        "dependencies": [{"pr": 0, "merged": True, "at": "2026-10-14T08:00:00Z"}],
        "queue": {"prior_failures": []},
        "authorization": {"head_sha": HEAD, "scope": ["enqueue"], "revoked": False,
                          "granted_by": "owner", "at": "2026-10-14T09:30:00Z"},
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

    def test_review_not_required_is_not_demanded(self):
        snap = ready()
        snap["review"] = {"required": False, "reviews": []}
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")

    def test_unresolved_threads_block(self):
        snap = ready()
        snap["threads"]["unresolved"] = 2
        self.assertEqual(reasons(snap), ["REVIEW_THREADS_UNRESOLVED"])

    def test_evidence_must_be_present_and_bound_to_head(self):
        snap = ready()
        snap["evidence"]["required"][0]["head_sha"] = OTHER
        self.assertEqual(reasons(snap), ["EVIDENCE_MISSING"])

    def test_unmerged_dependency_blocks(self):
        snap = ready()
        snap["dependencies"][0]["merged"] = False
        self.assertEqual(reasons(snap), ["DEPENDENCY_NOT_MERGED"])

    def test_prior_mq_failure_blocks_same_head_until_addressed(self):
        snap = ready()
        snap["queue"]["prior_failures"] = [
            {"head_sha": HEAD, "cause": "lint", "cause_addressed": False, "at": "2026-10-14T10:20:00Z"}]
        self.assertEqual(reasons(snap), ["PRIOR_MQ_FAILURE_UNRESOLVED"])
        snap["queue"]["prior_failures"][0]["cause_addressed"] = True
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
        snap["threads"]["unresolved"] = 1
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


if __name__ == "__main__":
    unittest.main()
