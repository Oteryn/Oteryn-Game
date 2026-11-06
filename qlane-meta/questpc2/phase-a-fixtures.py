# One-time authoring script; do not rerun. Reproducible producers live under tools/.
import json
from pathlib import Path
p=Path('crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc2_rewards.rs')
s=p.read_text().replace('    quantity: u32,\n','    quantity: u32,\n    contents: &\'static [(&\'static str, i64)],\n    text_digest: Option<&\'static str>,\n')
s=s.replace('    quantity: 1,\n','    quantity: 1,\n    contents: &[],\n    text_digest: None,\n')
claims={r['definition']['identity']['key']:r['definition'] for p in Path('content/interactions/reward_claims').glob('reward-claim*.json') for r in json.loads(p.read_text())['records']}
cohorts={'PHASE_A_LEAVES':['kv.orcfortress1','kv.orcfortress2','kv.orcfortress3','kv.platearmor','kv.purpletome1','kv.purpletome2','kv.purpletome3'], 'PHASE_A_CONTAINERS':['kv.orcshaman','kv.panpipe','kv.poisondaggers','kv.powerbolt1'], 'PHASE_A_BOOK':['kv.powerbolt2']}
out='// Pinned Crystal startup chest table; exact roots and source-ordered children.\n'
for name, keys in cohorts.items():
 out+=f'const {name}: &[Chest] = &[\n'
 for key in keys:
  d=claims['oteryn:reward-claim.'+key]; v=d['placements'][0]; root=v['reward'].get('container',v['reward']['items'][0]['item'])['key']; pos=v['source_binding']['project_position']; uid=next(i['unique_id'] for i in v['source_binding']['legacy_unique_ids'] if i['server']=='crystalserver')
  children=d.get('native_contents',{}).get('placements',[{}])[0].get('contents',[])
  out+=f'''    Chest {{
        claim: "{d['identity']['key']}",
        position: ({pos['x']}, {pos['y']}, -{pos['z']}),
        unique: {uid},
        appearance: {v['appearance_tibia_id']},
        reward: "{root}",
        quantity: 1,
        contents: &[{', '.join('("'+i['item']['key']+'", '+str(i['count'])+')' for i in children)}],
        text_digest: {('Some("'+v['reward']['written_text']['text_ref']['sha256']+'")') if 'written_text' in v['reward'] else 'None'},
    }},
'''
 out+='];\n'
out+='''
fn phase_a_chests() -> impl Iterator<Item = &'static Chest> {
    PHASE_A_LEAVES.iter().chain(PHASE_A_CONTAINERS).chain(PHASE_A_BOOK)
}

#[test]
fn questpc2_rewards_phase_a_leaf_cohort() -> TestResult {
    execute_cohort(PHASE_A_LEAVES)
}

#[test]
fn questpc2_rewards_phase_a_container_cohort() -> TestResult {
    execute_cohort(PHASE_A_CONTAINERS)
}

#[test]
fn questpc2_rewards_phase_a_book_cohort() -> TestResult {
    execute_cohort(PHASE_A_BOOK)
}

fn execute_cohort(chests: &'static [Chest]) -> TestResult {
    for chest in chests {
        execute(chest)?;
    }
    Ok(())
}

'''
s=s.replace('fn committed_attrs(',out+'fn committed_attrs(')
s=s.replace('for chest in [&NAGINATA, &NIGHTMARE_TEDDY, &NOBLE_ARMOR, &CROWN_HELMET] {','for chest in [&NAGINATA, &NIGHTMARE_TEDDY, &NOBLE_ARMOR, &CROWN_HELMET]\n        .into_iter().chain(phase_a_chests()) {')
needle='''            if let Some(original) = original_item {'''
insert='''            let children: Vec<(String, i64)> = sqlx::query_as(
                "SELECT i.definition_production_key,i.quantity \\
                 FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) \\
                 WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY e.placement_ordinal",
            )
            .bind(item.item_instance_id.as_slice())
            .fetch_all(&harness.pool)
            .await?;
            assert_eq!(children, chest.contents.iter().map(|(key, count)| ((*key).into(), *count)).collect::<Vec<_>>(), "{} contents", chest.claim);
            if let Some(digest) = chest.text_digest {
                let text: (String, i64) = sqlx::query_as(
                    "SELECT encode(sha256(convert_to(text,'UTF8')),'hex'),text_revision::bigint \\
                     FROM game_item_texts WHERE item_instance_id=encode($1,'hex')::uuid",
                )
                .bind(item.item_instance_id.as_slice())
                .fetch_one(&harness.pool)
                .await?;
                assert_eq!(text, (digest.into(), 1));
            }
'''
assert s.count(needle)==1
s=s.replace(needle,insert+needle)
s=s.replace('WHERE item_instance_id=encode($2,\'hex\')::uuid)', 'WHERE item_instance_id=encode($2,\'hex\')::uuid OR item_instance_id IN (SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=encode($2,\'hex\')::uuid))')
s=s.replace('assert_eq!(counts, (1, 1, 1, 1));','assert_eq!(counts, (1, 1, 1 + i64::try_from(chest.contents.len())?, 1));')
p.write_text(s)
Path('.questpc2/phase-a-claims.json').write_text(json.dumps(['oteryn:reward-claim.'+k for ks in cohorts.values() for k in ks]))
