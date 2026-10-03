"""Offline acceptance of reviewed transcription deltas; never Native admission.

The approval is a separately reviewed authored input, not a signature. Exact
file/graph hashes replay that approval; this does not itself re-execute donors.
"""
import copy
import hashlib
import json
from pathlib import Path

DIRECTORY = 'tools/content-schema/quest-authoring/samples/completion242/'
RECEIPT = DIRECTORY + 'source-fix-receipt.json'
APPROVAL = DIRECTORY + 'source-fix-approval.json'
COUNTERS = {'interactions_with_gaps', 'unresolved_items'}


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def digest(value):
    return hashlib.sha256(canonical(value).encode()).hexdigest()


def fields(value, required):
    if not isinstance(value, dict) or set(value) != set(required):
        raise ValueError('unexpected receipt/approval fields')


def read_json(path):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError('duplicate JSON field: ' + key)
            result[key] = value
        return result
    return json.loads(path.read_text(encoding='utf-8'), object_pairs_hook=pairs)


def verify_file(root, descriptor):
    fields(descriptor, ('path', 'sha256'))
    relative = Path(descriptor['path'])
    if relative.is_absolute() or '..' in relative.parts:
        raise ValueError('input path must stay in repository')
    path = (root / relative).resolve()
    if not path.is_relative_to(root.resolve()):
        raise ValueError('input path escapes repository')
    if hashlib.sha256(path.read_bytes()).hexdigest() != descriptor['sha256']:
        raise ValueError('input/proof/compiler SHA mismatch: ' + str(relative))


def verified_receipt(root):
    path = root / RECEIPT
    if not path.exists():
        return None, {}
    receipt = read_json(path)
    extra = ('owner_associations',) if 'owner_associations' in receipt else ()
    fields(receipt, ('schema', 'parent_frozen_head', 'approval', 'immutable_inputs',
                     'compiler_inputs', 'proof_inputs', 'changes', 'native_runtime_admission') + extra)
    if receipt['schema'] != 'OTERYN_SOURCE_TRANSCRIPTION_CHANGES/v1' or receipt['native_runtime_admission'] is not False:
        raise ValueError('receipt schema or Native admission differs')
    if receipt['approval']['path'] != APPROVAL:
        raise ValueError('wrong reviewed approval path')
    verify_file(root, receipt['approval'])
    approval = read_json(root / APPROVAL)
    fields(approval, ('schema', 'parent_frozen_head', 'immutable_inputs', 'compiler_inputs',
                      'proof_inputs', 'approved_graphs', 'approved_core_digests', 'native_runtime_admission') + extra)
    if approval['schema'] != 'OTERYN_REVIEWED_SOURCE_TRANSCRIPTION_APPROVAL/v1' or approval['native_runtime_admission'] is not False:
        raise ValueError('approval schema or Native admission differs')
    if extra:
        if receipt['owner_associations'] != approval['owner_associations']:
            raise ValueError('owner association approval substitution')
        if receipt['owner_associations']['path'] != 'tools/content-schema/quest-authoring/samples/owner-associations/approval.json':
            raise ValueError('wrong owner association approval path')
        verify_file(root, receipt['owner_associations'])
    head = receipt['parent_frozen_head']
    if len(head) != 40 or any(c not in '0123456789abcdef' for c in head) or head != approval['parent_frozen_head']:
        raise ValueError('parent frozen head mismatch')
    for category in ('immutable_inputs', 'compiler_inputs', 'proof_inputs'):
        if receipt[category] != approval[category] or not receipt[category]:
            raise ValueError('approved input/proof/compiler substitution')
        for descriptor in receipt[category]:
            verify_file(root, descriptor)
    expected = {DIRECTORY + name for name in ('baseline-core-digests.json', 'selection.json', 'recipes.json')}
    if {r['path'] for r in receipt['immutable_inputs']} != expected or len(receipt['immutable_inputs']) != 3:
        raise ValueError('immutable R9 witnesses missing or duplicated')
    for name in ('approved_graphs', 'approved_core_digests'):
        seen = set()
        for row in approval[name]:
            fields(row, ('key', 'from_digest', 'to_digest'))
            signature = (row['key'], row['from_digest'], row['to_digest'])
            if signature in seen or row['from_digest'] == row['to_digest']:
                raise ValueError('duplicate or unchanged reviewed snapshot')
            seen.add(signature)
        if not seen:
            raise ValueError('missing reviewed snapshots')
    return receipt, approval


def normalize_core(core):
    """Separate eligible graph snapshots/counters; every other field stays exact."""
    normalized = copy.deepcopy(core)
    source = normalized.get('source_data')
    if source is not None:
        for name in ('interactions', 'interaction_source_conflicts'):
            source.pop(name, None)
    report = normalized['reported_source_readiness']
    gaps = report.get('data_gaps', {}) if report is not None else {}
    counters = {k: gaps.get(k, 0) for k in COUNTERS}
    for count in counters.values():
        if type(count) is not int or count < 0:
            raise ValueError('invalid source gap counter')
    if report is not None:
        report['data_gaps'] = {k: v for k, v in gaps.items() if k not in COUNTERS}
        report.pop('features', None)
        report.pop('interactions', None)
    recorded = {}
    kept = []
    for issue in normalized['missing_data']:
        if issue.get('code') == 'reported_source_gap' and issue.get('field') in COUNTERS:
            fields(issue, ('code', 'field', 'count'))
            if issue['field'] in recorded:
                raise ValueError('duplicate reported source gap')
            recorded[issue['field']] = issue['count']
        else:
            kept.append(issue)
    if recorded != {k: v for k, v in counters.items() if v}:
        raise ValueError('reported source gaps do not match missing-data counters')
    normalized['missing_data'] = kept
    return normalized


