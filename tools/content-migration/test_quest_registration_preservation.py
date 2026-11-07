"""No-network guards for retaining separately authored Quest data during legacy regeneration."""
import json
import tempfile
import unittest
from pathlib import Path
import world_project_v2_to_tree as migration


class QuestRegistrationPreservation(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.shard = 'content/quests/definitions/quests-00000-00000.json'
        self.write('content/manifest.json', {'families': {'Quest': {'records': 1, 'index': migration.QUEST_INDEX}}})
        self.write(migration.QUEST_INDEX, {'schema': 'OTERYN_FAMILY_INDEX/v1', 'family': 'Quest', 'record_count': 1, 'shards': [self.shard]})
        self.write(self.shard, {'family': 'Quest', 'shard': {'count': 1}, 'records': [{'definition': {'identity': {'key': 'oteryn:quest.example', 'revision': 'quest-r1'}, 'readiness': 'waiting_data'}}]})

    def write(self, relative, payload):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(payload))

    def test_retains_registration_index_and_shards_without_touching_authored_bytes(self):
        before = {p.relative_to(self.root).as_posix(): p.read_bytes() for p in self.root.rglob('*.json')}
        families, paths = migration.retained_quest_registration(self.root)
        self.assertEqual(families, {'Quest': {'records': 1, 'index': migration.QUEST_INDEX}})
        self.assertEqual(paths, [migration.QUEST_INDEX, self.shard])
        self.assertEqual({p.relative_to(self.root).as_posix(): p.read_bytes() for p in self.root.rglob('*.json')}, before)

    def test_unregistered_family_is_not_synthesized_from_source_files(self):
        self.write('content/manifest.json', {'families': {}})
        self.assertEqual(migration.retained_quest_registration(self.root), ({}, []))

    def test_disagreement_between_index_and_manifest_fails_closed(self):
        self.write('content/manifest.json', {'families': {'Quest': {'records': 2, 'index': migration.QUEST_INDEX}}})
        with self.assertRaisesRegex(RuntimeError, 'QUEST_REGISTRATION_COUNT'):
            migration.retained_quest_registration(self.root)

    def test_missing_shard_records_do_not_preserve_a_false_family_count(self):
        self.write(self.shard, {'family': 'Quest', 'shard': {'count': 0}, 'records': []})
        with self.assertRaisesRegex(RuntimeError, 'QUEST_RECORD_COUNT'):
            migration.retained_quest_registration(self.root)

    def test_source_import_path_cannot_be_registered_as_quest_shard(self):
        self.write(migration.QUEST_INDEX, {'schema': 'OTERYN_FAMILY_INDEX/v1', 'family': 'Quest', 'record_count': 1, 'shards': ['imports/source.json']})
        with self.assertRaisesRegex(RuntimeError, 'QUEST_SHARD_PATH'):
            migration.retained_quest_registration(self.root)


if __name__ == '__main__':
    unittest.main()
