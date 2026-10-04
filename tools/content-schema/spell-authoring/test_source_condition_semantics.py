import gzip
import json
from pathlib import Path
import unittest
import source_condition_semantics as exporter

ROOT = Path(__file__).resolve().parents[3]


class ConditionSemanticsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.records = [json.loads(line) for line in gzip.decompress((ROOT/'docs/reference/spells/r31-source-closure/source-condition-templates.jsonl.gz').read_bytes()).splitlines()]
        cls.enums, cls.proofs = exporter.source_rules('canary')

    def record(self, path, source='canary'):
        return next(r for r in self.records if r['source']==source and path in r['path'])

    def test_indefinite_is_not_serialization_persistence(self):
        rows=[exporter.semantics(r,self.enums) for r in self.records if 'virtue_of_justice.lua' in r['path']]
        self.assertEqual(len(rows),2)
        for row in rows:
            self.assertEqual(row['lifetime']['mode'],'indefinite_no_countdown')
            self.assertEqual(row['lifetime']['source_ticks']['value'],-1)
            self.assertEqual(row['lifetime']['serialization_persistence'],'false_under_default_constructor')
            self.assertEqual(row['lifetime']['removable_on_death'],'false_under_default_constructor')
            self.assertEqual(row['prefix_sub_id'],{'kind':'source_symbol','value':'AttrSubId_VirtueOfJustice'})
            self.assertFalse(row['runtime_activation'])

    def test_explicit_constructor_identity_subid_not_overwritten_by_defaults(self):
        row=exporter.semantics(self.record('divine_grenade.lua'),self.enums)
        self.assertEqual(row['constructor_condition_id'],{'namespace':'ConditionId_t','symbol':'CONDITIONID_DEFAULT','numeric_value':-1})
        self.assertEqual(row['constructor_sub_id'],{'kind':'scalar','value':258})
        self.assertEqual(row['prefix_sub_id'],row['constructor_sub_id'])

    def test_unregistered_global_is_not_resolved_from_unrelated_enum(self):
        row=exporter.semantics(self.record('mentor_other.lua'),self.enums)
        self.assertEqual(row['prefix_sub_id'],{'kind':'source_symbol','value':'MentorOther'})
        self.assertEqual(row['lifetime']['source_ticks'],{'kind':'source_symbol','value':'conditionTicks'})
        self.assertEqual(row['lifetime']['mode'],'unresolved_or_initial_zero')

    def test_later_tick_assignment_is_preserved_without_claiming_applied_duration(self):
        row=exporter.semantics(self.record('avatar_of_balance.lua'),self.enums)
        ticks=[p for p in row['parameter_assignments'] if p['parameter']['symbol']=='CONDITION_PARAM_TICKS']
        self.assertTrue(ticks)
        self.assertTrue(all(not p['declaration_prefix'] for p in ticks))
        self.assertEqual(row['lifetime']['mode'],'unresolved_or_initial_zero')

    def test_all_records_strict_schema_and_enum_namespaces(self):
        from jsonschema import Draft202012Validator
        schema=json.loads((Path(__file__).parent/'source-condition-semantics.schema.json').read_text())
        validator=Draft202012Validator(schema)
        bysource={'canary':self.enums,'crystal':exporter.source_rules('crystal')[0]}
        for record in self.records:
            row=exporter.semantics(record,bysource[record['source']])
            validator.validate(row)
            self.assertEqual(row['condition_type']['namespace'],'ConditionType_t')
            self.assertTrue(all(p['parameter']['namespace']=='ConditionParam_t' for p in row['parameter_assignments']))
        self.assertEqual(len(self.records),59)
        with self.assertRaises(ValueError): exporter.enum_value('ConditionId_t','MentorOther',self.enums)


if __name__=='__main__': unittest.main()
