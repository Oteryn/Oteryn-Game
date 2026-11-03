"""Author and check self-contained per-quest review evidence for batch 2.

This is a bounded evidence adapter, not a content or gameplay generator. Capture
accepts the frozen lane packets as explicit inputs. Check reads only committed
snapshots and, when supplied, the exact Crystal/Canary Git objects. It never needs
the original workspace, credentials, a network connection, or a Lua interpreter.
"""
import argparse
from collections import Counter
import copy
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess

SCHEMA = 'OTERYN_QUESTS_2_REVIEW_SNAPSHOT/v1'
ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUTPUT = ROOT / 'docs/agents/evidence/quests-2-20261102'
PINS = {
    'zimbadev/crystalserver': '201189f871d23597b82caf88fd611454a2ddefb8',
    'opentibiabr/canary': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
}
QUEST_ROWS = {
    'dawnport': 35, 'devil_helmet': 5, 'draconia': 7, 'extension_mota': 2,
    'grave_danger': 119, 'liquid_black': 21, 'mintwallin_quest': 2,
    'mysterious_ornate': 1, 'primal_ordeal_quest': 48, 'rookgaard_quests': 1,
    'rottin_wood_and_married_men': 24, 'soul_war': 308,
    'the_gravedigger_of_drefia': 158, 'the_new_frontier': 217,
    'the_outlaw_camp': 5, 'the_postman_missions_quest': 104,
    'the_queen_of_the_banshees': 51, 'threatened_dreams_quest': 116,
    'too_hot_to_handle_quest': 8, 'troll_sabotage': 7,
    'wrath_of_the_emperor': 151,
}
STATUS = {
    'LEFT_OUT': ('LEFT_OUT', 'REFERENCE_ONLY'),
    'NEW_NATIVE_NPC': ('IMPLEMENTED', 'NATIVE_NPC'),
    'NEW_EXISTING_CALLBACK': ('IMPLEMENTED', 'CALLBACK_DATA'),
    'EXISTING_QUALIFIED_DATA': ('EXISTING', 'QUALIFIED_DATA'),
    'AUTHORED_CLOSED_DRAGON_ADAPTER': ('IMPLEMENTED', 'NATIVE_NPC'),
    'AUTHORED_EXISTING_CALLBACK_CORE': ('IMPLEMENTED', 'CALLBACK_CORE'),
    'NATIVE_DIALOGUE_IMPLEMENTED': ('IMPLEMENTED', 'NATIVE_NPC'),
    'NATIVE_DIALOGUE_IMPLEMENTED_WITH_TIMING_LIMITATION': ('IMPLEMENTED', 'NATIVE_NPC'),
    'DATA_IMPLEMENTED_MAP_QUALIFICATION_HELD': ('IMPLEMENTED', 'CALLBACK_DATA'),
    'EXISTING_CALLBACK_PRESERVED_MAP_QUALIFICATION_HELD': ('EXISTING', 'CALLBACK_DATA'),
    'EXISTING_REWARD_CONSUMER_PRESERVED_WORLD_QUALIFICATION_HELD': ('EXISTING', 'REWARD_CONSUMER'),
}
REFERENCE = re.compile(r'^([^@]+)@([0-9a-f]{40}):([^:]+):([1-9][0-9]*)(?:-([1-9][0-9]*))?$')
DECISION = re.compile(r'^https://github\.com/Oteryn/Oteryn-Game/pull/2844#issuecomment-[0-9]+$')
HEX256 = re.compile(r'^[0-9a-f]{64}$')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(value):
    return hashlib.sha256(value).hexdigest()


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()


def read_document(path):
    raw = path.read_bytes()
    return json.loads(raw), {'path': path.name, 'sha256': digest(raw)}


def reference(value):
    if isinstance(value, str):
        match = REFERENCE.fullmatch(value)
        require(match is not None, f'Invalid donor reference: {value!r}')
        repository, revision, path, start, end = match.groups()
        start, end = int(start), int(end or start)
    else:
        require(isinstance(value, dict), 'Donor reference must be text or an object')
        repository, revision = value['repository'], value.get('commit', value.get('revision'))
        path, start = value['path'], value['line']
        end = value.get('end_line', start)
    require(repository in PINS and PINS[repository] == revision, 'Unpinned donor reference')
    pure = PurePosixPath(path)
    require(bool(path) and not pure.is_absolute() and '..' not in pure.parts
            and str(pure) == path and '\\' not in path, 'Unsafe donor source path')
    require(type(start) is int and type(end) is int and 1 <= start <= end, 'Invalid source range')
    return {'repository': repository, 'revision': revision, 'path': path,
            'start_line': start, 'end_line': end}


