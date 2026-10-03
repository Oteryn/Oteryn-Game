"""Adapt optional packet receipts and redundant local Loot catalog declarations."""
import argparse
import copy
import hashlib
import json
import shutil
from pathlib import Path


def normalize(packet):
    result = copy.deepcopy(packet)
    local_loot = {p['monster']: p['value']['identity'] for p in result['patches']
                  if p['file'] == 'monster.json' and p['pointer'] == '/loot'}
    removed = []
    kept = []
    for patch in result['patches']:
        identity = local_loot.get(patch['monster'])
        if (identity and patch['file'] == 'catalog.json'
                and patch['pointer'] == '/definitions/-'
                and patch['value'] == dict(identity, family='Loot')):
            removed.append(patch)
        else:
            kept.append(patch)
    result['patches'] = kept
    if removed:
        result['redundant_local_loot_catalog_patches'] = removed
    for row in result.get('restored_source_rows', []):
        if 'limitation' in row:
            continue
        debt = row.get('remaining_exact_parity_debt')
        if not isinstance(debt, str) or not debt.strip():
            raise ValueError('Restored source row lacks an explicit remaining limitation')
        matches = [p['value']['ability'] for p in result['patches']
                   if p['monster'] == row['monster']
                   and p['pointer'] in ('/behavior/attacks/-', '/behavior/defenses/-')
                   and p['source'].get('source_field') == row['source_field']]
        if len(matches) != 1 or matches[0].get('family') != 'Ability':
            raise ValueError('Restored source row requires one exact scheduled Ability')
        row['limitation'] = debt
        row['direct_typed_core'] = copy.deepcopy(matches[0])
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--packet', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise ValueError('Refusing to overwrite a packet')
    original = args.packet.read_bytes()
    result = normalize(json.loads(original))
    result['normalized_from_packet_sha256'] = hashlib.sha256(original).hexdigest()
    result['normalization_scope'] = 'Receipt aliases and redundant local Loot catalog refs only; gameplay values and named-target evidence unchanged.'
    args.output.parent.mkdir(parents=True, exist_ok=True)
    for patch in result['patches']:
        source = patch['source']
        if source.get('kind') != 'oteryn_balance_estimate':
            continue
        name = Path(source['source_file'])
        if name.is_absolute() or '..' in name.parts:
            raise ValueError('Unsafe balance ledger path')
        ledger = args.packet.parent / name
        if hashlib.sha256(ledger.read_bytes()).hexdigest() != source['ledger_sha256']:
            raise ValueError('Balance ledger SHA differs')
        target = args.output.parent / name
        if target.exists():
            if target.read_bytes() != ledger.read_bytes():
                raise ValueError('Refusing to overwrite a differing ledger')
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ledger, target)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')


if __name__ == '__main__':
    main()
