import copy
import importlib.util
import json
import os
import pathlib
import tempfile
import unittest

import class_profile_bridge as bridge

ROOT = pathlib.Path(__file__).parent
R7 = pathlib.Path(
    os.environ.get("NPC_BRIDGE_SOURCE_BASE", str(ROOT / "source-fixtures"))
)
NATIVE_INPUT = pathlib.Path(
    os.environ.get("NPC_BRIDGE_NATIVE_INPUT", str(ROOT / "native-class-input.json"))
)
HAS_INPUTS = (R7 / "source-programs-r7.json").exists() and NATIVE_INPUT.exists()
HAS_PARSER = importlib.util.find_spec("luaparser") is not None


class CustodyTests(unittest.TestCase):
    def test_source_tamper_fails_before_parse(self):
        with tempfile.TemporaryDirectory() as directory:
            base = pathlib.Path(directory)
            raw = b"local marker = 1\n"
            (base / "source.lua").write_bytes(raw + b"-- changed\n")
            capture = {
                "capture": "source.lua",
                "bytes": len(raw),
                "sha256": bridge.sha(raw),
                "git_blob_sha1": bridge.hashlib.sha1(
                    b"blob " + str(len(raw)).encode() + b"\0" + raw
                ).hexdigest(),
            }
            with self.assertRaises(bridge.BridgeError):
                bridge.verify(base, capture)

    def test_generated_services_require_existing_native_reference(self):
        npc = {"services": [{"family": "Service", "key": "missing", "revision": "r1"}]}
        program = {"source_program": {"definition": {"display_name": "Example"}}}
        with self.assertRaises(bridge.BridgeError):
            bridge.generate_profile(npc, program, {})


@unittest.skipUnless(
    HAS_PARSER and HAS_INPUTS, "dedicated parser and verified research inputs required"
)
class AstBridgeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.summary = json.loads(
            (R7 / "dialogue-completion-summary-r7.json").read_bytes()
        )
        cls.qualification = json.loads(
            (R7 / "program-qualification-r7.json").read_bytes()
        )
        cls.programs = json.loads((R7 / "source-programs-r7.json").read_bytes())
        cls.native = json.loads(NATIVE_INPUT.read_bytes())
        cls.packet = bridge.bridge(
            cls.summary, cls.qualification, cls.programs, R7, cls.native
        )

    def test_regeneration_is_deterministic_and_allocates_no_identity(self):
        again = bridge.bridge(
            self.summary, self.qualification, self.programs, R7, self.native
        )
        self.assertEqual(bridge.canonical(self.packet), bridge.canonical(again))
        self.assertEqual(self.packet["counts"]["source_class_records"], 73)
        self.assertEqual(self.packet["counts"]["generated_profiles"], 62)
        self.assertEqual(self.packet["counts"]["native_quest_ids_allocated"], 0)
        unadmitted = [
            r for r in self.packet["records"] if not r["native_target_admitted"]
        ]
        self.assertEqual(len(unadmitted), 2)
        self.assertTrue(all(r["npc_key"] is None for r in unadmitted))

    def test_structured_program_tamper_is_rejected(self):
        programs = copy.deepcopy(self.programs)
        # Select a source used by the missing classes rather than an unrelated recovered NPC.
        name = self.summary["existing_176_dispositions"][
            "SOURCE_DEFAULT_ONLY_NO_EXPLICIT_STATIC_DIALOGUE"
        ]["names"][0]
        q = next(q for q in self.qualification["records"] if q["name"] == name)
        target = q["accepted_sources"][0]["source_program_sha256"]
        row = next(
            r for r in programs["records"] if r["source_program_sha256"] == target
        )
        row["source_program"]["source_callback_registrations"] = [{"fabricated": True}]
        with self.assertRaises(bridge.BridgeError):
            bridge.bridge(self.summary, self.qualification, programs, R7, self.native)

    def test_every_ast_span_matches_actual_source_bytes(self):
        def visit(value, raw):
            if isinstance(value, dict):
                span = value.get("source_span")
                if span is not None:
                    actual = raw[span["byte_start"] : span["byte_end_exclusive"]]
                    self.assertEqual(bridge.sha(actual), span["sha256"])
                for child in value.values():
                    visit(child, raw)
            elif isinstance(value, list):
                for child in value:
                    visit(child, raw)

        for record in self.packet["records"]:
            for source in record["source_programs"]:
                raw = bridge.verify(R7, source["source_capture"])
                visit(source["typed_callback_ir"], raw)

    def test_dynamic_callback_speech_and_guards_are_not_flattened(self):
        row = next(r for r in self.packet["records"] if r["name"] == "Inkaef")
        callback = row["source_programs"][0]["typed_callback_ir"][0]
        self.assertGreater(callback["guard_count"], 0)
        self.assertGreater(callback["effect_or_opaque_call_count"], 0)
        dynamic = [s for s in callback["speech_observations"] if s["dynamic_text"]]
        self.assertGreaterEqual(len(dynamic), 2)
        self.assertTrue(all(s["static_parts"] is None for s in dynamic))
        self.assertFalse(callback["safe_const_callback_candidate"])
        self.assertIsNone(callback["native_guards"])
        self.assertIsNotNone(callback["callback_ir"])

    def test_unresolved_callbacks_get_no_fabricated_program(self):
        rows = [
            r for r in self.packet["records"] if r["name"].startswith("Kesar's Valet")
        ]
        self.assertEqual(len(rows), 2)
        for row in rows:
            cb = row["source_programs"][0]["typed_callback_ir"][0]
            self.assertEqual(cb["status"], "UNRESOLVED_SOURCE_SYMBOL")
            self.assertEqual(cb["function_symbol"], "creatureSayCallback")
            self.assertIsNone(cb["callback_ir"])
            self.assertFalse(cb["safe_const_callback_candidate"])

    def test_module_condition_initialization_is_retained(self):
        row = next(r for r in self.packet["records"] if r["name"] == "Demonguard")
        callback = row["source_programs"][0]["typed_callback_ir"][0]
        bindings = json.dumps(callback["referenced_module_bindings_ir"])
        effects = json.dumps(callback["module_initialization_calls_ir"])
        self.assertIn("CONDITION_FIRE", bindings)
        self.assertIn("addDamage", effects)
        self.assertIn("setParameter", effects)
        self.assertIn("If", json.dumps(callback["callback_ir"]))


@unittest.skipUnless(HAS_PARSER, "install dedicated requirements-dialogue-bridge.txt")
class AstParserTests(unittest.TestCase):
    def test_const_speech_detector_rejects_guards_effects_boolean_and_dynamic_values(
        self,
    ):
        def candidate(statement):
            source = (
                "local function reply(npc, creature)\n"
                + statement
                + "\nreturn true\nend\nnpcHandler:setCallback(CALLBACK_GREET,reply)"
            ).encode()
            return bridge.parse_source(source)[0]["safe_const_callback_candidate"]

        self.assertTrue(candidate('npcHandler:say("Hello", npc, creature)'))
        for code in [
            'if unlocked then npcHandler:say("Hello", npc, creature) end',
            'npcHandler:say("Hello", npc, creature); player:setStorageValue(key, 1)',
            'npcHandler:say(allowed and "Hello" or "Later", npc, creature)',
            'npcHandler:say("Hello" .. playerName, npc, creature)',
            'npcHandler:say("Hello", allowed and npc or other, creature)',
            'npcHandler:say("Hello", npc, creature); unknownOperation()',
        ]:
            self.assertFalse(candidate(code), code)


if __name__ == "__main__":
    unittest.main()
