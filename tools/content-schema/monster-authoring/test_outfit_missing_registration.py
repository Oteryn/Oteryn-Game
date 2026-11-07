"""Closed pinned registry evidence removes only Chayenne's failing lazy outfit."""
import copy
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import canary_batch as cb
from validate_monster import validate

SOURCE = Path(os.environ.get('OTERYN_CANARY', '/workspace/monster-reference-sources/canary'))
COMPLETE = Path(os.environ.get('OTERYN_CANARY_COMPLETE_SOURCE_TREE',
    '/workspace/monster-round6-output/reference-dependencies/canary-complete-source'))
PRIMAL = 'data-otservbr-global/lib/quests/the_primal_ordeal.lua'
ENGINE = ('config.lua.dist', 'src/canary_server.cpp', 'src/lua/scripts/scripts.cpp',
    'src/lua/functions/core/game/game_functions.cpp', 'src/creatures/monsters/monsters.cpp',
    'src/creatures/combat/condition.cpp', 'src/creatures/combat/combat.cpp')


class ClosedRegistry(unittest.TestCase):
    def test_exact_case_fold_not_filename_or_plural_guess(self):
        names, count = cb.literal_active_monster_names({cb.MONSTER_DIR+'/misleading.lua':
            'local mType = Game.createMonsterType("Dragon Hatchling")\nlocal monster = {}'})
        self.assertEqual(({'dragon hatchling'}, 1), (names, count))
        self.assertNotIn('dragon hatchlings', names); self.assertNotIn('misleading', names)

    def test_disabled_basename_skips_even_dynamic_creator(self):
        self.assertEqual((set(), 0), cb.literal_active_monster_names({
            cb.MONSTER_DIR+'/#disabled.lua': 'Game.createMonsterType(dynamicName)'}))

    def test_alias_variant_alternate_buffered_dynamic_and_scope_fail_closed(self):
        for path, text in [
            (cb.MONSTER_DIR+'/x.lua', 'local x = Game.createMonsterType\nx("Devovorga")'),
            (cb.MONSTER_DIR+'/x.lua', 'local x = Game\nx.createMonsterType("Devovorga")'),
            (cb.MONSTER_DIR+'/x.lua', 'local mType = Game.createMonsterType("X", "", "Devovorga")'),
            (cb.MONSTER_DIR+'/x.lua', 'local mType = Game.createMonsterType("X", "!Devovorga")'),
            (cb.MONSTER_DIR+'/x.lua', 'local function later()\nGame.createMonsterType("Devovorga")\nend'),
            (cb.MONSTER_DIR+'/x.lua', 'local mType = Game.createMonsterType(name)'),
            ('data/scripts/x.lua', 'local mType = Game.createMonsterType("Devovorga")'),
            (cb.MONSTER_DIR+'/lib/x.lua', 'local mType = Game.createMonsterType("Devovorga")'),
        ]:
            with self.subTest(text=text), self.assertRaises(ValueError):
                cb.literal_active_monster_names({path: text})

    def test_only_pinned_primal_suffix_excluded(self):
        text = (COMPLETE/PRIMAL).read_text()
        self.assertEqual((set(), 0), cb.literal_active_monster_names({PRIMAL: text}))
        with self.assertRaises(ValueError):
            cb.literal_active_monster_names({PRIMAL: text.replace(' (Primal)', '')})

    def fixture(self, registration='local mType = Game.createMonsterType("Chayenne")'):
        temp = tempfile.TemporaryDirectory(); self.addCleanup(temp.cleanup); root = Path(temp.name)
        records = {p: (COMPLETE/p).read_bytes() for p in (*ENGINE, PRIMAL)}
        records[cb.MONSTER_DIR+'/fixture.lua'] = registration.encode()
        for p, data in records.items():
            path = root/p; path.parent.mkdir(parents=True, exist_ok=True); path.write_bytes(data)
        listing = b''.join(('100644 blob '+cb.blob_id(data)+'\t'+p).encode()+b'\0'
                           for p, data in sorted(records.items()))
        def git(args, **kwargs):
            if args[3:5] == ['rev-parse', 'HEAD']: return cb.REVISION+'\n'
            if args[3] == 'status': return ''
            if args[3] == 'ls-tree': return listing
            self.fail('must not fetch/write Git objects: '+str(args))
        return root, git

    def proof(self, root, git):
        with patch.dict(os.environ, {'OTERYN_CANARY_COMPLETE_SOURCE_TREE': str(root)}), \
             patch.object(cb.subprocess, 'check_output', side_effect=git):
            return cb.qualified_missing_outfit_source(root, 'Devovorga')

    def test_registered_future_target_and_untrusted_dynamic_registry(self):
        for creation in ('local mType = Game.createMonsterType("Devovorga")',
                         'local mType = Game.createMonsterType(dynamicName)'):
            root, git = self.fixture(creation); self.assertIsNone(self.proof(root, git))

    def test_missing_changed_config_dirty_and_wrong_pin_fail_closed(self):
        for mutation in ('missing', 'changed', 'config', 'dirty', 'revision'):
            root, git = self.fixture()
            if mutation == 'missing': (root/ENGINE[-1]).unlink()
            if mutation == 'changed': (root/ENGINE[-1]).write_text('untrusted combat')
            if mutation == 'config': (root/'config.lua').write_text('dataPackDirectory="custom"')
            def changed_git(args, **kwargs):
                if mutation == 'dirty' and args[3] == 'status': return '?? data/scripts/new.lua\n'
                if mutation == 'revision' and args[3] == 'rev-parse': return '0'*40+'\n'
                return git(args, **kwargs)
            with self.subTest(mutation=mutation): self.assertIsNone(self.proof(root, changed_git))

    def test_complete_actual_tree_loader_registry_condition_and_visual_order(self):
        with patch.dict(os.environ, {'OTERYN_CANARY_COMPLETE_SOURCE_TREE': str(COMPLETE)}):
            proof = cb.qualified_missing_outfit_source(SOURCE, 'Devovorga')
        self.assertIsNotNone(proof)
        self.assertEqual(1655, proof['active_literal_creation_calls'])
        self.assertEqual(5278, proof['verified_source_files'])
        self.assertFalse(proof['target_registered'])
        self.assertEqual(set(ENGINE), set(proof['engine_git_blobs']))
        condition = (COMPLETE/'src/creatures/combat/condition.cpp').read_text()
        start = condition.split('bool ConditionOutfit::startCondition(', 1)[1].split('\nbool ', 1)[0]
        failure = start.index('Monster {} does not exist')
        self.assertLess(start.index('return false;', failure), start.index('Condition::startCondition(creature)'))
        combat = (COMPLETE/'src/creatures/combat/combat.cpp').read_text()
        body = combat.split('void Combat::doCombatCondition(const std::shared_ptr<Creature> &caster, const std::shared_ptr<Creature> &target', 1)[1].split('\nvoid ', 1)[0]
        self.assertLess(body.index('Combat::sendCombatEffect('), body.index('CombatConditionFunc('))


