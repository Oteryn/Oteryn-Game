#!/usr/bin/env python3
"""Verify the active shared encounter owner against retained source bytes."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import tarfile

ROOT = Path(__file__).resolve().parents[2]
ARCHIVE = 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz'
ARCHIVE_SHA = 'f3fe990f2a72e2779fb360b9c8f98be9d4174b898fa486d4251aa00137c56277'
MANIFEST_SHA = '9823fd5e25796d5cfcd88a586530a60c0d7a9ca8a4f1130df25afbc2fb21b693'
REVISION = '04b83b512114bfd888000d6e1433ed8ecaec7c5b'
SOURCE = 'data-otservbr-global/scripts/quests/the_annihilator/lever.lua'
BLOB = '62e92e8acd676bad545a501c8431c983dcd4edfa'
SOURCE_SHA = '02913ff4a52944f154e0d4148b6d0baf16c9999373fd699bf76a7dd577ed7fa2'
CALLBACKS = 'content/quests/shared-access-callbacks.json'
KEY = 'oteryn:quest-callback/annihilator-entry'


def verify_source(root=ROOT):
    raw = (root / ARCHIVE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != ARCHIVE_SHA:
        raise ValueError('GROUP_ENCOUNTER_SOURCE_ARCHIVE_PIN')
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        manifest = archive.extractfile('corpus-manifest.json').read()
        if hashlib.sha256(manifest).hexdigest() != MANIFEST_SHA:
            raise ValueError('GROUP_ENCOUNTER_SOURCE_MANIFEST_PIN')
        rows = [row for row in json.loads(manifest)['files']
                if row['source'] == 'canary' and row['path'] == SOURCE]
        if len(rows) != 1:
            raise ValueError('GROUP_ENCOUNTER_SOURCE_SELECTION')
        row = rows[0]
        if (row['repository'] != 'opentibiabr/canary' or row['revision'] != REVISION
                or row['git_blob_sha1'] != BLOB or row['sha256'] != SOURCE_SHA
                or row['cache_path'] != 'blobs/' + BLOB):
            raise ValueError('GROUP_ENCOUNTER_SOURCE_BINDING')
        body = archive.extractfile('blobs/' + BLOB).read()
        if (len(body) != row['byte_count'] or hashlib.sha256(body).hexdigest() != SOURCE_SHA
                or hashlib.sha1(f'blob {len(body)}\0'.encode() + body).hexdigest() != BLOB):
            raise ValueError('GROUP_ENCOUNTER_SOURCE_BYTES')


def position(x, y):
    return {'x': x, 'y': y, 'floor': -13}


def annihilator():
    # Exact literals in the verified source: level/daily lines3-5, layout7-20,
    # player vacancy41-45, six native spawns47-50, one-way transform62-78.
    return {
        'key': KEY, 'quest': 'oteryn:quest.the_annihilator', 'trigger': 'use',
        'selector': {'position': position(33226, 31671), 'unique': 30025, 'action': None, 'appearance': 2772},
        'lever': [2772, 2773], 'reversible': False, 'passages': [], 'teleport': None,
        'effects': {'transform': None, 'facing': None, 'magic': None, 'portals': None,
                    'item_cycle': None, 'encounter': {
            'players': [{'from': position(x, 31671), 'to': position(x - 3, 31659)}
                        for x in [33225, 33224, 33223, 33222]],
            'monsters': [{'creature': 'oteryn:creature.angry_demon', 'position': position(x, y)}
                          for x, y in [(33219, 31657), (33221, 31657), (33223, 31659),
                                       (33224, 31659), (33220, 31661), (33222, 31661)]],
            'room_min': position(33217, 31655), 'room_max': position(33225, 31663),
            'minimum_level': 100,
        }},
        'storage': [], 'offerings': [],
        'return_on_failure': False, 'failure_message': None,
        'sources': ['crystal:201189f871d23597b82caf88fd611454a2ddefb8:'
                    'data-global/scripts/quests/the_annihilator/lever.lua:1-101'],
    }


def outputs(root=ROOT):
    verify_source(root)
    document = json.loads((root / CALLBACKS).read_bytes())
    if document['schema'] != 1:
        raise ValueError('GROUP_ENCOUNTER_CALLBACK_SCHEMA')
    callbacks = {row['key']: row for row in document['callbacks']}
    if len(callbacks) != len(document['callbacks']):
        raise ValueError('GROUP_ENCOUNTER_DUPLICATE_CALLBACK')
    # Shared effects.encounter is the published engine owner. Never append a
    # parallel callback or silently replace a changed active encounter.
    if callbacks.get(KEY) != annihilator():
        raise ValueError('GROUP_ENCOUNTER_SHARED_OWNER_DRIFT')
    return {CALLBACKS: (root / CALLBACKS).read_bytes()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    outputs(args.root)
    print('GROUP_ENCOUNTER_SHARED_OWNER_VERIFIED', KEY)


if __name__ == '__main__':
    main()
