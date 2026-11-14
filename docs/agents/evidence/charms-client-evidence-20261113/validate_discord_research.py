"""Check both-server research scope and prevent invalid searches becoming evidence."""
import argparse
import copy
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent


def require(ok, label):
    if not ok:
        raise ValueError(label)


def load():
    return json.loads((HERE/'discord-research-source-matrix.json').read_text(encoding='utf-8'))


def validate(data):
    require(data['schema'] == 'OTERYN_CHARMS_DISCORD_UI_PASS/v1', 'SCHEMA')
    servers={row['id'] for row in data['servers']}
    require(servers == {'1431612466818977814','836592213327609957'}, 'BOTH_SERVERS')
    require(data['compared_main'] == '3e168ce6afe374485da20ee0ac13d3749558e2f8', 'SOURCE_HEAD')
    require(not any(data[key] for key in ['runtime_changes','game_actions',
                'complete_server_archive_reviewed','original_media_published'])
            and not data['mechanic_status_promotions'], 'RESEARCH_BOUNDARY')
    require(len(data['result_windows']) == 10 and len(data['claims']) == 6
            and len(data['invalid_attempts']) == 5, 'POPULATION')
    for row in data['result_windows']:
        require(row['server_id'] in servers and row['reviewed_scope']
                and row['status'] == 'MATCHING_RESULT_WINDOW_OBSERVED'
                and not row['exhaustive'] and not row['absence_established'], 'WINDOW_BOUNDARY')
    for row in data['invalid_attempts']:
        require(row['status'] == 'INVALID_QUERY_NOT_ABSENCE_EVIDENCE'
                and row['usable_result_count'] is None, 'INVALID_QUERY_BOUNDARY')
    public=json.loads((HERE/'public-research-source-matrix.json').read_text(encoding='utf-8'))
    paths={row['path'] for row in public['source_receipts']}
    for row in data['claims']:
        expected='https://discord.com/channels/'+row['server_id']+'/'+row['channel_id']+'/'+row['message_id']
        require(row['permalink'] == expected and row['published_local'] and row['context']
                and row['comparison_with_oteryn'] and row['source_domain'] == 'GLOBAL_TIBIA_COMMUNITY'
                and row['evidence_kind'], 'CLAIM_PROVENANCE')
        require(row['current_global_measured_proof'] is False and row['runtime_defect_reproduced'] is False,
                'CLAIM_PROMOTION')
        require(all(path in paths and line > 0 for path,line in row['source_pointers']), 'SOURCE_BINDING')
    return {'servers':2,'result_windows':10,'dated_contexts':6,'invalid_attempts_excluded':5,'runtime_promotions':0}


class BoundaryTests(unittest.TestCase):
    def setUp(self):
        self.data=copy.deepcopy(load())

    def test_packet(self):
        validate(self.data)

    def test_invalid_query_cannot_be_valid_zero(self):
        self.data['invalid_attempts'][0]['usable_result_count']=0
        with self.assertRaisesRegex(ValueError,'INVALID_QUERY_BOUNDARY'):
            validate(self.data)

    def test_question_or_historical_claim_cannot_be_current_measurement(self):
        self.data['claims'][0]['current_global_measured_proof']=True
        with self.assertRaisesRegex(ValueError,'CLAIM_PROMOTION'):
            validate(self.data)

    def test_partial_window_cannot_establish_absence(self):
        self.data['result_windows'][0]['absence_established']=True
        with self.assertRaisesRegex(ValueError,'WINDOW_BOUNDARY'):
            validate(self.data)


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--test',action='store_true')
    args=parser.parse_args()
    if args.test:
        unittest.main(argv=['validate_discord_research'])
    else:
        print(json.dumps(validate(load()),sort_keys=True))
