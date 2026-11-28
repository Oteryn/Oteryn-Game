use super::{TestResult, fixture::debug};

pub(super) const HUNT: &str = "qengine-rewards-afflicted-earned-loot";

pub(super) async fn hunt(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &super::AdmittedSession,
    h: &super::fixture::Harness,
) -> TestResult {
    use crate::foundation::{CharacterId, CharacterLease};

    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "Afflicted loot fence")?
        .ok_or("Afflicted loot session")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("Afflicted hunter")?,
            admitted.game_session_id,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;

    for (index, (creature, id)) in (12551..=12556)
        .map(|id| ("feverish_citizen", id))
        .chain([("swamp_troll", 12517)])
        .enumerate()
    {
        let item = format!("oteryn:item.tibia.i{id}");
        super::l6_tomb_native_fixtures::hunt_loot(
            owner,
            admitted,
            h,
            creature,
            &item,
            1,
            25_000 + 10 * u64::try_from(index)?,
        )
        .await?;
        let carried: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_container_entries e
             JOIN game_item_instances i USING(item_instance_id)
             WHERE i.definition_production_key=$1 AND i.lifecycle=1",
        )
        .bind(item)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(carried, 1);
    }
    Ok(())
}
