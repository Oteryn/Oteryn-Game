from pathlib import Path
import json
cs=json.loads(Path('.merge-work/allchunks.json').read_text())
def chunks(suffix):return cs[next(p for p in cs if p.endswith(suffix))]
def edit(suffix,f):
 p=next(Path(p) for p in cs if p.endswith(suffix));p.write_text(f(p.read_text()))
edit('batch3_a/hot_cuisine.rs',lambda s:s) if any(p.endswith('batch3_a/hot_cuisine.rs') for p in cs) else None
p=Path('crates/gameplay/quests/src/quest/batch3_a/hot_cuisine.rs');p.write_text(p.read_text().replace('read_only: false,\nset_if: None,\nadd: None,\nincrement: false,\nat_least: false,\n',''))
c=chunks('qengine_reward_item_profiles.py')
extra=c[0][2].replace('IDS =','SIDE_IDS =').replace('IDS |=','SIDE_IDS |=')
# STATIC_IDS belongs to both the destination qualifier and the source definition guard.
edit('qengine_reward_item_profiles.py',lambda s:s.replace("PROOF =",extra+'\nIDS |= SIDE_IDS\nPROOF =',1).replace(c[1][0],c[1][2]+c[1][0][c[1][0].index("        prior="):]).replace('            merged = reconcile(prior, record)','            reconcile_active_physical(prior, record)\n            import questpc7_quest_item_reconcile\n            questpc7_quest_item_reconcile.preserve(prior, record)\n            import questpc7_physical_custody_join\n            physical = questpc7_physical_custody_join.observation(prior, record, accepted.accepted_first)\n            merged = reconcile(prior, physical)'))
# Existing expanded cosmetic intake owns item construction; add sideC programmes afterwards.
c=chunks('qengine_rewards.py')
edit('qengine_rewards.py',lambda s:s.replace('    result = {}','    paths += ["data-global/npc/dermot.lua"]\n    result = {}',1).replace(c[3][0],c[3][0]+c[3][2]).replace(c[5][0],c[5][0]+c[5][2]).replace(c[7][0],c[7][2]))
# Both static count mechanisms remain supported in pin refresh.
c=chunks('refresh_equipment_profile_pin.py')
extra=c[0][2].split('def regenerate')[0]
edit('refresh_equipment_profile_pin.py',lambda s:s.replace('def frozen_union():',extra+'def frozen_union():',1).replace('+ sum(map(int, extra.groups()))','+ sum(map(int, extra.groups()))\n            + questpc2_additions(source)'))
# Strict read-only validation retains at_least and conditional-set constraints.
c=chunks('quest_batch4.py')
edit('quest_batch4.py',lambda s:s.replace(c[0][0],c[0][2]).replace(c[1][0],c[1][2]).replace("'increment', 'at_least'","'increment', 'at_least', 'set_if'").replace("        require(type(storage.get('read_only', False)) is bool, key + ': invalid read_only')", "        if storage.get('set_if') is not None:\n            require(integer(storage['set_if'], storage['minimum'], storage['maximum'])\n                    and not storage.get('preserve', False) and not storage.get('read_only', False)\n                    and not storage.get('increment', False) and not storage.get('at_least', False)\n                    and callback.get('native_rule') is None, key + ': invalid conditional set')\n        require(type(storage.get('read_only', False)) is bool, key + ': invalid read_only')"))
p=Path('tools/content-schema/quest-authoring/test_quest_claim_qualification.py');s=p.read_text();start=s.index('    def test_missing_row_is_rejected(self):');end=s.index('        # 189 source claims',start);s=s[:start]+s[end:];s=s.replace('self.assertEqual(sum(counts.values()), 145)','self.assertEqual(sum(counts.values()), qualification.EXPECTED_DENOMINATOR)');p.write_text(s)