class ChayennePayload(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        objects = cb.load_appearance_objects(SOURCE/'data/items/appearances.dat')
        items = cb.load_items_xml(SOURCE/'data/items/items.xml')
        names, index = cb.name_index(objects, items)
        cls.converter = cb.Converter(SOURCE, objects, items, names, index)

    def convert(self, qualified):
        self.converter.pending_definitions = set()
        context = patch.dict(os.environ, {'OTERYN_CANARY_COMPLETE_SOURCE_TREE': str(COMPLETE)}) if qualified \
            else patch.object(cb, 'qualified_missing_outfit_source', return_value=None)
        with context: return self.converter.convert('raids/chayenne')

    def test_failed_payload_only_schedule_visual_and_normal_outfits_unchanged(self):
        original, corrected = self.convert(False), self.convert(True)
        self.assertEqual(original[1], corrected[1]); self.assertEqual(original[3], corrected[3])
        old, new = copy.deepcopy(original[2]), copy.deepcopy(corrected[2])
        old_effect = next(e for e in old['effects'] if e['identity']['key'].endswith('/defense-3'))
        new_effect = next(e for e in new['effects'] if e['identity']['key'].endswith('/defense-3'))
        self.assertEqual('appearance_transform', old_effect['operation'])
        self.assertEqual('canary:creature/devovorga', old_effect['appearance_transform']['creature']['key'])
        self.assertEqual({'identity': old_effect['identity'], 'operation': 'presentation_only',
                         'presentation': old_effect['presentation']}, new_effect)
        self.assertEqual('canary.appearance:effect/energyhit', new_effect['presentation']['impact_asset_binding'])
        old['effects'].remove(old_effect); new['effects'].remove(new_effect)
        self.assertEqual(old, new); self.assertEqual([], validate(*corrected[1:5]))
        self.assertNotIn(('Creature', 'canary:creature/devovorga'), self.converter.pending_definitions)
        self.assertTrue(any(e['operation']=='appearance_transform' and
                            e['appearance_transform']['creature']['key']=='canary:creature/chayenne'
                            for e in new['effects']))
        old_entries = {e['source_field']: e for e in original[4]['entries']}
        changes = [e for e in corrected[4]['entries'] if e != old_entries[e['source_field']]]
        self.assertEqual(['defenses[3]'], [e['source_field'] for e in changes])
        self.assertIn('PINNED_COMPLETE_DEFAULT_SOURCE_TREE_REGISTRY', changes[0]['resolution'])

    def test_unqualified_registry_preserves_real_dependency(self):
        result = self.convert(False)
        effect = next(e for e in result[2]['effects'] if e['identity']['key'].endswith('/defense-3'))
        self.assertEqual('appearance_transform', effect['operation']); self.assertEqual(10000, effect['duration_ms'])
        self.assertIn(('Creature', 'canary:creature/devovorga'), self.converter.pending_definitions)


if __name__ == '__main__':
    unittest.main()
