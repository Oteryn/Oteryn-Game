import copy
import json
import tempfile
import unittest
from pathlib import Path
import jsonschema
import export_unresolved_spell_links as links


def slot(registered=True):
    source={'revision':'a'*40,'path':'monster.lua','sha256':'b'*64,'git_blob':'c'*40}
    registered_source={**source,'path':'spell.lua'} if registered else None
    return {'candidate_id':'canary/monster','source':'canary','monster':'Monster','group':'attacks','source_slot_index':2,
            'source_parameters':{'name':'custom','chance':0,'target':False,'type':'@COMBAT_UNDEFINEDDAMAGE'},
            'monster_source':source,'registered_source':registered_source,'conversion_status':'unresolved_semantics',
            'resolution':'registered' if registered else 'inline','script_tier':'P4' if registered else None,
            'conversion_manifest_rows':[{'status':'unresolved_semantics','resolution':'explicit unknown source behavior'}],
            'external_verification':'NOT_VERIFIED'}


def fact():return {'source':'canary',**slot()['registered_source'],'calls':[{'method':'unexecuted'}]}


class UnresolvedLinks(unittest.TestCase):
    def test_exact_identity_links_and_preserves_source_params_status(self):
        original=slot();descriptor={'source_identity':{k:v for k,v in fact().items() if k!='calls'},'mechanic_category':'custom'}
        row=links.join([original],[fact()],[descriptor],'inventory.gz','custom.gz')[0]
        self.assertEqual(original['source_parameters'],row['source_parameters'])
        self.assertEqual('unresolved_semantics',row['conversion_status'])
        self.assertIsNotNone(row['registered_fact_link']);self.assertIsNotNone(row['custom_descriptor_link'])
        self.assertFalse(row['native_ability_admission']);self.assertFalse(row['runtime_activation'])

    def test_same_name_path_different_sha_cannot_link(self):
        bad=fact();bad['sha256']='d'*64
        with self.assertRaisesRegex(ValueError,'exact revision/path/SHA'):
            links.join([slot()],[bad],[],'inventory.gz','custom.gz')

    def test_custom_descriptor_wrong_revision_is_not_attached(self):
        wrong={k:v for k,v in fact().items() if k!='calls'};wrong['revision']='d'*40
        row=links.join([slot()],[fact()],[{'source_identity':wrong,'mechanic_category':'custom'}],'inventory.gz','custom.gz')[0]
        self.assertIsNone(row['custom_descriptor_link'])

    def test_inline_invalid_type_is_preserved_and_missing_stays_missing(self):
        original=slot(False)
        row=links.join([original],[],[],'inventory.gz','custom.gz')[0]
        self.assertEqual('@COMBAT_UNDEFINEDDAMAGE',row['inline_raw_type']);self.assertIsNone(row['registered_fact_link'])
        del original['source_parameters']['type']
        row=links.join([original],[],[],'inventory.gz','custom.gz')[0]
        self.assertFalse(row['inline_type_present']);self.assertIsNone(row['inline_raw_type'])

    def test_duplicate_slot_identity_refused(self):
        with self.assertRaisesRegex(ValueError,'duplicate unresolved'):
            links.join([slot(),slot()],[fact()],[],'inventory.gz','custom.gz')

    def test_strict_schema_refuses_mapped_status_and_activation(self):
        schema=json.loads(Path(links.__file__).with_name('unresolved-monster-spell-links.schema.json').read_text())
        row=links.join([slot()],[fact()],[],'inventory.gz','custom.gz')[0]
        jsonschema.validate(row,schema)
        for changed in [{'conversion_status':'mapped'},{'runtime_activation':True},{'native_ability_admission':True}]:
            with self.assertRaises(jsonschema.ValidationError):jsonschema.validate({**row,**changed},schema)

    def test_tampered_original_archive_is_refused_before_reads(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            (root/'monster-source-package-manifest.json').write_text(json.dumps({'archive':{'path':'archive.gz','sha256':'a'*64}}))
            (root/'archive.gz').write_bytes(b'tampered')
            with self.assertRaisesRegex(ValueError,'archive SHA mismatch'):
                links.export(root,root/'out')


if __name__=='__main__':unittest.main()
