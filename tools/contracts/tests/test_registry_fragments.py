"""REGISTRY-SPLIT-1: the fragment trees assemble to their aggregate and split back exactly."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import registry_fragments as rf  # noqa: E402


def committed(registry: rf.Registry) -> dict[str, list[tuple[str, str]]]:
    fragments = {}
    for key in registry.collections:
        directory = rf.ROOT / rf.collection_dir(registry, key)
        fragments[key] = sorted(
            (path.name, path.read_text(encoding="utf-8")) for path in directory.glob("*.json")
        )
    return fragments


class RegistryFragmentsTests(unittest.TestCase):
    def test_trees_are_valid(self):
        for registry in (rf.PROTOCOL, rf.RESOURCE_LIMITS):
            with self.subTest(registry.base_path):
                self.assertEqual(rf.validate(registry), [])

    def test_split_of_the_aggregate_is_the_committed_tree(self):
        for registry in (rf.PROTOCOL, rf.RESOURCE_LIMITS):
            with self.subTest(registry.base_path):
                text = rf.load_text(registry)
                base, fragments = rf.split(registry, text)
                self.assertEqual(base, (rf.ROOT / registry.base_path).read_text(encoding="utf-8"))
                self.assertEqual({k: sorted(v) for k, v in fragments.items()}, committed(registry))
                self.assertEqual(rf.assemble(base, fragments), text)

    def test_source_files_are_the_base_and_every_fragment(self):
        for registry in (rf.PROTOCOL, rf.RESOURCE_LIMITS):
            with self.subTest(registry.base_path):
                files = rf.source_files(registry)
                self.assertEqual(files[0], registry.base_path)
                count = sum(len(items) for items in committed(registry).values())
                self.assertEqual(len(files), 1 + count)

    def test_split_preserves_sparse_ordinals(self):
        registry = rf.RESOURCE_LIMITS
        base = '{\n  "entries": []\n}\n'
        items = [
            (f"{prefix:04d}-{identifier}.json", json.dumps({"id": identifier}, indent=2) + "\n")
            for prefix, identifier in (
                (1100, "ZETA-RL-01"), (1110, "ALPHA-RL-01"), (1120, "OMEGA-RL-01"),
            )
        ]
        with tempfile.TemporaryDirectory(dir=rf.ROOT) as tmp:
            root = Path(tmp)
            directory = root / rf.collection_dir(registry, "entries")
            directory.mkdir(parents=True)
            (root / registry.base_path).write_text(base, encoding="utf-8")
            for name, body in items:
                (directory / name).write_text(body, encoding="utf-8")
            self.assertEqual(rf.validate(registry, root), [])
            text = rf.load_text(registry, root)
            split_base, fragments = rf.split(registry, text, root)
            self.assertEqual(split_base, base)
            self.assertEqual(fragments, {"entries": items})
            self.assertEqual(rf.assemble(split_base, fragments), text)

    def test_split_allocates_new_ordinals_after_existing_prefixes(self):
        registry = rf.RESOURCE_LIMITS
        base = '{\n  "entries": []\n}\n'
        first = ('1100-EXISTING-RL-01.json', '{\n  "id": "EXISTING-RL-01"\n}\n')
        added = ('1101-NEW-RL-01.json', '{\n  "id": "NEW-RL-01"\n}\n')
        text = rf.assemble(base, {"entries": [first, added]})
        with tempfile.TemporaryDirectory(dir=rf.ROOT) as tmp:
            root = Path(tmp)
            # No tree yet: the initial split still uses dense ordinals.
            _, fragments = rf.split(registry, text, root)
            self.assertEqual(
                [name for name, _ in fragments["entries"]],
                ["0001-EXISTING-RL-01.json", "0002-NEW-RL-01.json"],
            )
            directory = root / rf.collection_dir(registry, "entries")
            directory.mkdir(parents=True)
            (directory / first[0]).write_text(first[1], encoding="utf-8")
            split_base, fragments = rf.split(registry, text, root)
            self.assertEqual(fragments, {"entries": [first, added]})
            self.assertEqual(rf.assemble(split_base, fragments), text)

    def _tree(self, tmp: str, registry: rf.Registry) -> Path:
        root = Path(tmp)
        for rel in rf.source_files(registry):
            target = root / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((rf.ROOT / rel).read_bytes())
        return root

    def test_a_new_fragment_lands_in_order(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self._tree(tmp, rf.PROTOCOL)
            directory = root / rf.collection_dir(rf.PROTOCOL, "state_domains")
            last = json.loads(sorted(directory.glob("*.json"))[-1].read_text(encoding="utf-8"))
            entry = dict(last, id=last["id"] + 1, name="SPLIT_TEST")
            name = f"{entry['id']:04d}-split-test.json"
            (directory / name).write_text(json.dumps(entry, indent=2) + "\n", encoding="utf-8")
            self.assertEqual(rf.validate(rf.PROTOCOL, root), [])
            domains = rf.load(rf.PROTOCOL, root)["state_domains"]
            self.assertEqual(domains[-1]["name"], "SPLIT_TEST")

    def test_a_misnamed_or_duplicate_fragment_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self._tree(tmp, rf.RESOURCE_LIMITS)
            directory = root / rf.collection_dir(rf.RESOURCE_LIMITS, "entries")
            first = sorted(directory.glob("*.json"))[0]
            (directory / "9999-OTHER-ID.json").write_bytes(first.read_bytes())
            errors = rf.validate(rf.RESOURCE_LIMITS, root)
            self.assertTrue(any("<ordinal>-<id>.json" in error for error in errors), errors)
        with tempfile.TemporaryDirectory() as tmp:
            root = self._tree(tmp, rf.RESOURCE_LIMITS)
            directory = root / rf.collection_dir(rf.RESOURCE_LIMITS, "entries")
            first = sorted(directory.glob("*.json"))[0]
            (directory / f"9999-{first.name.split('-', 1)[1]}").write_bytes(first.read_bytes())
            errors = rf.validate(rf.RESOURCE_LIMITS, root)
            self.assertTrue(any("duplicate id" in error for error in errors), errors)


if __name__ == "__main__":
    unittest.main()
