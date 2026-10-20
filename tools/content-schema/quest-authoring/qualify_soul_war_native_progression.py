"""Run the isolated Soul War fixture through unchanged native QuestState source.

Requires the repository-pinned Rust toolchain and cached locked dependencies.
A temporary Cargo package path-loads the real native module; no engine copy,
workspace registration, production catalogue mutation or database is involved.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib

import soul_war_native_progression as fixture


def qualify(root: Path, scratch: Path):
    here = Path(__file__).resolve().parent
    rust = here / 'native_soul_war_progression.rs'
    inputs = [root / 'Cargo.toml', root / 'Cargo.lock', root / 'rust-toolchain.toml',
              rust, root / fixture.OUTPUT, root / fixture.PACKET,
              root / 'content/quests/missions/quest-state.json',
              *sorted((root / 'apps/game-server/src/quest').glob('*.rs'))]

    def snapshot():
        return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in inputs}

    before = snapshot()
    if (root / fixture.OUTPUT).read_bytes() != fixture.encoded(fixture.expected(root)):
        raise ValueError('regenerate the stale qualification input before native checks')
    source_lock = tomllib.loads((root / 'Cargo.lock').read_text(encoding='utf-8'))
    source_packages = {(p['name'], p['version'], p.get('source'), p.get('checksum'))
                       for p in source_lock['package']}
    workspace = (root / 'Cargo.toml').read_text(encoding='utf-8')
    channel = tomllib.loads((root / 'rust-toolchain.toml').read_text(encoding='utf-8'))['toolchain']['channel']
    manifest = ('[package]\nname="soul-war-native-qualification"\nversion="0.0.0"\n'
                'edition="2024"\n[workspace]\n[lib]\npath=' + json.dumps(rust.as_posix())
                + '\n[dependencies]\n')
    # Only these directly pinned workspace dependencies are needed by the real module.
    for name in ('sha2', 'serde', 'serde_json'):
        matches = re.findall(r'^' + name + r'\s*=\s*(?:\{\s*version\s*=\s*)?"(=[^"]+)"', workspace, re.M)
        if len(matches) != 1:
            raise ValueError('expected one exact workspace dependency pin: ' + name)
        features = ',features=["derive"]' if name == 'serde' else (',default-features=false' if name == 'sha2' else '')
        manifest += name + '={version=' + json.dumps(matches[0]) + features + '}\n'
    (scratch / 'Cargo.toml').write_text(manifest, encoding='utf-8')
    shutil.copyfile(root / 'Cargo.lock', scratch / 'Cargo.lock')
    env = {**os.environ, 'CARGO_TARGET_DIR': str(scratch / 'target')}
    cargo = ['cargo', '+' + channel]
    # Resolve/prune only the temporary package lock; reject dependency versions absent from the source lock.
    subprocess.run([*cargo, 'metadata', '--offline', '--format-version=1'],
                   cwd=scratch, env=env, stdout=subprocess.DEVNULL, check=True)
    locked = tomllib.loads((scratch / 'Cargo.lock').read_text(encoding='utf-8'))
    for package in locked['package']:
        if package['name'] == 'soul-war-native-qualification':
            continue
        identity = (package['name'], package['version'], package.get('source'), package.get('checksum'))
        if identity not in source_packages:
            raise ValueError('qualification dependency differs from repository lock: ' + repr(identity))
    command = [*cargo, 'test', '--frozen', '--lib', '--quiet']
    result = subprocess.run(command, cwd=scratch, env=env, capture_output=True)
    print(result.stdout.decode('utf-8', 'replace'), end='')
    print(result.stderr.decode('utf-8', 'replace'), end='')
    if snapshot() != before:
        raise ValueError('native qualification changed a source input')
    return {'command': command, 'exit_code': result.returncode, 'source_sha256': before,
            'dependencies_are_repository_locked': True, 'inputs_unchanged': True,
            'scope': 'native loader/reducer unit qualification; no SQL, dispatch, world or gameplay proof',
            'runtime_admitted': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--report', type=Path, help='Optional qualification result outside the repository')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[3]
    if args.report and args.report.resolve().is_relative_to(root):
        parser.error('write qualification reports outside the repository')
    with tempfile.TemporaryDirectory(prefix='soul-war-native-qualification-') as directory:
        result = qualify(root, Path(directory))
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print('Native Soul War qualification exit=' + str(result['exit_code']) + '; runtime admission=false')
    raise SystemExit(result['exit_code'])


if __name__ == '__main__':
    main()
