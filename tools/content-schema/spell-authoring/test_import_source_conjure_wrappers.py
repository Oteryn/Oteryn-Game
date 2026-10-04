import copy
import gzip
import json
import os
import tempfile
import unittest
from pathlib import Path

import import_source_conjure_wrappers as wrappers

ROOT=Path(__file__).resolve().parents[3]
R28=ROOT/'docs/reference/spells/r28-source-closure/player-source-bundles'


class ConjureWrappers(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows=[r for r in map(json.loads,gzip.decompress((R28/'source-callback-facts.jsonl.gz').read_bytes()).splitlines())
                  if '/spells/conjuring/chameleon_rune.lua#1' in r['registration_key']]

    def test_rune_carrier_cannot_qualify_as_instant_wrapper(self):
        row=copy.deepcopy(self.rows[0]);row['source_callback_facts']['spell_type']='rune'
        with self.assertRaisesRegex(ValueError,'carrier/tier'):
            wrappers.qualify(row,b'')

    def test_wrong_source_pin_is_refused(self):
        row=copy.deepcopy(self.rows[0]);row['source_revision']='0'*40
        with self.assertRaisesRegex(ValueError,'revision'):
            wrappers.qualify(row,b'')

    @unittest.skipUnless(os.environ.get('OTERYN_SPELL_SOURCE_GIT'),'requires existing local pinned source objects')
    def test_source_fact_count_mismatch_refuses_before_candidate(self):
        row=copy.deepcopy(self.rows[0]);raw=row['source_callback_facts']
        data=wrappers.base.source_file(Path(os.environ['OTERYN_SPELL_SOURCE_GIT'])/'canary',row['source_revision'],raw['file'])
        raw['cast']['conjure']['count']=2
        with self.assertRaisesRegex(ValueError,'arguments differ'):
            wrappers.qualify(row,data)

    @unittest.skipUnless(os.environ.get('OTERYN_SPELL_SOURCE_GIT'),'requires existing local pinned source objects')
    def test_all_four_actual_source_candidates_preserve_provider_refs_and_gaps(self):
        with tempfile.TemporaryDirectory() as tmp:
            out=Path(tmp)/'r34'
            summary=wrappers.generate(out,Path(os.environ['OTERYN_SPELL_SOURCE_GIT']),R28)
            self.assertEqual({'CANDIDATE_SCHEMA_VALID':4},summary['status_counts'])
            for item in summary['records_index']:
                snapshot=item['registration_key'].split('/')[0]
                row_id=wrappers.base.sha(item['registration_key'].encode())[:16]
                folder=out/snapshot/row_id
                spell=json.loads((folder/'spell.json').read_text())['spell']
                receipt=json.loads((folder/'receipt.json').read_text())
                conjure=spell['execution']['conjure']
                self.assertEqual('source-player-r34',spell['identity']['revision'])
                self.assertEqual('source-player-r28',conjure['result']['revision'])
                self.assertEqual('source-player-r28',conjure['reagent']['revision'])
                self.assertEqual(2,spell['costs']['soul'])
                self.assertNotIn('effect_asset_binding',conjure)
                self.assertFalse(receipt['runtime_activation'])
                self.assertFalse(receipt['native_execution_qualified'])
                self.assertEqual('conjure.source_effect_branch',receipt['remaining_mechanics'][0]['source_field'])
                self.assertEqual(1 if 'chameleon' in item['registration_key'] else 3,conjure['count'])


if __name__=='__main__':unittest.main()
