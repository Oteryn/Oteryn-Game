"""Stage exact external completion fixtures from verified public pinned checkouts."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import subprocess

from convert_spells import SOURCES

PREFIX = PurePosixPath('tools/content-schema/spell-authoring/fixtures/completion-sources')

def verified_payload(entry, raw):
    if len(raw) != entry['bytes'] or hashlib.sha256(raw).hexdigest() != entry['sha256']:
        raise ValueError('Completion fixture byte count/SHA256 differs: ' + entry['source_path'])
    blob = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
    if blob != entry['git_blob']:
        raise ValueError('Completion fixture Git blob differs: ' + entry['source_path'])
    return raw

def stage(manifest_path, checkout_root, source_root):
    manifest = json.loads(Path(manifest_path).read_text())
    if manifest.get('schema') != 'OTERYN_SPELL_TEST_EXTERNAL_FIXTURES/v1' or len(manifest.get('entries', [])) != 6:
        raise ValueError('Expected exact six-fixture external manifest')
    checkout_root = Path(checkout_root).resolve()
    pending = []; seen = set()
    for name, pin in SOURCES.items():
        actual = subprocess.check_output(['git', '-C', str(Path(source_root) / name), 'rev-parse', 'HEAD'], text=True).strip()
        if actual != pin['revision']:
            raise ValueError('Source checkout revision differs: ' + name)
    for entry in manifest['entries']:
        name = next((n for n,p in SOURCES.items() if p['repository'] == entry['source_repository']), None)
        if name is None or entry['source_revision'] != SOURCES[name]['revision']:
            raise ValueError('Unknown or unpinned completion source')
        path = PurePosixPath(entry['source_path'])
        local = PurePosixPath(entry['local_path'])
        if path.is_absolute() or '..' in path.parts or local != PREFIX / name / path or local in seen:
            raise ValueError('Invalid or duplicate completion fixture path')
        seen.add(local)
        expected_url = f"https://raw.githubusercontent.com/{entry['source_repository']}/{entry['source_revision']}/{path}"
        if entry['url'] != expected_url:
            raise ValueError('Completion fixture URL is not exact pinned source')
        destination = checkout_root / local
        if not destination.resolve().is_relative_to(checkout_root):
            raise ValueError('Completion fixture destination escapes checkout')
        raw = subprocess.check_output(['git', '-C', str(Path(source_root) / name), 'show', f'HEAD:{path}'])
        pending.append((destination, verified_payload(entry, raw)))
    # Verify all inputs before changing any fixture.
    for destination, raw in pending:
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(raw)
    return len(pending)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest', type=Path, required=True)
    parser.add_argument('--checkout-root', type=Path, required=True)
    parser.add_argument('--sources', type=Path, required=True)
    args = parser.parse_args()
    print(f'Staged {stage(args.manifest, args.checkout_root, args.sources)} exact pinned completion fixtures')

if __name__ == '__main__':
    main()
