#!/usr/bin/env python3
"""Run only the focused Cake SQL cases in a uniquely named, migrated scratch DB.

Uses standard-library Python and an already installed psql. Supply the same
OTERYN_TEST_POSTGRES_ADMIN_URL used by the repository PostgreSQL harnesses.
"""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import uuid


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, help="repository root (normally inferred)")
    parser.add_argument("--sql-overlay", type=Path, action="append", default=[],
                        help="unmerged SQL overlay, applied after migrations; repeatable")
    parser.add_argument("--log", type=Path, help="save complete psql output")
    parser.add_argument("--case-overlay", type=Path, action="append", default=[],
                        help="additional focused SQL cases after the shared32 fixtures")
    args = parser.parse_args()
    repository = args.repository or Path(__file__).resolve().parents[4]
    migrations = sorted((repository / "apps/game-server/migrations").glob("*.sql"))
    if not migrations:
        parser.error("repository has no server migrations")
    admin = os.environ.get("OTERYN_TEST_POSTGRES_ADMIN_URL")
    psql = shutil.which("psql")
    if not admin or not psql:
        parser.error("source the repository environment; PostgreSQL admin URL and installed psql are required")
    database = "srv7_cake_sql_" + uuid.uuid4().hex
    quote = '"' + database + '"'
    command = [psql, admin, "-X", "-v", "ON_ERROR_STOP=1", "-v", "case_database=" + database]
    cases = Path(__file__).with_suffix(".sql").read_text(encoding="utf-8")
    script = ["CREATE DATABASE " + quote + ";\n\\connect " + database + "\nBEGIN;\n",
              "CREATE TABLE _sqlx_migrations(version BIGINT PRIMARY KEY,description TEXT,installed_on TIMESTAMPTZ DEFAULT now(),success BOOLEAN,checksum BYTEA,execution_time BIGINT);\n"]
    for path in [*migrations, *args.sql_overlay]:
        script.extend(["\n-- Source: " + path.name + "\n", path.read_text(encoding="utf-8"), "\n"])
    script.extend(["\nCOMMIT;\n", cases])
    for path in args.case_overlay:
        script.extend(["\n-- Focused cases: " + path.name + "\n", path.read_text(encoding="utf-8"), "\n"])
    started = time.perf_counter()
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", suffix=".sql") as source:
            source.write("".join(script))
            source.flush()
            result = subprocess.run([*command, "-f", source.name], capture_output=True, text=True, check=False)
        output = result.stdout + result.stderr
        if args.log:
            args.log.write_text(output, encoding="utf-8")
        if result.returncode:
            sys.stderr.write("\n".join(output.splitlines()[-40:]) + "\n")
            return result.returncode
        count = output.count("NOTICE:  PASS ")
        if count != 32:
            sys.stderr.write(f"expected 32 executed cases; saw {count}\n")
            return 1
        print(f"SRV7_CAKE_WORLD_SQL_PASSED cases={count} elapsed={time.perf_counter()-started:.3f}s")
        return 0
    finally:
        # Only this invocation's unguessable, constant-prefix throwaway name.
        cleanup = subprocess.run([psql, admin, "-X", "-v", "ON_ERROR_STOP=1", "-c",
                                  "DROP DATABASE IF EXISTS " + quote + " WITH (FORCE)"],
                                 capture_output=True, text=True, check=False)
        if cleanup.returncode:
            sys.stderr.write("scratch database cleanup failed: " + cleanup.stderr)


if __name__ == "__main__":
    raise SystemExit(main())
