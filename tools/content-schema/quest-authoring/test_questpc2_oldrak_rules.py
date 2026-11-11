import io
import json
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import questpc2_oldrak_rules as source

ROOT = Path(__file__).resolve().parents[3]


class OldrakAuthoringProof(unittest.TestCase):
    def test_committed_program_reproduces_from_exact_retained_source(self):
        outputs = source.outputs(ROOT)
        self.assertEqual(set(outputs), {source.OUTPUT})
        self.assertEqual((ROOT / source.OUTPUT).read_text(), outputs[source.OUTPUT])
        doc = json.loads(outputs[source.OUTPUT])
        self.assertEqual(doc['source_files'], [{'path': source.SOURCE, 'sha256': source.SHA,
                                              'revision': source.REV}])
        self.assertEqual({r['npc'] for r in doc['rules']}, {'oteryn:npc.oldrak'})

    def test_modified_retained_source_cannot_author_a_program(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz'
            path.parent.mkdir(parents=True)
            manifest = {'files': [{'revision': source.REV, 'path': source.SOURCE,
                                  'sha256': source.SHA, 'cache_path': 'blobs/oldrak'}]}
            with tarfile.open(path, 'w:gz') as archive:
                for name, data in [('corpus-manifest.json', json.dumps(manifest).encode()),
                                   ('blobs/oldrak', b'modified source')]:
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    archive.addfile(info, io.BytesIO(data))
            with self.assertRaisesRegex(ValueError, 'OLDRAK_SOURCE_PIN'):
                source.outputs(root)
            self.assertFalse((root / source.OUTPUT).exists())


if __name__ == '__main__':
    unittest.main()
