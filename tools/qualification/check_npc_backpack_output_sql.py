#!/usr/bin/env python3
"""Run current migrations and NPC-output SQL boundaries in a disposable database.

The actual Sandra framed caller separately proves positive MINT authority.
Connection credentials stay in the subprocess environment, never error arguments.
"""
import os
from pathlib import Path
import subprocess
import time
from urllib.parse import parse_qs, unquote, urlsplit, urlunsplit

ROOT = Path(__file__).resolve().parents[2]
CASES = r'''
DO $negative$ DECLARE source_key TEXT; rejected BOOLEAN:=FALSE;BEGIN
 FOREACH source_key IN ARRAY ARRAY['oteryn:npc-cosmetic/sandra/belt-ticket-283','oteryn:npc-cosmetic/sandra/belt-ticket-284','oteryn:npc-cosmetic/sandra/belt-ticket-285'] LOOP
  IF game_npc_source_output_id(source_key) IS DISTINCT FROM 5957 THEN RAISE EXCEPTION 'Sandra source mapping';END IF;
 END LOOP;
 IF game_npc_source_output_id('oteryn:npc-cosmetic/bozo/jester-addon-1') IS DISTINCT FROM 895 OR game_npc_source_output_id('oteryn:npc-cosmetic/bozo/jester-addon-2') IS DISTINCT FROM 894 OR game_npc_source_output_id('oteryn:npc-cosmetic/gnomadness/ripptor-prize') IS DISTINCT FROM 39546 OR game_npc_source_output_id('oteryn:npc-cosmetic/sandra/belt-ticket-286') IS NOT NULL THEN RAISE EXCEPTION 'finite source map';END IF;
 IF has_table_privilege('oteryn_game_runtime','game_npc_source_item_outputs','INSERT') THEN RAISE EXCEPTION 'runtime may forge child';END IF;
 BEGIN PERFORM game_npc_source_bind_output('00000000-0000-7000-8000-000000000001');EXCEPTION WHEN check_violation THEN rejected:=TRUE;END;
 IF NOT rejected THEN RAISE EXCEPTION 'unbound output accepted';END IF;
END $negative$;
'''


def psql(url, *arguments, sql=None):
    parts = urlsplit(url)
    environment = dict(os.environ)
    environment.update(PGHOST=parts.hostname or '127.0.0.1',
                       PGPORT=str(parts.port or 5432),
                       PGUSER=unquote(parts.username or ''),
                       PGPASSWORD=unquote(parts.password or ''),
                       PGDATABASE=unquote(parts.path.lstrip('/')))
    for key, values in parse_qs(parts.query).items():
        if key == 'sslmode' and len(values) == 1:
            environment['PGSSLMODE'] = values[0]
        else:
            raise ValueError('Unsupported PostgreSQL connection option')
    return subprocess.run(
        ['psql', '-X', '-v', 'ON_ERROR_STOP=1', *arguments],
        input=sql, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        env=environment, check=False,
    )


def main():
    admin = os.environ['OTERYN_TEST_POSTGRES_ADMIN_URL']
    name = f'srv7_npc_output_check_{os.getpid()}'
    target = urlunsplit(urlsplit(admin)._replace(path='/' + name))
    created = psql(admin, '-qc', 'CREATE DATABASE ' + name)
    if created.returncode:
        raise SystemExit(created.stdout)
    start = time.monotonic()
    try:
        migrations = sorted((ROOT / 'apps/game-server/migrations').glob('*.sql'))
        sql = 'BEGIN;\nCREATE TABLE _sqlx_migrations(version BIGINT PRIMARY KEY,description TEXT NOT NULL,installed_on TIMESTAMPTZ NOT NULL DEFAULT now(),success BOOLEAN NOT NULL,checksum BYTEA NOT NULL,execution_time BIGINT NOT NULL);\n'
        sql += '\n'.join(path.read_text() for path in migrations)
        result = psql(target, sql=sql + '\n' + CASES + '\nCOMMIT;')
        log = Path('/tmp/srv7-npc-output-sql-current.log')
        log.write_text(result.stdout)
        print(f'NPC output SQL checks exit={result.returncode} duration={time.monotonic()-start:.3f}s log={log}')
        if result.returncode:
            print(result.stdout[-2000:])
            raise SystemExit(result.returncode)
    finally:
        dropped = psql(admin, '-qc', 'DROP DATABASE ' + name)
        if dropped.returncode:
            raise SystemExit(dropped.stdout)


if __name__ == '__main__':
    main()
