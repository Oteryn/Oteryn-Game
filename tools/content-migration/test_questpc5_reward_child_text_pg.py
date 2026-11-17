"""Focused real-PG contract for migration 2071; no persistent schema or data.

The production predicate is loaded verbatim inside a rolled-back private schema.
The full migrator/legacy MINT case separately checks its deferred-trigger patch.
"""
import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import unittest
import uuid
from urllib.parse import parse_qs, unquote, urlsplit

ROOT = Path(__file__).resolve().parents[2]


class ChildTextPostgres(unittest.TestCase):
    def test_authored_child_text_matches_only_exact_ordered_mint(self):
        admin = os.environ.get('OTERYN_TEST_POSTGRES_ADMIN_URL')
        if not admin:
            self.fail('OTERYN_TEST_POSTGRES_ADMIN_URL is required; this case never skips')
        psql = os.environ.get('PSQL', shutil.which('psql'))
        if not psql:
            self.fail('PSQL must name the PostgreSQL client executable')
        migration = (ROOT / 'apps/game-server/migrations/2071_quest_reward_child_text.sql').read_text()
        predicate, patch = migration.split('DO $migration$', 1)
        self.assertIn('game_quest_reward_complete()', patch)
        give = dict(count=1, contents=[dict(item='parchment', count=1)],
                    text=dict(contents=[dict(index=0, text='source body')]))
        children = [dict(ordinal=1, production_key='parchment', quantity=1, text='source body')]
        cases = [(None, children, give, True)]
        for field, value in [('text', 'forged'), ('ordinal', 2), ('quantity', 2),
                             ('production_key', 'another item')]:
            changed = copy.deepcopy(children)
            changed[0][field] = value
            cases.append((None, changed, give, False))
        cases.append(('unexpected root', children, give, False))
        for field, value in [('index', 1), ('index', -1), ('index', 0.5),
                             ('text', ''), ('text', 'x' * 7997)]:
            changed = copy.deepcopy(give)
            changed['text']['contents'][0][field] = value
            cases.append((None, children, changed, False))
        cases.extend([
            ('legacy', [], dict(contents=[], text='legacy'), True),
            ('forged', [], dict(contents=[], text='legacy'), False),
        ])
        def literal(value, kind):
            if value is None:
                return 'NULL'
            raw = json.dumps(value, ensure_ascii=False) if kind == 'jsonb' else value
            return "'" + raw.replace("'", "''") + "'::" + kind
        statements = []
        for ordinal, (root, actual, authored, expected) in enumerate(cases):
            args = ','.join([literal(root, 'text'), literal(actual, 'jsonb'), literal(authored, 'jsonb')])
            statements.append(f"IF game_quest_reward_text_matches({args}) IS DISTINCT FROM {str(expected).lower()} THEN RAISE EXCEPTION 'child text case {ordinal}'; END IF;")
        schema = 'questpc5_text_' + uuid.uuid4().hex
        sql = ('BEGIN; CREATE SCHEMA ' + schema + '; SET LOCAL search_path=' + schema + ',pg_catalog;\n'
               + predicate + '\nDO $test$ BEGIN\n' + '\n'.join(statements)
               + f"\nRAISE NOTICE 'M15_CHILD_TEXT_POSTGRES_EXECUTED cases={len(cases)} server=%', current_setting('server_version'); END $test$; ROLLBACK;")
        endpoint = urlsplit(admin)
        query = parse_qs(endpoint.query)
        env = dict(os.environ, PGDATABASE=unquote(endpoint.path.lstrip('/')),
                   PGUSER=unquote(endpoint.username or ''),
                   PGPASSWORD=unquote(endpoint.password or ''),
                   PGHOST=query.get('host', [endpoint.hostname or 'localhost'])[0],
                   PGPORT=str(endpoint.port or 5432))
        if 'sslmode' in query:
            env['PGSSLMODE'] = query['sslmode'][0]
        started = time.monotonic()
        subprocess.run([psql, '-X', '-q', '-v', 'ON_ERROR_STOP=1'], input=sql, text=True, env=env, check=True)
        duration = time.monotonic() - started
        print(f'M15_CHILD_TEXT_PG_COMPLETED duration={duration:.6f}s', flush=True)
        self.assertGreater(duration, 0.01)


if __name__ == '__main__':
    unittest.main()
