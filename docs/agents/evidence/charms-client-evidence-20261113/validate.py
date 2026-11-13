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
FILES += ['active-pass-qualified.json', 'active-private-image-manifest.json',
          'baseline-qualified.json', 'reset-qualified.json',
          'active-source-stage-values.json', 'official-reset-fee-source.json',
          'reset-fee-comparison.json']
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
    active_result = validate_active(data)
    return {'cards': 25, 'major': 14, 'minor': 11, 'value_matches': 25, 'cost_matches': 25,
            'next_value_labels': 10, 'private_image_receipts': 39,
            'scope': 'STATIC_25_PLUS_BOUNDED_RESET_AND_RESTORE_NOT_COMBAT', **active_result}


def normalized_name(name):
    return name.replace("'", '').casefold()


def validate_active(data):
    active = data['active-pass-qualified.json']
    baseline = data['baseline-qualified.json']
    reset = active['reset']
    restored = active['restore']
    images = data['active-private-image-manifest.json']
    images_by_name = {Path(row['private_path']).name: row for row in images}
    require(len(images) == len(images_by_name) == 75 and all(
        SHA256.fullmatch(row['sha256']) and row['bytes'] > 0 for row in images), 'ACTIVE_PRIVATE_PROOFS')
    require(reset['gold_before'] - reset['gold_after'] == reset['paid_gold'] == 5495250
            and reset['gold_before'] == baseline['gold_before_tests']
            and reset['gold_after'] == restored['gold_final'], 'RESET_GOLD_ACCOUNTING')
    require(reset['major_before'] == [baseline['available_major'], baseline['total_major']]
            and reset['minor_before'] == [baseline['available_minor'], baseline['total_minor']]
            and reset['major_after'] == [6914, 6914] and reset['minor_after'] == [100, 100],
            'RESET_POINTS_ACCOUNTING')
    require(restored['paid_resets'] == 1 and restored['assignments_restored'] == 9
            and restored['Fatal_Hold_unassigned_as_before'] is True, 'BOUNDED_ACTION_COUNT')
    transactions = [dict(row, stage=1) for row in restored['initial_pilot']] + restored['level_transactions']
    source_values = data['active-source-stage-values.json']
    require(len(transactions) == len(source_values) == 18, 'ACTIVE_LEVEL_COUNT')
    actual = {(normalized_name(row['charm']), row['stage']): row for row in transactions}
    expected = {(normalized_name(row['name']), stage)
                for row in baseline['major'] + baseline['minor'] for stage in range(1, row['stage'] + 1)}
    require(len(actual) == 18 and actual.keys() == expected, 'RESTORED_STAGE_SET')
    spend = Counter()
    echoes = 100
    for row in source_values:
        transaction = actual[(normalized_name(row['charm']), row['stage'])]
        require(transaction['expected_cost'] == row['source_cost'], 'ACTIVE_SOURCE_COST')
        spend[row['currency']] += row['source_cost']
        if row['currency'] == 'charm_points':
            echoes += {1: 50, 2: 100, 3: 200}[row['stage']]
    require(spend == {'charm_points': 6800, 'minor_charm_echoes': 850}
            and restored['major_level_spend'] == 6800 and restored['minor_level_spend'] == 850,
            'RESTORE_SPEND')
    require(restored['major_available_final'] == baseline['available_major'] == 6914 - 6800
            and restored['minor_available_final'] == baseline['available_minor'] == echoes - 850
            and restored['major_total_final'] == baseline['total_major'] == 6914
            and restored['minor_total_final'] == baseline['total_minor'] == echoes == 900,
            'RESTORE_FINAL_BALANCES')
    assignments = {normalized_name(row['charm']): row['creature']
                   for row in restored['assignment_transactions']}
    before_assignments = {normalized_name(row['name']): row['assigned_name_candidate']
                          for row in baseline['major'] + baseline['minor'] if row['assigned_name_candidate']}
    require(len(assignments) == 9 and assignments == before_assignments, 'RESTORE_ASSIGNMENT_NAMES')
    for row in active['echo_samples']:
        require(row['major_before'] - row['major_after'] == row['cost']
                and row['minor_total_after'] - row['minor_total_before'] == row['grant'], 'ECHO_SAMPLE')
    require([row['grant'] for row in active['echo_samples']] == [50, 100], 'ECHO_SAMPLE')
    required_proofs = {reset['proof'], restored['proof'], *(row['proof'] for row in active['echo_samples'])}
    required_proofs.update(row['file'] for row in baseline['proofs'])
    required_proofs.update(row['confirmation_file'] for row in restored['level_transactions'])
    required_proofs.update(row['confirmation_file'].replace('-confirmation.png', '-result.png')
                           for row in restored['level_transactions'])
    required_proofs.update('restore-assignment-' + row['charm'].lower().replace("'", '').replace(' ', '-')
                           + '-verified.png' for row in restored['assignment_transactions'])
    require(required_proofs <= images_by_name.keys(), 'ACTIVE_PROOF_LINK')
    official = data['official-reset-fee-source.json']
    for level, fee in official['examples_without_discount'].items():
        require(100000 + 11000 * max(int(level) - 100, 0) == fee, 'OFFICIAL_FEE_EXAMPLE')
    fee = data['reset-fee-comparison.json']
    require(fee['tested_level'] == 757 and fee['official_example_derived_base'] == 7327000
            and fee['effective_25_percent_discount_fee'] == fee['observed_paid_fee'] == reset['paid_gold']
            and fee['effective_25_percent_discount_fee'] == 7327000 * 75 // 100
            and fee['current_game_full_level_fee_with_same_discount'] == (100000 + 757 * 11000) * 75 // 100
            and fee['difference'] == 825000, 'RESET_FEE_COMPARISON')
    require(all(fee[key] is False for key in ['first_free_reset_tested', 'all_discount_entitlements_tested',
                'actual_entitlement_identity_verified', 'runtime_fix_included']), 'FEE_SCOPE_BOUNDARY')
    return {'active_private_image_receipts': 75, 'paid_resets': 1, 'level_purchase_actions': 18,
            'assignment_actions': 9, 'restored_major_spend': 6800, 'restored_minor_spend': 850}


def verify_private(data, directory):
    for row in data['private-image-receipts.json'] + data['active-private-image-manifest.json']:
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
    for row in data['active-source-stage-values.json']:
        source = definitions[row['charm']]
        stage = next(stage for stage in source['stages'] if stage['stage'] == row['stage'])
        require(row['source_key'] == source['identity']['key'] and row['currency'] == source['cost_currency']
                and row['source_cost'] == stage['cost'] and row['source_value_percent'] == stage['value'],
                'ACTIVE_SOURCE_VALUES_MISMATCH')


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
