"""Read prior profile ownership from immutable published source, not generated output."""
import hashlib
import json
import re
import subprocess

PATHS = (
    'content/items/definitions/spell-native-profiles.json',
    'content/items/definitions/srx1-item-updates.json',
    'content/items/definitions/starting-loadout-profiles.json',
)


def load(root, revision):
    if not re.fullmatch('[0-9a-f]{40}', revision):
        raise ValueError('profile baseline requires an immutable commit')
    keys, files = set(), []
    for path in PATHS:
        raw = subprocess.check_output(['git', '-C', str(root), 'show', revision + ':' + path])
        rows = [r['production_definition']['production_key'] for r in json.loads(raw)['records']]
        if len(rows) != len(set(rows)):
            raise ValueError('duplicate profile in published baseline')
        keys.update(rows)
        files.append({'path': path, 'sha256': hashlib.sha256(raw).hexdigest()})
    return keys, {'revision': revision, 'files': files}
