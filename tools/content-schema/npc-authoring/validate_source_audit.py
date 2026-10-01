"""Check portable NPC candidate custody, source branch hashes and admission holds."""
import argparse
import json
from collections import Counter
from pathlib import Path

from export_source_audit import digest, encode


def semantics(node):
    return {k: [semantics(child) for child in value] if k == 'children' else value
            for k, value in node.items() if k != 'key'}


def validate(root):
    manifest = json.loads((root / 'manifest.json').read_bytes())
    assert manifest['scope'] == 'SOURCE_AUTHORING_CANDIDATES_ONLY'
    for key in ('runtime_qualified', 'global_complete', 'active_content_modified'):
        assert manifest[key] is False, key
    payloads = {}
    for item in manifest['files']:
        path = Path(item['path'])
        assert path.name == item['path'] and path.suffix == '.json'
        raw = (root / path).read_bytes()
        assert len(raw) == item['bytes'] and digest(raw) == item['sha256'], str(path)
        assert b'C:\\\\Users' not in raw and b'C:/Users' not in raw, str(path)
        parsed = json.loads(raw)
        payloads[path.name] = parsed if path.name == 'root-proofs.json' else parsed['records']
    npcs = payloads['npc-candidates.json']
    dialogues = payloads['dialogue-candidates.json']
    services = payloads['service-corrections.json']
    index = payloads['source-index.json']
    assert len(npcs) == manifest['npc_candidates'] == 1112
    assert len(services) == manifest['service_corrections'] == 71
    assert len(index) == len(npcs)
    by_npc = {r['identity']['key']: r for r in npcs}
    by_dialogue = {r['identity']['key']: r for r in dialogues}
    assert len(by_npc) == len(npcs) and len(by_dialogue) == len(dialogues)
    assert {r['npc_key'] for r in index} == set(by_npc)
    assert manifest['unallocated_npc_proposals_included'] == 0
    assert not any(r['identity']['key'].startswith('oteryn:dialogue.observed.') for r in dialogues)
    for row in index:
        npc = by_npc[row['npc_key']]
        assert row['dialogue'] == npc.get('dialogue')
        for key in ('game_keyword_matcher_proven', 'game_handler_precedence_proven',
                    'quest_guards_focus_actions_proven', 'runtime_qualified', 'global_branch_complete'):
            assert row[key] is False, (row['npc_key'], key)
        if row['dialogue']:
            ref = row['dialogue']
            declaration = by_dialogue[ref['key']]
            assert ref['revision'] == declaration['identity']['revision']
            assert len(declaration['keywords']) == row['accepted_roots']
            assert digest(encode(declaration['keywords'])) == row['candidate_keywords_sha256']
        else:
            assert row['accepted_roots'] == 0
    assert sum(bool(d['keywords']) for d in dialogues) == manifest['nonempty_dialogues'] == 902
    assert sum(not d['keywords'] for d in dialogues) == manifest['empty_dialogue_holds'] == 39
    assert sum(r['accepted_roots'] for r in index) == manifest['accepted_roots'] == 20872
    assert sum(r['held_root_groups'] for r in index) == manifest['held_root_groups'] == 2505
    statuses = Counter(r['source_status'] for r in index if r['accepted_roots'])
    assert dict(statuses) == manifest['nonempty_source_statuses']
    proofs = payloads['root-proofs.json']
    assert {p['npc_key'] for p in proofs['npcs']} == {p['npc_key'] for p in index if p['accepted_roots']}
    for proof in proofs['npcs']:
        npc = by_npc[proof['npc_key']]
        roots = by_dialogue[npc['dialogue']['key']]['keywords']
        assert [r['key'] for r in roots] == [r[0] for r in proof['root_proofs']]
        for root, branch in zip(roots, proof['root_proofs']):
            raw = json.dumps(semantics(root), ensure_ascii=False, sort_keys=True,
                             separators=(',', ':')).encode('utf-8')
            assert digest(raw) == branch[1], (proof['npc_key'], branch[0])
    print('PASS: 1112 NPC candidates, 902 source programs, 20872 roots, 71 service corrections; unactivated')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('packet', type=Path)
    validate(parser.parse_args().packet)
