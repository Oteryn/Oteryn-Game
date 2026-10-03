"""Exercise persistence guards in an explicitly disposable local PostgreSQL schema.

These database cases test actual migration constraints, not production session
admission or cast E2E. Synthetic historical Character roots are fixture data only;
no test constructs a spell authority seal, House runtime or position registry.
"""
from pathlib import Path
import argparse
import subprocess
import re

ROOT = Path(__file__).resolve().parents[3]
SCHEMA = "spell_house_privacy_audit"

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--container", default="oteryn-spells-pg")
    args = parser.parse_args()
    command = ["docker", "exec", "-i", args.container, "sh", "-c", 'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -X -q']
    bootstrap = [f"\\set ON_ERROR_STOP on\nDROP SCHEMA IF EXISTS {SCHEMA} CASCADE;\nCREATE SCHEMA {SCHEMA};\nSET search_path={SCHEMA},pg_catalog;\nCREATE TABLE _sqlx_migrations(version BIGINT PRIMARY KEY,description TEXT,installed_on TIMESTAMPTZ DEFAULT now(),success BOOLEAN,checksum BYTEA,execution_time BIGINT);\n"]
    for path in sorted((ROOT / "apps/game-server/migrations").glob("*.sql")):
        if int(path.name.split("_", 1)[0]) > 48:
            continue
        # Fixture-only qualification for baseline function rowtypes in the
        # private test schema. Production migration bytes are unchanged.
        source = path.read_text().replace("pg_catalog, public, pg_temp", f"pg_catalog, {SCHEMA}, pg_temp").replace("pg_catalog,public,pg_temp", f"pg_catalog,{SCHEMA},pg_temp")
        bootstrap.append(f"-- actual migration {path.name}\n{source}\n")
    bootstrap.append(f"GRANT USAGE ON SCHEMA {SCHEMA} TO oteryn_game_runtime,oteryn_game_control;\n")
    for sql in ["\n".join(bootstrap), (ROOT / "apps/game-server/tests/support/house_spell_privacy_cases.sql").read_text(), (ROOT / "apps/game-server/tests/support/spell_parameter_result_cases.sql").read_text(), (ROOT / "apps/game-server/tests/support/world_house_instance_cases.sql").read_text(), (ROOT / "apps/game-server/tests/support/native_map_item_cases.sql").read_text()]:
        completed = subprocess.run(command, input=sql, text=True, capture_output=True)
        if completed.returncode:
            raise SystemExit(completed.stderr[-4000:])
        if completed.stdout.strip():
            print(completed.stdout.strip())
    # Exercise the actual receipt-reader predicate, not a separately mirrored query.
    source = (ROOT / "apps/game-server/src/durability/spell_item_transaction.rs").read_text()
    predicate = re.search(r'async fn verify_paid_due_source\([\s\S]+?query_scalar\("([^"]+)"\)', source)
    if predicate is None:
        raise SystemExit("missing exact due receipt predicate")
    install = f"SET search_path={SCHEMA},pg_catalog;\nCREATE FUNCTION test_due_paid_match(BYTEA,TEXT,BYTEA,BYTEA,BYTEA,TEXT,TEXT,BYTEA,BYTEA,TEXT,TEXT) RETURNS BOOLEAN LANGUAGE SQL AS $source${predicate.group(1)}$source$;\n"
    completed = subprocess.run(command, input=install + (ROOT / "apps/game-server/tests/support/spell_due_read_cases.sql").read_text(), text=True, capture_output=True)
    if completed.returncode:
        raise SystemExit(completed.stderr[-4000:])
    print(completed.stdout.strip())

if __name__ == "__main__":
    main()
