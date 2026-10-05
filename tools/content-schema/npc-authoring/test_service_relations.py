"""Regressions for source custody, rejected gates and upstream refusal paths."""
import argparse
import copy
import hashlib
import json
import pathlib
import re
import tempfile
import unittest

import build_service_relations as tool

REPO = None


def walk(value):
    yield value
    if isinstance(value, dict):
        for child in value.values():
            yield from walk(child)
    elif isinstance(value, list):
        for child in value:
            yield from walk(child)


class SourceRelationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if REPO is None:
            raise unittest.SkipTest('Run dedicated CLI with --repo --evidence-dir --source-dir for pinned 631-relation fixtures')
        cls.packet = tool.build(REPO)
        cls.index = json.loads((tool.R7/'source-index.json').read_text())
        cls.report = json.loads((tool.R7/'service-condition-report.json').read_text())

    def test_complete_inventory_and_null_runtime_bindings(self):
        p = self.packet
        self.assertEqual([len(p[k]) for k in ('offer_relations','route_relations','discount_relations')], [457,3,171])
        self.assertEqual(p['counts']['total_relations'],631)
        self.assertEqual(sum(r['relation']['kind']=='RequiresSourceQuestCompletion' for r in p['offer_relations']),284)
        for name in ('native_quest_declarations','native_track_declarations','native_transition_declarations'):
            self.assertEqual(p[name],[])
        for r in p['offer_relations']+p['route_relations']+p['discount_relations']:
            self.assertFalse(r['runtime_eligible'])
        for value in walk(p):
            if isinstance(value,dict):
                for key in ('native_gate','native_predicate','native_transition','native_effect','native_variant_binding'):
                    if key in value:
                        self.assertIsNone(value[key])

    def test_all_original_offer_custody_and_tuple_drift_rejected(self):
        docs = {}
        for h in self.report['all_held_offers']:
            tool.verify_hold_custody(REPO,h,docs)
        changed = copy.deepcopy(self.report['all_held_offers'][0])
        changed['record']['old_native_offer']['unit_price'] += 1
        with self.assertRaisesRegex(ValueError,'custody drifted'):
            tool.verify_hold_custody(REPO,changed,docs)

    def test_raw_digest_tamper_rejected(self):
        row = next(r for r in self.index['records'] if r['source']=='canary' and r['path'].endswith('/alesar.lua'))
        previous = tool.SOURCE_DIR
        try:
            with tempfile.TemporaryDirectory() as temporary:
                tool.SOURCE_DIR = pathlib.Path(temporary)
                (tool.SOURCE_DIR/(row['sha256']+'.lua')).write_bytes(b'changed source')
                with self.assertRaisesRegex(ValueError,'digest mismatch'):
                    tool.verified_capture(self.index,'canary',row['path'])
        finally:
            tool.SOURCE_DIR = previous

    def test_merchant_denial_branch_cannot_become_allow(self):
        raw,_ = tool.verified_capture(self.index,'canary','data-otservbr-global/npc/rashid.lua')
        callback = re.search(rb'local function onTradeRequest\b[\s\S]*?\nend',raw)[0]
        symbol = b'Storage.Quest.U8_1.TheTravellingTrader.Mission07'
        tool.merchant_branch(callback,symbol,1,'rashid')
        for changed in (callback.replace(b'return false',b'return true'), callback.replace(b'~= 1',b'~= 2')):
            with self.assertRaisesRegex(ValueError,'branches drifted'):
                tool.merchant_branch(changed,symbol,1,'rashid')

    def test_merchant_unknown_and_completion_boundaries(self):
        cond = self.packet['merchant_source_bindings']['oteryn:service.trade.alesar']['condition']
        key = cond['track']['existing_source_id']
        self.assertIsNone(tool.compare_source(cond,{}))
        self.assertIsNone(tool.compare_source(cond,{key:True}))
        self.assertFalse(tool.compare_source(cond,{key:2}))
        self.assertTrue(tool.compare_source(cond,{key:3}))
        self.assertFalse(tool.compare_source(cond,{key:4}))

    def test_travel_refusal_success_and_guard_boundaries(self):
        effect = self.packet['route_relations'][0]['source_effect']
        key = effect['condition']['track']['existing_source_id']
        state = {key:1}
        for outcome in ('PremiumRefused','LevelRefused','PzLocked','FundsRefused','CooldownRefused','PRE_COMMIT_REFUSAL','DestinationInvalidBeforeCommit','DebitFailed'):
            self.assertIsNone(tool.effect_preview(effect,state,outcome))
        self.assertEqual(tool.effect_preview(effect,state,'SuccessfulTravelCommit'),{'source_track':key,'from':1,'to':2})
        self.assertEqual(state,{key:1})
        self.assertIsNone(tool.effect_preview(effect,{},'SuccessfulTravelCommit'))
        self.assertIsNone(tool.effect_preview(effect,{key:2},'SuccessfulTravelCommit'))
        committed_state = {key:2}
        preserved = tool.effect_preview(effect,committed_state,'AFTER_KNOWN_COMMIT')
        self.assertEqual(preserved['effect'],'PreserveCommittedQuestEffects')
        self.assertEqual(preserved['arrival_obligation'],'PreserveForFencedPlacementAndRecovery')
        self.assertFalse(preserved['native_eligible'])
        self.assertEqual(committed_state,{key:2})
        self.assertEqual(effect['on_pre_commit_refusal'],'NoQuestMutation')
        self.assertIn('ThroughDisconnectRecovery',effect['after_known_commit'])

    def test_upstream_cooldown_debit_and_return_drift(self):
        for finding in self.packet['source_flow_findings']:
            order = finding['planned_required_order']
            commit = order.index('CommitFeePendingArrivalObligationAndQualifiedQuestEffects')
            self.assertLess(order.index('RecheckAllAccessAndCooldown'),commit)
            self.assertLess(order.index('RequireAcceptedOwningTravelAndQuestContracts'),commit)
            self.assertLess(commit,order.index('FencedRuntimePlacement'))
            self.assertLess(order.index('FencedRuntimePlacement'),order.index('ConsumeArrivalObligationAccordingToOwnerContract'))
        for source in ('canary','crystal'):
            raw,_ = tool.verified_capture(self.index,source,'data/npclib/npc_system/modules.lua')
            start = raw.index(b'function StdModule.travel(')
            stop = raw.index(b'\n\tFocusModule',start)
            travel = raw[start:stop]
            money,cooldown,teleport,returned = tool.reviewed_travel_flow(travel)
            self.assertLess(money,cooldown)
            self.assertLess(cooldown,teleport)
            self.assertLess(teleport,returned)
            with self.assertRaisesRegex(ValueError,'return/action path changed'):
                tool.reviewed_travel_flow(travel[:returned]+travel[returned:].replace(b'return true',b'return false',1))

    def test_action_dispatch_cannot_disappear(self):
        raw,origin = tool.verified_capture(self.index,'canary','data/npclib/npc_system/keyword_handler.lua')
        pattern = rb'if childNode:processMessage\(npc, player, messageLower\) then\s*childNode:processAction\(player, messageLower\)\s*return true'
        tool.span(raw,origin,pattern)
        with self.assertRaisesRegex(ValueError,'nonunique reviewed source branch'):
            tool.span(raw.replace(b'childNode:processAction(player, messageLower)',b'changed()'),origin,pattern)

    def test_unqualified_postman_rank_never_grants_route_discount(self):
        row = self.packet['discount_relations'][0]
        key = row['prospective_global_privilege']['track']['existing_source_id']
        self.assertEqual(tool.discount_preview(row,{key:3})['state'],'HOLD')
        self.assertEqual(tool.discount_preview(row,{},True)['state'],'HOLD')
        self.assertEqual(tool.discount_preview(row,{key:2},True)['price'],row['base_gold'])
        small = copy.deepcopy(row);small['base_gold']=5
        result=tool.discount_preview(small,{key:3},True)
        self.assertEqual(result['price'],0)
        self.assertFalse(result['native_eligible'])
        self.assertTrue(all(r['source_condition']['disposition']=='PLACEHOLDER_REJECTED' for r in self.packet['discount_source_helpers']))

    def test_all_171_registration_route_pointer_hashes_and_scalar_rejection(self):
        documents = {}
        for relation in self.packet['discount_relations']:
            witness = relation['registration_custody']
            if witness['path'] not in documents:
                raw = (REPO/witness['path']).read_bytes()
                documents[witness['path']] = (hashlib.sha256(raw).hexdigest(),json.loads(raw))
            digest,document = documents[witness['path']]
            self.assertEqual(digest,witness['sha256'])
            route = tool.resolve_pointer(document,witness['pointer'])
            self.assertIsInstance(route,dict)
            self.assertTrue({'destination','price','discount','destination_keyword'} <= route.keys())
            canonical = json.dumps(route,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode()
            self.assertEqual(hashlib.sha256(canonical).hexdigest(),witness['row_sha256'])
            self.assertEqual(tool.verify_registration_custody(document,witness),route)
            changed = dict(witness,pointer=witness['discount_field_pointer'])
            with self.assertRaisesRegex(ValueError,'must identify a whole route'):
                tool.verify_registration_custody(document,changed)

    def test_item_fluid_count_and_citizen_title_are_not_reinterpreted(self):
        fluid = next(r for r in self.packet['offer_relations'] if r['relation'].get('source_fluid_symbol')=='FLUID_OIL')
        self.assertEqual(fluid['held_native_tuple']['count'],7)
        self.assertIsNone(fluid['relation']['native_variant_binding'])
        citizen = self.packet['route_relations'][2]['source_gate']
        self.assertIsNone(citizen['numeric_threshold'])
        self.assertIsNone(citizen['existing_source_track'])
        self.assertEqual(citizen['qualification'],'CITIZEN_TITLE_NOT_POINT_TALLY')

    def test_every_byte_witness_and_internal_pointer(self):
        for value in walk(self.packet):
            if not isinstance(value,dict):
                continue
            if 'byte_start' in value:
                raw=(tool.SOURCE_DIR/(value['sha256']+'.lua')).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(),value['sha256'])
                fragment=raw[value['byte_start']:value['byte_end_exclusive']]
                self.assertTrue(fragment)
                self.assertEqual(hashlib.sha256(fragment).hexdigest(),value['fragment_sha256'])
            for key,item in value.items():
                if key.endswith('_pointer') and isinstance(item,str) and item.startswith(('/source_track_registry/','/merchant_source_bindings/')):
                    tool.resolve_pointer(self.packet,item)
                if key.endswith('_pointers') and isinstance(item,list):
                    for pointer in item:
                        tool.resolve_pointer(self.packet,pointer)
            for forbidden in ('raw','raw_shop_row','text','excerpt','raw_expression'):
                self.assertNotIn(forbidden,value)

    def test_deterministic_regeneration(self):
        self.assertEqual(tool.encoded(self.packet),tool.encoded(tool.build(REPO)))


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--repo',type=pathlib.Path,required=True)
    parser.add_argument('--evidence-dir',type=pathlib.Path,required=True)
    parser.add_argument('--source-dir',type=pathlib.Path,required=True)
    args,remaining=parser.parse_known_args()
    REPO=args.repo
    tool.R7=args.evidence_dir;tool.SOURCE_DIR=args.source_dir
    unittest.main(argv=['test_service_relations.py']+remaining)
