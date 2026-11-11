"""Reconcile every pinned A..M title without promoting partial quest proofs.

This is a local task record producer, not a runtime or content-authoring input.
Historical Native/binding queues are evidence of unfinished work, not an
exhaustive description of the current production adapters.
"""
import hashlib
import json
from collections import Counter
from pathlib import Path


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


matrix_path = 'tools/content-schema/quest-authoring/samples/completion-matrix/all373.json'
binding_path = 'content/quests/missions/completion-binding-plan.json'
source_binding_path = 'content/quests/missions/source-lowered-binding-plan.json'
coverage_path = 'docs/repository/QUEST_CALLBACK_COVERAGE.json'
inputs = [matrix_path, binding_path, source_binding_path, coverage_path]
authored_path = 'tools/content-schema/quest-authoring/samples/authored68/recipes.json'
inputs.append(authored_path)
authored = {row['wiki_title']: row for row in read(authored_path)['recipes']}
definitions = {}
for path in sorted(Path('content/quests/definitions').glob('quests-*.json')):
    inputs.append(str(path))
    for index, record in enumerate(read(path)['records']):
        definition = record['definition']
        definitions[definition['identity']['key']] = (
            definition, f'{path}:1#/records/{index}/definition'
        )
bindings = {row['quest']: (row, index)
            for index, row in enumerate(read(binding_path)['records'])}
source_bindings = {row['quest']: (row, index)
                   for index, row in enumerate(read(source_binding_path)['records'])}
coverage = {row['quest']: row for row in read(coverage_path)['quests']}

# The only whole-contract qualifications here are standalone claims with no
# NPC, mission, item-consume or world-object stage in the pinned source contract.
# Lone Medusa also retains the separately documented modern-access gap.
additional_access = {
    'Lone Medusa Quest': (
        '.questpc/research/medusa.md',
        'Modern access/quest-door route is not exercised by the reward-source fixture.'
    ),
    'Mintwallin Cyclops Quest': (
        'content/quests/definitions/quests-00000-00099.json',
        'The source contract includes item_consume and world_object; the chest '
        'scenario does not exercise that offering/access path.'
    ),
}

# These wiki titles share canonical owners with other, independently routed
# quests. Select the relevant branch rather than the family's first stage.
title_stage = {
    'Against the Spider Cult Quest': 's3',
    "Arito's Task Quest": 's2',
    'Into The Bone Pit Quest': 's4',
    'Jack to the Future Quest': 's6',
    'Loose Ends Quest': 's9',
    'Citizen Outfits Quest': 's2',
    'Barbarian Outfits Quest': 's6',
    'Mage Outfits Quest': 's10',
    'Formorgar Mines Quest': 's6',
}

