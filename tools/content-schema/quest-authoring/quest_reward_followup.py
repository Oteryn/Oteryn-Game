"""Finite chosen XP omissions from pinned donor scripts; original Source remains untouched."""
import copy
import hashlib
import json
import tarfile

PATH = 'tools/content-schema/quest-authoring/samples/recipe-followup/rewards.json'
SHA256 = '23feb1cc25e40587f7cc605265d82d2a10c39aea16cd4b4b3121a5902e9a443c'
CORPUS = 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz'
ALLOWED = {'oteryn:quest.sea_of_light_quest': [100, 400, 500, 1000],
           'oteryn:quest.the_new_frontier_quest': [8000]}


def digest(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def verify_source(packet, raw_sources):
    for evidence in packet['source_files']:
        p = evidence['provenance']
        sid = ':'.join(p[k] for k in ('source', 'revision', 'path'))
        raw = raw_sources[sid]
        if hashlib.sha256(raw).hexdigest() != p['sha256'] or len(raw) != p['byte_count']:
            raise ValueError('reward Source bytes differ')
        if hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest() != p['git_blob_sha1']:
            raise ValueError('reward Source Git blob differs')
        key = evidence['canonical_key']
        spans = evidence['grant_spans']
        if sorted(s['experience'] for s in spans) != sorted(ALLOWED[key]):
            raise ValueError('reward Source grant membership differs')
        for s in spans:
            fragment = raw[s['start_byte']:s['end_byte_exclusive']]
            if fragment.decode() != s['raw'] or hashlib.sha256(fragment).hexdigest() != s['span_sha256']:
                raise ValueError('reward Source span differs')
            if s['guard_expression'] not in s['raw'] or f'player:addExperience({s["experience"]}, true)' not in s['raw']:
                raise ValueError('reward Source grant guard differs')


def apply_packet(rows, packet, raw_sources):
    if packet['schema'] != 'OTERYN_CHOSEN_REWARD_FOLLOWUP/v1' or packet['runtime_enabled'] is not False or packet['source_holds_preserved'] is not True:
        raise ValueError('reward followup scope differs')
    if len(packet['changes']) != 2 or {c['canonical_key'] for c in packet['changes']} != set(ALLOWED):
        raise ValueError('reward followup finite changes differ')
    witnesses = {(s['canonical_key'], s['provenance']['source']) for s in packet['source_files']}
    if len(packet['source_files']) != 4 or witnesses != {(k, donor) for k in ALLOWED for donor in ['canary', 'crystalserver']}:
        raise ValueError('reward followup donor coverage differs')
    verify_source(packet, raw_sources)
    out = copy.deepcopy(rows)
    for fix in packet['changes']:
        selected = [r for r in out if r['definition']['identity']['key'] == fix['canonical_key']]
        if len(selected) != 1:
            raise ValueError('reward followup owner differs')
        recipe = selected[0]['definition']['oteryn_recipe']['payload']['recipe']
        if digest(recipe) != fix['baseline_recipe_sha256']:
            raise ValueError('reward followup recipe fence differs')
        reward = fix['reward_intent']
        if reward != {'basis': 'CHOSEN_OTERYN_APPROXIMATION', 'kind': 'experience', 'name': 'experience', 'count': sum(ALLOWED[fix['canonical_key']])}:
            raise ValueError('reward followup amount differs')
        if any(r['kind'] == 'experience' for r in recipe['reward_intents']):
            raise ValueError('reward followup experience already present')
        recipe['reward_intents'].append(copy.deepcopy(reward))
        recipe['source_notes'].append(fix['source_note'])
    return out


def load_sources(root, packet):
    archive = root / CORPUS
    if hashlib.sha256(archive.read_bytes()).hexdigest() != packet['corpus_archive_sha256']:
        raise ValueError('reward followup donor archive differs')
    sources = {}
    with tarfile.open(archive, 'r:gz') as tar:
        manifest = json.load(tar.extractfile('corpus-manifest.json'))
        by_id = {':'.join(r[k] for k in ('source', 'revision', 'path')): r for r in manifest['files']}
        for evidence in packet['source_files']:
            p = evidence['provenance']
            sid = ':'.join(p[k] for k in ('source', 'revision', 'path'))
            if any(p[k] != by_id[sid][k] for k in p):
                raise ValueError('reward followup manifest differs')
            sources[sid] = tar.extractfile('blobs/' + p['git_blob_sha1']).read()
    return sources


def apply(root, rows):
    raw = (root / PATH).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SHA256:
        raise ValueError('reward followup packet differs')
    packet = json.loads(raw)
    return apply_packet(rows, packet, load_sources(root, packet)), {'path': PATH, 'sha256': SHA256}
