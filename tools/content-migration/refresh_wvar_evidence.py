"""Refresh dependent static evidence only after exact WVAR canonical admission proof."""
from __future__ import annotations

import copy
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[2]
BASELINE = '9668e0a04'


def refresh(root=ROOT):
    archive = subprocess.check_output(['git', '-C', str(root), 'archive', BASELINE,
                                       'content/items/definitions'])
    expected = {}
    with tarfile.open(fileobj=io.BytesIO(archive)) as files:
        for member in files:
            if re.fullmatch(r'content/items/definitions/items-\d+-\d+\.json', member.name):
                for row in json.load(files.extractfile(member))['records']:
                    definition = row['definition']
                    expected[definition['identity']['key']] = copy.deepcopy(definition)
    import wvar_item_profiles
    sys.path.insert(0, str(root / 'tools/content-schema/native-gameplay'))
    import wvar_corpse_items
    wvar_corpse_items.apply_admissions(expected.values(), root)
    wvar_item_profiles.apply_canonical_admissions(list(expected.values()), root)
    actual = {row['definition']['identity']['key']: row['definition']
              for path in (root / 'content/items/definitions').glob('items-*.json')
              for row in json.loads(path.read_bytes())['records']}
    if actual != expected:
        changed = sorted(key for key in actual.keys() | expected.keys()
                         if actual.get(key) != expected.get(key))
        raise ValueError('WVAR_DEPENDENT_EVIDENCE_UNQUALIFIED_CANONICAL_CHANGE:' + ','.join(changed))
    here = root / 'tools/content-schema/imbuement-authoring'
    # These producers retain all identity, source, equipment and runtime guards.
    for producer in ('binding_evidence.py', 'eligibility_evidence.py'):
        subprocess.run([sys.executable, str(here / producer)], cwd=root, check=True)
    authoring = here / 'imbuement_authoring.py'
    source = authoring.read_text()
    for name in ('imbuement-bindings.json', 'imbuement-eligibility.json'):
        digest = hashlib.sha256((here / 'samples' / name).read_bytes()).hexdigest()
        source, count = re.subn(r'("' + re.escape(name) + r'": ")[0-9a-f]{64}("[,])',
                                lambda match: match[1] + digest + match[2], source)
        if count != 1:
            raise ValueError('WVAR_DEPENDENT_EVIDENCE_PIN_LAYOUT')
    authoring.write_text(source)
    subprocess.run([sys.executable, str(authoring), 'build'], cwd=root, check=True)
    print('WVAR_DEPENDENT_EVIDENCE_VERIFIED canonical_items=' + str(len(actual)))


if __name__ == '__main__':
    refresh()
