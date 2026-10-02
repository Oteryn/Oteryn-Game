import copy
import json
from pathlib import Path
import unittest
from jsonschema import ValidationError
import wiki_requirement_interpretations as module


class RequirementSyntaxTests(unittest.TestCase):
    def setUp(self):
        self.w = {'line': 1, 'line_sha256': 'a'*64, 'byte_offset': 0, 'byte_length': 20, 'span_sha256': 'a'*64}
    def parse(self, text): return module.parse_requirement(text, self.w, 'Requirements')
    def test_literal_level_is_source_only(self):
        p=self.parse('* Level 20 or more.');n=p['prerequisite_expressions'][0]
        self.assertEqual(n['nodes'][0]['value'],20)
        self.assertEqual(n['execution_semantics'],'UNKNOWN')
        for text in ['* Some locations require level 250 or higher.', '* Level 20 recommended.', '* Level 20?', '* Level 20-40.', '* Level 20 or more unless X.']:
            with self.subTest(text=text): self.assertIsNone(self.parse(text))
    def test_achievement_templates_require_whole_declaration(self):
        p=self.parse('* Ter os achievements {{Achievement|Test One}} e {{Achievement|Test Two}}.')
        self.assertEqual([n['entity'] for n in p['prerequisite_expressions'][0]['nodes']],['Test One','Test Two'])
        for text in ['* Ter os achievements {{Achievement| }}.', '* Ter os achievements {{Achievement|Test|extra}}.', '* Ter os achievements {{Achievement|Test}} se X.', '* {{Unknown|Test}}']:
            with self.subTest(text=text): self.assertIsNone(self.parse(text))
    def test_scopes_and_currency_are_facts_not_gates(self):
        p=self.parse("* For the ''optional mission'' '''Test Mission'''")
        self.assertEqual(p['prerequisite_expressions'],[])
        self.assertEqual(p['source_fact_entries'][0]['facts'],{'optional_mission_heading':'Test Mission'})
        p=self.parse('* 35,000,000 gold coins for donations.')
        self.assertEqual(p['source_fact_entries'][0]['facts']['required_amount_literal'],35000000)
        self.assertEqual(p['prerequisite_expressions'],[])
        for text in ["* For the ''optional mission'' '''X''' and level 20", '* about 500 gold coins for donations.', '* Dinheiro para viagens', '* [[Arquivo:Sword.gif]] uma arma física', '* All previous bosses defeated']:
            with self.subTest(text=text): self.assertIsNone(self.parse(text))
    def test_cumulative_threshold_not_sum_and_vague_unknown_retained(self):
        p=self.parse('* Os pontos de War Exp são acumulativos. Isso quer dizer que, para conseguir o primeiro addon você precisa de 500 pontos (não 300 + 500).')
        f=p['source_fact_entries'][0]['facts'];self.assertEqual(f['required_amount_literal'],500)
        self.assertEqual(f['excluded_additive_terms'],[300,500])
        self.assertTrue(f['cumulative_declared'])
        self.assertIsNone(self.parse('* Suprimentos.'))


class InterpretationIdentityTests(unittest.TestCase):
    def setUp(self):
        samples=Path(__file__).parent/'samples/wiki-requirements'
        self.a=json.loads((samples/'interpretations-input.json').read_text())
        self.s=json.loads((samples/'interpretation-input.schema.json').read_text())
        self.d=json.loads((samples/'test-baseline-holds.json').read_text())
        self.r=json.loads((samples/'interpretation-receipt.json').read_text())
        self.r['specifications_semantic_sha256']=module.digest(self.d)
    def run_build(self): return module.build(self.d,self.a,self.r,self.s)
    def test_historical_counts_preserved_and_current_projection_separate(self):
        before=copy.deepcopy(self.d);p=self.run_build()
        self.assertEqual((p['historical_unparsed_holds'],p['interpreted_holds'],p['current_unparsed_holds']),(205,18,187))
        self.assertEqual(before,self.d);self.assertFalse(p['definition_complete']);self.assertFalse(p['raw_body_rechecked'])
    def test_pins_schema_and_payload_tampering_rejected(self):
        for mutate in [lambda:self.a[0].update(execution_semantics='READY'),lambda:self.a[0]['historical_evidence'].update(revid=999),lambda:self.a.append(self.a[0]),lambda:self.s.update(additionalProperties=True)]:
            saved=copy.deepcopy((self.a,self.r,self.s));mutate()
            self.r['authored_interpretations_sha256']=module.digest(self.a)
            self.r['interpretation_record_digests']=sorted(module.digest(x) for x in self.a)
            with self.assertRaises((ValueError,ValidationError)):self.run_build()
            self.a,self.r,self.s=saved


if __name__ == '__main__': unittest.main()
