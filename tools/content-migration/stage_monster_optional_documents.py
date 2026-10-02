#!/usr/bin/env python3
"""Admit existing native Document declarations without making them playable records."""
from __future__ import annotations
import copy
import json
import re
import sys
import tempfile
from pathlib import Path
import creature_admission_stage as admission

DOCUMENT_TYPES = {'Book', 'Letter', 'Note', 'Diary', 'Report', 'Scroll', 'Parchment', 'Tablet', 'Inscription', 'Notice', 'Other'}

class DocumentMapper(admission.Mapper):
    def key(self, family, key):
        if family != 'Document':
            return super().key(family, key)
        if key.startswith('canary:document/'):
            native = 'oteryn:document.creature.' + key[len('canary:document/'):].replace('/', '.')
        elif key.startswith('oteryn:document/') or key.startswith('oteryn:document.'):
            native = key
        else:
            raise admission.StageError('Document key is outside the supported authoring namespace')
        if admission.nonproduction(native) or not re.fullmatch(r'[a-z0-9:._/-]+', native):
            raise admission.StageError('Document key is not a production key')
        return native


def declaration(document, mapper):
    allowed = {'identity', 'document_type', 'title', 'author', 'language', 'content'}
    if set(document) - allowed or not {'identity', 'document_type', 'title', 'language', 'content'} <= set(document):
        raise admission.StageError('Document source shape is incomplete or contains unknown fields')
    if document['document_type'] not in DOCUMENT_TYPES:
        raise admission.StageError('Document type is outside the existing native enum')
    for field in ['title', 'language', *(['author'] if 'author' in document else [])]:
        if not isinstance(document[field], str) or not document[field].strip() or '\x00' in document[field]:
            raise admission.StageError('Document text field is invalid')
    if not re.fullmatch(r'[a-z]{2,3}(?:-[A-Za-z0-9]{2,8})*', document['language']):
        raise admission.StageError('Document language is invalid')
    content = document['content']
    if not isinstance(content, list) or not content or any(not isinstance(p, str) or not p.strip() or '\x00' in p for p in content):
        raise admission.StageError('Document content is empty, incomplete or invalid')
    identity = mapper.identity('Document', document['identity'])
    return {'kind': 'Document', 'identity': {'key': identity['key'], 'revision': identity['revision']},
            **{field: copy.deepcopy(document[field]) for field in allowed - {'identity'} if field in document}, 'fields': []}


class DocumentStage(admission.Stage):
    def stage_dependencies(self, dependencies, owner):
        stripped = dict(dependencies, documents=[])
        # Keep the existing rejection for unsupported nested Loot, Item fences and all effects.
        super().stage_dependencies(stripped, owner)
        for document in dependencies['documents']:
            doc = declaration(document, self.mapper)
            # This is a graph-only carrier: upstream closure derives produced identities from records.
            # finalize() always removes the carrier before native staged output is made visible.
            carrier = dict(doc, identity=dict(doc['identity'], family='Document'))
            self.add(self.records, 'Document', doc['identity']['key'], carrier, owner)


def finalize(packet):
    result = copy.deepcopy(packet)
    documents = [row for row in result['records'] if row['kind'] == 'Document']
    if not documents:
        return result
    result['records'] = [row for row in result['records'] if row['kind'] != 'Document']
    known = {(row['kind'], row['identity']['key'], row['identity']['revision']) for row in result['declarations']}
    for row in documents:
        doc = dict(row, identity={key: row['identity'][key] for key in ['key', 'revision']})
        identity = ('Document', doc['identity']['key'], doc['identity']['revision'])
        if identity in known:
            raise admission.StageError('Duplicate Document declaration identity')
        known.add(identity)
        result['declarations'].append(doc)
    result['declarations'].sort(key=lambda row: (row['kind'], row['identity']['key']))
    result['counts']['records'] = len(result['records'])
    result['counts']['documents'] = len(documents)
    result['source']['optional_document_admission'] = {
        'profile': 'EXISTING_PROJECT_V2_DOCUMENT_DECLARATION/v1',
        'reference_playable_documents': False, 'source_bindings_added': 0,
        'paragraphs_preserved_without_translation_or_rewrite': True}
    validate_document_closure(result)
    return result


