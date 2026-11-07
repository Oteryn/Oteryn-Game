"""Reject incomplete answers, mutable provenance and OTS-to-Global promotion."""
from copy import deepcopy
import hashlib
import unittest

import behavior_answers as answers


def fixture():
    sources = {}
    for index, engine in enumerate(sorted(answers.ENGINE_IDS), 1):
        revision = str(index) * 40
        repo = "opentibiabr/canary" if engine == "canary" else "zimbadev/crystalserver"
        sources[engine + "_source"] = {
            "url": f"https://github.com/{repo}/blob/{revision}/src/player.cpp",
            "revision": revision, "sha256": str(index) * 64,
            "method": "NORMAL_HTTP_PUBLIC_SOURCE_FILE", "role": "OTS_IMPLEMENTATION_REFERENCE",
            "digest_scope": "Exact complete public source file bytes at the pinned commit",
            "repo": repo, "branch": "main" if engine == "canary" else
            ("imbuements" if engine == "crystal_imbuements" else "summer-update"),
            "line_start": 10, "line_end": 20,
        }
    questions = []
    for qid in sorted(answers.QUESTION_IDS):
        owner = qid == "exact_target_snapshot"
        questions.append({
            "id": qid,
            "resolution_status": "OWNER_SCOPE_RESOLVED" if owner else "SOURCE_QUALIFIED_ANSWER",
            "answer": "Current October scope selected." if owner else "Named engine behavior is source-qualified.",
            "engine_answers": {} if owner else {
                engine: {"answer": "The pinned implementation answers this case.",
                         "source_refs": [engine + "_source"]}
                for engine in answers.ENGINE_IDS},
            "global_evidence": {
                "status": "OWNER_SCOPE_RESOLVED" if owner else "PARTIALLY_SOURCED_CURRENT_PUBLIC_REFERENCE",
                "answer": "Owner requests current data." if owner else "Public context supports a bounded answer.",
                "source_refs": [] if owner else [{"catalogue": "global-rules-evidence.json", "id": "public_manual"}],
            },
            "global_limitations": [] if owner else ["Engine internals are not independent official Global evidence."],
        })
    return {
        "schema": "OTERYN_IMBUEMENT_CURRENT_BEHAVIOR_ANSWERS/v1", "target": answers.TARGET,
        "activation": "DRAFT_NOT_RUNTIME_READY", "as_of": "2026-10-01",
        "sources": sources, "questions": questions,
        "counts": {"original_questions": 12, "owner_scope_resolved": 1, "engine_answered_questions": 11},
    }


