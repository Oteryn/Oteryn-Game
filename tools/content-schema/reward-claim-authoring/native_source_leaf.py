"""Pinned single written rewards use the existing native claim and obligation owners.

This separate source format preserves historical claim formats and their holds.
It executes no Lua and derives only a literal, once-only written Item reward.
"""
import hashlib
import json
import re
import sys
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
INPUT = 'tools/content-schema/reward-claim-authoring/native-source-leaves.json'
TOOL = 'tools/content-schema/reward-claim-authoring/native_source_leaf.py'
STARTUP = 'content/world/quest-startup-bindings.json'
SHARD = 'content/interactions/reward_claims/reward-claim-source-leaves.json'
PROFILE = 'native_source_leaf_v1'
SCHEMA = 'OTERYN_NATIVE_SOURCE_LEAF/v1'
CORPUS = 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz'
sys.path.insert(0, str(ROOT / 'tools/content-schema/quest-authoring'))
import lua_tables


def require(ok, reason):
    if not ok:
        raise ValueError('native source leaf: ' + reason)


def inputs(root):
    path = root / INPUT
    if not path.exists():
        return []
    doc = json.loads(path.read_bytes())
    require(set(doc) == {'schema', 'leaves'} and type(doc['schema']) is int and doc['schema'] == 1,
            'input shape')
    rows = doc['leaves']
    require(isinstance(rows, list) and len(rows) <= 64, 'source count')
    seen = set()
    for row in rows:
        require(set(row) == {'claim', 'quest', 'source', 'track', 'transition', 'rule'}, 'record fields')
        for name, prefix in [('claim', 'oteryn:reward-claim.'), ('quest', 'oteryn:quest.'),
                             ('track', 'oteryn:quest-progress/'), ('transition', 'oteryn:quest-transition/')]:
            value = row[name]
            require(isinstance(value, str) and value.startswith(prefix) and len(value) <= 128
                    and re.fullmatch(r'oteryn:[A-Za-z0-9._:/-]+', value), 'invalid ' + name)
            require((name, value) not in seen, 'duplicate ' + name)
            if name != 'quest':
                seen.add((name, value))
        require(isinstance(row['rule'], str) and 0 < len(row['rule']) <= 1024, 'explicit source rule')
        src = row['source']
        require(set(src) == {'repository', 'revision', 'path', 'table', 'unique_id'}, 'source fields')
        require(src['repository'] in ('zimbadev/crystalserver', 'opentibiabr/canary')
                and re.fullmatch('[0-9a-f]{40}', src['revision'])
                and src['path'].startswith(('data-global/', 'data-otservbr-global/'))
                and src['path'].endswith('.lua') and '..' not in src['path']
                and re.fullmatch('[A-Za-z_][A-Za-z0-9_]*', src['table'])
                and type(src['unique_id']) is int and 1 <= src['unique_id'] <= 65535,
                'source identity')
    return rows


