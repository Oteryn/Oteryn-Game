"""Validate retained source/candidate boundaries; optionally verify private proofs."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess

FILES = [
    'summary.json', 'client1533-creatures.json', 'staticdata-delta.json',
    'binding-candidates.json', 'outfit-metadata-coverage.json',
    'bestiary-824-field-presence.json', 'source-receipts.json',
    'ui-class-counts.json', 'ui-rat-control-observation.json',
    'rat-repo-context.json', 'provenance.json', 'private-image-receipts.json',
    'gap-queue.json',
    'qualified-reader-checkpoint.json', 'continuation-private-image-receipts.json',
]
HASH = re.compile(r'[0-9a-f]{64}\Z')


def require(ok, code):
    if not ok:
        raise ValueError(code)


def load(directory):
    return {name: json.loads((directory / name).read_text(encoding='utf-8')) for name in FILES}


def validate(data):
    provenance = data['provenance.json']
    require(all(provenance.get(key) is False for key in [
        'runtime_activation', 'server_identity_mapping_verified',
        'all_845_cards_ui_audited', 'pixel_or_animation_parity_verified',
    ]), 'AUTHORITY_BOUNDARY')
    summary = data['summary.json']
    rows = data['client1533-creatures.json']['records']
    ids = {row['source_id'] for row in rows}
    require(len(rows) == len(ids) == summary['client_races'] == 845, 'CLIENT_POPULATION')
    require(all(HASH.fullmatch(row['staticdata_sha256']) for row in rows), 'CLIENT_ROW_HASH')
    require(data['client1533-creatures.json']['file_sha256'] == summary['client_staticdata_sha256']
            == provenance['staticdata_sha256'], 'CLIENT_FILE_HASH')
    classes = data['client1533-creatures.json']['classes']
    require(len(classes) == len({row['source_id'] for row in classes}) == 21, 'CLIENT_CLASSES')
    delta = data['staticdata-delta.json']
    added = delta['added']
    by_id = {row['source_id']: row for row in rows}
    require(delta['old_count'] == delta['common_ids'] == delta['byte_identical_common'] == 833
            and delta['new_count'] == 845 and not delta['removed'] and not delta['changed']
            and len(added) == len({row['source_id'] for row in added}) == 12
            and all(by_id.get(row['source_id']) == row for row in added), 'STATICDATA_DELTA')
    bindings = data['binding-candidates.json']
    require(len(bindings) == 845 and {row['client_source_id'] for row in bindings} == ids,
            'BINDING_POPULATION')
    expected = {
        'NAME_AND_OUTFIT_CANDIDATE_NOT_ID_AUTHORITY': 813,
        'NEEDS_BINDING_REVIEW': 32,
    }
    require(dict(Counter(row['status'] for row in bindings)) == expected, 'BINDING_BOUNDARY')
    for row in bindings:
        candidates = [candidate for candidate in row['candidates']
                      if candidate['has_bestiary'] and candidate['asset_outfit_matches']]
        require((len(candidates) == 1) == (row['status'] == 'NAME_AND_OUTFIT_CANDIDATE_NOT_ID_AUTHORITY'),
                'BINDING_CANDIDATE')
    frames = data['outfit-metadata-coverage.json']
    require(len(frames) == 845 and {row['client_source_id'] for row in frames} == ids,
            'OUTFIT_POPULATION')
    require(sum(row['status'] == 'METADATA_ONLY_NO_PIXEL_OR_UI_ANIMATION_PARITY'
                for row in frames) == 844, 'OUTFIT_BOUNDARY')
    ui_classes = data['ui-class-counts.json']
    require(len(ui_classes) == len({row['class'] for row in ui_classes}) == 21
            and sum(row['ui_total'] for row in ui_classes) == 845
            and all(0 <= row['ui_known'] <= row['ui_total'] for row in ui_classes), 'UI_CLASSES')
    coverage = data['bestiary-824-field-presence.json']
    require(coverage['denominator'] == summary['canonical_bestiary'] == 824
            and all(0 <= n <= 824 for n in coverage['counts'].values()), 'CANONICAL_POPULATION')
    images = data['private-image-receipts.json']['images']
    image_by_path = {row['path']: row for row in images}
    require(len(image_by_path) == len(images) and all(HASH.fullmatch(row['sha256'])
            and row['bytes'] > 0 for row in images), 'PRIVATE_PROOF')
    rat = data['ui-rat-control-observation.json']
    required_proofs = {rat['proof'], *rat['locations']['proofs'],
                       *(row['proof'] for row in ui_classes)}
    require(required_proofs <= image_by_path.keys(), 'PRIVATE_PROOF')
    require(rat['scope'] == 'ONE_CONTROL_CARD_NOT_ALL_BESTIARY'
            and rat['kill_thresholds'] == [10, 100, 250], 'UI_SAMPLE_BOUNDARY')
    require(rat['locations']['status'] == 'SOURCE_TEXT_DIFFERENCE_CONFIRMED_FULL_LITERAL_TRANSCRIPTION_PENDING',
            'LOCATION_TRANSCRIPTION_BOUNDARY')
    require(all(HASH.fullmatch(row['sha256']) and row['commit'] == provenance['comparison_commit']
                for row in data['source-receipts.json']), 'REPOSITORY_SOURCE_RECEIPT')
    checkpoint = data['qualified-reader-checkpoint.json']
    require(checkpoint['status'] == 'BESTIARY_PAUSED_OWNER_SWITCHED_TO_CHARMS'
            and checkpoint['next_scope'] == 'Charms; no further Bestiary collection', 'FINAL_SCOPE_BOUNDARY')
    later_images = data['continuation-private-image-receipts.json']
    later_by_path = {row['private_path']: row for row in later_images}
    require(len(later_images) == len(later_by_path) == 78
            and all(HASH.fullmatch(row['sha256']) and row['bytes'] > 0 for row in later_images),
            'CONTINUATION_PRIVATE_PROOF')
    searches = checkpoint['searches']
    require(len(searches) == 19, 'CONTINUATION_SEARCH_COUNT')
    for row in searches:
        require(row['file'] in later_by_path
                and row['sha256'] == later_by_path[row['file']]['sha256'], 'CONTINUATION_PRIVATE_PROOF')
        require(not row['visually_confirmed_result'] or row['query'] in {'nomad', 'butterfly'},
                'UNQUALIFIED_SEARCH_RESULT')
    require(sum(row['visually_confirmed_result'] for row in searches) == 3,
            'QUALIFIED_SEARCH_COUNT')
    wafer = checkpoint['wafer_paper_butterfly']
    require(wafer['file'] in later_by_path and wafer['sha256'] == later_by_path[wafer['file']]['sha256']
            and wafer['kill_thresholds'] == 'NOT_READ'
            and wafer['loot'] == 'NO_VISIBLE_ROWS_ONLY_NOT_NO_LOOT_AUTHORITY',
            'WAFER_EVIDENCE_BOUNDARY')
    return {'client_races': len(rows), 'canonical_bestiary': 824, 'new_rows': 12,
            'binding_candidates': 813, 'binding_review': 32,
            'private_image_receipts': len(images) + len(later_images),
            'qualified_continuation_search_results': 3,
            'scope': 'SOURCE_INVENTORY_AND_PARTIAL_UI_GAP_PASS_ONLY'}


def verify_private(data, directory):
    for row in data['private-image-receipts.json']['images']:
        path = (directory / row['path']).resolve()
        require(path.is_relative_to(directory.resolve()), 'PRIVATE_PATH_ESCAPE')
        raw = path.read_bytes()
        require(len(raw) == row['bytes'] and hashlib.sha256(raw).hexdigest() == row['sha256'],
                'PRIVATE_BYTES_MISMATCH')
    for row in data['continuation-private-image-receipts.json']:
        path = Path(row['private_path']).resolve()
        require(path.is_relative_to(directory.resolve()), 'PRIVATE_PATH_ESCAPE')
        raw = path.read_bytes()
        require(len(raw) == row['bytes'] and hashlib.sha256(raw).hexdigest() == row['sha256'],
                'CONTINUATION_PRIVATE_BYTES_MISMATCH')


def verify_repository(data, repository):
    for row in data['source-receipts.json']:
        raw = subprocess.check_output(['git', 'show', row['commit'] + ':' + row['path']], cwd=repository)
        require(hashlib.sha256(raw).hexdigest() == row['sha256'], 'REPOSITORY_BYTES_MISMATCH')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--private-root', type=Path)
    parser.add_argument('--repository', type=Path)
    args = parser.parse_args()
    packet = load(Path(__file__).resolve().parent)
    result = validate(packet)
    if args.private_root:
        verify_private(packet, args.private_root)
        result['private_bytes_verified'] = True
    if args.repository:
        verify_repository(packet, args.repository)
        result['repository_blobs_verified'] = True
    print(json.dumps(result, sort_keys=True))
