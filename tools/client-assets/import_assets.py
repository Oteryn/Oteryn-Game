#!/usr/bin/env python3
"""Reproduce the flat official asset inventory; optionally decode official sound indirection.

Field numbers: mehah/otclient src/protobuf/sounds.proto, revision cited in README.md.
The original sounds*.dat is REQUIRED; filenames and cue IDs are never inferred.
"""
import argparse
import hashlib
import json
from pathlib import Path

MAX_DATA_BYTES = 8 * 1024 * 1024


def digest(data):
    return hashlib.sha256(data).hexdigest()


def file_digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def plain(name):
    return bool(name) and name not in ('.', '..') and not any(c in name for c in '/\\\0:')


def fields(data):
    """Bounded protobuf reader for the public proto2 sound metadata schema."""
    offset = 0

    def varint():
        nonlocal offset
        value = 0
        for shift in range(0, 70, 7):
            if offset >= len(data):
                raise ValueError('truncated varint')
            byte = data[offset]
            offset += 1
            value |= (byte & 127) << shift
            if byte < 128:
                if value > (1 << 64) - 1:
                    raise ValueError('varint overflow')
                return value
        raise ValueError('varint overflow')

    while offset < len(data):
        tag = varint()
        field, wire = tag >> 3, tag & 7
        if not field:
            raise ValueError('zero protobuf field')
        if wire == 0:
            value = varint()
        elif wire in (1, 2, 5):
            size = varint() if wire == 2 else {1: 8, 5: 4}[wire]
            if size > len(data) - offset:
                raise ValueError('truncated protobuf field')
            value = data[offset:offset + size]
            offset += size
        else:
            raise ValueError('unsupported protobuf wire type')
        yield field, value


def unique(data):
    result = {}
    for key, value in fields(data):
        if key in result:
            raise ValueError('duplicate singular field')
        result[key] = value
    return result


def decode_cues(data):
    if len(data) > MAX_DATA_BYTES:
        raise ValueError('sound metadata exceeds 8 MiB')
    sounds, effects = {}, {}
    for field, record in fields(data):
        if field not in (1, 2):
            continue  # Ambience streams/music templates are not numeric protocol cues.
        row = unique(record)
        key = row.get(1)
        if not isinstance(key, int) or not (0 if field == 1 else 1) <= key <= (2**32 - 1 if field == 1 else 65535):
            raise ValueError('invalid sound/cue ID')
        table = sounds if field == 1 else effects
        if key in table:
            raise ValueError('duplicate sound/cue ID')
        table[key] = row
    filenames = {}
    for key, row in sounds.items():
        name = row.get(2, b'').decode('utf-8')
        if not plain(name) or Path(name).suffix not in ('.ogg', '.wav'):
            raise ValueError('unsafe or unsupported sound filename')
        filenames[key] = name
    cues = {}
    for key, row in effects.items():
        if (5 in row) == (6 in row):
            raise ValueError('cue must have exactly one simple/random effect')
        ids = [unique(row[5]).get(1)] if 5 in row else [v for f, v in fields(row[6]) if f == 1]
        if not ids or len(ids) > 128 or any(i not in filenames for i in ids):
            raise ValueError('cue references missing sound ID')
        cues[str(key)] = [filenames[i] for i in ids]
    if not cues:
        raise ValueError('no numeric sound cues in official metadata')
    return cues


def decode_ambient(data):
    """mehah/otclient def57c15 sounds.proto:29-33: ambient id -> looping_sound_id.

    oteryn-choice: admit the base loop only; delayed client-side embellishments
    are not required to start/replace/stop the server-owned ambient channel.
    """
    if len(data) > MAX_DATA_BYTES:
        raise ValueError('sound metadata exceeds 8 MiB')
    sounds, streams = {}, {}
    for field, record in fields(data):
        if field == 1:
            row = unique(record)
            key = row.get(1)
            if not isinstance(key, int) or key in sounds:
                raise ValueError('invalid or duplicate sound ID')
            name = row.get(2, b'').decode('utf-8')
            if not plain(name) or Path(name).suffix not in ('.ogg', '.wav'):
                raise ValueError('unsafe or unsupported sound filename')
            sounds[key] = name
        elif field == 3:
            row = {}
            for key, value in fields(record):
                if key in (1, 2):
                    if key in row:
                        raise ValueError('duplicate ambient field')
                    row[key] = value
            key = row.get(1)
            if not isinstance(key, int) or not 1 <= key <= 65535 or key in streams:
                raise ValueError('invalid or duplicate ambient ID')
            streams[key] = row.get(2)
    result = {}
    for key, sound in streams.items():
        if sound is None:
            continue
        if sound not in sounds:
            raise ValueError('ambient references missing sound ID')
        result[str(key)] = [sounds[sound]]
    return result


def inventory(source, archive_sha256, sound_data=None):
    rows = []
    for path in sorted(source.iterdir()):
        if path.is_symlink() or not plain(path.name):
            raise ValueError('unsafe source entry')
        if not path.is_file():
            raise ValueError('expected a flat official assets/sounds directory')
        rows.append({'name': path.name, 'bytes': path.stat().st_size, 'sha256': file_digest(path)})
    result = {'schema': 'OTERYN_CLIENT_ASSET_MANIFEST/v1',
              'source': 'owner-supplied local Tibia client assets folder (15.30 target)',
              'archive_sha256': archive_sha256, 'file_count': len(rows),
              'total_bytes': sum(row['bytes'] for row in rows), 'files': rows}
    if sound_data is not None:
        if not plain(sound_data) or not any(r['name'] == sound_data for r in rows):
            raise ValueError('sound metadata must be a file in the source directory')
        metadata = source / sound_data
        if metadata.stat().st_size > MAX_DATA_BYTES:
            raise ValueError('sound metadata exceeds 8 MiB')
        cues = decode_cues(metadata.read_bytes())
        ambient = decode_ambient(metadata.read_bytes())
        names = {r['name'] for r in rows}
        if any(name not in names for variants in [*cues.values(), *ambient.values()] for name in variants):
            raise ValueError('missing official sound file')
        result.update(source='owner-supplied local Tibia client sounds folder (15.30 target)',
                      sound_data=sound_data, sound_cues=cues, ambient_cues=ambient)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', required=True, type=Path)
    parser.add_argument('--archive', required=True, type=Path, help='original owner-packaged archive')
    parser.add_argument('--manifest-out', required=True, type=Path)
    parser.add_argument('--sound-data', help='official sounds*.dat basename inside --source')
    parser.add_argument('--copy-to', type=Path)
    args = parser.parse_args()
    result = inventory(args.source, file_digest(args.archive), args.sound_data)
    if args.copy_to:
        args.copy_to.mkdir(parents=True, exist_ok=True)
        for row in result['files']:
            raw = (args.source / row['name']).read_bytes()
            if len(raw) != row['bytes'] or digest(raw) != row['sha256']:
                raise ValueError('source changed during import')
            (args.copy_to / row['name']).write_bytes(raw)
    args.manifest_out.write_text(json.dumps(result, sort_keys=True, separators=(',', ':')) + '\n')


if __name__ == '__main__':
    main()
