"""Closed RAW evidence import; existing ImportBatch/ReimportFieldState only."""
import argparse
import hashlib
import json
from pathlib import Path

CAPTURE_SHA = '826e714bf91656fddb36eafe493ad3b1d6295fb530bd0a24f972c88410de65f4'
BATCH_ID = 'g4-item-fx-audio295-raw-evidence-r1'
PROFILE = 'OTERYN_ITEM_FX_AUDIO_RAW_REIMPORT_EVIDENCE/v1'
RECORD_PATH = 'source-evidence.item-fx-audio.raw-source-local-record'
CONTEXT_PATH = 'source-evidence.item-fx-audio.raw-contextual-frame'
ITEM_COUNT = 295
STATE_COUNT = 296


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def value(raw):
    return {'type': 'Text', 'value': canonical(raw).decode()}


def decide(baseline, upstream, local):
    eq = lambda a, b: canonical(a) == canonical(b)
    if eq(upstream, baseline) and eq(local, baseline):
        return 'Unchanged'
    if not eq(upstream, baseline) and eq(local, baseline):
        return 'AdoptUpstream'
    if eq(upstream, baseline) and not eq(local, baseline):
        return 'RetainLocal'
    if eq(upstream, local):
        return 'Converged'
    return 'Conflict'


def state(identity, field, raw):
    return {'stable_identity': identity, 'field_path': field, 'baseline': None,
            'upstream': value(raw), 'local': None, 'decision': 'AdoptUpstream'}


def validate_sources(capture, bundle):
    if bundle.get('schema') != 'OTERYN_RETAINED_PUBLIC_RAW_SOURCE_FILES/v1':
        raise ValueError('wrong raw transport schema')
    keyed = lambda r: (r['repository'], r['revision'], r['repository_path'])
    expected = {keyed(r): r for r in capture['source_files']}
    supplied = {keyed(r): r for r in bundle['source_files']}
    if len(supplied) != len(bundle['source_files']) or set(supplied) != set(expected):
        raise ValueError('missing/duplicate/outside source file')
    for key, source in supplied.items():
        if not isinstance(source.get('raw_utf8'), str):
            raise ValueError('source bytes must be UTF8 text')
        raw = source['raw_utf8'].encode()
        if digest(raw) != expected[key]['sha256'] or len(raw) != expected[key]['bytes']:
            raise ValueError('retained full source bytes opposed')


def build_batch(raw_capture, bundle):
    if digest(raw_capture) != CAPTURE_SHA:
        raise ValueError('unsealed raw capture')
    capture = json.loads(raw_capture)
    validate_sources(capture, bundle)
    rows = capture['per_numeric_source_item_rows']
    ids = [r['source_item_id'] for r in rows]
    if len(ids) != ITEM_COUNT or len(set(ids)) != ITEM_COUNT or any(type(i) is not int for i in ids):
        raise ValueError('closed295 IDs invalid')
    context = {k: v for k, v in capture.items() if k != 'per_numeric_source_item_rows'}
    states = [state('fx-audio295:context', CONTEXT_PATH, context)]
    for row in rows:
        states.append(state(f"fx-audio295:source-local-id:{row['source_item_id']:08d}", RECORD_PATH, row))
    states.sort(key=lambda r: (r['stable_identity'], r['field_path']))
    return {'batch_id': BATCH_ID,
            'source_repository': 'zimbadev/crystalserver;opentibiabr/canary',
            'source_revision': 'retained-fx-audio-staging:' + CAPTURE_SHA,
            'source_artifact_sha256': CAPTURE_SHA, 'access_disposition': 'PENDING',
            'source_generation_profile': PROFILE,
            'importer': 'OTERYN_RETAINED_FX_AUDIO_RAW_EVIDENCE_CAPTURE/v1',
            'mapper': 'ITEM_FX_AUDIO_RAW_REIMPORT_SPLIT/v1',
            'mapper_revision': 'raw-reimport-split-r1',
            'mapper_sha256': digest(Path(__file__).read_bytes()),
            'candidates': [], 'reimport_states': states}


def reconstruct(batch):
    expected = {'batch_id': BATCH_ID,
                'source_repository': 'zimbadev/crystalserver;opentibiabr/canary',
                'source_revision': 'retained-fx-audio-staging:' + CAPTURE_SHA,
                'source_artifact_sha256': CAPTURE_SHA, 'access_disposition': 'PENDING',
                'source_generation_profile': PROFILE,
                'importer': 'OTERYN_RETAINED_FX_AUDIO_RAW_EVIDENCE_CAPTURE/v1',
                'mapper': 'ITEM_FX_AUDIO_RAW_REIMPORT_SPLIT/v1',
                'mapper_revision': 'raw-reimport-split-r1',
                'mapper_sha256': digest(Path(__file__).read_bytes())}
    if set(batch) != set(expected) | {'candidates', 'reimport_states'} or any(
            canonical(batch[k]) != canonical(v) for k, v in expected.items()):
        raise ValueError('raw import provenance/header opposed')
    if batch['candidates'] or len(batch['reimport_states']) != STATE_COUNT:
        raise ValueError('raw evidence shape invalid')
    context = None
    rows = []
    pairs = []
    for r in batch['reimport_states']:
        pairs.append((r['stable_identity'], r['field_path']))
        if r['baseline'] is not None or r['local'] is not None or r['upstream']['type'] != 'Text':
            raise ValueError('source-state ownership changed')
        if r['decision'] != decide(r['baseline'], r['upstream'], r['local']):
            raise ValueError('false three-way decision')
        raw = json.loads(r['upstream']['value'])
        if r['field_path'] == CONTEXT_PATH and context is None and r['stable_identity'] == 'fx-audio295:context':
            context = raw
        elif r['field_path'] == RECORD_PATH:
            if r['stable_identity'] != f"fx-audio295:source-local-id:{raw['source_item_id']:08d}":
                raise ValueError('source ID selector changed')
            rows.append(raw)
        else:
            raise ValueError('unowned field selector')
    if pairs != sorted(set(pairs)) or context is None or len(rows) != ITEM_COUNT:
        raise ValueError('duplicate/unsorted/missing raw source field')
    context['per_numeric_source_item_rows'] = sorted(rows, key=lambda r: r['source_item_id'])
    raw = (json.dumps(context, ensure_ascii=False, sort_keys=True, indent=2) + '\n').encode()
    if digest(raw) != CAPTURE_SHA:
        raise ValueError('lossless reconstruction differs')
    return raw


def append_batch(import_document, batch):
    reconstruct(batch)
    old = import_document['batches']
    same = [b for b in old if b['batch_id'] == BATCH_ID]
    if same:
        if len(same) != 1 or canonical(same[0]) != canonical(batch):
            raise ValueError('conflicting existing raw batch')
        return import_document
    result = dict(import_document)
    result['batches'] = sorted(old + [batch], key=lambda b: b['batch_id'])
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--capture', type=Path, required=True)
    parser.add_argument('--sources', type=Path, required=True)
    parser.add_argument('--batch', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    result = build_batch(args.capture.read_bytes(), json.loads(args.sources.read_bytes()))
    reconstruct(result)
    output = (json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2) + '\n').encode()
    if args.check:
        if args.batch.read_bytes() != output:
            raise ValueError('sealed batch bytes drift')
    else:
        args.batch.write_bytes(output)


if __name__ == '__main__':
    main()
