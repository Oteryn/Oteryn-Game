"""Selected Noodles story uses exact native collar, actor binding and report values."""
import json
import unittest
import noodles_programme as n

class NoodlesProgramme(unittest.TestCase):
    def test_exact_actor_tool_and_atomic_report(self):
        callbacks,greetings,world=[],[],[]
        n.extend_rewards(callbacks,greetings);n.extend_world(world)
        self.assertEqual(len(callbacks),3)
        collar,tool,report=callbacks
        self.assertEqual(collar['give'],dict(item='oteryn:item.tibia.i14156',count=1,contents=[]))
        self.assertEqual(tool['take'],[dict(item='oteryn:item.tibia.i14156',count=1)])
        self.assertTrue(tool['carried_target_npc']);self.assertEqual(tool['npc'],'oteryn:npc.noodles')
        self.assertIsNone(tool['give'])
        self.assertEqual(report['give'],dict(item='oteryn:item.tibia.i3035',count=5,contents=[]))
        self.assertEqual(report['experience'],1000);self.assertEqual(report['achievement'],'oteryn:achievement/dog_sitter');self.assertTrue(report['completes'])
        self.assertEqual(world[0]['storage'],[dict(track=n.TRACK,minimum=0,maximum=0,set=1)])
        packet=json.loads((n.ROOT/'content/items/definitions/spell-native-profiles.json').read_text())
        profile,=[r for r in packet['records'] if r['production_definition']['production_key']=='oteryn:item.tibia.i14156']
        self.assertEqual(profile['production_binding']['disposition'],'EXACT')
        self.assertEqual(profile['admission']['stack_class'],'NonStackable')
        self.assertEqual(profile['semantics']['physical']['value']['weight'],dict(state='KNOWN',value=110))
        placements=[]
        for path in (n.ROOT/'content/world/npc-placements').glob('placements-*.json'):
            placements.extend(r['declaration'] for r in json.loads(path.read_text())['records'])
        for npc,cell in [('king_tibianus',dict(x=32311,y=32172,floor=6)),('noodles',dict(x=32315,y=32178,floor=6))]:
            row,=[r for r in placements if r['npc']=='oteryn:npc.'+npc];self.assertEqual(row['cell'],cell)

if __name__=='__main__':unittest.main()
