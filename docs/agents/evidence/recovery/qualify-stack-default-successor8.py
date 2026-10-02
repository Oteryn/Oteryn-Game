"""External qualification only; reuse immutable accepted default guards unchanged."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path('/workspace/pr1437-stack-default-completion')
OUT = Path('/workspace/audit-continuation')
PARENT = '62c27da6bdad8ee22e5f8e5ea73c6f957133f640'
IDS = frozenset({23229, 23230, 23231, 23232, 23295, 23299, 23335, 23339})
sys.path.insert(0, str(ROOT / 'tools/content-schema/item-authoring'))
import check_stack_default_historical_context as context
import appearance_membership as membership

base = context.base


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def file_pin(path):
    raw = (ROOT / path).read_bytes()
    return {'path': path, 'sha256': digest(raw), 'bytes': len(raw)}


def save(path, data):
    raw = (json.dumps(data, ensure_ascii=False, sort_keys=True,
                      separators=(',', ':')) + '\n').encode()
    (OUT / path).write_bytes(raw)
    return {'path': str(OUT / path), 'sha256': digest(raw), 'bytes': len(raw)}


assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT,
                               text=True).strip() == PARENT
source, objects, bound, wiki, pages, definitions, routed = context.current_inputs(ROOT)
old_packet = json.loads(base.OUTPUT.read_bytes())
old_ids = {r['source_item_id'] for r in old_packet['promotions']}
assert len(old_ids) == 1487 and not IDS.intersection(old_ids)
validation = context.current_validation(ROOT)
assert set(validation['current_eligible_outside_original_default_cohort']) == IDS
selected = {r['source_item_id']: r for r in source['records'] if r['source_item_id'] in IDS}
assert set(selected) == IDS

dat = base.checked(ROOT, base.CLIENT, base.CLIENT_SHA)
raw_objects = {}
for tag, raw in base.protobuf_fields(dat):
    if tag != 1:
        continue
    obj = base.decode_appearance_object(raw)
    if obj.get('id') in IDS:
        assert obj['id'] not in raw_objects
        raw_objects[obj['id']] = (obj, raw)
assert set(raw_objects) == IDS
_, admitted = membership.load_admitted(ROOT / 'imports/official/appearance-membership')
members = {label: {row[0]: row for row in table['entries']}
           for label, table in admitted.items()}

index_path = OUT / 'global-infobox-object-own-itemid-index.json'
index_raw = index_path.read_bytes()
index = json.loads(index_raw)
records, promotions = [], []
input_paths = [base.PROOF, base.COMPILER, context.CONTEXT,
               'tools/content-schema/item-authoring/check_stack_default_historical_context.py',
               base.DECODER, base.PARSER, base.CLIENT, base.BINDINGS, base.WIKI,
               source['bridge_proof']['path'], 'tools/content-schema/item-authoring/appearance_membership.py',
               'imports/official/appearance-membership/admitted.json']
admitted_index = json.loads((ROOT / 'imports/official/appearance-membership/admitted.json').read_bytes())
input_paths.extend('imports/official/appearance-membership/' + table['manifest']
                   for table in admitted_index['files'])
world_pins = [file_pin(str(p.relative_to(ROOT)))
              for family in ('objects', 'terrain')
              for p in sorted((ROOT / f'content/world/{family}').glob('*.json'))]

for iid in sorted(IDS):
    row = selected[iid]
    key = row['item_key']
    definition = definitions[key]
    binding = bound[key]
    obj, raw = raw_objects[iid]
    assert obj == objects[iid]
    assert digest(row['content'].encode()) == row['content_sha256']
    why = base.reasons(row, definition, binding, obj, routed, wiki[iid], pages,
                       source['qualification_cutoff'])
    assert not why, (iid, why)
    assert obj['id'] == iid and binding['target'] == definition['identity']
    assert definition['semantics']['stack'] == {'state': 'UNKNOWN'}
    assert key not in routed
    current_member = members['client-15.30'][iid]
    old_member = members['crystal-ff7ede5'][iid]
    projection = membership.sha256_hex(membership.canonical_bytes(
        membership.identity_projection(obj)))
    assert current_member[1] == old_member[1] == projection
    assert current_member[2] == digest(raw)
    refs = index['by_own_itemid_integer_mention'].get(str(iid), [])
    assert len(refs) == 1
    ref = refs[0]
    for name in ('page_id', 'revision_id', 'revision_timestamp', 'content_sha256'):
        assert ref[name] == row[name], (iid, name)
    assert ref['title'] == row['title']
    part_raw = Path(ref['capture_path']).read_bytes()
    assert digest(part_raw) == ref['capture_sha256']
    page = json.loads(part_raw)['pages'][ref['page_ordinal']]
    box = page['infobox_objects'][ref['box_index']]
    assert box['balanced'] and not box['inside_comment']
    assert box['positive_exact_infobox_object_match']
    fields = base.raw_parameters(row['content'])
    assert base.raw_parameters(box['raw']) == box['parameter_values'] == fields
    assert 'stackable' not in fields and box['raw'] in row['content']
    native_shards = []
    for shard in json.loads((ROOT / 'content/items/index.json').read_bytes())['shards']:
        content = (ROOT / shard).read_bytes()
        if any(r['definition']['identity']['key'] == key
               for r in json.loads(content)['records']):
            native_shards.append(file_pin(shard))
    assert len(native_shards) == 1
    record = {'source': copy.deepcopy(row), 'binding': binding,
              'current_native_definition': definition,
              'current_native_definition_sha256': digest(context.canonical(definition)),
              'current_native_source': native_shards[0],
              'official_object': obj, 'official_object_sha256': digest(raw),
              'current_member': current_member, 'crystal_member': old_member,
              'identity_projection_sha256': projection,
              'retained_observations': wiki[iid],
              'complete_selected_own_id_references': refs,
              'selected_own_raw_box': box,
              'selected_own_raw_box_sha256': digest(box['raw'].encode()),
              'guards': {'accepted_reasons': why, 'world_owner_absent': True,
                         'outside_original1487': True, 'native_stack_group': 'UNKNOWN'},
              'evidence': {'raw_stackable_argument': {'state': 'UNKNOWN', 'reason': 'ABSENT'},
                           'client_cumulative': {'state': 'UNKNOWN', 'reason': 'ABSENT'},
                           'derived_stackable': False,
                           'basis': 'DERIVED_DOCUMENTED_TEMPLATE_DEFAULT'},
              'current_public_continuity': 'NOT_NEWLY_FETCHED_CUTOFF_SOURCE_ONLY'}
    records.append(record)
    handle = {k: row[k] for k in ('page_id', 'revision_id', 'revision_timestamp',
              'title', 'content_sha256', 'capture_sha256', 'capture_url', 'capture_time')}
    promotions.append({'item_key': key, 'target': definition['identity'],
                       'source_item_id': iid, 'stackable': False, 'wiki': handle,
                       'binding': binding, 'appearance_id': iid,
                       'object_sha256': digest(raw)})

proof = {'schema': 'OTERYN_ITEM_STACK_DEFAULT_SUCCESSOR8_SOURCE_QUALIFICATION/v1',
         'status': 'SOURCE_QUALIFIED_NOT_NATIVE_APPLIED', 'actual_parent': PARENT,
         'closed_ids': sorted(IDS), 'qualification_cutoff': source['qualification_cutoff'],
         'documentation': source['documentation'], 'bridge': source['bridge'],
         'bridge_proof': source['bridge_proof'], 'records': records,
         'source_reuse': 'Exact immutable original1651 own raw facts; new independent closed8 qualification, not extension of historical1487.',
         'input_pins': [file_pin(p) for p in input_paths], 'world_owner_inputs': world_pins,
         'complete_own_id_index': {'path': str(index_path), 'sha256': digest(index_raw)},
         'current_original_context': validation,
         'scope': {'source_qualified_items': 8, 'native_applied_items': 0,
                   'old_defaults': 1487, 'old_historical': 7},
         'authority': 'Accepted documented own Infobox Object omitted stackable default; portable take/current full identity/name guards. No client-absence=false assumption, max/class/admission/runtime change.'}
proof_handle = save('stack-default-successor8-source-qualification.json', proof)
packet = {'schema': 'OTERYN_ITEM_STACK_FALSE_PROMOTION/v1',
          'status': 'PROPOSED_SOURCE_ONLY_NOT_APPLIED',
          'source_policy': 'DERIVED_DOCUMENTED_TEMPLATE_DEFAULT',
          'sources': {'source_proof': proof_handle, 'actual_qualification_parent': PARENT},
          'counts': {'promotions': 8, 'holds': 0}, 'promotions': promotions, 'holds': []}
packet_handle = save('stack-default-successor8-source-packet.json', packet)
print(json.dumps({'proof': proof_handle, 'packet': packet_handle,
                  'counts': packet['counts'], 'baseline': PARENT}, indent=2))
