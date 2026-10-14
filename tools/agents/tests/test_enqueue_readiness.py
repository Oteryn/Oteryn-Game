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
        "authorization": [{"head_sha": HEAD, "scope": ["enqueue"], "revoked": False,
                           "granted_by": "owner", "author": "blakinio", "at": "2026-10-14T09:30:00Z"}],
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

    def test_skews_do_not_compose_past_the_evaluator_clock(self):
        snap = ready()
        snap["observed_at"] = "2026-10-14T12:02:00Z"  # within 120 s of NOW (12:00:30)
        snap["checks"]["runs"][0]["at"] = "2026-10-14T12:04:00Z"  # within 120 s of observed_at, 210 s past NOW
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap["checks"]["runs"][0]["at"] = "2026-10-14T12:02:30Z"  # exactly now + 120 s
        result = evaluate(snap)
        self.assertEqual((result["verdict"], result["ready_at"]), ("ENQUEUE_ELIGIBLE", "2026-10-14T12:02:30Z"))

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
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])  # no classification or diagnoser
        snap["queue"]["prior_failures"][0].update(cause_class="external_infra", diagnosed_by="blakinio")
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")

    def test_failure_of_older_head_does_not_block_new_head(self):
        snap = ready()
        snap["queue"]["prior_failures"] = [{"head_sha": OTHER, "cause_addressed": False}]
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")

    def test_authorization_must_match_head_scope_and_not_be_revoked(self):
        for change in ({"head_sha": OTHER}, {"scope": ["review"]}, {"revoked": True}, {"granted_by": ""}):
            snap = ready()
            snap["authorization"][0].update(change)
            self.assertEqual(reasons(snap), ["AUTHORIZATION_MISSING"], change)

    def test_newest_authorization_record_governs(self):
        snap = ready()
        grant = snap["authorization"][0]
        revoke = dict(grant, revoked=True, at="2026-10-14T09:40:00Z")
        snap["authorization"] = [revoke, grant]  # order in the list is irrelevant
        self.assertEqual(reasons(snap), ["AUTHORIZATION_MISSING"])
        narrow = dict(grant, scope=["review"], at="2026-10-14T09:40:00Z")
        snap["authorization"] = [grant, narrow]
        self.assertEqual(reasons(snap), ["AUTHORIZATION_MISSING"])
        other_head = dict(grant, head_sha=OTHER, at="2026-10-14T09:40:00Z")
        snap["authorization"] = [grant, other_head]
        self.assertEqual(reasons(snap), ["AUTHORIZATION_MISSING"])
        regrant = dict(grant, at="2026-10-14T09:50:00Z")
        snap["authorization"] = [grant, revoke, regrant]
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")
        self.assertEqual(evaluate(snap)["ready_at"], "2026-10-14T10:05:00Z")

    def test_authorization_history_ties_and_empty_fail_closed(self):
        snap = ready()
        grant = snap["authorization"][0]
        snap["authorization"] = [grant, dict(grant, revoked=True)]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap["authorization"] = [grant, dict(grant)]
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")
        # A tie that differs only in granted_by is ambiguous in either order.
        for pair in ([dict(grant, granted_by=""), grant], [grant, dict(grant, granted_by="")]):
            snap["authorization"] = pair
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap["authorization"] = [grant, dict(grant, scope=list(reversed(grant["scope"])))]
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")
        snap["authorization"] = []
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap["authorization"] = grant  # a bare object is not a history
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_untrusted_newer_record_cannot_mask_or_revoke(self):
        snap = ready()
        grant = snap["authorization"][0]
        snap["authorization"] = [grant, dict(grant, author="random-user", revoked=True, at="2026-10-14T09:45:00Z")]
        self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE")

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
        snap["authorization"][0]["revoked"] = True
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
                     ("authorization", 0, "revoked"), ("review", "required")):
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
        snap["queue"]["prior_failures"] = [{"head_sha": HEAD, "cause_class": "external_infra", "diagnosed_by": "blakinio", "cause_addressed": True, "at": "2026-10-14T10:20:00Z",
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
            snap["queue"]["prior_failures"] = [dict({"head_sha": HEAD, "cause_class": "external_infra", "diagnosed_by": "blakinio", "cause_addressed": True, "at": "2026-10-14T10:20:00Z"}, **extra)]
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"], extra)

    def test_diagnosis_must_classify_cause_as_external_by_trusted_author(self):
        base = {"head_sha": HEAD, "cause": "test failed", "cause_addressed": True, "at": "2026-10-14T10:20:00Z",
                "addressed_at": "2026-10-14T10:30:00Z", "cause_class": "external_infra", "diagnosed_by": "blakinio"}
        for cls in ("external_infra", "base_branch_change", "flaky_runner_loss"):
            snap = ready()
            snap["queue"]["prior_failures"] = [dict(base, cause_class=cls)]
            self.assertEqual(evaluate(snap)["verdict"], "ENQUEUE_ELIGIBLE", cls)
        for change in ({"cause_class": "candidate_defect"}, {"cause_class": ""}, {"cause_class": "EXTERNAL_INFRA"},
                       {"diagnosed_by": "mallory"}):
            snap = ready()
            snap["queue"]["prior_failures"] = [dict(base, **change)]
            self.assertEqual(reasons(snap), ["PRIOR_MQ_FAILURE_UNRESOLVED"], change)
        for change in ({"cause_class": None}, {"cause_class": 5}, {"diagnosed_by": None}, {"addressed_at": "nope"}):
            snap = ready()
            snap["queue"]["prior_failures"] = [dict(base, **change)]
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"], change)
        for key in ("cause_class", "diagnosed_by"):
            snap = ready()
            snap["queue"]["prior_failures"] = [{k: v for k, v in base.items() if k != key}]
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"], key)
        snap = ready()  # a bad class with a malformed diagnosis time is still invalid input, not silently unresolved
        snap["queue"]["prior_failures"] = [dict(base, cause_class="candidate_defect", addressed_at="nope")]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_failure_diagnosis_must_follow_the_failure(self):
        base = {"head_sha": HEAD, "cause_class": "external_infra", "diagnosed_by": "blakinio", "cause_addressed": True, "at": "2026-10-14T10:20:00Z"}
        for addressed_at, expected in (("2026-10-14T10:19:59Z", ["PRIOR_MQ_FAILURE_UNRESOLVED"]),
                                       ("2026-10-14T10:20:00Z", [])):
            snap = ready()
            snap["queue"]["prior_failures"] = [dict(base, addressed_at=addressed_at)]
            self.assertEqual(reasons(snap), expected, addressed_at)
        for bad in (None, "", "nope", "2026-10-14T10:20:00", 5):  # same-head failure needs a valid failure time
            for flag in (True, False):
                snap = ready()
                snap["queue"]["prior_failures"] = [dict(base, cause_addressed=flag, addressed_at="2026-10-14T11:00:00Z")]
                snap["queue"]["prior_failures"][0]["at"] = bad
                self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"], (bad, flag))
        snap = ready()
        del base["at"]
        snap["queue"]["prior_failures"] = [dict(base, addressed_at="2026-10-14T11:00:00Z")]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap["queue"]["prior_failures"] = [dict(base, at="2026-10-14T10:20:00Z", addressed_at="2026-10-14T12:05:00Z")]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])  # addressed_at beyond now + skew

    def test_every_event_time_is_bounded_by_observed_at(self):
        late = "2026-10-14T11:00:00Z"  # after observed_at (08:00) but before NOW (12:00:30)
        def probe(mutate):
            snap = ready()
            snap["observed_at"] = "2026-10-14T08:00:00Z"
            for key in ("state_at",):
                snap["candidate"][key] = "2026-10-14T07:00:00Z"
            for run in snap["checks"]["runs"]:
                run["at"] = "2026-10-14T07:00:00Z"
            for r in snap["review"]["reviews"]:
                r["at"] = "2026-10-14T07:00:00Z"
            for a in snap["authorization"]:
                a["at"] = "2026-10-14T07:00:00Z"
            snap["threads"].pop("resolved_at", None)
            for item in snap["evidence"]["required"]:
                item["at"] = "2026-10-14T07:00:00Z"
            for dep in snap["dependencies"]:
                dep["at"] = "2026-10-14T07:00:00Z"
            mutate(snap)
            return snap
        self.assertEqual(evaluate(probe(lambda s: None))["verdict"], "ENQUEUE_ELIGIBLE")
        setters = {
            "state_at": lambda s: s["candidate"].update(state_at=late),
            "check": lambda s: s["checks"]["runs"][0].update(at=late),
            "review": lambda s: s["review"]["reviews"][0].update(at=late),
            "authorization": lambda s: s["authorization"][0].update(at=late),
            "older_review": lambda s: s["review"]["reviews"].append(
                dict(s["review"]["reviews"][0], state="running", at=late)),
            "older_authorization": lambda s: s["authorization"].append(
                dict(s["authorization"][0], revoked=True, at=late)),
        }
        for name, mutate in setters.items():
            with self.subTest(name):
                self.assertEqual(reasons(probe(mutate)), ["INPUT_INCOMPLETE"])

    def test_future_dated_selecting_records_are_rejected(self):
        future = "2026-10-14T12:10:00Z"
        snap = ready()  # a future-dated review, rerun, authorization or failure cannot become "newest"
        snap["review"]["reviews"].append(dict(snap["review"]["reviews"][0], state="running", at=future))
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap = ready()
        snap["checks"]["runs"].append(dict(snap["checks"]["runs"][0], conclusion="failure", at=future))
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap = ready()
        snap["authorization"].append(dict(snap["authorization"][0], revoked=True, at=future))
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap = ready()
        snap["queue"]["prior_failures"] = [{"head_sha": HEAD, "cause_addressed": False, "at": future}]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_scalar_scope_rejected(self):
        snap = ready()
        snap["authorization"][0]["scope"] = "no-enqueue"
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
            (lambda s: s["authorization"][0], "at"),
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
            lambda s: s["evidence"]["required"][0], lambda s: s["authorization"][0],
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
        snap["authorization"][0]["author"] = "random-user"
        snap["authorization"][0]["granted_by"] = "blakinio"
        result = evaluate(snap)
        self.assertEqual(result["verdict"], "BLOCKED")
        self.assertEqual(reasons(snap), ["AUTHORIZATION_MISSING"])

    def test_missing_or_non_string_author_fails_closed(self):
        for bad in (None, 5, ["blakinio"]):
            snap = ready()
            snap["authorization"][0]["author"] = bad
            self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])
        snap = ready()
        del snap["authorization"][0]["author"]
        self.assertEqual(reasons(snap), ["INPUT_INCOMPLETE"])

    def test_trusted_author_is_case_insensitive(self):
        snap = ready()
        snap["authorization"][0]["author"] = "BlakinIO"
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
