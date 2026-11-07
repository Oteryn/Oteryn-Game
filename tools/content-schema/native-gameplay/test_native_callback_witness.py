"""Source-rule witnesses cannot bless a changed callback merely by rebinding a hash."""
import copy
import json
from pathlib import Path
import unittest

import build_native_callback_witness as tool


ROOT = Path(__file__).resolve().parents[3]


class NativeCallbackWitnessTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.documents = tool.read_documents(ROOT)
        cls.native = json.loads((ROOT / tool.NATIVE).read_bytes())

    def test_current_source_rules_bind_every_native_encounter_callback(self):
        witness = tool.build(ROOT, self.documents, self.native)
        self.assertEqual(36, len(witness["entries"]))
        self.assertEqual(self.native["source_definitions_sha256"],
                         witness["source_definitions_sha256"])
        entries = {row["ability"]["key"] for row in witness["entries"]}
        self.assertIn("oteryn:ability.creature.cerebellum.field-fill-heal_brain_head", entries)
        self.assertIn("oteryn:ability.spell.gaz_haragoth_summon", entries)
        self.assertIn("oteryn:ability.spell.zamulosh_tp", entries)

    def test_transitive_event_children_retain_full_bodies_without_flattening_unrelated_spells(self):
        result = tool.build(ROOT, self.documents, self.native)
        names = {"baby_dragon", "the_enraged_thorn_knight", "the_shielded_thorn_knight", "thorn_steed", "eradicator2",
                 "shadow_tentacle", "possessed_tree", "unbound_blightwalker", "unbound_demon", "unbound_demon_outcast",
                 "unbound_defiler", "dragonking_zyrtarch", "soul_of_dragonking_zyrtarch"}
        records = result.get("event_creature_records", [])
        self.assertEqual({"oteryn:creature."+name for name in names}, {row["profile"]["target"]["key"] for row in records})
        retained = {row["target"]["key"]: row for row in result["profiles"]}
        for record in records:
            self.assertEqual(next(row for row in self.native["records"] if row["profile"]["target"]==record["profile"]["target"]), record)
            self.assertEqual(retained[record["profile"]["target"]["key"]], record["profile"])
        self.assertLessEqual(len(result["profiles"]),512)
        self.assertEqual(36,len(result["entries"]),"source events cannot manufacture Ability slots")
        self.assertTrue({"HealthForgotten","RuptureHeal","SoulcatcherSummon","PossessedTree","CorruptedSoulVortexStep"}.issubset({row["name"] for row in result["source_events"]}))

    def test_gaz_attached_event_closure_uses_the_existing_witness_without_fake_slots(self):
        result = tool.build(ROOT, self.documents, self.native)
        events = {row["name"] for row in result["source_events"]}
        self.assertTrue({"GazHaragothHeal", "MinionGazDeath", "GazNightmareVortexStep"}.issubset(events),
                        "actual Gaz/minion declarations and the created Item's step rule need pinned evidence")
        self.assertEqual(36, len(result["entries"]), "source creature events are not fabricated Ability slots")

    def test_outburst_event_closure_retains_actual_bodies_without_inventing_ability_slots(self):
        result = tool.build(ROOT, self.documents, self.native)
        retained = {row["target"]["key"]: row for row in result["profiles"]}
        for name in ("outburst", "spark_of_destruction"):
            record = next(row for row in self.native["records"]
                          if row["profile"]["target"]["key"] == "oteryn:creature." + name)
            for actual in (record["profile"], record["behavior"]):
                self.assertEqual(retained.get(actual["target"]["key"]), actual,
                                 "actual Outburst event closure must be retained")
        self.assertTrue({"OutburstCharge", "ChargingOutDeath"}.issubset(
                         {event["name"] for event in result.get("source_events", [])}))
        self.assertEqual(36, len(result["entries"]), "events are not fabricated Ability slots")

    def test_related_creature_events_keep_original_slots_and_existing_profiles(self):
        result = tool.build(ROOT, self.documents, self.native)
        before = json.loads((ROOT / tool.OUTPUT).read_bytes())
        actual = {row['target']['key']: row for row in result['profiles']}
        for row in before['profiles']:
            self.assertEqual(actual[row['target']['key']], row)
        self.assertEqual(result['entries'], before['entries'])
        expected = {'HeartMinionDeath','DisruptionTransform','ChargedDisruptionTransform',
                    'HeartVortexHunger','AnomalyVortexStep','BoneCapsule','HeartBossDeath'}
        self.assertTrue(expected.issubset({event['name'] for event in result['source_events']}))
        for name in ('charged_disruption','overcharged_disruption','bone_capsule','damage_resonance'):
            record = next(row for row in self.native['records'] if row['profile']['target']['key'] == 'oteryn:creature.' + name)
            self.assertEqual(actual.get(record['profile']['target']['key']),record['profile'])
            self.assertEqual(actual.get(record['behavior']['target']['key']),record['behavior'])

    def test_related_event_body_drift_cannot_be_blessed_by_current_root(self):
        for name in ('charged_disruption','bone_capsule','frenzy'):
            native = copy.deepcopy(self.native)
            record = next(row for row in native['records'] if row['profile']['target']['key'] == 'oteryn:creature.' + name)
            movement=record['behavior']['data']['profile']['movement']
            movement['push_items']=not movement['push_items']
            with self.subTest(name=name), self.assertRaisesRegex(ValueError,'event participant body changed'):
                tool.build(ROOT,self.documents,native)

    def test_changed_outburst_event_body_cannot_be_blessed_by_existing_root_binding(self):
        native = copy.deepcopy(self.native)
        record = next(row for row in native["records"]
                      if row["profile"]["target"]["key"] == "oteryn:creature.outburst")
        record["profile"]["data"]["profile"]["health"] += 1
        with self.assertRaisesRegex(ValueError, "Outburst event body differs"):
            tool.build(ROOT, self.documents, native)

    def test_every_callback_retains_its_actual_canonical_encounter_body(self):
        result=tool.build(ROOT,self.documents,self.native)
        retained={profile["target"]["key"]:profile for profile in result["profiles"]}
        actual={profile["target"]["key"]:profile for profile in self.documents["declarations"]["authoring_profiles"]}
        for entry in result["entries"]:
            key=entry["encounter"]["key"]
            self.assertTrue(key in retained,"native constructors must retain actual Encounter: "+key)
            self.assertEqual(retained[key],actual[key])

    def test_rebinding_root_digest_does_not_bless_changed_source_encounter(self):
        documents = dict(self.documents)
        declarations = dict(documents["declarations"])
        declarations["authoring_profiles"] = list(declarations["authoring_profiles"])
        old = next(row for row in declarations["authoring_profiles"]
                   if row["target"]["key"] == "oteryn:encounter.field_fill_cerebellum")
        changed = copy.deepcopy(old)
        changed["data"]["profile"]["details"]["rules"][0]["actions"][0]["amount"]["max"] = 900
        declarations["authoring_profiles"][declarations["authoring_profiles"].index(old)] = changed
        documents["declarations"] = declarations
        native = dict(self.native)
        native["source_definitions_sha256"] = tool.source_digest(documents)
        with self.assertRaisesRegex(ValueError, "source rule differs"):
            tool.build(ROOT, documents, native)

    def test_changed_active_parent_body_is_not_relabelled_as_the_same_callback(self):
        native = dict(self.native)
        native["records"] = list(native["records"])
        old = next(row for row in native["records"]
                   if row["profile"]["target"]["key"] == "oteryn:creature.cerebellum")
        changed = copy.deepcopy(old)
        parent = next(row for row in changed["attack_definitions"]["profiles"]
                      if row["target"]["key"] == "oteryn:ability.creature.cerebellum.field-fill-heal_brain_head")
        parent["data"]["profile"]["details"]["encounter"]["key"] = "oteryn:encounter.zamulosh"
        native["records"][native["records"].index(old)] = changed
        with self.assertRaisesRegex(ValueError, "native callback body differs"):
            tool.build(ROOT, self.documents, native)

    def test_rebinding_both_roots_does_not_change_reviewed_spawn_chance(self):
        documents = dict(self.documents)
        declarations = dict(documents["declarations"])
        declarations["authoring_profiles"] = list(declarations["authoring_profiles"])
        old = next(row for row in declarations["authoring_profiles"]
                   if row["target"]["key"] == "oteryn:behavior.creature.gaz_haragoth")
        changed = copy.deepcopy(old)
        changed["data"]["profile"]["attacks"][11]["chance_ppm"] = 500_000
        declarations["authoring_profiles"][declarations["authoring_profiles"].index(old)] = changed
        documents["declarations"] = declarations
        native = dict(self.native)
        native["records"] = list(native["records"])
        native["source_definitions_sha256"] = tool.source_digest(documents)
        old_native = next(row for row in native["records"]
                          if row["profile"]["target"]["key"] == "oteryn:creature.gaz_haragoth")
        changed_native = copy.deepcopy(old_native)
        changed_native["behavior"] = changed
        closure = changed_native["attack_definitions"]["profiles"]
        old_closure = next(row for row in closure if row["target"] == changed["target"])
        closure[closure.index(old_closure)] = changed
        native["records"][native["records"].index(old_native)] = changed_native
        with self.assertRaisesRegex(ValueError, "source rule differs.*schedule"):
            tool.build(ROOT, documents, native)

    def test_spawn_child_closure_retains_actual_child_policy_and_accepted_health(self):
        result = tool.build(ROOT, self.documents, self.native)
        profiles = {row["target"]["key"]: row for row in result["profiles"]}
        for name, health in (("corrupted_soul", 1100), ("lost_time", 4000),
                             ("thorn_minion", 3600), ("time_waster", 1000)):
            key = "oteryn:creature." + name
            self.assertTrue(key in profiles, key)
            self.assertEqual(profiles[key]["data"]["profile"]["health"], health)

    def test_spawn_child_health_cannot_change_by_rebinding_root_metadata(self):
        documents = copy.deepcopy(self.documents)
        native = copy.deepcopy(self.native)
        key = "oteryn:creature.corrupted_soul"
        canonical = next(row for row in documents["declarations"]["authoring_profiles"]
                         if row["target"]["key"] == key)
        canonical["data"]["profile"]["health"] = 999999
        actual = next(row for row in native["records"] if row["profile"]["target"]["key"] == key)
        actual["profile"]["data"]["profile"]["health"] = 999999
        native["source_definitions_sha256"] = tool.source_digest(documents)
        with self.assertRaisesRegex(ValueError, "spawn child.*health"):
            tool.build(ROOT, documents, native)


if __name__ == "__main__":
    unittest.main()
