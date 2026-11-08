"""Expected claim identities come from both registered authoring inputs."""
import importlib.util
import json


def normalized_keys(claims):
    keys = []
    for claim in claims:
        if (claim['claim']['per'] != 'character'
                or claim['claim']['repeat']['kind'] not in ('once', 'cooldown')):
            raise ValueError('Unsupported registered claim inventory shape')
        marker = claim['identity']['key'].split('reward-claim/', 1)[1].replace('/', '.')
        keys.append('oteryn:reward-claim.' + marker)
    if len(keys) != len(set(keys)):
        raise ValueError('Duplicate registered claim identity')
    return frozenset(keys)


def expected_keys(root):
    pilot = root / 'tools/content-schema/quest-authoring/samples/chests/claims.json'
    path = root / 'tools/content-schema/reward-claim-authoring/native_source_claims.py'
    spec = importlib.util.spec_from_file_location('questl2_native_claim_source', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    native, _, _ = module.load(root)
    return normalized_keys(json.loads(pilot.read_bytes())['claims'] + native)
