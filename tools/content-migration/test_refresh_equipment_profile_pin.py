import unittest
import copy
import json
import tempfile
from pathlib import Path
from unittest.mock import patch

import refresh_equipment_profile_pin as refresh

from refresh_equipment_profile_pin import regenerate, declared_cohorts, source_speed_pin, TEST


def document(*ids):
    return {
        "records": [{"production_definition": {"production_key": str(i)}} for i in ids]
    }


class EquipmentPinTests(unittest.TestCase):
    def test_source_speed_pin_retains_unknown_pick_and_rejects_other_unknowns(self):
        packet = document('oteryn:item.tibia.i3456', 'oteryn:item.tibia.i123')
        packet['records'][0]['attributes'] = {}
        packet['records'][1]['attributes'] = {'speed_bonus': 0}
        source = TEST.read_text()
        fixed = source_speed_pin(source, packet)
        self.assertIn('("qengine-reward-item-profiles.json", 2)', fixed)
        self.assertIn('assert_eq!(actual.attributes.speed_bonus, None', fixed)
        self.assertIn('assert_eq!(actual.attributes.speed_bonus, Some(0)', fixed)
        self.assertEqual(source_speed_pin(fixed, packet), fixed)
        packet['records'][1]['attributes'] = {}
        with self.assertRaisesRegex(ValueError, 'unknown-speed pick'):
            source_speed_pin(source, packet)
        packet['records'][1]['attributes'] = {'speed_bonus': 1}
        with self.assertRaisesRegex(ValueError, 'unknown-speed pick'):
            source_speed_pin(source, packet)

    SOURCE = """const ORIGINAL_PROFILES: usize = 1;
const PROFILE_ADDITIONS: &[(&str, usize)] = &[("QNPCFX", 1)];
// cohorts add 1 + 1 keys
    assert_eq!(
        profiles.records.len(),
        ORIGINAL_PROFILES + PROFILE_ADDITIONS.iter().map(|(_, n)| n).sum::<usize>()
            + npc_additions.len()
    );
    assert_eq!(venison.attributes.speed_bonus, Some(0));
"""

    def generate(self, source=None, base=None):
        return regenerate(
            source or self.SOURCE,
            base or document(0, 1, 2, 3),
            document(4),
            document(0),
            document(1),
        )

    def test_source_identity_expression_is_counted_and_unknown_expressions_refuse(self):
        shared = [1791, 1948, 2822, 2823, 2848, 3593, 5102, 5103, 21982]
        with tempfile.TemporaryDirectory() as temporary:
            test = Path(temporary) / 'ability_tests.rs'
            test.with_name('questpc2_source_profile_cohort.rs').write_text(
                'const IDS: &[u16] = &[' + ','.join(map(str, shared + [100, 101])) + '];\n'
                'const SHARED_WITH_MAIN: &[u16] = &[' + ','.join(map(str, shared)) + '];\n')
            source = self.SOURCE.replace('[("QNPCFX", 1)];', '''[
    ("QNPCFX", 1),
    ("QUESTPC2-SOURCE-CLOSURE", questpc2_source_profile_cohort::IDS.len()
        - questpc2_source_profile_cohort::SHARED_WITH_MAIN.len()),
];''')
            with patch('refresh_equipment_profile_pin.TEST', test):
                self.assertEqual(declared_cohorts(source), {
                    'QNPCFX': 1, 'QUESTPC2-SOURCE-CLOSURE': 2})
                with self.assertRaisesRegex(ValueError, 'unresolved declared profile cohort'):
                    declared_cohorts(source.replace('QUESTPC2-SOURCE-CLOSURE', 'UNKNOWN-COHORT'))

    def test_registered_npc_is_counted_once_without_removing_identity_checks(self):
        fixed, count = self.generate()
        self.assertEqual(count, 5)
        self.assertNotIn("+ npc_additions.len()", fixed)
        self.assertIn("assert_eq!(venison.attributes.speed_bonus, Some(0))", fixed)
        self.assertEqual(self.generate(fixed), (fixed, 5))

    def test_missing_actual_profile_does_not_lower_the_declared_pin(self):
        with self.assertRaisesRegex(ValueError, "declared cohorts=5"):
            self.generate(base=document(0, 1, 2))

    def test_source_cohort_disagreement_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "NPC source proof"):
            self.generate(self.SOURCE.replace('("QNPCFX", 1)', '("QNPCFX", 2)'))

    def test_starting_loadout_overlay_is_included_in_the_source_union(self):
        source = self.SOURCE.replace(
            "ORIGINAL_PROFILES: usize = 1", "ORIGINAL_PROFILES: usize = 2"
        )
        _, count = regenerate(
            source,
            document(0, 1, 2, 3),
            document(4),
            document(0),
            document(1),
            document(5),
        )
        self.assertEqual(count, 6)

    def test_existing_registered_engine_is_counted_once(self):
        source = self.SOURCE.replace('("QNPCFX", 1)', '("QNPCFX", 1), ("QENGINE-REWARDS", 1)')
        source = source.replace('            + npc_additions.len()',
                                '            + (additions.len() - 1)\n            + npc_additions.len()')
        fixed, count = self.generate(source)
        self.assertEqual(count, 5)
        self.assertIn('(additions.len() - 1)', fixed)
        with self.assertRaisesRegex(ValueError, 'registered QENGINE cohort'):
            self.generate(source.replace('additions.len() - 1', 'additions.len() - 2'))
    def test_merged_census_uses_frozen_keys_and_inserts_only_declared_cohort(self):
        source = self.SOURCE.replace(
            '[("QNPCFX", 1)];',
            '[\n    ("QNPCFX", 1),\n    ("QUICKLOOT-1", 0),\n];',
        ).replace(
            "    assert_eq!(\n        profiles.records.len(),\n"
            "        ORIGINAL_PROFILES + PROFILE_ADDITIONS.iter().map(|(_, n)| n).sum::<usize>()\n"
            "            + npc_additions.len()\n    );",
            "    assert_eq!(profiles.records.len(), baseline + custody_keys.len());",
        )
        frozen = ({str(i) for i in range(7)}, 2)
        fixed, count = regenerate(source, document(0, 1, 2, 3, 4, 5), document(6),
                                  document(0), document(1), frozen=frozen)
        self.assertEqual(count, 7)
        self.assertIn('("QUESTPC8-MERGED", 2)', fixed)
        self.assertIn('baseline + custody_keys.len()', fixed)
        self.assertIn('assert_eq!(venison.attributes.speed_bonus, Some(0))', fixed)
        self.assertEqual(regenerate(fixed, document(0, 1, 2, 3, 4, 5), document(6),
                                    document(0), document(1), frozen=frozen), (fixed, 7))
        with self.assertRaisesRegex(ValueError, "frozen published union"):
            regenerate(source, document(0, 1, 2, 3, 4, 8), document(6),
                       document(0), document(1), frozen=frozen)

    def test_declared_new_recipe_profile_preserves_baseline_partition(self):
        source = self.SOURCE.replace(
            '[("QNPCFX", 1)];',
            '[\n    ("QNPCFX", 1),\n    ("QUICKLOOT-1", 0),\n];',
        ) + """
    let later_main_profiles = PROFILE_ADDITIONS.iter().filter(|(name, _)| matches!(*name, "QUESTPC8-MERGED")).count();
    assert_eq!(later_main_profiles, 999);
    let baseline = 3663;
    assert_eq!(baseline, 3663);
"""
        fixed, count = regenerate(source, document(0, 1, 2, 3, 4, 5), document(6),
                                  document(0), document(1), frozen=({str(i) for i in range(7)}, 2))
        self.assertEqual(count, 7)
        self.assertIn('assert_eq!(later_main_profiles, 3);', fixed)
        self.assertIn('assert_eq!(baseline, 3663);', fixed)

    def test_declared_pick_overlap_requires_both_source_owners(self):
        source = self.SOURCE.replace(
            'ORIGINAL_PROFILES: usize = 1', 'ORIGINAL_PROFILES: usize = 2')
        source += '\n- 1 // the pick already belongs to the base provider\n'
        pick = 'oteryn:item.tibia.i3456'
        _, count = regenerate(source, document(pick, 1, 2, 3),
                              document(4), document(pick), document(1))
        self.assertEqual(count, 5)
        with self.assertRaisesRegex(ValueError, 'pick overlap'):
            regenerate(source, document(0, 1, 2, 3),
                       document(4), document(pick), document(1))

    def test_duplicate_profile_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "duplicate production key"):
            self.generate(base=document(0, 0, 1, 2, 3))

    def test_shared_engine_exact_key_pin_is_refreshed_from_independent_source(self):
        source = self.SOURCE + (
            "    assert_eq!(added_keys.len(), 60);\n"
            '    ("qengine-reward-item-profiles.json", 60),\n'
            '        for source in records {\n'
            '        }\n    }\n'
            '    for id in [2949, 2950] {}\n'
        )
        pick = 'oteryn:item.tibia.i3456'
        qengine = document(pick)
        qengine['records'][0]['attributes'] = {'speed_bonus': None}
        args = (document(pick, 1, 2, 3), document(4), qengine, document(1))
        fixed, count = regenerate(source, *args)
        self.assertEqual(count, 5)
        self.assertIn("assert_eq!(added_keys.len(), 1);", fixed)
        self.assertIn('("qengine-reward-item-profiles.json", 1)', fixed)
        self.assertIn("assert_eq!(venison.attributes.speed_bonus, Some(0))", fixed)
        self.assertIn("assert_eq!(actual.attributes.speed_bonus, None", fixed)
        self.assertEqual(regenerate(fixed, *args), (fixed, 5))


    def srx3_source(self):
        return self.SOURCE.replace("            + npc_additions.len()", "            + additions.len()\n            + npc_additions.len()")

    def test_source_packet_minus_declared_prior_identities_adds_only_new_keys(self):
        proof = [("SRX3-TEST", document(2, 5), {"2"})]
        fixed, count = regenerate(self.srx3_source(), document(0, 1, 2, 3, 5),
                                  document(4), document(0), document(1), srx3=proof)
        self.assertEqual(count, 6)
        self.assertIn('(\"SRX3-TEST\", 1)', fixed)
        self.assertIn('assert_eq!(venison.attributes.speed_bonus, Some(0))', fixed)
        self.assertEqual(regenerate(fixed, document(0, 1, 2, 3, 5), document(4),
                                    document(0), document(1), srx3=proof), (fixed, 6))

    def test_registered_engine_subtraction_composes_with_source_cohorts(self):
        source = self.srx3_source().replace(
            '("QNPCFX", 1)', '("QNPCFX", 1), ("QENGINE-REWARDS", 1)'
        ).replace('+ additions.len()', '+ (additions.len() - 1)')
        proof = [("SRX3-TEST", document(2, 5), {"2"})]
        args = (document(0, 1, 2, 3, 5), document(4), document(0), document(1))
        fixed, count = regenerate(source, *args, srx3=proof)
        self.assertEqual(count, 6)
        self.assertIn('+ (additions.len() - 1)', fixed)
        self.assertIn('+ SRX3_SOURCE_COHORTS.iter()', fixed)
        self.assertIn('assert_eq!(venison.attributes.speed_bonus, Some(0))', fixed)
        self.assertEqual(regenerate(fixed, *args, srx3=proof), (fixed, 6))

    def test_missing_new_source_profile_is_not_replaced_by_a_lower_count(self):
        with self.assertRaisesRegex(ValueError, "SRX3 source profile differs"):
            regenerate(self.srx3_source(), document(0, 1, 2, 3), document(4),
                       document(0), document(1), srx3=[("SRX3-TEST", document(2, 5), {"2"})])

    def test_unproven_actual_key_does_not_increase_source_expectation(self):
        with self.assertRaisesRegex(ValueError, "declared cohorts=6, generated profile union=7"):
            regenerate(self.srx3_source(), document(0, 1, 2, 3, 5, 6), document(4),
                       document(0), document(1), srx3=[("SRX3-TEST", document(2, 5), {"2"})])

    def test_changed_new_profile_provenance_is_refused(self):
        actual = document(0, 1, 2, 3, 5)
        actual["records"][-1]["production_binding"] = {"disposition": "forged"}
        with self.assertRaisesRegex(ValueError, "SRX3 source profile differs"):
            regenerate(self.srx3_source(), actual, document(4), document(0), document(1),
                       srx3=[("SRX3-TEST", document(2, 5), {"2"})])

    def test_false_prior_intersection_and_overlapping_source_cohorts_are_refused(self):
        with self.assertRaisesRegex(ValueError, "source-base intersection"):
            regenerate(self.srx3_source(), document(0, 1, 2, 3, 5), document(4), document(0), document(1),
                       srx3=[("SRX3-TEST", document(2, 5), {"3"})])
        with self.assertRaisesRegex(ValueError, "source cohorts overlap"):
            regenerate(self.srx3_source(), document(0, 1, 2, 3, 5), document(4), document(0), document(1),
                       srx3=[("SRX3-A", document(2, 5), {"2"}), ("SRX3-B", document(5), set())])


    def coowned(self, *, source=None, actual=None, proof=None, coowner=None, key="5"):
        source = source or self.srx3_source().replace(
            '("QNPCFX", 1)', '("QNPCFX", 1), ("W10-PRIMAL", 1)')
        return regenerate(source, actual or document(0, 1, 2, 3, key), document(4),
                          document(0), document(1),
                          srx3=[("SRX3-TEST", proof or document(2, key), {"2"})],
                          coowners=[("W10-PRIMAL", coowner or document(key), {key})])

    def test_independently_counted_coowner_is_not_counted_again(self):
        fixed, count = self.coowned()
        self.assertEqual(count, 6)
        self.assertIn('("SRX3-TEST", 0)', fixed)
        self.assertIn('("W10-PRIMAL", 1)', fixed)
        self.assertEqual(self.coowned(source=fixed), (fixed, 6))

    def test_missing_coowned_profile_is_refused(self):
        with self.assertRaisesRegex(ValueError, "coowner identity set"):
            self.coowned(actual=document(0, 1, 2, 3))

    def test_coowner_declared_count_must_match_its_independent_source(self):
        source = self.srx3_source().replace(
            '("QNPCFX", 1)', '("QNPCFX", 1), ("W10-PRIMAL", 2)')
        with self.assertRaisesRegex(ValueError, "coowner declaration"):
            self.coowned(source=source)

    def test_changed_shared_semantics_cannot_be_hidden_by_matching_actual_and_srx3(self):
        actual, proof = document(0, 1, 2, 3, 5), document(2, 5)
        actual['records'][-1]['semantics'] = {'invented': True}
        proof['records'][-1]['semantics'] = {'invented': True}
        with self.assertRaisesRegex(ValueError, "coowner profile differs"):
            self.coowned(actual=actual, proof=proof)

    def test_only_four_closed_candidate_revision_aliases_are_allowed(self):
        import copy
        for item in (39527, 39528, 39529, 39530):
            key = f'oteryn:item.tibia.i{item}'
            actual, proof, coowner = document(0, 1, 2, 3, key), document(2, key), document(key)
            for doc, revision in [(actual, 'srx3-boss-corpses-crystal-201189f'),
                                  (proof, 'srx3-boss-corpses-crystal-201189f'),
                                  (coowner, 'spell-p2-r21')]:
                doc['records'][-1]['authoring'] = {'item': {
                    'family': 'Item', 'key': f'candidate:item/{item}', 'revision': revision}}
            self.assertEqual(self.coowned(key=key, actual=actual, proof=proof, coowner=coowner)[1], 6)
            for field, value in [('extra', 'unqualified'), ('revision', 'forged')]:
                broken = copy.deepcopy(coowner)
                if field == 'revision': broken['records'][-1]['authoring']['item'][field] = value
                else: broken['records'][-1]['authoring'][field] = value
                with self.assertRaisesRegex(ValueError, "coowner profile differs"):
                    self.coowned(key=key, actual=actual, proof=proof, coowner=broken)
        key = 'oteryn:item.tibia.i39176'
        actual, proof, coowner = document(0, 1, 2, 3, key), document(2, key), document(key)
        for doc, revision in [(actual, 'srx3-boss-corpses-crystal-201189f'),
                              (proof, 'srx3-boss-corpses-crystal-201189f'),
                              (coowner, 'spell-p2-r21')]:
            doc['records'][-1]['authoring'] = {'item': {
                'family': 'Item', 'key': 'candidate:item/39176', 'revision': revision}}
        with self.assertRaisesRegex(ValueError, "coowner profile differs"):
            self.coowned(key=key, actual=actual, proof=proof, coowner=coowner)

    def test_coowner_must_actually_overlap_the_qualified_srx3_owner(self):
        with self.assertRaisesRegex(ValueError, "coowner source overlap"):
            self.coowned(proof=document(2))

    def test_collector_census_retains_the_frozen_base_and_exact_added_identities(self):
        census = {'schema': 1, 'base': '1ae39c7fb9bef09c3cd50d16410374f4134b1676',
                  'base_count': 3663, 'added_keys': ['3663', '3664']}
        args = (self.SOURCE, document(*range(3665)), document(), document(0), document(1))
        self.assertEqual(regenerate(*args, custody=census), (self.SOURCE, 3665))
        with self.assertRaisesRegex(ValueError, 'source identity absent'):
            regenerate(*args, custody={**census, 'added_keys': ['3663', 'missing']})
        with self.assertRaisesRegex(ValueError, 'declared cohorts=3664'):
            regenerate(*args, custody={**census, 'added_keys': ['3663']})

    def test_collector_census_rejects_changed_baselines_and_duplicate_additions(self):
        census = {'schema': 1, 'base': '1ae39c7fb9bef09c3cd50d16410374f4134b1676',
                  'base_count': 3663, 'added_keys': ['3663']}
        args = (self.SOURCE, document(*range(3664)), document(), document(0), document(1))
        with self.assertRaisesRegex(ValueError, 'pinned custody baseline'):
            regenerate(*args, custody={**census, 'base_count': 3662})
        with self.assertRaisesRegex(ValueError, 'duplicate custody addition'):
            regenerate(*args, custody={**census, 'added_keys': ['3663', '3663']})

    def test_new_source_cohort_does_not_change_immutable_baseline_or_drop_engine_pin(self):
        source = self.SOURCE.replace('[("QNPCFX", 1)];',
            '[\n    ("QNPCFX", 1),\n    ("QUICKLOOT-1", 0),\n    ("QPC7-CORPSE", 2),\n];')
        source += '''
    let later_main_profiles = 0;
    const QENGINE2_BASE_PROFILES: usize = 1;
    assert_eq!(additions.len(), QENGINE2_BASE_PROFILES);
    let baseline = 3663;
    assert_eq!(baseline, 3663);
'''
        census = {'schema': 1, 'base': '1ae39c7fb9bef09c3cd50d16410374f4134b1676',
                  'base_count': 3663, 'added_keys': ['3663', '3664']}
        kwargs = dict(custody=census, frozen=({str(i) for i in range(3665)}, 2),
                      baseline_cohorts={'QUICKLOOT-1'}, baseline_qengine_count=1)
        args = (document(*range(3665)), document(), document(0), document(1))
        fixed, count = regenerate(source, *args, **kwargs)
        self.assertEqual(count, 3665)
        self.assertIn('"QPC7-CORPSE"', fixed.split('let later_main_profiles =')[1].split('let baseline =')[0])
        self.assertIn('const QENGINE2_BASE_PROFILES: usize = 1;', fixed)
        self.assertIn('assert_eq!(baseline, 3663);', fixed)
        self.assertEqual(regenerate(fixed, *args, **kwargs), (fixed, count))
    def test_census_includes_srx3_once_but_still_verifies_its_source_body(self):
        census = {'schema': 1, 'base': '1ae39c7fb9bef09c3cd50d16410374f4134b1676',
                  'base_count': 3663, 'added_keys': ['3663']}
        args = (self.SOURCE, document(*range(3664)), document(), document(0), document(1))
        proof = [("SRX3-TEST", document(3663), set())]
        fixed, count = regenerate(*args, custody=census, srx3=proof, duplicate_keys={'0'})
        self.assertEqual((fixed, count), (self.SOURCE, 3664))
        changed = document(3663)
        changed['records'][0]['production_binding'] = {'disposition': 'forged'}
        with self.assertRaisesRegex(ValueError, 'SRX3 source profile differs'):
            regenerate(*args, custody=census, srx3=[('SRX3-TEST', changed, set())])

    def test_changed_baseline_cohort_requires_a_separate_identity_proof(self):
        source = self.SOURCE + (
            '    let later_main_profiles = 0;\n'
            '    let baseline = 3663;\n'
        )
        census = {'schema': 1, 'base': '1ae39c7fb9bef09c3cd50d16410374f4134b1676',
                  'base_count': 3663, 'added_keys': []}
        source = source.replace('[("QNPCFX", 1)];', '[("QNPCFX", 1), ("EXISTING", 2)];')
        with self.assertRaisesRegex(ValueError, 'baseline cohort count changed'):
            regenerate(source, document(*range(3663)), document(), document(0), document(1),
                       custody=census, baseline_cohorts={'EXISTING': 1}, baseline_qengine_count=1)



