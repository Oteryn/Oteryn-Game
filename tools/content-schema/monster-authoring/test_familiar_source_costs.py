"""D16 special Familiar costs and D15 generic eligibility remain separate."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import canary_batch as cb
from validate_monster import validate

SOURCE = Path('/workspace/monster-reference-sources/canary')
COSTS = {'knight': 1000, 'druid': 3000, 'paladin': 2000, 'sorcerer': 3000, 'monk': 1500}


class FamiliarSourceCosts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        objects = cb.load_appearance_objects(SOURCE/'data/items/appearances.dat')
        items = cb.load_items_xml(SOURCE/'data/items/items.xml')
        names, index = cb.name_index(objects, items)
        cls.converter = cb.Converter(SOURCE, objects, items, names, index)
        cls.wiki = {m['monster']: m for m in json.loads((cb.ROOT/'samples/wiki-population-2026-09-27.json').read_text())['monsters']}

    def convert(self, vocation, wiki=False):
        self.converter.pending_definitions = set()
        self.converter.wiki = self.wiki if wiki else {}
        return self.converter.convert('familiars/'+vocation+'_familiar')

    def assert_existing_validation(self, result, vocation):
        # Bare Converter has two existing custom attack blockers; preserve them exactly.
        expected = ([f'manifest: unresolved_semantics {cb.MONSTER_DIR}/familiars/{vocation}_familiar.lua:72 attacks[4]']
                    if vocation in ('druid', 'sorcerer') else [])
        self.assertEqual(validate(*result[1:5]), expected)

    def test_all_five_actual_registered_spell_costs_and_exact_manifest_proof(self):
        for vocation, expected in COSTS.items():
            with self.subTest(vocation=vocation):
                result = self.convert(vocation)
                summoning = result[1]['creature']['summoning']
                self.assertEqual(expected, summoning['familiar']['mana_cost'])
                self.assertFalse(summoning['summonable']); self.assertFalse(summoning['convinceable'])
                self.assertNotIn('mana_cost', summoning)
                proof, = [r for r in result[4]['entries'] if r['source_field']=='spell:mana()']
                spell = SOURCE/proof['source_file']; text = spell.read_text()
                self.assertEqual(f'spell:mana({expected})', text.splitlines()[proof['source_line']-1])
                self.assertEqual('/monster/creature/summoning/familiar/mana_cost', proof['destination'])
                self.assertEqual({'repository': cb.REPOSITORY, 'revision': cb.REVISION}, result[4]['sources'][proof['source_index']])
                self.assertIn(hashlib.sha256(spell.read_bytes()).hexdigest(), proof['resolution'])
                self.assertIn(cb.blob_id(spell.read_bytes()), proof['resolution'])
                self.assert_existing_validation(result, vocation)
        self.assertEqual(2000, cb.load_monster(SOURCE/cb.MONSTER_DIR/'familiars/monk_familiar.lua')[1]['manaCost'])
        self.assertEqual(1500, self.convert('monk')[1]['creature']['summoning']['familiar']['mana_cost'])

    def test_real_five_wiki_observations_do_not_enable_generic_summon_or_rewrite_special_cost(self):
        for vocation, expected in COSTS.items():
            with self.subTest(vocation=vocation):
                result = self.convert(vocation, wiki=True)
                summoning = result[1]['creature']['summoning']
                self.assertFalse(summoning['summonable']); self.assertFalse(summoning['convinceable'])
                self.assertNotIn('mana_cost', summoning)
                self.assertEqual(expected, summoning['familiar']['mana_cost'])
                observed, = [r for r in result[4]['entries'] if r['source_field']=='Infobox Creature.summon']
                self.assertEqual('metadata_only', observed['status']); self.assertNotIn('destination', observed)
                raw, = [r for r in result[4]['entries'] if r['source_field']=='manaCost']
                self.assertEqual('approved_omission', raw['status']); self.assertNotIn('destination', raw)
                self.assertIn('Raw monster manaCost=', raw['resolution'])
                self.assert_existing_validation(result, vocation)

    def test_familiar_wiki_cost_cannot_mutate_existing_generic_flags_or_cost(self):
        creature = {'summoning': {'is_familiar': True, 'summonable': False, 'convinceable': True,
                                 'mana_cost': 777, 'familiar': {'mana_cost': 3000}}}
        original = copy.deepcopy(creature); observations = []
        record = {'rows': [{'status': 'DIFF', 'field': field, 'wiki': value}
                          for field, value in (('summon_mana_cost', 9999), ('convince_mana_cost', '--'))]}
        cb.Converter.adopt_wiki_summoning(record, creature, lambda *_: self.fail('generic adoption'),
                                         lambda *_: self.fail('generic wiki mapping'), lambda *args: observations.append(args))
        self.assertEqual(original, creature); self.assertEqual(2, len(observations))

    def test_ordinary_creature_generic_summoning_still_adopts_d15(self):
        creature = {'summoning': {'is_familiar': False, 'summonable': False, 'convinceable': False}}
        record = {'rows': [{'status': 'DIFF', 'field': 'summon_mana_cost', 'wiki': 600}]}
        cb.Converter.adopt_wiki_summoning(record, creature, lambda *_: None, lambda *_: None)
        self.assertTrue(creature['summoning']['summonable']); self.assertEqual(600, creature['summoning']['mana_cost'])

    def fixture(self, data):
        temporary = tempfile.TemporaryDirectory(); self.addCleanup(temporary.cleanup)
        root = Path(temporary.name); target = root/'data/scripts/spells/familiar/druid_familiar.lua'
        target.parent.mkdir(parents=True); target.write_bytes(data)
        return root

    def test_modified_spell_is_refused_against_pinned_git_object(self):
        original = (SOURCE/'data/scripts/spells/familiar/druid_familiar.lua').read_bytes()
        root = self.fixture(original.replace(b'spell:mana(3000)', b'spell:mana(9999)'))
        with patch.object(cb.subprocess, 'check_output', return_value=original):
            with self.assertRaisesRegex(ValueError, 'differs from pinned'): cb.familiar_spell_profile(root, 'druid')

    def test_wrong_vocation_registration_missing_duplicate_or_nonliteral_mana_fail_closed(self):
        original = (SOURCE/'data/scripts/spells/familiar/druid_familiar.lua').read_bytes()
        for old, new in ((b'"Summon Druid Familiar"', b'"Summon Knight Familiar"'),
                         (b'"druid;true"', b'"knight;true"'), (b'spell:register()', b''),
                         (b'spell:mana(3000)', b'spell:mana(3000+500)'),
                         (b'spell:mana(3000)', b'spell:mana(3000)\nspell:mana(9999)')):
            changed = original.replace(old, new); root = self.fixture(changed)
            with self.subTest(new=new), patch.object(cb.subprocess, 'check_output', return_value=changed):
                with self.assertRaises(ValueError): cb.familiar_spell_profile(root, 'druid')

    def test_unqualified_special_spell_blocks_carrier_without_default_mana(self):
        with patch.object(cb, 'familiar_spell_profile', side_effect=ValueError('unqualified source')):
            result = self.convert('druid', wiki=True)
        summoning = result[1]['creature']['summoning']
        self.assertNotIn('familiar', summoning); self.assertNotIn('mana_cost', summoning)
        self.assertFalse(summoning['summonable'])
        self.assertTrue(any(r['status']=='unresolved_dependency' and r['source_field']=='flags.familiar'
                            for r in result[4]['entries']))
        self.assertTrue(validate(*result[1:5]))


if __name__ == '__main__':
    unittest.main()
