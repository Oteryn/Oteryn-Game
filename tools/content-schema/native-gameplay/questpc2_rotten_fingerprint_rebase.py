"""Rebind strict outside-scope fingerprints only during an explicit main merge."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
TEST = 'tools/content-schema/native-gameplay/test_questpc2_rotten_loot_items.py'
KEYS = {f'oteryn:item.tibia.i{i}' for i in (43501, 43502, 43503, 43504, 43899, 43900)}


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)


def fingerprints(revision=None):
    paths = git('ls-tree', '-r', '--name-only', revision, '--',
                'content/items/definitions').decode().splitlines() if revision else [
                    str(p.relative_to(ROOT)) for p in (ROOT / 'content/items/definitions').iterdir()]
    read = lambda path: git('show', f'{revision}:{path}') if revision else (ROOT / path).read_bytes()
    canonical = {r['definition']['identity']['key']: r['definition']
                 for path in paths if re.fullmatch(r'content/items/definitions/items-.*\.json', path)
                 for r in json.loads(read(path))['records']}
    native = {r['production_definition']['production_key']: r for r in json.loads(
        read('content/items/definitions/spell-native-profiles.json'))['records']}
    digest = lambda rows: hashlib.sha256(json.dumps({k: v for k, v in rows.items() if k not in KEYS},
        sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    return dict(CANONICAL_OUTSIDE=digest(canonical), NATIVE_OUTSIDE=digest(native))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--main', required=True)
    args = parser.parse_args()
    if git('rev-parse', 'MERGE_HEAD').decode().strip() != git('rev-parse', args.main).decode().strip():
        raise SystemExit('Fingerprint rebase requires the exact active main merge')
    subprocess.run(['git', 'merge-base', '--is-ancestor', args.main, 'origin/main'], cwd=ROOT, check=True)
    before = git('show', f'HEAD:{TEST}').decode()
    for name, value in fingerprints('HEAD').items():
        if f"{name} = '{value}'" not in before:
            raise SystemExit('Previous committed outside-scope proof does not reproduce')
    target = ROOT / TEST
    text = target.read_text()
    for name, value in fingerprints().items():
        text, count = re.subn(rf"^{name} = '[0-9a-f]{{64}}'$", f"{name} = '{value}'", text, flags=re.M)
        if count != 1:
            raise SystemExit('Outside-scope fingerprint declaration differs')
        print(name, value)
    target.write_text(text)
