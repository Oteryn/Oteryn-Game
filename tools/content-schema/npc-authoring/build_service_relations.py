"""Build all source-only service dependencies; never lower source ids to native Quests."""
import argparse
import hashlib
import json
import pathlib
import re
import importlib.util
import urllib.request
import sys

sys.dont_write_bytecode = True

R7 = None
SOURCE_DIR = None
import service_source_conditions as SIMULATOR
TRACKS = {
    'Storage.Quest.U7_4.DjinnWar.EfreetFaction.Mission03': 'canary:quest-progress/quest/u7_4/djinn_war/efreet_faction/mission03',
    'Storage.Quest.U7_4.DjinnWar.MaridFaction.Mission03': 'canary:quest-progress/quest/u7_4/djinn_war/marid_faction/mission03',
    'Storage.Quest.U8_1.TheTravellingTrader.Mission07': 'canary:quest-progress/quest/u8_1/the_travelling_trader/mission07',
    'Storage.Quest.U7_24.ThePostmanMissions.Mission01': 'canary:quest-progress/quest/u7_24/the_postman_missions/mission01',
    'Storage.Quest.U7_24.ThePostmanMissions.Rank': 'canary:quest-progress/quest/u7_24/the_postman_missions/rank',
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def encoded(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def resolve_pointer(document, pointer):
    value = document
    for token in pointer.lstrip('/').split('/'):
        token = token.replace('~1', '/').replace('~0', '~')
        value = value[int(token)] if isinstance(value, list) else value[token]
    return value


def verify_registration_custody(document, custody):
    route = resolve_pointer(document, custody['pointer'])
    if not isinstance(route, dict) or not {'destination', 'price', 'discount', 'destination_keyword'} <= route.keys():
        raise ValueError('registration custody must identify a whole route')
    if sha(encoded(route)) != custody['row_sha256']:
        raise ValueError('registration route custody drifted')
    if custody['discount_field_pointer'] != custody['pointer'] + '/discount':
        raise ValueError('discount field does not belong to custody route')
    if sha(encoded(resolve_pointer(document, custody['discount_field_pointer']))) != custody['discount_field_sha256']:
        raise ValueError('registration discount field custody drifted')
    return route


def verify_hold_custody(repo, held, documents):
    custody = held['custody']
    path = custody['file']
    if path not in documents:
        documents[path] = json.loads((repo / path).read_text())
    if 'pointer' in custody:
        original = resolve_pointer(documents[path], custody['pointer'])
    else:
        fields = [field for row in documents[path]['records'] if row['identity']['key'] == custody['service']
                  for field in row.get('fields', []) if field['field_path'] == custody['field']]
        if len(fields) != 1:
            raise ValueError('held offer custody is not unique')
        original = json.loads(fields[0]['value']['value'])
    if encoded(original) != encoded(held['record']):
        raise ValueError('held offer custody drifted')


def file_evidence(repo, path):
    raw = (repo / path).read_bytes()
    return {'path': path, 'sha256': sha(raw), 'bytes': len(raw)}


def verified_capture(index, source, path):
    rows = [row for row in index['records'] if row['source'] == source and row['path'] == path]
    if len(rows) != 1 or rows[0]['status'] != 'CAPTURED':
        raise ValueError('exact source capture unavailable')
    row = rows[0]
    raw = (SOURCE_DIR / (row['sha256'] + '.lua')).read_bytes()
    if sha(raw) != row['sha256']:
        raise ValueError('source digest mismatch')
    return raw, {key: row[key] for key in ('repository', 'revision', 'path', 'url', 'sha256')}


def proof(raw, origin, start, end):
    return {**origin, 'byte_start': start, 'byte_end_exclusive': end,
            'line_start': raw[:start].count(b'\n') + 1,
            'line_end': raw[:end].count(b'\n') + 1, 'fragment_sha256': sha(raw[start:end])}


def span(raw, origin, pattern):
    hits = list(re.finditer(pattern, raw))
    if len(hits) != 1:
        raise ValueError('nonunique reviewed source branch')
    match = hits[0]
    return proof(raw, origin, match.start(), match.end())


def merchant_branch(callback, symbol, expected, slug):
    receiver = rb'Player\(creature\)' if slug == 'rashid' else rb'player'
    if re.findall(rb'\breturn\s+(\w+)', callback) != [b'false', b'true']:
        raise ValueError('merchant false/true source branches drifted')
    denial = (rb'if ' + receiver + rb':getStorageValue\(' + re.escape(symbol) + rb'\) ~= '
        + str(expected).encode() + rb' then\s*npcHandler:say\([^\n]*\)\s*return false\s*end\s*return true\s*end$')
    matched = re.search(denial, callback)
    if not matched or len(re.findall(rb'\bif\b', callback)) != 1:
        raise ValueError('merchant false/true source branches drifted')
    return matched


def reviewed_travel_flow(travel):
    if re.findall(rb'\breturn\s+(\w+)', travel) != [b'false', b'true']:
        raise ValueError('reviewed upstream travel return/action path changed')
    money = travel.index(b'elseif not player:removeMoneyBank(cost) then')
    cooldown = travel.index(b'if hasExhaustion > os.time() then')
    teleport = travel.index(b'player:teleportTo(destination)')
    returned = travel.rindex(b'return true')
    if not money < cooldown < teleport < returned:
        raise ValueError('reviewed upstream travel return/action path changed')
    return money, cooldown, teleport, returned


def compare_source(condition, state):
    """Tri-state source simulation. Unknown values cannot manufacture quest progress."""
    if condition['kind'] != 'CompareSourceTrack':
        raise ValueError('not a source track comparison')
    source = source_condition(condition)
    return SIMULATOR.source_compare(source, state)


def source_condition(condition):
    return {'kind': 'SOURCE_TRACK_COMPARE', 'source_track': condition['track']['existing_source_id'],
            'operator': {'Equal': '==', 'GreaterOrEqual': '>='}[condition['operator']],
            'value': condition['value']}


def effect_preview(effect, state, outcome):
    """Source-only preview; a known durable commit survives disconnect/recovery."""
    if outcome == 'AFTER_KNOWN_COMMIT':
        return {'state': 'KNOWN_COMMIT_AUTHORITATIVE', 'effect': 'PreserveCommittedQuestEffects',
                'arrival_obligation': 'PreserveForFencedPlacementAndRecovery', 'native_eligible': False}
    return SIMULATOR.transition_preview({'source_condition': source_condition(effect['condition']),
        'to': effect['set_to']}, state, outcome == 'SuccessfulTravelCommit')


def discount_preview(row, state, globally_qualified=False):
    """A title hypothesis cannot qualify a route; placeholder eligibility is rejected."""
    if not globally_qualified:
        return {'state': 'HOLD', 'reason': 'EXACT_GLOBAL_ROUTE_PRIVILEGE_REQUIRED', 'price': None}
    privilege = row['prospective_global_privilege']
    matched = compare_source(privilege, state)
    if matched is None:
        return {'state': 'HOLD', 'reason': 'UNKNOWN_SOURCE_TRACK', 'price': None}
    price = SIMULATOR.fare(row['base_gold'], [{'source_condition': source_condition(privilege),
        'amount_gold': row['effect']['subtract_gold']}], state)
    return {'state': 'SOURCE_PREVIEW_ONLY', 'price': price, 'native_eligible': False}


def build(repo):
    travel_semantics = load_module(repo / 'tools/content-schema/npc-authoring/travel_semantics.py', 'travel_semantics')
    report_path = R7 / 'service-condition-report.json'
    report = json.loads(report_path.read_text())
    transport_path = 'docs/agents/evidence/OTV2-20261001-npc-source-audit-r5/transport-repaired.json'
    transport = json.loads((repo / transport_path).read_text())
    index = json.loads((R7 / 'source-index.json').read_text())
    quest_path = 'docs/agents/evidence/OTV2-20261001-npc-source-audit-r5/quest-stage.json'
    quest = json.loads((repo / quest_path).read_text())
    registry = {row['record']['key']: row for row in quest['source_progress']}
    registry_positions = {symbol: i for i, symbol in enumerate(sorted(TRACKS))}
    def condition(source):
        symbol = source['source_track']
        source_id = TRACKS[symbol]
        registry[source_id]  # Only existing source-track identities are allowed.
        return {'kind': 'CompareSourceTrack', 'operator': {'==': 'Equal', '>=': 'GreaterOrEqual'}[source['operator']],
                'value': source['value'], 'track': {'domain': 'SOURCE_ONLY', 'symbol': symbol,
                    'existing_source_id': source_id, 'registry_pointer': '/source_track_registry/' + str(registry_positions[symbol])},
                'unknown_result': 'Hold', 'native_predicate': None}
    # Both source versions bind their literal identity and the whole registered callback.
    merchant = {}
    for entry in report['merchant_access']:
        slug = entry['service'].split('.')[-1]
        guards = []
        for source, prefix in [('canary', 'data-otservbr-global'), ('crystal', 'data-global')]:
            raw, origin = verified_capture(index, source, prefix + '/npc/' + slug + '.lua')
            name = re.search(rb'local internalNpcName = "([^"]+)"', raw)
            callback = re.search(rb'local function onTradeRequest\b[\s\S]*?\nend', raw)
            symbol = entry['source_condition']['source_track'].encode()
            expected = entry['source_condition']['value']
            if not callback or not name or b'CALLBACK_ON_TRADE_REQUEST, onTradeRequest' not in raw:
                raise ValueError('merchant binding unavailable')
            denied = merchant_branch(callback[0], symbol, expected, slug)
            guards.append({'source_npc': source + ':npc/' + slug, 'literal_internalNpcName': name[1].decode(),
                'callback_proof': proof(raw, origin, callback.start(), callback.end()),
                'denied_branch': {'comparison': 'NotEqual', 'value': expected, 'result': 'DenyTrade'},
                'allowed_branch': {'comparison': 'Equal', 'value': expected, 'result': 'AllowTradeRequest'}})
        merchant[entry['service']] = {'condition': condition(entry['source_condition']), 'source_branches': guards}
    flow = []
    for source in ('canary', 'crystal'):
        raw, origin = verified_capture(index, source, 'data/npclib/npc_system/modules.lua')
        start = raw.index(b'function StdModule.travel(')
        stop = raw.index(b'\n\tFocusModule', start)
        travel = raw[start:stop]
        money, cooldown, teleport, returned = reviewed_travel_flow(travel)
        handler, handler_origin = verified_capture(index, source, 'data/npclib/npc_system/keyword_handler.lua')
        action = span(handler, handler_origin, rb'if childNode:processMessage\(npc, player, messageLower\) then\s*childNode:processAction\(player, messageLower\)\s*return true')
        flow.append({'source': source, 'travel_function': proof(raw, origin, start, stop),
            'money_debit_branch': proof(raw, origin, start + money, start + money + len(b'elseif not player:removeMoneyBank(cost) then')),
            'cooldown_after_debit': proof(raw, origin, start + cooldown, start + cooldown + len(b'if hasExhaustion > os.time() then')),
            'action_dispatch': action, 'refusal_callback_result': True,
            'findings': ['QUEST_ACTION_RUNS_AFTER_TRAVEL_REFUSAL', 'MONEY_DEBIT_PRECEDES_COOLDOWN_REFUSAL'],
            'source_outcomes': ['PremiumRefused', 'LevelRefused', 'PzLocked', 'FundsRefused', 'CooldownRefused', 'Teleported'],
            'planned_required_order': ['ValidateSessionGeneration', 'AcquireCharacterRoot', 'RecheckAllAccessAndCooldown',
                'ComputeQualifiedFare', 'VerifyFunds', 'RequireAcceptedOwningTravelAndQuestContracts',
                'CommitFeePendingArrivalObligationAndQualifiedQuestEffects', 'PublishDurableCommittedOutcome',
                'FencedRuntimePlacement', 'ConsumeArrivalObligationAccordingToOwnerContract']})
    offers = []
    custody_documents = {}
    for i, held in enumerate(report['all_held_offers']):
        verify_hold_custody(repo, held, custody_documents)
        old = held['record']
        service = old['service']
        reason = old['reason']
        if reason == 'SOURCE_TRADE_ACCESS_NATIVE_PREDICATE_UNAVAILABLE':
            relation = {'kind': 'RequiresSourceQuestCompletion', 'condition': merchant[service]['condition'],
                'source_branch_binding_pointer': '/merchant_source_bindings/' + service,
                'recheck_phases': ['OfferVisibility', 'BeforeTradeCommitUnderCharacterRoot']}
            dependencies = ['exact-Global-merchant-access-proof', 'QUEST-CONTENT-1', 'QUEST-PRED-1', 'NPC-TRADE-1']
        else:
            relation = {'kind': 'RequiresQualifiedItemVariant', 'source_count': old.get('source_count'),
                'source_sub_type': old.get('source_sub_type'), 'source_fluid_symbol': old.get('source_fluid_symbol'),
                'source_observations': [{key: value for key, value in row.items() if key not in ('source_proof', 'old_source_offer')}
                    for row in old.get('source_observations', [])], 'native_variant_binding': None,
                'quantity_policy': 'DoNotReinterpretFluidEnumOrChargeCountAsTransactionUnits'}
            dependencies = ['ITEM-TRADE-variant-binding', 'NPC-TRADE-1', 'exact-full-tuple-source-qualification']
        offers.append({'existing_service': service, 'existing_npc': old['npc'], 'held_native_tuple': old['old_native_offer'],
            'source_authority': 'OTS_HYPOTHESIS_ONLY' if reason == 'SOURCE_TRADE_ACCESS_NATIVE_PREDICATE_UNAVAILABLE' else 'RETAINED_UNQUALIFIED_VARIANT_EVIDENCE',
            'hold_reason': reason, 'source_custody': held['custody'], 'r7_hold_pointer': '/all_held_offers/' + str(i),
            'r7_hold_sha256': sha(encoded(held)), 'relation': relation, 'required_dependencies': dependencies,
            'native_gate': None, 'runtime_eligible': False, 'authoring_disposition': 'EXACT_HELD_TUPLE_IMPLEMENTATION_DEPENDENCIES_CLASSIFIED'})
    routes = []
    for effect in report['travel_effects']:
        holds = [(i, row) for i, row in enumerate(transport['scripted_route_holds'])
                 if row['service'] == effect['service'] and row['route']['key'] == effect['route']]
        if len(holds) != 1:
            raise ValueError('route effect hold custody unavailable')
        hold_index, hold = holds[0]
        effect_proofs = []
        slug = effect['service'].split('.')[-1]
        for source, prefix in [('canary', 'data-otservbr-global'), ('crystal', 'data-global')]:
            raw, origin = verified_capture(index, source, prefix + '/npc/' + slug + '.lua')
            matches = list(re.finditer(rb'addTravelKeyword\("' + effect['route'].encode() + rb'", ([0-9]+), Position\(([0-9]+), ([0-9]+), ([0-9]+)\), function\(player\)\n([\s\S]*?)\nend\)', raw))
            route = matches[0] if len(matches) == 1 else None
            symbol = re.escape(effect['source_condition']['source_track'].encode())
            body = (rb'\s*if player:getStorageValue\(' + symbol + rb'\) == '
                + str(effect['source_condition']['value']).encode() + rb' then\s*player:setStorageValue\('
                + symbol + rb', ' + str(effect['to']).encode() + rb'\)\s*end\s*')
            if not route or not re.fullmatch(body, route[5]):
                raise ValueError('exact route action/guard changed')
            target = hold['route']['destination']
            if tuple(map(int, route.group(1, 2, 3, 4))) != (hold['route']['price'], target['x'], target['y'], target['floor']):
                raise ValueError('route effect tuple custody drifted')
            action_registration = span(raw, origin, rb'travelKeyword:addChildKeyword\(\{ "yes" \}, StdModule.travel, [^\n]*, nil, action\)')
            effect_proofs.append({'source_npc_id': source + ':npc/' + slug,
                'route_registration_and_effect': proof(raw, origin, route.start(), route.end()),
                'base_gold': int(route[1]), 'destination': dict(zip(('x', 'y', 'z'), map(int, route.group(2, 3, 4)))),
                'callback_action_binding': action_registration})
        routes.append({'existing_service': effect['service'], 'route_key': effect['route'],
            'held_native_route': hold['route'], 'source_custody': {'path': transport_path,
                'pointer': '/scripted_route_holds/' + str(hold_index), 'row_sha256': sha(encoded(hold))},
            'source_effect': {'kind': 'SetSourceTrackAfterCommit', 'condition': condition(effect['source_condition']),
                'set_to': effect['to'], 'on': 'SuccessfulTravelCommit', 'on_pre_commit_refusal': 'NoQuestMutation',
                'after_known_commit': 'PreserveCommittedEffectsAndArrivalObligationThroughDisconnectRecovery',
                'commit_prerequisite': 'ADMITTED_OWNING_TRAVEL_AND_QUEST_CONTRACTS',
                'requires_compare_and_set': True, 'native_transition': None},
            'source_proofs': effect_proofs, 'source_flow_finding_pointers': ['/source_flow_findings/0', '/source_flow_findings/1'],
            'required_dependencies': ['exact-Global-route-quest-effect-proof', 'QUEST-CONTENT-1', 'QUEST-STATE-1', 'QUEST-PRED-1', 'NPC-TRAVEL-1'],
            'native_effect': None, 'runtime_eligible': False})
    for route in report['wiki_route_access']:
        routes.append({'existing_service': route['service'], 'route_key': route['route']['key'],
            'held_native_route': route['route'], 'source_gate': {'kind': 'NamedSourcePrivilege',
                'value': route['source_access']['value'], 'existing_source_track': None,
                'evidence_scope': route['source_access']['evidence_scope'], 'numeric_threshold': None,
                'qualification': 'CITIZEN_TITLE_NOT_POINT_TALLY'}, 'source_custody': route['source_observation'],
            'source_conflict': {'donor_price': 200, 'wiki_price': 110, 'donor_citizen_predicate': 'ABSENT'},
            'required_dependencies': ['qualified-Citizen-title-grant-source', 'QUEST-CONTENT-1', 'QUEST-PRED-1', 'NPC-TRAVEL-1'],
            'native_gate': None, 'runtime_eligible': False})
    candidates_path = 'tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json'
    candidates = json.loads((repo / candidates_path).read_text())
    discounts = []
    helper_proofs = []
    rank_promotion = []
    for source in ('canary', 'crystal'):
        prefix = 'data-otservbr-global' if source == 'canary' else 'data-global'
        raw, origin = verified_capture(index, source, prefix + '/npc/kevin.lua')
        grant = span(raw, origin, rb'elseif npcHandler:getTopic\(playerId\) == 20 then\s*npcHandler:say\([^\n]*\)\s*player:setStorageValue\(Storage\.Quest\.U7_24\.ThePostmanMissions\.Rank, 3\)\s*npcHandler:setTopic\(playerId, 19\)')
        rank_promotion.append({'existing_source_npc_id': source + ':npc/kevin', 'topic_guard': 20,
            'source_track_id': TRACKS['Storage.Quest.U7_24.ThePostmanMissions.Rank'], 'set_rank': 3,
            'next_topic': 19, 'branch_proof': grant, 'native_transition': None})
        raw, origin = verified_capture(index, source, 'data/npclib/npc_system/custom_modules.lua')
        binding = span(raw, origin, rb'\["postman"\] = \{ price = 10, storage = Storage\.Quest\.ExampleQuest, value = 1 \}')
        helper_proofs.append({'binding': binding, 'source_condition': {'symbol': 'Storage.Quest.ExampleQuest',
            'operator': 'GreaterOrEqual', 'value': 1, 'disposition': 'PLACEHOLDER_REJECTED'},
            'effect': {'kind': 'SubtractGold', 'amount': 10}})
    for i, observed in enumerate(transport['conditional_discounts']):
        parts = observed['promotion_pointer'].strip('/').split('/')
        if len(parts) != 6 or parts[-1] != 'discount':
            raise ValueError('source discount pointer shape changed')
        candidate = candidates['candidates'][int(parts[1])]
        route = candidate['travel_service']['routes'][int(parts[4])]
        if candidate['travel_service']['identity']['key'] != observed['service'] or travel_semantics.canonical_keyword(route['destination_keyword']) != observed['route'] or route['discount'] != 'postman':
            raise ValueError('exact source route registration drifted')
        quote = observed['quote_source_proof']
        if route['price'] != observed['base_price'] and (not quote or quote['base_price'] != observed['base_price']):
            raise ValueError('source/base fare difference has no retained R5 quote custody')
        registration_custody = {**file_evidence(repo, candidates_path), 'pointer': '/' + '/'.join(parts[:-1]),
            'row_sha256': sha(encoded(route)), 'source_destination_keyword': route['destination_keyword'],
            'discount_field_pointer': observed['promotion_pointer'], 'discount_field_sha256': sha(encoded(route['discount']))}
        verify_registration_custody(candidates, registration_custody)
        discounts.append({'existing_service': observed['service'], 'route_key': observed['route'],
            'existing_source_npc_ids': {source: value['key'] for source, value in candidate['provenance'].items()},
            'registration_custody': registration_custody,
            'source_discount_key': 'postman', 'base_gold': observed['base_price'],
            'original_candidate_gold': route['price'], 'retained_quote_custody': quote,
            'effect': {'kind': 'SubtractGoldWithZeroFloor', 'subtract_gold': observed['amount_gold']},
            'actual_donor_helper_pointers': ['/discount_source_helpers/0', '/discount_source_helpers/1'],
            'prospective_global_privilege': condition(report['postman']['source_condition']),
            'source_privilege_status': 'RANK3_GRAND_POSTMAN_SOURCE_HYPOTHESIS; EXACT_GLOBAL_ROUTE_ELIGIBILITY_UNPROVEN',
            'source_observation_custody': {'path': 'docs/agents/evidence/OTV2-20261001-npc-source-audit-r5/transport-repaired.json',
                'pointer': '/conditional_discounts/' + str(i)},
            'required_dependencies': ['exact-Global-route-privilege-proof', 'QUEST-CONTENT-1', 'QUEST-PRED-1', 'NPC-TRAVEL-1'],
            'native_gate': None, 'runtime_eligible': False})
    if (len(offers), len(routes), len(discounts)) != (457, 3, 171):
        raise ValueError('scope inventory drifted')
    return {'schema': 'NPC_SOURCE_SERVICE_IMPLEMENTATION_PACKET/v1', 'execution_domain': 'OFFLINE_AUTHORING_ONLY',
        'runtime_eligible': False, 'native_quest_declarations': [], 'native_track_declarations': [],
        'native_transition_declarations': [],
        'inputs': [file_evidence(repo, path) for path in sorted(custody_documents)] + [file_evidence(repo, quest_path), file_evidence(repo, candidates_path),
            file_evidence(repo, 'docs/agents/evidence/OTV2-20261001-npc-source-audit-r5/transport-repaired.json'),
            file_evidence(repo, 'tools/content-schema/npc-authoring/travel_semantics.py'),
            {'path': 'r7/quests/service-condition-report.json', 'sha256': sha(report_path.read_bytes())},
            {'path': 'r7/quests/source-index.json', 'sha256': sha((R7/'source-index.json').read_bytes())},
            {'path': 'tools/content-schema/npc-authoring/service_source_conditions.py', 'sha256': sha(pathlib.Path(SIMULATOR.__file__).read_bytes())}],
        'source_track_registry': [{'symbol': symbol, 'existing_source_id': key, 'registry_evidence': registry[key]['evidence']}
            for symbol, key in sorted(TRACKS.items())], 'source_flow_findings': flow,
        'merchant_source_bindings': merchant, 'discount_source_helpers': helper_proofs,
        'postman_source_rank_promotion': rank_promotion,
        'counts': {'held_offer_relations': 457, 'route_relations': 3, 'discount_relations': 171,
            'merchant_gates': 284, 'variant_dependencies': 173, 'total_relations': 631},
        'offer_relations': offers, 'route_relations': routes, 'discount_relations': discounts}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=pathlib.Path, required=True)
    parser.add_argument('--evidence-dir', type=pathlib.Path, required=True,
                        help='R7 quests report/source index; retained structured evidence only')
    parser.add_argument('--source-dir', type=pathlib.Path, required=True,
                        help='External raw cache; never write source Lua into Git')
    parser.add_argument('--fetch-sources', action='store_true',
                        help='Fetch public pinned Lua into the external cache and verify SHA256')
    parser.add_argument('--out', type=pathlib.Path, required=True)
    args = parser.parse_args()
    R7, SOURCE_DIR = args.evidence_dir, args.source_dir
    if SOURCE_DIR.resolve().is_relative_to(args.repo.resolve()):
        raise ValueError('raw capture cache must remain outside the repository')
    if args.fetch_sources:
        SOURCE_DIR.mkdir(parents=True, exist_ok=True)
        for row in json.loads((R7 / 'source-index.json').read_text())['records']:
            if row['source'] not in ('canary', 'crystal') or not row['path'].endswith('.lua'):
                continue
            expected_url = 'https://raw.githubusercontent.com/' + row['repository'] + '/' + row['revision'] + '/' + row['path']
            if row['url'] != expected_url or not re.fullmatch('[a-f0-9]{40}', row['revision']):
                raise ValueError('not an exact public pinned source URL')
            target = SOURCE_DIR / (row['sha256'] + '.lua')
            raw = target.read_bytes() if target.exists() else urllib.request.urlopen(expected_url, timeout=30).read()
            if sha(raw) != row['sha256']:
                raise ValueError('source digest mismatch')
            target.write_bytes(raw)
    result = build(args.repo)
    args.out.write_text(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2) + '\n')
    print(json.dumps(result['counts'], sort_keys=True))