def row_references(row):
    values = row.get('donor', [row.get('source')])
    require(isinstance(values, list) and values and all(values), 'Missing donor source')
    refs = [reference(value) for value in values]
    if 'source_path' in row:
        require(len(refs) == 1 and refs[0]['path'] == row['source_path']
                and refs[0]['start_line'] == row['source_line']
                and refs[0]['end_line'] == row['source_end'], 'Source reference fields disagree')
    return refs


class Sources:
    def __init__(self, crystal=None, canary=None):
        self.roots = {'zimbadev/crystalserver': crystal, 'opentibiabr/canary': canary}
        self.cache = {}

    def read(self, ref):
        key = (ref['repository'], ref['revision'], ref['path'])
        if key not in self.cache:
            root = self.roots[key[0]]
            require(root is not None, f'Missing explicit checkout argument for {key[0]}')
            self.cache[key] = subprocess.check_output(
                ['git', '-C', str(root), 'show', f'{key[1]}:{key[2]}'])
        return self.cache[key]

    def witness(self, ref):
        return witness(ref, self.read(ref))


def witness(ref, raw):
    lines = raw.splitlines(keepends=True)
    require(ref['end_line'] <= len(lines), f'Source range exceeds blob: {ref}')
    excerpt = b''.join(lines[ref['start_line'] - 1:ref['end_line']])
    return {**ref, 'blob_sha256': digest(raw), 'blob_line_count': len(lines),
            'excerpt_sha256': digest(excerpt),
            'url': f"https://github.com/{ref['repository']}/blob/{ref['revision']}/{ref['path']}#L{ref['start_line']}-L{ref['end_line']}"}


def check_witness(ref, proof, sources):
    require(all(proof.get(key) == value for key, value in ref.items()), 'Donor witness identity differs')
    require(HEX256.fullmatch(proof.get('blob_sha256', '')) and HEX256.fullmatch(proof.get('excerpt_sha256', '')), 'Invalid donor digest')
    require(type(proof.get('blob_line_count')) is int and proof['blob_line_count'] >= ref['end_line'], 'Invalid donor line count')
    expected_url = f"https://github.com/{ref['repository']}/blob/{ref['revision']}/{ref['path']}#L{ref['start_line']}-L{ref['end_line']}"
    require(proof.get('url') == expected_url, 'Donor citation URL differs')
    if sources is not None:
        require(proof == sources.witness(ref), 'Pinned donor bytes or excerpt differ')


def normalize(lane, disposition):
    field = {'classic': 'status', 'soul': 'status', 'emperor': 'delivery', 'root': 'disposition'}[lane]
    source_status = disposition[field]
    require(source_status in STATUS, f'Unknown disposition: {source_status}')
    status, delivery = STATUS[source_status]
    if lane == 'root' and 'completion' in disposition:
        completion = disposition['completion']
        if completion == 'PARTIAL':
            require(source_status == 'AUTHORED_EXISTING_CALLBACK_CORE',
                    'PARTIAL requires an explicitly authored callback core')
            status = 'PARTIAL'
        else:
            require(completion == status, 'Root completion contradicts source disposition')
    reason = disposition.get('reason', disposition.get('limitation'))
    require(isinstance(reason, str) and bool(reason.strip()), 'Missing disposition explanation')
    links = disposition.get('question_or_answer', disposition.get('questions'))
    if links is None:
        value = disposition.get('answer', disposition.get('decision'))
        links = [value] if value else []
    require(isinstance(links, list) and all(isinstance(link, str) and DECISION.fullmatch(link)
                                          for link in links), 'Invalid decision link')
    require(len(links) == len(set(links)), 'Duplicate decision link')
    require(status != 'LEFT_OUT' or bool(links), 'Left out requires a question or answer')
    return {'status': status, 'delivery_kind': delivery, 'source_status': source_status,
            'left_out': status == 'LEFT_OUT', 'reason': reason,
            'question_or_answer_links': links,
            'validation_claim': disposition.get('validation'),
            'runtime_availability': 'NOT_CERTIFIED_BY_THIS_REVIEW_SNAPSHOT'}


