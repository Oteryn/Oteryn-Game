import copy
import gzip
import json
import os
from pathlib import Path
import sys
import unittest
import builder

ROOT=Path(os.environ.get('QUEST_COMPONENT_REPO_ROOT','/workspace/quest-data-completion-80-worktree'))
ASSIGNMENT=Path(os.environ.get('QUEST_COMPONENT_ASSIGNMENT',str(ROOT/'tools/content-schema/quest-authoring/samples/donor-source/components248/assignments/all.json')))
MANIFEST=Path(os.environ.get('QUEST_COMPONENT_CORPUS_MANIFEST','/workspace/quest-donor-first/evidence/corpus-manifest.json'))


def fake_controller(ctor, method):
    text=ctor+'\n'+method
    value={'kind':'AnonymousFunction','raw_expression':ctor,'span':{'start_char':0,'end_char_exclusive':len(ctor)}}
    statement={'kind':'Method','raw':method,'span':{'start_char':len(ctor)+1,'end_char_exclusive':len(text)}}
    return {'all_tables':[{'fields':[{'key':{'name':'__call'},'value':value}]}],'definition':{'ordered_statements':[statement]}},text


class BoundedProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        sys.path.insert(0,str(ROOT/'tools/content-schema/quest-authoring'))
        import lua_writers,lua_tables
        cls.mask=staticmethod(lua_writers.mask_code)
        cls.next=builder.prior(ROOT)
        cls.tables=lua_tables

    def test_controller_active_ast_fenced_flow(self):
        ctor='function(self, config) local boss = config.boss; return {name = boss.name:lower()} end'
        method='function BossLever:onUse(player) Game.createMonster(self.name, self.bossPosition) end'
        record,text=fake_controller(ctor,method)
        self.assertEqual(len(builder.controller_flow(record,text,self.mask)),4)

    def test_controller_comment_only_flow_rejected(self):
        ctor='function(self, config) -- local boss = config.boss; name = boss.name:lower()\n return {} end'
        method='function BossLever:onUse(player) -- Game.createMonster(self.name, self.bossPosition)\n end'
        record,text=fake_controller(ctor,method)
        self.assertIsNone(builder.controller_flow(record,text,self.mask))

    def test_controller_mutation_or_stale_ast_rejected(self):
        for mutation in ['boss.name = "Wrong";', 'boss = other;', 'mutate(boss);']:
            ctor='function(self, config) local boss = config.boss; '+mutation+' return {name = boss.name:lower()} end'
            method='function BossLever:onUse(player) Game.createMonster(self.name, self.bossPosition) end'
            record,text=fake_controller(ctor,method)
            self.assertIsNone(builder.controller_flow(record,text,self.mask))
        record,text=fake_controller('function(self, config) local boss = config.boss;return {name = boss.name:lower()} end',method)
        record['all_tables'][0]['fields'][0]['value']['raw_expression']='bad'
        self.assertIsNone(builder.controller_flow(record,text,self.mask))

    def test_spell_literal_registration(self):
        text='local spell=Spell("instant");spell:name("Eye Beam");spell:register()'
        self.assertEqual(builder.named_spells(text,self.mask,self.next)[0][0],'Eye Beam')

    def test_spell_escape_and_dynamic_second_name_rejected(self):
        for middle in ['rename(spell);','local alias=spell;','spell.name=other;','spell:name(dynamic);']:
            text='local spell=Spell("instant");spell:name("Eye Beam");'+middle+'spell:register()'
            self.assertEqual(builder.named_spells(text,self.mask,self.next),[])

    def test_attack_field_mutation_not_accepted(self):
        text='monster.attacks={{name="Eye Beam"}};monster.attacks[1].name="Other";m:register(monster)'
        span=(text.index('m:register'),len(text))
        self.assertEqual(builder.attack_names(text,span,self.mask,self.next,self.tables),[])


class RealReferences(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet=builder.build(ROOT,ASSIGNMENT,MANIFEST)
        cls.next=builder.prior(ROOT)
        cls.base=cls.next.module(ROOT)
        cls.corpus=cls.base.Corpus(MANIFEST)
        cls.previous=cls.next.build(ROOT,ASSIGNMENT,MANIFEST)

    def test_before_after_exact_membership(self):
        self.assertEqual(len(self.packet['records']),248)
        self.assertEqual(self.packet['summary']['joined_components_before'],37)
        self.assertGreater(self.packet['summary']['joined_components'],37)
        self.assertGreater(self.packet['summary']['new_joined_components'],0)
        self.assertEqual({r['source_component_id'] for r in self.packet['records']},{r['source_component_id'] for r in self.previous['records']})
        before={r['source_component_id']:set(r['quest_keys']) for r in self.previous['records']}
        self.assertTrue(all(before[r['source_component_id']]<=set(r['quest_keys']) for r in self.packet['records']))

    def test_source_spans_and_fences(self):
        for row in self.packet['records']:
            for proof in row['proofs']:
                for witness in proof['source_witnesses']+proof['shared_dependency_witnesses']:
                    self.assertEqual(witness['source'],row['provenance']['source'])
                    self.assertEqual(witness['revision'],row['provenance']['revision'])
                    if witness in proof['source_witnesses']:
                        self.assertEqual(witness['path'].split('/')[0],row['provenance']['path'].split('/')[0])
                    text=self.corpus.text(self.corpus.files[(witness['source'],witness['revision'],witness['path'])])
                    self.assertEqual(self.base.sha(text[witness['char_start']:witness['char_end']].encode()),witness['token_sha256'])
                    self.assertEqual(self.base.sha(text.splitlines()[witness['line']-1].encode()),witness['line_sha256'])

    def test_reverse_reference_never_promotes_owner_or_native(self):
        for row in self.packet['records']:
            self.assertFalse(row['native_admission'])
            for proof in row['proofs']:
                self.assertIn('NONEXCLUSIVE',proof['association_direction'])
                self.assertEqual(proof['dispatch_binding'],'NOT_PROVEN')
        self.assertEqual(self.packet['quest_semantic_completeness'],'NOT_ESTABLISHED')

    def test_boss_descriptor_mutation_and_duplicate_keys_rejected(self):
        boss=json.loads(gzip.decompress((ROOT/'tools/content-schema/quest-authoring/samples/donor-source/components248/boss.json.gz').read_bytes()))['records'][0]
        source=boss['provenance']; text=self.corpus.text(self.corpus.files[(source['source'],source['revision'],source['path'])])
        import lua_writers
        self.assertIsNotNone(builder.boss_descriptor(boss,text,self.next,lua_writers.mask_code))
        self.assertIsNone(builder.boss_descriptor(boss,text+'\nconfig.boss.name="Wrong"',self.next,lua_writers.mask_code))
        altered=copy.deepcopy(boss)
        fields=altered['definition']['config_tables'][0]['definition']['fields'][0]['value']['fields'];fields.append(fields[0])
        self.assertIsNone(builder.boss_descriptor(altered,text,self.next,lua_writers.mask_code))

    def test_closed_schema_and_owner_promotion_rejected(self):
        import jsonschema
        validator=jsonschema.Draft202012Validator(self.base.read(Path(__file__).with_name('schema.json')))
        validator.validate(self.packet)
        for modification in [{'native_admission':True},{'quest_semantic_completeness':'COMPLETE'},{'owner':'q'}]:
            with self.assertRaises(jsonschema.ValidationError):validator.validate(dict(self.packet,**modification))


if __name__=='__main__':unittest.main()