# Each value names a reachable source operation, beyond the already proven
# starts/rewards, and the actual native boundary investigated in this task.
specific = {
    "A Father's Burden Quest": (
        'crates/gameplay/durability/src/durability/quest_state/npc_program.rs:103',
        'Eight material burns, per-material 2500XP, progress, and final Old Cape/'
        '8000XP are not composed with the storage-only writer. The atomic '
        'eleven-track acceptance is a separate retained proof.'
    ),
    "Bigfoot's Burden Quest": (
        'crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/npc_program.rs:774',
        "Xelvar's adventures branch sets storage and grants four crystal items; "
        'the existing real-dispatch case requires whole-branch refusal. '
        'Gnomebase world callbacks also remain in the held packet.'
    ),
    'Hot Cuisine Quest': (
        'crates/gameplay/quests/src/quest/npc_rules/evaluate.rs:319',
        'Jean Pierre greet-august needs authoritative CalendarMonth; '
        'greet-outside-august needs EndConversation(delay_ms=1000). The storage '
        'planner rejects those obligations. Cooking exchanges remain uncomposed. '
        'Fresh PG: command2603 hi Rejected!=Ok, classic_quests.rs:168, 114.00s.'
    ),
    'In Service of Yalahar Quest': (
        'content/quests/world-callbacks-held/batch3-a-in_service_of_yalahar.json:1',
        'Mission start is proven separately; material/kill progress and held '
        'world/access callbacks are not one complete executable quest path.'
    ),
    "A Pirate's Tail Quest": (
        'content/quests/world-callbacks-held/batch3-b-a_pirates_tail_quest.json:1',
        'Raid/world progress and held portal/encounter callbacks are not composed '
        'with the source item/NPC branches into a complete quest. Shared world '
        'engine work remains allocated to QENGINE.'
    ),
    'Hero of Rathleton Quest': (
        'content/quests/world-callbacks-held/batch3-b-hero_of_rathleton.json:1',
        'The pipe greeting/access receipt is proven separately; later item, '
        'combat and held world callbacks lack a complete composed dispatch proof.'
    ),
    'Exchange Quests': (
        'content/quests/definitions/quests-00100-00199.json:1',
        'Friends and Traders is the shared canonical owner; the per-NPC item '
        'exchange branches have source dialogue associations but no complete '
        'composed native exchange path or real-dispatch PG qualification.'
    ),
    'Barbarian Arena Quest': (
        'content/quests/missions/completion-binding-plan.json:1',
        'Halvar admission, ten sequential fights per tier, encounter ownership '
        'and exit/reset must compose before the Greenhorn/Scrapper/Warlord '
        'reward sources constitute a whole quest. QENGINE owns the shared '
        'encounter/reset pieces; reward sources are independently qualified.'
    ),
    'An Interest In Botany Quest': (
        'content/quests/missions/completion-binding-plan.json:1',
        'Rabaz start and Giant Dreadcoil/Obsidian Knife plus Giant Verminous/'
        'Spider Web sample callbacks lack a full real-dispatch proof; the '
        'Botany Almanach chest alone is already qualified.'
    ),
}

fresh_path = Path('.questpc/restart5-reward-pg.qualified.json')
fresh_result_path = Path('.questpc/restart5-reward-pg.result.json')
if not fresh_path.exists() or not fresh_result_path.exists():
    raise SystemExit('Fresh reward cohorts have not completed; no final statuses written')
fresh = read(fresh_path)
result = read(fresh_result_path)
if result['exit'] != 0:
    raise SystemExit('Fresh reward cohorts failed; inspect exact failures before final statuses')
test_source = 'crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs'
assert fresh['test_source_sha256'] == sha(test_source)
assert result['log_sha256'] == sha(result['log'])
log_text = Path(fresh['log']).read_text()
proofs = {}
for test, cohort in fresh['qualified'].items():
    assert result['test_results'][test] == 'ok'
    for claim, placements in cohort['claims'].items():
        assert placements
        for placement in placements:
            assert placement['marker'] in log_text
            assert 'POSTGRES_CASE_COMPLETED' in placement['marker']
            assert 'native_source=true' in placement['marker']
            assert float(placement['duration'][:-1]) > 0  # these scenarios emit seconds
        proofs[claim] = {'test': test, 'placements': placements,
                         'evidence': str(fresh_path), 'log': fresh['log']}

