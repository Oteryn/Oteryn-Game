import copy
import os
from pathlib import Path
import sys
import tempfile
import tarfile
import unittest
import builder

ROOT = Path(os.environ.get('QUEST_COMPONENT_REPO_ROOT', str(Path(__file__).resolve().parents[6])))
ASSIGNMENT = Path(os.environ.get('QUEST_COMPONENT_ASSIGNMENT', str(ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/components248/assignments/all.json')))



class ParserControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        sys.path.insert(0, str(ROOT / 'tools/content-schema/quest-authoring'))
        import lua_writers
        cls.mask = staticmethod(lua_writers.mask_code)

    def test_literal_monster_call_only(self):
        text = 'Game.createMonster("Leiden",p)\nGame.createMonster(config.name,p)'
        self.assertEqual([m.group(2) for m in builder.literal_calls(text, self.mask)], ['Leiden'])

    def test_quotes_and_comments_not_calls(self):
        text = 'local s=\'Game.createMonster("Fake",p)\'\n-- Game.createMonster("Comment",p)'
        self.assertEqual(builder.literal_calls(text, self.mask), [])

    def test_registered_monster_event_names(self):
        text = 'local mType=Game.createMonsterType("Leiden");local monster={};monster.events={"LeidenHeal","Second",};mType:register(monster)'
        result = builder.monster_registration(text, self.mask)
        self.assertEqual(result[0][0], 'Leiden')
        self.assertEqual(result[0][4], ['LeidenHeal', 'Second'])

    def test_dynamic_event_table_remains_unresolved(self):
        text = 'local mType=Game.createMonsterType("Leiden");local monster={};monster.events={dynamic};mType:register(monster)'
        result = builder.monster_registration(text, self.mask)
        self.assertEqual(result[0][4], [])

    def test_missing_event_table_still_counts_as_monster_definition(self):
        text = 'local mType=Game.createMonsterType("Duplicate");local monster={};mType:register(monster)'
        result = builder.monster_registration(text, self.mask)
        self.assertEqual(result[0][0], 'Duplicate')
        self.assertEqual(result[0][4], [])

    def test_table_shadow_and_rebind_rejected(self):
        for replacement in ['local monster={}', 'monster={}']:
            text = 'local m=Game.createMonsterType("Boss");local monster={};monster.events={"A"};' + replacement + ';m:register(monster)'
            self.assertEqual(builder.monster_registration(text, self.mask), [])

    def test_events_after_register_or_mutated_rejected(self):
        for middle in ['m:register(monster);monster.events={"A"}', 'monster.events={"A"};monster.events[1]="B";m:register(monster)', 'monster.events={"A"};monster.events={"B"};m:register(monster)', 'monster.events={"A"};monster.events=nil;m:register(monster)', 'monster.events={"A"};local alias=monster.events;alias[1]="B";m:register(monster)']:
            text = 'local m=Game.createMonsterType("Boss");local monster={};' + middle
            self.assertFalse(any(item[4] for item in builder.monster_registration(text, self.mask)))

    def test_factory_shadow_and_table_escape_rejected(self):
        for middle in ['local m=other;', 'mutate(monster);', 'local alias=monster;', 'monster,other={},{ };']:
            text = 'local m=Game.createMonsterType("Boss");local monster={};monster.events={"A"};' + middle + 'm:register(monster)'
            self.assertEqual(builder.monster_registration(text, self.mask), [])

    def test_nested_table_or_events_scope_rejected(self):
        for text in ['local m=Game.createMonsterType("Boss");function f() local monster={} end;monster.events={"A"};m:register(monster)', 'local m=Game.createMonsterType("Boss");local monster={};function f() monster.events={"A"} end;m:register(monster)', 'local m=Game.createMonsterType("Boss");local monster={};do monster.events={"A"} end;m:register(monster)']:
            self.assertFalse(any(item[4] for item in builder.monster_registration(text, self.mask)))

    def test_events_before_table_declaration_rejected(self):
        text = 'local m=Game.createMonsterType("Boss");monster.events={"A"};local monster={};m:register(monster)'
        self.assertFalse(any(item[4] for item in builder.monster_registration(text, self.mask)))

    def test_shadowed_registered_table_not_used(self):
        text = 'local mType=Game.createMonsterType("Boss");monster.events={"Event"};mType:register(other)'
        self.assertFalse(any(item[4] for item in builder.monster_registration(text, self.mask)))


class PacketControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.temp.cleanup)
        directory = Path(cls.temp.name)
        with tarfile.open(ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as stream:
            stream.extractall(directory, filter='data')
        manifest = directory / 'corpus-manifest.json'
        cls.packet = builder.build(ROOT, ASSIGNMENT, manifest)
        cls.base = builder.module(ROOT)
        cls.corpus = cls.base.Corpus(manifest)
        cls.previous = cls.base.build(ROOT, ASSIGNMENT, manifest)

    def test_before_after_and_full_membership(self):
        self.assertEqual(self.packet['summary']['components'], 248)
        self.assertEqual(self.packet['summary']['joined_components_before'], 17)
        self.assertEqual(self.packet['summary']['joined_components'], 37)
        self.assertEqual(self.packet['summary']['new_joined_components'], 20)
        self.assertEqual(self.packet['summary']['joined_quests'], 9)
        self.assertEqual({r['source_component_id'] for r in self.packet['records']}, {r['source_component_id'] for r in self.previous['records']})

    def test_previous_joins_preserved(self):
        before = {r['source_component_id']: set(r['quest_keys']) for r in self.previous['records']}
        for row in self.packet['records']:
            self.assertTrue(before[row['source_component_id']] <= set(row['quest_keys']))

    def test_all_chain_witnesses_and_pack_fences(self):
        for row in self.packet['records']:
            for proof in row['proofs']:
                for witness in proof['source_witnesses']:
                    self.assertEqual(witness['source'], row['provenance']['source'])
                    self.assertEqual(witness['revision'], row['provenance']['revision'])
                    self.assertEqual(witness['path'].split('/')[0], row['provenance']['path'].split('/')[0])
                    f = self.corpus.files[(witness['source'], witness['revision'], witness['path'])]
                    text = self.corpus.text(f)
                    self.assertEqual(self.base.sha(text[witness['char_start']:witness['char_end']].encode()), witness['token_sha256'])
                    self.assertEqual(self.base.sha(text.splitlines()[witness['line'] - 1].encode()), witness['line_sha256'])

    def test_normalization_uses_pinned_cpp_registry(self):
        for row in self.packet['records']:
            for proof in row['proofs']:
                if proof['basis'] == 'PINNED_LITERAL_MONSTER_CREATION_TO_REGISTERED_EVENT_REFERENCE':
                    self.assertEqual(len(proof['normalization_source_witnesses']), 2)
                    for witness in proof['normalization_source_witnesses']:
                        self.assertEqual(witness['source'], row['provenance']['source'])
                        self.assertEqual(witness['revision'], row['provenance']['revision'])
                        self.assertEqual(witness['path'], 'src/creatures/monsters/monsters.cpp')
                        text = self.corpus.text(self.corpus.files[(witness['source'], witness['revision'], witness['path'])])
                        self.assertEqual(self.base.sha(text[witness['char_start']:witness['char_end']].encode()), witness['token_sha256'])

    def test_depth_two_is_not_direct_or_runtime(self):
        deep = [p for r in self.packet['records'] for p in r['proofs'] if p['reference_chain_depth'] == 2]
        self.assertTrue(deep)
        self.assertTrue(all(p['dispatch_binding'] == 'NOT_PROVEN' for p in deep))
        self.assertTrue(all(len(p['source_witnesses']) >= 12 for p in deep))

    def test_no_unjoined_role_becomes_a_missing_quest(self):
        self.assertEqual(self.packet['summary']['association_scopes']['WORLD_EVENT_WITHOUT_PROVEN_QUEST_OWNER'], 32)
        self.assertEqual(self.packet['summary']['association_scopes']['SHARED_FACTORY_INSTANCE_WITHOUT_PROVEN_QUEST_OWNER'], 81)
        for row in self.packet['records']:
            self.assertFalse(row['native_admission'])
            if not row['quest_keys']:
                self.assertEqual(row['closed_mapping_gaps'], [])
                self.assertIn('EXACT_QUEST_ASSOCIATION_NOT_ESTABLISHED', row['remaining_holds'])

    def test_closed_schema_and_native_negatives(self):
        import jsonschema
        schema = self.base.read(Path(__file__).with_name('schema.json'))
        validator = jsonschema.Draft202012Validator(schema)
        validator.validate(self.packet)
        for change in [{'native_admission': True}, {'invented_owner': 'q'}, {'quest_semantic_completeness': 'COMPLETE'}]:
            altered = dict(self.packet, **change)
            with self.assertRaises(jsonschema.ValidationError):
                validator.validate(altered)


if __name__ == '__main__':
    unittest.main()
