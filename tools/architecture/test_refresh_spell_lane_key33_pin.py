"""Reviewed SQL expiry owners cannot silently join the key33 inventory."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('key33_refresh', Path(__file__).with_name('refresh_spell_lane_key33_pin.py'))
OWNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(OWNER)
SQL_PATH = 'apps/game-server/migrations/0992_house_furniture_population.sql'
FUNCTION = 'game_furniture_ground_decay_expire'


class ReviewedSqlWriter(unittest.TestCase):
    def validate(self, sql, function=FUNCTION):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / SQL_PATH
            source.parent.mkdir(parents=True)
            source.write_text(sql)
            old_root = OWNER.ROOT
            try:
                OWNER.ROOT = Path(directory)
                OWNER.validate_sql_writer(SQL_PATH, function)
            finally:
                OWNER.ROOT = old_root

    def test_actual_reviewed_sql_retains_its_fences_and_reciprocity(self):
        self.validate((ROOT / SQL_PATH).read_text())

    def test_missing_authority_clock_and_receipt_proofs_are_rejected(self):
        source = (ROOT / SQL_PATH).read_text()
        for proof in (
            'PERFORM game_furniture_ground_clock_fence(p_node,p_node_revision,p_world,p_channel,p_scope);',
            'timer.deadline_at<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint',
            'ground.runtime_scope_ownership_generation=p_scope',
            'FOR UPDATE OF i,g',
            'CREATE CONSTRAINT TRIGGER game_furniture_carried_decay_proven',
            'w.created_xact_id<>pg_current_xact_id()',
        ):
            with self.subTest(proof=proof):
                self.assertIn(proof, source)
                with self.assertRaises(ValueError):
                    self.validate(source.replace(proof, 'removed_reviewed_proof'))

    def test_arbitrary_nontrigger_sql_writer_is_not_admitted(self):
        with self.assertRaises(ValueError):
            self.validate((ROOT / SQL_PATH).read_text(), 'unreviewed_ground_writer')


if __name__ == '__main__':
    unittest.main()