class BehaviorAnswerTests(unittest.TestCase):
    def setUp(self):
        self.packet = fixture()
        self.global_evidence = {"sources": {
            "public_manual": {"role": "OFFICIAL_PUBLIC"},
            "ots_code": {"role": "OTS_HYPOTHESIS_ONLY"},
        }}
        self.combat_evidence = {"sources": [{"id": "community_formula", "role": "DERIVED_COMMUNITY"}]}

    def validate(self, packet):
        return answers.validate(packet, global_evidence=self.global_evidence,
                                combat_evidence=self.combat_evidence)

    def question(self, packet=None, qid="etcher_consumption"):
        return next(q for q in (packet or self.packet)["questions"] if q["id"] == qid)

    def reject(self, change):
        packet = deepcopy(self.packet)
        change(packet)
        self.assertTrue(self.validate(packet))

    def test_complete_current_answer_packet_validates_without_mutation(self):
        before = deepcopy(self.packet)
        self.assertEqual(self.validate(self.packet), [])
        self.assertEqual(self.packet, before)

    def test_wrong_current_target_date_activation_and_schema_are_rejected(self):
        for key, value in (
            ("target", "global-tibia-observable-2026-07-28-post-server-save"),
            ("as_of", "2026-07-28"), ("activation", "RUNTIME_READY"), ("schema", "other/v1"),
        ):
            with self.subTest(key=key):
                self.reject(lambda p: p.update({key: value}))

    def test_missing_duplicate_and_extra_original_questions_are_rejected(self):
        self.reject(lambda p: p["questions"].pop())
        self.reject(lambda p: p["questions"].append(deepcopy(p["questions"][0])))
        self.reject(lambda p: p["questions"][0].update({"id": p["questions"][1]["id"]}))
        self.reject(lambda p: p["questions"][0].update({"id": "invented_new_gap"}))

    def test_all_three_engine_answers_and_local_sources_are_required(self):
        self.reject(lambda p: self.question(p)["engine_answers"].pop("canary"))
        self.reject(lambda p: self.question(p)["engine_answers"]["canary"].update({"answer": " "}))
        for refs in ([], ["missing_source"], [{"catalogue": "global-rules-evidence.json", "id": "public_manual"}], None):
            with self.subTest(refs=refs):
                self.reject(lambda p: self.question(p)["engine_answers"]["canary"].update({"source_refs": refs}))

    def test_only_original_snapshot_question_is_owner_resolved(self):
        self.reject(lambda p: self.question(p).update({"resolution_status": "OWNER_SCOPE_RESOLVED"}))
        self.reject(lambda p: self.question(p, "exact_target_snapshot").update({"engine_answers": {"canary": {}}}))
        self.reject(lambda p: self.question(p)["global_evidence"].update({"status": "OWNER_SCOPE_RESOLVED"}))

    def test_engine_answers_cannot_cite_a_different_engine_or_named_branch(self):
        self.reject(lambda p: self.question(p)["engine_answers"]["canary"].update({
            "source_refs": ["crystal_imbuements_source"]}))
        self.reject(lambda p: self.question(p)["engine_answers"]["crystal_imbuements"].update({
            "source_refs": ["crystal_summer_update_source"]}))
        self.reject(lambda p: p["sources"]["crystal_imbuements_source"].update({"engine": "crystal_summer_update"}))

    def test_nested_anchors_and_trace_context_refs_are_qualified(self):
        source = self.packet["sources"]["canary_source"]
        anchor = {"source_ref": "canary_source", "line": 10, "expression": "item:remove(1)",
                  "excerpt_sha256": hashlib.sha256(b"item:remove(1)").hexdigest(),
                  "github_anchor": source["url"] + "#L10"}
        self.question()["engine_answers"]["canary"]["code_anchors"] = [anchor]
        self.question()["engine_answers"]["canary"]["trace"] = {"validation": {"source_ref": "canary_source"}}
        self.assertEqual(self.validate(self.packet), [])
        self.reject(lambda p: self.question(p)["engine_answers"]["canary"]["trace"]["validation"].update({
            "source_ref": "crystal_imbuements_source"}))
        self.reject(lambda p: self.question(p)["engine_answers"]["canary"]["code_anchors"][0].update({
            "github_anchor": source["url"].replace("1" * 40, "2" * 40) + "#L10"}))
        self.reject(lambda p: self.question(p)["engine_answers"]["canary"]["code_anchors"][0].update({
            "expression": "item:remove(2)"}))

    def test_mutable_or_misbound_github_provenance_is_rejected(self):
        source = self.packet["sources"]["canary_source"]
        urls = [
            source["url"].replace("1" * 40, "main"),
            source["url"].replace("1" * 40, "2" * 40),
            source["url"].replace("https://github.com", "https://example.com"),
            source["url"] + "?raw=true", source["url"] + "#L10",
            source["url"].replace("/src/player.cpp", "/src/../player.cpp"),
        ]
        for url in urls:
            with self.subTest(url=url):
                self.reject(lambda p: p["sources"]["canary_source"].update({"url": url}))
        self.reject(lambda p: p["sources"]["canary_source"].update({"repo": "other/repository"}))

    def test_digest_revision_retrieval_method_and_line_ranges_are_guarded(self):
        for key, value in (
            ("sha256", "a" * 63), ("revision", "main"),
            ("method", "INDEXED_SEARCH_SNIPPET"), ("digest_scope", ""),
            ("line_start", False), ("line_end", 5),
        ):
            with self.subTest(key=key):
                self.reject(lambda p: p["sources"]["canary_source"].update({key: value}))

    def test_ots_sources_cannot_be_promoted_to_public_global_evidence(self):
        self.reject(lambda p: p["sources"]["canary_source"].update({"role": "OFFICIAL_GLOBAL"}))
        self.reject(lambda p: self.question(p)["global_evidence"].update({"source_refs": [
            {"catalogue": "global-rules-evidence.json", "id": "ots_code"}]}))
        self.reject(lambda p: self.question(p)["global_evidence"].update({"status": "OFFICIAL_GLOBAL_CONFIRMED"}))

    def test_unknown_and_malformed_public_references_are_rejected(self):
        for refs in (
            ["public_manual"], [{"catalogue": "global-rules-evidence.json", "id": "missing"}],
            [{"catalogue": "current-behavior-answers.json", "id": "canary_source"}],
            [{"catalogue": "imbuement-combat.json", "id": "missing"}],
            [{"catalogue": "global-rules-evidence.json", "id": "public_manual", "extra": "invented"}],
        ):
            with self.subTest(refs=refs):
                self.reject(lambda p: self.question(p)["global_evidence"].update({"source_refs": refs}))

    def test_both_public_catalogue_shapes_and_unconfirmed_context_refs_work(self):
        q = self.question()
        q["global_evidence"]["source_refs"] = [{"catalogue": "imbuement-combat.json", "id": "community_formula"}]
        self.assertEqual(self.validate(self.packet), [])
        q["global_evidence"]["status"] = "NO_EXPLICIT_PUBLIC_CONFIRMATION"
        self.assertEqual(self.validate(self.packet), [])
        q["global_evidence"]["source_refs"] = []
        self.assertEqual(self.validate(self.packet), [])

    def test_missing_public_support_or_limitations_cannot_claim_resolution(self):
        self.reject(lambda p: self.question(p)["global_evidence"].update({"source_refs": []}))
        self.reject(lambda p: self.question(p).update({"global_limitations": []}))
        self.reject(lambda p: self.question(p).update({"global_limitations": [""]}))
        self.reject(lambda p: self.question(p).update({"answer": ""}))

    def test_counts_are_exact_and_cannot_accept_booleans(self):
        for key, value in (("original_questions", 11), ("owner_scope_resolved", True), ("engine_answered_questions", 12)):
            with self.subTest(key=key):
                self.reject(lambda p: p["counts"].update({key: value}))

    def test_malformed_json_value_shapes_return_errors_instead_of_crashing(self):
        self.assertTrue(self.validate([]))
        for value in (None, [], "wrong"):
            self.reject(lambda p: p.update({"sources": value}))
            self.reject(lambda p: p.update({"questions": value}))
            self.reject(lambda p: self.question(p)["global_evidence"].update({"status": value}))


if __name__ == "__main__":
    unittest.main()
