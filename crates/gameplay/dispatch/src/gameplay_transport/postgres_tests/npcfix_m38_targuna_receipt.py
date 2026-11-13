"""Exercise the actual old and forward receipt constraints on session-local PG tables."""
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[6]


def main():
    connection = os.environ.get("OTERYN_TEST_POSTGRES_ADMIN_URL")
    if not connection:
        print("No PG test route; receipt case not exercised")
        return 0
    old = (ROOT / "apps/game-server/migrations/0223_npc_paid_travel.sql").read_text()
    shape = old[old.index("ADD CONSTRAINT game_travel_route_shape"):old.index("CREATE INDEX")].strip()
    migration = ROOT / "apps/game-server/migrations/2141_targuna_free_travel_receipt.sql"
    upgrade = migration.read_text() if migration.exists() else ""
    sql = f"""
SELECT 'POSTGRES_STARTED NPCFIX_M38_TARGUNA_RECEIPT';
CREATE TEMP TABLE game_newhaven_travel_receipts (
 route text NOT NULL, request_binding bytea, fee_transaction_id uuid,
 fare_gold_units bigint, base_fare_gold_units bigint, postman_discount_applied boolean
);
ALTER TABLE game_newhaven_travel_receipts {shape}
{upgrade}
INSERT INTO game_newhaven_travel_receipts(route) VALUES ('targuna'),('rookgaard'),('thais_peninsula');
DO $case$
DECLARE candidate text; metadata text;
BEGIN
 FOREACH candidate IN ARRAY ARRAY['targuna_extra','Targuna','oteryn:service.travel.anna/targuna'] LOOP
  BEGIN
   INSERT INTO game_newhaven_travel_receipts(route) VALUES(candidate);
   RAISE EXCEPTION 'invalid free route accepted: %', candidate;
  EXCEPTION WHEN check_violation THEN NULL;
  END;
 END LOOP;
 FOREACH metadata IN ARRAY ARRAY[
  'request_binding', 'fee_transaction_id', 'fare_gold_units',
  'base_fare_gold_units', 'postman_discount_applied'
 ] LOOP
  BEGIN
   EXECUTE format('INSERT INTO game_newhaven_travel_receipts(route,%I) VALUES (%L,%s)',
    metadata, 'targuna', CASE metadata
     WHEN 'request_binding' THEN 'decode(repeat(''ab'',32),''hex'')'
     WHEN 'fee_transaction_id' THEN '''01960000-0000-7001-8000-000000000001''::uuid'
     WHEN 'postman_discount_applied' THEN 'false' ELSE '0' END);
   RAISE EXCEPTION 'free route accepted metadata: %', metadata;
  EXCEPTION WHEN check_violation THEN NULL;
  END;
 END LOOP;
END $case$;
INSERT INTO game_newhaven_travel_receipts VALUES
 ('oteryn:service.travel.petros/issavi',decode(repeat('ab',32),'hex'),NULL,0,10,true),
 ('oteryn:service.travel.petros/issavi',decode(repeat('ab',32),'hex'),
 '01960000-0000-7001-8000-000000000001',120,130,true);
DO $case$
BEGIN
 BEGIN
  INSERT INTO game_newhaven_travel_receipts VALUES
   ('oteryn:service.travel.petros/issavi',decode(repeat('ab',32),'hex'),NULL,120,130,true);
  RAISE EXCEPTION 'positive paid fare accepted without burn';
 EXCEPTION WHEN check_violation THEN NULL;
 END;
 BEGIN
  INSERT INTO game_newhaven_travel_receipts VALUES
   ('oteryn:service.travel.petros/issavi',decode(repeat('ab',32),'hex'),
    '01960000-0000-7001-8000-000000000001',119,130,true);
  RAISE EXCEPTION 'paid discount mismatch accepted';
 EXCEPTION WHEN check_violation THEN NULL;
 END;
END $case$;
SELECT 'POSTGRES_CASE_COMPLETED NPCFIX_M38_TARGUNA_RECEIPT';
"""
    started = time.monotonic()
    result = subprocess.run(
        ["psql", "-X", "--quiet", "--set", "ON_ERROR_STOP=1", "--dbname", connection],
        input=sql, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
    )
    print(result.stdout.replace(connection, "<test-db>"), end="")
    print(f"POSTGRES_EXERCISED NPCFIX_M38_TARGUNA_RECEIPT duration={time.monotonic()-started:.6f}s rc={result.returncode}")
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