def graph_changes(old, new):
    """Exact occurrence snapshots, including raw conflict graph variants."""
    from collections import defaultdict
    def grouped(core):
        result = defaultdict(list)
        def visit(node):
            if isinstance(node, dict):
                if {'identity', 'rules', 'anchors', 'source'} <= set(node):
                    result[node['identity']['key']].append(node)
                else:
                    for value in node.values():
                        visit(value)
            elif isinstance(node, list):
                for value in node:
                    visit(value)
        source = core.get('source_data', {})
        for name in ('interactions', 'interaction_source_conflicts'):
            visit(source.get(name, []))
        return result
    before, after = grouped(old), grouped(new)
    changes = []
    for key in sorted(set(before) | set(after)):
        left, right = list(before[key]), list(after[key])
        for graph in list(left):
            if graph in right:
                left.remove(graph)
                right.remove(graph)
        if len(left) > len(right):
            raise ValueError('source graph removal is not admitted')
        for index, graph in enumerate(right):
            previous = left[index] if index < len(left) else None
            if previous is not None:
                metadata = lambda g: {k: v for k, v in g.items() if k not in ('rules', 'anchors', 'unresolved')}
                if metadata(previous) != metadata(graph):
                    raise ValueError('graph source identity/witness substitution')
            changes.append((key, digest(previous) if previous is not None else None, digest(graph)))
    return changes


def validate_change(change, baseline, current, approval, root=None):
    fields(change, ('key', 'from_digest', 'to_digest', 'old_core', 'new_core'))
    key = change['key']
    if key not in baseline or change['from_digest'] != baseline[key]:
        raise ValueError('unlisted or stale original from_digest')
    old, new = change['old_core'], change['new_core']
    signature = (key, change['from_digest'], change['to_digest'])
    approved = {(r['key'], r['from_digest'], r['to_digest']) for r in approval['approved_core_digests']}
    if signature not in approved or signature[1] == signature[2]:
        raise ValueError('unreviewed full core change')
    if old['identity']['key'] != key or new['identity'] != old['identity']:
        raise ValueError('source owner substitution')
    if digest(old) != change['from_digest'] or digest(new) != change['to_digest'] or new != current:
        raise ValueError('old/new core or current to_digest mismatch')
    structural_old, structural_new = old, new
    if 'owner_associations' in approval:
        from source_owner_guard import CORES, reviewed_pair
        if key in CORES:
            if root is None:
                raise ValueError('owner association proof requires repository root')
            structural_old, structural_new = reviewed_pair(root, change, approval['owner_associations'])
    if key == 'oteryn:quest.marlin_trophy_quest':
        from source_npc_exchange_guard import reviewed_pair as npc_pair
        structural_old, structural_new = npc_pair(structural_old, structural_new)
    from source_kilmaresh_owner_guard import CORES as kilmaresh_cores, reviewed_pair as kilmaresh_pair
    if key in kilmaresh_cores:
        structural_old, structural_new = kilmaresh_pair(structural_old, structural_new)
    if normalize_core(structural_old) != normalize_core(structural_new):
        raise ValueError('unrelated core, source pins/owner/alias or Native hold changed')
    graphs = {(r['key'], r['from_digest'], r['to_digest']) for r in approval['approved_graphs']}
    if not set(graph_changes(structural_old, structural_new)) <= graphs:
        raise ValueError('unreviewed graph replacement or joined graph')


def effective_digests(root, baseline, current):
    """Overlay only exact reviewed changes; unlisted cores keep the original map."""
    receipt, approval = verified_receipt(root)
    expected = dict(baseline)
    if receipt is None:
        return expected, None
    original = read_json(root / (DIRECTORY + 'baseline-core-digests.json'))
    if original != baseline:
        raise ValueError('caller baseline differs from immutable R9 map')
    readiness_path = 'tools/content-schema/quest-authoring/samples/readiness/readiness.json'
    if readiness_path not in {r['path'] for r in receipt['proof_inputs']}:
        raise ValueError('current readiness proof missing')
    readiness = {r['quest']: r for r in read_json(root / readiness_path)['quests']}
    changes = {}
    for change in receipt['changes']:
        key = change['key']
        if key in changes or key not in current:
            raise ValueError('duplicate or missing changed core')
        validate_change(change, baseline, current[key], approval, root)
        source_key = current[key]['source_refs']['quest']['key']
        if current[key]['reported_source_readiness'] != readiness.get(source_key):
            raise ValueError('current readiness row does not match core')
        changes[key] = change
    actual = {key for key in baseline if key in current and digest(current[key]) != baseline[key]}
    if set(changes) != actual:
        raise ValueError('missing or unrelated effective core changes')
    for key, change in changes.items():
        expected[key] = change['to_digest']
    provenance = {'path': RECEIPT, 'sha256': hashlib.sha256((root / RECEIPT).read_bytes()).hexdigest()}
    return expected, provenance
