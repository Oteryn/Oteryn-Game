"""PG statement conformance for the same SQL used by the fenced return-port owner.

The Rust PG case exercises real travel receipts, placement and authority fences.
This complementary statement case checks clock ordering and ambiguous history in
session-local tables, without changing production tables or credentials.
"""
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[6]
A = "oteryn:service.travel.m38_origin_a/travora"
B = "oteryn:service.travel.m38_origin_b/travora"
CHAR = "01960000000070018000000000000001"


def assertion(expected):
    expected_sql = "NULL" if expected is None else "'" + expected + "'"
    return f"""
DO $case$
DECLARE actual text;
BEGIN
    EXECUTE $read$EXECUTE npcfix_return_reader(decode('{CHAR}', 'hex'), ARRAY['{A}', '{B}'])$read$
        INTO actual;
    IF actual IS DISTINCT FROM {expected_sql} THEN
        RAISE EXCEPTION 'last completed return port mismatch: got %, expected %', actual, {expected_sql}::text;
    END IF;
END $case$;
"""


def main():
    connection = os.environ.get("OTERYN_TEST_POSTGRES_ADMIN_URL")
    if not connection:
        print("No PG test route; statement case not exercised")
        return 0
    query = (ROOT / "crates/gameplay/durability/src/durability/npc_travel/return_port.sql").read_text()
    index = (ROOT / "apps/game-server/migrations/2140_npc_travel_completed_origin.sql").read_text()
    sql = f"""
SELECT 'POSTGRES_STARTED NPCFIX_M38 return-port statement';
CREATE TEMP TABLE game_newhaven_travel_receipts (
    character_id uuid NOT NULL, route text NOT NULL,
    arrived_at timestamptz, created_xact_id xid8 NOT NULL
);
{index}
INSERT INTO game_newhaven_travel_receipts VALUES
('{CHAR}', '{A}', clock_timestamp()+interval '1 day', '100'),
('{CHAR}', '{B}', clock_timestamp()-interval '1 day', '200'),
('{CHAR}', '{A}', NULL, '300'),
('{CHAR}', 'oteryn:service.travel.unrelated/elsewhere', clock_timestamp(), '400'),
('01960000000070018000000000000002', '{A}', clock_timestamp(), '500');
PREPARE npcfix_return_reader(bytea,text[]) AS {query}
{assertion(B)}
INSERT INTO game_newhaven_travel_receipts VALUES ('{CHAR}','{A}',clock_timestamp(),'600');
{assertion(A)}
INSERT INTO game_newhaven_travel_receipts VALUES ('{CHAR}','{B}',clock_timestamp(),'600');
{assertion(None)}
INSERT INTO game_newhaven_travel_receipts VALUES ('{CHAR}','{B}',NULL,'700');
{assertion(None)}
UPDATE game_newhaven_travel_receipts SET arrived_at=clock_timestamp() WHERE created_xact_id='700';
{assertion(B)}
SELECT 'POSTGRES_CASE_COMPLETED NPCFIX_M38_RETURN_STATEMENT cases=5';
"""
    started = time.monotonic()
    # Never print the command or connection string, including on a nonzero exit.
    result = subprocess.run(
        ["psql", "-X", "--quiet", "--set", "ON_ERROR_STOP=1", "--dbname", connection],
        input=sql, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
    )
    print(result.stdout.replace(connection, "<test-db>"), end="")
    print(f"POSTGRES_EXERCISED NPCFIX_M38_RETURN_STATEMENT duration={time.monotonic()-started:.6f}s rc={result.returncode}")
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
