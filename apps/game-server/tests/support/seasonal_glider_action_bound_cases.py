#!/usr/bin/env python3
"""Focused installed Glider CHECK cases in a uniquely named throwaway PostgreSQL DB."""
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
    parser.add_argument('--sql-overlay', type=Path, action='append', default=[])
    parser.add_argument('--log', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[4]
    psql = shutil.which('psql')
    admin = os.environ.get('OTERYN_TEST_POSTGRES_ADMIN_URL')
    if not psql or not admin:
        parser.error('source the existing repository environment')
    database = 'srv7_glider_bound_' + uuid.uuid4().hex
    command = [psql, admin, '-X', '-v', 'ON_ERROR_STOP=1']
    script = ['CREATE DATABASE "' + database + '";\n\\connect ' + database + '\nBEGIN;\n',
              'CREATE TABLE _sqlx_migrations(version BIGINT PRIMARY KEY,description TEXT,installed_on TIMESTAMPTZ DEFAULT now(),success BOOLEAN,checksum BYTEA,execution_time BIGINT);\n']
    for path in [*sorted((root/'apps/game-server/migrations').glob('*.sql')), *args.sql_overlay]:
        script.extend(['\n', path.read_text(), '\n'])
    script.extend(['\nCOMMIT;\n', Path(__file__).with_suffix('.sql').read_text()])
    start = time.perf_counter()
    try:
        with tempfile.NamedTemporaryFile(mode='w', suffix='.sql') as source:
            source.write(''.join(script))
            source.flush()
            result = subprocess.run([*command, '-f', source.name], capture_output=True, text=True, check=False)
        output = result.stdout + result.stderr
        args.log.write_text(output)
        count = output.count('NOTICE:  GLIDER_BOUND_PASS ')
        print(f'SRV7_GLIDER_BOUND cases={count} elapsed={time.perf_counter()-start:.3f}s exit={result.returncode}')
        if result.returncode:
            print('\n'.join(output.splitlines()[-12:]))
        return result.returncode or (0 if count == 14 else 1)
    finally:
        subprocess.run([*command, '-c', 'DROP DATABASE IF EXISTS "' + database + '" WITH (FORCE)'], capture_output=True, check=False)


if __name__ == '__main__':
    raise SystemExit(main())