def validate_pair(lane, original, disposition):
    require(original['quest'] == disposition['quest'], 'Quest differs between inputs')
    step = original.get('step', original.get('element'))
    require(isinstance(step, str) and step, 'Missing source element')
    disposition_step = disposition.get('step', disposition.get('element'))
    if lane == 'root' and disposition_step is None:
        disposition_step = disposition.get('inventory', {}).get('step')
    require(step == disposition_step, 'Element differs between inputs')
    if lane == 'classic':
        require(original['donor'] == disposition['sources'], 'Classic source list differs')
        require(original['import_method'] == disposition['original_import'], 'Classic import kind differs')
    elif lane == 'soul':
        for field in ('patterns', 'donor', 'import_kind'):
            require(original[field] == disposition[field], f'Soul field differs: {field}')
        require(original['oteryn'] == disposition['existing_before'], 'Soul existing evidence differs')
    elif lane == 'emperor':
        require(all(disposition.get(key) == value for key, value in original.items()),
                'Emperor disposition changed the original source row')
    else:
        require(disposition.get('inventory') == original, 'Root disposition changed original inventory')
    return step


def row_key(row):
    return (row['quest'], row.get('step', row.get('element')), canonical(row_references(row)))


def join_rows(lane, inventory, dispositions):
    def key(row):
        if lane == 'soul':
            return row['quest'], row['element']
        if lane == 'emperor':
            return (row['quest'], row['source_path'], row['source_line'],
                    row['source_end'], row['step'])
        return row['quest'], row['step']
    source, result = {}, {}
    for row in inventory:
        require(key(row) not in source, 'Duplicate inventory element')
        source[key(row)] = row
    for row in dispositions:
        require(key(row) not in result, 'Duplicate disposition element')
        result[key(row)] = row
    require(source.keys() == result.keys(), 'Inventory/disposition coverage differs')
    return [(row, result[key(row)]) for row in inventory]


def make_rows(lane, pairs, sources):
    result = []
    seen = set()
    for index, (original, disposition) in enumerate(pairs, 1):
        step = validate_pair(lane, original, disposition)
        identity = row_key(original)
        require(identity not in seen, 'Duplicate source row')
        seen.add(identity)
        refs = row_references(original)
        result.append({'ordinal': index, 'element': step,
                       'source_row': copy.deepcopy(original),
                       'source_disposition': copy.deepcopy(disposition),
                       'normalized': normalize(lane, disposition),
                       'donor_witnesses': [sources.witness(ref) for ref in refs]})
    return result


def summary(rows):
    return {'inventory_rows': len(rows),
            'new_pattern_rows_without_assigned_id': sum(not row['source_row']['patterns'] for row in rows),
            'import_kinds': dict(sorted(Counter(row['source_row'].get('import_kind', row['source_row'].get('import_method')) for row in rows).items())),
            'normalized_status': dict(sorted(Counter(row['normalized']['status'] for row in rows).items())),
            'source_status': dict(sorted(Counter(row['normalized']['source_status'] for row in rows).items())),
            'complete_playable_quest_claimed': False}


def make_document(quest, lane, pairs, sources, inputs, source_context):
    rows = make_rows(lane, pairs, sources)
    doc = {'schema': SCHEMA, 'classification': 'DERIVED', 'quest': quest,
           'lane': lane, 'batch': 'quests-2-20261102',
           'scope': 'Frozen source rows and author dispositions preserved verbatim as JSON values. IMPLEMENTED means the named code/data subset only; all limitations and validation claims remain in the source disposition.',
           'input_documents': inputs, 'source_context': source_context,
           'source_rows_sha256': digest(canonical([r['source_row'] for r in rows])),
           'source_dispositions_sha256': digest(canonical([r['source_disposition'] for r in rows])),
           'summary': summary(rows), 'rows': rows}
    if 'source_comparison' in source_context:
        doc['source_comparison_witnesses'] = [
            sources.witness(reference(source_context['source_comparison'][name]))
            for name in ('crystal', 'canary')]
    check_document(doc, sources)
    return doc


