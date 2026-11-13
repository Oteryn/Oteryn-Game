"""Validate the 25-card evidence packet without inferring unobserved gameplay."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess

FILES = ['observations-25.json', 'findings.json', 'qualified-card-index.json',
         'private-image-receipts.json', 'source-receipts.json',
         'compared-source-values.json', 'provenance.json']
SHA256 = re.compile(r'[0-9a-f]{64}\Z')


def require(ok, code):
    if not ok:
        raise ValueError(code)


def load(directory):
    return {name: json.loads((directory / name).read_text(encoding='utf-8')) for name in FILES}


def validate(data):
    provenance = data['provenance.json']
    require(all(provenance.get(key) is False for key in [
        'all_three_stages_ui_verified', 'combat_or_proc_runtime_verified',
        'runtime_fix_included', 'icon_pixel_or_animation_parity_verified',
        'original_assets_published', 'publication_worker_game_input',
    ]), 'EVIDENCE_BOUNDARY')
    observations = data['observations-25.json']
    rows = observations['rows']
    names = {row['name'] for row in rows}
    require(len(rows) == len(names) == observations['count'] == provenance['evidence_count'] == 25,
            'CARD_POPULATION')
    require(Counter(row['category'] for row in rows) == {'major': 14, 'minor': 11}, 'CARD_CATEGORIES')
    index = data['qualified-card-index.json']
    require(index['count'] == 25 and index['major'] == 14 and index['minor'] == 11
            and {row['name'] for row in index['cards']} == names, 'INDEX_POPULATION')
    images = data['private-image-receipts.json']
    image_by_file = {row['private_path']: row for row in images}
    require(len(images) == len(image_by_file) == provenance['private_image_count'] == 39
            and all(SHA256.fullmatch(row['sha256']) and row['bytes'] > 0 for row in images),
            'PRIVATE_PROOFS')
    source_by_name = {row['name']: row for row in data['compared-source-values.json']}
    require(len(source_by_name) == 25 and source_by_name.keys() == names, 'SOURCE_POPULATION')
    for row in rows:
        source = source_by_name[row['name']]
        proof = row['proof']
        require(proof['private_file'] in image_by_file
                and proof['sha256'] == image_by_file[proof['private_file']]['sha256'], 'CARD_PROOF')
        require(row['description_literal_joined_visual_lines'] and row['runtime_effect_verified'] is False,
                'DESCRIPTION_BOUNDARY')
        require(row['source_key'] == source['source_key'] and row['category'] == source['category']
                and row['source_effect'] == source['effect'], 'SOURCE_BINDING')
        require(row['visible_stage_value_percent'] == source['current_or_preview_stage']['value']
                and row['preview_stage_candidate'] == source['current_or_preview_stage']['stage']
                and row['visible_unlock_or_upgrade_cost'] == source['charged_stage']['cost']
                and row['charged_stage_candidate'] == source['charged_stage']['stage'], 'VISIBLE_VALUE_OR_COST')
        require(row['currency'] == source['currency'] == (
            'charm_points' if row['category'] == 'major' else 'minor_charm_echoes'), 'CURRENCY_BOUNDARY')
        if row['button_literal'] == 'Unlock':
            require(row['current_owned_stage_candidate'] == 0
                    and row['preview_stage_candidate'] == row['charged_stage_candidate'] == 1
                    and row['next_value_percent'] is None, 'LOCKED_PREVIEW_NOT_OWNED')
        else:
            require(row['button_literal'] == 'Upgrade to ' + str(row['next_value_percent']) + '%'
                    and row['current_owned_stage_candidate'] == row['preview_stage_candidate']
                    and row['charged_stage_candidate'] == row['preview_stage_candidate'] + 1
                    and row['next_value_percent'] == source['charged_stage']['value'], 'NEXT_STAGE_BOUNDARY')
    require(sum(row['next_value_percent'] is not None for row in rows) == 10, 'NEXT_VALUE_COUNT')
    findings = data['findings.json']['findings']
    require(len(findings) == 6 and all(row['runtime_defect_asserted'] is False for row in findings),
            'FINDING_BOUNDARY')
    for row in data['source-receipts.json']:
        require(row['head'] == provenance['source_head'] and SHA256.fullmatch(row['sha256']), 'SOURCE_RECEIPT')
    return {'cards': 25, 'major': 14, 'minor': 11, 'value_matches': 25, 'cost_matches': 25,
            'next_value_labels': 10, 'private_image_receipts': 39,
            'scope': 'VISIBLE_CURRENT_OR_LOCKED_PREVIEW_AND_ONE_COST_EACH'}


def verify_private(data, directory):
    for row in data['private-image-receipts.json']:
        path = Path(row['private_path']).resolve()
        require(path.is_relative_to(directory.resolve()), 'PRIVATE_PATH_ESCAPE')
        raw = path.read_bytes()
        require(len(raw) == row['bytes'] and hashlib.sha256(raw).hexdigest() == row['sha256'],
                'PRIVATE_BYTES_MISMATCH')


def verify_repository(data, repository):
    receipts = data['source-receipts.json']
    catalogue = None
    for row in receipts:
        raw = subprocess.check_output(['git', 'show', row['head'] + ':' + row['path']], cwd=repository)
        require(hashlib.sha256(raw).hexdigest() == row['sha256'], 'SOURCE_BYTES_MISMATCH')
        if row['path'] == 'content/charms/charms-00000-00024.json':
            catalogue = json.loads(raw)
    require(catalogue is not None, 'SOURCE_CATALOGUE_MISSING')
    definitions = {row['definition']['name']: row['definition'] for row in catalogue['records']}
    for row in data['compared-source-values.json']:
        source = definitions[row['name']]
        require(row['source_key'] == source['identity']['key'] and row['currency'] == source['cost_currency']
                and row['current_or_preview_stage'] in source['stages']
                and row['charged_stage'] in source['stages'] and row['effect'] == source['effect'],
                'SOURCE_VALUES_MISMATCH')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--private-root', type=Path)
    parser.add_argument('--repository', type=Path)
    args = parser.parse_args()
    data = load(Path(__file__).resolve().parent)
    result = validate(data)
    if args.private_root:
        verify_private(data, args.private_root)
        result['private_images_verified'] = True
    if args.repository:
        verify_repository(data, args.repository)
        result['repository_sources_verified'] = True
    print(json.dumps(result, sort_keys=True))
