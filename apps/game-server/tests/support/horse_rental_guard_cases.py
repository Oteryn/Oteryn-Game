"""Execute finite rental/cash negative cases against all current migrations in an owned DB.

This checks real PostgreSQL trigger execution, not native gameplay. The native
command7/25/world cases are independent focused dispatch tests.
"""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[4]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--log', type=Path, default=Path('/tmp/srv7-horse-rental-guard-cases.log'))
    args = parser.parse_args()
    admin = os.environ['OTERYN_TEST_POSTGRES_ADMIN_URL']
    psql = shutil.which('psql')
    if not psql:
        raise RuntimeError('existing environment psql is required; do not install tools')
    migrations = sorted((ROOT / 'apps/game-server/migrations').glob('*.sql'))
    source = ''.join(p.read_text() for p in migrations)
    for marker in ['CREATE TABLE game_horse_station_worlds', 'game_npc_fee_cash_records']:
        if marker not in source:
            raise RuntimeError('current migrations do not yet contain ' + marker)
    database = 'srv7_horse_guards_' + uuid.uuid4().hex
    quoted = '"' + database + '"'
    command = [psql, admin, '-X', '-v', 'ON_ERROR_STOP=1']
    sql = ['CREATE DATABASE ' + quoted + ';\n\\connect ' + database + '\nBEGIN;\n',
           'CREATE TABLE _sqlx_migrations(version BIGINT PRIMARY KEY,description TEXT,installed_on TIMESTAMPTZ DEFAULT now(),success BOOLEAN,checksum BYTEA,execution_time BIGINT);\n', source, '\nCOMMIT;\n']
    for name in ['horse-rental-guard-cases.sql', 'npc-fee-carried-cash-guard-cases.sql']:
        sql.append((ROOT / 'tools/content-schema/native-gameplay' / name).read_text())
    started = time.perf_counter()
    try:
        with tempfile.NamedTemporaryFile(mode='w', suffix='.sql') as script:
            script.write(''.join(sql))
            script.flush()
            result = subprocess.run([*command, '-f', script.name], capture_output=True, text=True, check=False)
        output = result.stdout + result.stderr
        args.log.write_text(output)
        elapsed = time.perf_counter() - started
        print(f'SRV7_HORSE_CASH_POSTGRES_GUARDS elapsed={elapsed:.3f}s migrations={len(migrations)} exit={result.returncode}')
        for line in output.splitlines():
            if 'SRV7_' in line or 'ERROR:' in line:
                print(line)
        if result.returncode:
            raise SystemExit(result.returncode)
    finally:
        subprocess.run([*command, '-c', 'DROP DATABASE IF EXISTS ' + quoted + ' WITH (FORCE)'], capture_output=True, text=True, check=False)

if __name__ == '__main__':
    main()