def check_document(doc, sources=None):
    require(doc.get('schema') == SCHEMA and doc.get('classification') == 'DERIVED', 'Snapshot schema/classification')
    quest, lane = doc['quest'], doc['lane']
    require(quest in QUEST_ROWS, 'Quest is not in batch 2')
    require(lane in ('classic', 'emperor', 'soul', 'root'), 'Unknown evidence adapter')
    rows = doc['rows']
    require(isinstance(rows, list) and rows, 'Empty snapshot')
    expected = QUEST_ROWS[quest]
    require(expected is None or len(rows) == expected, f'Frozen count differs for {quest}')
    require(doc['summary'] == summary(rows), 'Snapshot counts/statuses differ')
    if lane == 'root':
        context = doc.get('source_context', {})
        for field, expected_value in (
            ('inventory_rows', len(rows)),
            ('auto_rows', doc['summary']['import_kinds'].get('AUTO', 0)),
            ('manual_rows', doc['summary']['import_kinds'].get('RĘCZNIE', 0)),
            ('left_out_rows', doc['summary']['normalized_status'].get('LEFT_OUT', 0)),
            ('summary', doc['summary']['normalized_status']),
        ):
            require(field not in context or context[field] == expected_value,
                    f'Root declared {field} differs from embedded source rows')
    require(doc['source_rows_sha256'] == digest(canonical([r['source_row'] for r in rows])), 'Original source row digest differs')
    require(doc['source_dispositions_sha256'] == digest(canonical([r['source_disposition'] for r in rows])), 'Original disposition digest differs')
    seen = set()
    for index, row in enumerate(rows, 1):
        original, disposition = row['source_row'], row['source_disposition']
        require(row['ordinal'] == index and original['quest'] == quest, 'Wrong quest/order')
        require(row['element'] == validate_pair(lane, original, disposition), 'Snapshot element differs')
        require(row['normalized'] == normalize(lane, disposition), 'Normalized disposition drift')
        require(original.get('import_kind', original.get('import_method')) in ('AUTO', 'RĘCZNIE'), 'Unknown import kind')
        patterns = original.get('patterns', [])
        require(isinstance(patterns, list) and
                all(re.fullmatch(r'P(?:0[1-9]|[12][0-9]|30)', pattern) for pattern in patterns),
                'Pattern outside P01-P30')
        require(patterns or 'NOWE WZORCE' in original.get('notes', ''),
                'Empty pattern list without explicit NOWE WZORCE evidence')
        key = row_key(original)
        require(key not in seen, 'Duplicate source row')
        seen.add(key)
        refs = row_references(original)
        require(len(refs) == len(row['donor_witnesses']), 'Donor witness coverage differs')
        for ref, proof in zip(refs, row['donor_witnesses']):
            check_witness(ref, proof, sources)
    for item in doc.get('input_documents', []):
        require(HEX256.fullmatch(item.get('sha256', '')), 'Invalid input document digest')
    comparison = doc.get('source_context', {}).get('source_comparison')
    if comparison is not None:
        proofs = doc.get('source_comparison_witnesses', [])
        require(len(proofs) == 2, 'Missing donor comparison witnesses')
        for name, proof in zip(('crystal', 'canary'), proofs):
            ref = reference(comparison[name])
            check_witness(ref, proof, sources)
            require(proof['blob_sha256'] == comparison['sha256'], 'Comparison digest differs')
        require(comparison['byte_identical'] is True and
                proofs[0]['blob_sha256'] == proofs[1]['blob_sha256'], 'Donor comparison is not byte identical')
    return doc['summary']


def capture_lanes(lanes, output, sources):
    documents = {}
    classic, classic_input = read_document(lanes / 'classic_lane/inventory_donor.json')
    classic_final, classic_final_input = read_document(lanes / 'classic_lane/implementation-disposition-classic.json')
    emperor, emperor_input = read_document(lanes / 'emperor_lane/inventory.json')
    emperor_final, emperor_final_input = read_document(lanes / 'emperor_lane/final-status.json')
    soul_final, soul_final_input = read_document(lanes / 'soul_war_lane/completion-ledger.json')
    soul, soul_inputs = [], []
    for quest in ('soul_war', 'grave_danger', 'primal_ordeal_quest', 'liquid_black'):
        rows, descriptor = read_document(lanes / 'soul_war_lane' / f'{quest}.json')
        soul.extend(rows)
        soul_inputs.append(descriptor)
    packets = [
        ('classic', classic['rows'], classic_final['rows'], [classic_input, classic_final_input],
         {'inventory': {k: v for k, v in classic.items() if k != 'rows'},
          'disposition': {k: v for k, v in classic_final.items() if k != 'rows'}}),
        ('emperor', emperor['rows'], emperor_final['rows'], [emperor_input, emperor_final_input],
         {'inventory': {k: v for k, v in emperor.items() if k != 'rows'},
          'disposition': {k: v for k, v in emperor_final.items() if k != 'rows'}}),
        ('soul', soul, soul_final, [*soul_inputs, soul_final_input], {}),
    ]
    for lane, inventory, dispositions, inputs, context in packets:
        pairs = join_rows(lane, inventory, dispositions)
        for quest in sorted({row['quest'] for row in inventory}):
            require(quest not in documents, 'Quest owned by multiple lanes')
            own = [(row, disposition) for row, disposition in pairs if row['quest'] == quest]
            documents[quest] = make_document(quest, lane, own, sources, inputs, context)
    # Root supplies its own independently authored packets. Existing normalized
    # root packets can be checked again without any external scratch documents.
    for quest in ('too_hot_to_handle_quest', 'threatened_dreams_quest'):
        path = output / f'{quest}.json'
        if not path.exists():
            continue
        packet, descriptor = read_document(path)
        if packet.get('schema') == SCHEMA:
            check_document(packet, sources)
            documents[quest] = packet
        else:
            require(packet.get('quest') == quest, 'Root packet quest differs')
            pairs = [(row['inventory'], row) for row in packet['rows']]
            documents[quest] = make_document(quest, 'root', pairs, sources, [descriptor],
                                             {key: value for key, value in packet.items() if key != 'rows'})
    # Validate every file before the first write; a malformed input cannot leave
    # a partially refreshed collection of review packets.
    for document in documents.values():
        check_document(document, sources)
    output.mkdir(parents=True, exist_ok=True)
    for quest, document in sorted(documents.items()):
        (output / f'{quest}.json').write_text(json.dumps(document, ensure_ascii=False, indent=2) + '\n')
    return documents


