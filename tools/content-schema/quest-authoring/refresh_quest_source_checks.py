"""Refresh explicit unresolved-reference evidence after the source converters.

No track, quest or runtime meaning is invented here. A diagnostic keeps the
transcription and its source manifest entry unresolved until declared content
actually satisfies the reference. --check compares without writing.
"""
import argparse
import collections
import json
from pathlib import Path

from validate_quest_content import gate_source_checks, interaction_source_checks, rule_leaves


def refresh(gates, gate_manifest, quests, progress, interactions, interaction_manifest):
    gate_manifest = json.loads(json.dumps(gate_manifest))
    interaction_manifest = json.loads(json.dumps(interaction_manifest))
    prior_gate_checks = {c['record'] for c in gate_manifest.get('source_checks', [])}
    gate_manifest['source_checks'] = gate_source_checks(gates, quests, progress)
    interaction_manifest['source_checks'] = interaction_source_checks(interactions, progress)
    bad_gates = {c['record'] for c in gate_manifest['source_checks']}
    bad_reads = {c['record'] for c in interaction_manifest['source_checks']}
    tracks = {t['key'] for t in progress['progress']}
    unresolved, undeclared = {}, set()
    for inter in interactions['interactions']:
        children, conditions = rule_leaves(inter['rules'])
        writes = {c['progress'] for c in children if c.get('request') == 'set_progress'} - tracks
        undeclared |= writes
        unresolved[inter['identity']['key']] = (bool(inter['unresolved']) or bool(writes)
            or inter['identity']['key'] in bad_reads
            or any('unresolved' in c for c in conditions)
            or any(c.get('status') == 'blocked' for c in children))
    interaction_manifest['undeclared_progress_tracks'] = sorted(undeclared)
    for manifest, blocked in ((gate_manifest, bad_gates), (interaction_manifest, unresolved)):
        for entry in manifest['entries']:
            key = entry.get('destination')
            if key and entry['status'] in ('mapped', 'unresolved_semantics'):
                blocked_here = key in blocked if isinstance(blocked, set) else blocked[key]
                if blocked_here:
                    entry['status'] = 'unresolved_semantics'
                elif manifest is interaction_manifest or key in prior_gate_checks:
                    entry['status'] = 'mapped'
        manifest['counts']['by_status'] = dict(sorted(collections.Counter(
            entry['status'] for entry in manifest['entries']).items()))
    return gate_manifest, interaction_manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--samples', type=Path, default=Path(__file__).resolve().parent / 'samples')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    load = lambda rel: json.loads((args.samples / rel).read_text())
    names = ('doors/manifest.json', 'interactions/manifest.json')
    updated = refresh(load('doors/gates.json'), load(names[0]), load('questlog/quests.json'),
                      load('questlog/progress.json'), load('interactions/interactions.json'), load(names[1]))
    stale = []
    for name, doc in zip(names, updated):
        path = args.samples / name
        encoded = json.dumps(doc, indent=2, ensure_ascii=False) + '\n'
        if path.read_text() != encoded:
            stale.append(name)
            if not args.check:
                path.write_text(encoded)
    print(json.dumps({'valid': not stale if args.check else True, 'updated': stale}, indent=2))
    if args.check and stale:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