def validate_document_closure(packet):
    docs = [row for row in packet['declarations'] if row['kind'] == 'Document']
    ids = {('Document', d['identity']['key'], d['identity']['revision']) for d in docs}
    if len(ids) != len(docs) or any(row['kind'] == 'Document' for row in packet['records']):
        raise admission.StageError('Document declaration/playable separation violated')
    for profile in packet['authoring_profiles']:
        if profile['data']['kind'] != 'Creature':
            continue
        ref = profile['data']['profile'].get('details', {}).get('encyclopedia_document')
        if ref is not None and tuple(ref.get(key) for key in ['family', 'key', 'revision']) not in ids:
            raise admission.StageError('Creature encyclopedia requires an exact declared Document reference')
    if packet['counts'].get('documents', 0) != len(docs):
        raise admission.StageError('Document declaration count disagrees')
    if packet['counts']['encounters'] != sum(row['kind'] == 'Encounter' for row in packet['declarations']):
        raise admission.StageError('Encounter count must exclude Documents')
    return {'documents': len(docs), 'playable_documents': 0}


def validate_full_population(config):
    """Reuse every old population fence while correcting its all-declarations Encounter counter."""
    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'monster-lab'))
    import validate_full_population as validator
    from lab import inventory, read_json
    report = inventory(config)
    inputs = config['read_only_inputs']
    packet, index = read_json(inputs['stage']), read_json(inputs['index'])
    doc_counts = validate_document_closure(packet)
    item_export = read_json(inputs['item_map'])
    if item_export.get('allocation_digest_sha256') != admission.ITEM_ALLOCATION_SHA256:
        raise admission.StageError('Protected Item allocation digest drift')
    item_map = {row['source_item_id']: row['native_key'] for row in item_export['records']}
    for path in admission.ITEM_REKEYS:
        evidence = json.loads(path.read_text())['source_identity']
        if item_map.get(evidence['source_item_id']) != evidence['current_native_key']:
            raise admission.StageError('Protected Item rekey drift')
        item_map[evidence['source_item_id']] = evidence['target_native_key']
    appearance_index, appearances = admission.load_admitted()
    current_ids = {entry[0] for entry in appearances[appearance_index['newest']]['entries']}
    admitted_items = admission.resolve_admitted_item_map(item_map,
        json.loads(admission.REFERENCE.read_text())['records'],
        json.loads(admission.ITEM_ALIASES.read_text())['entries'],
        json.loads(admission.ITEM_BINDINGS.read_text())['bindings'], current_ids)
    comparison = copy.deepcopy(packet)
    comparison['counts']['encounters'] = len(packet['declarations'])
    counts = validator.validate_stage(comparison, index, report['inputs']['index_sha256'], item_export, admitted_items.values())
    counts['encounters'] = packet['counts']['encounters']
    return {'schema': 'MONSTER_OPTIONAL_DOCUMENT_QUALIFICATION/v1', 'counts': dict(counts, **doc_counts),
            'inventory': report['counts'], 'inputs': report['inputs'], 'content_validation_passed': True,
            'all_indexed_actors_admitted': report['counts']['native_held'] == 0,
            'live_server_started': False, 'all_document_native_semantics_tested': False,
            'limitation': 'Python closure proof; independent native canonical validator must qualify Document text/reference semantics.'}


def main():
    if len(sys.argv) > 1 and sys.argv[1] == 'validate':
        import argparse
        parser = argparse.ArgumentParser()
        parser.add_argument('validate'); parser.add_argument('--config', type=Path, required=True)
        parser.add_argument('--output', type=Path, required=True)
        args = parser.parse_args()
        result = validate_full_population(json.loads(args.config.read_text()))
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result['counts']))
        return
    original_mapper, original_stage, original_argv = admission.Mapper, admission.Stage, sys.argv[:]
    try:
        destination = Path(sys.argv[sys.argv.index('--out') + 1])
    except (ValueError, IndexError):
        raise admission.StageError('Document successor requires an explicit --out')
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='document-stage-', dir=destination.parent) as temp:
        intermediate = Path(temp) / 'graph-only.json'
        try:
            admission.Mapper, admission.Stage = DocumentMapper, DocumentStage
            sys.argv[sys.argv.index('--out') + 1] = str(intermediate)
            admission.main()
            result = finalize(json.loads(intermediate.read_text()))
            destination.write_text(json.dumps(result, sort_keys=True, indent=2) + '\n')
            print(json.dumps({'documents': result['counts'].get('documents', 0), 'playable_documents': 0}))
        finally:
            admission.Mapper, admission.Stage, sys.argv = original_mapper, original_stage, original_argv

if __name__ == '__main__':
    main()
