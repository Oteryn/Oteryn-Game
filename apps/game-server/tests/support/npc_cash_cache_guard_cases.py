#!/usr/bin/env python3
"""Run isolated SQL memo lifecycle/authority and unarmed audit-shape cases, not native NPC behavior."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import uuid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sql-overlay', type=Path)
    parser.add_argument('--log', type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[4]
    psql = shutil.which('psql')
    if not psql:
        raise SystemExit('psql is required; this runner does not install it')
    command = [psql, os.environ['OTERYN_TEST_POSTGRES_ADMIN_URL'], '-X', '-v', 'ON_ERROR_STOP=1']
    database = 'srv7_npc_cash_cache_' + uuid.uuid4().hex
    quoted_database = '"' + database + '"'
    migrations = sorted((root / 'apps/game-server/migrations').glob('*.sql'))
    pieces = ['CREATE DATABASE ' + quoted_database + ';\n\\connect ' + database + '\nBEGIN;\n',
              'CREATE TABLE _sqlx_migrations(version BIGINT PRIMARY KEY,description TEXT,installed_on TIMESTAMPTZ DEFAULT now(),success BOOLEAN,checksum BYTEA,execution_time BIGINT);\n']
    for migration in migrations:
        pieces.extend(['\n-- ' + migration.name + '\n', migration.read_text(), '\n'])
    if args.sql_overlay:
        pieces.extend([args.sql_overlay.read_text(), '\n'])
    pieces.extend(['COMMIT;\n', Path(__file__).with_name('npc_cash_cache_cases.sql').read_text(), Path(__file__).with_name('npc_cash_shape_cases.sql').read_text(), Path(__file__).with_name('npc_cash_indexed_custody_cases.sql').read_text()])
    started = time.perf_counter()
    try:
        with tempfile.NamedTemporaryFile(mode='w', suffix='.sql') as script:
            script.write(''.join(pieces))
            script.flush()
            result = subprocess.run(command + ['-f', script.name], capture_output=True, text=True)
        output = result.stdout + result.stderr
        if args.log:
            args.log.write_text(output)
        print('migrations={} elapsed={:.3f}s status={}'.format(len(migrations), time.perf_counter() - started, result.returncode))
        print('\n'.join(output.splitlines()[-8:]))
        return result.returncode
    finally:
        subprocess.run(command + ['-c', 'DROP DATABASE IF EXISTS ' + quoted_database + ' WITH (FORCE)'], capture_output=True, text=True, check=False)


if __name__ == '__main__':
    raise SystemExit(main())
