"""Summarise converted NPC bundles into a committed readiness census (no text, no dialogue).

Readiness classes per NPC:
- STATIC_COMPLETE: RESOLVED; every definition, dialogue, service and placement fact is static.
- STATIC_SERVICES: PARTIAL, but every service row (trade, travel, spells, blessings, promotion) is
  ungated and every placement spawns; the open rows are dialogue scripting or presentation only.
- SCRIPTED: PARTIAL with at least one gated service row or Lua-only behaviour that needs a decision.
- UNPLACED: registered but never placed by the datapack's spawn files.
- LOAD_ERROR / NOT_AN_NPC: the sandbox could not register an NpcType.

`bundle_digest` is the SHA-256 over the sorted `<file name>:<sha256 of file>` lines, so a repeat
conversion at the same pins can be checked byte for byte.

Usage: python population_census.py --bundles <out/canary/bundles> --index <out/canary/index.json> --out census.json
"""
import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path

SCHEMA = 'OTERYN_NPC_POPULATION_CENSUS/v1'
SERVICE_KINDS = ('travel', 'spells', 'blessings', 'promotion')
DIALOGUE_ONLY = {'LUA_CALLBACK', 'LUA_PREDICATE', 'LUA_ACTION', 'NON_STATIC_TEXT', 'UNMAPPED_ENGINE_CALL',
                 'MULTIPLE_KEYWORD_HANDLERS', 'UNMAPPED_CONFIG_FIELD'}


def readiness(bundle):
    if bundle['status'] in ('LOAD_ERROR', 'NOT_AN_NPC'):
        return bundle['status']
    if not bundle['placements']:
        return 'UNPLACED'
    if bundle['status'] == 'RESOLVED':
        return 'STATIC_COMPLETE'
    services = bundle['services']
    service_paths = {row['dialogue_path'] for kind in SERVICE_KINDS for row in services[kind]}
    for row in bundle['unresolved']:
        if row['reason'] not in DIALOGUE_ONLY:
            return 'SCRIPTED'
        if any(row['path'].startswith(path + '.') or row['path'] == path for path in service_paths):
            return 'SCRIPTED'
    return 'STATIC_SERVICES'


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--bundles', required=True)
    parser.add_argument('--index', required=True)
    parser.add_argument('--out', required=True)
    args = parser.parse_args()
    index = json.loads(Path(args.index).read_text(encoding='utf-8'))
    if index['include_text']:
        raise SystemExit('refusing to census --include-text output')
    rows, status, classes, reasons_rows, reasons_npcs, services = [], Counter(), Counter(), Counter(), Counter(), Counter()
    digest_lines = []
    for path in sorted(Path(args.bundles).glob('*.json')):
        data = path.read_bytes()
        digest_lines.append(f'{path.name}:{hashlib.sha256(data).hexdigest()}')
        bundle = json.loads(data)
        status[bundle['status']] += 1
        klass = readiness(bundle)
        classes[klass] += 1
        row = {'key': bundle['key'], 'status': bundle['status'], 'readiness': klass}
        if 'definition' in bundle:
            s = bundle['services']
            row['name'] = bundle['definition']['name']
            row['placements'] = len(bundle['placements'])
            row['services'] = {k: len(s[k]) for k in ('travel', 'spells', 'blessings', 'promotion', 'kick') if s[k]}
            if s['trade']:
                row['services']['trade_offers'] = len(s['trade']['offers'])
                services['trade_npcs'] += 1
                services['trade_offers'] += len(s['trade']['offers'])
            for kind in ('travel', 'spells', 'blessings', 'promotion', 'kick'):
                if s[kind]:
                    services[f'{kind}_npcs'] += 1
                    services[f'{kind}_rows'] += len(s[kind])
            services['placements'] += len(bundle['placements'])
            reasons = Counter(r['reason'] for r in bundle['unresolved'])
            reasons_rows.update(reasons)
            reasons_npcs.update(reasons.keys())
            if reasons:
                row['unresolved'] = dict(sorted(reasons.items()))
        else:
            row['error'] = bundle['error']
        rows.append(row)
    census = {
        'schema': SCHEMA, 'evidence': 'OTS_HYPOTHESIS_ONLY', 'source': index['source'],
        'bundle_digest': hashlib.sha256('\n'.join(digest_lines).encode()).hexdigest(),
        'totals': {'bundles': len(rows), 'placements_in_spawn_files': index['placements'],
                   'orphan_placement_names': index['orphan_placement_names'],
                   'status': dict(sorted(status.items())), 'readiness': dict(sorted(classes.items())),
                   'services': dict(sorted(services.items())),
                   'unresolved_rows_by_reason': dict(sorted(reasons_rows.items())),
                   'npcs_by_unresolved_reason': dict(sorted(reasons_npcs.items()))},
        'rows': rows,
    }
    Path(args.out).write_text(json.dumps(census, indent=1, sort_keys=True) + '\n', encoding='utf-8')
    print(json.dumps(census['totals'], indent=1))


if __name__ == '__main__':
    main()
