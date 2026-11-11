"""Rebind the strict native fingerprint for only the qualified book speed leaf."""
import argparse
import copy
import json
import re
import questpc2_rotten_fingerprint_rebase as proof
import native_goal_loot_items as source


def refresh(base):
    if proof.git('rev-parse', base).strip() != proof.git('rev-parse', 'HEAD').strip():
        raise ValueError('BOOK_SPEED_BASE_MUST_BE_HEAD')
    before = proof.git('show', f'{base}:{proof.TEST}').decode()
    old_hashes = proof.fingerprints(base)
    if any(f"{name} = '{value}'" not in before for name, value in old_hashes.items()):
        raise ValueError('BOOK_SPEED_PREVIOUS_PROOF_DOES_NOT_REPRODUCE')
    path = 'content/items/definitions/spell-native-profiles.json'
    index = lambda raw: {r['production_definition']['production_key']: r
                         for r in json.loads(raw)['records']}
    old = index(proof.git('show', f'{base}:{path}'))
    current = index((proof.ROOT / path).read_bytes())
    key = 'oteryn:item.tibia.i2824'
    if old.keys() != current.keys() or {k for k in old if old[k] != current[k]} != {key}:
        raise ValueError('BOOK_SPEED_ONLY_ONE_EXISTING_RECORD_MAY_CHANGE')
    expected = copy.deepcopy(old[key])
    if expected['attributes'].get('speed_bonus') is not None:
        raise ValueError('BOOK_SPEED_ACCEPTED_STAT_MUST_NOT_BE_RETUNED')
    expected['attributes']['speed_bonus'] = 0
    qualified = source.profiles(proof.ROOT, {2824})[key]
    if current[key] != expected or qualified['attributes'].get('speed_bonus') != 0:
        raise ValueError('BOOK_SPEED_EXACT_SOURCE_LEAF_REQUIRED')
    hashes = proof.fingerprints()
    if hashes['CANONICAL_OUTSIDE'] != old_hashes['CANONICAL_OUTSIDE']:
        raise ValueError('BOOK_SPEED_CANONICAL_DEFINITIONS_MUST_STAY_IDENTICAL')
    target = proof.ROOT / proof.TEST
    text, count = re.subn(r"^NATIVE_OUTSIDE = '[0-9a-f]{64}'$",
                         f"NATIVE_OUTSIDE = '{hashes['NATIVE_OUTSIDE']}'",
                         target.read_text(), flags=re.M)
    if count != 1:
        raise ValueError('BOOK_SPEED_FINGERPRINT_DECLARATION')
    target.write_text(text)
    print('BOOK_SPEED_BOUNDED_NATIVE_FINGERPRINT', hashes['NATIVE_OUTSIDE'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', required=True)
    refresh(parser.parse_args().base)
