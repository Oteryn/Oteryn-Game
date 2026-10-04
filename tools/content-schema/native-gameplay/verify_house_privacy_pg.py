"""Exercise persistence guards in an explicitly disposable local PostgreSQL schema.

These database cases test actual migration constraints, not production session
admission or cast E2E. Synthetic historical Character roots are fixture data only;
no test constructs a spell authority seal, House runtime or position registry.
The spell Item guard cases run in their own schema, in dependency order: every
later case reuses the Character fixture that spell_item_owner_pg_cases.sql seeds.
"""
from pathlib import Path
import argparse
import subprocess
import re

ROOT = Path(__file__).resolve().parents[3]
SCHEMA = "spell_house_privacy_audit"
ITEM_SCHEMA = "spell_items_asset_audit"
ITEM_CASES = [
    "spell_item_owner_pg_cases.sql",
    "spell_inventory_owner_pg_cases.sql",
    "spell_item_temporal_pg_cases.sql",
    "spell_field_chain_pg_cases.sql",
    "spell_item_description_pg_cases.sql",
    "spell_direct_companion_pg_cases.sql",
]

def bootstrap(schema):
    parts = [f"\\set ON_ERROR_STOP on\nDROP SCHEMA IF EXISTS {schema} CASCADE;\nCREATE SCHEMA {schema};\nSET search_path={schema},pg_catalog;\nCREATE TABLE _sqlx_migrations(version BIGINT PRIMARY KEY,description TEXT,installed_on TIMESTAMPTZ DEFAULT now(),success BOOLEAN,checksum BYTEA,execution_time BIGINT);\n"]
    for path in sorted((ROOT / "apps/game-server/migrations").glob("*.sql")):
        # Fixture-only qualification for baseline function rowtypes in the
        # private test schema. Production migration bytes are unchanged.
        source = path.read_text().replace("pg_catalog, public, pg_temp", f"pg_catalog, {schema}, pg_temp").replace("pg_catalog,public,pg_temp", f"pg_catalog,{schema},pg_temp")
        parts.append(f"-- actual migration {path.name}\n{source}\n")
    parts.append(f"GRANT USAGE ON SCHEMA {schema} TO oteryn_game_runtime,oteryn_game_control;\n")
    return "\n".join(parts)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--container", default="oteryn-spells-pg")
    parser.add_argument("--url", help="disposable PostgreSQL URL for a local psql instead of docker")
    args = parser.parse_args()
    if args.url:
        command = ["psql", args.url, "-X", "-q"]
    else:
        command = ["docker", "exec", "-i", args.container, "sh", "-c", 'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -X -q']
    for sql in [bootstrap(SCHEMA), (ROOT / "apps/game-server/tests/support/house_spell_privacy_cases.sql").read_text(), (ROOT / "apps/game-server/tests/support/spell_parameter_result_cases.sql").read_text(), (ROOT / "apps/game-server/tests/support/world_house_instance_cases.sql").read_text(), (ROOT / "apps/game-server/tests/support/native_map_item_cases.sql").read_text()]:
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
    for sql in [bootstrap(ITEM_SCHEMA)] + [(ROOT / "apps/game-server/tests/support" / name).read_text() for name in ITEM_CASES]:
        completed = subprocess.run(command, input=sql, text=True, capture_output=True)
        if completed.returncode:
            raise SystemExit(completed.stderr[-4000:])
        if completed.stdout.strip():
            print(completed.stdout.strip())

if __name__ == "__main__":
    main()
