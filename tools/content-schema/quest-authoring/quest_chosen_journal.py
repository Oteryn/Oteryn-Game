"""Five finite CHOSEN journal mappings; never rewrites donor Source or readiness."""
import tarfile
import argparse, copy, hashlib, json, sys
from pathlib import Path
EXPECTED_PACKET_SHA256 = '09d5ce5d37857455e33d09f58238e1dfeb6329c00cc04287edf464280f748231'
BASIS = 'CHOSEN_OTERYN_APPROXIMATION'
PACKET = 'tools/content-schema/quest-authoring/samples/chosen-journal/corrections.json'

def stable(v):
    return json.dumps(v, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()

def sha(v):
    return hashlib.sha256(v).hexdigest()

def digest(v):
    return sha(stable(v))

def read(p):
    return json.loads(p.read_text())

def walk(v):
    if isinstance(v, dict):
        yield v
        for x in v.values():
            yield from walk(x)
    elif isinstance(v, list):
        for x in v:
            yield from walk(x)

def definitions(root):
    authoring = root / 'tools/content-schema/quest-authoring'
    sys.path.insert(0, str(authoring))
    from quest_recipe_refinements import apply_refinements
    rows, _ = apply_refinements(root, read(authoring / 'samples/completion242/recipes.json'))
    recipes = {row['identity']['key']: row for row in rows}
    result = {}
    for quest in read(authoring / 'samples/questlog/quests.json')['quests']:
        key = 'oteryn:quest.' + quest['identity']['key'].split(':quest/', 1)[1].replace('/', '.')
        if key in recipes:
            result[key] = {'source_data': {'quest': quest}, 'oteryn_recipe': {'payload': recipes[key]}}
    return result

def verify_evidence(value, manifest_path):
    archive = tarfile.open(manifest_path) if manifest_path.name.endswith('.tar.gz') else None
    manifest = json.load(archive.extractfile('corpus-manifest.json')) if archive else read(manifest_path)
    files = {(r['source'], r['revision'], r['path']): r for r in manifest['files']}
    count = 0
    for w in walk(value):
        if not ('source' in w and 'revision' in w and ('path' in w) and any((k in w for k in ('byte_start', 'char_start')))):
            continue
        row = files[w['source'], w['revision'], w['path']]
        path = Path(row['cache_path'])
        raw = archive.extractfile(str(path)).read() if archive else (path if path.is_absolute() else manifest_path.parent / path).read_bytes()
        expected = w.get('file_sha256', w.get('raw_sha256', row['sha256']))
        if sha(raw) != expected or sha(raw) != row['sha256'] or w.get('git_blob_sha1', row['git_blob_sha1']) != row['git_blob_sha1'] or (hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\x00' + raw).hexdigest() != row['git_blob_sha1']):
            raise ValueError('Source evidence file fence')
        piece = raw[w['byte_start']:w.get('byte_end_exclusive', w.get('byte_end'))] if 'byte_start' in w else raw.decode()[w['char_start']:w['char_end']].encode()
        if sha(piece) != w.get('slice_sha256', w.get('span_sha256')):
            raise ValueError('Source evidence slice fence')
        for k in ('raw', 'source_excerpt'):
            if k in w and w[k].encode() != piece:
                raise ValueError('Source excerpt fence')
        count += 1
    if archive:
        archive.close()
    if not count:
        raise ValueError('No exact Source witnesses')
    return count

def mission(definition, key):
    values = [m for m in definition['source_data']['quest']['missions'] if m['key'] == key]
    if len(values) != 1:
        raise ValueError('Mission owner/key not unique')
    return values[0]

def build(root, evidence_root, manifest_path):
    defs = definitions(root)
    spike = read(evidence_root / 'spike-choice.json')
    fox = read(evidence_root / 'fox-correction.json')
    cobra = read(evidence_root / 'scarlett-completion.json')
    records = []
    checks = 0
    for filename, value in [('spike-choice.json', spike), ('fox-correction.json', fox), ('scarlett-completion.json', cobra)]:
        checks += verify_evidence(value, manifest_path)

    def record(key, owner, mission_key, choice, filename):
        original = mission(defs[owner], mission_key)
        return {'correction_key': key, 'quest_key': owner, 'mission_key': mission_key, 'original_mission': original, 'original_mission_sha256': digest(original), 'chosen_mapping': choice, 'evidence': {'path': filename, 'sha256': sha((evidence_root / filename).read_bytes())}, 'basis': BASIS, 'source_equivalence': False, 'source_holds_preserved': True, 'runtime_enabled': False, 'native_admission': False}
    for r in spike['records']:
        choice = copy.deepcopy(r['proposed_choice'])
        owner = r['canonical_mission_ref']['quest_key']
        m = r['canonical_mission_ref']['mission']
        key = m['key']
        if digest(m) != r['canonical_mission_ref']['mission_sha256'] or m != mission(defs[owner], key) or choice['goal'] != 4 or (len(set(choice['distinct_task_set'])) != 4) or (choice['repeat_handin_advances_chosen_stage'] is not False) or (choice['chosen_fame100_or_outfit_reward_included'] is not False) or (choice['source_storage_alias'] is not None):
            raise ValueError('Spike finite choice fence')
        region = choice['region']
        start, tasks = {'Upper': (1, [2, 3, 4, 5]), 'Middle': (6, [7, 8, 9, 10]), 'Lower': (11, [12, 13, 14, 15])}[region]
        choice['recipe_stage_keys'] = ['s' + str(n) for n in range(start, tasks[-1] + 1)]
        if region == 'Lower':
            choice['recipe_stage_keys'].append('s16')
        task_stages = {'Upper': {'Pacifier': 's2', 'Mound': 's3', 'Track': 's4', 'Kill': 's5'}, 'Middle': {'Nest': 's7', 'Mushroom': 's8', 'Charge': 's9', 'Kill': 's10'}, 'Lower': {'Parcel': 's12', 'Undercover': 's13', 'Lava': 's14', 'Kill': 's15'}}[region]
        choice['source_task_stage_keys'] = {task: task_stages[task.rsplit('_', 2)[1]] for task in choice['distinct_task_set']}
        if not set(choice['recipe_stage_keys']) <= set((s['key'] for s in defs[owner]['oteryn_recipe']['payload']['recipe']['stages'])):
            raise ValueError('Spike recipe stage fence')
        records.append(record(choice['stage_key'], owner, key, choice, 'spike-choice.json'))
    owner = fox['owner_key']
    key = fox['mission_key']
    if fox['baseline']['mission_source'] != mission(defs[owner], key) or fox['chosen']['reward_intents_add'] != [] or fox['preserved']['source_equivalence_claimed'] is not False:
        raise ValueError('Fox choice fence')
    choice = copy.deepcopy(fox['chosen'])
    renames = {'s_fox_accept': 's17', 's_fox_recover': 's18', 's_fox_return': 's19', 's17': 's20'}
    for stage in choice['stages']:
        stage['key'] = renames[stage['key']]
        stage['next'] = [renames.get(k, k) for k in stage['next']]
    choice['title_stage_keys'] = ['s17', 's18', 's19']
    records.append(record('fox_child_journal', owner, key, choice, 'fox-correction.json'))
    owner = cobra['owner_quest_key']
    key = cobra['mission_key']
    if cobra['native_ready'] is not False or cobra['source_complete'] is not False or cobra['chosen']['recipe_stage_key'] != 's5':
        raise ValueError('Cobra admission/stage fence')
    choice = copy.deepcopy(cobra['chosen'])
    choice.update(eligible_current_stage='s5', fresh_authenticated_death_required=True, prior_kill_history_completes_stage=False, source_firstkill_achievement_guard_separate=True)
    records.append(record('cobra_scarlett_credit', owner, key, choice, 'scarlett-completion.json'))
    stage_fence = digest(defs[fox['owner_key']]['oteryn_recipe']['payload']['recipe']['stages'])
    return {'schema': 'OTERYN_CHOSEN_JOURNAL_CORRECTIONS/v1', 'records': records, 'fox_original_stages_sha256': stage_fence, 'source_equivalence': False, 'source_holds_preserved': True, 'runtime_enabled': False, 'native_admission': False, 'summary': {'corrections': 5, 'quests': 3, 'exact_source_spans': checks, 'chosen_fox_stages_added': 3}}

def check_packet(packet):
    import jsonschema
    jsonschema.validate(packet, read(Path(__file__).with_name('quest_chosen_journal.schema.json')))
    if packet['schema'] != 'OTERYN_CHOSEN_JOURNAL_CORRECTIONS/v1' or len(packet['records']) != 5:
        raise ValueError('Closed chosen packet selection')
    ids = {(r['quest_key'], r['mission_key'], r['correction_key']) for r in packet['records']}
    if len(ids) != 5 or sum((r['quest_key'] == 'oteryn:quest.spike_task' for r in packet['records'])) != 3 or {r['quest_key'] for r in packet['records']} != {'oteryn:quest.spike_task', 'oteryn:quest.tibia_tales', 'oteryn:quest.grave_danger_quest'}:
        raise ValueError('Chosen owner selection fence')
    for r in [packet, *packet['records']]:
        if r['source_equivalence'] is not False or r['source_holds_preserved'] is not True or r['runtime_enabled'] is not False or (r['native_admission'] is not False):
            raise ValueError('Chosen cannot claim Source/Native admission')
    for r in packet['records']:
        if r['basis'] != BASIS or digest(r['original_mission']) != r['original_mission_sha256']:
            raise ValueError('Chosen mission digest fence')

def apply_records(records, packet, packet_sha256):
    check_packet(packet)
    result = copy.deepcopy(records)
    by_key = {r['definition']['identity']['key']: r['definition'] for r in result}
    for r in packet['records']:
        d = by_key[r['quest_key']]
        if mission(d, r['mission_key']) != r['original_mission']:
            raise ValueError('Chosen current owner/mission fence')
        if d.get('oteryn_recipe', {}).get('runtime_enabled') is not False:
            raise ValueError('Chosen recipe admission fence')
    for owner in sorted({r['quest_key'] for r in packet['records']}):
        d = by_key[owner]
        selected = [r for r in packet['records'] if r['quest_key'] == owner]
        recipe = d['oteryn_recipe']['payload']['recipe']
        if 'chosen_journal_corrections' in d['oteryn_recipe']:
            raise ValueError('Chosen overlay already applied')
        d['oteryn_recipe']['chosen_journal_corrections'] = {'basis': BASIS, 'packet_path': PACKET, 'packet_sha256': packet_sha256, 'correction_keys': [r['correction_key'] for r in selected], 'record_sha256s': [digest(r) for r in selected], 'source_equivalence': False, 'source_holds_preserved': True, 'runtime_enabled': False, 'native_admission': False}
        recipe['source_notes'].append('CHOSEN journal mappings: ' + ', '.join((r['correction_key'] for r in selected)) + '. ' + PACKET + ' SHA256=' + packet_sha256 + '. Donor Source holds, runtime non-admission and original rewards remain unchanged.')
        if owner == 'oteryn:quest.tibia_tales':
            chosen = selected[0]['chosen_mapping']
            stages = recipe['stages']
            if digest(stages) != packet['fox_original_stages_sha256']:
                raise ValueError('Fox chosen stage baseline fence')
            index = next((i for i, s in enumerate(stages) if s['key'] == chosen['splice_before']))
            if index < 1 or stages[index - 1]['key'] != chosen['splice_after'] or stages[index - 1]['next'] != [chosen['splice_before']] or any((s['key'] in {v['key'] for v in stages[:-1]} for s in chosen['stages'])):
                raise ValueError('Fox splice fence')
            stages[index]['key'] = 's20'
            stages[index - 1]['next'] = [chosen['stages'][0]['key']]
            stages[index:index] = copy.deepcopy(chosen['stages'])
            recipe['source_notes'].extend(chosen['source_notes'])
            titles = d['oteryn_recipe']['payload']['title_stage_keys']
            expected = ['s' + str(n) for n in range(1, 18)]
            if titles['Tibia Tales'] != expected:
                raise ValueError('Fox anthology title stage fence')
            for title, keys in titles.items():
                titles[title] = ['s20' if k == 's17' else k for k in keys]
            titles['Tibia Tales'] = [s['key'] for s in stages]
            title = chosen['covered_title']
            titles[title] = chosen['title_stage_keys']
            payload = d['oteryn_recipe']['payload']
            payload['covered_wiki_titles'] = sorted(set(payload['covered_wiki_titles'] + [title]))
            from quest_completion_authoring import validate_journey
            validate_journey(recipe)
    return result

def apply(root, records):
    path = Path(root) / PACKET
    if not path.exists():
        raise ValueError('Required chosen journal packet missing')
    raw = path.read_bytes()
    if sha(raw) != EXPECTED_PACKET_SHA256:
        raise ValueError('Unapproved chosen journal packet')
    packet = read(path)
    for entry in packet['records']:
        evidence = entry['evidence']
        if sha((path.parent / evidence['path']).read_bytes()) != evidence['sha256']:
            raise ValueError('Chosen evidence packet fence')
    result = apply_records(records, packet, sha(raw))
    return (result, {'path': PACKET, 'sha256': sha(raw), 'runtime_enabled': False, 'source_equivalence': False})

def main():
    p = argparse.ArgumentParser()
    for k in ('root', 'evidence-root', 'corpus-manifest', 'out'):
        p.add_argument('--' + k, type=Path, required=True)
    a = p.parse_args()
    packet = build(a.root, a.evidence_root, a.corpus_manifest)
    a.out.write_text(json.dumps(packet, ensure_ascii=False, indent=2) + '\n')
    print(packet['summary'])
if __name__ == '__main__':
    main()
