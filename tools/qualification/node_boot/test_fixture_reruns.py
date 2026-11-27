"""Exercise the kept-stack fixture SQL on disposable PostgreSQL temporary tables."""
import os
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[3]
SERVER = ROOT / "apps/game-server"
PHASES = SERVER / "src/qualification/client_phases"
ADMIN_URL = os.environ.get("OTERYN_TEST_POSTGRES_ADMIN_URL")
WORLD = "01890f4c-3b2a-7c01-8d11-9a321b7c0001"
OWNER = "01890f4c-3b2a-7c01-8d11-9a321b7c0002"
OTHER = "01890f4c-3b2a-7c01-8d11-9a321b7c0003"
ACCOUNT = "01890f4c-3b2a-7c01-8d11-9a321b7c0004"
HOUSE = "oteryn:content.house.test"


@unittest.skipUnless(ADMIN_URL, "requires a disposable PostgreSQL admin route")
class KeptFixtureTests(unittest.TestCase):
    def sql(self, source):
        return subprocess.run(
            ["psql", "-XAtq", "-v", "ON_ERROR_STOP=1", ADMIN_URL],
            input=source, text=True, capture_output=True, check=False, timeout=15,
        )

    def test_owned_house_fixture_preserves_the_exact_owned_state_and_rejects_a_conflict(self):
        fixture = (SERVER / "tests/support/owned_house_fixture.sql").read_text().strip()
        setup = f"""
            BEGIN;
            CREATE TEMP TABLE game_character_roots (character_id uuid, account_id uuid);
            INSERT INTO game_character_roots VALUES
                ('{OWNER}', '{ACCOUNT}'), ('{OTHER}', '{ACCOUNT}');
            CREATE FUNCTION pg_temp.game_character_uuid_v7() RETURNS uuid LANGUAGE SQL
                AS 'SELECT gen_random_uuid()';
            CREATE TEMP TABLE game_house_properties (
                world_id uuid, house_key text, kind text, state smallint, rent_gold bigint,
                content_digest bytea, owner_character_id uuid, owner_account_id uuid,
                acquisition_receipt_ref uuid, paid_until timestamptz,
                ownership_generation bigint DEFAULT 1, revision bigint DEFAULT 1,
                acl_revision bigint DEFAULT 1, content_fence_operation uuid,
                PRIMARY KEY (world_id, house_key)
            );
            CREATE TEMP TABLE game_account_housing_slots (
                account_id uuid, world_id uuid, house_key text,
                PRIMARY KEY (account_id, world_id), UNIQUE (world_id, house_key)
            );
            CREATE TEMP TABLE game_house_ownership (
                world_id uuid, house_key text, owner_character_id uuid, ownership_revision bigint,
                acquisition_receipt_ref uuid, PRIMARY KEY (world_id, house_key)
            );
            PREPARE seed(bytea, text, bytea) AS
            {fixture.replace('game_character_uuid_v7()', 'pg_temp.game_character_uuid_v7()')}
        """
        def seed(owner):
            return f"EXECUTE seed(decode('{WORLD.replace('-', '')}', 'hex'), '{HOUSE}', " \
                   f"decode('{owner.replace('-', '')}', 'hex'));"

        snapshot = """
            SELECT row_to_json(p) FROM game_house_properties p;
            SELECT row_to_json(s) FROM game_account_housing_slots s;
            SELECT row_to_json(o) FROM game_house_ownership o;
        """
        repeated = self.sql(setup + seed(OWNER) + snapshot + seed(OWNER) + snapshot + "ROLLBACK;")
        self.assertEqual(repeated.returncode, 0, repeated.stderr)
        rows = repeated.stdout.splitlines()
        self.assertEqual(len(rows), 6)
        self.assertEqual(rows[:3], rows[3:])

        conflict = self.sql(setup + seed(OWNER) + seed(OTHER) + "ROLLBACK;")
        self.assertNotEqual(conflict.returncode, 0)
        self.assertIn('duplicate key value violates unique constraint "game_house_properties_pkey"',
                      conflict.stderr)

    def test_w10_kept_fixture_restores_exact_initial_state_for_its_character_only(self):
        row = (PHASES / "client_row_w10.rs").read_text()
        fixture = row.split('sqlx::query(\n        "UPDATE game_character_hazard', 1)
        update = "UPDATE game_character_hazard" + fixture[1].split('"', 1)[0] if len(fixture) == 2 else "SELECT 1"
        if len(fixture) == 2:
            reset = f"""PREPARE reset(bytea, bytea) AS {update};
            EXECUTE reset(decode('{OWNER.replace('-', '')}', 'hex'),
                          decode('{WORLD.replace('-', '')}', 'hex'));"""
        else:
            reset = ""
        result = self.sql(f"""
            BEGIN;
            CREATE TEMP TABLE game_character_roots (character_id uuid, world_id uuid);
            CREATE TEMP TABLE game_character_hazard (character_id uuid, unlocked smallint, selected smallint);
            INSERT INTO game_character_roots VALUES ('{OWNER}', '{WORLD}'), ('{OTHER}', '{WORLD}');
            INSERT INTO game_character_hazard VALUES ('{OWNER}', 2, 2), ('{OTHER}', 2, 2);
            SET LOCAL session_replication_role = replica;
            {reset}
            SELECT unlocked, selected FROM game_character_hazard ORDER BY character_id;
            ROLLBACK;
        """)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), ["1|1", "2|2"])


if __name__ == "__main__":
    unittest.main()
