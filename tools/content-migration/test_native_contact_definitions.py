"""Native additions cannot hide any drift or relax the historical base count guards."""
import copy
import json
import unittest
import native_contact_definitions as contacts


class NativeContactDefinitions(unittest.TestCase):
    def setUp(self):
        self.packet=json.loads(contacts.PACKET.read_text())
        self.npcs=[r['declaration'] for r in self.packet['records']]
        self.bindings=[b for r in self.packet['records'] for b in r['source_bindings']]

    def test_old_cohort_and_exact_addition_are_the_only_accepted_modes(self):
        self.assertEqual(contacts.qualified_counts([],[]),(0,0))
        self.assertEqual(contacts.qualified_counts(self.npcs,self.bindings),(1,1))
        unrelated={'identity':{'key':'oteryn:npc.unrelated'}}
        self.assertEqual(contacts.qualified_counts([unrelated,*self.npcs],self.bindings),(1,1))

    def test_drift_duplicates_partial_or_orphan_additions_are_rejected(self):
        for mode in range(7):
            npcs=copy.deepcopy(self.npcs);bindings=copy.deepcopy(self.bindings)
            if mode==0:npcs[0]['services']=[{'key':'unexpected'}]
            elif mode==1:npcs[0]['presentation']['key']='oteryn:presentation.other'
            elif mode==2:npcs.append(npcs[0])
            elif mode==3:bindings.append(bindings[0])
            elif mode==4:bindings=[]
            elif mode==5:npcs=[]
            else:bindings[0]['target']['key']='oteryn:npc.foreign'
            with self.subTest(mode=mode),self.assertRaises(ValueError):
                contacts.qualified_counts(npcs,bindings)

    def test_empty_duplicate_or_foreign_packet_targets_are_rejected(self):
        for mode in range(3):
            packet=copy.deepcopy(self.packet)
            if mode==0:packet['records']=[]
            elif mode==1:packet['records'].append(packet['records'][0])
            else:packet['records'][0]['source_bindings'][0]['target']['key']='oteryn:npc.foreign'
            for npcs, bindings in [(self.npcs, self.bindings), ([], [])]:
                with self.subTest(mode=mode, legacy=not npcs), self.assertRaises(ValueError):
                    contacts.qualified_counts(npcs, bindings, packet)
