"""Reproduce bounded historical video arithmetic without claiming modern runtime parity."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import unittest

HERE=Path(__file__).resolve().parent


def load():
    return json.loads((HERE/'primary-video-cases.json').read_text(encoding='utf-8'))


def require(ok, code):
    if not ok:
        raise ValueError(code)


def validate(data, repository=None):
    require(data['schema']=='OTERYN_CHARMS_PRIMARY_VIDEO_CASES/v1' and len(data['cases'])==6,'POPULATION')
    require(data['source_head']=='3e168ce6afe374485da20ee0ac13d3749558e2f8','SOURCE_HEAD')
    require(not any(data[key] for key in ['runtime_changed','game_actions',
        'raw_media_or_transcripts_published','current_global_combat_parity_verified'])
        and data['source_rule_status_promotions']==[],'VIDEO_BOUNDARY')
    rows=data['cases'][:4]
    require([(r['inputs']['hit'],r['observed_heal']) for r in rows]==[(795,219),(784,216),(958,285),(948,282)],'VISIBLE_TABLE_VALUES')
    for row in rows:
        inputs=row['inputs']; hit=inputs['hit']
        skill=inputs['gear_life_leech_hundredths']+inputs['historical_vampiric_bonus_hundredths']
        require(inputs['target_count']==2 and skill in [5000,5400],'HISTORICAL_INPUTS')
        numerator=hit*skill*11; denominator=200000
        require(row['candidate_exact_numerator']==numerator and row['candidate_exact_denominator']==denominator
                and (numerator+denominator//2)//denominator==row['candidate_rounded']==row['observed_heal'], 'AOE_ARITHMETIC')
        require(row['unscaled_helper_output_if_real_equals_hit']==min(hit,(hit*skill+5000)//10000),'SOURCE_HELPER_ARITHMETIC')
        require(row['half_tie_rounding_verified'] is False and row['current_global_verified'] is False,'ROUNDING_VERSION_BOUNDARY')
    overkill=data['cases'][4]
    require(overkill['inputs']['recorded_hp_loss']==30 and overkill['observed_heal']==258
            and overkill['source_helper_output']==(30*5000+5000)//10000==15
            and overkill['current_global_verified'] is False,'OVERKILL_BOUNDARY')
    cleanse=data['cases'][5]
    require(cleanse['caption_interval_seconds']==[61,73] and not cleanse['current_global_verified']
            and not cleanse['raw_timer_sequence_frame_verified']
            and cleanse['illustrative_distinct_key_sequence']['kind']=='DERIVED_NOT_VIDEO_TIMINGS','CLEANSE_BOUNDARY')
    require(data['itexo_identification']['discord_explicit_url_found'] is False,'IDENTIFICATION_BOUNDARY')
    for receipt in data['source_receipts']:
        if repository:
            raw=subprocess.check_output(['git','show',data['source_head']+':'+receipt['path']],cwd=repository)
            require(len(raw)==receipt['bytes'] and hashlib.sha256(raw).hexdigest()==receipt['sha256'],'SOURCE_BYTES')
            require(all(0<r['start']<=r['end']<=len(raw.decode('utf-8').splitlines())
                        for r in data['source_ranges'] if r['path']==receipt['path']),'SOURCE_RANGE')
    return {'cases':6,'aoe_matches':4,'source_blobs':5,'modern_runtime_promotions':0}


class BoundaryTests(unittest.TestCase):
    def setUp(self):
        self.data=copy.deepcopy(load())

    def test_packet(self):
        validate(self.data)

    def test_direct_per_hit_is_not_the_visible_aoe_result(self):
        self.data['cases'][0]['observed_heal']=398
        with self.assertRaisesRegex(ValueError,'VISIBLE_TABLE_VALUES'):
            validate(self.data)

    def test_non_ties_do_not_establish_tie_rounding(self):
        self.data['cases'][0]['half_tie_rounding_verified']=True
        with self.assertRaisesRegex(ValueError,'ROUNDING_VERSION_BOUNDARY'):
            validate(self.data)

    def test_caption_claim_is_not_complete_timer_frame_trace(self):
        self.data['cases'][5]['raw_timer_sequence_frame_verified']=True
        with self.assertRaisesRegex(ValueError,'CLEANSE_BOUNDARY'):
            validate(self.data)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repository',type=Path)
    parser.add_argument('--test',action='store_true')
    args=parser.parse_args()
    if args.test:
        unittest.main(argv=['validate_primary_video'])
    else:
        print(json.dumps(validate(load(),args.repository),sort_keys=True))
