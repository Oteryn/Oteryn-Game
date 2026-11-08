from pathlib import Path
import subprocess,sys,json
sys.path.insert(0,'.merge-work');from rust_structs import mask

def span(s,name):
 start=s.index('fn '+name+'(');m=mask(s);a=m.index('{',start);depth=1;b=a+1
 while depth:
  if m[b]=='{':depth+=1
  elif m[b]=='}':depth-=1
  b+=1
 return start,b
p=Path('crates/gameplay/quests/src/quest/reward_callbacks.rs');s=p.read_text();s=s.replace('fn extend_callbacks(\n    catalogue: QuestStateCatalogue,\n    callbacks: &[RewardCallback]','fn extend_fixture_callbacks(\n    catalogue: QuestStateCatalogue,\n    callbacks: &[RewardCallback]');s=s.replace('        let catalogue = extend_callbacks(\n            QuestStateCatalogue::new("counter-test"','        let catalogue = extend_fixture_callbacks(\n            QuestStateCatalogue::new("counter-test"');p.write_text(s)
p=Path('crates/gameplay/quests/src/quest/world_callbacks.rs');s=p.read_text();main=subprocess.check_output(['git','show',':2:'+str(p)]).decode();c=subprocess.check_output(['git','show',':3:'+str(p)]).decode();name='lane_qengine_shared_access_rejects_ambiguous_or_unbounded_effects';a,b=span(s,name);x,y=span(main,name);s=s[:a]+main[x:y]+s[b:];name='passage_state_guard_requires_a_reversible_removal_lever';x,y=span(c,name);pos=s.index('    #[test]\n    fn read_only_world_callback_has_no_quest_write_transition');s=s[:pos]+'    #[test]\n    '+c[x:y]+'\n\n'+s[pos:];p.write_text(s)
p=Path('crates/gameplay/quests/src/quest/authored_bindings/tests.rs');s=p.read_text().replace('            npc_legacy: base.npc_legacy.clone(),\n','');p.write_text(s)
p=Path('crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs');s=p.read_text();needle='            let read_only_gate = if callback.is_read_only() {';a=s.index(needle,s.index('pub(super) async fn') if 'pub(super) async fn' in s else 0);b=s.find(needle,a+1)
if b>=0:s=s[:b]+s[s.index('            let revision = map.facts.object_revision(*key);',b):]
p.write_text(s)
# Preserve both branches' scenario routing; shared harness stays single.
cs=json.loads(Path('.merge-work/allchunks.json').read_text());p=Path('crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs');s=p.read_text();rows=cs[str(p)]
s=s.replace('    let qengine_reward =',rows[0][2]+'    let qengine_reward =',1)
s=s.replace('                | "catalogue-npc-scoped"','                | "catalogue-npc-scoped"\n                | "questpc2-rotten-source-items"',1)
a=s.index('    let world = if name == "authored-wrath-crystals"');b=s.index('    run(async |admin| {',a)
own=rows[3][0];own=own[own.index('    let world ='):own.index('    run(async')];incoming=rows[3][2];incoming=incoming[incoming.index('    let world ='):incoming.index('    run(async')]
fallback='    } else {\n        world(&callback, trigger)?\n    };\n'
assert own.endswith(fallback)
joined=own[:-len(fallback)]+incoming.replace('    let world = if','    } else if',1)
s=s[:a]+joined+s[b:]
# Keep native-story actor capacity plus sideC's grimvale/friends budgets.
needle='            } else if native_story';pos=s.index(needle);s=s[:pos]+rows[5][2].split('            } else if matches!')[0]+s[pos:]
p.write_text(s)
