//! Read normal-death facts under the same session fence as settlement, without a death write.
use super::{adventurer, promotion};
use crate::domain::death::{
    AMULET_OF_LOSS_BLESSINGS_EXCLUSIVE, CONTAINER_LOSS_PER_MILLE, EQUIPMENT_LOSS_PER_MILLE,
    ITEM_LOSS_EXEMPT_MAX_LEVEL, death_experience_loss,
};
use crate::durability::{
    DurabilityError, DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::{
        CharacterProgressionError, CurrentCharacterGameplayFence, assert_item_only_fence,
    },
    db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations},
    runtime_scope_assignment::NodeIncarnationProof,
    spell_field_policy::{FieldWorldType, read_world_field_policy_in_transaction},
};
use oteryn_gameplay_combat_rules::pvp::death::{
    BlessingDisposition, TWIST_OF_FATE_KEY, blessing_disposition,
};
use oteryn_protocol_oteryn::cyclopedia_character::protection::{Consumption, Forecast, Scenario};
use sqlx::Row;
#[derive(Clone, Copy)]
struct Facts {
    observed: u64,
    level: u32,
    experience: u64,
    regular: u8,
    twist: bool,
    promoted: bool,
    skulled: bool,
    has_vocation: bool,
    amulet: bool,
    adventurer: bool,
    pvp_available: bool,
}
fn project(f: Facts) -> Result<Forecast, DurabilityError> {
    let raw = death_experience_loss(f.level, f.regular, f.promoted)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let loss = raw.min(u128::from(f.experience)) as u64;
    let scenario = |pvp: bool| {
        let protected = pvp && f.adventurer;
        let exempt = f.level <= ITEM_LOSS_EXEMPT_MAX_LEVEL || !f.has_vocation || protected;
        let amulet =
            !exempt && !f.skulled && f.amulet && f.regular < AMULET_OF_LOSS_BLESSINGS_EXCLUSIVE;
        let (gear, bag) = if exempt || amulet {
            (0, 0)
        } else if f.skulled {
            (1000, 1000)
        } else {
            let step = usize::from(f.regular.min(5));
            (
                EQUIPMENT_LOSS_PER_MILLE[step],
                CONTAINER_LOSS_PER_MILLE[step],
            )
        };
        let consumption = match blessing_disposition(
            pvp,
            f.skulled,
            protected,
            usize::from(f.regular),
            f.twist,
        ) {
            BlessingDisposition::KeepAll => Consumption::Keep,
            BlessingDisposition::ConsumeRegular => Consumption::Regular,
            BlessingDisposition::ConsumeTwist => Consumption::Twist,
        };
        Scenario {
            experience_lost: if protected { 0 } else { loss },
            equipment_per_mille: gear,
            container_per_mille: bag,
            consumption,
            amulet_consumed: amulet,
        }
    };
    Ok(Forecast {
        observed_at_unix_ms: f.observed,
        level: f.level,
        experience: f.experience,
        regular_blessings: f.regular,
        twist: f.twist,
        promotion_benefit: f.promoted,
        red_or_black: f.skulled,
        adventurer: f.adventurer,
        pve: scenario(false),
        pvp: f.pvp_available.then(|| scenario(true)),
    })
}
impl DurabilityRoot {
    pub async fn read_death_forecast(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        premium: bool,
    ) -> Result<Forecast, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let root = self.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            if assert_item_only_fence(&mut tx,&fence.item_fence(),&node).await?.is_err(){return Ok(Err(CharacterProgressionError::AuthorityRejected));}
            // The item fence already locks the Character root and serializes its writers.
            let row=sqlx::query("SELECT p.level,p.total_experience,b.vocation, \
                ARRAY(SELECT blessing_key FROM game_character_blessings WHERE character_id=p.character_id ORDER BY blessing_key LIMIT 9) AS held, \
                EXISTS(SELECT 1 FROM game_character_pvp_state WHERE character_id=p.character_id AND skull>0 AND skull_until>clock_timestamp()) AS skulled, \
                EXISTS(SELECT 1 FROM game_character_equipment_slots s JOIN game_item_instances i USING(item_instance_id) WHERE s.character_id=p.character_id AND s.slot=7 AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i3057') AS amulet, \
                floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS observed \
                FROM game_character_progression_state p LEFT JOIN game_character_build_state b USING(character_id) WHERE p.character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let narrow=||DurabilityError::InvalidStoredState;
            let level=u32::try_from(row.try_get::<i64,_>("level")?).map_err(|_|narrow())?;
            let held:Vec<String>=row.try_get("held")?;let twist=held.iter().any(|key|key==TWIST_OF_FATE_KEY);
            let regular=u8::try_from(held.len()-usize::from(twist)).map_err(|_|narrow())?;
            let promoted=promotion::benefit(&mut tx,fence.character_id,premium).await?;
            let channel=matches!(fence.runtime_scope,crate::foundation::RuntimeScopeRefV1::Channel{..});
            let pvp_available = if channel {
                // An unavailable policy cannot assert a player-death protection benefit.
                match read_world_field_policy_in_transaction(&mut tx, &root, fence.runtime_scope, fence.scope_ownership_generation.get()).await {
                    Ok(policy) => policy.is_some_and(|p| p.bind(&tx).mode() != FieldWorldType::NoPvp),
                    Err(DurabilityError::Unavailable) => false,
                    Err(error) => return Err(error),
                }
            } else { false };
            let protected=if pvp_available {adventurer::applies(&mut tx,&root,fence,level,true).await?}else{false};
            let vocation:Option<String>=row.try_get("vocation")?;
            let value=project(Facts {observed:u64::try_from(row.try_get::<i64,_>("observed")?).map_err(|_|narrow())?,level,experience:u64::try_from(row.try_get::<i64,_>("total_experience")?).map_err(|_|narrow())?,regular,twist,promoted,skulled:row.try_get("skulled")?,has_vocation:vocation.as_deref().is_some_and(|v|v!=crate::durability::character_build::NO_VOCATION),amulet:row.try_get("amulet")?,adventurer:protected,pvp_available})?;
            commit_semantic_transaction(tx,deadline).await?;Ok(Ok(value))
        })).await?
    }
}
#[cfg(test)]
#[path = "forecast_tests.rs"]
mod tests;
