"""Refresh derived imbuement Item fingerprints without changing source evidence.

Both existing evidence builders independently resolve the current named Item
identities. Only canonical definition hashes may differ. All identities, names,
observations, recipes, eligibility, slots and runtime-admission fields must stay
identical before either packet or its derived pin can be written.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
HERE = ROOT / 'tools/content-schema/imbuement-authoring'
sys.path.insert(0, str(HERE))
import binding_evidence
import eligibility_evidence
import imbuement_authoring


def verify_fingerprints_only(before, after, definitions, item=None, path=()):
    if before == after:
        return
    if isinstance(before, dict) and isinstance(after, dict):
        if set(before) != set(after):
            raise ValueError('IMBUEMENT_EVIDENCE_SHAPE:' + repr(path))
        reference = before.get('item_ref')
        if isinstance(reference, dict):
            item = reference.get('key', item)
        for key in before:
            if key in ('definition_sha256', 'canonical_definition_sha256') and before[key] != after[key]:
                if (item not in definitions or not isinstance(before[key], str)
                        or not re.fullmatch('[0-9a-f]{64}', before[key])
                        or after[key] != definitions[item]['definition_sha256']):
                    raise ValueError('IMBUEMENT_EVIDENCE_UNQUALIFIED_FINGERPRINT:' + repr(path + (key,)))
            else:
                verify_fingerprints_only(before[key], after[key], definitions, item, path + (key,))
    elif isinstance(before, list) and isinstance(after, list) and len(before) == len(after):
        for index, (old, new) in enumerate(zip(before, after)):
            verify_fingerprints_only(old, new, definitions, item, path + (index,))
    else:
        raise ValueError('IMBUEMENT_EVIDENCE_SEMANTIC_CHANGE:' + repr(path))


def prepare():
    definitions = binding_evidence.canonical_items()
    packets = {}
    for filename, builder in (
        ('imbuement-bindings.json', lambda old: binding_evidence.build()),
        ('imbuement-eligibility.json', eligibility_evidence.build),
    ):
        path = HERE / 'samples' / filename
        raw = path.read_bytes()
        if hashlib.sha256(raw).hexdigest() != imbuement_authoring.EVIDENCE_PINS[filename]:
            raise ValueError('IMBUEMENT_EVIDENCE_EXISTING_PIN:' + filename)
        before = json.loads(raw)
        after = builder(before)
        verify_fingerprints_only(before, after, definitions)
        if before != after:
            packets[path] = (json.dumps(after, ensure_ascii=False,
                sort_keys=filename == 'imbuement-bindings.json', indent=2)+'\n').encode()
    source_path = HERE / 'imbuement_authoring.py'
    source = source_path.read_text()
    for path, raw in packets.items():
        old = imbuement_authoring.EVIDENCE_PINS[path.name]
        new = hashlib.sha256(raw).hexdigest()
        needle = f'"{path.name}": "{old}"'
        if source.count(needle) != 1:
            raise ValueError('IMBUEMENT_DERIVED_PIN_LOCATION:' + path.name)
        source = source.replace(needle, f'"{path.name}": "{new}"')
    if packets:
        packets[source_path] = source.encode()
    return packets


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    packets = prepare()
    if args.check:
        if packets:
            raise SystemExit('IMBUEMENT_CURRENT_ITEM_FINGERPRINT_DRIFT')
        print('PASS current imbuement Item fingerprints')
        return
    for path, raw in packets.items():
        path.write_bytes(raw)
        print(path.relative_to(ROOT))
    for tool, command in (
        ('imbuement_authoring.py', ['build']),
        ('imbuement_content.py', ['content']),
    ):
        subprocess.run([sys.executable, str(HERE / tool), *command], cwd=ROOT, check=True)

if __name__ == '__main__':
    main()
