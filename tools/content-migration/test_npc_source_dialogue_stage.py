"""Custody and fail-closed source dialogue admission tests."""

import copy
import importlib.util
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "npc_source_dialogue_stage",
    Path(__file__).with_name("npc_source_dialogue_stage.py"),
)
s = importlib.util.module_from_spec(spec)
spec.loader.exec_module(s)


class SourceStageTests(unittest.TestCase):
    def fixture(self):
        roots = [
            {
                "key": "name",
                "triggers": ["name"],
                "reply": ["Welcome, Player."],
                "children": [
                    {"key": "yes", "triggers": ["yes"], "reply": ["First.", "Second."]}
                ],
            },
            {"key": "job", "triggers": ["job"], "reply": ["I sell food."]},
        ]
        raw = b"Player: name\nSam: Welcome, Player.\nPlayer: job\nSam: I sell food.\n"
        declaration = {
            "identity": {"key": "dialogue.sam", "revision": "r4"},
            "keywords": roots,
            "fields": [],
        }
        source = {
            "npc_key": "npc.sam",
            "name": "Sam",
            "dialogue": dict(declaration["identity"], family="Dialogue"),
            "accepted_roots": 2,
            "candidate_keywords_sha256": s.digest(s.source_encode(roots)),
            "held_root_groups": 4,
            "held_groups_sha256": "held-raw-digest",
            "source_status": "VERIFIED",
            "source_id": "Sam.txt",
            "source_metadata": {"raw_sha256": s.digest(raw)},
        }
        proofs = [
            [root["key"], s.semantic_hash(root), [[0, f"/source/{i}", ["T1"], i]]]
            for i, root in enumerate(roots)
        ]
        packet = {
            "source-index.json": {"records": [source]},
            "dialogue-candidates.json": {"records": [declaration]},
            "root-proofs.json": {
                "npcs": [{"npc_key": "npc.sam", "root_proofs": proofs}]
            },
        }
        return packet, raw

    def test_verified_recording_does_not_authorize_matcher_or_quest(self):
        packet, _ = self.fixture()
        old = copy.deepcopy(packet)
        result = s.stage(packet, [], [])
        row = result["records"][0]
        self.assertEqual(row["state"], "SOURCE_ONLY")
        self.assertFalse(row["runtime_qualified"])
        self.assertEqual(
            row["source_declaration"]["keywords"],
            row["candidate_declaration"]["keywords"],
        )
        self.assertEqual(
            row["source_root_proofs"],
            packet["root-proofs.json"]["npcs"][0]["root_proofs"],
        )
        self.assertEqual(row["held_root_groups"], 4)
        self.assertEqual(row["held_groups_sha256"], "held-raw-digest")
        self.assertEqual(packet, old)
        self.assertEqual(s.encode(result), s.encode(s.stage(packet, [], [])))

    def test_recorded_player_context_normalizes_without_changing_source_proof(self):
        packet, raw = self.fixture()
        row = s.stage(packet, [], [], {"npc.sam": raw})["records"][0]
        self.assertEqual(
            row["candidate_declaration"]["keywords"][0]["reply"],
            ["Welcome, |PLAYERNAME|."],
        )
        self.assertEqual(
            row["source_declaration"]["keywords"][0]["reply"], ["Welcome, Player."]
        )
        self.assertEqual(
            row["candidate_declaration"]["keywords"][0]["children"][0]["reply"],
            ["First.", "Second."],
        )
        link = row["candidate_proof_links"][0]
        self.assertEqual(
            link["source_semantic_sha256"],
            packet["root-proofs.json"]["npcs"][0]["root_proofs"][0][1],
        )
        self.assertNotEqual(
            link["source_semantic_sha256"], link["candidate_semantic_sha256"]
        )
        self.assertEqual(row["transformations"][0]["pointer"], "/keywords/0/reply/0")

    def test_observed_nickname_is_not_guessed(self):
        packet, _ = self.fixture()
        raw = b"Visitor: name\nSam: Welcome, Player.\n"
        packet["source-index.json"]["records"][0]["source_metadata"]["raw_sha256"] = (
            s.digest(raw)
        )
        row = s.stage(packet, [], [], {"npc.sam": raw})["records"][0]
        self.assertEqual(
            row["candidate_declaration"]["keywords"][0]["reply"], ["Welcome, Player."]
        )
        self.assertEqual(row["transformations"], [])

    def test_recaptured_custody_mismatch_fails(self):
        packet, _ = self.fixture()
        with self.assertRaisesRegex(s.StageError, "transcript digest mismatch"):
            s.stage(packet, [], [], {"npc.sam": b"altered recording"})

    def test_reply_order_and_child_mutation_fail_source_proof(self):
        for mutate in ("reverse", "child"):
            packet, _ = self.fixture()
            roots = packet["dialogue-candidates.json"]["records"][0]["keywords"]
            if mutate == "reverse":
                roots.reverse()
            else:
                roots[0]["children"][0]["reply"].reverse()
            packet["source-index.json"]["records"][0]["candidate_keywords_sha256"] = (
                s.digest(s.source_encode(roots))
            )
            with self.assertRaisesRegex(s.StageError, "root proof mismatch"):
                s.stage(packet, [], [])

    def test_generated_child_macro_holds_complete_root_instead_of_truncating(self):
        packet, _ = self.fixture()
        roots = packet["dialogue-candidates.json"]["records"][0]["keywords"]
        roots[0]["children"][0]["reply"] = ["Browse <GetFormattedShopCategoryNames()>"]
        packet["source-index.json"]["records"][0]["candidate_keywords_sha256"] = (
            s.digest(s.source_encode(roots))
        )
        packet["root-proofs.json"]["npcs"][0]["root_proofs"][0][1] = s.semantic_hash(
            roots[0]
        )
        row = s.stage(packet, [], [])["records"][0]
        self.assertEqual(len(row["source_declaration"]["keywords"]), 2)
        self.assertEqual(
            [r["key"] for r in row["candidate_declaration"]["keywords"]], ["job"]
        )
        self.assertTrue(row["candidate_proof_links"][0]["native_candidate_held"])
        self.assertEqual(
            row["transformations"][0]["pointer"], "/keywords/0/children/0/reply/0"
        )

    def test_empty_hold_preserved(self):
        packet, _ = self.fixture()
        declaration = packet["dialogue-candidates.json"]["records"][0]
        declaration["keywords"] = []
        source = packet["source-index.json"]["records"][0]
        source["accepted_roots"] = 0
        source["candidate_keywords_sha256"] = s.digest(s.source_encode([]))
        packet["root-proofs.json"]["npcs"] = []
        row = s.stage(packet, [], [])["records"][0]
        self.assertEqual(row["candidate_declaration"]["keywords"], [])
        self.assertIn("EMPTY_SOURCE_HOLD", row["holds"])
        self.assertEqual(row["state"], "SOURCE_ONLY")

    def test_admission_requires_all_three_frozen_semantics_and_evidence(self):
        packet, _ = self.fixture()
        q = {
            "game_keyword_matcher_proven": True,
            "game_handler_precedence_proven": True,
            "quest_guards_focus_actions_proven": True,
            "evidence": ["fixture:accepted-static-semantics"],
            "source_keywords_sha256": packet["source-index.json"]["records"][0][
                "candidate_keywords_sha256"
            ],
        }
        row = s.stage(packet, [], [], qualifications={"npc.sam": q})["records"][0]
        self.assertEqual(row["state"], "STATIC_ADMISSION_CANDIDATE")
        self.assertFalse(row["runtime_qualified"])
        q.pop("game_keyword_matcher_proven")
        self.assertEqual(
            s.stage(packet, [], [], qualifications={"npc.sam": q})["records"][0][
                "state"
            ],
            "SOURCE_ONLY",
        )

    def test_qualification_for_unknown_npc_rejected(self):
        packet, _ = self.fixture()
        with self.assertRaisesRegex(s.StageError, "unknown NPC"):
            s.stage(packet, [], [], qualifications={"npc.unallocated": {}})

    def static_fixture(self):
        packet, raw = self.fixture()
        native = copy.deepcopy(packet["dialogue-candidates.json"]["records"][0])
        native["keywords"][1]["reply"] = ["I sell wares."]
        npcs = [
            {
                "identity": {"key": "npc.sam"},
                "dialogue": dict(native["identity"], family="Dialogue"),
            }
        ]
        admitted = {
            "source": {"canary_revision": "frozen-source"},
            "dialogues": [
                {
                    "npc": "npc.sam",
                    "declaration": copy.deepcopy(native),
                    "provenance": ["canary", "crystal"],
                }
            ],
        }
        return packet, raw, native, npcs, admitted

    def test_actual_static_reply_repair_preserves_matcher_flags_and_order(self):
        packet, raw, native, npcs, admitted = self.static_fixture()
        old = copy.deepcopy(native)
        result = s.static_repairs(packet, npcs, [native], {"npc.sam": raw}, admitted)
        row = result["records"][0]
        self.assertEqual(
            row["native_declaration"]["keywords"][1]["reply"], ["I sell food."]
        )
        for before, after in zip(
            native["keywords"], row["native_declaration"]["keywords"]
        ):
            self.assertEqual(
                {k: v for k, v in before.items() if k != "reply"},
                {k: v for k, v in after.items() if k != "reply"},
            )
        self.assertFalse(row["runtime_qualified"])
        self.assertEqual(native, old)
        self.assertEqual(row["changes"][-1]["proof"]["recorded_player_lines"], [3])

    def test_unanchored_static_native_declaration_is_not_repaired(self):
        packet, raw, native, npcs, admitted = self.static_fixture()
        native["keywords"][1]["reset"] = True
        self.assertEqual(
            s.static_repairs(packet, npcs, [native], {"npc.sam": raw}, admitted)[
                "records"
            ],
            [],
        )

    def test_ambiguous_recorded_reply_does_not_replace_static_branch(self):
        packet, raw, native, npcs, admitted = self.static_fixture()
        raw += b"Player: job\nSam: Conditioned reply.\n"
        packet["source-index.json"]["records"][0]["source_metadata"]["raw_sha256"] = (
            s.digest(raw)
        )
        row = s.static_repairs(packet, npcs, [native], {"npc.sam": raw}, admitted)[
            "records"
        ][0]
        self.assertEqual(
            row["native_declaration"]["keywords"][1]["reply"], ["I sell wares."]
        )
        self.assertFalse(
            any(c["pointer"] == "/keywords/1/reply" for c in row["changes"])
        )

    def test_unknown_slash_alias_is_held_without_inventing_native_matcher(self):
        packet, raw, native, npcs, admitted = self.static_fixture()
        raw = raw.replace(b"Player: job", b"Player: job / shop")
        source = packet["source-index.json"]["records"][0]
        source["source_metadata"]["raw_sha256"] = s.digest(raw)
        roots = packet["dialogue-candidates.json"]["records"][0]["keywords"]
        roots.append({"key": "shop", "triggers": ["shop"], "reply": ["I sell food."]})
        source["accepted_roots"] = 3
        source["candidate_keywords_sha256"] = s.digest(s.source_encode(roots))
        packet["root-proofs.json"]["npcs"][0]["root_proofs"].append(
            ["shop", s.semantic_hash(roots[-1]), [[0, "/source/2", ["T3"], 3]]]
        )
        result = s.static_repairs(packet, npcs, [native], {"npc.sam": raw}, admitted)
        self.assertEqual(result["counts"]["held_alias_proposals"], 1)
        self.assertEqual(
            result["unqualified_alias_proposals"][0]["after"]["triggers"], ["shop"]
        )
        self.assertEqual(len(result["records"][0]["native_declaration"]["keywords"]), 2)

    def test_trade_macro_expansion_preserves_existing_message_hook(self):
        packet, raw, native, npcs, admitted = self.static_fixture()
        native["send_trade"] = ["Browse <GetFormattedShopCategoryNames()>."]
        admitted["dialogues"][0]["declaration"] = copy.deepcopy(native)
        raw += b"Player: trade\nSam: Browse potions, wands or runes.\n"
        source = packet["source-index.json"]["records"][0]
        source["source_metadata"]["raw_sha256"] = s.digest(raw)
        roots = packet["dialogue-candidates.json"]["records"][0]["keywords"]
        roots.append(
            {
                "key": "trade",
                "triggers": ["trade"],
                "reply": ["Browse potions, wands or runes."],
            }
        )
        source["accepted_roots"] = 3
        source["candidate_keywords_sha256"] = s.digest(s.source_encode(roots))
        packet["root-proofs.json"]["npcs"][0]["root_proofs"].append(
            ["trade", s.semantic_hash(roots[-1]), [[0, "/source/2", ["T3"], 5]]]
        )
        result = s.static_repairs(packet, npcs, [native], {"npc.sam": raw}, admitted)
        self.assertEqual(result["counts"]["native_message_expansions"], 1)
        declaration = result["records"][0]["native_declaration"]
        self.assertEqual(declaration["send_trade"], ["Browse potions, wands or runes."])
        self.assertFalse(
            any(node["triggers"] == ["trade"] for node in declaration["keywords"])
        )

    def test_stale_qualification_cannot_admit_changed_source(self):
        packet, _ = self.fixture()
        with self.assertRaisesRegex(s.StageError, "not bound to current source"):
            s.stage(
                packet,
                [],
                [],
                qualifications={
                    "npc.sam": {
                        "source_keywords_sha256": "old-candidate-hash",
                        "evidence": ["old-review"],
                    }
                },
            )

    def test_extra_source_proof_identity_is_not_silently_ignored(self):
        packet, _ = self.fixture()
        packet["root-proofs.json"]["npcs"].append(
            {"npc_key": "npc.unallocated", "root_proofs": []}
        )
        with self.assertRaisesRegex(s.StageError, "NPC inventory mismatch"):
            s.stage(packet, [], [])

    def test_file_digest_custody_checked_before_parsing_candidates(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            data = b'{"records":[]}\n'
            (root / "data.json").write_bytes(data)
            manifest = {
                "scope": "SOURCE_AUTHORING_CANDIDATES_ONLY",
                "files": [
                    {"path": "data.json", "bytes": len(data), "sha256": s.digest(data)}
                ],
            }
            (root / "manifest.json").write_bytes(s.encode(manifest))
            self.assertEqual(s.load_packet(root)["data.json"], {"records": []})
            (root / "data.json").write_bytes(b'{"records":[1]}\n')
            with self.assertRaisesRegex(s.StageError, "custody mismatch"):
                s.load_packet(root)


if __name__ == "__main__":
    unittest.main()
