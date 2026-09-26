"""Current-source Fandom request and cache regressions without network access."""
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import wiki_compare as wc


class CurrentFandom(unittest.TestCase):
    def test_latest_request_has_no_historical_revision_selector(self):
        response = {'curtimestamp': '2026-09-26T20:00:00Z', 'query': {'pages': [{
            'pageid': 1, 'title': 'Rat', 'revisions': [{'revid': 42,
            'timestamp': '2026-07-25T00:00:00Z', 'slots': {'main': {'content': 'facts'}}}]}]}}
        with patch.object(wc, 'api', return_value=response) as api:
            record = wc.revision('Rat')
        request = api.call_args.args[0]
        self.assertEqual(request['rvlimit'], 1)
        self.assertNotIn('rvstart', request)
        self.assertNotIn('rvend', request)
        self.assertNotIn('revids', request)
        self.assertEqual(record['revision_id'], 42)
        self.assertEqual(record['api_server_timestamp'], response['curtimestamp'])

    def test_historical_cache_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            cache = Path(directory)
            (cache / 'rat.json').write_text(json.dumps({'cut': {'revision_id': 1}}), encoding='utf-8')
            with self.assertRaisesRegex(ValueError, 'Historical'):
                wc.fetch('Rat', cache)

    def test_latest_cache_reproduces_without_network(self):
        record = {'request_mode': 'latest', 'current': {'revision_id': 42},
                  'retrieved_at': '2026-09-26T20:00:00Z'}
        with tempfile.TemporaryDirectory() as directory:
            cache = Path(directory)
            (cache / 'rat.json').write_text(json.dumps(record), encoding='utf-8')
            with patch.object(wc, 'revision', side_effect=AssertionError('Unexpected network')):
                self.assertEqual(wc.fetch('Rat', cache), record)


if __name__ == '__main__':
    unittest.main()