rows = []
for entry in read(matrix_path)['records']:
    title = entry['wiki_title']
    if not 'a' <= title[0].lower() <= 'm':
        continue
    assert len(entry['canonical']) == 1
    key = entry['canonical'][0]['key']
    definition, locator = definitions[key]
    source = definition.get('source_refs', {}).get('quest', {}).get('key')
    claims = coverage.get(source, {}).get('chests', [])
    features = definition.get('reported_source_readiness', {}).get('features', [])
    row = {'title': title, 'canonical': key, 'definition': locator,
           'status': 'BLOCKED', 'scope': 'complete pinned quest path',
           'reward_claims': [{'claim': claim['claim'], 'status': claim['status'],
                              'retained_test': claim['reason']} for claim in claims]}
    standalone = (
        definition['kind'] == 'reward_only'
        and definition['readiness'] == 'definition_ready'
        and not definition.get('shown_in_quest_log')
        and set(features) == {'reward_claim', 'trigger_use'}
        and not definition.get('source_refs', {}).get('gates')
        and 'outfit' not in title.lower()
        and title not in additional_access
    )
    if standalone:
        assert claims, title
        assert {claim['key'] for claim in definition['claims']} == {
            claim['claim'] for claim in claims
        }, title
        assert all(claim['status'] == 'dispatch_pg_qualified' for claim in claims), title
        assert all(claim['claim'] in proofs for claim in claims), title
        for claim in claims:
            expected = {tuple(p['position'][axis] for axis in ('x', 'y', 'z'))
                        for p in claim['placements']}
            actual = {tuple(p['position'])
                      for p in proofs[claim['claim']]['placements']}
            assert actual == expected, (title, claim['claim'], actual, expected)
        row.update(status='QUALIFIED', scope='standalone source reward-only contract',
                   qualification=[{'claim': claim['claim'], **proofs[claim['claim']]}
                                  for claim in claims])
    else:
        evidence = [locator]
        recipe = definition.get('oteryn_recipe', {}).get('payload', {}).get('recipe', {})
        if not recipe:
            recipe = authored.get(title, {})
            if recipe:
                evidence.append(authored_path + ':1#' + title)
        stages = recipe.get('stages', [])
        # Prefer the first material/world/encounter obligation rather than
        # asserting that a separately implemented initial greeting is absent.
        stage = next((s for s in stages if s['kind'] not in ('talk', 'complete')),
                     next((s for s in stages if s['kind'] != 'complete'), None))
        if title in title_stage:
            stage = next(s for s in stages if s['key'] == title_stage[title])
        if key in bindings:
            binding, index = bindings[key]
            evidence.append(f'{binding_path}:1#/records/{index}')
            if stage:
                candidate = next((s for s in binding['stages']
                                  if s['stage_key'] == stage['key']), None)
                if candidate:
                    row['queued_stage_binding'] = {
                        'stage': candidate['stage_key'],
                        'native_dispatch_binding': candidate['native_dispatch_binding'],
                        'runtime_enabled': candidate['runtime_enabled'],
                    }
        if key in source_bindings:
            binding, index = source_bindings[key]
            evidence.append(f'{source_binding_path}:1#/records/{index}')
            row['source_binding_queue'] = {
                'transitions': binding['transition_count'],
                'native_binding_count': binding['native_binding_count'],
            }
        if title == 'Dawnport Quest':
            missing = 'Owner-excluded Dawnport; no ordinary completion work authorized.'
            evidence.append('.questpc/OWNER-SCOPE-ALL.md:4')
            row['excluded'] = True
        elif title in specific:
            path, missing = specific[title]
            evidence.append(path)
        elif title in additional_access:
            path, missing = additional_access[title]
            evidence.append(path if ':' in path else path + ':1')
        elif 'outfit' in title.lower():
            obligation = (', '.join(stage.get('targets', [])) if stage else
                          'the source hand-in/addon branch')
            missing = (f'{title}: complete source objective/addon grant and durable outfit '
                       f'ownership ({obligation}) are not qualified through real NPC dispatch. '
                       'A reward chest or cfg(test) planner does not prove this path.')
            evidence.extend([
                'crates/gameplay/quests/src/quest/mod.rs:21',
                'crates/gameplay/quests/src/quest/npc_rules/evaluate.rs:364',
            ])
        elif stage:
            targets = ', '.join(stage.get('targets', [])) or stage['objective']
            missing = (f"No complete real-dispatch PG qualification for {stage['key']} "
                       f"({stage['kind']}: {targets}) and the subsequent completion/reward "
                       'composition. The identity/binding queue remains non-executable '
                       'evidence; existing source reward receipts do not prove this stage.')
        else:
            missing = ('Native trigger/progress/reward bindings and complete real-dispatch '
                       'PG path are absent from the authored recipe qualification. '
                       'The canonical definition retains WAITING_IMPLEMENTATION; '
                       'authored data is not production execution.')
        row['blocker'] = {'missing_piece': missing, 'evidence': evidence,
                          'attempts': [
                              'Read this canonical definition and chosen recipe, including source references.',
                              'Reconciled current reward coverage and retained PG locators; did not reuse obsolete claim-native-lowering holds.',
                              'Checked per-quest native/source binding queues against the existing generic production consumer and test-only batch3 planner boundary.',
                          ]}
        if title == 'Hot Cuisine Quest':
            row['blocker']['attempts'].append(
                'Ran catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic: '
                '.questpc/restart5-npc-scoped.log, failure2603 hi,114.00s; traced '
                'greet-august and greet-outside-august in quest-batch3-a-hot_cuisine.json.'
            )
        if stage:
            row['first_unqualified_stage'] = {
                'key': stage['key'], 'kind': stage['kind'],
                'objective': stage['objective'], 'targets': stage.get('targets', []),
            }
    rows.append(row)