def build(root=ROOT):
    rows = inputs(root)
    if not rows:
        return [], []
    startup = json.loads((root / STARTUP).read_bytes())
    records, bindings = [], []
    with tarfile.open(root / CORPUS) as archive:
        manifest = json.load(archive.extractfile('corpus-manifest.json'))
        for row in rows:
            src = row['source']; uid = src['unique_id']
            server = {'zimbadev/crystalserver': 'crystalserver', 'opentibiabr/canary': 'canary'}[src['repository']]
            entry, = [r for r in manifest['files'] if r['source'] == server
                      and r['revision'] == src['revision'] and r['path'] == src['path']]
            raw = archive.extractfile(entry.get('blob_path', entry.get('cache_path'))).read()
            sha = hashlib.sha256(raw).hexdigest()
            require(sha == entry['sha256'], 'donor digest')
            table = lua_tables.assignments(raw.decode(), {src['table']})[src['table']]
            field, = [f for f in table['fields'] if f['key'] == uid]
            value = lua_tables.as_python(field['value'])
            require(set(value) <= {'items', 'storage', 'formerValue'}
                    and value.get('formerValue', -1) == -1 and len(value.get('items', [])) == 1,
                    'literal once written reward required')
            reward = value['items'][0]
            require(set(reward) <= {'itemId', 'count', 'text'} and reward.get('count', 1) == 1
                    and type(reward['itemId']) is int and 1 <= reward['itemId'] <= 65535
                    and isinstance(reward.get('text'), str), 'single written Item required')
            body = reward['text'].strip('\n')
            require(1 <= len(body) <= 4000 and len(body.encode()) <= 7996 and '\0' not in body,
                    'written text bounds')
            completion = value['storage'].get('expr')
            require(isinstance(completion, str) and completion.startswith('Storage.')
                    and re.fullmatch('[A-Za-z0-9_.]+', completion), 'literal source completion')
            physical = [b for b in startup['bindings'] if b['attribute'] == 'unique' and b['value'] == uid]
            require(0 < len(physical) <= 16 and startup['repository'] == src['repository']
                    and startup['revision'] == src['revision'], 'exact startup source binding required')
            item = {'family': 'Item', 'key': f"oteryn:item.tibia.i{reward['itemId']}", 'revision': 'definition-r1'}
            witness = dict(repository=src['repository'], revision=src['revision'], path=src['path'],
                table=src['table'], unique_id=uid, line=field['line'], sha256=sha,
                git_blob_sha1=hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest(),
                completion=completion)
            placements, recipes = [], []
            for obj in physical:
                x, y, z = obj['position']
                placements.append(dict(appearance_tibia_id=obj['item'],
                    source_binding=dict(project_position=dict(x=x,y=y,z=z),
                        legacy_unique_ids=[dict(server='crystalserver', unique_id=uid)]),
                    reward=dict(items=[dict(item=item, count=1)], written_text=dict(item=item,
                        text_ref=dict(length=len(body),placeholders=[],sha256=hashlib.sha256(body.encode()).hexdigest()))),
                    quest_transition=row['transition']))
                recipes.append(dict(unique_id=uid,line=field['line'],root=item,capacity=None,
                    contents=[],root_text=body,completion=dict(kind='storage',key=completion)))
            records.append({'definition': dict(identity=dict(key=row['claim'],revision='reward-claim-r1'),
                definition_profile=PROFILE, claim=dict(per='character',repeat=dict(kind='once')),
                quest=dict(family='Quest',key=row['quest'],revision='quest-r1'), readiness='ready',
                provenance=dict(generator=TOOL,rule=row['rule']), placements=placements,
                native_source=dict(schema=SCHEMA,**witness),
                native_contents=dict(schema=SCHEMA,revision=src['revision'],source_sha256=sha,placements=recipes))})
            bindings.append(dict(claim=row['claim'],quest=row['quest'],track=row['track'],
                transition=row['transition'],initial=-1,minimum=-1,maximum=1,value=1,
                source=dict(path=src['path'])))
    return records, bindings


def extend_outputs(root, outputs, compact):
    records, _ = build(root)
    if not records:
        return []
    index_path = 'content/interactions/reward_claims/index.json'
    index = json.loads(outputs[index_path])
    existing = [r for path in index['shards'] for r in json.loads(outputs[path])['records']]
    keys = [r['definition']['identity']['key'] for r in existing + records]
    require(len(keys) == len(set(keys)), 'claim identity collision')
    positions = [tuple(p['source_binding']['project_position'][k] for k in 'xyz')
                 for r in existing + records for p in r['definition']['placements']]
    require(len(positions) == len(set(positions)), 'physical placement collision')
    outputs[SHARD] = compact(dict(family='RewardClaim',schema=SCHEMA,records=records))
    index['shards'].append(SHARD); index['record_count'] += len(records)
    index['readiness']['ready'] += len(records)
    outputs[index_path] = compact(index)
    return records


def validate(records, root):
    expected, _ = build(root)
    return [] if records == expected else ['native source leaf records differ from their pinned source']
