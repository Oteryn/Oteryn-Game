import copy
import json
import os
from pathlib import Path
import unittest
import source_npc_exchange_guard as npc
import source_fix_guard as guard


class ExactPairTests(unittest.TestCase):
    def setUp(self):
        root = Path(os.environ.get('OTERYN_NPC_PROFILE_ROOT', Path(__file__).parent / 'samples/npc-exchanges'))
        self.old = {'identity': {'key': npc.CORE, 'revision': 'quest-r1'},
            'source_data': {'progress': [json.loads((root/'old-track.json').read_text())]},
            'reported_source_readiness': {'data_gaps': {'unresolved_items': 1}},
            'missing_data': [{'code': 'reported_source_gap', 'field': 'unresolved_items', 'count': 1}],
            'native_lowering': {'state': 'waiting_native_bindings'}}
        self.new = copy.deepcopy(self.old)
        self.new['source_data']['progress'] = [json.loads((root/'new-track.json').read_text())]

    def approve(self, new):
        change = {'key': npc.CORE, 'from_digest': guard.digest(self.old), 'to_digest': guard.digest(new),
                  'old_core': self.old, 'new_core': new}
        approval = {'approved_core_digests': [{k: change[k] for k in ('key', 'from_digest', 'to_digest')}],
                    'approved_graphs': []}
        guard.validate_change(change, {npc.CORE: change['from_digest']}, new, approval)

    def test_exact_pair_passes_and_copies(self):
        self.approve(self.new)
        before, after = npc.reviewed_pair(self.old, self.new)
        self.assertEqual(before, after)
        self.assertNotEqual(self.old, self.new)

    def test_resealed_snapshot_mutants_reject(self):
        for mutation in [lambda c: c['source_data']['progress'].clear(),
                         lambda c: c['source_data']['progress'].append(copy.deepcopy(c['source_data']['progress'][0])),
                         lambda c: c['source_data']['progress'][0]['transitions'][0]['source_occurrences'].pop(),
                         lambda c: c['source_data']['progress'][0].update(auxiliary_of=['canary:quest/other']),
                         lambda c: c['source_data']['progress'][0]['transitions'][0]['source_occurrences'][0]['source_exchange'].update(remaining_holds=[]),
                         lambda c: c.update(native_lowering={'state': 'READY'}),
                         lambda c: c.update(requirements={'min_level': 0})]:
            with self.subTest(mutation=mutation):
                mutant = copy.deepcopy(self.new); mutation(mutant)
                with self.assertRaises(ValueError): self.approve(mutant)

    def test_foreign_core_cannot_use_exception(self):
        mutant = copy.deepcopy(self.new); mutant['identity']['key'] = 'oteryn:quest.other'
        with self.assertRaises(ValueError): npc.reviewed_pair(self.old, mutant)


if __name__ == '__main__':
    unittest.main()
