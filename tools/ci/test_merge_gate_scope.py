"""Exercise the protected-base exact-head scope contract without changing it."""
import contextlib
import copy
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / '.github/workflows/merge-gate.yml'
HEAD = 'a' * 40
BASE = 'b' * 40


class MergeGateScope(unittest.TestCase):
    def evaluate(self, change=None):
        source = WORKFLOW.read_text().split("          python - <<'PY'\n", 1)[1]
        source = source.split('\n          PY', 1)[0]
        source = '\n'.join(line[10:] for line in source.splitlines())
        before = {'state': 'open', 'changed_files': 2,
                  'head': {'sha': HEAD, 'repo': {'full_name': 'Oteryn/Oteryn-Game'}},
                  'base': {'sha': BASE, 'ref': 'main'}}
        after = copy.deepcopy(before)
        if change:
            change(after)
        comparison = {'files': [{'filename': 'crates/session/src/lib.rs', 'status': 'modified'},
                                {'filename': 'content/project.json', 'status': 'modified'}]}
        replies = iter([before, comparison, after])

        def response(request, timeout):
            self.assertEqual(timeout, 30)
            self.assertTrue(request.full_url.startswith('https://api.github.com/repos/Oteryn/Oteryn-Game/'))
            return io.BytesIO(json.dumps(next(replies)).encode())

        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'output'
            environment = {'REPOSITORY': 'Oteryn/Oteryn-Game', 'EVENT_PR_NUMBER': '3042',
                           'EVENT_PR_HEAD_SHA': HEAD, 'GH_TOKEN': 'scope-fixture',
                           'GITHUB_OUTPUT': str(output)}
            with patch.dict(os.environ, environment, clear=True), \
                    patch('urllib.request.urlopen', side_effect=response), \
                    contextlib.redirect_stdout(io.StringIO()), \
                    contextlib.redirect_stderr(io.StringIO()):
                exec(compile(source, str(WORKFLOW), 'exec'), {})
            return dict(line.split('=', 1) for line in output.read_text().splitlines())

    def test_stable_exact_head_preserves_immutable_file_enumeration(self):
        result = self.evaluate()
        self.assertEqual(result['target_sha'], HEAD)
        self.assertEqual(result['base_sha'], BASE)
        self.assertEqual(result['complete'], 'true')
        self.assertEqual(len(json.loads(result['file_records'])), 2)

    def test_count_race_discards_file_evidence_for_trusted_base_fallback(self):
        result = self.evaluate(lambda after: after.update(changed_files=3))
        self.assertEqual(result['target_sha'], HEAD)
        self.assertEqual(result['base_sha'], BASE)
        self.assertEqual(result['complete'], 'false')
        self.assertEqual(json.loads(result['file_records']), [])

    def test_count_race_cannot_hide_head_base_repository_or_state_movement(self):
        for field in ('head', 'base', 'repository', 'state', 'base_ref'):
            def change(after):
                after['changed_files'] = 3
                if field == 'head':
                    after['head']['sha'] = 'c' * 40
                elif field == 'base':
                    after['base']['sha'] = 'c' * 40
                elif field == 'repository':
                    after['head']['repo']['full_name'] = 'other/repository'
                elif field == 'state':
                    after['state'] = 'closed'
                else:
                    after['base']['ref'] = 'other'
            with self.subTest(field=field), self.assertRaises(SystemExit) as stopped:
                self.evaluate(change)
            self.assertEqual(stopped.exception.code, 1)


if __name__ == '__main__':
    unittest.main()
