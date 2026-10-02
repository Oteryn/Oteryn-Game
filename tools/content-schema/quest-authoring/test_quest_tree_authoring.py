"""No-network source binding, readiness and committed-batch schema tests."""
import copy
import json
import unittest
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path
import jsonschema
import quest_tree_authoring as tool


def source_quest():
    return {'identity': {'key': 'canary:quest/example', 'revision': 'source-r1'}, 'display_name': 'Example', 'kind': 'reward_only', 'shown_in_quest_log': False, 'claims': [{'family': 'RewardClaim', 'key': 'canary:reward-claim/example', 'revision': 'source-r1'}], 'requirements': {'premium': False, 'min_level': 0}, 'requirements_from_wiki': {'premium': 'no', 'lvl': '0'}}


def source_claim():
    return {'identity': {'key': 'oteryn:reward-claim.example', 'revision': 'reward-claim-r1'}, 'provenance': {'pilot_key': 'canary:reward-claim/example', 'pilot_revision': 'source-r1'}, 'quest': {'family': 'Quest', 'key': 'canary:quest/example', 'revision': 'source-r1'}, 'readiness': 'ready'}


class QuestTreeTests(unittest.TestCase):
    def test_ready_keeps_source_aliases_and_existing_canonical_reference(self):
        record = tool.build_records([source_quest()], [source_claim()])[0]['definition']
        self.assertEqual(record['identity']['key'], 'oteryn:quest.example')
        self.assertEqual(record['source_refs']['quest']['key'], 'canary:quest/example')
        self.assertEqual(record['claims'][0]['key'], 'oteryn:reward-claim.example')
        self.assertEqual(record['readiness'], 'definition_ready')

    def test_missing_claim_is_source_only_not_an_invented_canonical_reference(self):
        record = tool.build_records([source_quest()], [])[0]['definition']
        self.assertEqual(record['claims'], [])
        self.assertEqual(record['unresolved_claims'], source_quest()['claims'])
        self.assertEqual(record['missing_data'][0]['code'], 'claim_content_missing')

    def test_revision_mismatch_never_joins_a_similar_alias(self):
        claim = source_claim(); claim['provenance']['pilot_revision'] = 'source-r2'
        record = tool.build_records([source_quest()], [claim])[0]['definition']
        self.assertEqual(record['claims'], [])
        self.assertEqual(record['readiness'], 'waiting_data')

    def test_unknown_requirement_and_waiting_item_keep_distinct_reasons(self):
        quest = source_quest(); quest['requirements']['min_level'] = None
        claim = source_claim(); claim['readiness'] = 'waiting_item_semantics'
        record = tool.build_records([quest], [claim])[0]['definition']
        self.assertEqual({i['code'] for i in record['missing_data']}, {'requirement_unknown', 'claim_item_semantics_missing'})

    def test_missing_requirement_is_not_defaulted_to_zero(self):
        quest = source_quest(); del quest['requirements']['min_level']
        record = tool.build_records([quest], [source_claim()])[0]['definition']
        self.assertNotIn('min_level', record['requirements'])
        self.assertEqual(record['missing_data'], [{'code': 'requirement_unknown', 'field': 'min_level'}])

    def test_duplicate_identity_and_wrong_owner_fail(self):
        with self.assertRaisesRegex(ValueError, 'duplicate canonical'):
            tool.build_records([source_quest(), source_quest()], [source_claim()])
        claim = source_claim(); claim['quest']['key'] = 'canary:quest/other'
        with self.assertRaisesRegex(ValueError, 'owner'):
            tool.build_records([source_quest()], [claim])

    def test_false_readiness_and_changed_reference_are_detected(self):
        rows = tool.build_records([source_quest()], [])
        rows[0]['definition']['readiness'] = 'definition_ready'
        self.assertTrue(tool.validate(rows, [source_quest()], []))
        rows = tool.build_records([source_quest()], [source_claim()])
        rows[0]['definition']['claims'][0]['key'] = 'oteryn:reward-claim.invented'
        self.assertTrue(tool.validate(rows, [source_quest()], [source_claim()]))

    def test_source_gaps_and_kind_log_conflict_block_definition_readiness(self):
        quest = source_quest(); quest['shown_in_quest_log'] = True
        report = {'quest': quest['identity']['key'], 'kind': 'reward_only', 'features': [], 'interactions': 1, 'data_gaps': {'interactions_with_gaps': 1, 'unresolved_items': 2}}
        row = tool.build_records([quest], [source_claim()], {quest['identity']['key']: report})[0]['definition']
        self.assertEqual(row['readiness'], 'waiting_data')
        self.assertEqual(row['completeness'], 'NOT_ASSESSED')
        self.assertEqual(sum(i.get('count', 0) for i in row['missing_data']), 3)
        self.assertIn({'code': 'source_kind_log_flag_conflict'}, row['missing_data'])

    def test_registration_preserves_other_family_seals(self):
        project = {'migrated_families': ['Item'], 'next_population_families': ['Quest', 'Interaction']}
        manifest = {'families': {'Item': {'records': 1, 'index': 'items'}}, 'managed_files': [{'path': 'content/items/a.json'}]}
        lock = {'family_counts': {'Item': 1}, 'unchanged': 'seal'}
        originals = copy.deepcopy((project, manifest, lock))
        updated = tool.registered(project, manifest, lock, 111, ['content/quests/definitions/index.json'])
        self.assertEqual((project, manifest, lock), originals)
        self.assertEqual(updated[1]['families']['Item'], manifest['families']['Item'])
        self.assertEqual(updated[2]['unchanged'], 'seal')
        self.assertEqual(updated[0]['next_population_families'], ['Interaction'])
        self.assertEqual(tool.registered(*updated, 111, ['content/quests/definitions/index.json']), updated)

    def test_rebuild_removes_only_superseded_generated_shards(self):
        root = Path(__file__).resolve().parents[3]
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            directory = output / tool.DIRECTORY
            directory.mkdir(parents=True)
            stale = directory / 'quests-00999-00999.json'
            stale.write_text('{}')
            unrelated = directory / 'quests-handwritten-note.json'
            unrelated.write_text('{}')
            command = [sys.executable, str(Path(tool.__file__)), 'content', '--root', str(root), '--output', str(output)]
            subprocess.run(command, check=True, capture_output=True)
            self.assertFalse(stale.exists())
            self.assertTrue(unrelated.exists())
            subprocess.run([*command, '--check'], check=True, capture_output=True)

    def test_all_source_kinds_are_preserved_without_placeholder_conversion(self):
        root = Path(__file__).resolve().parents[3]
        packet = tool.source_catalogue_packet(root)
        source = tool.read(root, tool.SOURCE)
        self.assertEqual([r['source_quest'] for r in packet['records']], source['quests'])
        self.assertEqual(packet['by_kind'], dict(Counter(q['kind'] for q in source['quests'])))
        self.assertEqual(packet['canonical_admission'], 'NOT_ASSESSED')
        for r in packet['records']:
            if r['source_quest']['kind'] == 'storyline':
                self.assertTrue(r['source_quest']['missions'])

    def packet_fixture(self):
        root = Path(__file__).resolve().parents[3]
        return root, tool.source_catalogue_packet(root)

    def test_source_packet_false_cardinality_and_kind_counts_are_rejected(self):
        root, packet = self.packet_fixture()
        wrong = copy.deepcopy(packet); wrong['record_count'] += 1
        with self.assertRaisesRegex(ValueError, 'record_count'):
            tool.validate_source_packet(wrong, root)
        wrong = copy.deepcopy(packet); wrong['by_kind']['storyline'] += 1
        with self.assertRaisesRegex(ValueError, 'by_kind'):
            tool.validate_source_packet(wrong, root)

    def test_source_packet_duplicate_identity_is_rejected_even_with_matching_counts(self):
        root, packet = self.packet_fixture()
        packet['records'].append(copy.deepcopy(packet['records'][0]))
        packet['record_count'] += 1
        kind = packet['records'][0]['source_quest']['kind']
        packet['by_kind'][kind] += 1
        with self.assertRaisesRegex(ValueError, 'duplicate Quest'):
            tool.validate_source_packet(packet, root)

    def test_source_packet_missing_metadata_is_rejected(self):
        root, packet = self.packet_fixture()
        del packet['classification']
        with self.assertRaises(jsonschema.ValidationError):
            tool.validate_source_packet(packet, root)

    def test_source_packet_missing_storyline_shape_uses_existing_quest_schema(self):
        root, packet = self.packet_fixture()
        story = next(record['source_quest'] for record in packet['records'] if record['source_quest']['kind'] == 'storyline')
        del story['missions']
        with self.assertRaises(jsonschema.ValidationError):
            tool.validate_source_packet(packet, root)

    def test_source_packet_schema_binding_digest_is_verified(self):
        root, packet = self.packet_fixture()
        packet['source_schema_sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'schema digest'):
            tool.validate_source_packet(packet, root)

    def test_nonreward_empty_claims_do_not_create_a_fake_claim(self):
        quest = source_quest(); quest.update(kind='script_only', claims=[], wiki={'title': 'Example', 'pageid': 1, 'revid': 1})
        row = tool.build_records([quest], [])[0]['definition']
        self.assertEqual(row['claims'], [])
        self.assertEqual(row['unresolved_claims'], [])
        self.assertEqual(row['source_data']['quest'], quest)
        self.assertEqual(row['readiness'], 'waiting_data')
        self.assertEqual(row['native_lowering']['canonical_progress_refs'], [])
        self.assertEqual(row['missing_data'], [{'code': 'quest_native_lowering_missing'}])
        tool.tree_validator().validate({'schema': 'OTERYN_QUEST_TREE_SHARD/v1', 'family': 'Quest',
            'shard': {'index': 0, 'start': 0, 'end': 0, 'count': 1}, 'records': [{'definition': row}]})

    def test_native_claim_hold_is_not_mislabelled_as_item_semantics(self):
        claim = source_claim(); claim['readiness'] = 'waiting_implementation'
        row = tool.build_records([source_quest()], [claim])[0]['definition']
        self.assertEqual(row['missing_data'], [{'code': 'claim_native_lowering_missing', 'source_key': source_quest()['claims'][0]['key']}])
        claim.update(definition_profile='authored_variant_v1', readiness='waiting_data', data_holds=[{'category': 'source'}])
        row = tool.build_records([source_quest()], [claim])[0]['definition']
        self.assertEqual({r['code'] for r in row['missing_data']}, {'claim_native_lowering_missing', 'claim_source_data_missing'})
        claim['data_holds'].append({'category': 'item'})
        row = tool.build_records([source_quest()], [claim])[0]['definition']
        self.assertEqual({r['code'] for r in row['missing_data']}, {'claim_native_lowering_missing', 'claim_source_data_missing', 'claim_item_semantics_missing'})

    def test_full_tree_preserves_existing_reward_definitions_and_adds_real_graphs(self):
        root = Path(__file__).resolve().parents[3]
        quests = tool.read(root, tool.SOURCE)['quests']; claims = tool.claim_catalogue(root)
        reports = {q['quest']: q for q in tool.read(root, tool.READINESS)['quests']}
        gates = tool.read(root, tool.GATES)['gates']; checks = tool.read(root, tool.GATE_MANIFEST)['entries']
        data = tool.bound_source_data(root, quests, gates)
        rows = tool.build_records(quests, claims, reports, gates, checks, data)
        index = tool.read(root, tool.DIRECTORY + 'index.json')
        committed = [row for path in index['shards'] for row in tool.read(root, path)['records']]
        old = [r for r in committed if r['definition'].get('definition_profile') != 'oteryn_authored_v1']
        self.assertEqual(rows, old)
        self.assertEqual(len(committed) - len(old), 68)
        self.assertEqual(len(rows), len(quests))
        for row in [r for r in rows if 'source_data' in r['definition']]:
            q = row['definition']; source = next(v for v in quests if v['identity'] == {k: q['source_refs']['quest'][k] for k in ('key', 'revision')})
            self.assertEqual(q['source_data']['quest'], source)
            self.assertEqual(q['native_lowering']['state'], 'WAITING_IMPLEMENTATION')
            if q['kind'] == 'storyline':
                self.assertTrue(q['source_data']['quest']['missions'])
                self.assertTrue(q['source_data']['progress'])
        self.assertTrue(any(r['definition'].get('source_data', {}).get('interactions') for r in rows))

    def test_source_reference_gap_is_preserved_as_an_explicit_hold(self):
        quest = source_quest(); quest.update(kind='script_only', claims=[], wiki={'title': 'Example', 'pageid': 1, 'revid': 1})
        data = tool.build_records([quest], [])[0]['definition']['source_data']
        data['reference_gaps'] = [{'record_type': 'Interaction', 'record_key': 'canary:interaction/example/action',
                                  'field': '/rules/0/progress', 'target_type': 'Progress',
                                  'target_key': 'canary:quest-progress/missing', 'reason': 'not declared'}]
        row = tool.build_records([quest], [], source_data={quest['identity']['key']: data})[0]['definition']
        self.assertIn({'code': 'source_reference_missing', 'source_key': 'canary:quest-progress/missing'}, row['missing_data'])
        self.assertEqual(row['source_data']['reference_gaps'], data['reference_gaps'])

    def test_source_graph_mutation_is_not_accepted_by_exact_validator(self):
        quest = source_quest(); quest.update(kind='script_only', claims=[], wiki={'title': 'Example', 'pageid': 1, 'revid': 1})
        rows = tool.build_records([quest], [])
        rows[0]['definition']['source_data']['quest']['display_name'] = 'Guessed'
        self.assertTrue(tool.validate(rows, [quest], []))

    def test_schema_rejects_fake_canonical_tracks_and_missing_source_missions(self):
        root = Path(__file__).resolve().parents[3]
        files = tool.expected_files(root); index = json.loads(files[tool.DIRECTORY + 'index.json'])
        shard = next(json.loads(files[p]) for p in index['shards'] if any(r['definition']['kind'] == 'storyline' for r in json.loads(files[p])['records']))
        row = next(r['definition'] for r in shard['records'] if r['definition']['kind'] == 'storyline')
        row['native_lowering']['canonical_progress_refs'] = ['oteryn:quest-progress.guessed']
        with self.assertRaises(jsonschema.ValidationError):
            tool.tree_validator().validate(shard)
        row['native_lowering']['canonical_progress_refs'] = []
        del row['source_data']['quest']['missions']
        with self.assertRaises(jsonschema.ValidationError):
            tool.tree_validator().validate(shard)

    def test_source_interaction_conflict_keeps_both_typed_choices(self):
        root = Path(__file__).resolve().parents[3]
        data = tool.bound_source_data(root, tool.read(root, tool.SOURCE)['quests'], tool.read(root, tool.GATES)['gates'])
        q = data['canary:quest/the_inquisition_quest']
        conflict = next(c for c in q['interaction_source_conflicts'] if c['interaction'].endswith('/actions_rewards'))
        self.assertEqual({a['source'] for a in conflict['alternatives']}, {'canary', 'crystalserver'})
        self.assertIn('crystalserver:item/50261', json.dumps(conflict))

    def test_committed_inputs_and_schema_coverage_are_honest(self):
        # Work in the real repo after integration; fallback supports the isolated proposal.
        root = Path(__file__).resolve().parents[3]
        files = tool.expected_files(root)
        schema = json.loads(Path(__file__).with_name('quest_tree.schema.json').read_text())
        jsonschema.Draft202012Validator.check_schema(schema)
        index = json.loads(files[tool.DIRECTORY + 'index.json'])
        expected_count = len(tool.read(root, tool.SOURCE)['quests']) + 68
        self.assertEqual(index['record_count'], expected_count)
        self.assertEqual(index['catalogue_coverage']['source_records'], len(tool.read(root, tool.SOURCE)['quests']))
        self.assertTrue(index['catalogue_coverage']['complete'])
        self.assertEqual(index['runtime_readiness'], 'NOT_ASSESSED')
        self.assertEqual(sum(index['readiness'].values()), expected_count)
        for path in index['shards']:
            tool.tree_validator().validate(json.loads(files[path]))


if __name__ == '__main__':
    unittest.main()
