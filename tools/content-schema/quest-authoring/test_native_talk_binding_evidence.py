"""M01a: a runtime flag is evidence only when the server registers that callback."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from native_talk_binding_evidence import verify_talk_binding
from quest_native_binding_preflight import verify_stage_runtime


class NativeTalkBindingEvidenceTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.recipe = {"stages": [{"key": "s1", "kind": "talk", "count": 1}]}
        self.definition = {"missing_data": [], "identity": {"key": "oteryn:quest.example", "revision": "r1"},
                           "oteryn_recipe": {"profile": "chosen_source_completion_v1",
                           "chosen_data_complete": True, "payload": {"recipe": self.recipe}}}
        self.definition["oteryn_recipe"]["payload"]["identity"] = self.definition["identity"].copy()
        self.path = "content/quests/authored-bindings/m01a-example.json"
        self.reference = {"kind": "npc_dialogue", "document": self.path,
                          "stage": "s1", "callback": "oteryn:quest-reward/example"}
        self.document = {"schema": 1, "quests": [{"quest": "oteryn:quest.example",
            "sources": ["M01a engine test fixture"],
            "tracks": [{"key": "oteryn:quest-progress/example/s1", "initial": 0,
                        "minimum": 0, "maximum": 1}],
            "recipe_sha256": hashlib.sha256(json.dumps(self.recipe, sort_keys=True,
                separators=(",", ":")).encode()).hexdigest(),
            "stages": [{"key": "s1", "completes": False,
                        "trigger": {"kind": "npc_dialogue", "callback": {
                            "quest": "oteryn:quest.example", "key": self.reference["callback"],
                            "npc": "oteryn:npc.example", "keywords": ["mission"],
                            "topic": 0, "next_topic": 0, "take": [],
                            "reply": ["Stage accepted."], "sources": ["M01a test fixture"],
                            "storage": [{"track": "oteryn:quest-progress/example/s1",
                                         "minimum": 0, "maximum": 0, "set": 1}]}}}]}]}
        self.write()

    def write(self):
        path = self.root / self.path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(self.document))

    def test_registered_exact_callback_is_accepted(self):
        verify_talk_binding(self.root, self.definition, "s1", self.reference)

    def test_flags_cannot_authorize_forged_or_stale_callbacks(self):
        for mutate in [
            lambda d: d["quests"][0].update(recipe_sha256="f" * 64),
            lambda d: d["quests"][0].update(quest="oteryn:quest.foreign"),
            lambda d: d["quests"][0]["stages"][0]["trigger"].update(kind="carried_use"),
            lambda d: d["quests"][0]["stages"][0]["trigger"]["callback"]["storage"][0].update(track="oteryn:quest-progress/foreign"),
            lambda d: d["quests"][0]["stages"][0]["trigger"]["callback"].update(quest="oteryn:quest.foreign"),
        ]:
            with self.subTest(mutation=mutate):
                original = copy.deepcopy(self.document)
                mutate(self.document)
                self.write()
                with self.assertRaises(ValueError):
                    verify_talk_binding(self.root, self.definition, "s1", self.reference)
                self.document = original

    def test_registry_path_cannot_escape_registered_directory(self):
        for path in ["../outside.json", "content/quests/elsewhere.json",
                     "content/quests/authored-bindings/nested/other.json"]:
            with self.subTest(path=path), self.assertRaises(ValueError):
                verify_talk_binding(self.root, self.definition, "s1", dict(self.reference, document=path))

    def test_runtime_stage_requires_the_exact_registered_talk_callback(self):
        stage = {"stage_key": "s1", "runtime_enabled": True,
                 "event_identity_associations": {"kind": "talk"},
                 "native_dispatch_binding": self.reference}
        verify_stage_runtime(self.root, self.definition, stage)
        for change in [{"native_dispatch_binding": None},
                       {"runtime_enabled": False},
                       {"event_identity_associations": {"kind": "use"}}]:
            with self.subTest(change=change), self.assertRaises(ValueError):
                verify_stage_runtime(self.root, self.definition, dict(stage, **change))

    def test_group_evidence_cannot_hide_an_extra_callback(self):
        alternative = copy.deepcopy(self.document["quests"][0]["stages"][0]["trigger"]["callback"])
        alternative["key"] = "oteryn:quest-reward/second"
        alternative["npc"] = "oteryn:npc.second"
        self.document["quests"][0]["stages"][0]["trigger"]["alternatives"] = [alternative]
        self.write()
        with self.assertRaises(ValueError):
            verify_talk_binding(self.root, self.definition, "s1", self.reference)
        reference = dict(self.reference)
        first = reference.pop("callback")
        reference["callbacks"] = [first, alternative["key"]]
        self.assertEqual(2, len(verify_talk_binding(self.root, self.definition, "s1", reference)))
        self.document["quests"][0]["stages"][0]["completes"] = True
        self.write()
        with self.assertRaises(ValueError):
            verify_talk_binding(self.root, self.definition, "s1", reference)

    def test_original_source_hold_is_retained_but_chosen_data_hold_refuses(self):
        self.definition["missing_data"] = [{"code": "reported_source_gap"}]
        verify_talk_binding(self.root, self.definition, "s1", self.reference)
        self.definition["missing_data"].append({"code": "chosen_recipe_data_missing"})
        with self.assertRaises(ValueError):
            verify_talk_binding(self.root, self.definition, "s1", self.reference)

    def test_native_counter_operations_use_the_existing_callback_schema(self):
        callback = self.document["quests"][0]["stages"][0]["trigger"]["callback"]
        callback["state_ops"] = [{"track": "oteryn:quest-progress/example/counter",
                                 "from": {"kind": "between", "minimum": 0, "maximum": 1},
                                 "effect": {"kind": "add", "value": 1}}]
        self.document["quests"][0]["tracks"].append({"key": callback["state_ops"][0]["track"],
            "initial": 0, "minimum": 0, "maximum": 2})
        self.write()
        verify_talk_binding(self.root, self.definition, "s1", self.reference)
        callback["state_ops"][0]["effect"]["kind"] = "multiply"
        self.write()
        with self.assertRaises(ValueError):
            verify_talk_binding(self.root, self.definition, "s1", self.reference)

    def test_runtime_flags_refuse_operations_the_engine_cannot_admit(self):
        quest = self.document['quests'][0]
        callback = quest['stages'][0]['trigger']['callback']
        track = 'oteryn:quest-progress/example/counter'
        quest['tracks'].append({'key': track, 'initial': 0, 'minimum': 0, 'maximum': 2})
        stage = {'stage_key': 's1', 'runtime_enabled': True,
                 'event_identity_associations': {'kind': 'talk'},
                 'native_dispatch_binding': self.reference}
        for condition, effect in [
            ({'kind': 'between', 'minimum': 0, 'maximum': 2}, {'kind': 'add', 'value': 1}),
            ({'kind': 'any'}, {'kind': 'add', 'value': 1}),
            ({'kind': 'any'}, {'kind': 'set_now'}),
            ({'kind': 'elapsed_at_least', 'seconds': 1.0}, {'kind': 'add', 'value': 0}),
            ({'kind': 'between', 'minimum': 2, 'maximum': 0}, {'kind': 'add', 'value': 0}),
        ]:
            callback['state_ops'] = [{'track': track, 'from': condition, 'effect': effect}]
            self.write()
            with self.subTest(condition=condition, effect=effect), self.assertRaises(ValueError):
                verify_stage_runtime(self.root, self.definition, stage)
        quest['tracks'][-1].update(minimum=-(2 ** 63), maximum=2 ** 63 - 1)
        callback['state_ops'] = [{'track': track,
            'from': {'kind': 'between', 'minimum': -(2 ** 63), 'maximum': -(2 ** 63)},
            'effect': {'kind': 'add', 'value': 2 ** 63}}]
        self.write()
        with self.assertRaises(ValueError):
            verify_stage_runtime(self.root, self.definition, stage)
        quest['tracks'][-1].update(minimum=0, maximum=2)
        callback['state_ops'] = [{'track': track,
            'from': {'kind': 'between', 'minimum': 0, 'maximum': 1},
            'effect': {'kind': 'add', 'value': 1.0}}]
        self.write()
        with self.assertRaises(ValueError):
            verify_stage_runtime(self.root, self.definition, stage)
        callback['state_ops'][0]['effect']['value'] = 1
        self.write()
        verify_stage_runtime(self.root, self.definition, stage)
        callback['state_ops'] = []
        for flag in ['increment', 'at_least']:
            callback['storage'][0][flag] = True
            self.write()
            with self.subTest(flag=flag), self.assertRaises(ValueError):
                verify_stage_runtime(self.root, self.definition, stage)
            del callback['storage'][0][flag]

    def test_registered_stage_is_activated_by_the_producer_and_unknown_stage_refuses(self):
        from native_talk_binding_evidence import apply_native_talk_bindings
        plan = {"records": [{"quest": "oteryn:quest.example", "runtime_enabled": False,
            "stages": [{"stage_key": "s1", "runtime_enabled": False, "native_dispatch_binding": None,
                        "event_identity_associations": {"kind": "talk"}}]}]}
        paths = apply_native_talk_bindings(self.root, plan, {"oteryn:quest.example": self.definition})
        self.assertEqual([self.path], paths)
        stage = plan["records"][0]["stages"][0]
        self.assertTrue(stage["runtime_enabled"])
        self.assertFalse(plan["records"][0]["runtime_enabled"])
        self.assertEqual(self.reference, stage["native_dispatch_binding"])
        verify_stage_runtime(self.root, self.definition, stage)
        plan["records"][0]["stages"] = []
        with self.assertRaises(ValueError):
            apply_native_talk_bindings(self.root, plan, {"oteryn:quest.example": self.definition})
