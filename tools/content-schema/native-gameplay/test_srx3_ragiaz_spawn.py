"""The hidden phase actor has real map XML provenance, not a guessed bootstrap."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC=importlib.util.spec_from_file_location('ragiaz_spawn',Path(__file__).with_name('build_srx3_ragiaz_spawn.py'))
TOOL=importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TOOL)

class RagiazSourceSpawn(unittest.TestCase):
    def test_xml_offsets_and_respawn_are_source_data(self):
        raw=b'<monsters>\n<monster centerx="33485" centery="32333" centerz="14" radius="1">\n<monster name="Bone Capsule" x="0" y="0" z="14" spawntime="60"/>\n</monster>\n</monsters>'
        row=TOOL.extract(raw)
        self.assertEqual(row['position'],[33485,32333,14])
        self.assertEqual(row['respawn_seconds'],60)
        self.assertEqual(row['creature_key'],'oteryn:creature.bone_capsule')
        self.assertEqual(row['radius'],1)
        self.assertEqual(row['line'],3)

    def test_unreviewed_or_missing_point_refuses(self):
        for raw in (b'<monsters/>',b'<monsters><monster centerx="33485" centery="32333" centerz="14" radius="1"><monster name="Bone Capsule" x="1" y="0" z="14" spawntime="60"/></monster></monsters>'):
            with self.assertRaises(ValueError):TOOL.extract(raw)

    def test_unsigned_fixture_cannot_publish_source_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'world-monster.xml'
            path.write_bytes(b'<monsters/>')
            with self.assertRaisesRegex(ValueError,'pinned Crystal monster XML changed'):TOOL.build(path)

if __name__=='__main__':unittest.main()
