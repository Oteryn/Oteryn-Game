"""Offline Quest declarations/profile overlay for existing v2 Source authoring seam."""
import argparse, collections, hashlib, json, pathlib, re
PREFIX = 'oteryn:source.quest.'

def encode(x):
    return json.dumps(x, ensure_ascii=False, sort_keys=True, separators=(',', ':'))

def sha(b):
    return hashlib.sha256(b).hexdigest()

def field(k, v):
    return {'field_path': PREFIX + k, 'value': {'type': 'Text', 'value': v if isinstance(v, str) else encode(v)}}

def chosen(d):
    if d.get('definition_profile') == 'oteryn_authored_v1':
        return (d.get('recipe'), '/recipe')
    if d.get('oteryn_recipe', {}).get('profile') == 'chosen_source_completion_v1':
        return (d['oteryn_recipe']['payload']['recipe'], '/oteryn_recipe/payload/recipe')
    return (None, None)

def project(d, path, pointer, packet_sha):
    ident = d['identity']
    ref = {'path': path, 'packet_sha256': packet_sha, 'json_pointer': pointer, 'definition_sha256': sha(encode(d).encode())}
    fields = [field('canonical_ref', ref), field('classification', d['classification']), field('display_name', d['display_name']), field('readiness', d['readiness'])]
    declaration = {'kind': 'Quest', 'identity': dict(ident), 'fields': sorted(fields, key=lambda f: f['field_path'])}
    recipe, rp = chosen(d)
    profile = None
    omissions = []
    if recipe is not None:
        req = recipe.get('requirements', {})
        value = {}
        proof = []
        level = req.get('min_level')
        if type(level) is int and 0 <= level <= 65535:
            value['required_level'] = level
            proof.append({'target': 'required_level', 'source_pointer': pointer + rp + '/requirements/min_level', 'value': level})
        else:
            omissions.append({'field': 'required_level', 'reason': 'UNKNOWN_OR_NOT_UINT16', 'source_value': level})
        premium = req.get('premium')
        if type(premium) is bool:
            value['premium'] = premium
            proof.append({'target': 'premium', 'source_pointer': pointer + rp + '/requirements/premium', 'value': premium})
        else:
            omissions.append({'field': 'premium', 'reason': 'UNKNOWN_OR_NOT_BOOLEAN', 'source_value': premium})
        repeat = recipe.get('repeat', {}).get('kind')
        if repeat in ['once', 'daily']:
            value['repeatable'] = repeat != 'once'
            proof.append({'target': 'repeatable', 'source_pointer': pointer + rp + '/repeat/kind', 'source_value': repeat, 'value': repeat != 'once', 'loss': 'CYCLE_DETAILS_RETAINED_AS_SOURCE_TEXT_ONLY'})
        else:
            omissions.append({'field': 'repeatable', 'reason': 'UNKNOWN_REPEAT_KIND', 'source_value': repeat})
        sf = [field('projection_proof', proof), field('recipe_ref', ref | {'json_pointer': pointer + rp, 'definition_json_pointer': pointer, 'recipe_sha256': sha(encode(recipe).encode())}), field('interpretation', 'CHOSEN_RECIPE_SOURCE_AUTHORING_ONLY_EXECUTION_UNPROVEN')]
        for k in ['stages', 'repeat', 'reward_intents']:
            if k in recipe:
                sf.append(field('chosen_' + k, recipe[k]))
        if req.get('prerequisites'):
            sf.append(field('chosen_prerequisite_intents', req['prerequisites']))
        value['fields'] = sorted(sf, key=lambda f: f['field_path'])
        profile = {'target': {'family': 'Quest', **ident}, 'data': {'kind': 'Quest', 'profile': value}}
        for k in ['prerequisites', 'reward_items', 'reward_achievements', 'encounters']:
            omissions.append({'field': k, 'reason': 'NO_TYPED_IDENTITY_JOIN_IN_THIS_EXPORT', 'intent_retained_as_source_metadata': True})
    else:
        omissions.append({'field': 'authoring_profile', 'reason': 'NO_CHOSEN_RECIPE_SOURCE_CATALOGUE_ONLY'})
    return (declaration, profile, {'quest': ident, 'canonical_ref': ref, 'profile_present': profile is not None, 'omissions': omissions})

def production_key(key):
    segments = re.split(r"[:./_-]", key.lower())
    invalid_marker = "test" in segments or any(
        marker in key.lower()
        for marker in ["fixture", "synthetic", "evidence", "test-only"]
    )
    return bool(re.fullmatch(r"oteryn:[a-z0-9_.-]+", key)) and not invalid_marker

