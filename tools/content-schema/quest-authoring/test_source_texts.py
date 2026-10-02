"""Exact reference prose capture, unknown preservation and portable offline rebuilding."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import source_text_authoring as a
import source_texts as t
from test_bundle_authoring import ROOT, fixture


def capture(text='Hello %d |PLAYERNAME|'):
    literal=json.dumps(text,ensure_ascii=False)
    source={'source':'canary',**a.SOURCES['canary'],'path':'data-otservbr-global/npc/example.lua',
            'line':1,'blob_sha256':'1'*64,'token_kind':'string','raw_literal':literal,
            'literal_sha256':hashlib.sha256(literal.encode('utf-8')).hexdigest(),
            'decoder':'lua_tables._string/v1','source_read_mode':'raw_utf8','text_transform':'identity'}
    return {'schema':'OTERYN_SOURCE_TEXT_CAPTURE/v1','classification':'OTS_HYPOTHESIS_ONLY',
            'scope':'source_reference_prose_only; no native or runtime admission',
            'texts':[{'reference':t.reference(text),'text':text,'sources':[source]}],
            'source_checks':[],'summary':{'captured_texts':1,'parsed_lua_files':1,'unparsed_lua_files':0}}


class SourceTextTests(unittest.TestCase):
    def test_reference_hash_length_and_placeholders_are_exact(self):
        cap=capture('żółw %d |PLAYERNAME|');a.validate(cap,ROOT,'capture')
        for field,value in [('sha256','0'*64),('length',1),('placeholders',[])]:
            bad=copy.deepcopy(cap);bad['texts'][0]['reference'][field]=value
            with self.subTest(field=field),self.assertRaises(ValueError):a.validate(bad,ROOT,'capture')
    def test_literal_decodes_with_authoring_escapes_without_eval(self):
        cap=capture('one\ntwo\t"three"');a.validate(cap,ROOT,'capture')
        cap['texts'][0]['sources'][0]['raw_literal']='"os.execute(\"never\")"'
        with self.assertRaises(ValueError):a.validate(cap,ROOT,'capture')
    def test_universal_newlines_keep_original_literal_and_blob_witness(self):
        cap=capture('first\nsecond');row=cap['texts'][0];source=row['sources'][0]
        source['raw_literal']='[=[\r\nfirst\r\nsecond]=]';source['token_kind']='lstring'
        source['source_read_mode']='universal_newlines'
        source['literal_sha256']=hashlib.sha256(source['raw_literal'].encode()).hexdigest()
        a.validate(cap,ROOT,'capture')
        self.assertEqual(source['blob_sha256'],'1'*64)
        source['source_read_mode']='raw_utf8'
        with self.assertRaises(ValueError):a.validate(cap,ROOT,'capture')
    def test_arbitrary_source_revision_rejected(self):
        cap=capture();cap['texts'][0]['sources'][0]['revision']='0'*40
        with self.assertRaises(ValueError):a.validate(cap,ROOT,'capture')
    def test_unsupported_lua_escape_cannot_claim_known_text(self):
        cap=capture('r');source=cap['texts'][0]['sources'][0];source['raw_literal']='"\\r"'
        source['literal_sha256']=hashlib.sha256(source['raw_literal'].encode()).hexdigest()
        with self.assertRaisesRegex(ValueError,'unsupported'):a.validate(cap,ROOT,'capture')
    def test_missing_text_is_unknown_with_owner_and_reference_not_empty(self):
        data=fixture();key=data['quests'][0]['identity']['key'];data['quests'][0]['test_reference']=t.reference('missing')
        registry=t.registry(data,{'texts':[]});row=registry['texts'][0]
        self.assertEqual(row['classification'],'UNKNOWN');self.assertNotIn('text',row)
        self.assertEqual(row['references'][0]['owners'],[key])
        data['source_texts']=registry;self.assertEqual(t.unresolved_gaps(data)[0]['owners'],[key])
    def test_conflict_alternative_keeps_its_distinct_quest_owner(self):
        data=fixture();selected=data['quests'][0]['identity']['key'];alternate='crystalserver:quest/other'
        data['quests'].append({**copy.deepcopy(data['quests'][0]),'identity':{'key':alternate,'revision':'source-r1'}})
        primary={'identity':{'key':'canary:interaction/sample'},'quest':{'family':'Quest','key':selected},
                 'text':t.reference('selected')}
        other={'identity':{'key':'crystalserver:interaction/sample'},'quest':{'family':'Quest','key':alternate},
               'text':t.reference('alternative')}
        data['interactions']=[primary]
        data['interaction_source_conflicts']=[{'interaction':primary['identity']['key'],
                                               'alternatives':[{'source':'crystalserver','interaction':other}]}]
        registry=t.registry(data,{'texts':[]})
        location=next(r for r in registry['texts'] if r['reference']==t.reference('alternative'))['references'][0]
        self.assertEqual(location['owners'],[alternate])
        self.assertEqual(location['record'],primary['identity']['key'])
        self.assertTrue(location['path'].startswith('/conflict_alternatives/crystalserver'))
        data['source_texts']=registry
        gap=next(g for g in t.unresolved_gaps(data) if g['record'].startswith(primary['identity']['key']+'/conflict_alternatives'))
        self.assertEqual(gap['owners'],[alternate])

    def test_missing_text_cannot_be_replaced_with_empty_known_literal(self):
        cap=capture('');cap['texts'][0]['reference']=t.reference('missing')
        with self.assertRaises(ValueError):a.validate(cap,ROOT,'capture')
    def test_literal_digest_tampering_rejected(self):
        cap=capture();cap['texts'][0]['sources'][0]['literal_sha256']='0'*64
        with self.assertRaisesRegex(ValueError,'digest'):a.validate(cap,ROOT,'capture')
    def test_offline_build_uses_capture_with_no_donor_access(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);data=fixture()
            for role,rel in a.INPUTS.items():
                document=(data['wiki_catalogue'] if role=='wiki_catalogue' else {'entries':[]}
                          if role=='interactions_manifest' else {role:data[role]})
                dest=root/rel;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(json.dumps(document))
            cap=root/'source_text_capture.json';cap.write_text(json.dumps(capture()))
            with patch('subprocess.check_output',side_effect=AssertionError('donor access')):
                first=a.build(root,cap,ROOT);second=a.build(root,cap,ROOT)
            self.assertEqual(first,second);self.assertEqual(first['summary']['wanted'],0)
    def test_strict_registry_rejects_unknown_literal_injection(self):
        data=fixture();data['quests'][0]['test_reference']=t.reference('missing')
        registry=t.registry(data,{'texts':[]})
        registry['capture_provenance']={'path':'capture.json','sha256':'0'*64}
        registry['input_provenance']=[];registry['texts'][0]['text']='invented'
        with self.assertRaises(ValueError):a.validate(registry,ROOT,'registry')


if __name__=='__main__':unittest.main()