assert len(rows) == 178
assert len({row['title'] for row in rows}) == len(rows)
assert all(row['status'] in ('QUALIFIED', 'BLOCKED') for row in rows)
assert all(row.get('qualification') if row['status'] == 'QUALIFIED'
           else row['blocker']['missing_piece'] and row['blocker']['evidence']
           and row['blocker']['attempts'] for row in rows)
counts = dict(Counter(row['status'] for row in rows))
report = {
    'task_id': 'questpc', 'scope': 'Pinned wiki titles A..M; no numeric or N..Z titles',
    'lifecycle': 'AUTHORING; no frozen candidate or publication',
    'counts': counts, 'owner_excluded_with_blocked_status': 1,
    'qualification_boundary': 'QUALIFIED is confined to a standalone source reward-only '
        'contract with every claim and placement proven on the current product tree. '
        'It does not certify deployed World/map admission or modern source changes. '
        'BLOCKED records unfinished implementation/qualification, not external approval '
        'requests. Partial NPC starts, rewards and test-only planners never qualify '
        'an entire storyline or outfit.',
    'binding_queue_boundary': 'Historical binding queues are not exhaustive runtime '
        'registries. Their null fields identify outstanding composition/evidence, '
        'not proof that no bespoke adapter exists.',
    'inputs': {path: sha(path) for path in inputs},
    'fresh_pg': result, 'quests': rows,
}
Path('.questpc/QUEST-STATUS.json').write_text(json.dumps(report, indent=2) + '\n')
lines = ['# questpc A..M final statuses', '',
         f"{len(rows)} pinned titles: {counts.get('QUALIFIED', 0)} QUALIFIED standalone "
         f"reward-only contracts; {counts.get('BLOCKED', 0)} BLOCKED (includes one "
         'owner-excluded Dawnport title).', '', report['qualification_boundary'], '',
         'Exact claims, per-placement POSTGRES_CASE_COMPLETED markers/durations, '
         'canonical JSON pointers, attempts, input hashes and command results: '
         '`QUEST-STATUS.json`.', '',
         'Historical Native queues do not override bespoke runtime adapters. '
         'No runtime/content readiness flags were changed by this status audit.', '',
         '| Quest | Status | Evidence / exact missing piece |',
         '| --- | --- | --- |']
for row in rows:
    if row['status'] == 'QUALIFIED':
        tests = sorted({p['test'] for p in row['qualification']})
        durations = ', '.join(p['duration'] for proof in row['qualification']
                              for p in proof['placements'])
        detail = ', '.join(f'`{t}`' for t in tests) + '; ' + durations
    else:
        detail = row['blocker']['missing_piece'] + ' `' + row['blocker']['evidence'][0] + '`'
    lines.append('| ' + row['title'].replace('|', '\\|') + ' | ' + row['status']
                 + ' | ' + detail.replace('|', '\\|') + ' |')
Path('.questpc/QUEST-STATUS.md').write_text('\n'.join(lines) + '\n')
print(json.dumps({'titles': len(rows), **counts, 'all_final': True}))