class M25SourceCohortTests(unittest.TestCase):
    def test_primal_pod_replays_the_authoritative_registration_top_order(self):
        cohorts = refresh.source_cohorts(refresh.ROOT)
        document = next(document for name, document, _ in cohorts
                        if name == "SRX3-PRIMAL-POD")
        row = next(row for row in document["records"]
                   if row["production_definition"]["production_key"] == "oteryn:item.tibia.i39176")
        house = refresh.importlib.import_module("house_decoration_actions")
        overlay = json.loads((refresh.ROOT / house.PROFILES).read_bytes())
        self.assertEqual(overlay["always_on_top"]["39176"], 3)
        self.assertIs(row["attributes"].get("always_on_top"), True)
        self.assertEqual(row["attributes"].get("always_on_top_order"), 3)
        # Registration adds source order, never a new identity or custody rule.
        self.assertEqual(refresh.keys(document), {"oteryn:item.tibia.i39176"})
        self.assertEqual(row["admission"], {"materializable": True,
            "stack_class": "NonStackable", "legal_destinations": ["Ground"]})

    def test_m25_records_are_independently_qualified_before_counting(self):
        declaration = ("M25-GALTHEN-CORPSES", "m25_corpse_items", ())
        self.assertIn(declaration, refresh.SRX3_SOURCES)
        cohorts = refresh.source_cohorts(refresh.ROOT)
        selected = [row for row in cohorts if row[0] == declaration[0]]
        self.assertEqual(len(selected), 1)
        _, document, previous = selected[0]
        self.assertEqual(previous, set())
        self.assertEqual(refresh.keys(document), {
            f"oteryn:item.tibia.i{item}" for item in
            (42069, 42070, 42071, 42072, 44684, 44685, 44686, 44687)
        })
        owner = refresh.importlib.import_module("m25_corpse_items")
        proven, _ = owner.checked_packet(refresh.ROOT)
        taxonomy = {row["target"]["key"]: row["source_taxonomy"]
                    for row in json.loads((refresh.ROOT / "content/items/taxonomy/items.json").read_bytes())["records"]}
        expected = copy.deepcopy(proven)
        for row in expected["records"]:
            key = row["production_definition"]["production_key"]
            if key in taxonomy and "taxonomy" not in row["authoring"]:
                row["authoring"]["taxonomy"] = copy.deepcopy(taxonomy[key])
        house = refresh.importlib.import_module("house_decoration_actions")
        overlay = json.loads((refresh.ROOT / house.PROFILES).read_bytes())
        house.apply_top_order(expected["records"], overlay)
        self.assertEqual(document, {"records": sorted(expected["records"], key=lambda row: row["production_definition"]["production_key"])})

    def test_m25_corrupt_source_packet_never_becomes_a_count_only_cohort(self):
        # Load the ordinary owner chain first, as the production helper does.
        refresh.source_cohorts(refresh.ROOT)
        owner = refresh.importlib.import_module("m25_corpse_items")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in (owner.PATH, owner.PACKET, owner.PIN_PATH,
                         "content/items/taxonomy/items.json"):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes((refresh.ROOT / name).read_bytes())
            data = json.loads((root / owner.PATH).read_bytes())
            data["records"].pop()
            (root / owner.PATH).write_bytes(owner.encode(data))
            with patch.object(refresh, "SRX3_SOURCES", (("M25-GALTHEN-CORPSES", "m25_corpse_items", ()),)):
                with self.assertRaisesRegex(ValueError, "M25_CAPTURE_DRIFT"):
                    refresh.source_cohorts(root)

