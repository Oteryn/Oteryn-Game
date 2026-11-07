"""Quest source-stage identity, custody and fail-closed dependency regressions."""

import json
import tempfile
import unittest
from pathlib import Path

import npc_quest_binding_stage as q


def npc(key="oteryn:npc.alice", external="alice"):
    identity = {"key": key, "revision": "definition-r1"}
    binding = {
        "disposition": "EXACT",
        "external_id": external,
        "identity_namespace": "canary/npc-file",
        "source_key": "oteryn:source.canary",
        "source_revision": "newer-npc-revision",
        "target": {"family": "NPC", **identity},
    }
    return {
        "declaration": {"kind": "NPC", "identity": identity},
        "source_bindings": [binding],
    }


def source_item(key="canary:item/3233"):
    return {"family": "Item", "key": key, "revision": "source-r1"}


class QuestStageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.write(
            "content/npcs/definitions/npcs-00000-00499.json", {"records": [npc()]}
        )
        self.write(
            "content/items/definitions/items-00000-00499.json",
            {
                "records": [
                    {
                        "definition": {
                            "identity": {
                                "family": "Item",
                                "key": "oteryn:item.tibia.i99",
                                "revision": "definition-r1",
                            }
                        }
                    }
                ]
            },
        )
        self.transition = {
            "key": "npc_1",
            "owner": "npc",
            "from": {"op": "==", "value": 3, "exact": True},
            "to": 7,
            "servers": ["canary"],
            "requested_by": {
                "npc": "canary:npc/alice",
                "keywords": ["yes"],
                "topics": [2],
            },
        }
        self.quest = {
            "identity": {"key": "canary:quest/test", "revision": "source-r1"},
            "display_name": "Test",
            "start": None,
            "missions": [
                {
                    "key": "step",
                    "progress": "canary:quest-progress/test",
                    "transitions": [self.transition],
                }
            ],
            "gates": [
                {"family": "Gate", "key": "canary:gate/test", "revision": "source-r1"}
            ],
            "claims": [
                {
                    "family": "RewardClaim",
                    "key": "canary:claim/test",
                    "revision": "source-r1",
                }
            ],
        }
        self.write(q.SAMPLES + "questlog/quests.json", {"quests": [self.quest]})
        self.write(
            q.SAMPLES + "questlog/progress.json",
            {
                "progress": [
                    {
                        "key": "canary:quest-progress/test",
                        "missions": ["canary:quest/test#step"],
                        "transitions": [
                            {
                                "key": "npc_1",
                                "script": "npc/alice.lua",
                                "sources": {"canary": {"line": 13}},
                            }
                        ],
                    }
                ]
            },
        )
        self.write(
            q.SAMPLES + "questlog/manifest.json",
            {
                "sources": [
                    {
                        "repository": "opentibiabr/canary",
                        "revision": q.CANARY_QUEST_REVISION,
                    }
                ]
            },
        )
        self.write(
            q.SAMPLES + "doors/gates.json",
            {
                "gates": [
                    {
                        "identity": {"key": "canary:gate/test"},
                        "condition": {"kind": "quest_progress", "op": ">=", "value": 7},
                    }
                ]
            },
        )
        self.write(
            q.SAMPLES + "chests/claims.json",
            {
                "claims": [
                    {
                        "identity": {"key": "canary:claim/test"},
                        "reward": {"items": [{"item": source_item(), "count": 1}]},
                    }
                ]
            },
        )

    def write(self, relative, data):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(q.canonical(data))

    def test_stage_preserves_full_transition_track_gate_claim_and_custody(self):
        result = q.stage(self.root)
        assignment = result["records"][0]["source_assignments"][0]
        self.assertEqual(assignment["transition"], self.transition)
        self.assertEqual(assignment["source_track"], "canary:quest-progress/test")
        self.assertEqual(result["source_gates"][0]["record"]["condition"]["value"], 7)
        self.assertEqual(
            result["source_claims"][0]["record"]["reward"]["items"][0]["count"], 1
        )
        provenance = assignment["evidence"]
        self.assertEqual(
            provenance["json_pointer"], "/quests/0/missions/0/transitions/0"
        )
        self.assertEqual(
            provenance["sha256"],
            q.digest((self.root / provenance["path"]).read_bytes()),
        )
        self.assertFalse(result["runtime_eligible"])
        self.assertEqual(result["native_quest_declarations"], [])
        field = result["records"][0]["candidate_field"]
        self.assertEqual(field["field_path"], "oteryn:source.npc.quest_bindings")
        self.assertFalse(json.loads(field["value"]["value"])["runtime_eligible"])

    def test_no_suffix_or_narrative_identity_guessing(self):
        self.quest["missions"][0]["transitions"][0]["requested_by"]["npc"] = (
            "canary:npc/alice_functions"
        )
        self.write(q.SAMPLES + "questlog/quests.json", {"quests": [self.quest]})
        result = q.stage(self.root)
        self.assertEqual(result["counts"]["held_identity_transitions"], 1)
        self.assertEqual(result["records"], [])
        self.assertEqual(
            result["held"][0]["identity_hold"]["reason"], "NO_PROVEN_NPC_SOURCE_BINDING"
        )

    def test_conflicting_source_identity_is_held(self):
        self.write(
            "content/npcs/definitions/npcs-00000-00499.json",
            {"records": [npc(), npc("oteryn:npc.other", "alice")]},
        )
        result = q.stage(self.root)
        self.assertEqual(
            result["held"][0]["identity_hold"]["reason"], "AMBIGUOUS_NPC_SOURCE_BINDING"
        )
        self.assertEqual(result["counts"]["associated_transitions"], 0)

    def test_source_binding_cannot_target_another_npc(self):
        row = npc()
        row["source_bindings"][0]["target"]["key"] = "oteryn:npc.other"
        self.write("content/npcs/definitions/npcs-00000-00499.json", {"records": [row]})
        with self.assertRaises(q.StageError):
            q.stage(self.root)

    def test_computed_effect_is_preserved_and_held(self):
        transition = self.quest["missions"][0]["transitions"][0]
        del transition["to"]
        transition["computed"] = "unknown legacy expression"
        self.write(q.SAMPLES + "questlog/quests.json", {"quests": [self.quest]})
        assignment = q.stage(self.root)["records"][0]["source_assignments"][0]
        self.assertEqual(
            assignment["transition"]["computed"], "unknown legacy expression"
        )
        self.assertIn(
            "COMPUTED_TRANSITION_EFFECT_NOT_SUPPORTED", assignment["hold_reasons"]
        )

    def test_item_numeric_id_does_not_become_native_id(self):
        result = q.stage(self.root)
        self.assertEqual(
            result["item_dependencies"][0]["state"], "SOURCE_ITEM_BINDING_MISSING"
        )
        self.assertIsNone(result["item_dependencies"][0]["native_reference"])

    def test_item_binding_requires_exact_source_pin_and_active_target(self):
        target = {
            "family": "Item",
            "key": "oteryn:item.tibia.i99",
            "revision": "definition-r1",
        }
        binding = {
            "source_key": "oteryn:source.canary",
            "identity_namespace": "ots/item_server_id",
            "external_id": "3233",
            "disposition": "EXACT",
            "source_revision": "wrong",
            "target": target,
        }
        registry = {target["key"]: target}
        revisions = {"canary": q.CANARY_QUEST_REVISION}
        self.assertIsNone(
            q.join_item(source_item(), [binding], registry, revisions)[
                "native_reference"
            ]
        )
        binding["source_revision"] = q.CANARY_QUEST_REVISION
        self.assertEqual(
            q.join_item(source_item(), [binding], registry, revisions)[
                "native_reference"
            ],
            target,
        )
        self.assertIsNone(
            q.join_item(source_item(), [binding], {}, revisions)["native_reference"]
        )

    def test_helper_proof_requires_pin_digest_definition_and_actual_relation(self):
        caller = (
            b"local function callback()\n ParseAliceSay(npc, player, message)\nend\n"
        )
        helper = b"function ParseAliceSay(npc, player, message)\nend\n"
        (self.root / "alice.lua").write_bytes(caller)
        (self.root / "alice_functions.lua").write_bytes(helper)
        specs = {
            "canary:npc/alice_functions": {
                "target": "canary:npc/alice",
                "symbol": "ParseAliceSay",
                "mode": "called_helper",
                "caller_sha256": q.digest(caller),
                "helper_sha256": q.digest(helper),
            }
        }
        revisions = {"canary": q.CANARY_QUEST_REVISION}
        proof = q.helper_proof(
            "canary:npc/alice_functions", self.root, revisions, specs
        )
        self.assertEqual(proof["target_source_npc"], "canary:npc/alice")
        self.assertEqual(proof["caller"]["line"], 2)
        self.assertFalse(proof["execution_proven"])
        self.assertIsNone(
            q.helper_proof(
                "canary:npc/alice_functions", self.root, {"canary": "wrong"}, specs
            )
        )
        (self.root / "alice.lua").write_bytes(caller + b"-- changed\n")
        self.assertIsNone(
            q.helper_proof("canary:npc/alice_functions", self.root, revisions, specs)
        )

    def test_loading_helper_proves_ownership_without_call_authority(self):
        caller = b'dofile(DATA_DIRECTORY .. "/npc/alice_functions.lua")\n'
        helper = b"function ParseAliceSay(npc, player, message)\nend\n"
        (self.root / "alice.lua").write_bytes(caller)
        (self.root / "alice_functions.lua").write_bytes(helper)
        specs = {
            "canary:npc/alice_functions": {
                "target": "canary:npc/alice",
                "symbol": "ParseAliceSay",
                "mode": "loaded_helper",
                "caller_sha256": q.digest(caller),
                "helper_sha256": q.digest(helper),
            }
        }
        proof = q.helper_proof(
            "canary:npc/alice_functions",
            self.root,
            {"canary": q.CANARY_QUEST_REVISION},
            specs,
        )
        self.assertEqual(proof["relationship"], "loaded_helper")
        self.assertFalse(proof["execution_proven"])

    def test_repeated_stage_is_byte_identical(self):
        self.assertEqual(
            q.canonical(q.stage(self.root)), q.canonical(q.stage(self.root))
        )

    def test_source_xml_nodes_keep_exact_bytes_ranges_and_ignore_comments(self):
        node = b'<item fromid="3233" toid="3234" name="test > token"><attribute key="weight" value="100" /></item>'
        data = (
            b'<items>\n<!-- <item id="3233"/> -->\n'
            + node
            + b'\n<item id="3235" name="empty"/>\n</items>'
        )
        nodes = q.source_item_nodes(data)
        self.assertEqual(len(nodes[3233]), 1)
        proof = nodes[3233][0]
        self.assertEqual(proof["node_sha256"], q.digest(node))
        self.assertEqual(data[proof["byte_start"] : proof["byte_end_exclusive"]], node)
        self.assertEqual(nodes[3234][0]["node_sha256"], proof["node_sha256"])
        self.assertEqual(len(nodes[3235]), 1)

    def test_unqualified_source_capture_cannot_supply_item_map(self):
        for relative in ["canary/data/items/items.xml", "canary/src/items/items.cpp"]:
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"arbitrary caller bytes")
        with self.assertRaises(q.StageError):
            q.load_item_sources(self.root, {"canary": q.CANARY_QUEST_REVISION})
        self.assertEqual(
            q.load_item_sources(self.root, {"canary": "another revision"}), {}
        )

    def bridge_fixture(self, rows=None, membership=None):
        item_id = 3233
        target = {
            "family": "Item",
            "key": "oteryn:item.tibia.i3233",
            "revision": "definition-r1",
        }
        xml_node = b'<item id="3233" name="tear" />'
        proofs = {
            "canary": {
                "nodes": q.source_item_nodes(b"<items>" + xml_node + b"</items>"),
                "source": q.ITEM_SOURCES["canary"],
            }
        }
        row = {
            "key": "oteryn:item.retired.tear",
            "version": 1,
            "state": "ALIAS",
            "target": target["key"],
            "evidence": {"source_item_id": item_id},
        }
        aliases = {
            "key_rule": "OTERYN_TIBIA_ID_KEY_RULE_V1",
            "entries": rows if rows is not None else [row],
        }
        return q.bridge_item(
            source_item(),
            proofs,
            aliases,
            {target["key"]: target},
            {item_id} if membership is None else membership,
        ), row

    def test_identity_bridge_requires_protected_alias_and_appearance(self):
        result, row = self.bridge_fixture()
        self.assertEqual(result["state"], "BOUND")
        self.assertEqual(result["identity_bridge"]["source_client_object_id"], 3233)
        self.assertTrue(result["identity_bridge"]["xml"]["node_sha256"])
        self.assertEqual(
            self.bridge_fixture(membership=set())[0]["state"],
            "NATIVE_ITEM_OR_PROTECTED_APPEARANCE_MISSING",
        )
        self.assertEqual(
            self.bridge_fixture(rows=[])[0]["state"],
            "PROTECTED_ITEM_ALIAS_TARGET_MISSING_OR_AMBIGUOUS",
        )
        retired = dict(row, version=2, state="RETIRED_WITHOUT_SUCCESSOR")
        retired.pop("target")
        self.assertIsNone(
            self.bridge_fixture(rows=[row, retired])[0]["native_reference"]
        )

    def test_source_fluid_enum_is_not_an_object_identity(self):
        proof = {"canary": {"nodes": {}, "source": q.ITEM_SOURCES["canary"]}}
        result = q.bridge_item(
            source_item("canary:item/7"), proof, {"entries": []}, {}, set()
        )
        self.assertEqual(result["state"], "SOURCE_ID_IS_FLUID_ENUM_NOT_OBJECT")

    def test_duplicate_xml_identity_and_wrong_namespace_stay_held(self):
        proof = {
            "canary": {
                "nodes": q.source_item_nodes(
                    b'<items><item id="3233"/><item id="3233"/></items>'
                ),
                "source": q.ITEM_SOURCES["canary"],
            }
        }
        result = q.bridge_item(source_item(), proof, {"entries": []}, {}, set())
        self.assertEqual(result["state"], "SOURCE_XML_ITEM_AMBIGUOUS")
        result = q.bridge_item(
            source_item("crystalserver:item/3233"), proof, {"entries": []}, {}, set()
        )
        self.assertEqual(
            result["state"], "PINNED_ITEM_IDENTITY_LOADER_EVIDENCE_MISSING"
        )

    def test_real_samples_partition_every_npc_transition_without_alias_guess(self):
        result = q.stage(q.ROOT)
        self.assertEqual(
            result["counts"]["source_npc_transitions"],
            result["counts"]["associated_transitions"]
            + result["counts"]["held_identity_transitions"],
        )
        held_refs = {t["transition"]["requested_by"]["npc"] for t in result["held"]}
        self.assertEqual(
            held_refs, {"canary:npc/alesar_functions", "canary:npc/tereban_functions"}
        )
        self.assertTrue(
            all(not d["native_reference"] for d in result["item_dependencies"])
        )
        self.assertTrue(
            all(
                t["transition"]
                for r in result["records"]
                for t in r["source_assignments"]
            )
        )


if __name__ == "__main__":
    unittest.main()
