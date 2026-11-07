//! Actual finite kill/TAKE batch; the outer lexical owner is rebuilt only after logout/reset.
use super::*;
use crate::foundation::MovementLocalPosition;
pub(super) enum Batch {
    Complete { actual_deaths: usize },
    NeedsCorpseRetirement { actual_deaths: usize },
}
async fn carried(h: &Harness) -> TestResult<[i64; 3]> {
    let mut result = [0; 3];
    for (slot, item) in [12312, 12314, 12313].into_iter().enumerate() {
        result[slot]=sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid)c JOIN game_item_instances i ON i.item_instance_id=c.item JOIN game_item_mint_receipts r USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key=$3 AND r.loot_table_production_key='oteryn:loot.creature.stampor'")
 .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?;
    }
    Ok(result)
}
pub(super) async fn collect_batch(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    home: MovementLocalPosition,
    source_respawn_ms: u32,
) -> TestResult<Batch> {
    for count in 0..56_usize {
        if carried(h)
            .await?
            .into_iter()
            .zip([50, 30, 100])
            .all(|(have, need)| have >= need)
        {
            return Ok(Batch::Complete {
                actual_deaths: count,
            });
        }
        let live:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.world_id=encode($1,'hex')::uuid AND i.lifecycle=1")
   .bind(id(42).as_slice()).fetch_one(&h.pool).await?;
        if live >= 56 {
            return Ok(Batch::NeedsCorpseRetirement {
                actual_deaths: count,
            });
        }
        let old:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_mint_receipts r JOIN game_item_instances i USING(item_instance_id) WHERE r.loot_purpose_key='CORPSE_MATERIALIZATION' AND i.definition_production_key='oteryn:item.tibia.i12325'")
   .fetch_one(&h.pool).await?;
        let absent = {
            let runtime = owner.runtime.lock().await;
            runtime
                .spawn_point_creatures()
                .iter()
                .all(|(_, p, a)| *p != home || a.is_none())
        };
        if absent {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_millis(
                    u64::from(source_respawn_ms) + 1,
                ))
                .ok_or("source respawn clock")?;
            combat::respawn(owner).await?;
        }
        let character = admitted
            .item_fence
            .ok_or("actual Character fence")?
            .character_id;
        if matches!(crate::gameplay_transport::spell_character_facts::load_character_cast_facts(&h.root,owner.character,character).await,
            crate::gameplay_transport::spell_character_facts::CastFactsLoad::NoVocation{facts,..} if facts.level>=8)
        {
            // A real source-earned level8 composes the mature Native vocation
            // choice owner. No historical progression/build row is arranged.
            combat::choose_class(h, owner, admitted).await?;
        }
        combat::kill(h, owner, admitted, home).await?;
        combat::take_materials(h, owner, admitted).await?;
        let new:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_mint_receipts r JOIN game_item_instances i USING(item_instance_id) WHERE r.loot_purpose_key='CORPSE_MATERIALIZATION' AND i.definition_production_key='oteryn:item.tibia.i12325'")
   .fetch_one(&h.pool).await?;
        assert_eq!(
            new,
            old + 1,
            "one unchanged native source death and conserved body per turn"
        );
    }
    Ok(Batch::NeedsCorpseRetirement { actual_deaths: 56 })
}