def shape_check(doc):
    if set(doc) != {'schema', 'records', 'authoring_profiles'} or doc['schema'] != 'OTERYN_WORLD_PROJECT_DECLARATIONS/v2':
        raise ValueError('DeclarationsDocument shape')
    identities = set()

    def identity(i):
        if set(i) != {'key', 'revision'} or not production_key(i['key']) or (not re.fullmatch('[a-z0-9_.-]+', i['revision'])):
            raise ValueError('Identity shape')

    def fields(fs):
        keys = [f['field_path'] for f in fs]
        if keys != sorted(set(keys)):
            raise ValueError('Candidate fields sorted unique')
        for f in fs:
            if set(f) != {'field_path', 'value'} or not production_key(f['field_path']):
                raise ValueError('Candidate field shape')
            v = f['value']
            if set(v) != {'type', 'value'} or v['type'] != 'Text' or type(v['value']) is not str:
                raise ValueError('Source metadata Text shape')
    for d in doc['records']:
        if set(d) != {'kind', 'identity', 'fields'} or d['kind'] != 'Quest':
            raise ValueError('Quest declaration shape')
        identity(d['identity'])
        fields(d['fields'])
        key = (d['identity']['key'], d['identity']['revision'])
        if key in identities:
            raise ValueError('Duplicate identity')
        identities.add(key)
    targets = [(p['target']['family'], p['target']['key'], p['target']['revision']) for p in doc['authoring_profiles']]
    if targets != sorted(set(targets)):
        raise ValueError('Authoring profiles target sorted unique')
    for p in doc['authoring_profiles']:
        if set(p) != {'target', 'data'} or set(p['target']) != {'family', 'key', 'revision'} or p['target']['family'] != 'Quest':
            raise ValueError('Quest target')
        identity({k: p['target'][k] for k in ['key', 'revision']})
        if (p['target']['key'], p['target']['revision']) not in identities:
            raise ValueError('Unknown Quest target')
        if set(p['data']) != {'kind', 'profile'} or p['data']['kind'] != 'Quest':
            raise ValueError('Authoring profile tag')
        v = p['data']['profile']
        if set(v) - {'required_level', 'premium', 'repeatable', 'fields'}:
            raise ValueError('Unknown Quest authoring field')
        if 'required_level' in v and (type(v['required_level']) is not int or not 0 <= v['required_level'] <= 65535):
            raise ValueError('u16 required_level')
        for k in ['premium', 'repeatable']:
            if k in v and type(v[k]) is not bool:
                raise ValueError('Optional bool type')
        fields(v['fields'])

def build(root):
    root = pathlib.Path(root)
    index = json.loads((root / 'content/quests/definitions/index.json').read_bytes())
    records = []
    profiles = []
    rows = []
    sources = []
    held = []
    for path in index['shards']:
        raw = (root / path).read_bytes()
        packet_sha = sha(raw)
        sources.append({'path': path, 'sha256': packet_sha})
        for n, row in enumerate(json.loads(raw)['records']):
            if not production_key(row['definition']['identity']['key']):
                held.append({'identity': row['definition']['identity'], 'reason': 'EXISTING_PRODUCTION_KEY_RESERVED_MARKER', 'parser_error': 'invalid first-production namespaced key', 'parser_source': 'apps/game-server/src/content/production.rs:110,150', 'decision_request': 'Coordinator/architect must authorize canonical identity repair or accepted explicit alias; exporter must not silently invent either', 'canonical_ref': {'path': path, 'packet_sha256': packet_sha, 'json_pointer': f'/records/{n}/definition'}})
                continue
            decl, profile, report = project(row['definition'], path, f'/records/{n}/definition', packet_sha)
            records.append(decl)
            rows.append(report)
            if profile:
                profiles.append(profile)
    records.sort(key=lambda d: (d['identity']['key'], d['identity']['revision']))
    profiles.sort(key=lambda p: (p['target']['key'], p['target']['revision']))
    doc = {'schema': 'OTERYN_WORLD_PROJECT_DECLARATIONS/v2', 'records': records, 'authoring_profiles': profiles}
    shape_check(doc)
    report = {
        'scope': 'OFFLINE_V2_SOURCE_AUTHORING_OVERLAY_NOT_SERVER_INSTALL',
        'source_files': sources,
        'canonical_definitions': len(records) + len(held),
        'identity_holds': held,
        'declarations': len(records),
        'chosen_profiles': len(profiles),
        'source_catalogue_only': len(records) - len(profiles),
        'scalar_projections': dict(collections.Counter((k for p in profiles for k in p['data']['profile'] if k != 'fields'))),
        'native_admission': False,
        'runtime_activation': False,
        'whole_quest_complete': False,
        'parser_qualification': 'PYTHON_CLOSED_SHAPE_MATCH_TO_ACTUAL_RUST_TYPES_NOT_RUST_PARSER_RUN',
        'field_omissions': rows,
        'integration_required': ['Merge declarations/profiles by exact family/key/revision into existing definitions/declarations.json preserving existing families', 'Regenerate project manifest/lock through existing v2 package authoring; qualify package budgets and complete parser', 'Source metadata path/hash refs remain repository witnesses; server does not execute or dereference these metadata strings'],
    }
    return (doc, report)

def rendered(root):
    document, report = build(root)
    output = encode(document).encode() + b"\n"
    report["output_sha256"] = sha(output)
    report["output_bytes"] = len(output)
    return output, (encode(report) + "\n").encode(), report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=pathlib.Path,
                        default=pathlib.Path(__file__).resolve().parents[3])
    parser.add_argument("--out", type=pathlib.Path)
    parser.add_argument("--report", type=pathlib.Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    samples = args.repo_root / "tools/content-schema/quest-authoring/samples/v2-export"
    output_path = args.out or samples / "declarations.json"
    report_path = args.report or samples / "report.json"
    output, report_bytes, report = rendered(args.repo_root)
    for path, data in [(output_path, output), (report_path, report_bytes)]:
        if args.check:
            if not path.exists() or path.read_bytes() != data:
                raise SystemExit("v2 export differs: " + str(path))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    print({key: report[key] for key in [
        "declarations", "chosen_profiles", "source_catalogue_only",
        "scalar_projections", "output_bytes"]})


if __name__ == "__main__":
    main()
