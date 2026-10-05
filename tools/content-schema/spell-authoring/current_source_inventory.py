"""Read pinned registrar captures without executing Lua or downloading Git objects.

Captures are inputs, not authoring/runtime qualifications. Each capture's SHA is supplied
by its caller; every Lua byte hash and the entire spell/rune directory file coverage
are checked against the immutable donor tree. Separate branches retain separate keys.
"""
from dataclasses import dataclass
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess


@dataclass(frozen=True)
class Donor:
    snapshot: str
    revision: str
    branch: str


CANARY = Donor('canary-main-current', '04b83b512114bfd888000d6e1433ed8ecaec7c5b', 'main')
CRYSTAL_SUMMER = Donor('crystal-summer-current', '00ce02a57ca5a12e48f32a3476e37471167e4c3f', 'summer-update')
CRYSTAL_MAIN = Donor('crystal-main-current', '96d13eff5a1afef17b11b9abc7d574020381cc53', 'main')
DONORS = (CANARY, CRYSTAL_SUMMER, CRYSTAL_MAIN)
ROOTS = ('data/scripts/spells/', 'data/scripts/runes/')


class InventoryError(ValueError):
    pass


def _git(repo, *args):
    # GIT_NO_LAZY_FETCH prevents promisor caches from doing implicit network I/O.
    result = subprocess.run(['git', '-C', str(repo), *args], capture_output=True,
                            env={**os.environ, 'GIT_NO_LAZY_FETCH': '1'}, check=False)
    if result.returncode:
        raise InventoryError('Required local Git object unavailable: ' + result.stderr.decode(errors='replace').strip())
    return result.stdout


def _sha(data):
    return hashlib.sha256(data).hexdigest()


def inventory(repo, donor, capture_path, capture_sha256, *, exclusions=None):
    """Return compact, verified player registrar rows plus explicitly excluded files.

    Capture: JSONL (optionally gzip), one registrar per row; supports existing r28
    field names without retaining its artifacts/schema. `exclusions` maps omitted
    source paths to reasons, established by the caller's census. Unexplained missing
    Lua files fail closed. No aliasing branches or inferring player status from names.
    """
    if not re.fullmatch(r'[0-9a-f]{40}', donor.revision):
        raise InventoryError('Donor requires an immutable Git revision')
    payload = Path(capture_path).read_bytes()
    if _sha(payload) != capture_sha256:
        raise InventoryError('Capture checksum mismatch')
    raw = gzip.decompress(payload) if payload[:2] == b'\x1f\x8b' else payload
    source_rows = [json.loads(line) for line in raw.splitlines() if line.strip()]
    tree = _git(repo, 'ls-tree', '-r', '-z', donor.revision, '--', *(r.rstrip('/') for r in ROOTS))
    blobs = {}
    for entry in tree.split(b'\0'):
        if not entry:
            continue
        metadata, path = entry.split(b'\t', 1)
        mode, kind, blob = metadata.decode().split()
        path = path.decode()
        if path.endswith('.lua'):
            if kind != 'blob' or mode != '100644':
                raise InventoryError('Non-regular source file: ' + path)
            blobs[path] = blob
    cached, rows, keys, covered = {}, [], set(), set()
    for record in source_rows:
        if record['snapshot'] != donor.snapshot:
            continue
        path = record['source_file']
        if record['source_revision'] != donor.revision or path not in blobs:
            raise InventoryError('Capture donor/path mismatch: ' + path)
        if path not in cached:
            cached[path] = _git(repo, 'cat-file', 'blob', blobs[path])
        data = cached[path]
        if _sha(data) != record['source_sha256'] or record.get('git_blob', blobs[path]) != blobs[path]:
            raise InventoryError('Source checksum mismatch: ' + path)
        key = record['registration_key']
        if key.startswith(path + '#'):
            key = donor.snapshot + '/' + key
        if not re.fullmatch(re.escape(donor.snapshot + '/' + path) + r'#[0-9]+', key) or key in keys:
            raise InventoryError('Invalid or repeated registration identity: ' + key)
        excluded = record.get('excluded_reason')
        if not record['engine_enabled_path']:
            excluded = excluded or 'engine_disabled_path'
        carrier, name = record['logical_key'] or (None, None)
        if carrier not in ('instant', 'rune') and not (carrier in (None, '@spell_instant', '@spell_rune') and excluded):
            raise InventoryError('Unexpected carrier: ' + str(carrier))
        keys.add(key)
        covered.add(path)
        rows.append(dict(snapshot=donor.snapshot, source_revision=donor.revision,
                         source_branch=donor.branch, source_file=path, source_sha256=_sha(data),
                         git_blob=blobs[path], registration_key=key, carrier=carrier, name=name,
                         registrar=record['registrar'], engine_enabled_path=record['engine_enabled_path'],
                         excluded_reason=excluded, capture_sha256=capture_sha256))
    for path, reason in (exclusions or {}).items():
        if path not in blobs or path in covered or not reason:
            raise InventoryError('Invalid exclusion: ' + path)
        data = _git(repo, 'cat-file', 'blob', blobs[path])
        covered.add(path)
        rows.append(dict(snapshot=donor.snapshot, source_revision=donor.revision,
                         source_branch=donor.branch, source_file=path, source_sha256=_sha(data),
                         git_blob=blobs[path], registration_key=None, carrier=None, name=None,
                         registrar=None, engine_enabled_path=False, excluded_reason=reason,
                         capture_sha256=capture_sha256))
    if covered != set(blobs):
        raise InventoryError('Unexplained source files: ' + ', '.join(sorted(set(blobs) - covered)))
    return sorted(rows, key=lambda row: (row['source_file'], row['registration_key'] or ''))