class NpcHouseProjectionTests(unittest.TestCase):
    JAR = 'oteryn:item.tibia.i27654'
    TOP = ('always_on_top', 'always_on_top_order')

    @classmethod
    def setUpClass(cls):
        cls.engine = json.loads((refresh.ROOT / 'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json').read_bytes())
        cls.npc = json.loads((refresh.ROOT / 'tools/content-schema/native-gameplay/qnpcfx-item-profiles.json').read_bytes())
        cls.house = refresh.importlib.import_module('house_decoration_actions')
        cls.quick = refresh.importlib.import_module('quick_loot_item_profiles')
        cls.raw_jar = copy.deepcopy(cls.row(cls.npc))
        if (any(name in cls.raw_jar['attributes'] for name in cls.TOP)
                or 'taxonomy' in cls.raw_jar['authoring']):
            raise AssertionError('the retained NPC source must keep its raw Jar observation')
        cls.house_jar = refresh.registered_source_document(
            refresh.ROOT, {'records': [cls.raw_jar]})['records'][0]
        cls.taxonomy_jar = refresh.registered_jar_taxonomy_document(
            refresh.ROOT, {'records': [cls.raw_jar]})['records'][0]
        cls.complete_jar = refresh.registered_source_document(
            refresh.ROOT, {'records': [cls.taxonomy_jar]})['records'][0]
        taxonomy = json.loads((refresh.ROOT / 'content/items/taxonomy/items.json').read_bytes())
        entries = [row for row in taxonomy['records'] if row['target']['key'] == cls.JAR]
        if (len(entries) != 1 or entries[0]['target'] !=
                {'family': 'Item', 'key': cls.JAR, 'revision': 'definition-r1'}):
            raise AssertionError('canonical Jar taxonomy must have one exact identity')
        cls.taxonomy = entries[0]['source_taxonomy']
        cls.qualified_states = (cls.raw_jar, cls.house_jar, cls.taxonomy_jar, cls.complete_jar)
        # Authenticate the actual packet before constructing a first-pass fixture.
        # Never hide an unproved difference by stripping fields from that packet.
        refresh.keys(cls.engine)
        if json.dumps(cls.row(cls.engine), sort_keys=True) not in {
                json.dumps(row, sort_keys=True) for row in cls.qualified_states}:
            raise AssertionError('actual QENGINE Jar is not a qualified registration state')
        cls.raw_engine = copy.deepcopy(cls.engine)
        index = next(i for i, row in enumerate(cls.raw_engine['records'])
                     if row['production_definition']['production_key'] == cls.JAR)
        cls.raw_engine['records'][index] = copy.deepcopy(cls.raw_jar)
        cls.registered_engine = copy.deepcopy(cls.raw_engine)
        cls.registered_engine['records'][index] = copy.deepcopy(cls.house_jar)
        cls.taxonomy_engine = copy.deepcopy(cls.raw_engine)
        cls.taxonomy_engine['records'][index] = copy.deepcopy(cls.taxonomy_jar)
        cls.complete_engine = copy.deepcopy(cls.raw_engine)
        cls.complete_engine['records'][index] = copy.deepcopy(cls.complete_jar)

    @classmethod
    def row(cls, packet):
        return next(row for row in packet['records']
                    if row['production_definition']['production_key'] == cls.JAR)

    def compose(self, engine, npc, root=None):
        before = copy.deepcopy((engine, npc))
        try:
            retained_engine, projected_npc = refresh.registered_engine_npc_documents(
                root or refresh.ROOT, engine, npc)
            self.assertEqual(retained_engine, engine)
            self.assertIsNot(retained_engine, engine)
            return projected_npc
        finally:
            self.assertEqual((engine, npc), before, 'qualification must not rewrite either source packet')

    def qualify(self, engine, npc):
        # Drive the unchanged complete-record/intersection guard with real Jar
        # rows; other cohorts do not participate in this one-identity seam.
        source = EquipmentPinTests.SOURCE.replace('usize = 1', 'usize = 0').replace(
            'cohorts add 1 + 1 keys', 'cohorts add 0 + 0 keys')
        engine = {'records': [self.row(engine)]}
        npc = {'records': [self.row(npc)]}
        return regenerate(source, engine, {'records': []}, engine, npc,
                          engine_npc_shared=(self.JAR,))

    def test_current_immutable_packets_have_only_the_proven_house_successor(self):
        expected = copy.deepcopy(self.raw_jar)
        expected['attributes'].update(always_on_top=False, always_on_top_order=0)
        self.assertEqual(self.house_jar, expected)
        self.assertIs(self.house_jar['attributes']['always_on_top'], False)
        self.assertIs(type(self.house_jar['attributes']['always_on_top_order']), int)
        expected_taxonomy = copy.deepcopy(self.raw_jar)
        expected_taxonomy['authoring']['taxonomy'] = copy.deepcopy(self.taxonomy)
        self.assertEqual(self.taxonomy_jar, expected_taxonomy)
        expected['authoring']['taxonomy'] = copy.deepcopy(self.taxonomy)
        self.assertEqual(self.complete_jar, expected)
        self.assertEqual(self.row(self.raw_engine), self.raw_jar)
        self.assertIn(json.dumps(self.row(self.engine), sort_keys=True),
                      [json.dumps(row, sort_keys=True) for row in self.qualified_states])
        self.assertEqual(len(self.npc['records']), 8)

    def test_raw_equal_pair_keeps_the_first_registration_pass(self):
        with patch.object(refresh, 'registered_source_document', side_effect=AssertionError('raw pair needs no House replay')):
            result = self.compose(self.raw_engine, self.npc)
        self.assertEqual(result, self.npc)
        self.assertEqual(self.qualify(self.raw_engine, result)[1], 1)

    def test_real_house_projection_changes_only_the_jar_and_keeps_full_equality(self):
        result = self.compose(self.registered_engine, self.npc)
        expected = copy.deepcopy(self.npc)
        self.row(expected)['attributes'].update(always_on_top=False, always_on_top_order=0)
        self.assertEqual(result, expected, 'all packet metadata and seven other complete rows stay unchanged')
        self.assertEqual(self.row(result), self.house_jar)
        self.assertEqual(self.qualify(self.registered_engine, result),
                         self.qualify(self.raw_engine, self.npc))

    def test_already_registered_pair_and_replay_keep_identical_packets(self):
        registered = self.compose(self.registered_engine, self.npc)
        self.assertEqual(self.compose(self.registered_engine, registered), registered)
        self.assertEqual(self.compose(self.registered_engine, self.npc), registered)
        self.assertEqual(self.qualify(self.registered_engine, registered)[1], 1)

    def test_raw_engine_cannot_silently_downgrade_registered_npc(self):
        registered = copy.deepcopy(self.npc)
        self.row(registered)['attributes'].update(always_on_top=False, always_on_top_order=0)
        result = self.compose(self.raw_engine, registered)
        self.assertEqual(result, registered)
        with self.assertRaisesRegex(ValueError, 'QENGINE/NPC shared source identity or facts differ'):
            self.qualify(self.raw_engine, result)

    def test_partial_wrong_or_wrongly_typed_flags_refuse_before_house_replay(self):
        invalid = [{'always_on_top': False}, {'always_on_top_order': 0}]
        invalid += [{'always_on_top': value, 'always_on_top_order': 0}
                    for value in (True, 0, None, 'false')]
        invalid += [{'always_on_top': False, 'always_on_top_order': value}
                    for value in (1, False, 0.0, None, '0')]
        for owner in ('engine', 'npc'):
            for flags in invalid:
                with self.subTest(owner=owner, flags=flags):
                    engine, npc = copy.deepcopy((self.registered_engine, self.npc))
                    attributes = self.row(engine if owner == 'engine' else npc)['attributes']
                    for name in self.TOP:
                        attributes.pop(name, None)
                    attributes.update(flags)
                    with patch.object(refresh, 'registered_source_document', side_effect=AssertionError('invalid predecessor must not reach House')):
                        with self.assertRaisesRegex(ValueError, 'NPC_HOUSE_'):
                            self.compose(engine, npc)

    def test_missing_duplicate_foreign_or_extra_shared_identities_are_refused(self):
        for owner in ('engine', 'npc'):
            for mode in ('missing', 'duplicate', 'foreign'):
                with self.subTest(owner=owner, mode=mode):
                    engine, npc = copy.deepcopy((self.registered_engine, self.npc))
                    packet = engine if owner == 'engine' else npc
                    row = self.row(packet)
                    if mode == 'missing':
                        packet['records'].remove(row)
                    elif mode == 'duplicate':
                        packet['records'].append(copy.deepcopy(row))
                    else:
                        row['production_definition']['production_key'] = 'oteryn:item.tibia.i1'
                    with self.assertRaises(ValueError):
                        self.compose(engine, npc)
        engine = copy.deepcopy(self.registered_engine)
        engine['records'].append(copy.deepcopy(next(row for row in self.npc['records']
                                                    if row['production_definition']['production_key'] == 'oteryn:item.tibia.i919')))
        with self.assertRaisesRegex(ValueError, 'NPC_HOUSE_'):
            self.compose(engine, self.npc)

    def test_full_record_disagreements_survive_projection_and_are_refused(self):
        for field in ('authoring', 'semantics', 'attributes', 'admission',
                      'production_binding', 'production_definition'):
            with self.subTest(field=field):
                npc = copy.deepcopy(self.npc)
                self.row(npc)[field]['unreviewed_field'] = 'not a House fact'
                result = self.compose(self.registered_engine, npc)
                self.assertEqual(self.row(result)[field]['unreviewed_field'], 'not a House fact')
                with self.assertRaisesRegex(ValueError, 'QENGINE/NPC shared source identity or facts differ'):
                    self.qualify(self.registered_engine, result)

    def test_real_house_source_schema_and_catalogue_guards_are_retained(self):
        for mode in ('schema', 'catalogue_sha256'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                for path in (self.house.PROFILES, self.house.CATALOGUE):
                    target = root / path
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes((refresh.ROOT / path).read_bytes())
                overlay_path = root / self.house.PROFILES
                overlay = json.loads(overlay_path.read_bytes())
                overlay[mode] = 'unreviewed source'
                overlay_path.write_text(json.dumps(overlay))
                with self.assertRaisesRegex(ValueError, 'HOUSE_DECORATION_PROFILE_SOURCE_DRIFT'):
                    self.compose(self.registered_engine, self.npc, root)

    def test_house_projection_cannot_change_other_fields_or_lose_exact_types(self):
        for mode in ('extra_field', 'identity', 'partial', 'top_type', 'order_type'):
            with self.subTest(mode=mode):
                output = {'records': [copy.deepcopy(self.house_jar)]}
                row = output['records'][0]
                if mode == 'extra_field':
                    row['admission']['unreviewed'] = True
                elif mode == 'identity':
                    row['production_definition']['production_key'] = 'oteryn:item.tibia.i1'
                elif mode == 'partial':
                    del row['attributes']['always_on_top']
                elif mode == 'top_type':
                    row['attributes']['always_on_top'] = 0
                else:
                    row['attributes']['always_on_top_order'] = False
                with patch.object(refresh, 'registered_source_document', return_value=output):
                    with self.assertRaisesRegex(ValueError, 'NPC_HOUSE_'):
                        self.compose(self.registered_engine, self.npc)




    def test_real_quickloot_projection_changes_only_canonical_jar_taxonomy(self):
        result = self.compose(self.taxonomy_engine, self.npc)
        expected = copy.deepcopy(self.npc)
        self.row(expected)['authoring']['taxonomy'] = copy.deepcopy(self.taxonomy)
        self.assertEqual(result, expected, 'all metadata and seven other rows stay exact')
        self.assertEqual(self.row(result), self.taxonomy_jar)
        self.assertEqual(self.qualify(self.taxonomy_engine, result),
                         self.qualify(self.raw_engine, self.npc))

    def test_quickloot_and_house_successor_is_exact_and_replays(self):
        result = self.compose(self.complete_engine, self.npc)
        expected = copy.deepcopy(self.npc)
        self.row(expected)['authoring']['taxonomy'] = copy.deepcopy(self.taxonomy)
        self.row(expected)['attributes'].update(always_on_top=False, always_on_top_order=0)
        self.assertEqual(result, expected)
        self.assertEqual(self.row(result), self.complete_jar)
        self.assertEqual(self.compose(self.complete_engine, result), result)
        self.assertEqual(self.compose(self.complete_engine, self.npc), result)
        self.assertEqual(self.qualify(self.complete_engine, result),
                         self.qualify(self.raw_engine, self.npc))

    def test_actual_engine_packet_qualifies_without_replacing_its_row(self):
        result = self.compose(self.engine, self.npc)
        self.assertEqual(self.row(result), self.row(self.engine))
        self.assertEqual(self.qualify(self.engine, result),
                         self.qualify(self.raw_engine, self.npc))

    def test_partial_wrong_or_noncanonical_taxonomy_refuses_before_quickloot(self):
        invalid = (None, False, {}, 'Valuables', ['Valuables'],
                   {'primary': None}, {'primary': True}, {'primary': 'Tools'},
                   {'secondary': 'Valuables'}, {'primary': 'Valuables', 'unreviewed': True})
        for owner in ('engine', 'npc'):
            for taxonomy in invalid:
                with self.subTest(owner=owner, taxonomy=taxonomy):
                    engine, npc = copy.deepcopy((self.complete_engine, self.npc))
                    self.row(engine if owner == 'engine' else npc)['authoring']['taxonomy'] = taxonomy
                    with patch.object(refresh, 'registered_jar_taxonomy_document',
                                      side_effect=AssertionError('invalid predecessor must not reach QuickLoot')):
                        with self.assertRaisesRegex(ValueError, 'NPC_JAR_TAXONOMY_PREDECESSOR'):
                            self.compose(engine, npc)

    def test_raw_or_house_engine_cannot_downgrade_canonical_taxonomy(self):
        for engine in (self.raw_engine, self.registered_engine):
            for row in (self.taxonomy_jar, self.complete_jar):
                with self.subTest(engine=self.row(engine), npc=row):
                    npc = copy.deepcopy(self.npc)
                    index = next(i for i, item in enumerate(npc['records'])
                                 if item['production_definition']['production_key'] == self.JAR)
                    npc['records'][index] = copy.deepcopy(row)
                    result = self.compose(engine, npc)
                    self.assertEqual(self.row(result)['authoring']['taxonomy'], self.taxonomy)
                    with self.assertRaisesRegex(ValueError, 'QENGINE/NPC shared source identity or facts differ'):
                        self.qualify(engine, result)

    def test_taxonomy_projection_cannot_change_other_fields_or_source_value(self):
        for mode in ('body', 'identity', 'missing', 'partial', 'wrong_type', 'foreign'):
            with self.subTest(mode=mode):
                projected = {'records': [copy.deepcopy(self.taxonomy_jar)]}
                row = projected['records'][0]
                if mode == 'body':
                    row['admission']['unreviewed'] = True
                elif mode == 'identity':
                    row['production_definition']['production_key'] = 'oteryn:item.tibia.i1'
                elif mode == 'missing':
                    del row['authoring']['taxonomy']
                elif mode == 'partial':
                    row['authoring']['taxonomy'] = {}
                elif mode == 'wrong_type':
                    row['authoring']['taxonomy'] = 'Valuables'
                else:
                    row['authoring']['taxonomy'] = {'primary': 'Tools'}
                with patch.object(refresh, 'registered_jar_taxonomy_document', return_value=projected):
                    with self.assertRaisesRegex(ValueError, 'NPC_JAR_TAXONOMY_REGISTRATION_DELTA'):
                        self.compose(self.taxonomy_engine, self.npc)

    def test_real_quickloot_source_digest_is_required(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            taxonomy = root / 'content/items/taxonomy/items.json'
            taxonomy.parent.mkdir(parents=True)
            taxonomy.write_bytes((refresh.ROOT / 'content/items/taxonomy/items.json').read_bytes())
            source = root / self.quick.SOURCE
            source.parent.mkdir(parents=True)
            source.write_bytes(b'not the pinned Canary source')
            with self.assertRaisesRegex(ValueError, 'QUICKLOOT_SOURCE_DIGEST'):
                self.compose(self.taxonomy_engine, self.npc, root)

    def test_taxonomy_source_requires_one_exact_canonical_identity(self):
        original = json.loads((refresh.ROOT / 'content/items/taxonomy/items.json').read_bytes())
        row = next(row for row in original['records'] if row['target']['key'] == self.JAR)
        for mode in ('missing', 'duplicate', 'revision', 'family', 'shape'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                document = {'records': [copy.deepcopy(row)]}
                if mode == 'missing':
                    document['records'].clear()
                elif mode == 'duplicate':
                    document['records'].append(copy.deepcopy(row))
                elif mode in ('revision', 'family'):
                    document['records'][0]['target'][mode] = 'unreviewed'
                else:
                    document['records'][0]['source_taxonomy'] = {'primary': None}
                target = root / 'content/items/taxonomy/items.json'
                target.parent.mkdir(parents=True)
                target.write_text(json.dumps(document))
                with self.assertRaisesRegex(ValueError, 'NPC_JAR_TAXONOMY_SOURCE_'):
                    self.compose(self.taxonomy_engine, self.npc, root)


class SharedSourceDiagnosticTests(unittest.TestCase):
    PREFIX = "QENGINE/NPC shared source identity or facts differ"

    def refused(self, qengine, npc, expected_shared):
        source = EquipmentPinTests.SOURCE.replace(
            '("QNPCFX", 1)', f'("QNPCFX", {len(npc["records"])})')
        packets = (document(0, 1, 2, 3), document(4), qengine, npc)
        before = json.dumps(packets, sort_keys=True)
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "ability_tests.rs"
            target.write_text(source, encoding="utf-8")
            source_bytes = target.read_bytes()
            with patch.object(refresh, "TEST", target):
                with self.assertRaisesRegex(ValueError, self.PREFIX) as caught:
                    regenerate(target.read_text(encoding="utf-8"), *packets,
                               engine_npc_shared=expected_shared)
            self.assertEqual(target.read_bytes(), source_bytes)
            self.assertEqual(list(Path(directory).iterdir()), [target])
        self.assertEqual(json.dumps(packets, sort_keys=True), before)
        message = str(caught.exception)
        self.assertTrue(message.startswith(self.PREFIX + ": "), message)
        payload = json.loads(message[len(self.PREFIX) + 2:])
        self.assertEqual(message, self.PREFIX + ": " + json.dumps(payload, sort_keys=True))
        return payload

    def test_mismatching_complete_records_preserve_missing_null_bool_and_integer(self):
        jar = "oteryn:item.tibia.i27654"
        equal = "oteryn:item.tibia.i999"
        qengine = document(equal, jar)
        npc = document(jar, equal)
        engine_row = qengine["records"][1]
        npc_row = npc["records"][0]
        engine_row["attributes"] = {
            "speed_bonus": None, "always_on_top": False, "always_on_top_order": 0}
        npc_row["attributes"] = {
            "always_on_top": 0, "always_on_top_order": False}
        engine_row["authoring"] = {"source": "engine", "nested": {"retained": True}}
        npc_row["authoring"] = {"source": "npc", "nested": {"retained": True}}
        payload = self.refused(qengine, npc, (equal, jar))
        expected = {
            "expected_shared": sorted((equal, jar)),
            "actual_shared": sorted((equal, jar)),
            "mismatching_records": {jar: {"qengine": engine_row, "npc": npc_row}},
        }
        self.assertEqual(json.dumps(payload, sort_keys=True), json.dumps(expected, sort_keys=True))
        self.assertEqual(set(payload["mismatching_records"]), {jar})
        pair = payload["mismatching_records"][jar]
        self.assertIsNone(pair["qengine"]["attributes"]["speed_bonus"])
        self.assertNotIn("speed_bonus", pair["npc"]["attributes"])
        self.assertIs(pair["qengine"]["attributes"]["always_on_top"], False)
        self.assertIs(type(pair["qengine"]["attributes"]["always_on_top_order"]), int)
        self.assertIs(type(pair["npc"]["attributes"]["always_on_top"]), int)
        self.assertIs(pair["npc"]["attributes"]["always_on_top_order"], False)

    def test_identity_set_mismatch_reports_sorted_keys_without_equal_records(self):
        qengine = document("z-shared", "b-shared", "a-engine-only")
        npc = document("b-shared", "z-shared")
        payload = self.refused(qengine, npc, ("z-expected", "a-expected"))
        self.assertEqual(payload, {
            "expected_shared": ["a-expected", "z-expected"],
            "actual_shared": ["b-shared", "z-shared"],
            "mismatching_records": {},
        })


class RegisteredEngineNpcSourceTests(unittest.TestCase):
    KEY = "oteryn:item.tibia.i27654"

    @classmethod
    def setUpClass(cls):
        cls.engine = json.loads((refresh.ROOT /
            "tools/content-schema/native-gameplay/qengine-reward-item-profiles.json").read_bytes())
        cls.npc = json.loads((refresh.ROOT /
            "tools/content-schema/native-gameplay/qnpcfx-item-profiles.json").read_bytes())

    def shared(self, value):
        return next(row for row in value["records"]
                    if row["production_definition"]["production_key"] == self.KEY)

    def qualified(self, engine, npc):
        before = copy.deepcopy((engine, npc))
        try:
            return refresh.registered_engine_npc_documents(refresh.ROOT, engine, npc)
        finally:
            self.assertEqual((engine, npc), before)

    def generate_shared(self, engine, npc, source=None):
        base = document(0, 1, 2)
        base["records"].append(copy.deepcopy(self.shared(engine)))
        return regenerate(source or EquipmentPinTests.SOURCE, base, document(),
                          {"records": [self.shared(engine)]},
                          {"records": [self.shared(npc)]},
                          engine_npc_shared=(self.KEY,))

    def test_real_house_stage_difference_is_qualified_without_changing_other_rows(self):
        self.assertEqual(refresh.keys(self.engine) & refresh.keys(self.npc), {self.KEY})
        engine = copy.deepcopy(self.engine)
        index = next(i for i, row in enumerate(engine["records"])
                     if row["production_definition"]["production_key"] == self.KEY)
        engine["records"][index] = refresh.registered_source_document(
            refresh.ROOT, {"records": [self.shared(engine)]})["records"][0]
        with self.assertRaisesRegex(ValueError, "QENGINE/NPC shared source"):
            self.generate_shared(engine, self.npc)
        qualified = self.qualified(engine, self.npc)
        self.assertEqual(self.shared(qualified[0]), self.shared(qualified[1]))
        self.assertIs(self.shared(qualified[0])["attributes"]["always_on_top"], False)
        self.assertEqual(self.shared(qualified[0])["attributes"]["always_on_top_order"], 0)
        expected_taxonomy = None
        if "taxonomy" in self.shared(engine)["authoring"]:
            independent = refresh.registered_jar_taxonomy_document(
                refresh.ROOT, {"records": [copy.deepcopy(self.shared(self.npc))]})
            expected_taxonomy = independent["records"][0]["authoring"]["taxonomy"]
        for before, after in zip((engine, self.npc), qualified):
            expected = copy.deepcopy(before)
            self.shared(expected)["attributes"].update(
                always_on_top=False, always_on_top_order=0)
            if expected_taxonomy is not None:
                self.shared(expected)["authoring"]["taxonomy"] = copy.deepcopy(expected_taxonomy)
            self.assertEqual(after, expected)
            self.assertEqual(refresh.keys(after), refresh.keys(before))
        fixed, count = self.generate_shared(*qualified)
        self.assertEqual(count, 4)
        self.assertEqual(self.generate_shared(*qualified, source=fixed), (fixed, count))
        self.assertEqual(self.qualified(*qualified), qualified)

    def test_existing_house_values_require_exact_types_complete_pair_and_source_value(self):
        values = [
            {"always_on_top": True, "always_on_top_order": 0},
            {"always_on_top": False, "always_on_top_order": 1},
            {"always_on_top": 0, "always_on_top_order": 0},
            {"always_on_top": False, "always_on_top_order": False},
            {"always_on_top": False, "always_on_top_order": 0.0},
            {"always_on_top": None, "always_on_top_order": 0},
            {"always_on_top": False, "always_on_top_order": None},
            {"always_on_top": False},
            {"always_on_top_order": 0},
        ]
        for attributes in values:
            with self.subTest(attributes=attributes):
                engine = copy.deepcopy(self.engine)
                actual = self.shared(engine)["attributes"]
                actual.pop("always_on_top", None)
                actual.pop("always_on_top_order", None)
                actual.update(attributes)
                with self.assertRaisesRegex(ValueError, "shared House facts differ"):
                    self.qualified(engine, self.npc)

    def test_binding_known_facts_and_extra_authoring_still_refuse(self):
        for field in ("binding", "physical", "authoring"):
            with self.subTest(field=field):
                engine = copy.deepcopy(self.engine)
                row = self.shared(engine)
                if field == "binding":
                    row["production_binding"]["target"]["revision"] = "unqualified"
                elif field == "physical":
                    row["semantics"]["physical"]["value"]["weight"]["value"] += 1
                else:
                    row["authoring"]["unqualified_extra"] = True
                qualified = self.qualified(engine, self.npc)
                with self.assertRaisesRegex(ValueError, "QENGINE/NPC shared source"):
                    self.generate_shared(*qualified)

    def test_missing_foreign_and_duplicate_shared_identities_refuse(self):
        for case in ("missing", "changed_identity", "foreign_shared", "duplicate_engine",
                     "duplicate_npc"):
            with self.subTest(case=case):
                engine, npc = copy.deepcopy((self.engine, self.npc))
                if case == "missing":
                    npc["records"].remove(self.shared(npc))
                elif case == "changed_identity":
                    self.shared(engine)["production_definition"]["production_key"] += "-unqualified"
                elif case == "foreign_shared":
                    extra = next(row for row in engine["records"]
                                 if row["production_definition"]["production_key"] != self.KEY)
                    npc["records"].append(copy.deepcopy(extra))
                elif case == "duplicate_engine":
                    engine["records"].append(copy.deepcopy(self.shared(engine)))
                else:
                    npc["records"].append(copy.deepcopy(self.shared(npc)))
                with self.assertRaisesRegex(ValueError, "shared source|duplicate production key"):
                    self.qualified(engine, npc)


class Srx3SourceDiagnosticsTests(unittest.TestCase):
    def fixture(self, *, missing=False):
        source = EquipmentPinTests().srx3_source()
        proof = document(2, 5)
        proof['records'][-1]['facts'] = {
            'nested': {'mass': 3600, 'unit': 'centiweight'},
            'unknown': None, 'flag': False, 'zero': 0,
        }
        base = document(0, 1, 2, 3) if missing else document(0, 1, 2, 3, 5)
        if not missing:
            base['records'][-1] = copy.deepcopy(proof['records'][-1])
            base['records'][-1]['facts']['nested']['mass'] = 3601
        return source, base, document(4), document(0), document(1), proof

    def refusal(self, inputs):
        source, base, updates, engine, npc, proof = inputs
        before = copy.deepcopy(inputs)
        with self.assertRaisesRegex(ValueError, 'SRX3 source profile differs: SRX3-TEST/5') as caught:
            regenerate(source, base, updates, engine, npc,
                       srx3=[('SRX3-TEST', proof, {'2'})])
        self.assertEqual(inputs, before)
        return str(caught.exception)

    def assert_diagnostic(self, message, expected, actual):
        prefix = 'SRX3 source profile differs: SRX3-TEST/5'
        self.assertTrue(message.startswith(prefix + ': '), message)
        diagnostic = {
            'cohort': 'SRX3-TEST', 'key': '5',
            'expected_source_row': expected, 'actual_row': actual,
        }
        self.assertEqual(message, prefix + ': ' + json.dumps(diagnostic, sort_keys=True))
        parsed = json.loads(message[len(prefix) + 2:])
        self.assertEqual(parsed, diagnostic)
        self.assertIs(parsed['expected_source_row']['facts']['flag'], False)
        self.assertIsNone(parsed['expected_source_row']['facts']['unknown'])
        self.assertIs(type(parsed['expected_source_row']['facts']['zero']), int)
        return parsed

    def test_changed_nested_fact_refuses_with_complete_deterministic_operands(self):
        inputs = self.fixture()
        message = self.refusal(inputs)
        self.assertEqual(self.refusal(inputs), message)
        parsed = self.assert_diagnostic(
            message, inputs[-1]['records'][-1], inputs[1]['records'][-1])
        self.assertEqual(parsed['expected_source_row']['facts']['nested']['mass'], 3600)
        self.assertEqual(parsed['actual_row']['facts']['nested']['mass'], 3601)
        self.assertIs(parsed['actual_row']['facts']['flag'], False)
        self.assertIs(type(parsed['actual_row']['facts']['zero']), int)

    def test_missing_actual_row_refuses_with_complete_source_and_null_actual(self):
        inputs = self.fixture(missing=True)
        message = self.refusal(inputs)
        self.assertEqual(self.refusal(inputs), message)
        parsed = self.assert_diagnostic(message, inputs[-1]['records'][-1], None)
        self.assertIsNone(parsed['actual_row'])


class BossYieldPriorOwnerTests(unittest.TestCase):
    KEY = 'oteryn:item.tibia.i7385'
    MAP_CARRIERS = {f'oteryn:item.tibia.i{i}' for i in (1777, 2436, 2438, 2473, 2474)}

    @classmethod
    def setUpClass(cls):
        import authored_static_item_constructors as authored
        import boss_reward_yield_profiles as yields
        import lion_trial_item_profiles as lion
        import quest_callback_profiles as callbacks
        cls.authored, cls.yields, cls.callbacks = authored, yields, callbacks
        cls.requests = callbacks._requested_inputs(refresh.ROOT)
        requested, authored_keys, authored_callbacks = cls.requests
        if cls.KEY not in requested or cls.KEY not in authored_keys:
            raise AssertionError('actual requested Trader source is required')
        cls.trader_callbacks = [row for row in authored_callbacks if cls.KEY in json.dumps(row)]
        generated = {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}',
                     authored.ADMISSIONS: (refresh.ROOT / authored.ADMISSIONS).read_bytes()}
        # Execute the ordinary requested branch. The disjoint Lion cohort is
        # excluded so this focused test never needs the appearance decoder.
        with patch.object(callbacks, '_requested_inputs',
                          return_value=({cls.KEY}, {cls.KEY}, authored_callbacks)), \
                patch.object(lion, 'records', return_value={}), \
                patch.object(yields.corpses.source.engine_items, 'load_appearance_objects',
                             side_effect=AssertionError('appearance decoding is outside this test')):
            callbacks.admit(refresh.ROOT, generated, 'profiles', 'manifest',
                            lambda row: row['production_definition']['production_key'],
                            yields.corpses.encode)
        cls.before = json.loads(generated['profiles'])
        if refresh.keys(cls.before) != cls.MAP_CARRIERS | {cls.KEY}:
            raise AssertionError('the real five map carriers and ordinary Trader row are required')
        yields.admit(refresh.ROOT, generated, 'profiles', 'manifest',
                     lambda row: row['production_definition']['production_key'],
                     yields.corpses.encode)
        cls.actual = refresh.registered_source_document(
            refresh.ROOT, json.loads(generated['profiles']))
        cls.selection = next(row for row in refresh.SRX3_SOURCES if row[0] == 'SRX3-BOSS-YIELDS')
        with patch.object(refresh, 'SRX3_SOURCES', (cls.selection,)):
            cls.cohort = refresh.source_cohorts(refresh.ROOT)[0]
        isolated = {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
        yields.admit(refresh.ROOT, isolated, 'profiles', 'manifest',
                     lambda row: row['production_definition']['production_key'],
                     yields.corpses.encode)
        cls.isolated = refresh.registered_source_document(
            refresh.ROOT, json.loads(isolated['profiles']))
        definitions = [(path, row['definition'])
                       for path in sorted((refresh.ROOT / 'content/items/definitions').glob('items-*.json'))
                       for row in json.loads(path.read_bytes())['records']
                       if row['definition']['identity']['key'] == cls.KEY]
        if len(definitions) != 1:
            raise AssertionError('one exact canonical Trader definition is required')
        cls.definition_path, cls.definition = definitions[0]
        cls.binding_path = refresh.ROOT / 'imports/crystalserver/bindings/items.json'
        cls.bindings = json.loads(cls.binding_path.read_bytes())
        cls.static_path = refresh.ROOT / authored.ADMISSIONS
        cls.static = json.loads(cls.static_path.read_bytes())
        all_bindings = cls.bindings['bindings'] + [row['binding'] for row in cls.static['records']]
        cls.matches = [binding for binding in all_bindings if binding['target'] == cls.definition['identity']]
        cls.xml, cls.source_items = callbacks.canonical_source_inputs(refresh.ROOT)

    @classmethod
    def row(cls, document):
        return next(row for row in document['records']
                    if row['production_definition']['production_key'] == cls.KEY)

    def generate(self, actual=None):
        source = EquipmentPinTests().srx3_source().replace(
            'ORIGINAL_PROFILES: usize = 1;', 'ORIGINAL_PROFILES: usize = 19;').replace(
            '("QNPCFX", 1)', '("QNPCFX", 0)').replace(
            'cohorts add 1 + 1 keys', 'cohorts add 0 + 0 keys')
        empty = {'records': []}
        return regenerate(source, actual or self.actual, empty, empty, empty,
                          srx3=[self.cohort])

    def test_actual_trader_yield_house_chain_keeps_the_complete_cohort_guard(self):
        name, expected, previous = self.cohort
        self.assertEqual(name, 'SRX3-BOSS-YIELDS')
        self.assertEqual(len(expected['records']), 46)
        self.assertEqual(len(previous), 14)
        self.assertNotIn(self.KEY, previous)
        self.assertEqual(refresh.keys(self.actual), refresh.keys(expected) | self.MAP_CARRIERS)
        actual = {row['production_definition']['production_key']: row for row in self.actual['records']}
        isolated = {row['production_definition']['production_key']: row for row in self.isolated['records']}
        changed = []
        for row in expected['records']:
            key = row['production_definition']['production_key']
            self.assertEqual(row, actual[key], key)
            if row != isolated[key]:
                changed.append(key)
        self.assertEqual(changed, [self.KEY])
        self.assertEqual(len(refresh.keys(expected) - {self.KEY}), 45)
        self.assertNotIn('taxonomy', self.row(expected)['authoring'])
        generated, count = self.generate()
        self.assertEqual(count, 51)  # Fixed historical14 + map5 + new32.
        empty = {'records': []}
        self.assertEqual(regenerate(generated, self.actual, empty, empty, empty,
                                    srx3=[self.cohort]), (generated, count))
        with patch.object(refresh, 'SRX3_SOURCES', (self.selection,)):
            self.assertEqual(refresh.source_cohorts(refresh.ROOT), [self.cohort])

    def test_shared_factory_matches_the_actual_ordinary_branch_without_mutation(self):
        before = copy.deepcopy((self.definition, self.matches))
        result = self.callbacks.canonical_native_profile(
            refresh.ROOT, self.definition, self.matches, self.source_items)
        self.assertEqual(result, self.row(self.before))
        self.assertEqual((self.definition, self.matches), before)
        self.assertEqual(result['authoring'], {'item': self.definition['identity']})
        self.assertEqual(result['attributes'], {'speed_bonus': 0})
        self.assertEqual(result['semantics'],
                         self.callbacks.grant_semantics(refresh.ROOT, self.definition))
        self.assertEqual(result['semantics']['imbuement']['value']['slot_count'],
                         {'state': 'KNOWN', 'value': 2})
        self.assertEqual(result['semantics']['trade_restrictions']['value']['marketable'],
                         {'state': 'KNOWN', 'value': True})
        self.assertEqual({row['key'] for row in self.trader_callbacks},
                         {'oteryn:quest-reward/chosen/trader/buy-sword',
                          'oteryn:quest-reward/chosen/trader/sword'})

    def test_requested_membership_and_unique_canonical_identity_are_required(self):
        requested, authored, callbacks = copy.deepcopy(self.requests)
        requested.remove(self.KEY)
        with patch.object(self.callbacks, '_requested_inputs',
                          return_value=(requested, authored, callbacks)):
            with self.assertRaisesRegex(ValueError, 'SRX3_BOSS_YIELD_TRADER_NOT_REQUESTED'):
                refresh.boss_yield_prior_record(refresh.ROOT)
        original = Path.read_bytes
        document = json.loads(self.definition_path.read_bytes())
        for mode in ('missing', 'duplicate', 'revision'):
            with self.subTest(mode=mode):
                changed = copy.deepcopy(document)
                entries = [row for row in changed['records']
                           if row['definition']['identity']['key'] == self.KEY]
                self.assertEqual(len(entries), 1)
                if mode == 'missing':
                    changed['records'].remove(entries[0])
                elif mode == 'duplicate':
                    changed['records'].append(copy.deepcopy(entries[0]))
                else:
                    entries[0]['definition']['identity']['revision'] = 'unqualified'
                raw = json.dumps(changed).encode()
                def read(path):
                    return raw if path == self.definition_path else original(path)
                with patch.object(Path, 'read_bytes', read), \
                        patch.object(self.callbacks, '_requested_inputs', return_value=self.requests):
                    with self.assertRaisesRegex(ValueError, 'SRX3_BOSS_YIELD_TRADER_CANONICAL_IDENTITY'):
                        refresh.boss_yield_prior_record(refresh.ROOT)

    def test_full_base_and_authored_binding_inventory_rejects_missing_or_duplicate(self):
        original = Path.read_bytes
        for mode in ('missing', 'duplicate_base', 'duplicate_authored'):
            with self.subTest(mode=mode):
                bindings, static = copy.deepcopy((self.bindings, self.static))
                selected = [row for row in bindings['bindings']
                            if row['target'] == self.definition['identity']]
                self.assertEqual(len(selected), 1)
                if mode == 'missing':
                    bindings['bindings'].remove(selected[0])
                elif mode == 'duplicate_base':
                    bindings['bindings'].append(copy.deepcopy(selected[0]))
                else:
                    static['records'].append({'binding': copy.deepcopy(selected[0])})
                overlays = {self.binding_path: json.dumps(bindings).encode(),
                            self.static_path: json.dumps(static).encode()}
                def read(path):
                    return overlays[path] if path in overlays else original(path)
                with patch.object(Path, 'read_bytes', read), \
                        patch.object(self.callbacks, '_requested_inputs', return_value=self.requests):
                    with self.assertRaisesRegex(ValueError, 'QUEST_CALLBACK_ITEM_NOT_ADMITTED'):
                        refresh.boss_yield_prior_record(refresh.ROOT)

    def test_factory_preserves_canonical_binding_and_portable_guards(self):
        for mode in ('not_materializable', 'unknown_stack', 'binding_disposition',
                     'binding_target', 'unknown_physical', 'immovable', 'not_pickupable',
                     'unknown_weight'):
            with self.subTest(mode=mode):
                definition, matches = copy.deepcopy((self.definition, self.matches))
                if mode == 'not_materializable':
                    definition['materializable'] = False
                elif mode == 'unknown_stack':
                    definition['stack_class'] = 'Unknown'
                elif mode == 'binding_disposition':
                    matches[0]['disposition'] = 'UNQUALIFIED'
                elif mode == 'binding_target':
                    matches[0]['target']['revision'] = 'unqualified'
                elif mode == 'unknown_physical':
                    definition['semantics']['physical'] = {'state': 'UNKNOWN'}
                elif mode == 'unknown_weight':
                    definition['semantics']['physical']['value']['weight'] = {'state': 'UNKNOWN'}
                else:
                    field = 'movable' if mode == 'immovable' else 'pickupable'
                    definition['semantics']['physical']['value'][field] = {'state': 'KNOWN', 'value': False}
                with self.assertRaisesRegex(ValueError, 'QUEST_CALLBACK_'):
                    self.callbacks.canonical_native_profile(
                        refresh.ROOT, definition, matches, self.source_items)

    def test_source_hash_unique_xml_and_duplicate_speed_guards_remain(self):
        import xml.etree.ElementTree as ET
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / self.callbacks.SOURCE
            target.parent.mkdir(parents=True)
            target.write_bytes(self.xml + b'\n')
            with self.assertRaisesRegex(ValueError, 'QUEST_CALLBACK_SOURCE_DIGEST'):
                self.callbacks.canonical_source_inputs(root)
        selected = [item for item in self.source_items
                    if int(item.get('id', item.get('fromid', '0'))) <= 7385
                    <= int(item.get('id', item.get('toid', '0')))]
        self.assertEqual(len(selected), 1)
        for values in ([], selected + selected):
            with self.assertRaisesRegex(ValueError, 'QUEST_CALLBACK_SOURCE_ITEM_MISSING'):
                self.callbacks.canonical_native_profile(
                    refresh.ROOT, self.definition, self.matches, values)
        duplicate = copy.deepcopy(selected[0])
        ET.SubElement(duplicate, 'attribute', {'key': 'speed', 'value': '0'})
        ET.SubElement(duplicate, 'attribute', {'key': 'speed', 'value': '1'})
        with self.assertRaisesRegex(ValueError, 'QUEST_CALLBACK_DUPLICATE_SPEED'):
            self.callbacks.canonical_native_profile(
                refresh.ROOT, self.definition, self.matches, [duplicate])

    def test_all_remaining_actual_row_disagreements_still_reach_full_guard(self):
        for part in ('authoring', 'attributes', 'production_binding', 'admission', 'semantics'):
            with self.subTest(part=part):
                actual = copy.deepcopy(self.actual)
                self.row(actual)[part]['unqualified_extra'] = True
                with self.assertRaisesRegex(ValueError,
                        'SRX3 source profile differs: SRX3-BOSS-YIELDS/oteryn:item.tibia.i7385'):
                    self.generate(actual)



if __name__ == "__main__":
    unittest.main()