def capture_root(packet_path, inventory_path, output, sources):
    packet, packet_input = read_document(packet_path)
    inventory, inventory_input = read_document(inventory_path)
    quest = packet['quest']
    require(quest in ('threatened_dreams_quest', 'too_hot_to_handle_quest'), 'Not a root-owned quest')
    original_rows = inventory['rows']
    pairs = [(row['inventory'], row) for row in packet['rows']]
    require(len(original_rows) == len(pairs), 'Root final inventory coverage differs')
    for index, (original, pair) in enumerate(zip(original_rows, pairs), 1):
        require(original == pair[0], 'Root final inventory changed a source row')
        require(pair[1].get('row', index) == index, 'Root disposition row order differs')
    context = {key: value for key, value in packet.items() if key != 'rows'}
    document = make_document(quest, 'root', pairs, sources, [packet_input, inventory_input], context)
    output.mkdir(parents=True, exist_ok=True)
    (output / f'{quest}.json').write_text(json.dumps(document, ensure_ascii=False, indent=2) + '\n')


def check_collection(output, expected_quests=21, sources=None):
    require(1 <= expected_quests <= len(QUEST_ROWS), 'Invalid expected quest count')
    documents = {}
    for path in sorted(output.glob('*.json')):
        require(path.stem in QUEST_ROWS, f'Unexpected JSON evidence file: {path.name}')
        doc = json.loads(path.read_text())
        require(doc['quest'] == path.stem, 'Quest filename differs')
        check_document(doc, sources)
        documents[path.stem] = doc
    require(len(documents) == expected_quests, f'Expected {expected_quests} quest files, found {len(documents)}')
    totals = Counter()
    for document in documents.values():
        totals.update(document['summary']['normalized_status'])
    return {'schema': SCHEMA, 'quest_count': len(documents), 'batch_quest_count': 21,
            'missing_quests': sorted(set(QUEST_ROWS) - documents.keys()),
            'inventory_rows': sum(d['summary']['inventory_rows'] for d in documents.values()),
            'new_pattern_rows_without_assigned_id': sum(d['summary']['new_pattern_rows_without_assigned_id'] for d in documents.values()),
            'normalized_status': dict(sorted(totals.items())),
            'pinned_donor_bytes_verified': sources is not None,
            'runtime_completion_certified': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=('capture', 'capture-root', 'check'))
    parser.add_argument('--lane-root', type=Path, help='Capture only: explicit frozen lane packet directory')
    parser.add_argument('--root-packet', type=Path, help='Capture-root only: the final root disposition packet')
    parser.add_argument('--root-inventory', type=Path, help='Capture-root only: the exact final root inventory')
    parser.add_argument('--crystal', type=Path, help='Optional for check; mandatory for capture')
    parser.add_argument('--canary', type=Path)
    parser.add_argument('--output', type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument('--expected-quests', type=int, default=21,
                        help='Default 21 refuses incomplete batch coverage; interim count must be explicit')
    args = parser.parse_args()
    sources = Sources(args.crystal, args.canary) if args.crystal or args.canary else None
    if args.operation == 'capture':
        require(args.lane_root is not None and args.crystal is not None,
                'Capture needs explicit --lane-root and --crystal')
        capture_lanes(args.lane_root, args.output, sources)
    elif args.operation == 'capture-root':
        require(args.root_packet is not None and args.root_inventory is not None and args.crystal is not None,
                'Capture-root needs --root-packet, --root-inventory and --crystal')
        capture_root(args.root_packet, args.root_inventory, args.output, sources)
    print(json.dumps(check_collection(args.output, args.expected_quests, sources), ensure_ascii=False, indent=2))


if __name__ == '__main__':
    try:
        main()
    except (KeyError, TypeError, ValueError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f'QUESTS_2_REVIEW_SNAPSHOT: {error}') from error
